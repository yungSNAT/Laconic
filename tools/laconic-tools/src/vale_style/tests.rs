// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use std::collections::BTreeSet;

use laconic_check::lint::lint;

use crate::root;

fn fixture(name: &str) -> String {
    let path = root().join("vale/tests").join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .replace("\r\n", "\n")
}

/// The `lint` rule a generated Vale rule follows: its file name, less any suffix after the
/// rule's own name.
fn lint_rule(vale_rule: &str) -> &'static str {
    RULES
        .iter()
        .map(|r| r.name)
        .filter(|name| vale_rule.starts_with(name))
        .max_by_key(|name| name.len())
        .unwrap_or_else(|| panic!("'{vale_rule}' follows no lint rule"))
}

#[test]
fn the_repository_style_matches_the_rule_table() {
    assert_eq!(
        run(&root(), true),
        Ok(format!("{STYLE_DIR} matches lint's rule table"))
    );
}

#[test]
fn each_lint_rule_is_generated_or_listed_with_a_reason_and_documented() {
    let generated: BTreeSet<&str> = rules().iter().map(|(n, _)| lint_rule(n)).collect();
    let listed: BTreeSet<&str> = NOT_GENERATED.iter().map(|(n, _)| *n).collect();
    assert!(generated.is_disjoint(&listed), "{generated:?} {listed:?}");
    let every: BTreeSet<&str> = RULES.iter().map(|r| r.name).collect();
    assert_eq!(
        &generated | &listed,
        every,
        "a rule is neither generated nor listed"
    );
    let readme = fs::read_to_string(root().join("vale/README.md")).unwrap();
    for name in &every {
        assert!(
            readme.contains(&format!("`{name}`")),
            "vale/README.md: {name}"
        );
    }
}

/// The expected Vale output for `faults.md`, which the `vale-style` workflow compares with
/// Vale's own, as (line, `lint` rule) pairs.
fn vale_expected() -> BTreeSet<(usize, &'static str)> {
    fixture("faults.expected")
        .lines()
        .map(|line| {
            let mut fields = line.splitn(5, ':');
            let (Some("faults.md"), Some(number), Some(_), Some(check)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                panic!("not a Vale line: {line}");
            };
            let rule = check
                .strip_prefix("Laconic.")
                .unwrap_or_else(|| panic!("not a Laconic rule: {line}"));
            (number.parse().unwrap(), lint_rule(rule))
        })
        .collect()
}

#[test]
fn vale_flags_the_lines_and_rules_lint_flags_in_the_fixtures() {
    assert_eq!(lint(&fixture("clean.md")), []);
    let listed: Vec<&str> = NOT_GENERATED.iter().map(|(n, _)| *n).collect();
    let from_lint: BTreeSet<(usize, &str)> = lint(&fixture("faults.md"))
        .iter()
        .map(|f| (f.line, f.rule.name()))
        .filter(|(_, rule)| !listed.contains(rule))
        .collect();
    assert_eq!(from_lint, vale_expected());
    // Every generated rule is exercised.
    let exercised: BTreeSet<&str> = from_lint.iter().map(|(_, rule)| *rule).collect();
    let generated: BTreeSet<&str> = rules().iter().map(|(n, _)| lint_rule(n)).collect();
    assert_eq!(exercised, generated);
}

#[test]
fn literals_escape_regex_syntax_and_match_either_apostrophe() {
    assert_eq!(literal("bottom line:"), r"bottom\s+line:");
    assert_eq!(literal("it's (a.b)"), r"it['’]s\s+\(a\.b\)");
    assert_eq!(
        phrases(&["in short:", "deep dive"]),
        r"(?:\bin\s+short:|\bdeep\s+dive\b)"
    );
}

#[test]
fn yaml_quotes_double_a_single_quote() {
    assert_eq!(quote("it's"), "'it''s'");
}

#[test]
fn writing_reports_stale_rules_and_refuses_strays() {
    let dir = std::env::temp_dir().join(format!("laconic-vale-{}", std::process::id()));
    let style = dir.join(STYLE_DIR);
    assert!(run(&dir, true).unwrap_err().contains("differs"));
    assert!(run(&dir, false).unwrap().contains(": wrote "));
    assert!(run(&dir, true).is_ok());
    fs::write(style.join("em-dash.yml"), "changed").unwrap();
    assert!(run(&dir, true).unwrap_err().contains("(em-dash)"));
    assert!(run(&dir, false).unwrap().contains(": wrote 1 of "));
    // A Windows checkout's CRLF endings are not a difference.
    let crlf = fs::read_to_string(style.join("em-dash.yml"))
        .unwrap()
        .replace('\n', "\r\n");
    fs::write(style.join("em-dash.yml"), crlf).unwrap();
    assert!(run(&dir, true).is_ok());
    fs::write(style.join("Old.yml"), "").unwrap();
    assert!(run(&dir, false).unwrap_err().contains("Old.yml"));
    fs::remove_dir_all(&dir).unwrap();
}
