// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! A possible reversal: a sentence of the original negates a claim ('is not cast') and a
//! sentence of the edit states it without a negation ('is cast'). Dropping the negated half of
//! a staged contrast ('not a tool, a system' to 'a system') loses the negated word too, so it
//! is not reported, and nor is a sentence the edit drops. It is advice and never fails.

use std::collections::HashSet;

/// Words that never name what a negation negates and never count as a sentence's content.
const SKIP: &[&str] = &[
    "a", "an", "the", "be", "been", "being", "is", "are", "was", "were", "am", "do", "does", "did",
    "have", "has", "had", "to", "so", "very", "always", "really", "quite", "too", "yet", "even",
    "all", "any", "just", "only", "merely", "simply", "also", "it", "this", "that", "these",
    "those", "there", "their", "they", "them", "we", "you", "your", "our", "i", "he", "she", "his",
    "her", "its", "as", "at", "in", "on", "of", "for", "by", "with", "from", "will", "would",
    "can", "could", "should", "may", "might", "must", "shall",
];

/// After a negation, these add rather than negate: 'not just an app'.
const ADDITIVE: &[&str] = &["just", "only", "merely", "simply"];

/// A sentence of the edit with one of these negates or contrasts in its own words.
const EDIT_NEGATIONS: &[&str] = &[
    "not", "never", "cannot", "no", "none", "nor", "without", "dont", "cant", "wont", "doesnt",
    "didnt", "isnt",
];
const CONTRASTS: &[[&str; 2]] = &[["rather", "than"], ["instead", "of"]];

/// The longest stretch of an edit sentence a report quotes, in characters.
const QUOTED: usize = 80;

/// Sentences: lines, split again after '.', '!' or '?' and a space.
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            current.push(c);
            if matches!(c, '.' | '!' | '?') && chars.peek().is_some_and(|n| n.is_whitespace()) {
                out.push(std::mem::take(&mut current));
            }
        }
        out.push(current);
    }
    out.retain(|s| !s.trim().is_empty());
    out
}

/// A sentence's words, lower case, each a letter then letters, apostrophes or hyphens.
fn words(sentence: &str) -> Vec<String> {
    let lower = sentence.to_lowercase().replace('\u{2019}', "'");
    let mut out = Vec::new();
    let mut current = String::new();
    for c in lower.chars() {
        if c.is_alphabetic() || (!current.is_empty() && matches!(c, '\'' | '-')) {
            current.push(c);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn negation(word: &str) -> bool {
    matches!(word, "not" | "never" | "cannot" | "no" | "none" | "without") || word.ends_with("n't")
}

/// The content word right after each negation that negates: the first of the next 4 words
/// that is not in SKIP and not itself a negation.
fn negated(words: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    for (i, word) in words.iter().enumerate() {
        let additive = words
            .get(i + 1)
            .is_some_and(|next| ADDITIVE.contains(&next.as_str()));
        if !negation(word) || additive {
            continue;
        }
        if let Some(target) = words[i + 1..]
            .iter()
            .take(4)
            .find(|w| !SKIP.contains(&w.as_str()) && !negation(w))
        {
            out.push(target.as_str());
        }
    }
    out
}

/// Words that carry a sentence's content: not in SKIP and over 2 characters.
fn content(words: &[String]) -> HashSet<&str> {
    words
        .iter()
        .map(String::as_str)
        .filter(|w| !SKIP.contains(w) && w.chars().count() > 2)
        .collect()
}

fn negates(words: &[String]) -> bool {
    words
        .iter()
        .any(|w| EDIT_NEGATIONS.contains(&w.as_str()) || w.ends_with("n't"))
        || words
            .windows(2)
            .any(|pair| CONTRASTS.iter().any(|c| pair[0] == c[0] && pair[1] == c[1]))
}

/// Each sentence of `edited` that states without a negation a word a sentence of `original`
/// negates, where the 2 sentences share at least half of the original's content words.
pub fn reversals(original: &str, edited: &str) -> Vec<String> {
    let edits: Vec<(String, Vec<String>)> = sentences(edited)
        .into_iter()
        .map(|s| {
            let w = words(&s);
            (s, w)
        })
        .filter(|(_, w)| !negates(w))
        .collect();
    let edits: Vec<(&str, HashSet<&str>)> = edits
        .iter()
        .map(|(s, w)| (s.as_str(), content(w)))
        .collect();
    let mut out = Vec::new();
    for sentence in sentences(original) {
        let words = words(&sentence);
        let targets = negated(&words);
        let shared = content(&words);
        if targets.is_empty() || shared.is_empty() {
            continue;
        }
        for (edit, has) in &edits {
            if shared.intersection(has).count() * 2 < shared.len() {
                continue;
            }
            for target in targets.iter().filter(|t| has.contains(*t)) {
                let quoted: String = edit.trim().chars().take(QUOTED).collect();
                let more = if edit.trim().chars().count() > QUOTED {
                    "..."
                } else {
                    ""
                };
                // Debug quoting escapes any quotation mark or control character inside.
                let item = format!("'{target}' is no longer negated in {:?}", quoted + more);
                if !out.contains(&item) {
                    out.push(item);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_claim_the_edit_states_without_its_negation() {
        assert_eq!(
            reversals(
                "Every surface is milled, not cast. It ships today.",
                "Every surface is milled and cast. It ships today."
            ),
            ["'cast' is no longer negated in \"Every surface is milled and cast.\""]
        );
        // 'no', 'none' and 'without' negate too.
        for (original, edited) in [
            ("No straps are included.", "Straps are included."),
            ("None of the straps fit.", "The straps fit."),
            ("It ships without straps.", "It ships with straps."),
        ] {
            assert_eq!(
                reversals(original, edited).len(),
                1,
                "{original} -> {edited}"
            );
        }
        // Curly apostrophes, and a contraction the edit expands away.
        assert_eq!(
            reversals(
                "Don\u{2019}t water the fern daily.",
                "Water the fern daily."
            )
            .len(),
            1
        );
    }

    #[test]
    fn a_staged_contrast_an_addition_a_restated_negation_or_a_deletion_is_not_a_reversal() {
        for (original, edited) in [
            ("It's not a tool, it's a system.", "It's a system."),
            (
                "Resonance isn't just another EQ app.",
                "Resonance is another EQ app.",
            ),
            ("Straps are not included.", "No straps are included."),
            (
                "Straps are not included.",
                "Straps are excluded rather than included.",
            ),
            ("Straps are not included. It fits.", "It fits."),
            (
                "You don't need a coach.",
                "A coach would help others entirely.",
            ),
        ] {
            assert!(
                reversals(original, edited).is_empty(),
                "{original} -> {edited}"
            );
        }
    }

    #[test]
    fn quotes_a_long_sentence_in_part() {
        let long = format!("The pump is {} loud.", "very ".repeat(30));
        let found = reversals("The pump is not loud.", &long);
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("...\""), "{}", found[0]);
    }

    #[test]
    fn escapes_quotation_marks_in_the_sentence_it_quotes() {
        assert_eq!(
            reversals("Do not mark the cache stale.", "Mark the cache \"stale\"."),
            [r#"'mark' is no longer negated in "Mark the cache \"stale\".""#]
        );
    }
}
