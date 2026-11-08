// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Repository contract tests: hooks, OpenCode plugin, manifests, skills, install docs and
//! CI supply-chain guards. Ported from the Python `tests/test_*.py` files with the same
//! assertions; each module names the file it replaces.
//!
//! Each regular expression is the Python test's pattern, compiled by the regex crate (a
//! dev-dependency, so the binary does not carry it). Both engines read `\w`, `\d`, `\s` and
//! `` as Unicode classes; `$` without `(?m)` matches only at the end here, where Python also
//! allows a final newline, which no tested value contains.

mod ci_installs;
mod hooks;
mod install_docs;
mod manifests;
mod omp;
mod opencode;
mod skills;

use crate::pi_check::TempDir;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long a child may run when the Python test set no timeout. A hang fails the test
/// instead of stalling CI until the job times out.
const DEFAULT_DEADLINE: Duration = Duration::from_secs(120);

/// The repository root.
fn root() -> PathBuf {
    crate::root()
}

/// A repository or fixture file as Python's text mode reads it: CRLF and lone CR become LF.
fn read(path: &Path) -> String {
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    crate::lf(&text)
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&read(path))
        .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

/// Writes `text` as Python's `write_text` does: each LF becomes the platform line ending, so
/// Windows runs exercise CRLF fixtures.
fn write_text(path: &Path, text: &str) {
    let text = if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_owned()
    };
    fs::write(path, text).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// Creates an empty file, as Python's `Path.touch` does.
fn touch(path: &Path) {
    fs::write(path, "").unwrap_or_else(|e| panic!("cannot create {}: {e}", path.display()));
}

/// Copies the tree at `from` to the new directory `to`, as `shutil.copytree` does.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap_or_else(|e| panic!("cannot create {}: {e}", to.display()));
    let entries =
        fs::read_dir(from).unwrap_or_else(|e| panic!("cannot list {}: {e}", from.display()));
    for entry in entries {
        let source = entry.unwrap().path();
        let target = to.join(source.file_name().unwrap());
        if source.is_dir() {
            copy_tree(&source, &target);
        } else {
            fs::copy(&source, &target)
                .unwrap_or_else(|e| panic!("cannot copy {}: {e}", source.display()));
        }
    }
}

/// A new directory under the system temporary directory, removed when dropped.
fn temp_dir() -> TempDir {
    TempDir::new("laconic-contract-").unwrap()
}

/// `path` with symbolic links and 8.3 short names resolved.
fn resolved(path: &Path) -> PathBuf {
    let path =
        fs::canonicalize(path).unwrap_or_else(|e| panic!("cannot resolve {}: {e}", path.display()));
    // shortcut: Windows canonical paths carry the verbatim prefix \\?\, which Python's resolve()
    // drops and which would change every path the plugins report. Only drive paths are
    // stripped; a UNC temporary directory would keep its \\?\UNC\ prefix and fail the path
    // assertions loudly.
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(drive) if drive.as_bytes().get(1) == Some(&b':') => PathBuf::from(drive),
        _ => path,
    }
}

/// The first program on PATH named `name`; fails where Python skipped, naming the program.
fn require(name: &str) -> PathBuf {
    crate::pi_check::which(name).unwrap_or_else(|| panic!("{name} is required on PATH"))
}

/// A finished child process.
struct Output {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Output {
    /// Standard output as Python's text mode decodes it: CRLF and lone CR become LF.
    fn out(&self) -> String {
        crate::lf(&String::from_utf8_lossy(&self.stdout))
    }

    fn err(&self) -> String {
        crate::lf(&String::from_utf8_lossy(&self.stderr))
    }

    /// Asserts a zero exit status, showing standard error otherwise.
    fn assert_success(&self) {
        assert_eq!(self.code, Some(0), "stderr: {}", self.err());
    }
}

/// Runs `command` with `input` on standard input (none when `None`), failing the test when it
/// outlives `timeout` or, without one, `DEFAULT_DEADLINE`.
fn run(command: &mut Command, input: Option<&str>, timeout: Option<Duration>) -> Output {
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot start {command:?}: {e}"));
    if let Some(input) = input {
        // Dropping the handle closes the pipe, so the child sees the end of its input.
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(input.as_bytes()).unwrap();
    }
    let drain = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).map(|_| bytes)
        })
    };
    let stdout = drain(Box::new(child.stdout.take().unwrap()));
    let stderr = drain(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + timeout.unwrap_or(DEFAULT_DEADLINE);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{command:?} did not finish within its deadline");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        code: status.code(),
        stdout: stdout.join().unwrap().unwrap(),
        stderr: stderr.join().unwrap().unwrap(),
    }
}

/// Python's truth value of a JSON value: false for null, false, 0, "", [] and {}.
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// A regular expression from the Python tests.
fn re(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).unwrap_or_else(|e| panic!("bad pattern {pattern}: {e}"))
}

#[test]
fn helpers_read_and_write_text_as_python_does() {
    let dir = temp_dir();
    let file = dir.0.join("fixture.txt");
    write_text(&file, "a\nb\n");
    let expected: &[u8] = if cfg!(windows) {
        b"a\r\nb\r\n"
    } else {
        b"a\nb\n"
    };
    assert_eq!(fs::read(&file).unwrap(), expected);
    fs::write(&file, "a\r\nb\rc\n").unwrap();
    assert_eq!(read(&file), "a\nb\nc\n");
    for (value, expected) in [
        ("null", false),
        ("0", false),
        ("0.0", false),
        ("\"\"", false),
        ("[]", false),
        ("{}", false),
        ("false", false),
        ("1", true),
        ("\"x\"", true),
        ("[0]", true),
        ("true", true),
    ] {
        assert_eq!(
            truthy(&serde_json::from_str(value).unwrap()),
            expected,
            "{value}"
        );
    }
}

#[test]
fn run_kills_a_child_past_its_deadline() {
    let node = require("node");
    let result = std::panic::catch_unwind(|| {
        run(
            Command::new(&node).args(["-e", "setTimeout(() => {}, 60000)"]),
            None,
            Some(Duration::from_millis(500)),
        )
    });
    assert!(result.is_err());
}
