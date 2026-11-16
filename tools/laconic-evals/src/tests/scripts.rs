// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

#[test]
fn stops_convert_from_messages() {
    assert_eq!(Stop::from("x"), Stop::Failed("x".into()));
    assert_eq!(Stop::from(String::from("y")), Stop::Failed("y".into()));
}

fn today_stub() -> String {
    "2026-11-16".into()
}

fn session_stub() -> String {
    "session-1".into()
}

fn rows_text(rows: &[Value]) -> String {
    rows.iter().map(|r| format!("{r}\n")).collect()
}

fn usage_row(condition: &str, response: &str, cost: f64, usage: Value) -> Value {
    json!({"case_id": "a", "trial": 1, "condition": condition, "runner": "claude",
        "response": response, "usage": usage, "cost_usd": cost})
}

#[test]
fn validate_and_plan_read_the_shipped_catalogue_by_default() {
    assert_eq!(
        call(&["validate"], &NONE),
        (Ok(0), "Evaluation cases are valid.\n".into())
    );
    let holdout = cases::root().join("evals/holdout-cases.jsonl");
    let (status, printed) = call(
        &[
            "plan",
            "--cases",
            &arg(&holdout),
            "--trials",
            "2",
            "--include-comparator",
        ],
        &NONE,
    );
    assert_eq!(status, Ok(0));
    assert!(fails_on_a_closed_output(&["plan"], &NONE));
    let ids = cases::load_cases(&holdout).unwrap().len();
    let rows: Vec<Value> = printed
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), ids * 2 * 3);
    let distinct: std::collections::HashSet<String> = rows.iter().map(Value::to_string).collect();
    assert_eq!(distinct.len(), rows.len());
    let (status, printed) = call(&["plan"], &NONE);
    assert_eq!(status, Ok(0));
    assert!(!printed.contains("comparator"));

    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("cases.jsonl");
    fs::write(&bad, "{}\n").unwrap();
    assert_eq!(
        call(&["validate", "--cases", &arg(&bad)], &NONE),
        (Ok(1), String::new())
    );
    let (status, _) = call(&["plan", "--cases", &arg(&bad)], &NONE);
    assert_eq!(
        status,
        Err(Stop::Failed(
            "Case 1: missing fields: category, criteria, id, prompt, risk".into()
        ))
    );
    for command in [
        &["plan"][..],
        &[
            "run",
            "--runner",
            "x",
            "--condition",
            "baseline",
            "--output",
            "o",
        ],
    ] {
        for trials in ["0", "-1"] {
            let flag = ["--trials", trials];
            let line = ["laconic-evals"].iter().chain(command).chain(&flag);
            assert!(Cli::try_parse_from(line).is_err());
        }
    }
}

#[test]
fn checks_command_scores_recorded_responses_and_rejects_a_bad_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases.jsonl");
    let mut case = json!({"id": "probe", "category": "direct-answer",
        "prompt": "What is 17 multiplied by 6?", "risk": "low", "criteria": ["Answers 102."],
        "checks": {"must_include": [r"\b102\b"], "pass": ["102"], "fail": ["112"]}});
    fs::write(&cases, format!("{case}\n")).unwrap();
    let responses = dir.path().join("responses.jsonl");
    fs::write(
        &responses,
        rows_text(&[
            json!({"case_id": "probe", "trial": 1, "condition": "candidate", "response": "102"}),
            json!({"case_id": "probe", "trial": 2, "condition": "candidate", "response": "112"}),
        ]),
    )
    .unwrap();
    let (responses_arg, cases_arg) = (arg(&responses), arg(&cases));
    let command = ["checks", &responses_arg, "--cases", &cases_arg];
    let (status, printed) = call(&command, &NONE);
    assert_eq!(status, Ok(0));
    let summary: Value = serde_json::from_str(&printed).unwrap();
    let summary = &summary["candidate"];
    assert_eq!(
        (&summary["checked"], &summary["passed"]),
        (&json!(2), &json!(1))
    );
    assert_eq!(summary["failures"][0]["trial"], 2);
    case["checks"]["fail"] = json!([]);
    fs::write(&cases, format!("{case}\n")).unwrap();
    match call(&command, &NONE).0 {
        Err(Stop::Failed(message)) => {
            assert!(
                message.contains("at least one pass and one fail"),
                "{message}"
            );
        }
        other => panic!("{other:?}"),
    }
    let (status, _) = call(&["checks", &arg(&dir.path().join("none"))], &NONE);
    assert!(status.is_err());
}

#[test]
fn score_and_measure_print_their_summaries_against_a_reference() {
    let dir = tempfile::tempdir().unwrap();
    let score = |condition: &str, value: i64| {
        json!({"case_id": "direct-answer", "trial": 1, "condition": condition,
            "correctness": value, "autonomy": value, "actionability": value, "safety": value,
            "concision": value, "blocker": false, "notes": "fixture"})
    };
    let scores = [score("comparator", 3), score("candidate", 4)];
    let usage = json!({"input_tokens": 10, "output_tokens": 5});
    let usages = [
        usage_row("comparator", "abcd", 1.0, usage.clone()),
        usage_row("candidate", "ab", 0.5, usage),
    ];
    for (command, rows, expected) in [
        (
            "score",
            &scores,
            run_evals::summarize_scores(&scores, "comparator").unwrap(),
        ),
        (
            "measure",
            &usages,
            run_evals::summarize_usage(&usages, "comparator").unwrap(),
        ),
    ] {
        let path = dir.path().join(format!("{command}.jsonl"));
        fs::write(&path, rows_text(rows)).unwrap();
        let (status, printed) = call(&[command, &arg(&path), "--reference", "comparator"], &NONE);
        assert_eq!(status, Ok(0));
        assert_eq!(printed, format!("{}\n", py::dumps_pretty(&expected)));
        let (status, _) = call(&[command, &arg(&path)], &NONE);
        assert!(status.is_err());
        let line = ["laconic-evals", command, "x", "--reference", "candidate"];
        assert!(Cli::try_parse_from(line).is_err());
        let (status, _) = call(&[command, &arg(&dir.path().join("none"))], &NONE);
        assert!(status.is_err());
    }
}

#[test]
fn measure_reports_increased_input_and_cost_and_leaves_its_input_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("responses.jsonl");
    let contents = rows_text(&[
        usage_row(
            "baseline",
            "long answer",
            0.1,
            json!({"input_tokens": 100, "output_tokens": 20}),
        ),
        usage_row(
            "candidate",
            "short",
            0.15,
            json!({"input_tokens": 150, "output_tokens": 10}),
        ),
    ]);
    fs::write(&path, &contents).unwrap();
    let (status, printed) = call(&["measure", &arg(&path)], &NONE);
    assert_eq!(status, Ok(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), contents);
    let summary: Value = serde_json::from_str(&printed).unwrap();
    let delta = &summary["delta"]["candidate"];
    assert_eq!(delta["input_tokens"], 50);
    assert_eq!(delta["input_tokens_pct"].as_f64(), Some(50.0));
    assert_eq!(delta["output_tokens_pct"].as_f64(), Some(-50.0));
    assert!((delta["cost_usd_pct"].as_f64().unwrap() - 50.0).abs() < 1e-7);
}

#[test]
fn run_retries_with_the_remaining_budget_and_records_one_response() {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases.jsonl");
    let case = json!({"id": "probe", "category": "direct-answer",
        "prompt": "What is 17 multiplied by 6?", "risk": "low", "criteria": ["Answers 102."]});
    fs::write(&cases, format!("{case}\n")).unwrap();
    let config = dir.path().join("runners.json");
    let stub = json!({"stub": {"command": ["stub-runner"], "response_format": "claude-json",
        "budget_flag": "--max-budget-usd"}});
    fs::write(&config, stub.to_string()).unwrap();
    let output = dir.path().join("responses.jsonl");
    let runs = crate::testing::Runs::new(|index, _| match index {
        0 => done(1, "", "unavailable"),
        _ => done(0, r#"{"result": "102", "total_cost_usd": 0.25}"#, ""),
    });
    let paused = Mutex::new(Vec::new());
    let sleep = |d: Duration| paused.lock().unwrap().push(d);
    let tools = Tools {
        runner: &|c, i, d, t| runs.exec(c, i, d, t),
        sleep: &sleep,
        today: &today_stub,
        ..NONE
    };
    let (cases_arg, config_arg, output_arg) = (arg(&cases), arg(&config), arg(&output));
    let line = [
        "run",
        "--cases",
        &cases_arg,
        "--runner-config",
        &config_arg,
        "--runner",
        "stub",
        "--condition",
        "baseline",
        "--trials",
        "1",
        "--retries",
        "1",
        "--budget-usd",
        "1",
        "--output",
        &output_arg,
    ];
    let (status, printed) = call(&line, &tools);
    assert_eq!(status, Ok(0));
    assert_eq!(printed, "baseline trial 1: probe\nReported cost: $0.2500\n");
    let rows = cases::read_jsonl(&output).unwrap();
    assert_eq!(
        (&rows[0]["response"], &rows[0]["cost_usd"]),
        (&json!("102"), &json!(0.25))
    );
    let calls = runs.calls();
    assert_eq!(calls.len(), 2);
    for call in &calls {
        assert_eq!(call.command[1..3], ["--max-budget-usd", "1.0000"]);
        assert_eq!(call.timeout, Some(Duration::from_secs(300)));
    }
    assert_eq!(*paused.lock().unwrap(), [Duration::from_secs(1)]);
    let mut timed = line.to_vec();
    timed.extend(["--call-timeout", "-1"]);
    assert!(matches!(call(&timed, &tools).0, Err(Stop::Usage(_))));
    let defaults = [
        "run",
        "--runner",
        "stub",
        "--condition",
        "baseline",
        "--output",
        &output_arg,
    ];
    assert!(matches!(call(&defaults, &tools).0, Err(Stop::Failed(_))));
}

#[test]
fn judge_needs_responses_and_output_unless_calibrating() {
    let (status, _) = call(
        &["judge", "--runner", "claude", "--output", "scores.jsonl"],
        &NONE,
    );
    assert_eq!(
        status,
        Err(Stop::Usage(
            "--responses and --output are required unless --calibrate is given".into()
        ))
    );
    let (status, _) = call(
        &["judge", "--runner", "claude", "--responses", "r.jsonl"],
        &NONE,
    );
    assert!(matches!(status, Err(Stop::Usage(_))));
    let line = [
        "judge",
        "--runner",
        "c",
        "--responses",
        "r",
        "--output",
        "o",
        "--call-timeout",
        "nan",
    ];
    assert!(matches!(call(&line, &NONE).0, Err(Stop::Usage(_))));
    let line = [
        "laconic-evals",
        "judge",
        "--runner",
        "r",
        "--conditions",
        "baseline",
        "comparator",
    ];
    let cli = Cli::try_parse_from(line).unwrap();
    assert!(format!("{:?}", cli.command).contains("conditions: [\"baseline\", \"comparator\"]"));
}

#[test]
fn judge_calibrates_and_judges_through_the_runner() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("runners.json");
    let stub = json!({"stub": {"command": ["stub"], "response_format": "claude-json"}});
    fs::write(&config, stub.to_string()).unwrap();
    let verdict = json!({"correctness": 4, "autonomy": 4, "actionability": 4, "safety": 5,
        "concision": 3, "blocker": false, "notes": "fixture"});
    let result = json!({"A": verdict, "B": verdict}).to_string();
    let reply = json!({"result": result, "total_cost_usd": 0.25}).to_string();
    let runs = crate::testing::Runs::new(|_, _| done(0, &reply, ""));
    let tools = Tools {
        runner: &|c, i, d, t| runs.exec(c, i, d, t),
        ..NONE
    };
    let labelled = dir.path().join("labelled.jsonl");
    let fixture = json!({"case_id": "direct-answer", "name": "right", "response": "102",
        "blocker": false});
    fs::write(&labelled, rows_text(&[fixture])).unwrap();
    let config_arg = arg(&config);
    let common = ["judge", "--runner-config", &config_arg, "--runner", "stub"];
    let labelled_arg = arg(&labelled);
    let mut calibrate = common.to_vec();
    calibrate.extend(["--calibrate", &labelled_arg]);
    let (status, printed) = call(&calibrate, &tools);
    assert_eq!(status, Ok(0));
    assert_eq!(
        printed,
        "{\n  \"compared\": 1,\n  \"agreement\": 1.0,\n  \"errors\": [],\n  \"cost_usd\": 0.25\n}\n"
    );
    let responses = dir.path().join("responses.jsonl");
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "response": "x"}),
        json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate", "response": "y"}),
    ];
    fs::write(&responses, rows_text(&rows)).unwrap();
    let output = dir.path().join("scores.jsonl");
    let (responses_arg, output_arg) = (arg(&responses), arg(&output));
    let mut judge = common.to_vec();
    judge.extend(["--responses", &responses_arg, "--output", &output_arg]);
    assert_eq!(
        call(&judge, &tools),
        (
            Ok(0),
            "judged direct-answer/trial 1\nReported judge cost: $0.2500\n".into()
        )
    );
    assert_eq!(cases::read_jsonl(&output).unwrap().len(), 2);
    assert_eq!(runs.calls().len(), 2);
}

#[test]
fn scenario_validate_reports_valid_and_invalid_scenarios_without_a_provider() {
    let scenario = cases::root().join("evals/scenarios/persistence-topic-switch-stop");
    assert_eq!(
        call(&["scenario", "validate", &arg(&scenario)], &NONE),
        (
            Ok(0),
            "Valid scenario: persistence-topic-switch-stop\n".into()
        )
    );
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("case.jsonl"), "{}\n").unwrap();
    let (status, printed) = call(&["scenario", "validate", &arg(dir.path())], &NONE);
    assert!(
        matches!(status, Err(Stop::Failed(m)) if m.starts_with("Expected one valid scenario case"))
    );
    assert!(printed.is_empty());
}

#[test]
fn scenario_run_captures_one_session() {
    let dir = tempfile::tempdir().unwrap();
    let scenario = arg(&cases::root().join("evals/scenarios/persistence-topic-switch-stop"));
    let output = arg(&dir.path().join("baseline.jsonl"));
    let reply = r#"{"result": "ok", "total_cost_usd": 0.01}"#;
    let runs = crate::testing::Runs::new(|_, _| done(0, reply, ""));
    let tools = Tools {
        runner: &|c, i, d, t| runs.exec(c, i, d, t),
        session: &session_stub,
        ..NONE
    };
    let line = [
        "scenario",
        "run",
        "--scenario",
        &scenario,
        "--condition",
        "baseline",
        "--model",
        "fixture",
        "--budget-usd",
        "1",
        "--output",
        &output,
    ];
    let (status, printed) = call(&line, &tools);
    assert_eq!(status, Ok(0));
    assert!(printed.starts_with("Conversation saved to "), "{printed}");
    let row = &cases::read_jsonl(Path::new(&output)).unwrap()[0];
    assert_eq!(
        (&row["session_id"], &row["trial"]),
        (&json!("session-1"), &json!(1))
    );
    assert_eq!(runs.calls().len(), 6);
    assert!(matches!(call(&line, &tools).0, Err(Stop::Failed(_))));
    let mut wrong = line.to_vec();
    wrong[5] = "comparator";
    assert!(Cli::try_parse_from(["laconic-evals"].iter().chain(&wrong)).is_err());
}

#[test]
fn production_dates_and_sessions_have_their_forms() {
    let day = today();
    assert_eq!(day.len(), 10);
    assert_eq!(&day[4..5], "-");
    let id = session();
    assert_eq!(id.len(), 36);
    assert_ne!(id, session());
    assert_eq!(call_timeout(0.5), Ok(0.5));
    assert!(call_timeout(-0.5).is_err());
}
