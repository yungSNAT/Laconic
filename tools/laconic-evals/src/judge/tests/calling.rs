// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

#[test]
fn the_caller_retries_runner_failures_and_bad_verdicts() {
    let runs = Runs::new(|index, _| match index {
        0 => done(1, "partial", ""),
        1 => done(0, r#"{"result": "not a verdict"}"#, ""),
        _ => claude_verdict(&["A"]),
    });
    let paused = Mutex::new(Vec::new());
    let sleep = |d: Duration| paused.lock().unwrap().push(d);
    let stub = runner(&["judge-cli", "--tools", ""], "claude-json");
    let caller = Caller {
        exec: &|c, i, d, t| runs.exec(c, i, d, t),
        sleep: &sleep,
        runner: &stub,
        retries: 1,
        call_timeout: 2.5,
    };
    let one = labels(&[("candidate", "A")]);
    let (rows, cost) = caller
        .judge("PROMPT", &json!("c"), &json!(1), &one)
        .unwrap();
    assert_eq!((rows.len(), cost), (1, json!(0.25)));
    let calls = runs.calls();
    assert_eq!(calls.len(), 3);
    for call in &calls {
        // The prompt goes on standard input, never after a trailing option such as --tools "".
        assert_eq!(call.command, ["judge-cli", "--tools", ""]);
        assert_eq!(call.input.as_deref(), Some("PROMPT"));
        assert_eq!(call.timeout, Some(Duration::from_millis(2500)));
    }
    assert_ne!(calls[0].cwd, calls[1].cwd);
    assert_eq!(*paused.lock().unwrap(), [Duration::from_secs(1); 2]);
}

#[test]
fn the_caller_reports_what_retries_cannot_fix() {
    let stub = runner(&["judge-cli"], "claude-json");
    let one = labels(&[("candidate", "A")]);
    let caller = |exec: Exec<'static>, retries| Caller {
        exec,
        sleep: &no_pause,
        runner: &stub,
        retries,
        call_timeout: 1.0,
    };
    fn failing(
        _: &[String],
        _: &Input,
        _: Option<&Path>,
        _: Option<Duration>,
    ) -> Result<Output, Failure> {
        done(3, "out", "")
    }
    fn missing(
        _: &[String],
        _: &Input,
        _: Option<&Path>,
        _: Option<Duration>,
    ) -> Result<Output, Failure> {
        Err(Failure::Spawn("gone".into()))
    }
    fn unreadable(
        _: &[String],
        _: &Input,
        _: Option<&Path>,
        _: Option<Duration>,
    ) -> Result<Output, Failure> {
        done(0, "[]", "")
    }
    fn unindexable(
        _: &[String],
        _: &Input,
        _: Option<&Path>,
        _: Option<Duration>,
    ) -> Result<Output, Failure> {
        claude_verdict(&[]).map(|_| Output {
            code: 0,
            stdout: br#"{"result": "[\"A\"]"}"#.to_vec(),
            stderr: Vec::new(),
        })
    }
    fn wrong(
        _: &[String],
        _: &Input,
        _: Option<&Path>,
        _: Option<Duration>,
    ) -> Result<Output, Failure> {
        done(0, r#"{"result": "{}"}"#, "")
    }
    let judge = |exec, retries| caller(exec, retries).judge("P", &json!("c"), &json!(1), &one);
    assert_eq!(
        judge(&failing, 1),
        Err(JudgeError::Skip(
            "Judge runner failed after 2 attempts:\nout".into()
        ))
    );
    assert_eq!(
        judge(&missing, 0),
        Err(JudgeError::Fatal("cannot run judge-cli: gone".into()))
    );
    assert_eq!(
        judge(&unreadable, 0),
        Err(JudgeError::Fatal(
            "'list' object has no attribute 'get'".into()
        ))
    );
    assert_eq!(
        judge(&unindexable, 0),
        Err(JudgeError::Fatal(
            "list indices must be integers or slices, not str".into()
        ))
    );
    assert_eq!(
        judge(&wrong, 0),
        Err(JudgeError::Skip(
            "c/trial 1/label A: judge returned no verdict for label A".into()
        ))
    );
    assert_eq!(
        judge(&wrong, -1),
        Err(JudgeError::Fatal("--retries must not be negative".into()))
    );
    assert_eq!(
        caller(&wrong, -1).invoke("P"),
        Err(JudgeError::Fatal("--retries must not be negative".into()))
    );
}

#[cfg(unix)]
#[test]
fn judging_produces_paired_rows_the_scorer_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let payload = dir.path().join("verdict.json");
    fs::write(
        &payload,
        json!({"A": fixture_verdict(), "B": fixture_verdict()}).to_string(),
    )
    .unwrap();
    let captured = dir.path().join("prompt.txt");
    // Reads the prompt from stdin, not argv: a trailing option such as `--tools ""` otherwise
    // swallows a prompt appended to the command line.
    let script = format!("cat > {}; cat {}", captured.display(), payload.display());
    let fixture = Fixture::new(&["sh", "-c", &script], "text");
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "runner": "claude",
            "response": "Great question! The answer is 102."}),
        json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate", "runner": "claude",
            "response": "102"}),
    ];
    let lines: Vec<String> = rows.iter().map(|r| format!("{r}\n")).collect();
    fs::write(fixture.args.responses.as_ref().unwrap(), lines.concat()).unwrap();
    let (status, printed) = fixture.run(&crate::proc::run, &no_pause);
    assert_eq!(status, Ok(0));
    assert_eq!(printed, "judged direct-answer/trial 1\n");
    let scored = fixture.rows();
    let conditions: BTreeSet<&str> = scored
        .iter()
        .map(|r| r["condition"].as_str().unwrap())
        .collect();
    assert_eq!(conditions, BTreeSet::from(["baseline", "candidate"]));
    // The scorer is the real consumer: it must accept what the judge writes.
    let summary = crate::run_evals::summarize_scores(&scored, "baseline").unwrap();
    assert_eq!(summary["conditions"]["baseline"]["rows"], 1);
    // The prompt actually sent must carry the responses but never the conditions.
    let prompt = fs::read_to_string(&captured).unwrap();
    assert!(prompt.contains("Great question! The answer is 102."));
    assert!(!prompt.to_lowercase().contains("baseline"));
    assert!(!prompt.to_lowercase().contains("candidate"));
}

#[test]
fn a_malformed_verdict_or_failed_runner_skips_its_group_instead_of_killing_the_run() {
    // One bad grader response must not discard the groups already judged nor the ones still
    // queued behind it.
    let good = json!({"A": fixture_verdict(), "B": fixture_verdict()}).to_string();
    for (bad, failed) in [
        (
            Some(r#"{"A": {"correctness": 3}, "B": {"correctness": 3}}"#),
            None,
        ),
        (None, Some(7)),
    ] {
        let mut fixture = Fixture::new(&["stub"], "text");
        fixture.args.retries = 0;
        fixture.args.cases = fixture.dir.path().join("nested/cases.jsonl");
        fs::create_dir_all(fixture.dir.path().join("nested")).unwrap();
        fs::copy(root().join("evals/cases.jsonl"), &fixture.args.cases).unwrap();
        fixture.args.output = Some(fixture.dir.path().join("out/scores.jsonl"));
        fixture.write_responses(&[
            ("direct-answer", "baseline", 1),
            ("direct-answer", "candidate", 1),
            ("casual-message", "baseline", 1),
            ("casual-message", "candidate", 1),
        ]);
        let runs = Runs::new(|_, call: &Run| {
            let prompt = call.input.as_deref().unwrap();
            match (prompt.contains("casual-message"), failed) {
                (true, Some(code)) => done(code, "", ""),
                (true, None) => done(0, bad.unwrap(), ""),
                (false, _) => done(0, &good, ""),
            }
        });
        let (status, printed) = fixture.run(&|c, i, d, t| runs.exec(c, i, d, t), &no_pause);
        assert_eq!(status, Ok(1), "skipped groups must not report success");
        assert_eq!(printed, "judged direct-answer/trial 1\n");
        let ids: BTreeSet<String> = fixture
            .rows()
            .iter()
            .map(|r| py::key(&r["case_id"]))
            .collect();
        assert_eq!(ids, BTreeSet::from([cases_key("direct-answer")]));
        assert_eq!(fixture.rows().len(), 2);
    }
}

#[test]
fn missing_entire_condition_fails_before_judging() {
    let fixture = Fixture::new(&["stub"], "text");
    fixture.write_responses(&[("direct-answer", "baseline", 1)]);
    let runs = Runs::new(|_, _| done(99, "", ""));
    let (status, _) = fixture.run(&|c, i, d, t| runs.exec(c, i, d, t), &no_pause);
    fails(
        status,
        "Response conditions do not match --conditions: missing required condition(s): candidate",
    );
    assert!(!fixture.output().exists());
    assert!(runs.calls().is_empty());
}

#[test]
fn judge_reports_empty_unexpected_and_unknown_responses() {
    let fixture = Fixture::new(&["stub-runner"], "claude-json");
    let runs = Runs::new(|_, _| done(0, "", ""));
    let exec =
        |c: &[String], i: &Input, d: Option<&Path>, t: Option<Duration>| runs.exec(c, i, d, t);
    for (entries, expected) in [
        (&[][..], "no responses to judge"),
        (
            &[
                ("direct-answer", "baseline", 1),
                ("direct-answer", "candidate", 1),
                ("direct-answer", "comparator", 1),
            ],
            "Response conditions do not match --conditions: unexpected condition(s): comparator",
        ),
        (
            &[("direct-answer", "comparator", 1)],
            "missing required condition(s): baseline, candidate; unexpected condition(s): comparator",
        ),
        (
            &[("unknown", "baseline", 1), ("unknown", "candidate", 1)],
            "unknown: response has no matching case in",
        ),
    ] {
        fixture.write_responses(entries);
        fails(fixture.run(&exec, &no_pause).0, expected);
    }
    assert!(runs.calls().is_empty());
}

#[test]
fn resume_skips_judged_and_incomplete_trials_and_retries_failed_calls() {
    let mut fixture = Fixture::new(&["stub-runner"], "claude-json");
    fixture.args.retries = 1;
    fixture.write_responses(&[
        ("direct-answer", "baseline", 1),
        ("direct-answer", "candidate", 1),
        ("direct-answer", "baseline", 3),
        ("direct-answer", "candidate", 3),
        ("direct-answer", "baseline", 2),
    ]);
    let prior = json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline"});
    fs::write(fixture.output(), format!("{prior}\n")).unwrap();
    let runs = Runs::new(|index, _| match index {
        0 => done(1, "", "unavailable"),
        1 => done(0, r#"{"result": "bad verdict"}"#, ""),
        _ => claude_verdict(&["A", "B"]),
    });
    let paused = Mutex::new(Vec::new());
    let sleep = |d: Duration| paused.lock().unwrap().push(d);
    let (status, printed) = fixture.run(&|c, i, d, t| runs.exec(c, i, d, t), &sleep);
    assert_eq!(status, Ok(0));
    let rows = fixture.rows();
    assert_eq!(rows[0], prior);
    let rest: BTreeSet<(i64, &str)> = rows[1..]
        .iter()
        .map(|r| {
            (
                r["trial"].as_i64().unwrap(),
                r["condition"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(rest, BTreeSet::from([(3, "baseline"), (3, "candidate")]));
    assert_eq!(runs.calls().len(), 3);
    assert_eq!(*paused.lock().unwrap(), [Duration::from_secs(1); 2]);
    assert_eq!(
        printed,
        "skip judged direct-answer/trial 1\njudged direct-answer/trial 3\nReported judge cost: $0.2500\n"
    );
}

#[test]
fn script_calibrates_using_only_the_labelled_fixture() {
    let mut fixture = Fixture::new(&["stub-runner"], "claude-json");
    let labelled = fixture.dir.path().join("labelled.jsonl");
    let row =
        json!({"case_id": "direct-answer", "name": "right", "response": "102", "blocker": false});
    fs::write(&labelled, format!("{row}\n")).unwrap();
    fixture.args.calibrate = Some(labelled);
    let runs = Runs::new(|_, _| claude_verdict(&["A"]));
    let (status, printed) = fixture.run(&|c, i, d, t| runs.exec(c, i, d, t), &no_pause);
    assert_eq!(status, Ok(0));
    assert_eq!(
        serde_json::from_str::<Value>(&printed).unwrap(),
        json!({"compared": 1, "agreement": 1.0, "errors": [], "cost_usd": 0.25})
    );
    assert!(
        runs.calls()[0]
            .input
            .as_deref()
            .unwrap()
            .contains("## Response A\n\n102")
    );
    assert!(!fixture.output().exists());
    let failing = Runs::new(|_, _| done(1, "", "down"));
    fails(
        fixture
            .run(&|c, i, d, t| failing.exec(c, i, d, t), &no_pause)
            .0,
        "Judge runner failed after 3 attempts:\ndown",
    );
}

#[test]
fn the_command_stops_on_what_python_raised() {
    let mut fixture = Fixture::new(&["stub-runner"], "claude-json");
    fixture.args.retries = 0;
    fixture.write_responses(&[
        ("direct-answer", "baseline", 1),
        ("direct-answer", "candidate", 1),
    ]);
    let unreadable = Runs::new(|_, _| done(0, "[]", ""));
    fails(
        fixture
            .run(&|c, i, d, t| unreadable.exec(c, i, d, t), &no_pause)
            .0,
        "'list' object has no attribute 'get'",
    );
    let priced = Runs::new(|_, _| {
        let result = json!({"A": fixture_verdict(), "B": fixture_verdict()}).to_string();
        done(
            0,
            &json!({"result": result, "total_cost_usd": [1]}).to_string(),
            "",
        )
    });
    fails(
        fixture
            .run(&|c, i, d, t| priced.exec(c, i, d, t), &no_pause)
            .0,
        "float() argument",
    );
    let free = Runs::new(|_, _| {
        let result = json!({"A": fixture_verdict(), "B": fixture_verdict()}).to_string();
        done(0, &json!({"result": result}).to_string(), "")
    });
    fs::remove_file(fixture.output()).unwrap();
    assert_eq!(
        fixture.run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause),
        (Ok(0), "judged direct-answer/trial 1\n".to_owned())
    );
    fs::write(fixture.output(), "{}\n").unwrap();
    fails(
        fixture
            .run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause)
            .0,
        "'case_id'",
    );
    fixture.args.output = Some(fixture.dir.path().to_path_buf());
    assert!(
        fixture
            .run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause)
            .0
            .is_err()
    );
    fs::write(fixture.dir.path().join("file"), "").unwrap();
    fixture.args.output = Some(fixture.dir.path().join("file/scores.jsonl"));
    assert!(
        fixture
            .run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause)
            .0
            .is_err()
    );
    fixture.args.output = None;
    fails(
        fixture
            .run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause)
            .0,
        "--responses and --output are required unless --calibrate is given",
    );
    fixture.args.output = Some(PathBuf::from(""));
    assert!(
        fixture
            .run(&|c, i, d, t| free.exec(c, i, d, t), &no_pause)
            .0
            .is_err()
    );
}

#[test]
fn unsortable_groups_are_errors() {
    let fixture = Fixture::new(&["stub-runner"], "claude-json");
    let rows = [
        json!({"case_id": 1, "trial": 1, "condition": "baseline", "response": "x"}),
        json!({"case_id": "a", "trial": 1, "condition": "baseline", "response": "x"}),
    ];
    let lines: Vec<String> = rows.iter().map(|r| format!("{r}\n")).collect();
    fs::write(fixture.args.responses.as_ref().unwrap(), lines.concat()).unwrap();
    let mut only_baseline = fixture;
    only_baseline.args.conditions = vec!["baseline".into()];
    let runs = Runs::new(|_, _| done(0, "", ""));
    fails(
        only_baseline
            .run(&|c, i, d, t| runs.exec(c, i, d, t), &no_pause)
            .0,
        "'<' not supported",
    );
    only_baseline.args.conditions = vec!["baseline".into(), "candidate".into()];
    fs::write(
        only_baseline.args.responses.as_ref().unwrap(),
        format!(
            "{}{}",
            lines.concat(),
            json!({"case_id": "b", "trial": 1, "condition": "candidate", "response": "x"})
        ),
    )
    .unwrap();
    fails(
        only_baseline
            .run(&|c, i, d, t| runs.exec(c, i, d, t), &no_pause)
            .0,
        "'<' not supported",
    );
}
