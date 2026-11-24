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
            "f:1: unknown section '[rule]'; expected [rules] or [protect]",
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

#[test]
fn profile_and_opt_in_rules_follow_settings_and_accept_lists() {
    let report = Settings::parse("f", "profile = report\n").unwrap();
    assert_eq!(report.profile, Some(crate::profile::Profile::Report));
    assert!(report.needs_opt_in());
    assert!(!report.is_advisory(&RuleId::StackedHedge.into()));
    assert!(report.is_advisory(&RuleId::UnglossedTerm.into()));

    let bad = Settings::parse("f", "profile = memo\n").unwrap_err();
    assert!(bad.contains("unknown profile 'memo'"), "{bad}");

    let docs = Settings::parse("f", "profile = docs\n").unwrap();
    assert!(docs.needs_opt_in());
    assert!(docs.is_advisory(&RuleId::NumberWord.into()));

    let only_opt = Settings::parse("f", "only = unglossed-term\n").unwrap();
    assert!(only_opt.needs_opt_in());
    let disabled_opt =
        Settings::parse("f", "only = unglossed-term\ndisable = unglossed-term\n").unwrap();
    assert!(!disabled_opt.needs_opt_in());

    let text = "The QBRs ran.\n";
    let found = lint::lint_all(text);
    assert_eq!(found.len(), 1);
    assert!(!Settings::default().keeps(&found[0], text));
    assert!(report.keeps(&found[0], text));
    assert!(only_opt.keeps(&found[0], text));

    let accepted = Settings::parse("f", "profile = report\naccept = QBR\n").unwrap();
    assert!(!accepted.keeps(&found[0], text));

    let bold = "- **One:** a\n- **Two:** b\n- **Three:** c\n";
    let bold_findings = lint::lint(bold);
    assert_eq!(bold_findings.len(), 1);
    assert!(!docs.keeps(&bold_findings[0], "- **Three:** c"));
}

#[test]
fn toml_syntax_and_protect_exemptions_work_across_rules() {
    let toml = "\
profile = \"chat\"
max_findings = 2
disable = [\"serial-comma\"]
accept = [\"color\", \"QBR\"]

[protect]
phrases = [
    \"deep dive\",
    \"gates and seams\",
    'here\u{2019}s the thing',
    \"Best-in-Class\",
]
bottom line:
\"moving pieces\"
";
    let settings = Settings::parse("laconic.toml", toml).unwrap();
    assert_eq!(settings.profile, Some(crate::profile::Profile::Chat));
    assert_eq!(settings.max_findings, 2);
    assert_eq!(settings.disable, ["serial-comma"]);
    assert_eq!(settings.accept, ["color", "QBR"]);
    assert_eq!(
        settings.protect.phrases(),
        [
            "deep dive",
            "gates and seams",
            "here\u{2019}s the thing",
            "Best-in-Class",
            "bottom line:",
            "moving pieces",
        ]
    );

    // Protected phrase exempts only the matching span, not other findings on the same line.
    let line = "A deep dive into gates and seams, plus circle back.\n";
    let found = lint::lint_all(line);
    let kept: Vec<&str> = found
        .iter()
        .filter(|f| settings.keeps(f, line))
        .map(|f| f.rule.name())
        .collect();
    assert_eq!(kept, ["stock-phrase"]);

    // Case-sensitive protection when phrase contains an uppercase letter.
    let report = Settings::parse(
        "laconic.toml",
        "profile = \"report\"\n[protect]\nphrases = [\"Best-in-Class\"]\n",
    )
    .unwrap();
    let upper = "This is a Best-in-Class parser.\n";
    let lower = "This is a best-in-class parser.\n";
    assert!(
        lint::lint_all(upper)
            .iter()
            .all(|f| !report.keeps(f, upper))
    );
    assert!(lint::lint_all(lower).iter().any(|f| report.keeps(f, lower)));

    // Curly/straight apostrophe equivalence and whitespace collapse.
    let curly_line = "Here's   the thing about moving pieces.\n";
    assert!(
        lint::lint_all(curly_line)
            .iter()
            .all(|f| !settings.keeps(f, curly_line))
    );
}

#[test]
fn toml_and_protect_parse_errors_and_edge_cases() {
    for (text, wanted) in [
        ("profile = \"unterminated\n", "f:1: unterminated string"),
        ("profile = 'unterminated\n", "f:1: unterminated string"),
        ("profile = \"bad\\\n", "f:1: unterminated escape"),
        (
            "disable = [\"em-dash\"\n",
            "f:1: unterminated list for 'disable'",
        ),
        (
            "[protect]\nphrases = [\"deep dive\"\n",
            "f:2: unterminated list for 'phrases'",
        ),
        (
            "[protect]\nother = [\"x\"]\n",
            "f:2: unknown key 'other' in [protect]",
        ),
        ("[protect]\n\"\"\n", "f:2: protected phrase cannot be empty"),
        (
            "[protect]\nuse `code`\n",
            "f:2: 'use `code`' is not a valid protected phrase",
        ),
        (
            "only = [\"em-dash\", \"unterminated]\n",
            "f:1: unterminated list for 'only'",
        ),
        (
            "accept = [\"good\", \"bad word\"]\n",
            "f:1: 'bad word' is not one word of letters",
        ),
        (
            "accept = [\n\"bad word\"\n]\n",
            "f:1: 'bad word' is not one word of letters",
        ),
        (
            "accept = [\"good\"\n",
            "f:1: unterminated list for 'accept'",
        ),
    ] {
        let err = Settings::parse("f", text).unwrap_err();
        assert!(err.starts_with(wanted), "{text:?}: {err}");
    }

    // Escaped quotes, comments in multiline arrays, bare items in brackets, and find_in precedence.
    let escaped = Settings::parse(
        "f",
        "only = [em-dash, # comment\n\"us-spelling\"]\n[protect] # section note\nphrases = [\"a \\\"b\\\" \\\\ \\u2019 \\U00002019\"] # trailing note\n",
    )
    .unwrap();
    assert_eq!(escaped.only, ["em-dash", "us-spelling"]);
    assert_eq!(
        Settings::parse("f", "profile = 'report' # comment\n")
            .unwrap()
            .profile,
        Some(crate::profile::Profile::Report)
    );
    assert_eq!(escaped.protect.phrases(), ["a \"b\" \\ \u{2019} \u{2019}"]);
    assert_eq!(
        protect::decode_value("\"\\b\\f\\n\\r\\t\\\\\\\"\"").unwrap(),
        "\u{0008}\u{000c}\n\r\t\\\""
    );
    for bad in [
        "\"a\\\"",
        "\"a\\x\"",
        "\"a\\u12\"",
        "\"a\\u12",
        "\"a\\u12zz\"",
        "\"a\\uD800\"",
        "\"ok\" trailing",
        "'ok' trailing",
        "\"a\\",
    ] {
        assert!(protect::decode_value(bad).is_err(), "{bad}");
    }
    for bad_list in [
        "[\"a",
        "[\"a\"",
        "[\"a\\\"",
        "[\"a\\]",
        "[\"a]",
        "[\"a\" \"b\"]",
        "[, \"a\"]",
        "['a]",
        "[\"a\"] trailing",
    ] {
        assert!(protect::parse_list(bad_list).is_err(), "{bad_list}");
    }
    assert!(
        protect::decode_value("\"a\\x\"")
            .unwrap_err()
            .contains("unsupported escape")
    );
    assert!(
        protect::parse_list("[\"a\" \"b\"]")
            .unwrap_err()
            .contains("expected ','")
    );

    let compiled = |s: &str| vec![protect::CompiledPhrase::new(s.to_owned())];
    assert!(!protect::is_protected(&[], "abc", 1));
    assert!(!protect::is_protected(&compiled("abc"), "abc", 0));
    assert!(!protect::contains_column(&[(0, 3)], 0));
    assert!(!protect::contains_column(&[], 1));
    assert!(!protect::is_protected(
        &compiled("deep dive"),
        "xdeep dive",
        2
    ));
    assert!(!protect::is_protected(
        &compiled("deep dive"),
        "deep divex",
        1
    ));
    assert!(!protect::is_protected(
        &compiled("deep dive"),
        "deepdive",
        1
    ));

    let dir = std::env::temp_dir().join(format!("laconic-find-in-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(find_in(&dir), None);
    std::fs::write(dir.join(PROJECT_FILE), "disable = em-dash\n").unwrap();
    assert_eq!(find_in(&dir), Some(dir.join(PROJECT_FILE)));
    std::fs::write(dir.join(TOML_PROJECT_FILE), "disable = [\"em-dash\"]\n").unwrap();
    assert_eq!(find_in(&dir), Some(dir.join(TOML_PROJECT_FILE)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multiline_toml_array_comments_with_apostrophes_or_brackets_parse_cleanly() {
    let toml = "\
[protect]
phrases = [
    # don't flag this idiom [keep]
    \"deep dive\", # author's signature
    \"gates and seams\",
]
";
    let settings = Settings::parse("laconic.toml", toml).unwrap();
    assert_eq!(settings.protect.phrases(), ["deep dive", "gates and seams"]);
}

#[test]
fn protect_never_double_unquotes_and_supports_toml_comments_and_format_message() {
    let toml = "\
profile = \"chat\" # chat mode
max_findings = 3 # limit
[protect] # author idioms
phrases = [\"'tis the season\", '\"deep dive\"'] # inline comment
\"'tis another\" # line comment
'\"literal phrase\"'
";
    let settings = Settings::parse("laconic.toml", toml).unwrap();
    assert_eq!(settings.max_findings, 3);
    assert_eq!(
        settings.protect.phrases(),
        [
            "'tis the season",
            "\"deep dive\"",
            "'tis another",
            "\"literal phrase\"",
        ]
    );

    assert_eq!(
        settings.format_message(
            &RuleId::ChatFiller.into(),
            "architectural filler 'gates and seams' (advisory); name the module"
        ),
        "architectural filler 'gates and seams'; name the module"
    );
    let docs = Settings::parse("laconic.toml", "profile = \"docs\"\n").unwrap();
    assert_eq!(
        docs.format_message(&RuleId::NumberWord.into(), "write '3'"),
        "write '3' (advisory)"
    );
    assert_eq!(
        docs.format_message(&RuleId::StackedHedge.into(), "stacked hedge (advisory)"),
        "stacked hedge (advisory)"
    );
}

#[test]
fn cached_protection_handles_long_line_with_many_findings_and_overlapping_spans() {
    let settings = Settings::parse(
        "laconic.toml",
        "[protect]\nphrases = [\"deep dive\", \"dive into\"]\n",
    )
    .unwrap();
    let unit = "Take a deep dive into color. ";
    let line = unit.repeat(1_500);
    let found = lint::lint(&line);
    assert!(found.len() >= 1_500, "got {}", found.len());
    let mut cache = LineProtectCache::default();
    let kept: Vec<&str> = found
        .iter()
        .filter(|f| settings.keeps_cached(f, &line, &mut cache))
        .map(|f| f.rule.name())
        .collect();
    assert_eq!(kept.len(), 1_500);
    assert!(kept.iter().all(|&r| r == "us-spelling"));
    assert!(!cache.contains(&[], 1, &line, 1));
    assert!(!cache.contains(settings.protect.items(), 1, &line, 0));
}

#[test]
fn protect_unicode_casefold_bare_comments_and_project_advisory_literal_stay_exact() {
    // SHOULD 4 & 5: mid-word apostrophes and `=` inside trailing comments under [protect].
    let toml = "\
[protect]
here's the thing # idiom
\"deep dive\" # style = personal
moving pieces # style = personal
";
    let settings = Settings::parse("laconic.toml", toml).unwrap();
    assert_eq!(
        settings.protect.phrases(),
        ["here's the thing", "deep dive", "moving pieces"]
    );

    // SHOULD 2: 'İ' (U+0130) expands to 'i' + U+0307 and must not match plain 'i'.
    let unicode =
        Settings::parse("laconic.toml", "[protect]\nphrases = [\"three items\"]\n").unwrap();
    let plain = "We have three items.\n";
    let dotted = "We have three \u{0130}tems.\n";
    assert!(lint::lint(plain).iter().all(|f| !unicode.keeps(f, plain)));
    assert!(
        lint::lint(dotted)
            .iter()
            .any(|f| f.rule == RuleId::NumberWord && unicode.keeps(f, dotted))
    );

    // SHOULD 3: literal ` (advisory)` in user `[rules]` replacement is preserved verbatim.
    let project = Settings::parse(
        ".laconic-check",
        "[rules]\nplain: utilise = use (advisory)\n",
    )
    .unwrap();
    let text = "We utilise it.\n";
    let found = lint::lint_counting(text, &project.rules).0;
    assert_eq!(found.len(), 1);
    assert_eq!(
        project.format_message(&found[0].rule, &found[0].message),
        "'utilise': the project file prefers 'use (advisory)'"
    );
}
