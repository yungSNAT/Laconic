// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::cases::{read_jsonl, root};
use crate::judge::{
    assign_labels, build_judge_prompt, grader_rubric, group_responses, parse_judge_scores,
};
use crate::proc::Output;
use crate::run_evals::summarize_scores;
use crate::testing::{Runs, done};

const SESSION: &str = "4f2c1e9a-0000-4000-8000-000000000001";

fn scenario() -> PathBuf {
    root().join("evals/scenarios/persistence-topic-switch-stop")
}

fn session() -> String {
    SESSION.into()
}

fn args(dir: &Path) -> ScenarioArgs {
    ScenarioArgs {
        scenario: scenario(),
        condition: "candidate".into(),
        condition_skill: Some(root().join("skills/laconic/SKILL.md")),
        model: "fixture".into(),
        trial: 1,
        budget_usd: 1.0,
        output: dir.join("response.jsonl"),
    }
}

fn capture(args: &ScenarioArgs, exec: Exec) -> Result<Value, String> {
    let mut out = Vec::new();
    run_scenario(args, exec, &session, CALL_TIMEOUT, &mut out)?;
    let printed = String::from_utf8(out).unwrap();
    assert!(printed.ends_with(". Not graded.\n"), "{printed}");
    Ok(read_jsonl(&args.output).unwrap().remove(0))
}

fn fails<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.unwrap_err();
    assert!(error.contains(expected), "{error:?} lacks {expected:?}");
}

fn ok(cost: f64) -> Result<Output, Failure> {
    done(
        0,
        &json!({"result": "ok", "total_cost_usd": cost}).to_string(),
        "",
    )
}

fn write_case(dir: &Path, case: &Value) {
    fs::write(dir.join("case.jsonl"), format!("{case}\n")).unwrap();
}

#[test]
fn invalid_turns_are_rejected() {
    let original = load_scenario(&scenario()).unwrap();
    assert_eq!(original["turns"].as_array().unwrap().len(), 6);
    let dir = tempfile::tempdir().unwrap();
    let turn = |id: &str, prompt: &str| json!({"id": id, "prompt": prompt});
    for (turns, expected) in [
        (json!([]), "Scenario needs at least two turns"),
        (json!("ab"), "Scenario needs at least two turns"),
        (
            json!([null, null]),
            "Each turn needs a non-empty id and prompt",
        ),
        (
            json!([turn("a", " "), turn("b", "x")]),
            "Each turn needs a non-empty id and prompt",
        ),
        (
            json!([turn("a", "task"), turn("a", "task")]),
            "Duplicate turn id: a",
        ),
    ] {
        let mut case = original.clone();
        case["turns"] = turns;
        write_case(dir.path(), &case);
        assert_eq!(load_scenario(dir.path()), Err(expected.into()));
    }
    let mut case = original.clone();
    case.as_object_mut().unwrap().remove("turns");
    write_case(dir.path(), &case);
    fails(load_scenario(dir.path()), "at least two turns");
    fs::write(dir.path().join("case.jsonl"), "{}\n").unwrap();
    fails(
        load_scenario(dir.path()),
        "Expected one valid scenario case: Case 1: missing fields",
    );
    let two = format!("{original}\n{original}\n");
    fs::write(dir.path().join("case.jsonl"), two).unwrap();
    fails(
        load_scenario(dir.path()),
        "Expected one valid scenario case: ",
    );
}

#[test]
fn models_are_ids_never_options() {
    for model in [
        "",
        "-",
        "--help",
        "--tools=Bash",
        "-pwned",
        "opus --tools Bash",
        "opus\n",
        "é",
    ] {
        assert!(!valid_model(model), "{model:?}");
    }
    for model in [
        "sonnet",
        "claude-opus-4-1[1m]",
        "claude-opus-4-5@20251101",
        "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
        "vendor/model_2",
    ] {
        assert!(valid_model(model), "{model}");
    }
}

#[test]
fn a_frontmatter_only_candidate_skill_is_rejected_before_capture() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("empty-skill.md");
    fs::write(&skill, "---\nname: empty\n---\n").unwrap();
    let mut args = args(dir.path());
    args.condition_skill = Some(skill);
    let runs = Runs::new(|_, _| ok(0.0));
    fails(
        capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
        "Candidate skill cannot be empty",
    );
    assert!(!args.output.exists());
    assert!(runs.calls().is_empty());
    args.condition_skill = None;
    fails(
        capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)),
        "--condition-skill is required for the candidate condition",
    );
    args.condition_skill = Some(dir.path().join("missing.md"));
    assert!(capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).is_err());
    assert!(runs.calls().is_empty());
}

#[test]
fn capture_is_compatible_with_existing_blind_judge() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = args(dir.path());
    let runs = Runs::new(|_, _| {
        done(
            0,
            r#"{"result": "fixture response", "total_cost_usd": 0.01}"#,
            "",
        )
    });
    let exec =
        |c: &[String], i: &Input, d: Option<&Path>, t: Option<Duration>| runs.exec(c, i, d, t);
    let candidate = capture(&args, &exec).unwrap();
    let calls = runs.calls();
    assert_eq!(calls.len(), 6);
    let first = calls[0].input.as_deref().unwrap();
    assert!(first.contains("<response_style>"));
    assert!(!first.contains("disable-model-invocation:"));
    let at = |command: &[String], flag: &str| {
        command[command.iter().position(|w| w == flag).unwrap() + 1].clone()
    };
    assert_eq!(at(&calls[0].command, "--session-id"), SESSION);
    for call in &calls[1..] {
        assert_eq!(at(&call.command, "--resume"), SESSION);
        assert!(!call.input.as_deref().unwrap().contains("<response_style>"));
    }
    for call in &calls {
        assert_eq!(call.cwd, calls[0].cwd);
        assert!(!call.cwd.as_deref().unwrap().starts_with(root()));
        assert!(!call.null);
        assert_eq!(call.timeout, Some(CALL_TIMEOUT));
        assert!(call.command.contains(&"--safe-mode".to_owned()));
        assert!(call.command.contains(&"--strict-mcp-config".to_owned()));
        assert_eq!(at(&call.command, "--tools"), "");
    }
    assert_eq!(calls[5].command.last().unwrap(), "0.950000");
    assert!((candidate["cost_usd"].as_f64().unwrap() - 0.06).abs() < 1e-9);
    assert_eq!(candidate["runner"], "claude:fixture");
    assert_eq!(candidate["session_id"], SESSION);

    args.condition = "baseline".into();
    args.output = dir.path().join("baseline.jsonl");
    let runs = Runs::new(|_, _| {
        done(
            0,
            r#"{"result": "fixture response", "total_cost_usd": 0.01}"#,
            "",
        )
    });
    let baseline = capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)).unwrap();
    assert!(
        !runs.calls()[0]
            .input
            .as_deref()
            .unwrap()
            .contains("response_style")
    );
    assert_eq!(baseline["response"], candidate["response"]);
    let groups = group_responses(&[baseline, candidate]).unwrap();
    assert_eq!(groups.len(), 1);
    let group = &groups[0];
    let labels = assign_labels(&group.case_id, &group.trial, &["baseline", "candidate"]);
    let rubric = grader_rubric(&read_text(&root().join("evals/rubric.md")).unwrap());
    let prompt = build_judge_prompt(
        &load_scenario(&scenario()).unwrap(),
        &group.responses,
        &labels,
        &rubric,
        &[],
    )
    .unwrap();
    for hidden in ["baseline", "candidate", SESSION, "response_style"] {
        assert!(!prompt.contains(hidden), "{hidden}");
    }
    let mut verdict = serde_json::Map::new();
    for (_, label) in &labels {
        verdict.insert(
            label.clone(),
            json!({"correctness": 4, "autonomy": 4,
            "actionability": 4, "safety": 4, "concision": 4, "blocker": false,
            "notes": "fixture only"}),
        );
    }
    let scores = parse_judge_scores(
        &Value::Object(verdict).to_string(),
        &group.case_id,
        &group.trial,
        &labels,
    )
    .unwrap();
    assert_eq!(scores.len(), 2);
    let summary = summarize_scores(&scores, "baseline").unwrap();
    assert_eq!(summary["release_gate"]["passed"], false);
}

#[test]
fn failed_capture_cannot_be_judged_as_complete() {
    for (index, (payload, code, message)) in [
        (
            json!({"total_cost_usd": 0.12}),
            0,
            "Claude failed: missing response; cumulative reported cost: $0.130000",
        ),
        (
            json!({"result": "error", "total_cost_usd": 0.12}),
            1,
            "Claude failed: error; cumulative reported cost: $0.130000",
        ),
        (
            json!({"is_error": true, "result": "error", "total_cost_usd": 0.12}),
            0,
            "0.130000",
        ),
        (
            json!({"subtype": "error_max_turns", "result": "x", "total_cost_usd": 0.12}),
            0,
            "0.130000",
        ),
        (
            json!({"result": "ok", "total_cost_usd": 2}),
            0,
            "Budget exceeded; reported cost: $2.010000",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        let mut args = args(dir.path());
        args.output = dir.path().join(format!("failed-{index}.jsonl"));
        let runs = Runs::new(|call, _| match call {
            0 => ok(0.01),
            _ => done(code, &payload.to_string(), ""),
        });
        fails(capture(&args, &|c, i, d, t| runs.exec(c, i, d, t)), message);
        assert_eq!(fs::read_to_string(&args.output).unwrap(), "");
    }
}

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
