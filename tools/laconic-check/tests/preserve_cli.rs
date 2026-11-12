// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `preserve` exit statuses: a drop fails, an addition fails only with `--strict`.

use std::path::PathBuf;
use std::process::Command;

fn files(name: &str, original: &str, edited: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("laconic-check-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (o, e) = (dir.join("original.md"), dir.join("edited.md"));
    std::fs::write(&o, original).unwrap();
    std::fs::write(&e, edited).unwrap();
    (o, e)
}

fn preserve(args: &[&str], (o, e): &(PathBuf, PathBuf)) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .arg("preserve")
        .args(args)
        .arg(o)
        .arg(e)
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
    )
}

#[test]
fn an_invented_figure_is_advisory_unless_strict() {
    let pair = files(
        "added",
        "Refund the fee.",
        "You must refund the $1,200 fee.",
    );
    let (code, out) = preserve(&[], &pair);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "No tracked items dropped; meaning not checked.\nadded meaning word: 'must' (0 before, 1 after)\nadded number: $1,200\nlength: 3 to 6 words (200% of the original)\n"
    );
    assert_eq!(preserve(&["--strict"], &pair).0, 1);
}

#[test]
fn a_dropped_figure_fails_with_or_without_strict() {
    let pair = files("dropped", "Refund the $1,200 fee.", "Refund the fee.");
    let (code, out) = preserve(&[], &pair);
    assert_eq!(
        (code, out.as_str()),
        (
            1,
            "dropped number: $1,200\nlength: 4 to 3 words (75% of the original)\n"
        )
    );
    assert_eq!(preserve(&["--strict"], &pair).0, 1);
}

#[test]
fn a_faithful_edit_passes_strict() {
    let pair = files(
        "same",
        "Due on 15 April; it must not run.",
        "It must not run. Due on 15 April.",
    );
    assert_eq!(
        preserve(&["--strict"], &pair),
        (
            0,
            "No tracked items dropped; meaning not checked.\nlength: 8 to 8 words (100% of the original)\n"
                .into()
        )
    );
}

#[test]
fn a_dropped_name_is_advisory_even_with_strict() {
    let pair = files(
        "name",
        "Priya Nair approved the plan.",
        "The plan was approved.",
    );
    assert_eq!(
        preserve(&["--strict"], &pair),
        (
            0,
            "No tracked items dropped; meaning not checked.\ncheck name: Priya Nair\nlength: 5 to 4 words (80% of the original)\n".into()
        )
    );
}

#[test]
fn a_missing_file_argument_is_a_usage_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .args(["preserve", "--strict", "only-one.md"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn strict_may_come_before_between_or_after_the_files() {
    let (o, e) = files("positions", "Refund the fee.", "Refund the $1,200 fee.");
    let bin = env!("CARGO_BIN_EXE_laconic-check");
    for args in [
        vec![
            "preserve",
            "--strict",
            o.to_str().unwrap(),
            e.to_str().unwrap(),
        ],
        vec![
            "preserve",
            o.to_str().unwrap(),
            "--strict",
            e.to_str().unwrap(),
        ],
        vec![
            "preserve",
            o.to_str().unwrap(),
            e.to_str().unwrap(),
            "--strict",
        ],
    ] {
        assert_eq!(
            Command::new(bin).args(&args).status().unwrap().code(),
            Some(1),
            "{args:?}"
        );
    }
    let extra = Command::new(bin)
        .args([
            "preserve",
            o.to_str().unwrap(),
            e.to_str().unwrap(),
            "third.md",
        ])
        .status()
        .unwrap();
    assert_eq!(extra.code(), Some(2));
}

#[test]
fn a_file_named_strict_is_read_after_the_option_terminator() {
    let dir = std::env::temp_dir().join(format!("laconic-check-{}-terminator", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("--strict"), "Refund the fee.").unwrap();
    std::fs::write(dir.join("edited.md"), "Refund the $1,200 fee.").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .current_dir(&dir)
        .args(["preserve", "--", "--strict", "edited.md"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("added number: $1,200")
    );
}

#[test]
fn mixed_drops_and_additions_report_both_and_fail() {
    let pair = files(
        "mixed",
        "It may run on 3 hosts.",
        "It must run on 4 hosts. It may.",
    );
    let (code, out) = preserve(&[], &pair);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "dropped number: 3\nadded meaning word: 'must' (0 before, 1 after)\nadded number: 4\nlength: 6 to 8 words (133% of the original)\n"
    );
}

#[test]
fn a_repeated_value_is_not_new_but_a_repeated_meaning_word_is() {
    let pair = files(
        "repeat",
        "Pay 5 now; it must clear.",
        "Pay 5 now and 5 later; it must clear and must post.",
    );
    let (code, out) = preserve(&["--strict"], &pair);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "No tracked items dropped; meaning not checked.\nadded meaning word: 'must' (1 before, 2 after)\nlength: 6 to 12 words (200% of the original)\n"
    );
}

#[test]
fn standard_input_serves_one_side_only() {
    use std::io::Write;
    let (o, _) = files("stdin", "Refund the fee.", "");
    let bin = env!("CARGO_BIN_EXE_laconic-check");
    let mut child = Command::new(bin)
        .args(["preserve", o.to_str().unwrap(), "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Refund the $1,200 fee.")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("added number: $1,200")
    );
    // Closed standard input, so a regression fails here instead of waiting.
    let both = Command::new(bin)
        .args(["preserve", "-", "-"])
        .stdin(std::process::Stdio::null())
        .status()
        .unwrap();
    assert_eq!(both.code(), Some(2));
}
