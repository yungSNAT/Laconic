// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! A project's own rules, from the `[rules]` section of its `.laconic-check` file: phrases it
//! wants replaced or never used. They run on the same masked paragraphs as the house rules,
//! so code, URLs and quotations never match.

use std::rc::Rc;

use super::{Finding, RuleRef};

/// One rule: the phrases it flags under one name.
#[derive(Debug, PartialEq)]
pub struct ProjectRule {
    /// `project/` and the id the project file gives it.
    pub name: String,
    pub phrases: Vec<Phrase>,
}

/// A phrase a rule flags: words separated by single spaces, and the text to write instead, or
/// `None` for a phrase the project bans.
#[derive(Debug, PartialEq)]
pub struct Phrase {
    pub text: String,
    pub replacement: Option<String>,
}

impl ProjectRule {
    /// What the rule flags, for a SARIF rule description.
    pub fn description(&self) -> String {
        let phrases: Vec<String> = self
            .phrases
            .iter()
            .map(|p| match &p.replacement {
                Some(r) => format!("'{}' (write '{r}')", p.text),
                None => format!("'{}' (banned)", p.text),
            })
            .collect();
        format!("The project file's rule for {}.", phrases.join(", "))
    }
}

fn apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// Whether `text[at]` is part of a word, which a phrase may not touch at either end: a
/// letter, digit or hyphen, or an apostrophe between letters or digits, as in 'it's'.
fn in_word(text: &[char], at: usize) -> bool {
    let alphanumeric = |i: usize| text.get(i).is_some_and(|c| c.is_alphanumeric());
    text.get(at).is_some_and(|&c| {
        c.is_alphanumeric()
            || c == '-'
            || (apostrophe(c) && at > 0 && alphanumeric(at - 1) && alphanumeric(at + 1))
    })
}

/// The end of `phrase` matched at `start` of `text`, if it matches there as whole words. A
/// space in the phrase matches any run of whitespace, an apostrophe either kind, and a phrase
/// with no capital letter matches in any case; one with a capital matches only as written.
fn match_at(text: &[char], start: usize, phrase: &str) -> Option<usize> {
    let exact = phrase.chars().any(char::is_uppercase);
    let mut at = start;
    for p in phrase.chars() {
        let c = *text.get(at)?;
        if p == ' ' {
            if !c.is_whitespace() {
                return None;
            }
            while text.get(at).is_some_and(|c| c.is_whitespace()) {
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
    (!in_word(text, at)).then_some(at)
}

/// Adds a finding for each phrase of `rules` in the masked paragraph `text`, at its 1-based
/// column there. A match written exactly as its replacement is not a finding, so
/// 'github = GitHub' leaves 'GitHub' alone.
// shortcut: every phrase is tried at every word start, so the cost is the paragraph's length
// times the number of phrases; index the phrases by first letter if projects list thousands.
pub(super) fn check(text: &[char], rules: &[Rc<ProjectRule>], findings: &mut Vec<Finding>) {
    for start in 0..text.len() {
        if !text[start].is_alphanumeric() || (start > 0 && in_word(text, start - 1)) {
            continue;
        }
        for rule in rules {
            for phrase in &rule.phrases {
                let Some(end) = match_at(text, start, &phrase.text) else {
                    continue;
                };
                let found: String = text[start..end].iter().collect();
                let message = match &phrase.replacement {
                    Some(r) if *r == found => continue,
                    Some(r) => format!("'{found}': the project file prefers '{r}'"),
                    None => format!("'{found}': the project file bans this phrase"),
                };
                findings.push(Finding {
                    line: 0,
                    column: start + 1,
                    rule: RuleRef::Project(Rc::clone(rule)),
                    message,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
