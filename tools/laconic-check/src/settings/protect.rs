// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Author `[protect]` exemption list and TOML-friendly value parsing for `laconic.toml` and
//! `.laconic-check`.
//!
//! Protected phrases exempt matching spans on a line from every lint rule without disabling
//! the rule elsewhere on the line or in the document.

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

/// Strips a pair of matching double or single quotes from a TOML string literal.
pub(crate) fn unquote(raw: &str) -> Result<String, String> {
    let s = raw.trim();
    if let Some(inner) = s.strip_prefix('"') {
        let Some(body) = inner.strip_suffix('"') else {
            return Err(format!("unterminated string '{s}'"));
        };
        let mut out = String::with_capacity(body.len());
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => return Err(format!("unterminated escape in '{s}'")),
                }
            } else {
                out.push(c);
            }
        }
        return Ok(out);
    }
    if let Some(inner) = s.strip_prefix('\'') {
        let Some(body) = inner.strip_suffix('\'') else {
            return Err(format!("unterminated string '{s}'"));
        };
        return Ok(body.to_owned());
    }
    Ok(s.to_owned())
}

/// Parses a TOML string array (`["a", "b"]`) or comma-separated bare list into raw items.
pub(crate) fn parse_list(value: &str) -> Result<Vec<String>, String> {
    let trimmed = value.trim();
    if !trimmed.starts_with('[') {
        return trimmed
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(unquote)
            .collect();
    }
    let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Err(format!("unterminated list '{trimmed}'"));
    };
    let mut items = Vec::new();
    let mut chars = inner.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() || c == ',' {
            chars.next();
            continue;
        }
        if c == '#' {
            for ch in chars.by_ref() {
                if ch == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = chars.next().unwrap();
            let mut item = String::new();
            let mut closed = false;
            while let Some(ch) = chars.next() {
                if quote == '"' && ch == '\\' {
                    match chars.next() {
                        Some('"') => item.push('"'),
                        Some('\\') => item.push('\\'),
                        Some(other) => {
                            item.push('\\');
                            item.push(other);
                        }
                        None => return Err(format!("unterminated escape in '{trimmed}'")),
                    }
                } else if ch == quote {
                    closed = true;
                    break;
                } else {
                    item.push(ch);
                }
            }
            if !closed {
                return Err(format!("unterminated string in '{trimmed}'"));
            }
            items.push(item);
        } else {
            let mut token = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == ',' || ch == '\n' {
                    break;
                }
                token.push(ch);
                chars.next();
            }
            items.push(token.trim().to_owned());
        }
    }
    Ok(items)
}

/// Validates and normalises one protected phrase from `[protect]`.
pub(crate) fn normalise_phrase(raw: &str) -> Result<String, String> {
    let text = unquote(raw)?;
    let collapsed: Vec<&str> = text.split_whitespace().collect();
    if collapsed.is_empty() {
        return Err("protected phrase cannot be empty".into());
    }
    let joined = collapsed.join(" ");
    if joined.chars().any(|c| c.is_control() || c == '`') {
        return Err(format!("'{joined}' is not a valid protected phrase"));
    }
    Ok(joined)
}

/// Matches `phrase` at character index `start` in `text`, returning the exclusive end index.
fn match_at(text: &[char], start: usize, phrase: &str) -> Option<usize> {
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let starts_word = phrase_chars.first().is_some_and(|c| c.is_alphanumeric());
    if starts_word && start > 0 && is_word_char(text, start - 1) {
        return None;
    }
    let exact = phrase_chars.iter().any(|c| c.is_uppercase());
    let mut at = start;
    for &p in &phrase_chars {
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
        } else if exact {
            c == p
        } else {
            c.to_lowercase().eq(p.to_lowercase())
        };
        if !same {
            return None;
        }
        at += 1;
    }
    let ends_word = phrase_chars.last().is_some_and(|c| c.is_alphanumeric());
    if ends_word && is_word_char(text, at) {
        return None;
    }
    Some(at)
}

/// True when the 1-based `column` on `line` falls inside any occurrence of a protected phrase.
pub(crate) fn is_protected(phrases: &[String], line: &str, column: usize) -> bool {
    if phrases.is_empty() || column == 0 {
        return false;
    }
    let target = column - 1;
    let chars: Vec<char> = line.chars().collect();
    for phrase in phrases {
        for start in 0..chars.len() {
            if start > target {
                break;
            }
            if let Some(end) = match_at(&chars, start, phrase)
                && target < end
            {
                return true;
            }
        }
    }
    false
}
