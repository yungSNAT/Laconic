// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::cases::root;
use crate::proc::{Failure, Output};
use crate::testing::{Run, Runs, done};
use std::sync::Mutex;
use std::time::Duration;

mod calibration;
mod calling;
mod verdicts;

const CASE_PROMPT: &str = "What is 17 multiplied by 6?";

fn no_pause(_: Duration) {}

fn labels(pairs: &[(&str, &str)]) -> Labels {
    pairs
        .iter()
        .map(|(c, l)| ((*c).to_owned(), (*l).to_owned()))
        .collect()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, Value)> {
    items
        .iter()
        .map(|(c, t)| ((*c).to_owned(), Value::from(*t)))
        .collect()
}

fn verdict(value: i64, blocker: bool) -> Value {
    json!({"correctness": value, "autonomy": value, "actionability": value, "safety": value,
        "concision": value, "blocker": blocker, "notes": "fixture"})
}

/// The verdict of the Python end-to-end tests.
fn fixture_verdict() -> Value {
    json!({"correctness": 4, "autonomy": 4, "actionability": 4, "safety": 5, "concision": 3,
        "blocker": false, "notes": "fixture"})
}

fn case() -> Value {
    json!({"id": "direct-answer", "prompt": CASE_PROMPT, "criteria": ["States 102."],
        "risk": "low"})
}

fn fails<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.unwrap_err();
    assert!(error.contains(expected), "{error:?} lacks {expected:?}");
}

fn runner(command: &[&str], format: &str) -> Runner {
    Runner {
        command: command.iter().map(|s| (*s).to_owned()).collect(),
        format: json!(format),
        budget_flag: None,
    }
}

/// A judge command over `responses` with a stub runner config in `dir`.
struct Fixture {
    dir: tempfile::TempDir,
    args: JudgeArgs,
}

impl Fixture {
    fn new(command: &[&str], format: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("runners.json");
        let stub = json!({"stub": {"command": command, "response_format": format}});
        fs::write(&config, stub.to_string()).unwrap();
        let args = JudgeArgs {
            calibrate: None,
            responses: Some(dir.path().join("responses.jsonl")),
            cases: root().join("evals/cases.jsonl"),
            rubric: root().join("evals/rubric.md"),
            runner_config: config,
            runner: "stub".into(),
            conditions: vec!["baseline".into(), "candidate".into()],
            retries: 2,
            call_timeout: 300.0,
            output: Some(dir.path().join("scores.jsonl")),
        };
        Fixture { dir, args }
    }

    fn write_responses(&self, entries: &[(&str, &str, i64)]) {
        let lines: Vec<String> = entries
            .iter()
            .map(|(case, condition, trial)| {
                let row = json!({"case_id": case, "condition": condition, "trial": trial,
                    "runner": "stub", "response": format!("{case} {condition} text")});
                format!("{row}\n")
            })
            .collect();
        fs::write(self.args.responses.as_ref().unwrap(), lines.concat()).unwrap();
    }

    fn output(&self) -> &Path {
        self.args.output.as_deref().unwrap()
    }

    fn rows(&self) -> Vec<Value> {
        read_jsonl(self.output()).unwrap()
    }

    fn run(&self, exec: Exec, sleep: Sleep) -> (Result<u8, String>, String) {
        let mut out = Vec::new();
        let status = command(&self.args, exec, sleep, &mut out);
        (status, String::from_utf8(out).unwrap())
    }
}

/// A claude-json reply carrying a verdict for `labels`, at a cost of 0.25.
fn cases_key(id: &str) -> String {
    py::key(&json!(id))
}

fn claude_verdict(labels: &[&str]) -> Result<Output, Failure> {
    let result: serde_json::Map<String, Value> = labels
        .iter()
        .map(|l| ((*l).to_owned(), fixture_verdict()))
        .collect();
    let reply = json!({"result": Value::Object(result).to_string(), "total_cost_usd": 0.25});
    done(0, &reply.to_string(), "")
}
