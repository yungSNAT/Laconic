// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `lint`'s project settings, the `.laconic-check` file, which the command line and the
//! language server both apply.

use crate::lint::{self, Finding, RuleId};

/// The project file `lint` reads from the current directory, and the language server from the
/// workspace folder that holds a document.
pub const PROJECT_FILE: &str = ".laconic-check";

/// Which findings `lint` reports and how many fail it.
#[derive(Debug, Default, PartialEq)]
pub struct Settings {
    pub disable: Vec<&'static str>,
    pub only: Vec<&'static str>,
    pub max_findings: usize,
    /// Words `us-spelling` does not flag, matched as written: same case, whole word.
    pub accept: Vec<String>,
}

impl Settings {
    /// A project file: `key = value` lines for `disable`, `only`, `max-findings` and `accept`,
    /// with blank lines and lines that start with `#` ignored. `path` names it in errors.
    pub fn parse(path: &str, text: &str) -> Result<Settings, String> {
        let mut settings = Settings::default();
        for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let at = |message: String| format!("{path}:{}: {message}", index + 1);
            let Some((key, value)) = line.split_once('=') else {
                return Err(at(format!("expected 'key = value', found '{line}'")));
            };
            let value = value.trim();
            match key.trim() {
                "disable" => settings.disable.extend(rule_names(value).map_err(at)?),
                "only" => settings.only.extend(rule_names(value).map_err(at)?),
                "max-findings" => settings.max_findings = count(value).map_err(at)?,
                "accept" => settings.accept.extend(words(value).map_err(at)?),
                key => return Err(at(format!("unknown key '{key}'"))),
            }
        }
        Ok(settings)
    }

    /// Whether `lint` reports a finding, given the text of the line it is on.
    pub fn keeps(&self, finding: &Finding, line: &str) -> bool {
        let name = finding.rule.name();
        if self.disable.contains(&name) || !(self.only.is_empty() || self.only.contains(&name)) {
            return false;
        }
        // A `us-spelling` finding starts at its word, so the letters from there are that word.
        let accepted = || {
            let word: String = line
                .chars()
                .skip(finding.column.saturating_sub(1))
                .take_while(|c| c.is_alphabetic())
                .collect();
            self.accept.contains(&word)
        };
        !(finding.rule == RuleId::UsSpelling && accepted())
    }
}

/// A comma-separated list of rule names, each one in the rule table.
pub fn rule_names(list: &str) -> Result<Vec<&'static str>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            RuleId::named(name).map(RuleId::name).ok_or_else(|| {
                format!(
                    "unknown rule '{name}'; the rules are in {}",
                    lint::RULES_URL
                )
            })
        })
        .collect()
}

/// A whole number of findings.
pub fn count(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))
}

/// A comma-separated list of words, each made of letters only.
fn words(list: &str) -> Result<Vec<String>, String> {
    list.split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(|word| {
            if word.chars().all(char::is_alphabetic) {
                Ok(word.to_owned())
            } else {
                Err(format!("'{word}' is not one word of letters"))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_file_names_the_line_of_its_first_error() {
        for (text, wanted) in [
            (
                "disable = em-dash\nseverity = high\n",
                "f:2: unknown key 'severity'",
            ),
            (
                "\n\ndisable em-dash\n",
                "f:3: expected 'key = value', found 'disable em-dash'",
            ),
            ("only = em-dashes\n", "f:1: unknown rule 'em-dashes'"),
            ("max-findings = many\n", "f:1: 'many' is not a whole number"),
            (
                "accept = Color, ice cream\n",
                "f:1: 'ice cream' is not one word",
            ),
            ("accept = e-mail\n", "f:1: 'e-mail' is not one word"),
        ] {
            let error = Settings::parse("f", text).unwrap_err();
            assert!(error.starts_with(wanted), "{text:?}: {error}");
        }
    }

    #[test]
    fn accepted_words_pass_us_spelling_as_written_and_whole() {
        let settings = Settings::parse("f", "accept = Color, analyze,\naccept = center\n").unwrap();
        assert_eq!(settings.accept, ["Color", "analyze", "center"]);
        let text = "Color is the product. We analyze the color and the colors.\n\
                    The center box. Analyze it.";
        let kept: Vec<(usize, usize)> = lint::lint(text)
            .iter()
            .filter(|f| {
                let line = text.lines().nth(f.line - 1).unwrap();
                settings.keeps(f, line)
            })
            .map(|f| (f.line, f.column))
            .collect();
        // 'color' and 'colors' differ from 'Color'; 'Analyze' differs from 'analyze'.
        assert_eq!(kept, [(1, 38), (1, 52), (2, 17)]);
        let everything = lint::lint(text).len();
        assert_eq!(everything, 6, "{:?}", lint::lint(text));
    }

    #[test]
    fn disable_and_only_drop_findings_and_disable_wins() {
        let text = "The color \u{2014} red.";
        let found = lint::lint(text);
        let names = |settings: &Settings| -> Vec<&str> {
            let mut kept: Vec<&str> = found
                .iter()
                .filter(|f| settings.keeps(f, text))
                .map(|f| f.rule.name())
                .collect();
            kept.sort_unstable();
            kept
        };
        assert_eq!(names(&Settings::default()), ["em-dash", "us-spelling"]);
        let only = Settings::parse("f", "only = em-dash").unwrap();
        assert_eq!(names(&only), ["em-dash"]);
        let both = Settings::parse("f", "only = em-dash\ndisable = em-dash").unwrap();
        assert!(names(&both).is_empty());
    }
}
