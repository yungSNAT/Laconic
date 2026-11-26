// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The anchors a Markdown document offers: GitHub's heading anchors and HTML ids.

use std::collections::{HashMap, HashSet};

use super::syntax::*;

/// Whether `fragment` names one of `anchors`, ignoring case and GitHub's `user-content-` prefix.
pub(super) fn has_anchor(anchors: &HashSet<String>, fragment: &str) -> bool {
    let fragment = fragment.to_lowercase();
    let bare = fragment.strip_prefix("user-content-").unwrap_or(&fragment);
    bare == "top" || anchors.contains(ANY_ANCHOR) || anchors.contains(bare)
}

/// The anchors of a Markdown document, in lower case: each heading's GitHub anchor (with `-1`,
/// `-2` for repeats), each `{#id}` attribute and each HTML `id` or `name` value. Code fences
/// and a leading frontmatter block hold none.
pub(super) fn anchors(text: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut heading = |title: &str, found: &mut HashSet<String>| {
        // A `{#id}` attribute names an anchor for renderers that read it; GitHub does not,
        // and slugs the whole title, attribute included.
        let title = title.trim_end();
        if has_undecoded_entity(&rendered(title)) {
            found.insert(ANY_ANCHOR.to_owned());
        }
        if let Some((_, id)) = title
            .strip_suffix('}')
            .and_then(|rest| rest.rsplit_once("{#"))
        {
            found.insert(id.to_lowercase());
        }
        let slug = slug(title);
        let count = seen.entry(slug.clone()).or_insert(0);
        found.insert(if *count == 0 {
            slug.clone()
        } else {
            format!("{slug}-{count}")
        });
        *count += 1;
    };
    let mut lines = text.lines().peekable();
    if text.starts_with("---") && text.lines().skip(1).any(|l| l.trim_end() == "---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim_end() == "---" {
                break;
            }
        }
    }
    let mut fence: Option<&str> = None;
    let mut previous: Option<&str> = None;
    for line in lines {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(marker) = fence {
            if trimmed.starts_with(marker) {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(&trimmed[..3]);
            previous = None;
            continue;
        }
        collect_ids(trimmed, &mut found);
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        let atx = indent < 4
            && (1..=6).contains(&hashes)
            && trimmed[hashes..]
                .chars()
                .next()
                .is_none_or(|c| c == ' ' || c == '\t');
        let underline = indent < 4
            && !trimmed.is_empty()
            && (trimmed.trim_end().chars().all(|c| c == '=')
                || trimmed.trim_end().chars().all(|c| c == '-'));
        if atx {
            let title = trimmed[hashes..].trim().trim_end_matches('#');
            heading(title, &mut found);
            previous = None;
        } else if let (true, Some(title)) = (underline, previous) {
            heading(title, &mut found);
            previous = None;
        } else if trimmed.is_empty() {
            previous = None;
        } else if indent < 4 {
            previous = Some(trimmed);
        }
    }
    found
}

/// Records the values of `id="..."` and `name="..."` attributes in `line`.
pub(super) fn collect_ids(line: &str, found: &mut HashSet<String>) {
    for key in ["id=", "name="] {
        let mut rest = line;
        while let Some(at) = rest.find(key) {
            let after = &rest[at + key.len()..];
            let before = rest[..at].chars().next_back();
            rest = after;
            if !before.is_some_and(char::is_whitespace) {
                continue;
            }
            if let Some(quote) = after.chars().next().filter(|c| matches!(c, '"' | '\''))
                && let Some(value) = after[1..].split(quote).next()
            {
                found.insert(value.to_lowercase());
            }
        }
    }
}

/// GitHub's anchor for a heading: its rendered text (without links' targets, HTML tags,
/// emphasis marks and entity escapes) without punctuation other than hyphens and underscores, in lower case, with each space a hyphen.
pub(super) fn slug(title: &str) -> String {
    without_emphasis(&unescape_html(&rendered(title)))
        .replace(LITERAL_UNDERSCORE, "_")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// Stand-ins for an `&` and an `_` that a code span or a backslash keeps literal, so that no
/// entity or emphasis is read from them; the slug drops the first and restores the second.
const LITERAL_AMPERSAND: char = '\u{1}';
const LITERAL_UNDERSCORE: char = '\u{2}';

/// `title` as its anchor is read from it: without links' targets and HTML tags, and with the
/// text of code spans and backslash escapes kept literal.
fn rendered(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut in_code = vec![false; chars.len()];
    for (start, end) in code_spans(&chars) {
        in_code[start..end].fill(true);
    }
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Only these characters are tested for an escape, which keeps the test linear.
        let literal = matches!(c, '<' | ']' | '&' | '_') && (in_code[i] || is_escaped(&chars, i));
        let skip_to = match c {
            '<' if !literal => closing(&chars, i + 1, '>', &['<']).map(|end| end + 1),
            ']' if !literal && chars.get(i + 1) == Some(&'(') => {
                inline_target(&chars, i + 2).map(|(_, _, end)| end)
            }
            _ => None,
        };
        if let Some(end) = skip_to {
            i = end;
            continue;
        }
        text.push(match c {
            '&' if literal => LITERAL_AMPERSAND,
            '_' if literal => LITERAL_UNDERSCORE,
            c => c,
        });
        i += 1;
    }
    text
}

/// The longest entity name HTML defines (`CounterClockwiseContourIntegral`) has 31 characters,
/// so the `;` that ends an entity is at most this many characters past its `&`.
const LONGEST_ENTITY: usize = 32;

/// `text` with the entities for markup characters and numeric character references decoded,
/// as GitHub renders a heading before taking its anchor.
fn unescape_html(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest
            .char_indices()
            .take(LONGEST_ENTITY + 1)
            .find(|&(_, c)| c == ';')
            .map(|(k, _)| k);
        let decoded = end.and_then(|end| {
            let c = match &rest[1..end] {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                name => name
                    .strip_prefix('#')
                    .and_then(|n| match n.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => n.parse().ok(),
                    })
                    .and_then(char::from_u32)
                    .or_else(|| named_entity(name)),
            };
            c.map(|c| (c, end + 1))
        });
        let (c, len) = decoded.unwrap_or(('&', 1));
        out.push(c);
        rest = &rest[len..];
    }
    out + rest
}

/// HTML's names for U+00A0 to U+00FF, in code point order.
const LATIN_1: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf",
    "laquo", "not", "shy", "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro",
    "para", "middot", "cedil", "sup1", "ordm", "raquo", "frac14", "frac12", "frac34", "iquest",
    "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig", "Ccedil", "Egrave", "Eacute",
    "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve", "Oacute",
    "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute",
    "THORN", "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil",
    "egrave", "eacute", "ecirc", "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde",
    "ograve", "oacute", "ocirc", "otilde", "ouml", "divide", "oslash", "ugrave", "uacute", "ucirc",
    "uuml", "yacute", "thorn", "yuml",
];

/// Other named entities common in headings: typography, arrows and Latin Extended letters.
const NAMED: &[(&str, char)] = &[
    ("ndash", '\u{2013}'),
    ("mdash", '\u{2014}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("sbquo", '\u{201A}'),
    ("ldquo", '\u{201C}'),
    ("rdquo", '\u{201D}'),
    ("bdquo", '\u{201E}'),
    ("dagger", '\u{2020}'),
    ("Dagger", '\u{2021}'),
    ("bull", '\u{2022}'),
    ("hellip", '\u{2026}'),
    ("permil", '\u{2030}'),
    ("prime", '\u{2032}'),
    ("Prime", '\u{2033}'),
    ("lsaquo", '\u{2039}'),
    ("rsaquo", '\u{203A}'),
    ("euro", '\u{20AC}'),
    ("trade", '\u{2122}'),
    ("larr", '\u{2190}'),
    ("uarr", '\u{2191}'),
    ("rarr", '\u{2192}'),
    ("darr", '\u{2193}'),
    ("harr", '\u{2194}'),
    ("minus", '\u{2212}'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("thinsp", '\u{2009}'),
    ("zwnj", '\u{200C}'),
    ("zwj", '\u{200D}'),
    ("OElig", '\u{152}'),
    ("oelig", '\u{153}'),
    ("Scaron", '\u{160}'),
    ("scaron", '\u{161}'),
    ("Yuml", '\u{178}'),
    ("fnof", '\u{192}'),
    ("circ", '\u{2C6}'),
    ("tilde", '\u{2DC}'),
];

/// The character a named entity this rule knows stands for.
fn named_entity(name: &str) -> Option<char> {
    let latin = LATIN_1.iter().position(|&n| n == name);
    latin
        .and_then(|k| char::from_u32(0xA0 + k as u32))
        .or_else(|| NAMED.iter().find(|&&(n, _)| n == name).map(|&(_, c)| c))
}

/// Recorded with a document's anchors when a heading holds a named entity this rule cannot
/// decode: HTML defines over 2,000, so its anchor is unknown and no fragment is reported.
pub(super) const ANY_ANCHOR: &str = "\u{0}";

/// Whether `title` holds `&name;` for a name other than the markup ones and those above. Any
/// such word counts, so `R&D;`, which renders literally, also leaves the file's fragments
/// unchecked: the rule then reports less, never wrongly.
fn has_undecoded_entity(title: &str) -> bool {
    title.split('&').skip(1).any(|rest| {
        rest.split_once(';').is_some_and(|(name, _)| {
            name.starts_with(|c: char| c.is_ascii_alphabetic())
                && name.chars().all(|c| c.is_ascii_alphanumeric())
                && !matches!(name, "amp" | "lt" | "gt" | "quot" | "apos")
                && named_entity(name).is_none()
        })
    })
}

/// `text` without the `_` runs that open or close emphasis, which the rendered heading drops;
/// an `_` inside a word, as in `snake_case`, stays.
fn without_emphasis(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    // Whether the first character other than `_` from each index on is a word character, found
    // in one pass back, as a scan from each `_` takes time in the square of a run of them.
    let mut word_after = vec![false; chars.len() + 1];
    for i in (0..chars.len()).rev() {
        word_after[i] = if chars[i] == '_' {
            word_after[i + 1]
        } else {
            chars[i].is_alphanumeric()
        };
    }
    let mut word_before = false;
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c != '_' {
            word_before = c.is_alphanumeric();
            out.push(c);
        } else if word_before && word_after[i + 1] {
            out.push(c);
        }
    }
    out
}
