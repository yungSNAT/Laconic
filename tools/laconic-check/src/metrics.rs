// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Distributional prose metrics for `laconic-check lint --metrics`.
//!
//! Measures masked original prose only (excluding code, URLs, HTML tags, frontmatter and
//! non-alert block quotations) so writers and agents can compare drafts before and after a
//! rewrite objectively.

use crate::lint;
use crate::mcp::Json;

/// Nominalisation suffixes matched case-insensitively when a word has at least a 3-letter stem.
const NOMINALISATION_SUFFIXES: &[&str] = &[
    "tions", "tion", "sions", "sion", "ments", "ment", "nesses", "ness", "ities", "ity", "ances",
    "ance", "ences", "ence",
];

/// Clause-opening subordinators that distinguish a subordinate clause (`If it fails, rerun it
/// and report`) from a 3-item rhetorical triad (`logs, metrics and traces`).
const CLAUSE_OPENERS: &[&str] = &[
    "after", "although", "as", "because", "before", "if", "once", "since", "though", "unless",
    "until", "when", "whenever", "where", "whereas", "while", "whilst",
];

/// Distributional prose statistics for one file or an aggregated run.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ProseMetrics {
    words: usize,
    word_chars: usize,
    long_words: usize,
    nominalisations: usize,
    triads: usize,
    sentence_lengths: Vec<usize>,
}

impl ProseMetrics {
    /// Measures the masked original prose of `text`.
    pub fn measure(text: &str) -> Self {
        let mut metrics = Self::default();
        for paragraph in lint::prose_paragraphs(text) {
            metrics.add_paragraph(&paragraph);
        }
        metrics
    }

    /// Merges `other` into `self` using raw totals so multi-file means and medians are exact.
    pub fn merge(&mut self, other: &Self) {
        self.words += other.words;
        self.word_chars += other.word_chars;
        self.long_words += other.long_words;
        self.nominalisations += other.nominalisations;
        self.triads += other.triads;
        self.sentence_lengths.extend(&other.sentence_lengths);
    }

    pub fn words(&self) -> usize {
        self.words
    }

    pub fn sentences(&self) -> usize {
        self.sentence_lengths.len()
    }

    pub fn mean_word_length(&self) -> f64 {
        ratio(self.word_chars, self.words)
    }

    pub fn long_words(&self) -> usize {
        self.long_words
    }

    pub fn long_word_share(&self) -> f64 {
        ratio(self.long_words, self.words)
    }

    pub fn nominalisations(&self) -> usize {
        self.nominalisations
    }

    pub fn nominalisation_density(&self) -> f64 {
        ratio(self.nominalisations, self.words)
    }

    pub fn triads(&self) -> usize {
        self.triads
    }

    pub fn sentence_length_mean(&self) -> f64 {
        let total: usize = self.sentence_lengths.iter().sum();
        ratio(total, self.sentence_lengths.len())
    }

    pub fn sentence_length_median(&self) -> f64 {
        if self.sentence_lengths.is_empty() {
            return 0.0;
        }
        let mut sorted = self.sentence_lengths.clone();
        sorted.sort_unstable();
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 1 {
            sorted[mid] as f64
        } else {
            (sorted[mid - 1] + sorted[mid]) as f64 / 2.0
        }
    }

    /// Concise single-line human-readable summary for plain text `lint --metrics` output.
    pub fn format_line(&self, label: &str) -> String {
        format!(
            "metrics: {label}: words={} sentences={} mean_word_len={:.2} long_words={:.1}% \
             nominalisations={:.1}% triads={} sentence_len=mean {:.1} / median {:.1}",
            self.words(),
            self.sentences(),
            self.mean_word_length(),
            self.long_word_share() * 100.0,
            self.nominalisation_density() * 100.0,
            self.triads(),
            self.sentence_length_mean(),
            self.sentence_length_median(),
        )
    }

    /// JSON object representation for `lint --json --metrics`.
    pub fn to_json(&self, file: Option<&str>) -> Json {
        let mut fields = Vec::with_capacity(11);
        if let Some(name) = file {
            fields.push(("file".into(), Json::String(name.to_owned())));
        }
        fields.push(("words".into(), Json::Number(self.words().to_string())));
        fields.push((
            "sentences".into(),
            Json::Number(self.sentences().to_string()),
        ));
        fields.push((
            "mean_word_length".into(),
            Json::Number(format!("{:.2}", self.mean_word_length())),
        ));
        fields.push((
            "long_words".into(),
            Json::Number(self.long_words().to_string()),
        ));
        fields.push((
            "long_word_share".into(),
            Json::Number(format!("{:.4}", self.long_word_share())),
        ));
        fields.push((
            "nominalisations".into(),
            Json::Number(self.nominalisations().to_string()),
        ));
        fields.push((
            "nominalisation_density".into(),
            Json::Number(format!("{:.4}", self.nominalisation_density())),
        ));
        fields.push(("triads".into(), Json::Number(self.triads().to_string())));
        fields.push((
            "sentence_length_mean".into(),
            Json::Number(format!("{:.2}", self.sentence_length_mean())),
        ));
        fields.push((
            "sentence_length_median".into(),
            Json::Number(format!("{:.2}", self.sentence_length_median())),
        ));
        Json::Object(fields)
    }

    fn add_paragraph(&mut self, paragraph: &str) {
        let chars: Vec<char> = paragraph.chars().collect();
        let mut start = 0;
        while start < chars.len() {
            while start < chars.len() && chars[start].is_whitespace() {
                start += 1;
            }
            if start >= chars.len() {
                break;
            }
            let stop = sentence_end(&chars, start);
            let sentence: String = chars[start..stop].iter().collect();
            let words = extract_words(&sentence);
            if !words.is_empty() {
                self.sentence_lengths.push(words.len());
                for word in &words {
                    let letters = word.chars().filter(|c| c.is_alphabetic()).count();
                    self.words += 1;
                    self.word_chars += letters;
                    if letters >= 7 {
                        self.long_words += 1;
                    }
                    if is_nominalisation(word) {
                        self.nominalisations += 1;
                    }
                }
                self.triads += count_triads(&sentence);
            }
            start = stop;
        }
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn is_closing_punct(c: char) -> bool {
    matches!(c, '"' | '\'' | '\u{201d}' | '\u{2019}' | ')' | ']')
}

fn sentence_end(chars: &[char], start: usize) -> usize {
    let mut stop = start;
    while stop < chars.len() {
        if matches!(chars[stop], '.' | '!' | '?') {
            let mut after = stop + 1;
            while after < chars.len() && is_closing_punct(chars[after]) {
                after += 1;
            }
            if chars.get(after).is_none_or(|c| c.is_whitespace()) {
                return after;
            }
        }
        stop += 1;
    }
    chars.len()
}

/// Alphabetic words in `sentence`, keeping internal hyphens and apostrophes inside one word.
fn extract_words(sentence: &str) -> Vec<String> {
    let chars: Vec<char> = sentence.chars().collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len()
            && (chars[i].is_alphabetic()
                || (matches!(chars[i], '-' | '\'' | '\u{2019}')
                    && chars.get(i + 1).is_some_and(|c| c.is_alphabetic())))
        {
            i += 1;
        }
        words.push(chars[start..i].iter().collect());
    }
    words
}

/// True when `word` ends with a nominalisation suffix after at least a 3-letter stem.
fn is_nominalisation(word: &str) -> bool {
    let lower: String = word
        .chars()
        .filter(|c| c.is_alphabetic())
        .flat_map(char::to_lowercase)
        .collect();
    NOMINALISATION_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix) && lower.len() >= suffix.len() + 3)
}

/// Counts 3-item coordinate lists (`X, Y and Z` or `X, Y, and Z`, and the `or` forms) inside
/// `sentence`, skipping 4-or-more-item lists and subordinate clauses.
fn count_triads(sentence: &str) -> usize {
    sentence
        .split([';', ':'])
        .filter(|clause| is_triad_clause(clause))
        .count()
}

fn is_triad_clause(clause: &str) -> bool {
    let parts: Vec<&str> = clause.split(',').map(str::trim).collect();
    let (first_part, rest) = (&parts[0], &parts[1..]);
    let first_words = extract_words(first_part);
    if first_words.is_empty() || CLAUSE_OPENERS.contains(&first_words[0].to_lowercase().as_str()) {
        return false;
    }
    match rest {
        // `X, Y and Z` / `X, Y or Z`
        [tail] => split_conjunction(tail).is_some_and(|(mid, last)| {
            let mid_count = extract_words(mid).len();
            (1..=5).contains(&mid_count) && !extract_words(last).is_empty()
        }),
        // `X, Y, and Z` / `X, Y, or Z` (serial comma)
        [mid, tail] => {
            let mid_count = extract_words(mid).len();
            (1..=5).contains(&mid_count)
                && strip_leading_conjunction(tail)
                    .is_some_and(|last| !extract_words(last).is_empty())
        }
        _ => false,
    }
}

fn split_conjunction(text: &str) -> Option<(&str, &str)> {
    let lower = text.to_lowercase();
    for needle in [" and ", " or "] {
        if let Some(pos) = lower.find(needle) {
            // `text` and `lower` may differ in byte length on non-ASCII characters; split by chars.
            let char_idx = lower[..pos].chars().count();
            let split_at = text
                .char_indices()
                .nth(char_idx)
                .map_or(text.len(), |(b, _)| b);
            let after_at = text
                .char_indices()
                .nth(char_idx + needle.chars().count())
                .map_or(text.len(), |(b, _)| b);
            return Some((&text[..split_at], &text[after_at..]));
        }
    }
    None
}

fn strip_leading_conjunction(text: &str) -> Option<&str> {
    let lower = text.to_lowercase();
    for prefix in ["and ", "or "] {
        if lower.starts_with(prefix) {
            let after_at = text
                .char_indices()
                .nth(prefix.len())
                .map_or(text.len(), |(b, _)| b);
            return Some(&text[after_at..]);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_code_only_text_report_zero_metrics() {
        let m = ProseMetrics::measure("```rust\nlet implementation = 1;\n```\n");
        assert_eq!(m.words(), 0);
        assert_eq!(m.sentences(), 0);
        assert_eq!(m.mean_word_length(), 0.0);
        assert_eq!(m.long_words(), 0);
        assert_eq!(m.long_word_share(), 0.0);
        assert_eq!(m.nominalisations(), 0);
        assert_eq!(m.nominalisation_density(), 0.0);
        assert_eq!(m.triads(), 0);
        assert_eq!(m.sentence_length_mean(), 0.0);
        assert_eq!(m.sentence_length_median(), 0.0);
    }

    #[test]
    fn measures_words_long_words_nominalisations_triads_and_sentences() {
        let text = "Bring logs, metrics and traces. The implementation improves reliability, clarity, and performance.\n";
        let m = ProseMetrics::measure(text);
        assert_eq!(m.words(), 12);
        assert_eq!(m.sentences(), 2);
        assert_eq!(m.triads(), 2);
        // Long words (>=7): metrics(7), implementation(14), improves(8), reliability(11), clarity(7), performance(11) = 6
        assert_eq!(m.long_words(), 6);
        assert!((m.long_word_share() - 0.5).abs() < 1e-9);
        // Nominalisations: implementation, reliability, clarity, performance = 4
        assert_eq!(m.nominalisations(), 4);
        assert!((m.nominalisation_density() - (4.0 / 12.0)).abs() < 1e-9);
        // Sentence lengths: 5 and 7 -> mean 6.0, median 6.0
        assert!((m.sentence_length_mean() - 6.0).abs() < 1e-9);
        assert!((m.sentence_length_median() - 6.0).abs() < 1e-9);
        let line = m.format_line("sample.md");
        assert!(
            line.starts_with("metrics: sample.md: words=12 sentences=2 "),
            "{line}"
        );
        assert!(line.contains("triads=2"), "{line}");
    }

    #[test]
    fn odd_median_and_merge_compute_exact_overall_distribution() {
        let mut a = ProseMetrics::measure("One two. One two three four.");
        let b = ProseMetrics::measure("One two three four five six.");
        a.merge(&b);
        // Lengths: [2, 4, 6] -> mean 4.0, median 4.0
        assert_eq!(a.sentences(), 3);
        assert_eq!(a.words(), 12);
        assert!((a.sentence_length_mean() - 4.0).abs() < 1e-9);
        assert!((a.sentence_length_median() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn triads_skip_four_item_lists_and_subordinate_clauses() {
        assert_eq!(
            ProseMetrics::measure("Fast, reliable, scalable and secure.").triads(),
            0
        );
        assert_eq!(
            ProseMetrics::measure("If the build fails, rerun it and report the error.").triads(),
            0
        );
        assert_eq!(
            ProseMetrics::measure("Pick red, blue or green.").triads(),
            1
        );
        assert_eq!(
            ProseMetrics::measure("Pick red, blue, or green.").triads(),
            1
        );
        assert_eq!(ProseMetrics::measure("Red and blue.").triads(), 0);
        assert_eq!(ProseMetrics::measure("Red, blue, green.").triads(), 0);
        assert_eq!(ProseMetrics::measure("   ").triads(), 0);
    }

    #[test]
    fn nominalisation_requires_three_letter_stem() {
        assert!(is_nominalisation("implementation"));
        assert!(is_nominalisation("clarity"));
        assert!(is_nominalisation("performance"));
        assert!(is_nominalisation("readiness"));
        assert!(is_nominalisation("decision"));
        assert!(is_nominalisation("management"));
        assert!(!is_nominalisation("city"));
        assert!(!is_nominalisation("action"));
        assert!(!is_nominalisation("plain"));
    }

    #[test]
    fn json_output_includes_optional_file_name_and_all_fields() {
        let m = ProseMetrics::measure("Clear prose works.");
        let with_file = m.to_json(Some("a.md"));
        let without_file = m.to_json(None);
        let serialized_with = crate::mcp::to_string(&with_file);
        let serialized_without = crate::mcp::to_string(&without_file);
        assert!(serialized_with.contains("\"file\":\"a.md\""));
        assert!(!serialized_without.contains("\"file\":"));
        for key in [
            "\"words\":3",
            "\"sentences\":1",
            "\"mean_word_length\":",
            "\"long_words\":0",
            "\"long_word_share\":0.0000",
            "\"nominalisations\":0",
            "\"nominalisation_density\":0.0000",
            "\"triads\":0",
            "\"sentence_length_mean\":3.00",
            "\"sentence_length_median\":3.00",
        ] {
            assert!(serialized_without.contains(key), "{serialized_without}");
        }
    }

    #[test]
    fn skips_punctuation_only_sentences_and_handles_empty_clause() {
        let m = ProseMetrics::measure("...   It's well-being. `code`");
        assert_eq!(m.words(), 2);
        assert_eq!(m.sentences(), 1);
        assert_eq!(m.long_words(), 1);
        assert!(!is_triad_clause(""));
        assert!(!is_triad_clause(", a and b"));
        assert!(!is_triad_clause("a, b"));
        assert!(!is_triad_clause("a, b, c"));
    }

    #[test]
    fn counts_sentences_ending_before_closing_quotes_or_brackets() {
        for sample in [
            "He asked, \"Really?\" Then he left.",
            "She shouted, \"Stop!\" He froze.",
            "He asked, \u{201c}Really?\u{201d} Then he left.",
            "He asked, 'Really?' Then he left.",
            "He asked (really?) Then he left.",
            "She called [Wait!] Then ran.",
        ] {
            let m = ProseMetrics::measure(sample);
            assert_eq!(m.sentences(), 2, "{sample:?}");
        }
        assert_eq!(ProseMetrics::measure("Version 1.2 works.").sentences(), 1);
        assert_eq!(
            ProseMetrics::measure("Heading without punctuation").sentences(),
            1
        );
    }
}
