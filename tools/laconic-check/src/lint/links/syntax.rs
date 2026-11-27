// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Markdown syntax the link rules read: code spans, link targets, reference definitions
//! and HTML comments.

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
pub(super) fn closing(chars: &[char], from: usize, close: char, stop: &[char]) -> Option<usize> {
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
/// so a paragraph of unclosed links is read in linear time: a bare destination ends without a
/// link at the next `](`, and an angle-bracket one at a `<` or a line break.
pub(super) fn inline_target(chars: &[char], from: usize) -> Option<(String, usize, usize)> {
    let mut i = from;
    while chars.get(i).is_some_and(|c| c.is_whitespace()) {
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
            ']' if chars.get(i + 1) == Some(&'(') && !is_escaped(chars, i) => return None,
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
    while chars.get(start).is_some_and(|c| c.is_whitespace()) {
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
