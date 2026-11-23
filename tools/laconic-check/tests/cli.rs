// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

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
    assert!(
        out.contains("laconic-check completions <bash|zsh|fish|powershell>"),
        "{out}"
    );
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

/// `hook` exits 0 with no output once a turn continued or outside a repository, and 1 on input
/// that is not a JSON object.
#[test]
fn hook_is_silent_once_a_turn_continued_or_outside_a_repository_and_fails_on_bad_input() {
    use std::io::Write;
    use std::process::Stdio;
    // Git's ceiling stops it looking above the temporary directory, which may be in a repository.
    let ceiling = std::env::temp_dir();
    let outside = ceiling.join(format!("laconic-cli-outside-{}", std::process::id()));
    std::fs::create_dir_all(&outside).unwrap();
    // Debug quoting escapes the backslashes and quotation marks JSON escapes too.
    let event = format!(r#"{{"cwd":{:?}}}"#, outside.to_str().unwrap());
    for (input, code, out, err) in [
        (r#"{"stop_hook_active":true}"#, 0, "", ""),
        (event.as_str(), 0, "", ""),
        (
            "not json",
            1,
            "",
            "laconic-check hook: standard input is not a JSON object\n",
        ),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
            .arg("hook")
            .env("GIT_CEILING_DIRECTORIES", &ceiling)
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
    let _ = std::fs::remove_dir_all(&outside);
}

/// `lsp` exits 0 after `shutdown` and `exit`, 1 for `exit` alone, and 2 on a broken header.
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

/// An argument that is not valid Unicode: 'caf' and '.md' around a lone Latin-1 byte on Unix,
/// or around an unpaired surrogate on Windows.
#[cfg(unix)]
fn not_unicode() -> std::ffi::OsString {
    std::os::unix::ffi::OsStringExt::from_vec(b"caf\xe9.md".to_vec())
}

#[cfg(windows)]
fn not_unicode() -> std::ffi::OsString {
    std::os::windows::ffi::OsStringExt::from_wide(&[0x63, 0x61, 0x66, 0xD800, 0x2E, 0x6D, 0x64])
}

/// An argument that is not valid Unicode is reported as a usage error, not a panic.
#[test]
fn an_argument_that_is_not_unicode_is_a_usage_error() {
    let out = Command::new(env!("CARGO_BIN_EXE_laconic-check"))
        .arg("lint")
        .arg(not_unicode())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "laconic-check: an argument is not valid Unicode: caf\u{fffd}.md\n"
    );
}

#[test]
fn completions_prints_a_script_for_each_supported_shell() {
    for (shell, marker) in [
        ("bash", "complete -F _laconic_check laconic-check"),
        ("zsh", "#compdef laconic-check"),
        ("fish", "complete -c laconic-check"),
        (
            "powershell",
            "Register-ArgumentCompleter -Native -CommandName 'laconic-check'",
        ),
    ] {
        let (code, out, err) = run(&["completions", shell]);
        assert_eq!((code, err.as_str()), (0, ""), "{shell}");
        assert!(out.contains(marker), "{shell}: {out}");
    }
}

#[test]
fn completions_rejects_unknown_missing_and_extra_arguments() {
    let (code, out, err) = run(&["completions", "tcsh"]);
    assert_eq!((code, out.as_str()), (2, ""));
    assert_eq!(
        err,
        "laconic-check: unknown shell 'tcsh' (expected bash, zsh, fish or powershell)\n"
    );

    for args in [&["completions"][..], &["completions", "bash", "extra"]] {
        let (code, out, err) = run(args);
        assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        assert!(err.starts_with("usage:\n"), "{args:?}: {err}");
    }
}
