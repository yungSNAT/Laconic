// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

#[test]
fn reads_options_in_either_form_and_only_files_after_a_double_dash() {
    let read = options(&args(&[
        "--json",
        "--disable=us-spelling, em-dash",
        "a.md",
        "--only",
        "word-choice",
        "--max-findings",
        "3",
        "-",
        "--",
        "--warn",
    ]))
    .unwrap();
    assert_eq!(
        read,
        Options {
            json: true,
            settings: Settings {
                disable: vec!["us-spelling".into(), "em-dash".into()],
                only: vec!["word-choice".into()],
                max_findings: 3,
                ..Settings::default()
            },
            files: args(&["a.md", "-", "--warn"]),
            ..Options::default()
        }
    );
}

#[test]
fn rejects_unknown_rules_options_and_numbers() {
    for (list, wanted) in [
        (
            &["--disable", "us-spellings"][..],
            "unknown rule 'us-spellings'",
        ),
        (&["--colour"], "unknown option '--colour'"),
        (&["-x"], "unknown option '-x'"),
        (&["--max-findings=-1"], "'-1' is not a whole number"),
        (&["--max-findings"], "--max-findings needs a value"),
        (&["--warn=yes"], "--warn takes no value"),
        (&["--profile=memo"], "unknown profile 'memo'"),
        (&["--profile"], "--profile needs a value"),
        (&["--sarif", "--metrics"], "--sarif and --metrics"),
        (&["--format=xml"], "unknown format 'xml'"),
        (&["--format"], "--format needs a value"),
        (
            &["--format=text", "--json"],
            "--format=text cannot be combined",
        ),
        (&["--format=json", "--sarif"], "--json and --sarif"),
    ] {
        let error = options(&args(list)).unwrap_err();
        assert!(error.starts_with(wanted), "{list:?}: {error}");
    }
    let with_format = options(&args(&["--format", "json", "a.md"])).unwrap();
    assert!(with_format.json && !with_format.sarif);
    let with_sarif = options(&args(&["--format=sarif", "a.md"])).unwrap();
    assert!(with_sarif.sarif && !with_sarif.json);
    let with_text = options(&args(&["--format=text", "a.md"])).unwrap();
    assert!(!with_text.json && !with_text.sarif);
    let with_profile = options(&args(&["--profile", "report", "--metrics", "a.md"])).unwrap();
    assert!(with_profile.metrics);
    assert_eq!(
        with_profile.settings.profile,
        Some(laconic_check::profile::Profile::Report)
    );
}

#[test]
fn the_command_line_adds_rules_to_the_project_file_and_overrides_its_count() {
    let dir = std::env::temp_dir().join(format!("laconic-check-options-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let config = dir.join("settings");
    fs::write(
        &config,
        "\u{feff}# settings\n\ndisable = em-dash,\nonly = us-spelling\nmax-findings = 4\n",
    )
    .unwrap();
    let config = config.to_str().unwrap();
    let read = options(&args(&["--config", config, "--disable", "latin", "a.md"])).unwrap();
    assert_eq!(read.settings.disable, ["em-dash", "latin"]);
    assert_eq!(read.settings.only, ["us-spelling"]);
    assert_eq!(read.settings.max_findings, 4);
    let read = options(&args(&["--config", config, "--max-findings", "0", "a.md"])).unwrap();
    assert_eq!(read.settings.max_findings, 0);
    let missing = dir.join("missing");
    let error = options(&args(&["--config", missing.to_str().unwrap()])).unwrap_err();
    assert!(error.starts_with(missing.to_str().unwrap()), "{error}");
}

#[test]
fn a_commit_message_keeps_what_git_keeps_on_the_same_lines() {
    let message = "fix: stop the color\n\n# Please enter the commit message\nBody text.\n\
                   # ------------------------ >8 ------------------------\n\
                   # Do not modify or remove the line above.\n+color\n";
    assert_eq!(
        commit_message(message),
        "fix: stop the color\n\n\nBody text.\n"
    );
    assert_eq!(commit_message("one\r\n#two\r\n"), "one\n\n");
}
