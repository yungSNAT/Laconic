// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::proc::{Failure, Input, Output};
use crate::testing::{done, write_rows};
use serde_json::{Value, json};
use std::fs;
use std::sync::Mutex;

fn no_judge(_: &str, _: Duration) -> Result<(Value, f64), String> {
    panic!("no judge call expected")
}

fn no_exec(_: &[String], _: &Input) -> Result<Output, Failure> {
    panic!("no checker expected")
}

fn no_get(url: &str) -> Result<Vec<u8>, String> {
    panic!("no download expected: {url}")
}

fn no_git(_: &[String]) -> Result<(), String> {
    panic!("no git call expected")
}

fn no_claude(_: &[String], _: &Input, _: Duration) -> Result<Output, Failure> {
    panic!("no claude call expected")
}

fn no_run(_: &[String], _: &Input, _: &Path, _: Duration) -> Result<Output, Failure> {
    panic!("no command expected")
}

fn found(name: &str) -> Option<PathBuf> {
    Some(PathBuf::from(format!("/bin/{name}")))
}

const NONE: Tools<'static> = Tools {
    judge: &no_judge,
    checker: &no_exec,
    preserve: &no_exec,
    get: &no_get,
    git: &no_git,
    claude: &no_claude,
    run_in: &no_run,
    which: &found,
    runner: &no_runner,
    sleep: &no_sleep,
    today: &no_today,
    session: &no_session,
};

fn no_runner(
    _: &[String],
    _: &Input,
    _: Option<&Path>,
    _: Option<Duration>,
) -> Result<Output, Failure> {
    panic!("no runner call expected")
}

fn no_sleep(_: Duration) {
    panic!("no pause expected")
}

fn no_today() -> String {
    panic!("no date expected")
}

fn no_session() -> String {
    panic!("no session expected")
}

/// Runs the command line `args` and returns its status and what it printed.
fn call(args: &[&str], tools: &Tools) -> (Result<u8, Stop>, String) {
    let cli = Cli::try_parse_from(["laconic-evals"].iter().chain(args)).unwrap();
    let mut out = Vec::new();
    let status = run(cli, &mut out, tools);
    (status, String::from_utf8(out).unwrap())
}

/// Whether the command fails as it should when its standard output is closed.
fn fails_on_a_closed_output(args: &[&str], tools: &Tools) -> bool {
    let cli = Cli::try_parse_from(["laconic-evals"].iter().chain(args)).unwrap();
    let mut closed: &mut [u8] = &mut [];
    matches!(run(cli, &mut closed, tools), Err(Stop::Failed(message)) if message == "failed to write whole buffer")
}

fn arg(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn flags_keep_their_python_names_and_abbreviations() {
    Cli::command().debug_assert();
    let cli = Cli::try_parse_from([
        "laconic-evals",
        "fact-survival",
        "judge",
        "r.jsonl",
        "--out",
        "a",
        "--output",
        "b",
        "--budget",
        "3",
    ])
    .unwrap();
    let shown = format!("{:?}", cli.command);
    for field in [
        "output: \"b\"",
        "budget_usd: 3.0",
        "jobs: 4",
        "call_timeout: 300.0",
    ] {
        assert!(shown.contains(field), "{shown}");
    }
    assert!(Cli::try_parse_from(["laconic-evals", "fact-survival", "spend", "f"]).is_err());
}

#[test]
fn the_fact_audit_needs_an_executable_checker() {
    let missing = Tools {
        which: &|_| None,
        ..NONE
    };
    let (status, _) = call(&["fact-audit", "--laconic-check", "nothing"], &missing);
    assert_eq!(
        status,
        Err(Stop::Usage(
            "--laconic-check nothing is not an executable file".into()
        ))
    );
}

#[test]
fn the_fact_audit_reads_its_cache_and_prints_the_report() {
    let dir = tempfile::tempdir().unwrap();
    let units = dir
        .path()
        .join(fact_audit::FACT_AUDIT_COMMIT)
        .join("fact_units.jsonl");
    write_rows(
        &units,
        &[
            json!({"set": "commercial", "tool": "t", "aihb_cycle": fact_audit::CYCLE, "sample_id": "s1", "units": [{"group": "num", "surface": "4", "kept": false}]}),
        ],
    );
    let rewrites = dir
        .path()
        .join(fact_audit::AIHB_REVISION)
        .join(fact_audit::CYCLE)
        .join("t.jsonl");
    write_rows(
        &rewrites,
        &[json!({"humanizer": "t", "sample_id": "s1", "input": "It is 4.", "output": "It is."})],
    );
    let calls = Mutex::new(Vec::new());
    let preserve = |command: &[String], _: &Input| {
        calls.lock().unwrap().push(command.to_vec());
        done(1, "dropped number: 4\n", "")
    };
    let tools = Tools {
        preserve: &preserve,
        ..NONE
    };
    let (status, printed) = call(
        &[
            "fact-audit",
            "--laconic-check",
            "lc",
            "--cache",
            &arg(dir.path()),
            "--loose",
        ],
        &tools,
    );
    assert_eq!(status, Ok(0));
    assert!(printed.starts_with("Judged documents: 1\n\n"), "{printed}");
    assert!(
        printed.ends_with("Documents where the checkers differ on numbers or quotations: 0\n"),
        "{printed}"
    );
    assert_eq!(
        calls.lock().unwrap()[0][..3],
        ["/bin/lc", "preserve", "--loose"]
    );
}

#[test]
fn build_cases_writes_the_committed_fact_cases() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("fact-cases.jsonl");
    assert_eq!(
        call(
            &["fact-survival", "build-cases", "--output", &arg(&output)],
            &NONE
        ),
        (Ok(0), String::new())
    );
    let committed = fs::read_to_string(evals("fact-cases.jsonl")).unwrap();
    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        py::universal_newlines(&committed)
    );
    let (status, _) = call(
        &[
            "fact-survival",
            "build-cases",
            "--output",
            &arg(&dir.path().join("none/x")),
        ],
        &NONE,
    );
    assert!(status.is_err());
}

fn first_case() -> String {
    let cases = fact_survival::read_jsonl(&evals("fact-cases.jsonl")).unwrap();
    cases[0]["id"].as_str().unwrap().to_owned()
}

#[test]
fn judge_resumes_and_prints_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let (responses, output) = (
        dir.path().join("responses.jsonl"),
        dir.path().join("scores.jsonl"),
    );
    let case = first_case();
    write_rows(
        &responses,
        &[
            json!({"runner": "a", "case_id": case, "trial": 1, "response": "x"}),
            json!({"runner": "b", "case_id": case, "trial": 1, "response": "y"}),
        ],
    );
    // A finished row ends its line, as the command writes it.
    let finished = json!({"setup": "a", "case_id": case, "trial": 1, "cost": 0.5});
    fs::write(&output, format!("{finished}\n")).unwrap();
    let timeouts = Mutex::new(Vec::new());
    let judge = |_: &str, timeout: Duration| {
        timeouts.lock().unwrap().push(timeout);
        Ok((Value::from(r#"{"items": [], "invented": []}"#), 0.125))
    };
    let tools = Tools {
        judge: &judge,
        ..NONE
    };
    let args = [
        "fact-survival",
        "judge",
        &arg(&responses),
        "--output",
        &arg(&output),
        "--call-timeout",
        "7.5",
    ];
    assert_eq!(call(&args, &tools), (Ok(0), "judge cost: $0.12\n".into()));
    assert_eq!(*timeouts.lock().unwrap(), [Duration::from_millis(7500)]);
    assert!(fails_on_a_closed_output(&args, &tools));
    let (status, _) = call(
        &[
            "fact-survival",
            "judge",
            "r",
            "--output",
            "o",
            "--call-timeout=-1",
        ],
        &NONE,
    );
    assert!(matches!(status, Err(Stop::Usage(_))));
    let (status, _) = call(
        &[
            "fact-survival",
            "judge",
            &arg(&dir.path().join("none")),
            "--output",
            "o",
        ],
        &NONE,
    );
    assert!(matches!(status, Err(Stop::Failed(_))));
}

#[test]
fn calibrate_prints_its_result_and_can_record_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let fixtures = dir.path().join("fixtures.jsonl");
    let case = first_case();
    write_rows(
        &fixtures,
        &[
            json!({"name": "copy", "case_id": case, "rewrite": "@source", "default": "dropped", "expected": {}, "invented": false}),
        ],
    );
    let judge = |_: &str, timeout: Duration| {
        assert_eq!(timeout, Duration::from_secs(300));
        Ok((Value::from(r#"{"items": [], "invented": []}"#), 0.25))
    };
    let tools = Tools {
        judge: &judge,
        ..NONE
    };
    let overhead = dir.path().join("overhead.jsonl");
    let (status, printed) = call(
        &[
            "fact-survival",
            "calibrate",
            "--fixtures",
            &arg(&fixtures),
            "--overhead",
            &arg(&overhead),
        ],
        &tools,
    );
    assert_eq!(status, Ok(0));
    assert!(printed.starts_with("{\n  \"compared\": "), "{printed}");
    assert!(printed.ends_with("  \"cost_usd\": 0.25\n}\n"), "{printed}");
    assert_eq!(
        fs::read_to_string(&overhead).unwrap(),
        "{\"stage\": \"calibrate\", \"cost_usd\": 0.25}\n"
    );
    let (status, _) = call(
        &["fact-survival", "calibrate", "--fixtures", &arg(&fixtures)],
        &tools,
    );
    assert_eq!(status, Ok(0));
    let jammed = dir.path().join("jammed");
    fs::create_dir(&jammed).unwrap();
    let (status, _) = call(
        &[
            "fact-survival",
            "calibrate",
            "--fixtures",
            &arg(&fixtures),
            "--overhead",
            &arg(&jammed),
        ],
        &tools,
    );
    assert!(status.is_err());
}

#[test]
fn spend_exits_1_once_the_cap_is_reached() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("scores.jsonl"), "{\"cost\": 2.5}\n").unwrap();
    let folder = arg(dir.path());
    assert_eq!(
        call(&["fact-survival", "spend", &folder, "--cap", "3"], &NONE),
        (Ok(0), "reported cost: $2.50 of $3.00\n".into())
    );
    assert_eq!(
        call(&["fact-survival", "spend", &folder, "--cap", "2.5"], &NONE),
        (Ok(1), "reported cost: $2.50 of $2.50\n".into())
    );
    assert!(fails_on_a_closed_output(
        &["fact-survival", "spend", &folder, "--cap", "3"],
        &NONE
    ));
    fs::write(dir.path().join("scores.jsonl"), "[]\n").unwrap();
    assert!(
        call(&["fact-survival", "spend", &folder, "--cap", "3"], &NONE)
            .0
            .is_err()
    );
}

#[test]
fn report_summarises_every_planned_setup() {
    let dir = tempfile::tempdir().unwrap();
    let (cases, scores, responses) = (
        dir.path().join("cases.jsonl"),
        dir.path().join("scores.jsonl"),
        dir.path().join("responses.jsonl"),
    );
    write_rows(&cases, &[json!({"id": "c", "prompt": "Do.\n\none two"})]);
    let marks = json!([{"kind": "fact", "verdict": "kept", "unverified_quote": false}]);
    let setups = fact_survival::SETUPS;
    write_rows(
        &scores,
        &setups.map(
            |s| json!({"setup": s, "case_id": "c", "trial": 1, "verdicts": marks, "invented": []}),
        ),
    );
    write_rows(
        &responses,
        &setups.map(|s| json!({"runner": s, "case_id": "c", "trial": 1, "response": "one"})),
    );
    let checker = |command: &[String], _: &Input| {
        assert_eq!(command[..2], ["laconic-check", "preserve"]);
        done(0, "added number: 3\n", "")
    };
    let tools = Tools {
        checker: &checker,
        ..NONE
    };
    let args = [
        "fact-survival",
        "report",
        &arg(&scores),
        &arg(&responses),
        "--cases",
        &arg(&cases),
        "--reference",
        "laconic-prose",
        "--trials",
        "1",
    ];
    let (status, printed) = call(&args, &tools);
    assert_eq!(status, Ok(0));
    let summary = crate::cases::parse(&printed).unwrap();
    assert_eq!(summary.as_object().unwrap().len(), 8);
    assert_eq!(summary["no-skill"]["added_per_text"], 1);
    let (status, _) = call(
        &[
            "fact-survival",
            "report",
            &arg(&scores),
            &arg(&responses),
            "--cases",
            &arg(&cases),
            "--reference",
            "x",
        ],
        &tools,
    );
    assert!(status.is_err());
}

#[test]
fn fact_setups_writes_runners_and_preflight_checks_them() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path().join("facts");
    for inner in [
        "humanizer",
        "avoid/plugins/avoid-ai-writing",
        "stop-slop",
        "caveman/plugins/caveman/skills/caveman",
    ] {
        fs::create_dir_all(here.join("src").join(inner)).unwrap();
    }
    let (status, printed) = call(&["fact-setups", &arg(&here)], &NONE);
    assert_eq!(status, Ok(0));
    assert!(
        printed.starts_with("laconic-prose\n") && printed.ends_with("no-skill\n"),
        "{printed}"
    );
    fs::write(here.join("runners.json"), "{}").unwrap();
    assert_eq!(
        call(&["fact-setups", "--preflight", &arg(&here)], &NONE),
        (Ok(0), String::new())
    );
    // An empty folder is refused as a bad argument.
    assert!(Cli::try_parse_from(["laconic-evals", "fact-setups", ""]).is_err());
}

#[test]
fn humanise_metrics_measures_responses_to_known_cases() {
    let dir = tempfile::tempdir().unwrap();
    let responses = dir.path().join("responses.jsonl");
    write_rows(
        &responses,
        &[json!({"case_id": "unknown", "condition": "a", "response": "x"})],
    );
    assert_eq!(
        call(&["humanise-metrics", &arg(&responses), "--dslop"], &NONE),
        (Ok(0), "{}\n".into())
    );
    let (status, _) = call(&["humanise-metrics", &arg(&dir.path().join("none"))], &NONE);
    assert!(status.is_err());
}

#[test]
fn agent_bench_resumes_finished_runs() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    let rows: Vec<Value> = agent_bench::TASKS
        .iter()
        .flat_map(|(task, _)| {
            ["A", "B"].map(|setup| json!({"task": task, "setup": setup, "trial": 1}))
        })
        .collect();
    write_rows(&out, &rows);
    assert_eq!(
        call(&["agent-bench", "a.md", "b.md", "1", &arg(&out)], &NONE),
        (Ok(0), String::new())
    );
    let (status, _) = call(&["agent-bench", "a.md"], &NONE);
    assert_eq!(status, Err(Stop::Failed(agent_bench::USAGE.into())));
    let nothing = Tools {
        which: &|_| None,
        ..NONE
    };
    assert!(call(&["agent-bench"], &nothing).0.is_err());
}

#[test]
fn agent_bench_runs_and_records_an_unfinished_session() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    let (last, _) = agent_bench::TASKS.last().unwrap();
    // Every run but the last task's under B is finished, each row ending its line.
    let rows: String = agent_bench::TASKS
        .iter()
        .flat_map(|(task, _)| {
            ["A", "B"].map(|setup| json!({"task": task, "setup": setup, "trial": 1}))
        })
        .filter(|row| !(row["task"] == last.as_str() && row["setup"] == "B"))
        .map(|row| format!("{row}\n"))
        .collect();
    fs::write(&out, rows).unwrap();
    let commands = Mutex::new(Vec::new());
    let run_in = |command: &[String], _: &Input, _: &Path, _: Duration| {
        commands.lock().unwrap().push(command[0].clone());
        if command[0].ends_with("claude") {
            done(0, r#"{"num_turns": 3}"#, "")
        } else {
            done(1, "", "")
        }
    };
    let tools = Tools {
        run_in: &run_in,
        ..NONE
    };
    let (status, printed) = call(&["agent-bench", "a.md", "b.md", "1", &arg(&out)], &tools);
    assert_eq!(status, Ok(0));
    assert_eq!(
        *commands.lock().unwrap(),
        ["/bin/python3", "/bin/claude", "/bin/python3"]
    );
    let row: Value = serde_json::from_str(&printed).unwrap();
    assert_eq!(
        [&row["task"], &row["setup"], &row["turns"], &row["passed"]],
        [&json!(last), &json!("B"), &json!(3), &json!(false)]
    );
    assert_eq!(fact_survival::read_jsonl(&out).unwrap().last(), Some(&row));
}

#[test]
fn the_stand_ins_refuse_every_call() {
    let refused =
        |call: &dyn Fn()| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).is_err();
    assert!(refused(&|| drop(no_judge("p", Duration::ZERO))));
    assert!(refused(&|| drop(no_exec(&[], &Input::Null))));
    assert!(refused(&|| drop(no_get("u"))));
    assert!(refused(&|| drop(no_git(&[]))));
    assert!(refused(&|| drop(no_claude(
        &[],
        &Input::Null,
        Duration::ZERO
    ))));
    assert!(refused(&|| drop(no_run(
        &[],
        &Input::Null,
        Path::new("."),
        Duration::ZERO
    ))));
    assert!(refused(&|| drop(no_runner(&[], &Input::Null, None, None))));
    assert!(refused(&|| no_sleep(Duration::ZERO)));
    assert!(refused(&|| drop(no_today())));
    assert!(refused(&|| drop(no_session())));
}

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
