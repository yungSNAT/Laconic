// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

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
fn a_closed_output_stops_the_run_and_drops_the_calls_still_running() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| reply(0.25));
    let world = world!(runs, &no_pause);
    let mut args = catalogue_args(
        dir.path(),
        &["direct-answer", "debugging-cause"],
        "claude-json",
        &["stub"],
    );
    args.budget_usd = 10.0;
    args.jobs = 2;
    let mut closed: &mut [u8] = &mut [];
    fails(
        run_evaluations(&args, &world, &mut closed),
        "failed to write whole buffer",
    );
    assert_eq!(runs.calls().len(), 2);
    // The first response is saved before its line fails to print; the second is not.
    assert_eq!(rows(&args.output).len(), 1);
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
