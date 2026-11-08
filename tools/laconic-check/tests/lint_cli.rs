// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `lint` exit statuses, filters and project file, and the exit-status table in README.md.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh directory for one test, holding the given files.
fn folder(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("laconic-check-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (file, text) in files {
        std::fs::write(dir.join(file), text).unwrap();
    }
    dir
}

/// Runs `lint` in `dir` and returns its exit status, standard output and standard error.
fn lint(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .current_dir(dir)
        .arg("lint")
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

const TWO: &str = "The color is red.\n\nThe flavor is sweet.\n";

#[test]
fn fails_only_on_more_failing_findings_than_allowed() {
    let dir = folder(
        "count",
        &[
            ("two.md", TWO),
            ("clean.md", "The colour is red.\n"),
            ("advisory.md", "It could potentially rain.\n"),
        ],
    );
    assert_eq!(lint(&dir, &["clean.md"]).0, 0);
    assert_eq!(lint(&dir, &["advisory.md"]).0, 0);
    assert_eq!(lint(&dir, &["two.md"]).0, 1);
    assert_eq!(lint(&dir, &["--max-findings", "1", "two.md"]).0, 1);
    assert_eq!(lint(&dir, &["--max-findings=2", "two.md"]).0, 0);
    assert_eq!(lint(&dir, &["--warn", "two.md"]).0, 0);
}

#[test]
fn filters_drop_findings_before_they_are_counted() {
    let dir = folder(
        "filters",
        &[("mixed.md", "The color is red, e.g. crimson.\n")],
    );
    let (code, out, _) = lint(&dir, &["--disable", "us-spelling", "mixed.md"]);
    assert_eq!(
        (code, out.as_str()),
        (
            1,
            "mixed.md:1:19: latin: Latin abbreviation; write 'for example' or 'that is'\n"
        )
    );
    let (code, out, _) = lint(&dir, &["--only", "us-spelling", "mixed.md"]);
    assert_eq!(code, 1);
    assert!(out.starts_with("mixed.md:1:5: us-spelling: "), "{out}");
    assert_eq!(
        lint(
            &dir,
            &[
                "--only",
                "us-spelling",
                "--disable",
                "us-spelling",
                "mixed.md"
            ]
        )
        .0,
        0
    );
}

#[test]
fn reads_the_project_file_in_the_current_directory() {
    let with = folder(
        "project",
        &[
            ("two.md", TWO),
            (".laconic-check", "# house rules\ndisable = us-spelling\n"),
        ],
    );
    assert_eq!(lint(&with, &["two.md"]), (0, String::new(), String::new()));
    assert_eq!(lint(&with, &["--config", "two.md", "two.md"]).0, 2);
    let without = folder("no-project", &[("two.md", TWO)]);
    assert_eq!(lint(&without, &["two.md"]).0, 1);
}

#[test]
fn usage_settings_and_read_errors_exit_2() {
    let dir = folder(
        "errors",
        &[("two.md", TWO), (".laconic-check", "severity = high\n")],
    );
    for args in [
        &[][..],
        &["--disable", "us-spellings", "two.md"],
        &["--colour", "two.md"],
        &["missing.md"],
        &["two.md"],
    ] {
        let (code, out, err) = lint(&dir, args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(!err.is_empty(), "{args:?}");
    }
}

#[test]
fn json_reports_every_finding_and_the_counts() {
    let quiet = "<!-- laconic-check-disable-next-line us-spelling -->\nThe flavor is sweet.\n";
    let dir = folder("json", &[("two.md", TWO), ("quiet.md", quiet)]);
    let (code, out, _) = lint(&dir, &["--json", "two.md", "quiet.md", "two.md"]);
    assert_eq!(code, 1);
    let finding = |line: usize, column: usize, message: &str| {
        format!(
            r#"{{"file":"two.md","line":{line},"column":{column},"rule":"us-spelling","advisory":false,"message":"{message}","url":"https://github.com/yungSNAT/Laconic/blob/main/docs/rules.md#us-spelling"}}"#
        )
    };
    let color = finding(1, 5, "'color': Australian form is 'colour'");
    let flavor = finding(3, 5, "'flavor': Australian form is 'flavour'");
    assert_eq!(
        out,
        format!(r#"{{"findings":[{color},{flavor},{color},{flavor}],"failing":4,"suppressed":1}}"#)
            + "\n"
    );
    let clean = folder("json-clean", &[("clean.md", "Fine.\n")]);
    assert_eq!(
        lint(&clean, &["--json", "clean.md"]),
        (
            0,
            "{\"findings\":[],\"failing\":0,\"suppressed\":0}\n".into(),
            String::new()
        )
    );
}

#[test]
fn a_commit_message_is_linted_without_its_comments_or_diff() {
    let dir = folder(
        "commit-msg",
        &[(
            "COMMIT_EDITMSG",
            "fix: stop the flavor\n\n# Changes to be committed: color\n\
             # ------------------------ >8 ------------------------\n+color\n",
        )],
    );
    let (code, out, _) = lint(&dir, &["--commit-msg", "COMMIT_EDITMSG"]);
    assert_eq!(code, 1);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.starts_with("COMMIT_EDITMSG:1:15: us-spelling: "),
        "{out}"
    );
}

/// README.md's exit-status table lists exactly the statuses `main.rs` can return.
#[test]
fn the_readme_lists_every_exit_status() {
    let readme =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")).unwrap();
    let table: Vec<&str> = readme
        .lines()
        .skip_while(|line| !line.starts_with("| Exit status |"))
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|row| row.split('|').nth(1).unwrap().trim())
        .collect();
    let main = include_str!("../src/main.rs");
    let mut codes: Vec<&str> = main
        .split("ExitCode::from(")
        .skip(1)
        .map(|rest| rest.split(')').next().unwrap())
        .collect();
    if main.contains("ExitCode::SUCCESS") {
        codes.push("0");
    }
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(table, codes);
}

/// Golden outputs: `tests/golden/<case>.out` holds `lint`'s exact output on
/// `tests/golden/input.md` for each case, so any change to either format fails here. To
/// update them after a deliberate change, run `LACONIC_WRITE_GOLDEN=1 cargo test golden`.
#[test]
fn golden_outputs_are_unchanged() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut changed = Vec::new();
    for (case, args, status) in [
        ("text", &[][..], 1),
        ("json", &["--json"], 1),
        (
            "only",
            &["--json", "--only", "us-spelling,stacked-hedge"],
            1,
        ),
    ] {
        let args: Vec<&str> = args.iter().copied().chain(["input.md"]).collect();
        let (code, out, err) = lint(&dir, &args);
        assert_eq!((code, err.as_str()), (status, ""), "{case}");
        let path = dir.join(format!("{case}.out"));
        if std::env::var_os("LACONIC_WRITE_GOLDEN").is_some() {
            std::fs::write(&path, &out).unwrap();
        } else if std::fs::read_to_string(&path)
            .map(|s| s.replace("\r\n", "\n"))
            .ok()
            != Some(out)
        {
            changed.push(case);
        }
    }
    assert!(changed.is_empty(), "golden outputs differ: {changed:?}");
}
