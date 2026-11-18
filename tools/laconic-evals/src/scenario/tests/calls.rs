// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn invalid_costs_and_payloads_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut payloads: Vec<String> = ["[]", "null", "\"error\"", "{}"].map(str::to_owned).into();
    for cost in ["true", "false", "NaN", "Infinity", "-1", "null", "\"0.1\""] {
        payloads.push(format!(r#"{{"result": "ok", "total_cost_usd": {cost}}}"#));
    }
    for payload in payloads {
        let runs = Runs::new(|_, _| done(0, &payload, ""));
        let exec =
            |c: &[String], i: &Input, d: Option<&Path>, t: Option<Duration>| runs.exec(c, i, d, t);
        fails(
            call_claude(
                &exec,
                &["claude".to_owned()],
                dir.path(),
                0.1,
                None,
                CALL_TIMEOUT,
            ),
            "Claude returned invalid JSON or cost (",
        );
        assert!(runs.calls()[0].null);
    }
    let runs = Runs::new(|_, _| {
        done(
            0,
            "\u{feff}{\"result\": \" ok \", \"total_cost_usd\": 0}",
            "",
        )
    });
    let exec =
        |c: &[String], i: &Input, d: Option<&Path>, t: Option<Duration>| runs.exec(c, i, d, t);
    assert_eq!(
        call_claude(
            &exec,
            &["claude".to_owned()],
            dir.path(),
            0.1,
            None,
            CALL_TIMEOUT
        ),
        Ok(("ok".to_owned(), 0.0))
    );
    let bad = |_: &[String],
               _: &Input,
               _: Option<&Path>,
               _: Option<Duration>|
     -> Result<Output, Failure> {
        Ok(Output {
            code: 0,
            stdout: vec![0xff],
            stderr: Vec::new(),
        })
    };
    fails(
        call_claude(
            &bad,
            &["claude".to_owned()],
            dir.path(),
            0.1,
            None,
            CALL_TIMEOUT,
        ),
        "invalid utf-8",
    );
    let missing = |_: &[String],
                   _: &Input,
                   _: Option<&Path>,
                   _: Option<Duration>|
     -> Result<Output, Failure> { Err(Failure::Spawn("not found".into())) };
    assert_eq!(
        call_claude(
            &missing,
            &["claude".to_owned()],
            dir.path(),
            0.1,
            None,
            CALL_TIMEOUT
        ),
        Err("cannot run claude: not found".into())
    );
    let hung = |_: &[String],
                _: &Input,
                _: Option<&Path>,
                _: Option<Duration>|
     -> Result<Output, Failure> { Err(Failure::Timeout) };
    assert_eq!(
        call_claude(
            &hung,
            &["claude".to_owned()],
            dir.path(),
            0.1,
            None,
            CALL_TIMEOUT
        ),
        Err("Claude timed out; call cost unavailable; reported so far: $0.100000".into())
    );
}

#[test]
fn invalid_inputs_and_existing_output_do_not_start_provider() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::new(|_, _| ok(0.0));
    let mut args = args(dir.path());
    for budget in [0.0, -1.0, 26.0, f64::NAN, f64::INFINITY] {
        args.budget_usd = budget;
        fails(
            capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
            "Budget must be in (0, 25]; trial must be positive",
        );
    }
    args.budget_usd = 1.0;
    args.trial = 0;
    fails(
        capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
        "trial must be positive",
    );
    args.trial = 1;
    fs::write(&args.output, "existing results").unwrap();
    assert!(capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).is_err());
    assert_eq!(
        fs::read_to_string(&args.output).unwrap(),
        "existing results"
    );
    fs::write(dir.path().join("file"), "").unwrap();
    args.output = dir.path().join("file/response.jsonl");
    assert!(capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).is_err());
    // An output with no folder part names no file.
    args.output = PathBuf::new();
    assert!(capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).is_err());
    assert!(runs.calls().is_empty());
}

#[test]
fn model_cannot_become_another_cli_option() {
    for (index, model) in [
        "",
        "-",
        "--help",
        "--tools=Bash",
        "-pwned",
        "opus --tools Bash",
        "opus\n",
    ]
    .into_iter()
    .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        let runs = Runs::new(|_, _| ok(0.01));
        let mut args = args(dir.path());
        args.model = model.into();
        args.output = dir.path().join(format!("invalid-{index}.jsonl"));
        fails(
            capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
            "Model must",
        );
        assert!(runs.calls().is_empty());
        assert!(!args.output.exists());
    }
    for model in ["sonnet", "claude-opus-4-1[1m]", "claude-opus-4-5@20251101"] {
        let dir = tempfile::tempdir().unwrap();
        let runs = Runs::new(|_, _| ok(0.01));
        let mut args = args(dir.path());
        args.model = model.into();
        let row = capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).unwrap();
        assert_eq!(row["runner"], format!("claude:{model}"));
        for call in runs.calls() {
            assert!(call.command.contains(&format!("--model={model}")));
            assert!(!call.command.contains(&"--model".to_owned()));
        }
    }
}

#[test]
fn remaining_budget_is_rounded_down_and_stops_further_calls() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = args(dir.path());
    args.budget_usd = 0.123_456_9;
    let runs = Runs::new(|_, _| ok(0.123_456));
    fails(
        capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
        "Budget exhausted; reported cost: $0.123456",
    );
    let calls = runs.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].command.last().unwrap(), "0.123456");
    assert!(read_jsonl(&args.output).unwrap().is_empty());
}

#[test]
fn a_closed_output_fails_after_the_conversation_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = args(dir.path());
    args.output = dir.path().join("new/response.jsonl");
    let runs = Runs::new(|_, _| ok(0.1));
    let mut closed: &mut [u8] = &mut [];
    fails(
        run_scenario(
            &args,
            &|c, i, d, t| runs.exec(c, i, d, t),
            &session,
            CALL_TIMEOUT,
            &mut closed,
        ),
        "failed to write whole buffer",
    );
    assert_eq!(read_jsonl(&args.output).unwrap().len(), 1);
}

#[test]
fn prompts_reach_the_child_as_data_with_their_bytes_intact() {
    // A parser exit must never substitute for a completed evaluation turn: every prompt goes on
    // standard input, never into the command line.
    let original = load_scenario(&scenario()).unwrap();
    for condition in ["baseline", "candidate"] {
        let dir = tempfile::tempdir().unwrap();
        let prompts = [
            "--version",
            "--help",
            "-h",
            "--tools=Bash",
            "mcp",
            "auth status",
            "$(echo literal); & echo literal",
            "  Croft Manor: caf\u{e9} \u{6f22}\u{5b57} \u{1f3f9}\nLF\r\nCRLF\rCR\n  ",
        ];
        let turns: Vec<Value> = prompts
            .iter()
            .enumerate()
            .map(|(i, p)| json!({"id": i.to_string(), "prompt": p}))
            .collect();
        let mut case = original.clone();
        case["turns"] = Value::from(turns);
        write_case(dir.path(), &case);
        let mut args = args(dir.path());
        args.scenario = dir.path().to_path_buf();
        args.condition = condition.into();
        let runs = Runs::new(|_, call: &crate::testing::Run| {
            let reply = json!({"result": call.input.clone().unwrap(), "total_cost_usd": 0.01});
            done(0, &reply.to_string(), "")
        });
        let row = capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).unwrap();
        let transcript: Vec<Value> =
            serde_json::from_str(row["response"].as_str().unwrap()).unwrap();
        assert_eq!(transcript.len(), prompts.len());
        let calls = runs.calls();
        for (index, (turn, call)) in transcript.iter().zip(&calls).enumerate() {
            let expected = if index == 0 {
                let skill = args.condition_skill.as_deref();
                condition_prompt(prompts[0], condition, skill, true).unwrap()
            } else {
                prompts[index].to_owned()
            };
            assert_eq!(call.input.as_deref(), Some(expected.as_str()));
            assert_eq!(turn["response"], py::strip(&expected));
            assert_eq!(turn["prompt"], prompts[index]);
            assert!(!call.command.contains(&expected));
        }
        assert!((row["cost_usd"].as_f64().unwrap() - 0.08).abs() < 1e-9);
    }
}

#[test]
fn utf8_stderr_is_readable_when_a_child_fails() {
    let dir = tempfile::tempdir().unwrap();
    for (stdout, code, cost_message) in [
        ("not json", 0, "call cost unavailable"),
        (
            r#"{"result":"error","total_cost_usd":0.02}"#,
            1,
            "cumulative reported cost: $0.120000",
        ),
    ] {
        let runs = Runs::new(|_, _| {
            let mut reply = done(code, stdout, "caf\u{e9} failed").unwrap();
            reply.stderr.push(0xff);
            Ok(reply)
        });
        let exec =
            |c: &[String], i: &Input, d: Option<&Path>, t: Option<Duration>| runs.exec(c, i, d, t);
        let error = call_claude(
            &exec,
            &["claude".to_owned()],
            dir.path(),
            0.1,
            None,
            CALL_TIMEOUT,
        )
        .unwrap_err();
        assert!(error.contains("caf\u{e9} failed\\xff"), "{error}");
        assert!(error.contains(cost_message), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn an_actual_child_gets_no_inherited_input_and_times_out() {
    let dir = tempfile::tempdir().unwrap();
    let sh = |script: &str| ["sh", "-c", script].map(str::to_owned);
    let script = r#"printf '{"result": "[%s]", "total_cost_usd": 0}' "$(cat)""#;
    assert_eq!(
        call_claude(
            &crate::proc::run,
            &sh(script),
            dir.path(),
            0.0,
            None,
            CALL_TIMEOUT
        ),
        Ok(("[]".to_owned(), 0.0))
    );
    assert_eq!(
        call_claude(
            &crate::proc::run,
            &sh(script),
            dir.path(),
            0.0,
            Some("given"),
            CALL_TIMEOUT
        ),
        Ok(("[given]".to_owned(), 0.0))
    );
    let error = call_claude(
        &crate::proc::run,
        &sh("sleep 5"),
        dir.path(),
        0.1,
        None,
        Duration::from_millis(100),
    )
    .unwrap_err();
    assert!(
        error.contains("timed out") && error.contains("0.100000"),
        "{error}"
    );
}
