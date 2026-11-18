// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Child processes as Python's `subprocess.run` ran them: an argument list with no shell,
//! captured output, an optional working directory and timeout, and a choice of what the child
//! reads on standard input.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// What the child reads on standard input.
pub enum Input<'a> {
    /// The parent's own standard input, as `subprocess.run` gives when given no `input`.
    Inherit,
    /// Nothing: `stdin=subprocess.DEVNULL`.
    Null,
    /// These bytes, then end of input: `input=...`.
    Bytes(&'a [u8]),
}

/// A finished child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// The exit status, or minus the signal that ended the child, as Python reports it.
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Why a child gave no output.
#[derive(Debug)]
pub enum Failure {
    /// It could not start, such as a program not found.
    Spawn(String),
    /// It ran past the timeout and was killed.
    Timeout,
}

impl Failure {
    pub fn message(&self, program: &str) -> String {
        match self {
            Failure::Spawn(error) => format!("cannot run {program}: {error}"),
            Failure::Timeout => format!("{program} timed out"),
        }
    }
}

/// A child that could not be started or waited for.
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Failure::Spawn(error.to_string())
    }
}

fn code(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return -signal;
        }
    }
    status.code().unwrap_or(-1)
}

fn drain(mut source: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = source.read_to_end(&mut bytes);
        bytes
    })
}

/// Waits for `child`, killing it once `timeout` passes.
fn wait(child: &mut Child, timeout: Option<Duration>) -> Result<ExitStatus, Failure> {
    let Some(timeout) = timeout else {
        return Ok(child.wait()?);
    };
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Failure::Timeout);
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Runs `command` (program and arguments) to completion.
pub fn run(
    command: &[String],
    input: &Input,
    cwd: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<Output, Failure> {
    let (program, arguments) = command
        .split_first()
        .ok_or_else(|| Failure::Spawn("an empty command".into()))?;
    let mut builder = Command::new(program);
    builder
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    builder.stdin(match input {
        Input::Inherit => Stdio::inherit(),
        Input::Null => Stdio::null(),
        Input::Bytes(_) => Stdio::piped(),
    });
    if let Some(cwd) = cwd {
        builder.current_dir(cwd);
    }
    let mut child = builder.spawn()?;
    let stdout = drain(child.stdout.take().expect("piped stdout"));
    let stderr = drain(child.stderr.take().expect("piped stderr"));
    if let Input::Bytes(bytes) = input {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let bytes = bytes.to_vec();
        // A child that exits without reading its input closes the pipe; that is not an error.
        thread::spawn(move || {
            let _ = stdin.write_all(&bytes);
        });
    }
    let status = wait(&mut child, timeout)?;
    Ok(Output {
        code: code(status),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

/// Output decoded as Python's `text=True` does with UTF-8: strict, with universal newlines.
pub fn text(bytes: &[u8]) -> Result<String, String> {
    std::str::from_utf8(bytes)
        .map(crate::py::universal_newlines)
        .map_err(|e| format!("'utf-8' codec can't decode the output: {e}"))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn sh(script: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), script.into()]
    }

    #[test]
    fn captures_output_status_and_input() {
        let done = run(
            &sh("cat; echo err >&2; exit 3"),
            &Input::Bytes(b"in"),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            done,
            Output {
                code: 3,
                stdout: b"in".to_vec(),
                stderr: b"err\n".to_vec()
            }
        );
        let done = run(&sh("cat"), &Input::Null, None, None).unwrap();
        assert!(done.stdout.is_empty());
    }

    #[test]
    fn runs_in_the_directory_given() {
        let dir = tempfile::tempdir().unwrap();
        let done = run(&sh("pwd"), &Input::Null, Some(dir.path()), None).unwrap();
        let printed = String::from_utf8(done.stdout).unwrap();
        assert_eq!(
            Path::new(printed.trim()).canonicalize().unwrap(),
            dir.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn a_hung_child_is_killed_at_the_timeout() {
        let started = Instant::now();
        let failure = run(
            &sh("sleep 5"),
            &Input::Null,
            None,
            Some(Duration::from_millis(200)),
        );
        assert!(matches!(failure, Err(Failure::Timeout)));
        assert!(started.elapsed() < Duration::from_secs(4));
        assert_eq!(Failure::Timeout.message("x"), "x timed out");
        let done = run(
            &sh("echo ok"),
            &Input::Null,
            None,
            Some(Duration::from_secs(30)),
        )
        .unwrap();
        assert_eq!(done.stdout, b"ok\n");
    }

    #[test]
    fn a_signal_is_reported_as_python_does() {
        let done = run(&sh("kill -9 $$"), &Input::Null, None, None).unwrap();
        assert_eq!(done.code, -9);
    }

    #[test]
    fn a_missing_program_or_empty_command_cannot_start() {
        let missing = run(&["no-such-program-xyz".into()], &Input::Null, None, None).unwrap_err();
        assert!(
            missing
                .message("no-such-program-xyz")
                .starts_with("cannot run no-such-program-xyz: ")
        );
        assert!(matches!(
            run(&[], &Input::Null, None, None),
            Err(Failure::Spawn(_))
        ));
    }

    #[test]
    fn a_child_that_ignores_its_input_does_not_fail_the_run() {
        let big = vec![b'x'; 1 << 20];
        let done = run(&sh("exit 0"), &Input::Bytes(&big), None, None).unwrap();
        assert_eq!(done.code, 0);
    }

    #[test]
    fn text_decodes_strictly_with_universal_newlines() {
        assert_eq!(text(b"a\r\nb\rc").unwrap(), "a\nb\nc");
        assert!(text(&[0xff]).unwrap_err().contains("can't decode"));
    }
}
