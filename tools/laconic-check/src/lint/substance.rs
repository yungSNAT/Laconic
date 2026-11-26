// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Opt-in deterministic substance-heuristic rules and chat-mode filler checks.
//!
//! Each rule is advisory by default and runs only when opt-in rules are enabled (via `--only`,
//! a register profile or a project file). Matching runs on masked prose so code, URLs and
//! quoted spans never trigger, whilst sentence-level anchor checks inspect the raw paragraph
//! so Markdown links and bare URLs still count as citations.

use super::{Finding, RuleId, lower_exact};

mod lists;
use lists::{
    AI_RESIDUE, CHAT_FILLER, MAGNITUDE_SUFFIXES, PLACEHOLDER_OPENERS, SUPERLATIVES,
    UNNAMED_EVIDENCE, VAGUE_MAGNITUDES,
};

struct ParagraphView<'a> {
    masked: &'a [char],
    lower: String,
    lower_chars: Vec<char>,
    raw: &'a [char],
    raw_lower: Vec<char>,
}

/// Runs all opt-in substance and chat-filler rules on a single masked paragraph.
pub(super) fn check(masked: &[char], raw: &[char], findings: &mut Vec<Finding>) {
    let masked_str: String = masked.iter().collect();
    let lower = lower_exact(&masked_str);
    let lower_chars: Vec<char> = lower.chars().collect();
    let raw_str: String = raw.iter().collect();
    let view = ParagraphView {
        masked,
        lower,
        lower_chars,
        raw,
        raw_lower: lower_exact(&raw_str).chars().collect(),
    };

    check_magnitudes(&view, findings);
    check_phrases(
        &view,
        SUPERLATIVES,
        RuleId::UnattributedSuperlative,
        Some(has_attribution_anchor),
        findings,
        |phrase| {
            format!(
                "unattributed superlative or comparative claim '{phrase}' (advisory); name the baseline and source, or remove the claim"
            )
        },
    );
    check_phrases(
        &view,
        UNNAMED_EVIDENCE,
        RuleId::UnnamedEvidence,
        Some(has_evidence_anchor),
        findings,
        |phrase| {
            format!(
                "appeal to unnamed evidence '{phrase}' (advisory); name the study, author or source, or cut the appeal"
            )
        },
    );
    check_ai_placeholders(&view, findings);
    check_phrases(
        &view,
        CHAT_FILLER,
        RuleId::ChatFiller,
        None,
        findings,
        |phrase| {
            format!(
                "architectural filler '{phrase}' (advisory); name the concrete check, module or interface"
            )
        },
    );
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '\'' || c == '\u{2019}'
}

fn whole_word_at(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !is_word_char(chars[start - 1]);
    let after_ok = start + len >= chars.len() || !is_word_char(chars[start + len]);
    before_ok && after_ok
}

fn is_decimal_dot(chars: &[char], i: usize) -> bool {
    chars[i] == '.'
        && i > 0
        && chars[i - 1].is_ascii_digit()
        && chars.get(i + 1).is_some_and(char::is_ascii_digit)
}

/// Finds the `[start, end)` char range of the sentence containing `col_idx` in `raw`.
fn sentence_bounds(raw: &[char], col_idx: usize) -> (usize, usize) {
    let pos = col_idx.min(raw.len());
    let mut start = 0;
    for i in (0..pos).rev() {
        if matches!(raw[i], '.' | '!' | '?')
            && raw.get(i + 1).is_some_and(|c| c.is_whitespace())
            && !is_decimal_dot(raw, i)
        {
            start = i + 2;
            break;
        }
    }
    let mut end = raw.len();
    for i in pos..raw.len() {
        if matches!(raw[i], '.' | '!' | '?')
            && (i + 1 == raw.len() || raw[i + 1].is_whitespace())
            && !is_decimal_dot(raw, i)
        {
            end = i + 1;
            break;
        }
    }
    (start, end)
}

fn sentence_slice(view: &ParagraphView<'_>, col_idx: usize) -> (String, String) {
    let (s, e) = sentence_bounds(view.raw, col_idx);
    let orig: String = view.raw[s..e].iter().collect();
    let low: String = view.raw_lower[s..e.max(s)].iter().collect();
    (orig, low)
}

/// True when `sent` has a Markdown link, bare URL, bracketed numeric citation (`[1]`)
/// or parenthesised 4-digit year (`(2025)`).
fn has_citation(sent: &str) -> bool {
    if sent.contains("](") || sent.contains("http://") || sent.contains("https://") {
        return true;
    }
    let bytes = sent.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'[' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 && j - (i + 1) <= 3 && bytes.get(j) == Some(&b']') {
                return true;
            }
        }
        if bytes[i] == b'('
            && i + 5 < bytes.len()
            && (bytes[i + 1] == b'1' || bytes[i + 1] == b'2')
            && bytes[i + 2..i + 5].iter().all(u8::is_ascii_digit)
            && bytes[i + 5] == b')'
        {
            return true;
        }
    }
    false
}

fn has_from_to_digits(low: &str) -> bool {
    let Some(from_pos) = low.find("from ") else {
        return false;
    };
    let rest = &low[from_pos + 5..];
    let Some(to_pos) = rest.find(" to ") else {
        return false;
    };
    rest[..to_pos].bytes().any(|b| b.is_ascii_digit())
        && rest[to_pos + 4..].bytes().any(|b| b.is_ascii_digit())
}

fn has_left_boundary(low: &str, pos: usize) -> bool {
    pos == 0
        || low[..pos]
            .chars()
            .next_back()
            .is_some_and(|c| !is_word_char(c))
}

fn contains_word_cue(low: &str, cue: &str) -> bool {
    low.match_indices(cue)
        .any(|(pos, _)| has_left_boundary(low, pos))
}

fn has_magnitude_anchor(orig: &str, low: &str) -> bool {
    if has_citation(orig) || has_from_to_digits(low) {
        return true;
    }
    let cues = [
        "according to ",
        "measured in ",
        "measured by ",
        "benchmark",
        "survey",
        "out of ",
        "per ",
    ];
    if cues.iter().any(|cue| contains_word_cue(low, cue)) {
        return true;
    }
    for prep in ["across ", "over ", "of ", "in 19", "in 20"] {
        for (pos, _) in low.match_indices(prep) {
            if !has_left_boundary(low, pos) {
                continue;
            }
            let tail = &low[pos + prep.len()..];
            if prep.starts_with("in ") || tail.as_bytes().first().is_some_and(u8::is_ascii_digit) {
                return true;
            }
        }
    }
    false
}

fn has_attribution_anchor(orig: &str, low: &str) -> bool {
    has_citation(orig)
        || [
            "according to ",
            "measured by ",
            "reported by ",
            "benchmarked against ",
            "compared with ",
            "compared to ",
        ]
        .iter()
        .any(|cue| low.contains(cue))
}

fn has_evidence_anchor(orig: &str, low: &str) -> bool {
    if has_citation(orig) || low.contains("according to ") {
        return true;
    }
    for prep in [" by ", " from ", " at "] {
        for (pos, _) in low.match_indices(prep) {
            let char_idx = low[..pos + prep.len()].chars().count();
            let after: String = orig.chars().skip(char_idx).collect();
            let trimmed = after.strip_prefix("the ").unwrap_or(&after);
            if trimmed.chars().next().is_some_and(char::is_uppercase) {
                return true;
            }
        }
    }
    false
}

fn check_magnitudes(view: &ParagraphView<'_>, findings: &mut Vec<Finding>) {
    check_phrases(
        view,
        VAGUE_MAGNITUDES,
        RuleId::UnanchoredMagnitude,
        Some(has_magnitude_anchor),
        findings,
        |phrase| {
            format!(
                "unanchored magnitude '{phrase}' (advisory); add a denominator, timeframe or source"
            )
        },
    );
    check_numeric_multipliers(view, findings);
}

fn check_numeric_multipliers(view: &ParagraphView<'_>, findings: &mut Vec<Finding>) {
    let mut i = 0;
    while i < view.masked.len() {
        if !view.masked[i].is_ascii_digit() || (i > 0 && is_word_char(view.masked[i - 1])) {
            i += 1;
            continue;
        }
        let start = i;
        while i < view.masked.len() && (view.masked[i].is_ascii_digit() || view.masked[i] == '.') {
            i += 1;
        }
        if !matches!(view.masked.get(i), Some('x' | 'X' | '%'))
            || view.masked.get(i + 1) != Some(&' ')
        {
            continue;
        }
        let word_start = i + 2;
        let mut word_end = word_start;
        while word_end < view.lower_chars.len() && view.lower_chars[word_end].is_ascii_alphabetic()
        {
            word_end += 1;
        }
        let word: String = view.lower_chars[word_start..word_end].iter().collect();
        if !MAGNITUDE_SUFFIXES.contains(&word.as_str()) {
            continue;
        }
        let (orig, low) = sentence_slice(view, start);
        if !has_magnitude_anchor(&orig, &low) {
            let claim: String = view.masked[start..word_end].iter().collect();
            findings.push(Finding {
                line: 0,
                column: start + 1,
                rule: RuleId::UnanchoredMagnitude.into(),
                message: format!(
                    "unanchored magnitude '{claim}' (advisory); add a baseline, denominator or source"
                ),
                fix: None,
            });
        }
        i = word_end;
    }
}

fn check_phrases(
    view: &ParagraphView<'_>,
    phrases: &[&str],
    rule: RuleId,
    anchor: Option<fn(&str, &str) -> bool>,
    findings: &mut Vec<Finding>,
    mut message: impl FnMut(&str) -> String,
) {
    for &phrase in phrases {
        let plen = phrase.chars().count();
        let mut from = 0;
        // Counted from the last match on, so a paragraph is read once per phrase.
        let mut col_idx = 0;
        while let Some(pos) = view.lower[from..].find(phrase) {
            col_idx += view.lower[from..from + pos].chars().count();
            if whole_word_at(&view.lower_chars, col_idx, plen)
                && anchor.is_none_or(|f| {
                    let (orig, low) = sentence_slice(view, col_idx);
                    !f(&orig, &low)
                })
            {
                findings.push(Finding {
                    line: 0,
                    column: col_idx + 1,
                    rule: rule.into(),
                    message: message(phrase),
                    fix: None,
                });
            }
            from += pos + phrase.len();
            col_idx += plen;
        }
    }
}

/// Whether `raw[at]` follows an odd run of backslashes, which makes it literal text.
fn is_escaped(raw: &[char], at: usize) -> bool {
    raw[..at].iter().rev().take_while(|&&c| c == '\\').count() % 2 == 1
}

/// The index of the `)` ending an inline link destination that opens at `raw[close + 1]`, when
/// `raw[close]` is an unescaped `]`. The destination has no space unless a quoted title
/// follows it, so `[TODO](see below)` is no link.
fn link_tail_end(raw: &[char], close: usize) -> Option<usize> {
    if raw.get(close) != Some(&']') || is_escaped(raw, close) || raw.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = close + 2 + raw[close + 2..].iter().position(|&c| c == ')')?;
    let inner: String = raw[close + 2..end].iter().collect();
    let inner = inner.trim();
    let valid = match inner.split_once(char::is_whitespace) {
        None => true,
        Some((_, title)) => {
            let title = title.trim_start();
            title.len() >= 2
                && ((title.starts_with('"') && title.ends_with('"'))
                    || (title.starts_with('\'') && title.ends_with('\'')))
        }
    };
    valid.then_some(end)
}

fn is_markdown_link_tail(raw: &[char], close: usize) -> bool {
    link_tail_end(raw, close).is_some()
}

fn markdown_link_label_ranges(raw: &[char]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if let Some(end) = link_tail_end(raw, i) {
            // The nearest unescaped bracket or line break before `]`; a `[` opens the label.
            if let Some(open) = (0..i)
                .rev()
                .find(|&pos| {
                    raw[pos] == '\n'
                        || ((raw[pos] == '[' || raw[pos] == ']') && !is_escaped(raw, pos))
                })
                .filter(|&pos| raw[pos] == '[')
            {
                ranges.push((open, i));
            }
            i = end + 1;
            continue;
        }
        i += 1;
    }
    ranges
}

fn check_ai_placeholders(view: &ParagraphView<'_>, findings: &mut Vec<Finding>) {
    check_phrases(
        view,
        AI_RESIDUE,
        RuleId::AiPlaceholder,
        None,
        findings,
        |p| {
            format!("AI residue phrase '{p}' (advisory); remove self-reference from finished prose")
        },
    );
    let mut skip_ranges = markdown_link_label_ranges(view.raw);
    for opener in PLACEHOLDER_OPENERS {
        let (mut from, mut counted, mut col_idx) = (0, 0, 0);
        while let Some(pos) = view.lower[from..].find(opener) {
            col_idx += view.lower[counted..from + pos].chars().count();
            counted = from + pos;
            if let Some(close) = view.lower_chars[col_idx..].iter().position(|&c| c == ']') {
                let end = col_idx + close;
                if is_markdown_link_tail(view.raw, end) {
                    from += pos + opener.len();
                    continue;
                }
                skip_ranges.push((col_idx, end));
                let span: String = view.masked[col_idx..=end].iter().collect();
                findings.push(Finding {
                    line: 0,
                    column: col_idx + 1,
                    rule: RuleId::AiPlaceholder.into(),
                    message: format!(
                        "leftover placeholder '{span}' (advisory); fill it in or mark the gap explicitly"
                    ),
                    fix: None,
                });
            }
            from += pos + opener.len();
        }
    }
    // Sorted, the ranges are passed once: a long paragraph of links stays linear.
    skip_ranges.sort_unstable();
    let mut ranges = skip_ranges.iter().peekable();
    let mut i = 0;
    while i < view.masked.len() {
        while ranges.next_if(|&&(_, end)| end < i).is_some() {}
        if let Some(&&(_, end)) = ranges.peek().filter(|&&&(start, _)| start <= i) {
            i = end + 1;
            continue;
        }
        if view.masked[i] == 'T' && (i == 0 || !is_word_char(view.masked[i - 1])) {
            let token = if view.masked.get(i..i + 4) == Some(&['T', 'O', 'D', 'O'])
                && (i + 4 == view.masked.len() || !is_word_char(view.masked[i + 4]))
            {
                Some(("TODO", 4))
            } else if view.masked.get(i..i + 3) == Some(&['T', 'B', 'D'])
                && (i + 3 == view.masked.len() || !is_word_char(view.masked[i + 3]))
            {
                Some(("TBD", 3))
            } else {
                None
            };
            if let Some((word, len)) = token {
                findings.push(Finding {
                    line: 0,
                    column: i + 1,
                    rule: RuleId::AiPlaceholder.into(),
                    message: format!(
                        "leftover placeholder '{word}' (advisory); replace with a fact or explicit gap note"
                    ),
                    fix: None,
                });
                i += len;
                continue;
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests;
