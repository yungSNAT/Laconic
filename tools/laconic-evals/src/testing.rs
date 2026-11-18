// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Test helpers: stand-in programs and recorded calls, so no test makes a paid or network call.

use crate::proc::{Failure, Input, Output};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// A finished stand-in call.
pub fn done(code: i32, stdout: &str, stderr: &str) -> Result<Output, Failure> {
    Ok(Output {
        code,
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
    })
}

/// One recorded call: the command and what it got on standard input.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub command: Vec<String>,
    pub input: Option<String>,
    pub inherited: bool,
}

/// Records each call and answers from `reply`.
pub struct Recorder {
    pub calls: Mutex<Vec<Call>>,
}

impl Recorder {
    pub fn new() -> Self {
        Recorder {
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, command: &[String], input: &Input) {
        self.calls.lock().unwrap().push(Call {
            command: command.to_vec(),
            input: match input {
                Input::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
                _ => None,
            },
            inherited: matches!(input, Input::Inherit),
        });
    }

    pub fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

/// An executable shell script named `name` in `dir`.
#[cfg(unix)]
pub fn script(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

/// Writes JSON Lines rows, one per line, with no final newline, as the Python tests did.
pub fn write_rows(path: &Path, rows: &[serde_json::Value]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let lines: Vec<String> = rows.iter().map(|r| r.to_string()).collect();
    fs::write(path, lines.join("\n")).unwrap();
}

/// One recorded call of a `run_evals::Exec` stand-in.
#[derive(Debug, Clone)]
pub struct Run {
    pub command: Vec<String>,
    /// What the runner read on standard input; None when it read nothing or inherited it.
    pub input: Option<String>,
    pub null: bool,
    pub cwd: Option<std::path::PathBuf>,
    pub timeout: Option<std::time::Duration>,
}

impl Run {
    fn new(
        command: &[String],
        input: &Input,
        cwd: Option<&Path>,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        Run {
            command: command.to_vec(),
            input: match input {
                Input::Bytes(b) => Some(String::from_utf8_lossy(b).into_owned()),
                _ => None,
            },
            null: matches!(input, Input::Null),
            cwd: cwd.map(Path::to_path_buf),
            timeout,
        }
    }
}

/// A stand-in for the runner calls of `run`, `judge` and `scenario run`: records each call
/// and answers with `reply(call index, call)`.
pub struct Runs<F> {
    calls: Mutex<Vec<Run>>,
    reply: F,
}

impl<F: Fn(usize, &Run) -> Result<Output, Failure> + Sync> Runs<F> {
    pub fn new(reply: F) -> Self {
        Runs {
            calls: Mutex::new(Vec::new()),
            reply,
        }
    }

    pub fn exec(
        &self,
        command: &[String],
        input: &Input,
        cwd: Option<&Path>,
        timeout: Option<std::time::Duration>,
    ) -> Result<Output, Failure> {
        let call = Run::new(command, input, cwd, timeout);
        let index = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(call.clone());
            calls.len() - 1
        };
        (self.reply)(index, &call)
    }

    pub fn calls(&self) -> Vec<Run> {
        self.calls.lock().unwrap().clone()
    }
}
