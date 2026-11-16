// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Agentic cost benchmark: does Laconic's agent-work guidance cut turns, tokens and cost?
//!
//! Builds 5 small synthetic Python repositories (evals/agent-bench-tasks.json), each with one
//! failing unittest, then asks Claude Code (print mode, isolated settings, tools limited to
//! Read, Edit, Glob, Grep and `python -m unittest`) to fix each one under two rule sets.
//! Records whether the tests pass afterwards, plus turns, tokens and cost. Everything runs in
//! temporary folders. The fixtures stay Python: they are what the agent fixes.

use crate::cases::parse;
use crate::proc::{self, Failure, Input, Output};
use crate::py::{self, Num};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

pub const USAGE: &str =
    "usage: laconic-evals agent-bench <rules-a.md> <rules-b.md> <trials> <out.jsonl>";
pub const PROMPT: &str = "One test in this repository fails. Find the cause, fix the code (not the test) and run the tests to confirm they pass.";

/// A task's files: each path within its repository and its text.
pub type Files = Vec<(String, String)>;

/// Each task's files, in the order Python wrote them.
pub static TASKS: LazyLock<Vec<(String, Files)>> = LazyLock::new(|| {
    let tasks: Map<String, Value> =
        serde_json::from_str(include_str!("../../../evals/agent-bench-tasks.json"))
            .expect("valid tasks");
    tasks
        .into_iter()
        .map(|(task, files)| {
            let files = files.as_object().expect("a task's files").iter();
            let files = files
                .map(|(rel, text)| (rel.clone(), text.as_str().expect("file text").to_owned()));
            (task, files.collect())
        })
        .collect()
});

/// Runs a command in a folder with the given input and timeout.
pub type Run<'a> = &'a dyn Fn(&[String], &Input, &Path, Duration) -> Result<Output, Failure>;

/// The production `Run`.
pub fn run_in(
    command: &[String],
    input: &Input,
    cwd: &Path,
    timeout: Duration,
) -> Result<Output, Failure> {
    proc::run(command, input, Some(cwd), Some(timeout))
}

pub fn build(task: &str, root: &Path) -> Result<(), String> {
    let (_, files) = TASKS
        .iter()
        .find(|(name, _)| name == task)
        .ok_or_else(|| format!("no task {task}"))?;
    let write = |path: &Path, text: &str| {
        fs::create_dir_all(path.parent().expect("a file in a folder"))
            .and_then(|()| fs::write(path, text))
            .map_err(|e| format!("{}: {e}", path.display()))
    };
    for (rel, text) in files {
        write(&root.join(rel), text)?;
    }
    write(&root.join("app/__init__.py"), "")?;
    write(&root.join("tests/__init__.py"), "")
}

/// The Python interpreter the fixtures' tests run under.
pub fn python(which: &dyn Fn(&str) -> Option<PathBuf>) -> Result<PathBuf, String> {
    which("python3")
        .or_else(|| which("python"))
        .ok_or_else(|| "agent-bench: no python3 or python on PATH".to_owned())
}

pub fn tests_pass(python: &Path, root: &Path, run: Run) -> Result<bool, String> {
    let command: Vec<String> = [python.display().to_string()]
        .into_iter()
        .chain(["-m", "unittest", "discover", "-s", "tests", "-t", "."].map(str::to_owned))
        .collect();
    let done = run(&command, &Input::Inherit, root, Duration::from_secs(120))
        .map_err(|e| e.message(&command[0]))?;
    Ok(done.code == 0)
}

fn number(value: &Value) -> Result<Num, String> {
    value
        .as_i64()
        .map(Num::Int)
        .or_else(|| value.as_f64().map(Num::Float))
        .ok_or_else(|| {
            format!(
                "unsupported operand type(s) for +: 'int' and {}",
                py::repr(value)
            )
        })
}

fn length(value: Option<&Value>) -> Result<usize, String> {
    match value.filter(|v| py::truthy(v)) {
        None => Ok(0),
        Some(Value::Array(items)) => Ok(items.len()),
        Some(Value::Object(map)) => Ok(map.len()),
        Some(Value::String(s)) => Ok(s.chars().count()),
        Some(other) => Err(format!("{} has no len()", py::repr(other))),
    }
}

/// One Claude Code session on one repository: turns, cost, tokens and denials, or the name of
/// the error Python reported when the session timed out or its output was not JSON.
pub fn run(claude: &Path, rules: &str, root: &Path, run: Run) -> Result<Value, String> {
    let command: Vec<String> = [claude.display().to_string()]
        .into_iter()
        .chain(
            [
                "--print",
                "--output-format",
                "json",
                "--no-session-persistence",
                "--strict-mcp-config",
                "--setting-sources",
                "",
                "--model",
                "claude-opus-5-5",
                "--permission-mode",
                "default",
                "--tools",
                "Read,Edit,Glob,Grep,Bash",
                "--allowedTools",
                "Read",
                "Edit",
                "Glob",
                "Grep",
                "Bash(python -m unittest:*)",
                "--append-system-prompt-file",
                rules,
                "--max-budget-usd",
                "1.5",
                PROMPT,
            ]
            .map(str::to_owned),
        )
        .collect();
    let error = |name: &str| Ok(py::object([("error", Value::from(name))]));
    let done = match run(&command, &Input::Null, root, Duration::from_secs(600)) {
        Err(Failure::Timeout) => return error("TimeoutExpired"),
        Err(failure) => return Err(failure.message(&command[0])),
        Ok(done) => done,
    };
    let Ok(stdout) = proc::text(&done.stdout) else {
        return error("UnicodeDecodeError");
    };
    let Ok(out) = parse(&stdout) else {
        return error("JSONDecodeError");
    };
    let Value::Object(out) = out else {
        return Err(format!("{} has no attribute 'get'", py::repr(&out)));
    };
    let empty = Value::Object(Map::new());
    let usage = out.get("usage").filter(|u| py::truthy(u)).unwrap_or(&empty);
    let Value::Object(usage) = usage else {
        return Err(format!("{} has no attribute 'get'", py::repr(usage)));
    };
    let mut tokens = Vec::new();
    for name in [
        "input_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        tokens.push(
            usage
                .get(name)
                .filter(|v| py::truthy(v))
                .map_or(Ok(Num::Int(0)), number)?,
        );
    }
    let ints: Option<Vec<i64>> = tokens
        .iter()
        .map(|t| if let Num::Int(i) = t { Some(*i) } else { None })
        .collect();
    let input_tokens = match ints {
        Some(ints) => Num::Int(ints.iter().sum()),
        None => Num::Float(py::sum(tokens.iter().map(|t| t.as_f64()))),
    };
    let field =
        |map: &Map<String, Value>, name: &str| map.get(name).cloned().unwrap_or(Value::Null);
    Ok(py::object([
        ("turns", field(&out, "num_turns")),
        ("cost", field(&out, "total_cost_usd")),
        ("input_tokens", input_tokens.to_value()),
        ("output_tokens", field(usage, "output_tokens")),
        (
            "denials",
            Value::from(length(out.get("permission_denials"))?),
        ),
    ]))
}

/// The finished (task, setup, trial) keys in `out`; a blank or malformed line is an error.
fn finished(out: &Path) -> Result<Vec<[String; 3]>, String> {
    if !out.exists() {
        return Ok(Vec::new());
    }
    let text = crate::cases::read_text(out)?;
    py::split_lines(&text)
        .into_iter()
        .map(|line| {
            let row = parse(line)?;
            Ok([
                py::key(py::field(&row, "task")?),
                py::key(py::field(&row, "setup")?),
                py::key(py::field(&row, "trial")?),
            ])
        })
        .collect()
}

/// The command: runs each task under each rule set for each trial not already in `out`,
/// appending and printing one JSON line per run.
pub fn command(
    args: &[String],
    claude: Option<&Path>,
    python: &Path,
    session: &dyn Fn(&Path, &str, &Path) -> Result<Value, String>,
    run: Run,
    stdout: &mut dyn Write,
) -> Result<(), String> {
    let [rules_a, rules_b, trials, out] = args else {
        return Err(USAGE.into());
    };
    let claude = claude.ok_or("agent-bench: the claude CLI is not on PATH")?;
    let trials: i64 = py::strip(trials).parse().map_err(|_| {
        format!(
            "invalid literal for int() with base 10: {}",
            py::str_repr(trials)
        )
    })?;
    let out = Path::new(out);
    let done = finished(out)?;
    for trial in 1..=trials {
        for (task, _) in TASKS.iter() {
            for (setup, path) in [("A", rules_a), ("B", rules_b)] {
                let key = [
                    py::key(&Value::from(task.as_str())),
                    py::key(&Value::from(setup)),
                    trial.to_string(),
                ];
                if done.contains(&key) {
                    continue;
                }
                let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
                build(task, dir.path())?;
                if tests_pass(python, dir.path(), run)? {
                    return Err(format!("{task} should start failing"));
                }
                let mut result = session(claude, path, dir.path())?;
                let passed = tests_pass(python, dir.path(), run)?;
                let map = result.as_object_mut().expect("a result object");
                map.insert("task".into(), Value::from(task.as_str()));
                map.insert("setup".into(), Value::from(setup));
                map.insert("trial".into(), Value::from(trial));
                map.insert("passed".into(), Value::from(passed));
                drop(dir);
                let line = py::dumps(&result, None, true);
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(out)
                    .and_then(|mut file| file.write_all(format!("{line}\n").as_bytes()))
                    .map_err(|e| format!("{}: {e}", out.display()))?;
                writeln!(stdout, "{line}")
                    .and_then(|()| stdout.flush())
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
