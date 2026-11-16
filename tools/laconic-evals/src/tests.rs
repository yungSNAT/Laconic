// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::proc::{Failure, Input, Output};
use crate::testing::{done, write_rows};
use serde_json::{Value, json};
use std::fs;
use std::sync::Mutex;

mod scripts;
mod survival;

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
