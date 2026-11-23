// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `lint`'s project settings, the `.laconic-check` file, which the command line, the MCP
//! server and the language server all apply.

use std::rc::Rc;

use crate::lint::{self, Finding, Phrase, ProjectRule, RuleId, RuleRef};
use crate::profile::Profile;

/// The project file `lint` and the MCP server read from the current directory, and the
/// language server from the workspace folder that holds a document.
pub const PROJECT_FILE: &str = ".laconic-check";

/// Which findings `lint` reports, how many fail it, and the project's own rules.
#[derive(Debug, Default, PartialEq)]
pub struct Settings {
    pub disable: Vec<String>,
    pub only: Vec<String>,
    pub max_findings: usize,
    /// Register preset adjusting rule enablement and severity; `None` keeps default `lint`.
    pub profile: Option<Profile>,
    /// Words `us-spelling` and `unglossed-term` do not flag, matched as written: whole word.
    pub accept: Vec<String>,
    /// The `[rules]` section's rules, in the order the file first names them.
    pub rules: Vec<Rc<ProjectRule>>,
}

impl Settings {
    /// A project file: `key = value` lines for `disable`, `only`, `max-findings` and `accept`,
    /// then optionally a `[rules]` section of `id: phrase = replacement` lines, with blank
    /// lines and lines that start with `#` ignored. `path` names it in errors.
    pub fn parse(path: &str, text: &str) -> Result<Settings, String> {
        let mut settings = Settings::default();
        let mut rules: Vec<ProjectRule> = Vec::new();
        // Each phrase's line, for a duplicate's error.
        let mut phrase_lines: Vec<(String, usize)> = Vec::new();
        // Rule names in disable and only, checked once every project rule is known.
        let mut named: Vec<(usize, String)> = Vec::new();
        let mut in_rules = false;
        for (index, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = line.trim();
            let number = index + 1;
            let at = |message: String| format!("{path}:{number}: {message}");
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                if line != "[rules]" {
                    return Err(at(format!(
                        "unknown section '{line}'; the only section is [rules]"
                    )));
                }
                in_rules = true;
                continue;
            }
            if in_rules {
                let (id, phrase) = rule_line(line).map_err(at)?;
                if let Some((_, first)) = phrase_lines.iter().find(|(p, _)| *p == phrase.text) {
                    return Err(at(format!(
                        "'{}' is already a rule on line {first}",
                        phrase.text
                    )));
                }
                phrase_lines.push((phrase.text.clone(), number));
                let name = format!("project/{id}");
                match rules.iter_mut().find(|r| r.name == name) {
                    Some(rule) => rule.phrases.push(phrase),
                    None => rules.push(ProjectRule {
                        name,
                        phrases: vec![phrase],
                    }),
                }
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(at(format!("expected 'key = value', found '{line}'")));
            };
            let value = value.trim();
            match key.trim() {
                "disable" | "only" => {
                    let names = rule_names(value);
                    named.extend(names.iter().map(|n| (number, n.clone())));
                    if key.trim() == "disable" {
                        settings.disable.extend(names);
                    } else {
                        settings.only.extend(names);
                    }
                }
                "max-findings" => settings.max_findings = count(value).map_err(at)?,
                "profile" => settings.profile = Some(profile(value).map_err(at)?),
                "accept" => settings.accept.extend(words(value).map_err(at)?),
                key => return Err(at(format!("unknown key '{key}'"))),
            }
        }
        settings.rules = rules.into_iter().map(Rc::new).collect();
        for (number, name) in named {
            settings
                .check_rule(&name)
                .map_err(|message| format!("{path}:{number}: {message}"))?;
        }
        Ok(settings)
    }

    /// Reads and parses the project file at `path`.
    pub fn read(path: &str) -> Result<Settings, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        Settings::parse(path, &text)
    }

    /// The project file in the current directory, or the defaults when there is none.
    pub fn current_dir() -> Result<Settings, String> {
        if std::path::Path::new(PROJECT_FILE).is_file() {
            Settings::read(PROJECT_FILE)
        } else {
            Ok(Settings::default())
        }
    }

    /// An error unless `name` is a house rule or one of this file's project rules.
    pub fn check_rule(&self, name: &str) -> Result<(), String> {
        if RuleId::named(name).is_some() || self.rules.iter().any(|r| r.name == name) {
            Ok(())
        } else if name.starts_with("project/") {
            Err(format!(
                "unknown rule '{name}'; the project file's [rules] section does not name it"
            ))
        } else {
            Err(format!(
                "unknown rule '{name}'; the rules are in {}",
                lint::RULES_URL
            ))
        }
    }

    /// True when the active profile or `--only` filter enables any opt-in rule.
    pub fn needs_opt_in(&self) -> bool {
        if !self.only.is_empty() {
            return self.only.iter().any(|name| {
                !self.disable.contains(name)
                    && RuleId::named(name).is_some_and(|id| id.rule().opt_in)
            });
        }
        self.profile.is_some_and(|p| p.enables_opt_in())
    }

    /// Effective advisory flag for `rule`, after any active register profile.
    pub fn is_advisory(&self, rule: &RuleRef) -> bool {
        match (self.profile, rule.house()) {
            (Some(profile), Some(id)) => profile.is_advisory(id),
            _ => rule.advisory(),
        }
    }

    /// Whether `lint` reports a finding, given the text of the line it is on.
    pub fn keeps(&self, finding: &Finding, line: &str) -> bool {
        let name = finding.rule.name();
        let listed = |names: &[String]| names.iter().any(|n| n == name);
        if listed(&self.disable) {
            return false;
        }
        if !self.only.is_empty() {
            if !listed(&self.only) {
                return false;
            }
        } else if let Some(id) = finding.rule.house() {
            if id.rule().opt_in {
                if !self.profile.is_some_and(|p| p.enables(id)) {
                    return false;
                }
            } else if self.profile.is_some_and(|p| p.disables(id)) {
                return false;
            }
        }
        // A `us-spelling` or `unglossed-term` finding's column is the first letter of a whole
        // run of letters, so the letters from there are that word.
        let word_at = || -> String {
            line.chars()
                .skip(finding.column.saturating_sub(1))
                .take_while(|c| c.is_alphabetic())
                .collect()
        };
        if finding.rule == RuleId::UsSpelling && self.accept.contains(&word_at()) {
            return false;
        }
        if finding.rule == RuleId::UnglossedTerm {
            let word = word_at();
            let base = word.strip_suffix('s').unwrap_or(&word);
            if self.accept.iter().any(|a| a == &word || a == base) {
                return false;
            }
        }
        true
    }
}

/// A `[rules]` line, `id: phrase = replacement`: the id, of lower-case letters, digits and
/// inner hyphens, and the phrase, of words of letters and digits with inner hyphens or
/// apostrophes. Nothing after `=` bans the phrase.
fn rule_line(line: &str) -> Result<(&str, Phrase), String> {
    let expected = || {
        format!("expected 'id: phrase = replacement', found '{line}'; settings go before [rules]")
    };
    let (id, rest) = line.split_once(':').ok_or_else(expected)?;
    let (phrase, replacement) = rest.split_once('=').ok_or_else(expected)?;
    let id = id.trim();
    let id_char = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    if !(id.chars().all(|c| id_char(c) || c == '-')
        && id.starts_with(id_char)
        && id.ends_with(id_char))
    {
        return Err(format!(
            "'{id}' is not a rule id; use lower-case letters, digits and hyphens"
        ));
    }
    let words: Vec<&str> = phrase.split_whitespace().collect();
    let word = |w: &&str| {
        w.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '\'' | '\u{2019}'))
            && w.starts_with(char::is_alphanumeric)
            && w.ends_with(char::is_alphanumeric)
    };
    if words.is_empty() || !words.iter().all(word) {
        return Err(format!(
            "'{}' is not a phrase of words of letters and digits",
            phrase.trim()
        ));
    }
    let replacement = replacement.trim();
    Ok((
        id,
        Phrase {
            text: words.join(" "),
            replacement: (!replacement.is_empty()).then(|| replacement.to_owned()),
        },
    ))
}

/// A comma-separated list of rule names; `Settings::check_rule` says whether each is one.
pub fn rule_names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A whole number of findings.
pub fn count(value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("'{value}' is not a whole number"))
}

/// A register profile name (`email`, `report`, `docs`, `chat` or `essay`).
pub fn profile(value: &str) -> Result<Profile, String> {
    Profile::parse(value).ok_or_else(|| {
        format!(
            "unknown profile '{value}'; expected {}",
            Profile::NAMES.join(", ")
        )
    })
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
mod tests;
