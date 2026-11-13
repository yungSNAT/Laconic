// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Top-level options: `--version` and `--help` succeed on standard output; anything else
//! unknown is a usage error on standard error.

use std::process::Command;

fn run(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn version_prints_the_crate_version_and_succeeds() {
    let wanted = format!("laconic-check {}\n", env!("CARGO_PKG_VERSION"));
    assert_eq!(run(&["--version"]), (0, wanted, String::new()));
}

#[test]
fn help_prints_usage_to_standard_output_and_succeeds() {
    let (code, out, err) = run(&["--help"]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(out.starts_with("usage:\n  laconic-check lint"), "{out}");
    assert!(out.contains("--version prints the version"), "{out}");
}

#[test]
fn an_unknown_option_is_still_a_usage_error() {
    for args in [&["--bogus"][..], &["--version", "extra"], &[]] {
        let (code, out, err) = run(args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(err.starts_with("usage:\n"), "{args:?}: {err}");
    }
}
