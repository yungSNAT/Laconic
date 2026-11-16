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
};

/// Runs the command line `args` and returns its status and what it printed.
fn call(args: &[&str], tools: &Tools) -> (Result<u8, Stop>, String) {
    let cli = Cli::try_parse_from(["laconic-evals"].iter().chain(args)).unwrap();
    let mut out = Vec::new();
    let status = run(cli, &mut out, tools);
    (status, String::from_utf8(out).unwrap())
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
    write_rows(
        &output,
        &[json!({"setup": "a", "case_id": case, "trial": 1, "cost": 0.5})],
    );
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
}

#[test]
fn stops_convert_from_messages() {
    assert_eq!(Stop::from("x"), Stop::Failed("x".into()));
    assert_eq!(Stop::from(String::from("y")), Stop::Failed("y".into()));
}
