// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The `reference-shape` rule: offline structural checks for explicitly labelled ISBN-10,
//! ISBN-13 and modern arXiv identifiers.

use super::links::code_spans;
use super::{Finding, RuleId};

/// Scans a raw paragraph for malformed labelled ISBN and arXiv identifiers, skipping inline
/// code spans.
pub(super) fn check(raw: &[char], findings: &mut Vec<Finding>) {
    let spans = code_spans(raw);
    let mut start = 0;
    for (code_start, code_end) in spans.into_iter().chain([(raw.len(), raw.len())]) {
        scan_segment(raw, start, code_start, findings);
        start = code_end;
    }
}

fn scan_segment(raw: &[char], start: usize, limit: usize, findings: &mut Vec<Finding>) {
    let mut i = start;
    while i < limit {
        let boundary = i == 0 || !raw[i - 1].is_ascii_alphanumeric();
        if boundary {
            if let Some(next) = scan_isbn(raw, i, limit, findings) {
                i = next;
                continue;
            }
            if let Some(next) = scan_arxiv(raw, i, limit, findings) {
                i = next;
                continue;
            }
        }
        i += 1;
    }
}

fn eq_ascii_ci(raw: &[char], at: usize, limit: usize, needle: &[u8]) -> bool {
    at + needle.len() <= limit
        && raw[at..at + needle.len()]
            .iter()
            .zip(needle)
            .all(|(&c, &b)| c.is_ascii() && (c as u8).eq_ignore_ascii_case(&b))
}

fn skip_spaces(raw: &[char], mut i: usize, limit: usize) -> usize {
    while i < limit && matches!(raw[i], ' ' | '\t') {
        i += 1;
    }
    i
}

fn take_digits(raw: &[char], mut i: usize, limit: usize) -> usize {
    while i < limit && raw[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn is_standalone_x(raw: &[char], i: usize, limit: usize) -> bool {
    matches!(raw[i], 'X' | 'x') && !(i + 1 < limit && raw[i + 1].is_ascii_alphabetic())
}

fn is_isbn_sep(c: char) -> bool {
    c == ':' || c == ' ' || c == '\t'
}

fn parse_isbn_declared(raw: &[char], j: usize, limit: usize) -> (Option<usize>, usize) {
    let is_ver = j + 3 <= limit
        && matches!(raw[j], '-' | ' ' | '\t')
        && (eq_ascii_ci(raw, j + 1, limit, b"10") || eq_ascii_ci(raw, j + 1, limit, b"13"))
        && (j + 3 == limit || is_isbn_sep(raw[j + 3]));
    if is_ver {
        (Some(if raw[j + 2] == '0' { 10 } else { 13 }), j + 3)
    } else {
        (None, j)
    }
}

fn skip_isbn_separator(raw: &[char], start: usize, limit: usize) -> Option<usize> {
    let mut j = skip_spaces(raw, start, limit);
    if j < limit && raw[j] == ':' {
        j = skip_spaces(raw, j + 1, limit);
    }
    (j > start).then_some(j)
}

fn scan_isbn(
    raw: &[char],
    start: usize,
    limit: usize,
    findings: &mut Vec<Finding>,
) -> Option<usize> {
    if !eq_ascii_ci(raw, start, limit, b"isbn") {
        return None;
    }
    let (declared, after_ver) = parse_isbn_declared(raw, start + 4, limit);
    let Some(j) = skip_isbn_separator(raw, after_ver, limit) else {
        return Some(after_ver);
    };
    if j >= limit || !raw[j].is_ascii_digit() {
        return Some(j);
    }
    let (digits, end) = read_isbn_digits(raw, j, limit);
    if digits.len() >= 8
        && let Some(reason) = validate_isbn(&digits, declared)
    {
        let snippet: String = raw[j..end].iter().collect();
        findings.push(finding(start + 1, &snippet, &reason));
    }
    Some(end)
}

fn can_bridge_isbn(raw: &[char], k: usize, limit: usize, has_hyphen: bool, digits: &[u8]) -> bool {
    let sep = raw[k] == '-' || (raw[k] == ' ' && !has_hyphen && !is_valid_isbn(digits));
    sep && k + 1 < limit && (raw[k + 1].is_ascii_digit() || is_standalone_x(raw, k + 1, limit))
}

fn read_isbn_digits(raw: &[char], start: usize, limit: usize) -> (Vec<u8>, usize) {
    let mut digits = Vec::new();
    let mut has_hyphen = false;
    let mut k = start;
    let mut end = start;
    while k < limit {
        if raw[k].is_ascii_digit() {
            digits.push(raw[k] as u8);
            k += 1;
            end = k;
        } else if is_standalone_x(raw, k, limit) {
            digits.push(b'X');
            k += 1;
            end = k;
        } else if can_bridge_isbn(raw, k, limit, has_hyphen, &digits) {
            has_hyphen |= raw[k] == '-';
            k += 1;
        } else {
            break;
        }
    }
    (digits, end)
}

fn is_valid_isbn(digits: &[u8]) -> bool {
    match digits.len() {
        10 => isbn10_ok(digits),
        13 => (digits.starts_with(b"978") || digits.starts_with(b"979")) && isbn13_ok(digits),
        _ => false,
    }
}

fn isbn10_ok(digits: &[u8]) -> bool {
    if digits[..9].contains(&b'X') {
        return false;
    }
    let sum: usize = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| (10 - i) * if d == b'X' { 10 } else { usize::from(d - b'0') })
        .sum();
    sum.is_multiple_of(11)
}

fn isbn13_ok(digits: &[u8]) -> bool {
    if digits.contains(&b'X') {
        return false;
    }
    let sum: usize = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| usize::from(d - b'0') * if i.is_multiple_of(2) { 1 } else { 3 })
        .sum();
    sum.is_multiple_of(10)
}

fn validate_isbn(digits: &[u8], declared: Option<usize>) -> Option<String> {
    match digits.len() {
        10 => validate_isbn10(digits, declared),
        13 => validate_isbn13(digits, declared),
        n => Some(format!("ISBN has {n} characters; expected 10 or 13")),
    }
}

fn validate_isbn10(digits: &[u8], declared: Option<usize>) -> Option<String> {
    if declared == Some(13) {
        Some("ISBN-13 label has 10 characters; expected 13".into())
    } else if !isbn10_ok(digits) {
        Some("ISBN-10 checksum fails".into())
    } else {
        None
    }
}

fn validate_isbn13(digits: &[u8], declared: Option<usize>) -> Option<String> {
    if declared == Some(10) {
        Some("ISBN-10 label has 13 characters; expected 10".into())
    } else if !digits.starts_with(b"978") && !digits.starts_with(b"979") {
        Some("ISBN-13 must start with 978 or 979".into())
    } else if !isbn13_ok(digits) {
        Some("ISBN-13 checksum fails".into())
    } else {
        None
    }
}

fn parse_arxiv_prefix(raw: &[char], j: usize, limit: usize) -> Option<usize> {
    if eq_ascii_ci(raw, j, limit, b".org/abs/") || eq_ascii_ci(raw, j, limit, b".org/pdf/") {
        Some(j + 9)
    } else if j < limit && raw[j] == ':' {
        Some(skip_spaces(raw, j + 1, limit))
    } else if j < limit && matches!(raw[j], ' ' | '\t') {
        Some(skip_spaces(raw, j, limit))
    } else {
        None
    }
}

fn read_arxiv_version(raw: &[char], j: usize, limit: usize) -> (Option<&[char]>, usize) {
    if j + 1 < limit && matches!(raw[j], 'v' | 'V') && raw[j + 1].is_ascii_digit() {
        let end = take_digits(raw, j + 1, limit);
        (Some(&raw[j + 1..end]), end)
    } else {
        (None, j)
    }
}

fn scan_arxiv(
    raw: &[char],
    start: usize,
    limit: usize,
    findings: &mut Vec<Finding>,
) -> Option<usize> {
    if !eq_ascii_ci(raw, start, limit, b"arxiv") {
        return None;
    }
    let Some(cand_start) = parse_arxiv_prefix(raw, start + 5, limit) else {
        return Some(start + 5);
    };
    let yymm_end = take_digits(raw, cand_start, limit);
    if yymm_end == cand_start
        || yymm_end + 1 >= limit
        || raw[yymm_end] != '.'
        || !raw[yymm_end + 1].is_ascii_digit()
    {
        return Some(yymm_end);
    }
    let seq_start = yymm_end + 1;
    let seq_end = take_digits(raw, seq_start, limit);
    if yymm_end - cand_start < 3 && seq_end - seq_start < 3 {
        return Some(seq_end);
    }
    let (ver, end) = read_arxiv_version(raw, seq_end, limit);
    if let Some(reason) = validate_arxiv(&raw[cand_start..yymm_end], &raw[seq_start..seq_end], ver)
    {
        let snippet: String = raw[cand_start..end].iter().collect();
        findings.push(finding(start + 1, &snippet, &reason));
    }
    Some(end)
}

fn validate_arxiv(yymm: &[char], seq: &[char], ver: Option<&[char]>) -> Option<String> {
    if yymm.len() != 4 {
        return Some("arXiv YYMM prefix must have 4 digits".into());
    }
    let yy = (yymm[0] as u8 - b'0') * 10 + (yymm[1] as u8 - b'0');
    let mm = (yymm[2] as u8 - b'0') * 10 + (yymm[3] as u8 - b'0');
    if !(1..=12).contains(&mm) {
        return Some(format!("arXiv month '{mm:02}' is not 01 to 12"));
    }
    if (yy, mm) < (7, 4) {
        return Some("arXiv YYMM.NNNN identifiers start at 0704 (April 2007)".into());
    }
    let want = if (yy, mm) < (15, 1) { 4 } else { 5 };
    if seq.len() != want {
        return Some(if want == 4 {
            "arXiv identifiers before 1501 use a 4-digit sequence number".into()
        } else {
            "arXiv identifiers from 1501 onward use a 5-digit sequence number".into()
        });
    }
    if seq.iter().all(|&c| c == '0') {
        return Some("arXiv sequence numbers start at 1".into());
    }
    if ver.is_some_and(|v| v.iter().all(|&c| c == '0')) {
        return Some("arXiv version numbers start at v1".into());
    }
    None
}

fn finding(column: usize, snippet: &str, reason: &str) -> Finding {
    Finding {
        line: 0,
        column,
        rule: RuleId::ReferenceShape.into(),
        message: format!(
            "'{snippet}': {reason} (advisory); check the identifier against the source"
        ),
        fix: None,
    }
}

#[cfg(test)]
mod tests;
