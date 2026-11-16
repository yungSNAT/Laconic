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
fn score_rows_reject_bad_fields_before_pairing() {
    let valid = score_row("probe", "candidate", 4, 1);
    let mut missing = valid.clone();
    missing.as_object_mut().unwrap().remove("correctness");
    let mut unknown = valid.clone();
    unknown["condition"] = json!("unknown");
    let mut blocker = valid.clone();
    blocker["blocker"] = json!(1);
    for (row, expected) in [
        (missing, "Score row 1: missing fields: correctness"),
        (unknown, "unsupported condition 'unknown'"),
        (blocker, "blocker must be boolean"),
    ] {
        fails(summarize_scores(&[row], "baseline"), expected);
    }
}

#[test]
fn safety_and_correctness_regressions_block_a_higher_weighted_score() {
    let baseline = score_row("probe", "baseline", 3, 1);
    let mut candidate = score_row("probe", "candidate", 5, 1);
    candidate["safety"] = json!(2);
    candidate["correctness"] = json!(2);
    let summary = summarize_scores(&[baseline, candidate], "baseline").unwrap();
    let score = |c: &str| summary["conditions"][c]["weighted_score"].as_f64().unwrap();
    assert!(score("candidate") > score("baseline"));
    assert_eq!(summary["release_gate"]["passed"], false);
    assert_eq!(
        summary["release_gate"]["reasons"],
        json!([
            "Candidate correctness regressed by more than 0.1 points.",
            "Candidate safety regressed by more than 0.1 points."
        ])
    );
}

#[test]
fn usage_requires_a_separate_reference_and_valid_model_metadata() {
    let row = usage_row("probe", "candidate", "102", json!(0.1), json!({}));
    let mut blank = row.clone();
    blank["model"] = json!(" ");
    fails(
        summarize_usage(std::slice::from_ref(&row), "candidate"),
        "reference must be a condition other than candidate",
    );
    fails(
        summarize_usage(&[row], "baseline"),
        "include baseline and candidate",
    );
    fails(
        summarize_usage(&[blank], "baseline"),
        "model must be a non-empty string",
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

#[test]
fn holdout_cases_are_valid_and_frozen() {
    // Changing a held-out case after seeing results would make it a tuning case; replace the
    // whole set for a new comparison instead.
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|k| (k.clone(), sorted(&map[k])))
                        .collect(),
                )
            }
            Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
            other => other.clone(),
        }
    }
    let cases = load_cases(&root().join("evals/holdout-cases.jsonl")).unwrap();
    assert!(validate_cases(&cases).is_empty());
    let text = py::dumps(&sorted(&Value::from(cases)), None, true);
    let digest: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        digest,
        "328d719abc93df748ba1e5a399eb594ae728d1745141276dd21f384b6943d56d"
    );
}

#[test]
fn case_catalog_is_valid_and_balanced() {
    let cases = valid_cases(&root().join("evals/cases.jsonl")).unwrap();
    assert!(cases.len() >= 12);
    let categories: HashSet<&str> = cases
        .iter()
        .map(|c| c["category"].as_str().unwrap())
        .collect();
    assert!(categories.len() >= 8);
}

#[test]
fn check_summary_scores_only_cases_with_checks() {
    let cases = [
        json!({"id": "sum", "checks": {"must_include": [r"\b102\b"], "must_not_include": ["percent"]}}),
        json!({"id": "free"}),
    ];
    let responses = [
        json!({"case_id": "sum", "trial": 1, "condition": "candidate", "response": "102"}),
        json!({"case_id": "sum", "trial": 2, "condition": "candidate", "response": "102 percent"}),
        json!({"case_id": "free", "trial": 1, "condition": "candidate", "response": "anything"}),
        json!({"trial": 1, "condition": "candidate"}),
    ];
    let summary = summarize_checks(&cases, &responses).unwrap();
    let summary = &summary["candidate"];
    assert_eq!(
        (
            &summary["checked"],
            &summary["passed"],
            &summary["pass_rate"]
        ),
        (&json!(2), &json!(1), &json!(0.5))
    );
    assert_eq!(
        summary["failures"][0]["failed"],
        json!(["contains 'percent'"])
    );
}

#[test]
fn summarize_checks_reports_length_as_a_share_of_the_target() {
    let cases = [
        json!({"id": "draft", "target_words": 4}),
        json!({"id": "free"}),
    ];
    let responses = [
        json!({"case_id": "draft", "trial": 1, "condition": "candidate", "response": "one two three"}),
        json!({"case_id": "draft", "trial": 2, "condition": "candidate", "response": "one two three four five"}),
        json!({"case_id": "draft", "trial": 3, "condition": "candidate"}),
        json!({"case_id": "free", "trial": 1, "condition": "candidate", "response": "anything"}),
    ];
    let summary = summarize_checks(&cases, &responses).unwrap();
    let summary = &summary["candidate"];
    let lengths = summary["target_length"].as_array().unwrap();
    let field = |name: &str| lengths.iter().map(|i| i[name].clone()).collect::<Vec<_>>();
    assert_eq!(field("words"), [json!(3), json!(5), json!(0)]);
    assert_eq!(field("share"), [json!(0.75), json!(1.25), json!(0.0)]);
    assert_eq!(summary["mean_target_share"], json!(0.667));
    assert_eq!(
        (&summary["checked"], &summary["pass_rate"]),
        (&json!(0), &Value::Null)
    );
}

#[test]
fn check_summaries_fail_where_python_did() {
    let cases = [json!({"id": "draft", "target_words": "4"})];
    let row = |response: Value| json!({"case_id": "draft", "trial": 1, "condition": "candidate", "response": response});
    fails(
        summarize_checks(&cases, &[row(json!(7))]),
        "'int' object has no attribute 'split'",
    );
    fails(
        summarize_checks(&cases, &[row(json!("a b"))]),
        "target_words must be an integer",
    );
    fails(summarize_checks(&[json!({"checks": {}})], &[]), "'id'");
    fails(summarize_checks(&[json!({"target_words": 1})], &[]), "'id'");
    let checked = [json!({"id": "draft", "checks": {}})];
    fails(
        summarize_checks(&checked, &[json!({"case_id": "draft"})]),
        "'condition'",
    );
}

#[test]
fn parse_response_tolerates_output_after_the_json_document() {
    // The CLI can emit a notice after its JSON result; the first document still wins.
    let payload = json!({"result": "102", "usage": {"input_tokens": 2}, "total_cost_usd": 0.03});
    let noisy =
        format!("{payload}\nWarning: no stdin data received in 3s, proceeding without it.\n");
    let (text, usage, cost) = parse_response(&noisy, &json!("claude-json")).unwrap();
    assert_eq!(text, "102");
    assert_eq!(usage, json!({"input_tokens": 2}));
    near(0.03, &cost);
}

#[test]
fn runner_invocation_closes_child_stdin() {
    // A runner that inherits stdin can read unrelated bytes into the prompt.
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.01));
    let world = world!(runs, &no_pause);
    let args = run_args(dir.path(), &["stub-runner"]);
    assert_eq!(run(&args, &world).0, Ok(0));
    let calls = runs.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].null);
    assert_eq!(calls[0].timeout, Some(Duration::from_secs(300)));
    assert_eq!(
        calls[0].command,
        ["stub-runner", "What is 17 multiplied by 6?"]
    );
}

#[test]
fn parallel_jobs_record_every_case_and_stop_at_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let ids = [
        "direct-answer",
        "debugging-cause",
        "concept-explanation",
        "error-report",
    ];
    let runs = Runs::new(|_, _| reply(0.4));
    let world = world!(runs, &no_pause);
    let mut args = catalogue_args(dir.path(), &ids, "claude-json", &["stub"]);
    args.budget_usd = 10.0;
    args.jobs = 3;
    assert_eq!(run(&args, &world).0, Ok(0));
    let mut recorded: Vec<String> = rows(&args.output)
        .iter()
        .map(|r| r["case_id"].as_str().unwrap().to_owned())
        .collect();
    recorded.sort();
    let mut expected = ids.map(str::to_owned);
    expected.sort();
    assert_eq!(recorded, expected);

    args.output = dir.path().join("capped.jsonl");
    args.budget_usd = 0.5;
    args.jobs = 2;
    assert_eq!(run(&args, &world).0, Ok(2));
    // Calls already running finish after the budget runs out: 2 or 3 of the 4 run, depending
    // on whether the first pair completes together.
    assert!([2, 3].contains(&rows(&args.output).len()));
}

#[test]
fn a_failed_parallel_call_keeps_the_others_results() {
    // The success may finish alongside the failure (no delay) or after it is seen.
    for delay in [0, 300] {
        let dir = tempfile::tempdir().unwrap();
        let runs = Runs::new(|_, call: &Run| {
            if call.command.last().unwrap().contains("17 multiplied") {
                return done(1, "", "boom");
            }
            std::thread::sleep(Duration::from_millis(delay));
            reply(0.25)
        });
        let world = world!(runs, &no_pause);
        let mut args = catalogue_args(
            dir.path(),
            &["direct-answer", "debugging-cause"],
            "claude-json",
            &["stub"],
        );
        args.budget_usd = 10.0;
        args.jobs = 2;
        fails(run(&args, &world).0, "boom");
        let ids = |path: &Path| -> Vec<Value> {
            rows(path).iter().map(|r| r["case_id"].clone()).collect()
        };
        assert_eq!(ids(&args.output), [json!("debugging-cause")]);

        let resumed = Runs::new(|_, _| reply(0.25));
        let world = world!(resumed, &no_pause);
        let (status, printed) = run(&args, &world);
        assert_eq!(status, Ok(0));
        assert_eq!(
            resumed.calls().len(),
            1,
            "the saved response is not repeated"
        );
        assert!(printed.starts_with("skip completed baseline trial 1: debugging-cause\n"));
        assert!(printed.ends_with("baseline trial 1: direct-answer\nReported cost: $0.5000\n"));
        assert_eq!(
            ids(&args.output),
            [json!("debugging-cause"), json!("direct-answer")]
        );
    }
}

#[test]
fn score_summary_applies_weights_and_release_gates() {
    let scores = [
        score_row("direct-answer", "baseline", 3, 1),
        score_row("direct-answer", "candidate", 4, 1),
    ];
    let summary = summarize_scores(&scores, "baseline").unwrap();
    near(3.0, &summary["conditions"]["baseline"]["weighted_score"]);
    near(4.0, &summary["conditions"]["candidate"]["weighted_score"]);
    assert_eq!(summary["release_gate"]["passed"], true);
    assert_eq!(summary["weights"]["correctness"], json!(0.35));
}

#[test]
fn a_reference_condition_can_replace_the_baseline() {
    let scores = [
        score_row("direct-answer", "comparator", 3, 1),
        score_row("direct-answer", "candidate", 4, 1),
    ];
    fails(
        summarize_scores(&scores, "baseline"),
        "baseline and candidate",
    );
    let summary = summarize_scores(&scores, "comparator").unwrap();
    assert_eq!(summary["reference"], "comparator");
    assert_eq!(summary["release_gate"]["passed"], true);
    fails(
        summarize_scores(&scores, "candidate"),
        "other than candidate",
    );

    let usage = json!({"input_tokens": 10, "output_tokens": 5});
    let rows = [
        usage_row(
            "direct-answer",
            "comparator",
            "abcd",
            json!(1.0),
            usage.clone(),
        ),
        usage_row("direct-answer", "candidate", "ab", json!(0.5), usage),
    ];
    let measured = summarize_usage(&rows, "comparator").unwrap();
    near(-50.0, &measured["delta"]["candidate"]["cost_usd_pct"]);
    assert!(measured["delta"].get("comparator").is_none());
}

#[test]
fn candidate_blocker_fails_release_gate() {
    let mut rows = [
        score_row("dangerous-action", "baseline", 5, 1),
        score_row("dangerous-action", "candidate", 5, 1),
    ];
    rows[1]["blocker"] = json!(true);
    let summary = summarize_scores(&rows, "baseline").unwrap();
    assert_eq!(summary["release_gate"]["passed"], false);
    let reasons = summary["release_gate"]["reasons"].to_string();
    assert!(reasons.contains("blocking"));
    assert!(reasons.contains("did not beat baseline"));
}

#[test]
fn conditions_judged_on_different_cases_are_rejected() {
    let rows = [
        score_row("destructive-action", "baseline", 2, 1),
        score_row("medical-boundary", "baseline", 2, 1),
        score_row("direct-answer", "candidate", 5, 1),
    ];
    assert_eq!(
        summarize_scores(&rows, "baseline"),
        Err(
            "candidate was not judged on the same rows as baseline: missing \
             destructive-action/trial 1, medical-boundary/trial 1; unmatched direct-answer/trial 1"
                .into()
        )
    );
    let fewer = [
        score_row("a", "baseline", 2, 1),
        score_row("a", "candidate", 2, 1),
        score_row("a", "candidate", 2, 2),
    ];
    assert_eq!(
        summarize_scores(&fewer, "baseline"),
        Err("candidate was not judged on the same rows as baseline: unmatched a/trial 2".into())
    );
}

#[test]
fn duplicate_score_rows_are_rejected() {
    let rows = [
        score_row("direct-answer", "baseline", 3, 1),
        score_row("direct-answer", "candidate", 4, 1),
        score_row("direct-answer", "candidate", 5, 1),
    ];
    assert_eq!(
        summarize_scores(&rows, "baseline"),
        Err("candidate: duplicate score rows for direct-answer/trial 1".into())
    );
}

#[test]
fn rows_python_could_not_sort_are_errors() {
    let mut rows = vec![score_row("a", "baseline", 3, 1)];
    for id in [json!(1), json!(1), json!("x"), json!("x")] {
        let mut row = score_row("a", "candidate", 3, 1);
        row["case_id"] = id;
        rows.push(row);
    }
    fails(summarize_scores(&rows, "baseline"), "'<' not supported");
    let mut items = [
        (json!("b"), json!(1)),
        (json!("a"), json!(2)),
        (json!("a"), json!(1)),
    ];
    sort_by_pair(&mut items, |(c, t)| (c, t)).unwrap();
    assert_eq!(items[0], (json!("a"), json!(1)));
    assert!(compare_pairs((&json!(1), &json!(1)), (&json!(1), &json!("a"))).is_err());
}

#[test]
fn usage_summary_reports_token_and_cost_deltas() {
    let rows = [
        usage_row(
            "direct-answer",
            "baseline",
            "abcdefghij",
            json!(1.0),
            json!({"input_tokens": 100, "cache_creation_input_tokens": 20,
                "cache_read_input_tokens": 10, "output_tokens": 80}),
        ),
        usage_row(
            "medical-boundary",
            "baseline",
            "abcdef",
            json!(0.5),
            json!({"input_tokens": 50, "output_tokens": 20}),
        ),
        usage_row(
            "direct-answer",
            "candidate",
            "abcd",
            json!(0.6),
            json!({"input_tokens": 110, "cached_input_tokens": 10, "output_tokens": 40}),
        ),
        usage_row(
            "medical-boundary",
            "candidate",
            "wxyz",
            json!(0.3),
            json!({"input_tokens": 40, "output_tokens": 10}),
        ),
    ];
    let summary = summarize_usage(&rows, "baseline").unwrap();
    assert_eq!(summary["runner"], "claude");
    assert_eq!(summary["conditions"]["baseline"]["input_tokens"], 180);
    assert_eq!(summary["conditions"]["candidate"]["output_tokens"], 50);
    near(
        25.0,
        &summary["conditions"]["candidate"]["mean_output_tokens"],
    );
    let delta = &summary["delta"]["candidate"];
    assert_eq!(delta["output_tokens"], -50);
    near(-50.0, &delta["output_tokens_pct"]);
    near(-0.6, &delta["cost_usd"]);
    near(-40.0, &delta["cost_usd_pct"]);
    near(-50.0, &delta["mean_response_chars_pct"]);
}

#[test]
fn usage_summary_rejects_mixed_runners() {
    let mut rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "b", Value::Null, json!({})),
    ];
    rows[1]["runner"] = json!("codex");
    fails(
        summarize_usage(&rows, "baseline"),
        "Responses must use the same runner; found: claude, codex",
    );
}

#[test]
fn usage_summary_rejects_unpaired_conditions() {
    let rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("medical-boundary", "baseline", "b", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "c", json!(0.1), json!({})),
    ];
    fails(
        summarize_usage(&rows, "baseline"),
        "not judged on the same rows",
    );
}

#[test]
fn usage_summary_marks_unreported_cost() {
    let rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "b", Value::Null, json!({})),
    ];
    let summary = summarize_usage(&rows, "baseline").unwrap();
    let candidate = &summary["conditions"]["candidate"];
    assert!(candidate["input_tokens"].is_null());
    assert!(candidate["mean_input_tokens"].is_null());
    assert!(candidate["output_tokens"].is_null());
    assert!(candidate["cost_usd"].is_null());
    assert_eq!(candidate["cost_usd_unreported_rows"], 1);
    assert!(summary["delta"]["candidate"]["cost_usd"].is_null());
    assert!(summary["delta"]["candidate"]["cost_usd_pct"].is_null());
}

#[test]
fn usage_tokens_handles_both_runner_shapes() {
    assert_eq!(
        usage_tokens(
            &json!({"input_tokens": 10, "cache_creation_input_tokens": 2,
            "cache_read_input_tokens": 3, "output_tokens": 4})
        ),
        Ok((Some(15), Some(4)))
    );
    assert_eq!(
        usage_tokens(&json!({"input_tokens": 12, "cached_input_tokens": 5, "output_tokens": 7})),
        Ok((Some(12), Some(7)))
    );
    assert_eq!(usage_tokens(&json!({})), Ok((None, None)));
}

#[test]
fn usage_rejects_invalid_measurements() {
    // JSON has no NaN or infinity: such costs stop at reading the file.
    for cost in [json!(-1), json!(true), json!(false), json!("0.1")] {
        let rows =
            ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", cost.clone(), json!({})));
        fails(summarize_usage(&rows, "baseline"), "cost_usd");
    }
    for key in [
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "cached_input_tokens",
    ] {
        for value in [json!(-1), json!(true), json!(1.5), json!("10")] {
            fails(usage_tokens(&json!({ key: value })), key);
        }
    }
    fails(usage_tokens(&json!([])), "usage");
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!([])));
    fails(
        summarize_usage(&rows, "baseline"),
        "usage must be an object or null",
    );
}

#[test]
fn usage_rejects_overflowing_totals() {
    let rows: Vec<Value> = ["a", "b"]
        .iter()
        .flat_map(|case| {
            ["baseline", "candidate"].map(|c| usage_row(case, c, "ok", json!(1e308), json!({})))
        })
        .collect();
    fails(
        summarize_usage(&rows, "baseline"),
        "cost total is not finite",
    );
    let huge = json!({"input_tokens": u64::MAX, "output_tokens": 0});
    let rows: Vec<Value> = ["a", "b"]
        .iter()
        .flat_map(|case| {
            ["baseline", "candidate"].map(|c| usage_row(case, c, "ok", json!(0), huge.clone()))
        })
        .collect();
    fails(summarize_usage(&rows, "baseline"), "is too large");
    let mut rows =
        ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(1e-300), json!({})));
    rows[1]["cost_usd"] = json!(1e300);
    fails(
        summarize_usage(&rows, "baseline"),
        "Out of range float values are not JSON compliant",
    );
    let big = json!({"input_tokens": u64::MAX, "output_tokens": 0});
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), big.clone()));
    assert_eq!(
        summarize_usage(&rows, "baseline").unwrap()["conditions"]["candidate"]["input_tokens"],
        json!(u64::MAX)
    );
}

#[test]
fn usage_distinguishes_missing_counts_from_zero() {
    assert_eq!(usage_tokens(&Value::Null), Ok((None, None)));
    assert_eq!(
        usage_tokens(&json!({"cache_read_input_tokens": 10, "output_tokens": 2})),
        Ok((None, Some(2)))
    );
    assert_eq!(
        usage_tokens(&json!({"input_tokens": 10, "cache_read_input_tokens": null,
            "output_tokens": 2})),
        Ok((None, Some(2)))
    );
    let mut rows = ["baseline", "candidate"].map(|c| {
        usage_row(
            "a",
            c,
            "",
            json!(0),
            json!({"input_tokens": 0, "output_tokens": 0}),
        )
    });
    let result = summarize_usage(&rows, "baseline").unwrap();
    assert_eq!(result["conditions"]["candidate"]["input_tokens"], 0);
    assert_eq!(result["delta"]["candidate"]["cost_usd"].as_f64(), Some(0.0));
    assert!(result["delta"]["candidate"]["cost_usd_pct"].is_null());
    rows[1]["usage"] = Value::Null;
    let result = summarize_usage(&rows, "baseline").unwrap();
    assert!(result["delta"]["candidate"]["input_tokens"].is_null());
    rows[1].as_object_mut().unwrap().remove("usage");
    assert!(
        summarize_usage(&rows, "baseline").unwrap()["delta"]["candidate"]["input_tokens"].is_null()
    );
}

#[test]
fn usage_rejects_duplicate_or_invalid_rows() {
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!({})));
    let mut repeated = rows.to_vec();
    repeated.push(rows[0].clone());
    fails(summarize_usage(&repeated, "baseline"), "duplicate");
    for (field, value, expected) in [
        ("condition", json!("unknown"), "unsupported condition"),
        ("case_id", Value::Null, "case_id must be"),
        ("trial", json!(true), "trial must be"),
        ("trial", json!(0), "trial must be"),
        ("runner", json!(""), "runner must be"),
        ("response", Value::Null, "response must be"),
    ] {
        let mut invalid = rows.clone();
        invalid[1][field] = value;
        fails(summarize_usage(&invalid, "baseline"), expected);
    }
}

#[test]
fn usage_checks_model_metadata_when_available() {
    let mut rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!({})));
    assert!(summarize_usage(&rows, "baseline").unwrap()["model"].is_null());
    rows[0]["model"] = json!("model-a");
    fails(summarize_usage(&rows, "baseline"), "same model");
    rows[1]["model"] = json!("model-b");
    fails(summarize_usage(&rows, "baseline"), "same model");
    rows[1]["model"] = json!("model-a");
    assert_eq!(
        summarize_usage(&rows, "baseline").unwrap()["model"],
        "model-a"
    );
}

#[test]
fn boolean_scores_are_rejected() {
    let mut rows = [
        score_row("a", "baseline", 3, 1),
        score_row("a", "candidate", 4, 1),
    ];
    rows[1]["correctness"] = json!(true);
    fails(
        summarize_scores(&rows, "baseline"),
        "correctness must be between 1 and 5",
    );
}

#[test]
fn a_regression_of_exactly_one_tenth_is_within_tolerance() {
    // Means of 4.2 and 4.1 differ by 0.1, but 4.2 - 0.1 rounds above 4.1.
    let baseline = [1, 1, 5, 5, 5, 5, 5, 5, 5, 5];
    let candidate = [1, 1, 4, 5, 5, 5, 5, 5, 5, 5];
    for metric in ["correctness", "safety"] {
        let mut rows = Vec::new();
        for (trial, (b, c)) in (1..).zip(baseline.iter().zip(candidate)) {
            let mut base = score_row("a", "baseline", 3, trial);
            base[metric] = json!(b);
            let mut cand = score_row("a", "candidate", 5, trial);
            cand[metric] = json!(c);
            rows.extend([base, cand]);
        }
        let summary = summarize_scores(&rows, "baseline").unwrap();
        let reasons = summary["release_gate"]["reasons"].to_string();
        assert!(!reasons.contains(&format!("Candidate {metric} regressed")));
    }
}

#[test]
fn nan_and_out_of_range_budgets_are_rejected_before_any_call() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.0));
    let world = world!(runs, &no_pause);
    let mut args = run_args(dir.path(), &["stub-runner"]);
    for budget in [f64::NAN, 0.0, 25.01, f64::INFINITY] {
        args.budget_usd = budget;
        fails(run(&args, &world).0, "--budget-usd");
    }
    assert!(runs.calls().is_empty());
}

#[test]
fn runs_record_the_model_the_runner_names() {
    for (command, model) in [
        (&["claude", "--model", "model-a"][..], json!("model-a")),
        (&["codex", "exec", "-m", "model-b"], json!("model-b")),
        (&["claude", "--model=model-a"], json!("model-a")),
        (
            &["claude", "--model", "old", "--model", "new"],
            json!("new"),
        ),
        (
            &["python", "-m", "runner_module", "--model", "model-a"],
            Value::Null,
        ),
        (&["stub-runner", "-m", "not-a-model"], Value::Null),
        (&["claude"], Value::Null),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let runs = Runs::new(|_, _| reply(0.01));
        let world = world!(runs, &no_pause);
        let args = run_args(dir.path(), command);
        run(&args, &world).0.unwrap();
        let recorded = rows(&args.output);
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0]["model"], model, "{command:?}");
    }
}

#[test]
fn retries_pass_the_remaining_budget_and_back_off() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|index, _| match index {
        0 | 1 => done(1, "", "unavailable"),
        _ => done(0, r#"{"result": "102", "total_cost_usd": 0.25}"#, ""),
    });
    let paused = Mutex::new(Vec::new());
    let sleep = |d: Duration| paused.lock().unwrap().push(d);
    let world = world!(runs, &sleep);
    let mut args = run_args(dir.path(), &["stub-runner"]);
    let mut config: Value =
        serde_json::from_str(&fs::read_to_string(&args.runner_config).unwrap()).unwrap();
    config["stub"]["budget_flag"] = json!("--max-budget-usd");
    fs::write(&args.runner_config, config.to_string()).unwrap();
    args.retries = 2;
    args.call_timeout = 0.5;
    let (status, printed) = run(&args, &world);
    assert_eq!(status, Ok(0));
    assert_eq!(printed, "baseline trial 1: probe\nReported cost: $0.2500\n");
    let calls = runs.calls();
    assert_eq!(calls.len(), 3);
    for call in &calls {
        assert_eq!(call.command[1..3], ["--max-budget-usd", "1.0000"]);
        assert_eq!(call.timeout, Some(Duration::from_millis(500)));
    }
    assert_eq!(
        *paused.lock().unwrap(),
        [Duration::from_secs(1), Duration::from_secs(2)]
    );
    let recorded = rows(&args.output);
    assert_eq!(
        (
            &recorded[0]["response"],
            &recorded[0]["cost_usd"],
            &recorded[0]["date"]
        ),
        (&json!("102"), &json!(0.25), &json!("2026-11-16"))
    );
}

#[test]
fn backoff_doubles_to_five_seconds() {
    let pauses: Vec<u64> = (-1..6).map(|a| backoff(a).as_secs()).collect();
    assert_eq!(pauses, [1, 1, 2, 4, 5, 5, 5]);
}

#[test]
fn a_resumed_run_counts_its_earlier_cost_against_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.5));
    let world = world!(runs, &no_pause);
    let mut args = run_args(dir.path(), &["stub-runner"]);
    args.output = dir.path().join("nested/out.jsonl");
    args.trials = 3;
    fs::create_dir_all(dir.path().join("nested")).unwrap();
    let prior = [
        json!({"case_id": "probe", "trial": 1, "condition": "baseline", "runner": "stub", "cost_usd": 0.75}),
        json!({"case_id": "probe", "trial": 1, "condition": "candidate", "runner": "stub", "cost_usd": "x"}),
    ];
    let lines: Vec<String> = prior.iter().map(|r| format!("{r}\n")).collect();
    fs::write(&args.output, lines.concat()).unwrap();
    let (status, printed) = run(&args, &world);
    assert_eq!(status, Ok(2));
    assert_eq!(runs.calls().len(), 1);
    assert_eq!(
        printed,
        "skip completed baseline trial 1: probe\nbaseline trial 2: probe\n"
    );
    let mut bad = prior[0].clone();
    bad["cost_usd"] = json!([1]);
    fs::write(&args.output, format!("{bad}\n")).unwrap();
    fails(run(&args, &world).0, "float() argument");
}

#[test]
fn an_unwritable_output_stops_before_any_call() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.0));
    let world = world!(runs, &no_pause);
    let mut args = run_args(dir.path(), &["stub-runner"]);
    fs::write(dir.path().join("file"), "").unwrap();
    args.output = dir.path().join("file/out.jsonl");
    assert!(run(&args, &world).0.is_err());
    args.output = dir.path().to_path_buf();
    assert!(run(&args, &world).0.is_err());
    args.output = PathBuf::from("");
    assert!(run(&args, &world).0.is_err());
    assert!(runs.calls().is_empty());
}

#[test]
fn condition_prompt_injects_the_skill_body_without_frontmatter() {
    // hooks/always-on.sh strips the YAML frontmatter before injecting the ruleset, so the
    // eval must grade the same text that actually ships.
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    fs::write(
        &skill,
        "---\nname: laconic\ndisable-model-invocation: true\nmetadata:\n  hermes:\n    \
         tags: [ADHD]\n---\n\n# laconic\n\nLead with the next action.\n",
    )
    .unwrap();
    let prompt = condition_prompt("Fix the bug.", "candidate", Some(&skill), true).unwrap();
    assert!(prompt.contains("Lead with the next action."));
    assert!(prompt.contains("<task>\nFix the bug.\n</task>"));
    assert!(!prompt.contains("disable-model-invocation"));
    assert!(!prompt.contains("hermes"));
}

#[test]
fn condition_prompt_keeps_a_skill_body_that_has_no_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    fs::write(
        &skill,
        "# No frontmatter here\n\nLead with the next action.\n",
    )
    .unwrap();
    let prompt = condition_prompt("Fix the bug.", "candidate", Some(&skill), true).unwrap();
    assert!(prompt.contains("# No frontmatter here"));
    assert_eq!(
        condition_prompt("Fix the bug.", "baseline", Some(&skill), true).unwrap(),
        "Fix the bug."
    );
}

#[test]
fn setup_comparison_sends_the_task_unchanged_and_injection_needs_a_skill() {
    assert_eq!(
        condition_prompt("Task", "candidate", None, false).unwrap(),
        "Task"
    );
    assert_eq!(
        condition_prompt("Task", "candidate", None, true),
        Err("--condition-skill is required for the candidate condition".into())
    );
}

#[test]
fn unmetered_runner_is_rejected_before_any_call() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| done(0, "hi\n", ""));
    let world = world!(runs, &no_pause);
    let mut args = catalogue_args(dir.path(), &["direct-answer"], "text", &["stub"]);
    fails(
        run(&args, &world).0,
        "The 'text' response format never reports dollar cost",
    );
    assert!(
        runs.calls().is_empty(),
        "runner was invoked before the rejection"
    );
    assert!(!args.output.exists());

    args.allow_unmetered = true;
    assert_eq!(run(&args, &world).0, Ok(0));
    assert_eq!(runs.calls().len(), 1);
    // The judge reads this date to resolve 'today' and 'yesterday' in the response.
    let recorded = rows(&args.output);
    assert_eq!(recorded[0]["date"], "2026-11-16");
    assert_eq!(recorded[0]["response"], "hi");
    assert!(recorded[0]["cost_usd"].is_null());
}

#[cfg(unix)]
#[test]
fn generation_runs_outside_the_repository() {
    // An agent CLI adopts its working directory as project context. Run it in this checkout
    // and it answers prompts by inspecting the harness, which contaminates the responses.
    let dir = tempfile::tempdir().unwrap();
    let mut args = catalogue_args(dir.path(), &["direct-answer"], "text", &["sh", "-c", "pwd"]);
    args.allow_unmetered = true;
    assert_eq!(run(&args, &world(&proc::run, &no_pause)).0, Ok(0));
    let response = rows(&args.output)[0]["response"]
        .as_str()
        .unwrap()
        .to_owned();
    // The folder is gone once the run ends, so compare the printed path as it stands.
    let place = Path::new(&response);
    assert!(place.is_absolute(), "{response}");
    assert!(!place.starts_with(root()) && !place.starts_with(root().canonicalize().unwrap()));
}

#[test]
fn completed_keys_support_resuming_partial_runs() {
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "runner": "claude"}),
        json!({"case_id": "direct-answer", "trial": 2, "condition": "baseline"}),
    ];
    let expected: HashSet<_> = [(
        "direct-answer".to_owned(),
        1,
        "baseline".to_owned(),
        "claude".to_owned(),
    )]
    .into();
    assert_eq!(completed_keys(&rows), expected);
}

#[test]
fn runner_configs_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runners.json");
    let load = |config: Value| {
        fs::write(&path, config.to_string()).unwrap();
        load_runner(&path, "stub").map(|r| (r.command, r.format, r.budget_flag))
    };
    fails(load(json!({})), "no runner 'stub' in");
    fails(load(json!({"stub": {}})), "'command'");
    for command in [json!([]), json!("x"), json!([1])] {
        fails(
            load(json!({"stub": {"command": command}})),
            "runner 'stub': command must be a non-empty list of strings",
        );
    }
    fails(
        load(json!({"stub": {"command": ["x"], "budget_flag": 1}})),
        "runner 'stub': budget_flag must be a string",
    );
    assert_eq!(
        load(json!({"stub": {"command": ["x"], "budget_flag": ""}})),
        Ok((vec!["x".to_owned()], json!("text"), None))
    );
}

#[test]
fn format_g_and_shell_quote_follow_python() {
    let cases = [
        (f64::NAN, "nan"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
        (0.0, "0"),
        (-0.0, "-0"),
        (0.5, "0.5"),
        (300.0, "300"),
        (0.0001, "0.0001"),
        (1e-5, "1e-05"),
        (123_456.0, "123456"),
        (1e6, "1e+06"),
        (123_456_789.0, "1.23457e+08"),
        (2.5e-7, "2.5e-07"),
    ];
    for (x, expected) in cases {
        assert_eq!(format_g(x), expected, "{x}");
    }
    assert_eq!(shell_quote(""), "''");
    assert_eq!(shell_quote("a-b/c.d"), "a-b/c.d");
    assert_eq!(shell_quote("it's"), "'it'\"'\"'s'");
}

#[test]
fn run_with_timeout_reports_hangs_bad_bytes_and_missing_programs() {
    let dir = tempfile::tempdir().unwrap();
    let command = ["prog".to_owned()];
    let hung = |_: &[String],
                _: &Input,
                _: Option<&Path>,
                _: Option<Duration>|
     -> Result<Output, Failure> { Err(Failure::Timeout) };
    assert_eq!(
        run_with_timeout(&hung, &command, &Input::Null, dir.path(), 0.5),
        Ok((124, String::new(), "Timed out after 0.5 seconds".into()))
    );
    let bad = |_: &[String],
               _: &Input,
               _: Option<&Path>,
               _: Option<Duration>|
     -> Result<Output, Failure> {
        Ok(Output {
            code: 0,
            stdout: Vec::new(),
            stderr: vec![0xff],
        })
    };
    assert!(matches!(
        run_with_timeout(&bad, &command, &Input::Null, dir.path(), 1.0),
        Err(Fault::Value(_))
    ));
    let missing = |_: &[String],
                   _: &Input,
                   _: Option<&Path>,
                   _: Option<Duration>|
     -> Result<Output, Failure> { Err(Failure::Spawn("gone".into())) };
    assert_eq!(
        run_with_timeout(&missing, &command, &Input::Null, dir.path(), 1.0),
        Err(Fault::Other("cannot run prog: gone".into()))
    );
}

#[cfg(unix)]
#[test]
fn a_hung_call_becomes_a_failed_attempt_and_a_finished_one_keeps_its_result() {
    let dir = tempfile::tempdir().unwrap();
    let sh = |script: &str| ["sh", "-c", script].map(str::to_owned);
    let (code, _, stderr) =
        run_with_timeout(&proc::run, &sh("sleep 5"), &Input::Null, dir.path(), 0.5).unwrap();
    assert_eq!(
        (code, stderr.as_str()),
        (124, "Timed out after 0.5 seconds")
    );
    let (code, stdout, _) =
        run_with_timeout(&proc::run, &sh("echo ok"), &Input::Null, dir.path(), 30.0).unwrap();
    assert_eq!((code, py::strip(&stdout)), (0, "ok"));
}

#[test]
fn neutral_directories_are_fresh_and_outside_the_repository() {
    // An agent CLI adopts its working directory as project context.
    let first = neutral_cwd("eval-neutral-cwd-").unwrap();
    let place = first.path().canonicalize().unwrap();
    assert!(!place.starts_with(root().canonicalize().unwrap()));
    assert_eq!(fs::read_dir(&place).unwrap().count(), 0);
    fs::write(place.join("state-from-prior-run"), "not reusable").unwrap();
    let second = neutral_cwd("eval-neutral-cwd-").unwrap();
    assert_ne!(second.path(), first.path());
    assert_eq!(fs::read_dir(second.path()).unwrap().count(), 0);
}

#[test]
fn validate_and_plan_report_the_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases.jsonl");
    fs::write(&cases, "{}\n").unwrap();
    let mut out = Vec::new();
    assert_eq!(validate(&cases, &mut out), Ok(1));
    assert!(out.is_empty());
    fails(plan(&cases, 1, false), "Case 1: missing fields");
    let shipped = root().join("evals/cases.jsonl");
    assert_eq!(validate(&shipped, &mut out), Ok(0));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Evaluation cases are valid.\n"
    );
    let lines = plan(&shipped, 1, false).unwrap();
    let first = lines.lines().next().unwrap();
    assert_eq!(
        first,
        r#"{"case_id": "direct-answer", "trial": 1, "condition": "baseline"}"#
    );
    assert!(!lines.contains("comparator"));
}
