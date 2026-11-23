// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Helpers for `unglossed-term`: unfamiliar uppercase acronyms used without an earlier gloss.

/// Common 3-to-8 letter uppercase acronyms that readers are expected to recognise without a
/// gloss. Two-letter acronyms such as `AI`, `CI`, `ID`, `OS`, `PR`, `UI` and `UX` are already
/// below the 3-letter threshold. Kept sorted for binary search.
const COMMON_ACRONYMS: &[&str] = &[
    "ABN", "ACN", "ACT", "AEDT", "AEST", "API", "AST", "ATO", "AUD", "AWST", "BAS", "CBD", "CEO",
    "CFO", "CLI", "CPU", "CSS", "CSV", "CTO", "DNA", "DNS", "DOM", "EOF", "EST", "FAQ", "FTP",
    "GDP", "GIF", "GMT", "GPS", "GPU", "GST", "GUI", "HTML", "HTTP", "HTTPS", "IDE", "ISBN",
    "ISSN", "JSON", "JWT", "LAN", "LLM", "LSP", "MCP", "MIME", "NPM", "NSW", "NZD", "OCR", "PDF",
    "PIN", "PNG", "PST", "QLD", "RAM", "REST", "RGB", "RNA", "RPC", "RSS", "SDK", "SHA", "SIM",
    "SMS", "SQL", "SSH", "SSL", "SSO", "SVG", "TAS", "TCP", "TFN", "TLS", "TOML", "UDP", "URI",
    "URL", "USA", "USB", "USD", "UTC", "UTF", "UUID", "VAT", "VIC", "VIP", "VPN", "WASM", "XML",
    "XSS", "YAML", "ZIP",
];

/// True when `s` is a canonical uppercase Roman numeral (`I`..`MMMCMXCIX`).
fn is_roman_numeral(mut s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    const GROUPS: &[&[&str]] = &[
        &["MMM", "MM", "M"],
        &["CM", "DCCC", "DCC", "DC", "D", "CD", "CCC", "CC", "C"],
        &["XC", "LXXX", "LXX", "LX", "L", "XL", "XXX", "XX", "X"],
        &["IX", "VIII", "VII", "VI", "V", "IV", "III", "II", "I"],
    ];
    for group in GROUPS {
        if let Some(rest) = group.iter().find_map(|prefix| s.strip_prefix(prefix)) {
            s = rest;
        }
    }
    s.is_empty()
}

/// Returns the uppercase base of a candidate acronym of 3 to 8 ASCII capitals (allowing a
/// trailing plural `s`, as in `QBRs`).
pub(super) fn candidate(word: &str) -> Option<&str> {
    let base = word.strip_suffix('s').unwrap_or(word);
    ((3..=8).contains(&base.len()) && base.bytes().all(|b| b.is_ascii_uppercase())).then_some(base)
}

/// True when `acronym` is in the common allowlist or is a canonical Roman numeral.
pub(super) fn is_exempt(acronym: &str) -> bool {
    COMMON_ACRONYMS.binary_search(&acronym).is_ok() || is_roman_numeral(acronym)
}

/// True when the word at `masked[start..end]` sits next to another all-uppercase word
/// separated only by spaces or tabs, as in an all-caps heading or warning banner.
pub(super) fn in_all_caps_run(masked: &[char], start: usize, end: usize) -> bool {
    adjacent_all_caps_before(masked, start) || adjacent_all_caps_after(masked, end)
}

fn adjacent_all_caps_before(masked: &[char], start: usize) -> bool {
    let mut i = start;
    while i > 0 && matches!(masked[i - 1], ' ' | '\t') {
        i -= 1;
    }
    if i == start || i == 0 || !masked[i - 1].is_alphabetic() {
        return false;
    }
    let word_end = i;
    while i > 0 && masked[i - 1].is_alphabetic() {
        i -= 1;
    }
    is_caps_word(&masked[i..word_end])
}

fn adjacent_all_caps_after(masked: &[char], end: usize) -> bool {
    let mut i = end;
    while i < masked.len() && matches!(masked[i], ' ' | '\t') {
        i += 1;
    }
    if i == end || i >= masked.len() || !masked[i].is_alphabetic() {
        return false;
    }
    let word_start = i;
    while i < masked.len() && masked[i].is_alphabetic() {
        i += 1;
    }
    is_caps_word(&masked[word_start..i])
}

fn is_caps_word(chars: &[char]) -> bool {
    chars.len() >= 2 && chars.iter().all(|c| c.is_ascii_uppercase())
}

/// True when `after` (immediately following an acronym) opens a parenthesised gloss such as
/// ` (Quarterly Business Review)` whose words spell `acronym`.
pub(super) fn is_post_glossed(after: &[char], acronym: &str) -> bool {
    let mut i = 0;
    while i < after.len() && matches!(after[i], ' ' | '\t') {
        i += 1;
    }
    if after.get(i) != Some(&'(') {
        return false;
    }
    let inner_start = i + 1;
    let Some(offset) = after[inner_start..].iter().position(|&c| c == ')') else {
        return false;
    };
    let inner = &after[inner_start..inner_start + offset];
    let text: String = inner.iter().collect();
    let alpha_words = text
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphabetic))
        .count();
    alpha_words >= 2 && super::spells(inner, acronym)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlists_are_sorted_and_unique() {
        assert!(COMMON_ACRONYMS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn roman_numeral_check_accepts_canonical_forms_and_rejects_invalid_ones() {
        for valid in [
            "III",
            "VII",
            "VIII",
            "XIV",
            "XIX",
            "XXII",
            "XLII",
            "MCMXCIV",
            "MMMCMXCIX",
        ] {
            assert!(is_exempt(valid), "{valid}");
        }
        for invalid in ["", "IIII", "VV", "IC", "XM", "MIXED", "QBR", "OKR"] {
            assert!(!is_roman_numeral(invalid), "{invalid}");
        }
    }

    #[test]
    fn candidate_accepts_three_to_eight_capitals_and_optional_plural() {
        assert_eq!(candidate("QBR"), Some("QBR"));
        assert_eq!(candidate("QBRs"), Some("QBR"));
        assert_eq!(candidate("EBITDA"), Some("EBITDA"));
        assert_eq!(candidate("AI"), None);
        assert_eq!(candidate("IMPORTANT"), None);
        assert_eq!(candidate("Camel"), None);
    }

    #[test]
    fn exempt_recognises_common_acronyms_and_roman_numerals() {
        for common in ["API", "JSON", "ATO", "AEDT", "WASM", "III", "VIII"] {
            assert!(is_exempt(common), "{common}");
        }
        for unfamiliar in ["QBR", "OKR", "NDIS", "SLA"] {
            assert!(!is_exempt(unfamiliar), "{unfamiliar}");
        }
    }

    #[test]
    fn all_caps_run_skips_banners_but_not_ordinary_prose() {
        let banner: Vec<char> = "DO NOT EDIT".chars().collect();
        assert!(in_all_caps_run(&banner, 3, 6));
        assert!(in_all_caps_run(&banner, 7, 11));

        let prose: Vec<char> = "Review QBR and OKR.".chars().collect();
        assert!(!in_all_caps_run(&prose, 7, 10));

        let comma: Vec<char> = "QBR, OKR".chars().collect();
        assert!(!in_all_caps_run(&comma, 0, 3));
        assert!(!in_all_caps_run(&comma, 5, 8));

        let single: Vec<char> = "QBR".chars().collect();
        assert!(!in_all_caps_run(&single, 0, 3));
    }

    #[test]
    fn post_gloss_recognises_expansions_and_rejects_short_notes() {
        let chars = |s: &str| s.chars().collect::<Vec<char>>();
        assert!(is_post_glossed(
            &chars(" (Quarterly Business Review)"),
            "QBR"
        ));
        assert!(is_post_glossed(&chars("(Service Level Agreement)"), "SLA"));
        assert!(!is_post_glossed(&chars(" (draft)"), "SLA"));
        assert!(!is_post_glossed(&chars(" (2026)"), "QBR"));
        assert!(!is_post_glossed(&chars(" (unclosed"), "QBR"));
        assert!(!is_post_glossed(&chars(" without parens"), "QBR"));
        assert!(!is_post_glossed(&chars(" (see appendix)"), "QBR"));
    }
}
