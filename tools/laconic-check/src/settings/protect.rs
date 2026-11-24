// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Author `[protect]` exemption list and TOML-friendly value parsing for `laconic.toml` and
//! `.laconic-check`.
//!
//! Protected phrases exempt matching spans on a line from every lint rule without disabling
//! the rule elsewhere on the line or in the document. Phrases are precompiled once on parse,
//! and protected spans are computed once per physical line for O(log P) lookup per finding.

use std::collections::HashMap;
use std::str::Chars;

/// Precompiled protected phrase ready for whole-word matching.
#[derive(Debug, PartialEq)]
pub(crate) struct CompiledPhrase {
    pub(crate) text: String,
    chars: Vec<char>,
    exact: bool,
    starts_word: bool,
    ends_word: bool,
}

impl CompiledPhrase {
    pub(crate) fn new(text: String) -> Self {
        let raw_chars: Vec<char> = text.chars().collect();
        let exact = raw_chars.iter().any(|c| c.is_uppercase());
        let starts_word = raw_chars.first().is_some_and(|c| c.is_alphanumeric());
        let ends_word = raw_chars.last().is_some_and(|c| c.is_alphanumeric());
        let chars = if exact {
            raw_chars
        } else {
            raw_chars
                .into_iter()
                .map(|c| c.to_lowercase().next().unwrap_or(c))
                .collect()
        };
        Self {
            text,
            chars,
            exact,
            starts_word,
            ends_word,
        }
    }

    fn match_at(&self, text: &[char], lower: &[char], start: usize) -> Option<usize> {
        if self.starts_word && start > 0 && is_word_char(text, start - 1) {
            return None;
        }
        let mut at = start;
        for &p in &self.chars {
            let &c = text.get(at)?;
            if p == ' ' {
                if !c.is_whitespace() {
                    return None;
                }
                while text.get(at).is_some_and(|ch| ch.is_whitespace()) {
                    at += 1;
                }
                continue;
            }
            let same = if apostrophe(p) {
                apostrophe(c)
            } else if self.exact {
                c == p
            } else {
                lower[at] == p
            };
            if !same {
                return None;
            }
            at += 1;
        }
        if self.ends_word && is_word_char(text, at) {
            return None;
        }
        Some(at)
    }
}

/// Precompiled list of author-protected phrases exempt from lint findings on matching spans.
#[derive(Debug, Default, PartialEq)]
pub struct ProtectList {
    items: Vec<CompiledPhrase>,
}

impl ProtectList {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn phrases(&self) -> Vec<&str> {
        self.items.iter().map(|p| p.text.as_str()).collect()
    }

    pub(crate) fn push(&mut self, phrase: String) {
        self.items.push(CompiledPhrase::new(phrase));
    }

    pub(crate) fn items(&self) -> &[CompiledPhrase] {
        &self.items
    }
}

/// Per-line cache of merged protected `[start, end)` char ranges, avoiding repeated scanning
/// when one physical line produces many findings.
#[derive(Default)]
pub struct LineProtectCache {
    spans: HashMap<usize, Vec<(usize, usize)>>,
}

impl LineProtectCache {
    pub(crate) fn contains(
        &mut self,
        phrases: &[CompiledPhrase],
        line_no: usize,
        line: &str,
        column: usize,
    ) -> bool {
        if phrases.is_empty() || column == 0 {
            return false;
        }
        let ranges = self
            .spans
            .entry(line_no)
            .or_insert_with(|| protected_spans(phrases, line));
        contains_column(ranges, column)
    }
}

fn apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

fn is_word_char(text: &[char], at: usize) -> bool {
    let alphanumeric = |i: usize| text.get(i).is_some_and(|c| c.is_alphanumeric());
    text.get(at).is_some_and(|&c| {
        c.is_alphanumeric()
            || c == '-'
            || (apostrophe(c) && at > 0 && alphanumeric(at - 1) && alphanumeric(at + 1))
    })
}

/// Strips an unquoted `# ...` comment to the end of a single line, respecting quoted strings.
pub(crate) fn strip_comment(line: &str) -> &str {
    let mut in_quote: Option<char> = None;
    let mut escape = false;
    for (idx, c) in line.char_indices() {
        if let Some(q) = in_quote {
            if q == '"' && escape {
                escape = false;
            } else if q == '"' && c == '\\' {
                escape = true;
            } else if c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => in_quote = Some(c),
            '#' => return line[..idx].trim_end(),
            _ => {}
        }
    }
    line.trim_end()
}

fn decode_hex_scalar(chars: &mut Chars<'_>, digits: usize, raw: &str) -> Result<char, String> {
    let mut hex = String::with_capacity(digits);
    for _ in 0..digits {
        let Some(c) = chars.next() else {
            return Err(format!("incomplete unicode escape in '{raw}'"));
        };
        if !c.is_ascii_hexdigit() {
            return Err(format!("invalid unicode escape in '{raw}'"));
        }
        hex.push(c);
    }
    let code = u32::from_str_radix(&hex, 16).unwrap_or(u32::MAX);
    char::from_u32(code).ok_or_else(|| format!("invalid unicode scalar '\\u{hex}' in '{raw}'"))
}

fn decode_basic_escape(chars: &mut Chars<'_>, raw: &str) -> Result<char, String> {
    match chars.next() {
        Some('"') => Ok('"'),
        Some('\\') => Ok('\\'),
        Some('b') => Ok('\u{0008}'),
        Some('f') => Ok('\u{000C}'),
        Some('n') => Ok('\n'),
        Some('r') => Ok('\r'),
        Some('t') => Ok('\t'),
        Some('u') => decode_hex_scalar(chars, 4, raw),
        Some('U') => decode_hex_scalar(chars, 8, raw),
        Some(other) => Err(format!("unsupported escape '\\{other}' in '{raw}'")),
        None => Err(format!("unterminated escape in '{raw}'")),
    }
}

/// Parses one quoted TOML string at the start of `input`, returning `(decoded, remainder)`.
fn take_quoted<'a>(input: &'a str, ctx: &str) -> Result<(String, &'a str), String> {
    if let Some(rest) = input.strip_prefix('"') {
        let mut chars = rest.chars();
        let mut out = String::new();
        while let Some(c) = chars.next() {
            if c == '\\' {
                out.push(decode_basic_escape(&mut chars, ctx)?);
            } else if c == '"' {
                return Ok((out, chars.as_str()));
            } else {
                out.push(c);
            }
        }
        return Err(format!("unterminated string in '{ctx}'"));
    }
    let rest = input.strip_prefix('\'').unwrap();
    let Some((body, tail)) = rest.split_once('\'') else {
        return Err(format!("unterminated string in '{ctx}'"));
    };
    Ok((body.to_owned(), tail))
}

/// Decodes a single TOML basic (`"..."`) or literal (`'...'`) string, or returns a bare value
/// unchanged (after stripping any trailing unquoted `#` comment).
pub(crate) fn decode_value(raw: &str) -> Result<String, String> {
    let s = strip_comment(raw.trim()).trim();
    if s.starts_with('"') || s.starts_with('\'') {
        let (decoded, tail) = take_quoted(s, s)?;
        if !tail.trim().is_empty() {
            return Err(format!("unexpected trailing text after string in '{s}'"));
        }
        return Ok(decoded);
    }
    Ok(s.to_owned())
}

/// Parses a TOML string array (`["a", "b"]`) or legacy comma-separated bare list into decoded items.
pub(crate) fn parse_list(value: &str) -> Result<Vec<String>, String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') {
        return strip_comment(trimmed)
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(decode_value)
            .collect();
    }
    let mut rest = &trimmed[1..];
    let mut items = Vec::new();
    let mut need_comma = false;
    loop {
        rest = rest.trim_start();
        if rest.starts_with('#') {
            rest = rest.find('\n').map_or("", |i| &rest[i + 1..]);
            continue;
        }
        if let Some(after) = rest.strip_prefix(']') {
            if !strip_comment(after).trim().is_empty() {
                return Err(format!(
                    "unexpected trailing text after list in '{trimmed}'"
                ));
            }
            return Ok(items);
        }
        if rest.is_empty() {
            return Err(format!("unterminated list '{trimmed}'"));
        }
        if let Some(after) = rest.strip_prefix(',') {
            if !need_comma {
                return Err(format!("unexpected ',' in '{trimmed}'"));
            }
            rest = after;
            need_comma = false;
            continue;
        }
        if need_comma {
            return Err(format!("expected ',' between list items in '{trimmed}'"));
        }
        if rest.starts_with('"') || rest.starts_with('\'') {
            let (item, after) = take_quoted(rest, trimmed)?;
            items.push(item);
            rest = after;
        } else {
            let end = rest.find([',', ']', '\n', '#']).unwrap_or(rest.len());
            items.push(rest[..end].trim().to_owned());
            rest = &rest[end..];
        }
        need_comma = true;
    }
}

/// Validates and normalises an already-decoded protected phrase (never unquotes a second time).
pub(crate) fn normalise_phrase_text(decoded: &str) -> Result<String, String> {
    let collapsed: Vec<&str> = decoded.split_whitespace().collect();
    if collapsed.is_empty() {
        return Err("protected phrase cannot be empty".into());
    }
    let joined = collapsed.join(" ");
    if joined.chars().any(|c| c.is_control() || c == '`') {
        return Err(format!("'{joined}' is not a valid protected phrase"));
    }
    Ok(joined)
}

/// Computes sorted, merged `[start, end)` char ranges of all protected phrases on `line`.
pub(crate) fn protected_spans(phrases: &[CompiledPhrase], line: &str) -> Vec<(usize, usize)> {
    let text: Vec<char> = line.chars().collect();
    let lower: Vec<char> = text
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut spans = Vec::new();
    for phrase in phrases {
        for start in 0..text.len() {
            if let Some(end) = phrase.match_at(&text, &lower, start) {
                spans.push((start, end));
            }
        }
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
    for (s, e) in spans {
        if let Some(last) = merged.last_mut()
            && s <= last.1
        {
            last.1 = last.1.max(e);
            continue;
        }
        merged.push((s, e));
    }
    merged
}

/// True when the 1-based `column` falls inside any merged `[start, end)` range (`O(log P)`).
pub(crate) fn contains_column(spans: &[(usize, usize)], column: usize) -> bool {
    if column == 0 || spans.is_empty() {
        return false;
    }
    let target = column - 1;
    let idx = spans.partition_point(|&(start, _)| start <= target);
    idx > 0 && target < spans[idx - 1].1
}

/// Convenience wrapper for single-finding checks; multi-finding loops use `LineProtectCache`.
pub(crate) fn is_protected(phrases: &[CompiledPhrase], line: &str, column: usize) -> bool {
    if phrases.is_empty() || column == 0 {
        return false;
    }
    contains_column(&protected_spans(phrases, line), column)
}
