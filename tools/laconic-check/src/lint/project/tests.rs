// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::lint::{RuleId, lint_counting};

fn rule(name: &str, phrases: &[(&str, Option<&str>)]) -> Rc<ProjectRule> {
    Rc::new(ProjectRule {
        name: format!("project/{name}"),
        phrases: phrases
            .iter()
            .map(|(text, replacement)| Phrase {
                text: (*text).into(),
                replacement: replacement.map(Into::into),
            })
            .collect(),
    })
}

/// Each project finding as (line, column, rule, message).
fn found(text: &str, rules: &[Rc<ProjectRule>]) -> Vec<(usize, usize, String, String)> {
    lint_counting(text, rules)
        .0
        .into_iter()
        .filter(|f| matches!(f.rule, RuleRef::Project(_)))
        .map(|f| (f.line, f.column, f.rule.name().to_owned(), f.message))
        .collect()
}

#[test]
fn a_lower_case_phrase_matches_any_case_and_a_capital_only_as_written() {
    let rules = [
        rule("plain", &[("utilise", Some("use"))]),
        rule("names", &[("Javascript", Some("JavaScript"))]),
    ];
    let text = "We utilise it. UTILISE it. Utilise Javascript, javascript and JavaScript.";
    assert_eq!(
        found(text, &rules),
        [
            (
                1,
                4,
                "project/plain".into(),
                "'utilise': the project file prefers 'use'".into()
            ),
            (
                1,
                16,
                "project/plain".into(),
                "'UTILISE': the project file prefers 'use'".into()
            ),
            (
                1,
                28,
                "project/plain".into(),
                "'Utilise': the project file prefers 'use'".into()
            ),
            (
                1,
                36,
                "project/names".into(),
                "'Javascript': the project file prefers 'JavaScript'".into()
            ),
        ]
    );
}

#[test]
fn a_match_written_as_its_replacement_is_not_a_finding() {
    let rules = [rule("names", &[("github", Some("GitHub"))])];
    let columns: Vec<usize> = found("GitHub, github and GITHUB.", &rules)
        .iter()
        .map(|f| f.1)
        .collect();
    assert_eq!(columns, [9, 20]);
}

#[test]
fn phrases_are_whole_words_and_hyphens_join_words() {
    let rules = [
        rule("plain", &[("utilise", Some("use"))]),
        rule("mail", &[("e-mail", Some("email"))]),
    ];
    let text = "We utilised it, re-utilise it, utilise-ready, e-mail and e-mailed.";
    let columns: Vec<usize> = found(text, &rules).iter().map(|f| f.1).collect();
    assert_eq!(columns, [47]);
}

#[test]
fn a_banned_phrase_matches_across_a_line_wrap_and_either_apostrophe() {
    let rules = [rule(
        "filler",
        &[("going forward", None), ("it's worth", None)],
    )];
    let text = "We will keep going\nforward. It\u{2019}s worth it, and going  forward too.";
    assert_eq!(
        found(text, &rules),
        [
            (
                1,
                14,
                "project/filler".into(),
                "'going forward': the project file bans this phrase".into()
            ),
            (
                2,
                10,
                "project/filler".into(),
                "'It\u{2019}s worth': the project file bans this phrase".into()
            ),
            (
                2,
                29,
                "project/filler".into(),
                "'going  forward': the project file bans this phrase".into()
            ),
        ]
    );
}

#[test]
fn code_urls_and_quotations_never_match() {
    let rules = [rule("plain", &[("utilise", Some("use"))])];
    let text = "Run `utilise` at https://e.com/utilise and say 'utilise this' once.\n\n\
                ```\nutilise\n```";
    assert_eq!(found(text, &rules), []);
}

#[test]
fn a_suppression_names_a_project_rule_and_an_unknown_one_is_reported() {
    let rules = [rule("plain", &[("utilise", Some("use"))])];
    let text = "<!-- laconic-check-disable-next-line project/plain -->\nWe utilise it.\n\n\
                <!-- laconic-check-disable-next-line project/other -->\nWe utilise it.\n";
    let (findings, suppressed) = lint_counting(text, &rules);
    assert_eq!(suppressed, 1);
    let named: Vec<(usize, &str)> = findings.iter().map(|f| (f.line, f.rule.name())).collect();
    assert_eq!(
        named,
        [(4, "unused-suppression"), (5, "project/plain")],
        "{findings:?}"
    );
    assert!(
        findings[0]
            .message
            .starts_with("'project/other' is not a rule")
    );
    assert!(!findings[1].rule.advisory());
    assert_eq!(findings[1].rule.url(), None);
    assert!(findings[1].rule != RuleId::UsSpelling);
}

#[test]
fn without_project_rules_the_house_rules_are_unchanged() {
    let text = "We utilise the color.";
    assert_eq!(lint_counting(text, &[]).0, crate::lint::lint(text));
    let with = lint_counting(text, &[rule("plain", &[("utilise", Some("use"))])]).0;
    assert_eq!(with.len(), 2);
    assert_eq!(with[0].rule, RuleId::UsSpelling);
    assert_eq!(with[1].rule.name(), "project/plain");
}

#[test]
fn the_description_lists_each_phrase() {
    let r = rule(
        "plain",
        &[("utilise", Some("use")), ("going forward", None)],
    );
    assert_eq!(
        r.description(),
        "The project file's rule for 'utilise' (write 'use'), 'going forward' (banned)."
    );
}
