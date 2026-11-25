// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! A project's own rules, from the `[rules]` section of its `.laconic-check` file: phrases it
//! wants replaced or never used. They run on the same masked paragraphs as the house rules,
//! so code, URLs and quotations never match.

use std::rc::Rc;

use super::{Finding, Fix, RuleRef};

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

use crate::settings::protect::{is_word_char, match_phrase};

/// Adds a finding for each phrase of `rules` in the masked paragraph `text`, at its 1-based
/// column there. A match written exactly as its replacement is not a finding, so
/// 'github = GitHub' leaves 'GitHub' alone.
// shortcut: every phrase is tried at every word start, so the cost is the paragraph's length
// times the number of phrases; index the phrases by first letter if projects list thousands.
pub(super) fn check(text: &[char], rules: &[Rc<ProjectRule>], findings: &mut Vec<Finding>) {
    for start in 0..text.len() {
        if !text[start].is_alphanumeric() || (start > 0 && is_word_char(text, start - 1)) {
            continue;
        }
        for rule in rules {
            for phrase in &rule.phrases {
                let exact = phrase.text.chars().any(char::is_uppercase);
                let Some(end) = match_phrase(text, start, phrase.text.chars(), exact, false, true)
                else {
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
                    // A replacement with a capital ('GitHub') as the project file writes it;
                    // one without, in the case of the text it replaces ('Use' for 'Utilise').
                    fix: phrase.replacement.as_deref().and_then(|new| {
                        if new.chars().any(char::is_uppercase) {
                            Fix::of(text, start, end - start, new.to_owned())
                        } else {
                            Fix::cased(text, start, end - start, new)
                        }
                    }),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests;
