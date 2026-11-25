// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! `laconic-check instructions [topic]`: what a coding agent needs to use the checker in a
//! repository, as plain text. The CLI prints it and the MCP server offers it as a prompt. The
//! rule list is generated from the rule table, so it cannot drift from `lint`.

use crate::lint::{RULES, RULES_URL};
use crate::profile::Profile;
use std::fmt::Write;

/// The topics after `instructions`, with what each covers.
pub const TOPICS: &[(&str, &str)] = &[
    ("config", "the laconic.toml project file"),
    (
        "rules",
        "every rule, whether it fails lint and whether it is opt-in",
    ),
];

const OVERVIEW: &str = "\
laconic-check checks Markdown prose against Laconic's house style: Australian English, the
Australian Government Style Manual and common AI writing patterns. It reports patterns, never
who or what wrote a text.

Check prose:
  laconic-check lint <file.md>...      one line a finding: file:line:column: rule: message
  laconic-check lint -                 read a draft from standard input
  laconic-check lint --json <file.md>  the same findings as one JSON document
  laconic-check preserve <original> <edited>
                                       fail when an edit dropped a number, date, name, quotation,
                                       URL, inline code or a word such as 'not' or 'must'

Read a finding: it is a reason to look, not proof of a defect. A message ending '(advisory)'
never fails lint; every other finding does. Each rule is described under its name in
RULES_URL.

Act on a finding: fix the text where the rule is right. Where the text is right as written,
leave it and, if the finding would block CI, silence that one finding with a comment on its
own line before it, naming the rule:
  <!-- laconic-check-disable-next-line us-spelling -->
lint reports a comment that silences nothing as unused-suppression. Never rewrite a
quotation, a name or code to satisfy a rule: lint skips code, URLs and quoted text, and a
[protect] phrase in laconic.toml exempts an author's own term wherever it appears.

Gate CI:
  laconic-check lint --max-findings 0 <files>   exit 1 on any failing finding
  laconic-check lint --warn <files>             report only, never fail
  laconic-check lint --sarif --warn <files>     a SARIF log for code scanning
The GitHub Action yungSNAT/Laconic runs lint on every tracked Markdown file and annotates each
finding.

Set up a repository: put a laconic.toml beside the files you lint to turn rules off, pick a
register profile or protect terms; `laconic-check instructions config` shows its keys.
`laconic-check mcp` serves lint and preserve as MCP tools, and `laconic-check hook` is a Stop
hook that asks the agent to look at failing findings before it finishes a turn.

Exit codes: 0 clean, 1 findings (or dropped items for preserve), 2 a usage or read error.
";

const CONFIG: &str = "\
laconic.toml, in the directory lint runs from (.laconic-check is the older name and is read
when laconic.toml is absent; --config <file> names another file). '#' starts a comment.

  disable = [\"serial-comma\"]     rules to drop (a list or comma-separated names)
  only = [\"em-dash\"]             keep only these rules; disable wins over only
  accept = [\"color\"]             words us-spelling and unglossed-term do not flag,
                                 matched whole and in the same case
  profile = \"report\"             a register preset: PROFILES
  max_findings = 3               fail only on more than 3 failing findings

  [rules]                        the project's own rules, one a line, reported as project/<id>
  plain: utilise = use           suggest a replacement
  plain: leverage =              ban a phrase

  [protect]                      phrases exempt from every rule where they appear
  phrases = [\"Best-in-Class\", \"deep dive\"]

A phrase with a capital letter matches only as written; one without matches in any case.
Command-line --disable and --only add to the file's lists; --profile and --max-findings
replace its values. A malformed file is an error naming its line.
";

/// The text for `topic`, or `None` for an unknown topic.
pub fn render(topic: Option<&str>) -> Option<String> {
    match topic {
        None => Some(OVERVIEW.replace("RULES_URL", RULES_URL) + &topics()),
        Some("config") => Some(CONFIG.replace("PROFILES", &Profile::NAMES.join(", "))),
        Some("rules") => Some(rules()),
        Some(_) => None,
    }
}

/// The topic list, as the overview ends and as an unknown topic's error gives it.
pub fn topics() -> String {
    let mut out = String::from("\nTopics: laconic-check instructions <topic>\n");
    for (name, about) in TOPICS {
        let _ = writeln!(out, "  {name:<8} {about}");
    }
    out
}

fn rules() -> String {
    let mut out = format!(
        "Each rule, whether a finding fails lint, and whether it runs by default. An opt-in rule\n\
         runs only when a profile enables it or `only` names it. Details: {RULES_URL}\n\n"
    );
    for rule in RULES {
        let kind = if rule.advisory { "advisory" } else { "fails" };
        let when = if rule.opt_in { "opt-in" } else { "default" };
        let _ = writeln!(out, "{:<26} {kind:<8} {when:<7} {}", rule.name, rule.checks);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::RuleId;
    use crate::settings::Settings;

    #[test]
    fn every_topic_renders_and_an_unknown_one_does_not() {
        assert!(
            render(None)
                .unwrap()
                .contains("laconic-check instructions <topic>")
        );
        for (name, _) in TOPICS {
            assert!(render(Some(name)).is_some(), "{name}");
        }
        assert_eq!(render(Some("update")), None);
    }

    #[test]
    fn the_rule_list_names_every_rule_once_with_its_kind() {
        let text = render(Some("rules")).unwrap();
        for rule in RULES {
            let line = text
                .lines()
                .find(|l| l.split_whitespace().next() == Some(rule.name))
                .unwrap_or_else(|| panic!("{} missing", rule.name));
            assert_eq!(line.contains(" advisory "), rule.advisory, "{line}");
            assert_eq!(line.contains(" opt-in "), rule.opt_in, "{line}");
        }
    }

    /// The config topic's example lines must parse as a project file, so the advice cannot
    /// name a key, rule or profile the checker rejects.
    #[test]
    fn the_config_example_is_a_valid_project_file() {
        let text = render(Some("config")).unwrap();
        let example: String = text
            .lines()
            .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
            .map(|l| {
                // Each setting is followed by 2 or more spaces and its explanation.
                let l = l.trim();
                l.find("  ").map_or(l, |at| &l[..at]).to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let settings =
            Settings::parse("laconic.toml", &example).unwrap_or_else(|e| panic!("{e}\n{example}"));
        assert_eq!(settings.protect.phrases(), ["Best-in-Class", "deep dive"]);
        assert!(text.contains(&Profile::NAMES.join(", ")));
        assert!(RuleId::named("serial-comma").is_some() && RuleId::named("em-dash").is_some());
    }
}
