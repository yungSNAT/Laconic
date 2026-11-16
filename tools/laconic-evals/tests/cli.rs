// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The binary's exit statuses and streams, as the Python entry points gave them.

use std::fs;
use std::process::{Command, Output};

fn evals(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_laconic-evals"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn bad_arguments_exit_2_with_the_usage() {
    for args in [
        &[][..],
        &["fact-survival", "spend", "folder"],
        &[
            "fact-audit",
            "--laconic-check",
            "/nonexistent/laconic-check",
        ],
    ] {
        let out = evals(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            text(&out.stderr).contains("Usage: laconic-evals"),
            "{}",
            text(&out.stderr)
        );
    }
    let out = evals(&[
        "fact-audit",
        "--laconic-check",
        "/nonexistent/laconic-check",
    ]);
    assert!(
        text(&out.stderr)
            .contains("--laconic-check /nonexistent/laconic-check is not an executable file")
    );
}

#[test]
fn spend_prints_the_cost_and_exits_1_at_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("overhead.jsonl"),
        "{\"stage\": \"preflight\", \"cost_usd\": 1.5}\n",
    )
    .unwrap();
    let folder = dir.path().to_str().unwrap();
    let out = evals(&["fact-survival", "spend", folder, "--cap", "2"]);
    assert_eq!(
        (out.status.code(), text(&out.stdout)),
        (Some(0), "reported cost: $1.50 of $2.00\n".into())
    );
    let out = evals(&["fact-survival", "spend", folder, "--cap", "1.5"]);
    assert_eq!(
        (out.status.code(), text(&out.stdout)),
        (Some(1), "reported cost: $1.50 of $1.50\n".into())
    );
}

#[test]
fn failures_exit_1_with_the_reason() {
    let out = evals(&[
        "fact-survival",
        "report",
        "missing.jsonl",
        "missing.jsonl",
        "--cases",
        "missing.jsonl",
        "--reference",
        "a",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).starts_with("laconic-evals: missing.jsonl: "),
        "{}",
        text(&out.stderr)
    );
    assert!(out.stdout.is_empty());
}

/// The judge command runs the claude CLI found on the PATH; a stand-in answers here.
#[cfg(unix)]
#[test]
fn calibrate_calls_the_claude_cli_on_the_path() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join("claude");
    let reply = r#"{"result": "{\"items\": [], \"invented\": []}", "total_cost_usd": 0.5}"#;
    fs::write(
        &claude,
        format!("#!/bin/sh\ncat > /dev/null\nprintf '%s' '{reply}'\n"),
    )
    .unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let cases = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../evals/fact-cases.jsonl"
    ))
    .unwrap();
    let id = cases.split('"').nth(3).unwrap().to_owned();
    let fixtures = dir.path().join("fixtures.jsonl");
    let fixture = format!(
        r#"{{"name": "copy", "case_id": "{id}", "rewrite": "@source", "default": "dropped", "expected": {{}}, "invented": false}}"#
    );
    fs::write(&fixtures, fixture).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-evals"))
        .args([
            "fact-survival",
            "calibrate",
            "--fixtures",
            fixtures.to_str().unwrap(),
        ])
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).contains("\"agreement\": 1.0,"),
        "{}",
        text(&out.stdout)
    );
}
