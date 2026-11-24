// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `lint`'s project settings, read from `laconic.toml` or `.laconic-check`, which the command
//! line, the MCP server, the language server and the Stop hook all apply.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::lint::{self, Finding, Phrase, ProjectRule, RuleId, RuleRef};
use crate::profile::Profile;

pub(crate) mod protect;
pub use protect::{LineProtectCache, ProtectList};

/// The legacy dotfile `lint` reads when `laconic.toml` is absent.
pub const PROJECT_FILE: &str = ".laconic-check";

/// The TOML project file `lint` checks before `.laconic-check`.
pub const TOML_PROJECT_FILE: &str = "laconic.toml";

/// Returns the first project file present in `dir`: `laconic.toml`, then `.laconic-check`.
pub fn find_in(dir: &Path) -> Option<PathBuf> {
    for name in [TOML_PROJECT_FILE, PROJECT_FILE] {
        let candidate = if dir == Path::new(".") {
            PathBuf::from(name)
        } else {
            dir.join(name)
        };
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Which findings `lint` reports, how many fail it, author-protected phrases and project rules.
#[derive(Debug, Default, PartialEq)]
pub struct Settings {
    pub disable: Vec<String>,
    pub only: Vec<String>,
    pub max_findings: usize,
    /// Register preset adjusting rule enablement and severity; `None` keeps default `lint`.
    pub profile: Option<Profile>,
    /// Words `us-spelling` and `unglossed-term` do not flag, matched as written: whole word.
    pub accept: Vec<String>,
    /// Author signature idioms/phrases exempt from lint findings on matching spans.
    pub protect: ProtectList,
    /// The `[rules]` section's rules, in the order the file first names them.
    pub rules: Vec<Rc<ProjectRule>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Top,
    Rules,
    Protect,
}

#[derive(Clone, Copy)]
enum ListTarget {
    Disable,
    Only,
    Accept,
    Protect,
}

impl ListTarget {
    fn key(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Only => "only",
            Self::Accept => "accept",
            Self::Protect => "phrases",
        }
    }
}

impl Settings {
    /// Parses a `.laconic-check` or `laconic.toml` project file. Top-level `key = value` lines
    /// configure `disable`, `only`, `max-findings` (`max_findings`), `profile` and `accept`,
    /// followed optionally by `[rules]` (`id: phrase = replacement`) and `[protect]` (author
    /// phrases exempt from lint rules, as `phrases = [...]` or one phrase per line).
    pub fn parse(path: &str, text: &str) -> Result<Settings, String> {
        let mut settings = Settings::default();
        let mut rules: Vec<ProjectRule> = Vec::new();
        let mut phrase_lines: Vec<(String, usize)> = Vec::new();
        let mut named: Vec<(usize, String)> = Vec::new();
        let mut section = Section::Top;
        let mut pending: Option<(usize, ListTarget, String)> = None;

        for (index, raw) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line = raw.trim();
            let number = index + 1;
            let at = |message: String| format!("{path}:{number}: {message}");

            if let Some((start_line, target, mut buf)) = pending.take() {
                buf.push('\n');
                buf.push_str(line);
                if bracket_balanced(&buf) {
                    apply_list_key(&mut settings, &mut named, target, &buf, start_line)
                        .map_err(|m| format!("{path}:{start_line}: {m}"))?;
                } else {
                    pending = Some((start_line, target, buf));
                }
                continue;
            }

            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                let header = protect::strip_comment(line);
                section = match header {
                    "[rules]" => Section::Rules,
                    "[protect]" => Section::Protect,
                    _ => {
                        return Err(at(format!(
                            "unknown section '{header}'; expected [rules] or [protect]"
                        )));
                    }
                };
                continue;
            }

            match section {
                Section::Rules => {
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
                }
                Section::Protect => {
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        if key != "phrases" {
                            return Err(at(format!(
                                "unknown key '{key}' in [protect]; use 'phrases = [...]' or list one phrase per line"
                            )));
                        }
                        let value = value.trim();
                        if value.starts_with('[') && !bracket_balanced(value) {
                            pending = Some((number, ListTarget::Protect, value.to_owned()));
                        } else {
                            apply_list_key(
                                &mut settings,
                                &mut named,
                                ListTarget::Protect,
                                value,
                                number,
                            )
                            .map_err(at)?;
                        }
                    } else {
                        let decoded = protect::decode_value(line).map_err(at)?;
                        let phrase = protect::normalise_phrase_text(&decoded).map_err(at)?;
                        settings.push_protect(phrase);
                    }
                }
                Section::Top => {
                    let Some((key, value)) = line.split_once('=') else {
                        return Err(at(format!("expected 'key = value', found '{line}'")));
                    };
                    let key = key.trim();
                    let value = value.trim();
                    let target = match key {
                        "disable" => Some(ListTarget::Disable),
                        "only" => Some(ListTarget::Only),
                        "accept" => Some(ListTarget::Accept),
                        _ => None,
                    };
                    if let Some(target) = target {
                        if value.starts_with('[') && !bracket_balanced(value) {
                            pending = Some((number, target, value.to_owned()));
                        } else {
                            apply_list_key(&mut settings, &mut named, target, value, number)
                                .map_err(at)?;
                        }
                        continue;
                    }
                    match key {
                        "max-findings" | "max_findings" => {
                            let val = protect::strip_comment(value);
                            settings.max_findings = count(val).map_err(at)?;
                        }
                        "profile" => {
                            let val = protect::decode_value(value).map_err(at)?;
                            settings.profile = Some(profile(&val).map_err(at)?);
                        }
                        _ => return Err(at(format!("unknown key '{key}'"))),
                    }
                }
            }
        }

        if let Some((start_line, target, _)) = pending {
            return Err(format!(
                "{path}:{start_line}: unterminated list for '{}'",
                target.key()
            ));
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

    /// The project file in the current directory (`laconic.toml` or `.laconic-check`),
    /// or the defaults when neither is present.
    pub fn current_dir() -> Result<Settings, String> {
        match find_in(Path::new(".")) {
            Some(path) => Settings::read(&path.to_string_lossy()),
            None => Ok(Settings::default()),
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

    fn push_protect(&mut self, phrase: String) {
        self.protect.push(phrase);
    }

    /// Effective advisory flag for `rule`, after any active register profile.
    pub fn is_advisory(&self, rule: &RuleRef) -> bool {
        match (self.profile, rule.house()) {
            (Some(profile), Some(id)) => profile.is_advisory(id),
            _ => rule.advisory(),
        }
    }

    /// Aligns a finding's `(advisory)` tag with its effective advisory state under the active
    /// profile, so text output and Action runners agree when a profile tightens or relaxes a rule.
    pub fn format_message(&self, rule: &RuleRef, message: &str) -> String {
        let advisory = self.is_advisory(rule);
        if advisory {
            if message.contains("(advisory)") {
                message.to_owned()
            } else {
                format!("{message} (advisory)")
            }
        } else {
            message.replace(" (advisory)", "")
        }
    }

    /// Whether `lint` reports a finding, given the text of the line it is on.
    pub fn keeps(&self, finding: &Finding, line: &str) -> bool {
        self.keeps_inner(finding, line, None)
    }

    /// Like [`Settings::keeps`], reusing per-line protected spans via `cache` so long lines with
    /// many findings stay linear in line length.
    pub fn keeps_cached(
        &self,
        finding: &Finding,
        line: &str,
        cache: &mut LineProtectCache,
    ) -> bool {
        self.keeps_inner(finding, line, Some(cache))
    }

    fn keeps_inner(
        &self,
        finding: &Finding,
        line: &str,
        cache: Option<&mut LineProtectCache>,
    ) -> bool {
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
        if !self.protect.is_empty() {
            let protected = match cache {
                Some(cache) => {
                    cache.contains(self.protect.items(), finding.line, line, finding.column)
                }
                None => protect::is_protected(self.protect.items(), line, finding.column),
            };
            if protected {
                return false;
            }
        }
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

fn bracket_balanced(text: &str) -> bool {
    let mut in_quote: Option<char> = None;
    let mut escape = false;
    let mut depth = 0usize;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if let Some(q) = in_quote {
            if q == '"' && escape {
                escape = false;
            } else if q == '"' && c == '\\' {
                escape = true;
            } else if c == q {
                in_quote = None;
            }
            continue;
        }
        match c {
            '#' => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            '"' | '\'' => in_quote = Some(c),
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth == 0
}

fn apply_list_key(
    settings: &mut Settings,
    named: &mut Vec<(usize, String)>,
    target: ListTarget,
    value: &str,
    line_no: usize,
) -> Result<(), String> {
    let items = protect::parse_list(value)?;
    match target {
        ListTarget::Disable => {
            named.extend(items.iter().map(|n| (line_no, n.clone())));
            settings.disable.extend(items);
        }
        ListTarget::Only => {
            named.extend(items.iter().map(|n| (line_no, n.clone())));
            settings.only.extend(items);
        }
        ListTarget::Accept => {
            for word in items {
                if !word.chars().all(char::is_alphabetic) {
                    return Err(format!("'{word}' is not one word of letters"));
                }
                settings.accept.push(word);
            }
        }
        ListTarget::Protect => {
            for decoded in items {
                settings.push_protect(protect::normalise_phrase_text(&decoded)?);
            }
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests;
