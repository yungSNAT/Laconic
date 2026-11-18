// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

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
fn a_rules_section_error_names_its_line() {
    for (text, wanted) in [
        (
            "[rule]\n",
            "f:1: unknown section '[rule]'; the only section is [rules]",
        ),
        (
            "[rules]\n\nplain: utilise = use\ndisable = em-dash\n",
            "f:4: expected 'id: phrase = replacement', found 'disable = em-dash'; settings go before [rules]",
        ),
        (
            "[rules]\nplain utilise\n",
            "f:2: expected 'id: phrase = replacement'",
        ),
        (
            "[rules]\nPlain: utilise = use\n",
            "f:2: 'Plain' is not a rule id",
        ),
        (
            "[rules]\n-plain: utilise = use\n",
            "f:2: '-plain' is not a rule id",
        ),
        ("[rules]\n: utilise = use\n", "f:2: '' is not a rule id"),
        (
            "[rules]\nplain: = use\n",
            "f:2: '' is not a phrase of words",
        ),
        (
            "[rules]\nplain: use `this` = that\n",
            "f:2: 'use `this`' is not a phrase of words",
        ),
        (
            "[rules]\nplain: -utilise = use\n",
            "f:2: '-utilise' is not a phrase of words",
        ),
        (
            "[rules]\nplain: going  forward =\nother: going forward = next\n",
            "f:3: 'going forward' is already a rule on line 2",
        ),
        (
            "disable = project/plain\n[rules]\nother: utilise = use\n",
            "f:1: unknown rule 'project/plain'; the project file's [rules] section does not name it",
        ),
    ] {
        let error = Settings::parse("f", text).unwrap_err();
        assert!(error.starts_with(wanted), "{text:?}: {error}");
    }
}

/// A rule's name and its phrases with their replacements.
type NamedPhrases<'a> = (&'a str, Vec<(&'a str, Option<&'a str>)>);

#[test]
fn rules_group_phrases_by_id_and_disable_may_name_one_defined_later() {
    let text = "disable = project/filler\n\n[rules]\n# Plain words.\n\
                plain: utilise = use\nfiller: going   forward =\nplain: it\u{2019}s e-mail = it's email\n\
                [rules]\nfiller: Moreover =\n";
    let settings = Settings::parse("f", text).unwrap();
    assert_eq!(settings.disable, ["project/filler"]);
    let rules: Vec<NamedPhrases> = settings
        .rules
        .iter()
        .map(|r| {
            let phrases = r
                .phrases
                .iter()
                .map(|p| (p.text.as_str(), p.replacement.as_deref()))
                .collect();
            (r.name.as_str(), phrases)
        })
        .collect();
    assert_eq!(
        rules,
        [
            (
                "project/plain",
                vec![
                    ("utilise", Some("use")),
                    ("it\u{2019}s e-mail", Some("it's email"))
                ]
            ),
            (
                "project/filler",
                vec![("going forward", None), ("Moreover", None)]
            ),
        ]
    );
    assert_eq!(settings.check_rule("project/plain"), Ok(()));
    assert_eq!(settings.check_rule("em-dash"), Ok(()));
    assert!(settings.check_rule("project/other").is_err());
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
    let text = "The color \u{2014} red; we utilise it.";
    let settings = Settings::parse("f", "[rules]\nplain: utilise = use\n").unwrap();
    let found = lint::lint_counting(text, &settings.rules).0;
    let names = |extra: &str| -> Vec<&str> {
        let settings =
            Settings::parse("f", &format!("{extra}\n[rules]\nplain: utilise = use")).unwrap();
        let mut kept: Vec<&str> = found
            .iter()
            .filter(|f| settings.keeps(f, text))
            .map(|f| f.rule.name())
            .collect();
        kept.sort_unstable();
        kept
    };
    assert_eq!(names(""), ["em-dash", "project/plain", "us-spelling"]);
    assert_eq!(names("only = em-dash"), ["em-dash"]);
    assert_eq!(names("only = project/plain"), ["project/plain"]);
    assert_eq!(names("disable = project/plain"), ["em-dash", "us-spelling"]);
    assert!(names("only = em-dash\ndisable = em-dash").is_empty());
}
