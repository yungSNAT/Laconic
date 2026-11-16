// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Top-level options: `--version` and `--help` succeed on standard output; anything else
//! unknown is a usage error on standard error. Also `lsp`'s and `hook`'s exit statuses.

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

/// `lsp` exits 0 after `shutdown` and `exit`, 1 for `exit` alone, and 2 on a broken header.
#[test]
fn hook_is_silent_once_a_turn_continued_and_fails_on_bad_input() {
    use std::io::Write;
    use std::process::Stdio;
    for (input, code, out, err) in [
        (r#"{"stop_hook_active":true}"#, 0, "", ""),
        (
            "not json",
            1,
            "",
            "laconic-check hook: standard input is not a JSON object\n",
        ),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
            .arg("hook")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let done = child.wait_with_output().unwrap();
        assert_eq!(done.status.code(), Some(code), "{input}");
        assert_eq!(String::from_utf8(done.stdout).unwrap(), out, "{input}");
        assert_eq!(String::from_utf8(done.stderr).unwrap(), err, "{input}");
    }
}

#[test]
fn lsp_exit_statuses_follow_the_protocol() {
    use std::io::Write;
    use std::process::Stdio;
    let frame = |body: &str| format!("Content-Length: {}\r\n\r\n{body}", body.len());
    let init =
        frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#);
    let shutdown = frame(r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#);
    let exit = frame(r#"{"jsonrpc":"2.0","method":"exit"}"#);
    for (input, wanted) in [
        (format!("{init}{shutdown}{exit}"), 0),
        (format!("{init}{exit}"), 1),
        ("Content-Type: x\r\n\r\n".to_owned(), 2),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
            .arg("lsp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(wanted), "{input:?}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(
            stdout.matches("Content-Length:").count(),
            [2, 1, 0][wanted as usize]
        );
    }
}
