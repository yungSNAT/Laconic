// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Markdown syntax the link rules read: code spans, link targets, reference definitions,
//! HTML ids and GitHub's heading anchors.

use std::collections::{HashMap, HashSet};
/// A reference label as CommonMark matches it: case-folded, with runs of whitespace as one
/// space and none at either end.
pub(super) fn normalise(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(super) fn is_escaped(chars: &[char], i: usize) -> bool {
    chars[..i].iter().rev().take_while(|&&c| c == '\\').count() % 2 == 1
}

/// The `[start, end)` ranges of inline code spans: a run of backticks to the next run of the
/// same length.
pub(super) fn code_spans(chars: &[char]) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '`' || is_escaped(chars, i) {
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|&&c| c == '`').count();
        let mut k = i + run;
        let mut close = None;
        while k < chars.len() {
            if chars[k] == '`' {
                let other = chars[k..].iter().take_while(|&&c| c == '`').count();
                if other == run {
                    close = Some(k + run);
                    break;
                }
                k += other;
            } else {
                k += 1;
            }
        }
        match close {
            Some(end) => {
                spans.push((i, end));
                i = end;
            }
            None => i += run,
        }
    }
    spans
}

/// For each unescaped `]` in `chars`, the `[` that opens its bracketed text, allowing nested
/// brackets; found in one pass, as a search back from each `]` takes time in the square of a
/// paragraph of unmatched brackets.
pub(super) fn opening_brackets(chars: &[char]) -> Vec<Option<usize>> {
    let mut open = Vec::new();
    let mut found = vec![None; chars.len()];
    for (k, &c) in chars.iter().enumerate() {
        match c {
            '[' if !is_escaped(chars, k) => open.push(k),
            ']' if !is_escaped(chars, k) => found[k] = open.pop(),
            _ => {}
        }
    }
    found
}

/// The first unescaped `close` from `from`, stopping without one at an unescaped character in
/// `stop`, so that each scan ends where the next could begin.
fn closing(chars: &[char], from: usize, close: char, stop: &[char]) -> Option<usize> {
    let k = (from..chars.len())
        .find(|&k| (chars[k] == close || stop.contains(&chars[k])) && !is_escaped(chars, k))?;
    (chars[k] == close).then_some(k)
}

/// The index past the `)` that ends an inline link whose destination ends just before `from`,
/// after an optional title in quotes or parentheses.
fn link_end(chars: &[char], from: usize) -> Option<usize> {
    let skip = |mut i: usize| {
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        i
    };
    let mut i = skip(from);
    if let Some(&quote) = chars.get(i).filter(|c| matches!(c, '"' | '\'' | '(')) {
        let (close, stop) = if quote == '(' {
            (')', &['('][..])
        } else {
            (quote, &[][..])
        };
        i = skip(closing(chars, i + 1, close, stop)? + 1);
    }
    (chars.get(i) == Some(&')')).then_some(i + 1)
}

/// The destination of an inline link whose `(` ends just before `from`: its text, where it
/// starts and the index past the closing `)`. Each scan stops where another link could begin,
/// so a paragraph of unclosed links is read in linear time; a bare destination holding a
/// bracket is taken as no link, which reports nothing.
pub(super) fn inline_target(chars: &[char], from: usize) -> Option<(String, usize, usize)> {
    let mut i = from;
    while chars.get(i).is_some_and(|c| *c == ' ') {
        i += 1;
    }
    if chars.get(i) == Some(&'<') {
        let end = closing(chars, i + 1, '>', &['<', '\n'])?;
        return Some((
            chars[i + 1..end].iter().collect(),
            i + 1,
            link_end(chars, end + 1)?,
        ));
    }
    let start = i;
    let mut depth = 0;
    while let Some(&c) = chars.get(i) {
        match c {
            '(' if !is_escaped(chars, i) => depth += 1,
            ')' if !is_escaped(chars, i) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            '[' | ']' if !is_escaped(chars, i) => return None,
            c if c.is_whitespace() => break,
            _ => {}
        }
        i += 1;
    }
    Some((chars[start..i].iter().collect(), start, link_end(chars, i)?))
}

/// A reference definition, `[label]: target`, opening the line at `at` (up to 3 spaces in):
/// its label, target and the target's start.
pub(super) fn definition(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    let indent = chars[at..]
        .iter()
        .take(4)
        .take_while(|&&c| c == ' ')
        .count();
    let open = at + indent;
    if indent > 3 || chars.get(open) != Some(&'[') || chars.get(open + 1) == Some(&'^') {
        return None;
    }
    let close = closing(chars, open + 1, ']', &['['])?;
    if chars.get(close + 1) != Some(&':') || close == open + 1 {
        return None;
    }
    let label: String = chars[open + 1..close].iter().collect();
    let mut start = close + 2;
    while chars.get(start).is_some_and(|c| *c == ' ') {
        start += 1;
    }
    let (start, end) = if chars.get(start) == Some(&'<') {
        (start + 1, closing(chars, start + 1, '>', &['<', '\n'])?)
    } else {
        let end = (start..chars.len())
            .find(|&k| chars[k].is_whitespace())
            .unwrap_or(chars.len());
        (start, end)
    };
    (end > start).then(|| (label, chars[start..end].iter().collect(), start))
}

/// The quoted value of an HTML `href` or `src` attribute whose `=` is at `eq`: its text, where
/// it starts and the index past the closing quote.
pub(super) fn attribute(chars: &[char], eq: usize) -> Option<(String, usize, usize)> {
    let name: String = chars[..eq]
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        .to_lowercase();
    let len = if name.ends_with("href") {
        4
    } else if name.ends_with("src") {
        3
    } else {
        return None;
    };
    if eq < len + 1 || !chars[eq - len - 1].is_whitespace() {
        return None;
    }
    let quote = *chars.get(eq + 1).filter(|c| matches!(c, '"' | '\''))?;
    let end = (eq + 2..chars.len()).find(|&k| chars[k] == quote)?;
    Some((chars[eq + 2..end].iter().collect(), eq + 2, end + 1))
}

/// The decoded path and fragment of a target this rule checks, or `None` for one it never
/// checks: an absolute URL or path, a template placeholder or a target that does not decode.
pub(super) fn split_target(target: &str) -> Option<(String, Option<String>)> {
    let target = target.trim();
    let scheme = target.split_once(':').is_some_and(|(s, _)| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    if target.is_empty()
        || scheme
        || target.starts_with('/')
        || target.contains(['{', '\\', '<', '>'])
        || target.contains('$')
    {
        return None;
    }
    let (rest, fragment) = match target.split_once('#') {
        Some((rest, fragment)) => (rest, Some(decode(fragment)?)),
        None => (target, None),
    };
    let path = rest.split('?').next().unwrap_or("");
    let path = decode(path)?;
    // A drive letter or a backslash, encoded, could still name a UNC or absolute path.
    if path.contains([':', '\\']) {
        return None;
    }
    Some((path, fragment.filter(|f| !f.is_empty())))
}

/// `text` with its percent escapes decoded, or `None` when they do not form UTF-8.
pub(super) fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

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
        if has_undecoded_entity(title) {
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
    let chars: Vec<char> = title.chars().collect();
    let mut text = String::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '<' => match closing(&chars, i + 1, '>', &['<']) {
                Some(end) => i = end + 1,
                None => i += 1,
            },
            ']' if chars.get(i + 1) == Some(&'(') => match inline_target(&chars, i + 2) {
                Some((_, _, end)) => i = end,
                None => i += 1,
            },
            c => {
                text.push(c);
                i += 1;
            }
        }
    }
    without_emphasis(&unescape_html(&text))
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// `text` with the entities for markup characters and numeric character references decoded,
/// as GitHub renders a heading before taking its anchor.
fn unescape_html(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').and_then(|end| {
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

/// Whether `title` holds `&name;` for a name other than the markup ones and those above.
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
    let word = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric());
    (0..chars.len())
        .filter(|&i| {
            chars[i] != '_'
                || (word(chars[..i].iter().rev().find(|&&c| c != '_'))
                    && word(chars[i + 1..].iter().find(|&&c| c != '_')))
        })
        .map(|i| chars[i])
        .collect()
}

/// Where a place in the text sits: 1-based line and character column.
pub(super) type Place = (usize, usize);

/// The HTML comments in `text` outside fenced code, from `<!--` to the end of `-->`. A `<!--`
/// inside a code span opens nothing; a comment left open runs to the end of the text.
pub(super) fn comment_spans(text: &str) -> Vec<(Place, Place)> {
    let mut spans = Vec::new();
    let mut open: Option<Place> = None;
    let mut fence: Option<&str> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if open.is_none() {
            if let Some(marker) = fence {
                if trimmed.starts_with(marker) {
                    fence = None;
                }
                continue;
            }
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fence = Some(&trimmed[..3]);
                continue;
            }
        }
        let chars: Vec<char> = line.chars().collect();
        let mut in_code = vec![false; chars.len()];
        for (start, end) in code_spans(&chars) {
            in_code[start..end].fill(true);
        }
        let mut i = 0;
        while i < chars.len() {
            let at = |token: &str| {
                chars[i..]
                    .iter()
                    .copied()
                    .take(token.len())
                    .eq(token.chars())
            };
            match open {
                None if at("<!--") && !in_code[i] => {
                    open = Some((index + 1, i + 1));
                    i += 4;
                }
                Some(start) if at("-->") => {
                    spans.push((start, (index + 1, i + 3)));
                    open = None;
                    i += 3;
                }
                _ => i += 1,
            }
        }
    }
    spans.extend(open.map(|start| (start, (usize::MAX, usize::MAX))));
    spans
}
