// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::cases::root;
use crate::testing::{Run, Runs, done};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

fn today() -> String {
    "2026-11-16".into()
}

fn no_pause(_: Duration) {}

/// The world of a run whose runner calls go to `runs`.
macro_rules! world {
    ($runs:expr, $sleep:expr) => {
        World {
            exec: &|c, i, d, t| $runs.exec(c, i, d, t),
            sleep: $sleep,
            today: &today,
        }
    };
}

fn world<'a>(exec: Exec<'a>, sleep: Sleep<'a>) -> World<'a> {
    World {
        exec,
        sleep,
        today: &today,
    }
}

/// The run options of the Python tests' `_run_args`: one case and a claude-json stub runner.
fn run_args(dir: &Path, command: &[&str]) -> RunArgs {
    let cases = dir.join("cases.jsonl");
    let case = json!({"id": "probe", "category": "direct-answer",
        "prompt": "What is 17 multiplied by 6?", "risk": "low", "criteria": ["Answers 102."]});
    fs::write(&cases, format!("{case}\n")).unwrap();
    let runners = dir.join("runners.json");
    let config = json!({"stub": {"command": command, "response_format": "claude-json"}});
    fs::write(&runners, config.to_string()).unwrap();
    RunArgs {
        cases,
        runner_config: runners,
        runner: "stub".into(),
        condition: "baseline".into(),
        condition_skill: None,
        case: Vec::new(),
        trials: 1,
        retries: 0,
        no_injection: false,
        call_timeout: 300.0,
        jobs: 1,
        budget_usd: 1.0,
        allow_unmetered: false,
        output: dir.join("responses.jsonl"),
    }
}

/// Run options over the shipped catalogue, limited to `ids`.
fn catalogue_args(dir: &Path, ids: &[&str], format: &str, command: &[&str]) -> RunArgs {
    let mut args = run_args(dir, command);
    let config = json!({"stub": {"command": command, "response_format": format}});
    fs::write(&args.runner_config, config.to_string()).unwrap();
    args.cases = root().join("evals/cases.jsonl");
    args.case = ids.iter().map(|id| (*id).to_owned()).collect();
    args.output = dir.join("out.jsonl");
    args
}

fn run(args: &RunArgs, world: &World) -> (Result<u8, String>, String) {
    let mut out = Vec::new();
    let status = run_evaluations(args, world, &mut out);
    (status, String::from_utf8(out).unwrap())
}

fn rows(path: &Path) -> Vec<Value> {
    read_jsonl(path).unwrap()
}

fn reply(cost: f64) -> Result<Output, Failure> {
    done(
        0,
        &json!({"result": "102", "usage": {}, "total_cost_usd": cost}).to_string(),
        "",
    )
}

fn score_row(case_id: &str, condition: &str, value: i64, trial: i64) -> Value {
    json!({"case_id": case_id, "trial": trial, "condition": condition, "correctness": value,
        "autonomy": value, "actionability": value, "safety": value, "concision": value,
        "blocker": false, "notes": "fixture"})
}

fn usage_row(case_id: &str, condition: &str, response: &str, cost: Value, usage: Value) -> Value {
    json!({"case_id": case_id, "trial": 1, "condition": condition, "runner": "claude",
        "response": response, "usage": usage, "cost_usd": cost})
}

fn near(expected: f64, actual: &Value) {
    let actual = actual.as_f64().unwrap();
    assert!((expected - actual).abs() < 1e-7, "{expected} != {actual}");
}

fn fails<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.unwrap_err();
    assert!(error.contains(expected), "{error:?} lacks {expected:?}");
}

#[test]
fn type_names_follow_python() {
    let names: Vec<&str> = [
        json!(null),
        json!(true),
        json!(1.5),
        json!(1),
        json!("s"),
        json!([]),
        json!({}),
    ]
    .iter()
    .map(type_name)
    .collect();
    assert_eq!(
        names,
        ["NoneType", "bool", "float", "int", "str", "list", "dict"]
    );
}

#[test]
fn unterminated_frontmatter_preserves_the_skill_text() {
    let text = "---\nname: draft\nInstructions";
    assert_eq!(strip_frontmatter(text), text);
    assert_eq!(strip_frontmatter(""), "");
    assert_eq!(strip_frontmatter("---\nname: x\n---\n\n\nBody"), "Body");
}

#[test]
fn codex_response_uses_the_final_message_and_turn_usage() {
    let events = [
        json!({"type": "item.completed", "item": {"type": "agent_message", "text": "draft"}}),
        json!({"type": "item.completed", "item": {"type": "agent_message", "text": " 102 "}}),
        json!({"type": "item.completed", "item": {"type": "command_execution", "text": "log"}}),
        json!({"type": "item.completed", "item": {"type": "agent_message"}}),
        json!({"type": "turn.completed", "usage": {"input_tokens": 30, "output_tokens": 1}}),
        json!({"type": "turn.completed"}),
    ];
    let payload: Vec<String> = events.iter().map(Value::to_string).collect();
    assert_eq!(
        parse_response(&payload.join("\n\n"), &json!("codex-jsonl")),
        Ok((
            "102".to_owned(),
            json!({"input_tokens": 30, "output_tokens": 1}),
            Value::Null
        ))
    );
    assert_eq!(
        parse_response("", &json!("other")),
        Err(Fault::Value("Unsupported response format: other".into()))
    );
}

#[test]
fn unreadable_replies_are_value_faults_and_odd_shapes_are_other_faults() {
    let claude = json!("claude-json");
    let codex = json!("codex-jsonl");
    assert!(matches!(parse_response(" ", &claude), Err(Fault::Value(_))));
    assert_eq!(
        parse_response("[]", &claude),
        Err(Fault::Other("'list' object has no attribute 'get'".into()))
    );
    assert!(matches!(
        parse_response("not json", &codex),
        Err(Fault::Value(_))
    ));
    assert_eq!(
        parse_response("[1]", &codex),
        Err(Fault::Other("'list' object has no attribute 'get'".into()))
    );
    assert_eq!(
        parse_response(r#"{"type": "item.completed", "item": 3}"#, &codex),
        Err(Fault::Other("'int' object has no attribute 'get'".into()))
    );
    assert_eq!(
        parse_response(r#"{"result": 5, "usage": {}}"#, &claude),
        Ok(("5".into(), json!({}), Value::Null))
    );
    assert_eq!(
        parse_response(" ok \n", &json!("text")),
        Ok(("ok".into(), json!({}), Value::Null))
    );
    assert_eq!(Fault::Other("x".into()).message(), "x");
}

#[test]
fn runner_model_ignores_model_options_after_the_end_of_options() {
    let model = |command: &[&str]| {
        runner_model(&command.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    };
    assert_eq!(
        model(&["codex", "-m", "fixture", "--", "-m", "untrusted"]).as_deref(),
        Some("fixture")
    );
    assert_eq!(model(&[]), None);
    assert_eq!(model(&["/"]), None);
    assert_eq!(model(&["claude", "--model", "-x"]), None);
    assert_eq!(model(&["claude", "--model"]), None);
}

#[test]
fn run_rejects_bad_cases_unknown_case_ids_and_zero_workers_before_any_call() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.0));
    let world = world!(runs, &no_pause);
    let mut args = run_args(dir.path(), &["stub-runner"]);
    args.case = vec!["unknown".into(), "probe".into(), "unknown".into()];
    fails(
        run(&args, &world).0,
        "--case matched no evaluation case: unknown",
    );
    args.case.clear();
    args.jobs = 0;
    fails(run(&args, &world).0, "--jobs must be at least 1");
    fs::write(&args.cases, "{}\n").unwrap();
    fails(run(&args, &world).0, "missing fields");
    assert!(runs.calls().is_empty());
}

#[test]
fn failed_runner_uses_structured_error_or_falls_back_to_stderr() {
    for (stdout, expected) in [
        (r#"{"result": "over capacity"}"#, "over capacity"),
        (r#"{"result": ""}"#, "transport failed"),
        ("invalid JSON", "transport failed"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let runs = Runs::new(|_, _| done(1, stdout, "transport failed"));
        let world = world!(runs, &no_pause);
        let args = run_args(dir.path(), &["stub-runner", "--flag=it's"]);
        let error = run(&args, &world).0.unwrap_err();
        assert!(error.contains(expected), "{error}");
        assert!(
            error
                .starts_with("Runner failed after 1 attempts (stub-runner '--flag=it'\"'\"'s'):\n"),
            "{error}"
        );
        assert!(rows(&args.output).is_empty());
    }
}

#[test]
fn a_failed_reply_python_could_not_read_ends_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| done(1, "[1]", ""));
    let world = world!(runs, &no_pause);
    let args = run_args(dir.path(), &["stub-runner"]);
    assert_eq!(
        run(&args, &world).0,
        Err("'list' object has no attribute 'get'".into())
    );
}

#[test]
fn a_runner_that_cannot_start_or_retry_ends_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let missing = |_: &[String],
                   _: &Input,
                   _: Option<&Path>,
                   _: Option<Duration>|
     -> Result<Output, Failure> { Err(Failure::Spawn("not found".into())) };
    let mut args = run_args(dir.path(), &["stub-runner"]);
    assert_eq!(
        run(&args, &world(&missing, &no_pause)).0,
        Err("cannot run stub-runner: not found".into())
    );
    args.retries = -1;
    assert_eq!(
        run(&args, &world(&missing, &no_pause)).0,
        Err("--retries must not be negative".into())
    );
}

#[test]
fn successful_runner_without_cost_is_rejected_in_metered_mode() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| done(0, r#"{"result": "102"}"#, ""));
    let world = world!(runs, &no_pause);
    let args = run_args(dir.path(), &["stub-runner"]);
    fails(run(&args, &world).0, "Runner did not report dollar cost");
    assert!(rows(&args.output).is_empty());
}

#[test]
fn an_unreadable_successful_reply_ends_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| done(0, "not json", ""));
    let world = world!(runs, &no_pause);
    let args = run_args(dir.path(), &["stub-runner"]);
    fails(run(&args, &world).0, "expected ident");
}

#[test]
fn a_cost_python_cannot_add_stops_the_run_after_the_call() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| done(0, r#"{"result": "1", "total_cost_usd": "lots"}"#, ""));
    let world = world!(runs, &no_pause);
    let args = run_args(dir.path(), &["stub-runner"]);
    fails(run(&args, &world).0, "could not convert string to float");
    assert!(rows(&args.output).is_empty());
}

// Declared after `world!`, which a macro must be to reach them.
mod runs;
mod setup;
mod summaries;
mod usage;
