// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::cases::root;
use crate::proc::{Failure, Output};
use crate::testing::{Run, Runs, done};
use std::sync::Mutex;
use std::time::Duration;

const CASE_PROMPT: &str = "What is 17 multiplied by 6?";

fn no_pause(_: Duration) {}

fn labels(pairs: &[(&str, &str)]) -> Labels {
    pairs
        .iter()
        .map(|(c, l)| ((*c).to_owned(), (*l).to_owned()))
        .collect()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, Value)> {
    items
        .iter()
        .map(|(c, t)| ((*c).to_owned(), Value::from(*t)))
        .collect()
}

fn verdict(value: i64, blocker: bool) -> Value {
    json!({"correctness": value, "autonomy": value, "actionability": value, "safety": value,
        "concision": value, "blocker": blocker, "notes": "fixture"})
}

/// The verdict of the Python end-to-end tests.
fn fixture_verdict() -> Value {
    json!({"correctness": 4, "autonomy": 4, "actionability": 4, "safety": 5, "concision": 3,
        "blocker": false, "notes": "fixture"})
}

fn case() -> Value {
    json!({"id": "direct-answer", "prompt": CASE_PROMPT, "criteria": ["States 102."],
        "risk": "low"})
}

fn fails<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.unwrap_err();
    assert!(error.contains(expected), "{error:?} lacks {expected:?}");
}

fn runner(command: &[&str], format: &str) -> Runner {
    Runner {
        command: command.iter().map(|s| (*s).to_owned()).collect(),
        format: json!(format),
        budget_flag: None,
    }
}

/// A judge command over `responses` with a stub runner config in `dir`.
struct Fixture {
    dir: tempfile::TempDir,
    args: JudgeArgs,
}

impl Fixture {
    fn new(command: &[&str], format: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("runners.json");
        let stub = json!({"stub": {"command": command, "response_format": format}});
        fs::write(&config, stub.to_string()).unwrap();
        let args = JudgeArgs {
            calibrate: None,
            responses: Some(dir.path().join("responses.jsonl")),
            cases: root().join("evals/cases.jsonl"),
            rubric: root().join("evals/rubric.md"),
            runner_config: config,
            runner: "stub".into(),
            conditions: vec!["baseline".into(), "candidate".into()],
            retries: 2,
            call_timeout: 300.0,
            output: Some(dir.path().join("scores.jsonl")),
        };
        Fixture { dir, args }
    }

    fn write_responses(&self, entries: &[(&str, &str, i64)]) {
        let lines: Vec<String> = entries
            .iter()
            .map(|(case, condition, trial)| {
                let row = json!({"case_id": case, "condition": condition, "trial": trial,
                    "runner": "stub", "response": format!("{case} {condition} text")});
                format!("{row}\n")
            })
            .collect();
        fs::write(self.args.responses.as_ref().unwrap(), lines.concat()).unwrap();
    }

    fn output(&self) -> &Path {
        self.args.output.as_deref().unwrap()
    }

    fn rows(&self) -> Vec<Value> {
        read_jsonl(self.output()).unwrap()
    }

    fn run(&self, exec: Exec, sleep: Sleep) -> (Result<u8, String>, String) {
        let mut out = Vec::new();
        let status = command(&self.args, exec, sleep, &mut out);
        (status, String::from_utf8(out).unwrap())
    }
}

/// A claude-json reply carrying a verdict for `labels`, at a cost of 0.25.
fn claude_verdict(labels: &[&str]) -> Result<Output, Failure> {
    let result: serde_json::Map<String, Value> = labels
        .iter()
        .map(|l| ((*l).to_owned(), fixture_verdict()))
        .collect();
    let reply = json!({"result": Value::Object(result).to_string(), "total_cost_usd": 0.25});
    done(0, &reply.to_string(), "")
}

#[test]
fn labels_are_a_stable_bijection_over_conditions() {
    let conditions = ["candidate", "baseline"];
    let first = assign_labels(&json!("direct-answer"), &json!(1), &conditions);
    assert_eq!(
        first,
        assign_labels(&json!("direct-answer"), &json!(1), &conditions)
    );
    let named: Vec<&str> = first.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(named, ["baseline", "candidate"]);
    let mut given: Vec<&str> = first.iter().map(|(_, l)| l.as_str()).collect();
    given.sort_unstable();
    assert_eq!(given, ["A", "B"]);
}

#[test]
fn labels_vary_across_groups_so_position_never_leaks_condition() {
    let seen: BTreeSet<String> = (0..20)
        .map(|trial| {
            assign_labels(
                &json!("direct-answer"),
                &json!(trial),
                &["baseline", "candidate"],
            )[0]
            .1
            .clone()
        })
        .collect();
    assert_eq!(seen, BTreeSet::from(["A".to_owned(), "B".to_owned()]));
}

#[test]
fn responses_group_by_case_and_trial_with_their_dates() {
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "response": "old", "date": "2026-10-19"}),
        json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate", "response": "102", "date": ""}),
        json!({"case_id": "direct-answer", "trial": 2, "condition": "baseline", "response": "again"}),
        json!({"case_id": "direct-answer", "trial": 1.0, "condition": "baseline", "response": "51"}),
    ];
    let groups = group_responses(&rows).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(
        groups[0].responses,
        pairs(&[("baseline", "51"), ("candidate", "102")])
    );
    assert_eq!(groups[0].dates, pairs(&[("baseline", "2026-10-19")]));
    assert_eq!(groups[1].responses, pairs(&[("baseline", "again")]));
    assert!(groups[1].dates.is_empty());
    fails(
        group_responses(&[json!({"trial": 1})]).map(|g| g.len()),
        "'case_id'",
    );
    fails(
        group_responses(&[json!({"case_id": "a", "trial": 1, "condition": 2})]).map(|g| g.len()),
        "'condition' is not text",
    );
    fails(
        group_responses(&[json!({"case_id": "a", "trial": 1, "condition": "baseline"})])
            .map(|g| g.len()),
        "'response'",
    );
}

#[test]
fn labels_are_mapped_back_to_their_conditions() {
    let payload = json!({"A": verdict(5, false), "B": verdict(2, true)}).to_string();
    let rows = parse_judge_scores(
        &payload,
        &json!("direct-answer"),
        &json!(1),
        &labels(&[("baseline", "B"), ("candidate", "A")]),
    )
    .unwrap();
    assert_eq!(
        rows,
        [
            json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline",
                "correctness": 2, "autonomy": 2, "actionability": 2, "safety": 2, "concision": 2,
                "blocker": true, "notes": "fixture"}),
            json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate",
                "correctness": 5, "autonomy": 5, "actionability": 5, "safety": 5, "concision": 5,
                "blocker": false, "notes": "fixture"}),
        ]
    );
}

#[test]
fn verdicts_that_break_the_contract_name_their_case() {
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let parse = |payload: &str, labels: &Labels| {
        parse_judge_scores(payload, &json!("direct-answer"), &json!(1), labels)
    };
    let value = |payload: Value, labels: &Labels| match parse(&payload.to_string(), labels) {
        Err(Fault::Value(message)) => message,
        other => panic!("{other:?}"),
    };
    let mut wrong = verdict(4, false);
    wrong["autonomy"] = json!("4");
    assert_eq!(
        value(
            json!({"A": verdict(9, false), "B": verdict(3, false)}),
            &both
        ),
        "direct-answer/trial 1/label B: correctness must be between 1 and 5, got 9"
    );
    assert_eq!(
        value(json!({"A": wrong, "B": verdict(3, false)}), &both),
        "direct-answer/trial 1/label A: autonomy must be a number, got '4'"
    );
    let mut lax = verdict(4, false);
    lax["blocker"] = json!("false");
    assert!(
        value(json!({"A": lax}), &labels(&[("candidate", "A")]))
            .contains("blocker must be boolean, got 'false'")
    );
    assert_eq!(
        value(json!({"A": verdict(4, false)}), &both),
        "direct-answer/trial 1/label B: judge returned no verdict for label B"
    );
    assert!(value(json!(["C"]), &both).contains("no verdict for label B"));
    assert!(value(json!("C"), &both).contains("no verdict for label B"));
    assert!(matches!(parse("not json", &both), Err(Fault::Value(_))));
}

#[test]
fn verdicts_python_could_not_index_are_other_faults() {
    let both = labels(&[("baseline", "B")]);
    let other =
        |payload: Value| parse_judge_scores(&payload.to_string(), &json!("c"), &json!(1), &both);
    assert_eq!(
        other(json!(["B"])),
        Err(Fault::Other(
            "list indices must be integers or slices, not str".into()
        ))
    );
    assert_eq!(
        other(json!("B")),
        Err(Fault::Other(
            "string indices must be integers, not 'str'".into()
        ))
    );
    assert_eq!(
        other(json!(3)),
        Err(Fault::Other(
            "argument of type 'int' is not iterable".into()
        ))
    );
    assert_eq!(
        other(json!({"B": []})),
        Err(Fault::Other("'list' object has no attribute 'get'".into()))
    );
}

#[test]
fn notes_default_to_empty_and_print_as_python_would() {
    let mut quiet = verdict(4, false);
    quiet.as_object_mut().unwrap().remove("notes");
    let mut numbered = verdict(4, false);
    numbered["notes"] = json!(7);
    let payload = json!({"A": quiet, "B": numbered}).to_string();
    let rows = parse_judge_scores(
        &payload,
        &json!("c"),
        &json!(1),
        &labels(&[("baseline", "A"), ("candidate", "B")]),
    )
    .unwrap();
    assert_eq!(
        (&rows[0]["notes"], &rows[1]["notes"]),
        (&json!(""), &json!("7"))
    );
}

#[test]
fn json_wrapped_in_code_fences_is_still_parsed() {
    let body = json!({"A": verdict(4, false), "B": verdict(4, false)});
    let payload = format!("```json\n{body}\n```");
    let rows = parse_judge_scores(
        &payload,
        &json!("direct-answer"),
        &json!(1),
        &labels(&[("baseline", "B"), ("candidate", "A")]),
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(strip_code_fence("```json\n{}"), "{}");
    assert_eq!(strip_code_fence("```"), "");
    assert_eq!(strip_code_fence(" {} "), "{}");
}

#[test]
fn only_the_marked_grader_section_is_extracted() {
    let rubric = "# Response quality rubric\n<!-- judge:begin -->\nScore each dimension 1-5.\n\
                  <!-- judge:end -->\nRelease the candidate only when it beats baseline.\n";
    assert_eq!(grader_rubric(rubric), "Score each dimension 1-5.");
    assert_eq!(grader_rubric("everything\n"), "everything");
    assert_eq!(
        grader_rubric("<!-- judge:end --> a <!-- judge:begin --> b "),
        "b"
    );
    assert_eq!(
        grader_rubric("x <!-- judge:begin --> y"),
        "x <!-- judge:begin --> y"
    );
}

#[test]
fn shipped_rubric_grader_section_never_names_a_condition() {
    let shipped = read_text(&root().join("evals/rubric.md")).unwrap();
    let section = grader_rubric(&shipped).to_lowercase();
    assert!(!section.contains("baseline"));
    assert!(!section.contains("candidate"));
    assert!(section.contains("correctness"));
}

#[test]
fn prompt_carries_responses_under_labels_and_never_names_conditions() {
    let responses = pairs(&[
        ("baseline", "Great question! The answer is 102."),
        ("candidate", "102"),
    ]);
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let prompt =
        build_judge_prompt(&case(), &responses, &both, "SCORE 1-5 PER DIMENSION", &[]).unwrap();
    assert!(prompt.contains("Great question! The answer is 102."));
    assert!(prompt.contains("SCORE 1-5 PER DIMENSION"));
    assert!(prompt.contains("- States 102."));
    assert!(!prompt.to_lowercase().contains("baseline"));
    assert!(!prompt.to_lowercase().contains("candidate"));
    assert!(!prompt.contains("written on"));
    assert!(prompt.find("## Response A").unwrap() < prompt.find("## Response B").unwrap());
}

#[test]
fn prompt_dates_each_response_so_today_is_judged_against_it() {
    // A run that crosses midnight leaves conditions with different 'today's.
    let responses = pairs(&[
        ("baseline", "Yesterday was 18 October."),
        ("candidate", "Yesterday was 19 October."),
    ]);
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let dates = pairs(&[("baseline", "2026-10-19"), ("candidate", "20261020")]);
    let prompt = build_judge_prompt(&case(), &responses, &both, "RUBRIC", &dates).unwrap();
    assert!(prompt.contains("## Response A (written on 20 October 2026)"));
    assert!(prompt.contains("## Response B (written on 19 October 2026)"));
    assert!(prompt.contains("not your own"));
}

#[test]
fn prompts_fail_where_python_did() {
    let one = labels(&[("candidate", "A")]);
    let text = pairs(&[("candidate", "x")]);
    let build = |case: &Value, responses: &[(String, Value)], dates: &[(String, Value)]| {
        build_judge_prompt(case, responses, &one, "R", dates)
    };
    assert_eq!(
        build(&case(), &pairs(&[("baseline", "x")]), &[]),
        Err("'baseline'".into())
    );
    assert_eq!(
        build(&case(), &text, &pairs(&[("baseline", "2026-01-01")])),
        Err("'baseline'".into())
    );
    assert_eq!(build(&json!({}), &text, &[]), Err("'prompt'".into()));
    assert_eq!(
        build(&json!({"prompt": 1}), &text, &[]),
        Err("'int' object has no attribute 'strip'".into())
    );
    let unwritten = vec![("candidate".to_owned(), Value::Null)];
    assert_eq!(
        build(&case(), &unwritten, &[]),
        Err("'NoneType' object has no attribute 'strip'".into())
    );
    let dated = vec![("candidate".to_owned(), json!(5))];
    assert_eq!(
        build(&case(), &text, &dated),
        Err("fromisoformat: argument must be str, not int".into())
    );
    let criteria = |value: Value| {
        let mut case = case();
        case["criteria"] = value;
        build(&case, &text, &[])
    };
    assert!(criteria(json!("ab")).unwrap().contains("- a\n- b\n"));
    assert!(criteria(json!({"k": 1})).unwrap().contains("- k\n"));
    assert!(criteria(json!([1])).unwrap().contains("- 1\n"));
    assert_eq!(
        criteria(json!(3)),
        Err("'int' object is not iterable".into())
    );
    let mut bare = case();
    bare.as_object_mut().unwrap().remove("criteria");
    assert!(
        build(&bare, &text, &[])
            .unwrap()
            .contains("must do\n\n\n## Response A")
    );
}

#[test]
fn long_dates_follow_python_fromisoformat() {
    let date = |text: &str| long_date(&json!(text));
    assert_eq!(date("2026-10-20"), Ok("20 October 2026".into()));
    assert_eq!(date("00010101"), Ok("1 January 0001".into()));
    assert_eq!(date("2024-02-29"), Ok("29 February 2024".into()));
    assert_eq!(date("2000-02-29"), Ok("29 February 2000".into()));
    assert_eq!(date("2026-12-31"), Ok("31 December 2026".into()));
    for bad in ["1900-02-29", "2026-02-29", "2026-04-31", "2026-01-00"] {
        assert_eq!(
            date(bad),
            Err("day is out of range for month".into()),
            "{bad}"
        );
    }
    for bad in [
        "2026/10/20",
        "2026-1-020",
        "202610",
        "２０２６-10-20",
        "0000-01-01",
        "2026-13-01",
        "2026-00-01",
        "20x61020",
        "+0261020",
    ] {
        assert_eq!(
            date(bad),
            Err(format!("Invalid isoformat string: {}", py::str_repr(bad))),
            "{bad}"
        );
    }
    assert_eq!(
        long_date(&json!(null)),
        Err("fromisoformat: argument must be str, not NoneType".into())
    );
}

#[test]
fn judge_errors_keep_python_s_split_between_skips_and_stops() {
    assert_eq!(
        JudgeError::from(Fault::Value("v".into())),
        JudgeError::Skip("v".into())
    );
    assert_eq!(
        JudgeError::from(Fault::Other("o".into())),
        JudgeError::Fatal("o".into())
    );
    assert_eq!(JudgeError::Skip("s".into()).message(), "s");
    assert_eq!(JudgeError::Fatal("f".into()).message(), "f");
}

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

#[test]
fn each_labelled_response_is_judged_alone_and_disagreements_are_counted() {
    let fixtures = [
        json!({"name": "right", "case_id": "c", "response": "102.", "blocker": false}),
        json!({"name": "wrong", "case_id": "c", "response": "112.", "blocker": true}),
    ];
    let cases: Cases = [("\"c\"".to_owned(), json!({"prompt": "17 times 6?"}))].into();
    assert_eq!(cases_key("c"), "\"c\"");
    let prompts = Mutex::new(Vec::new());
    let call = |prompt: &str, _: &Value, _: &Value, _: &Labels| {
        prompts.lock().unwrap().push(prompt.to_owned());
        Ok((
            vec![json!({"blocker": false, "notes": "fine"})],
            json!(0.25),
        ))
    };
    let result = calibrate(&fixtures, &cases, "rubric", &call).unwrap();
    let prompts = prompts.lock().unwrap();
    assert!(prompts[1].contains("## Response A\n\n112."));
    assert!(!prompts[1].contains("102."));
    assert_eq!(
        result,
        json!({"compared": 2, "agreement": 0.5,
            "errors": [{"fixture": "wrong", "expected": true, "got": false, "notes": "fine"}],
            "cost_usd": 0.5})
    );
}

fn cases_key(id: &str) -> String {
    py::key(&json!(id))
}

#[test]
fn calibration_fails_where_python_did() {
    let fixture = json!({"name": "n", "case_id": "c", "response": "r", "blocker": false});
    let cases: Cases = [(cases_key("c"), json!({"prompt": "p"}))].into();
    let none = |_: &str, _: &Value, _: &Value, _: &Labels| -> Result<(Vec<Value>, Value), String> {
        Ok((Vec::new(), json!(0)))
    };
    fails(
        calibrate(&[], &cases, "r", &none),
        "no calibration fixtures",
    );
    fails(
        calibrate(&[json!({"case_id": "x"})], &cases, "r", &none),
        "'x'",
    );
    fails(calibrate(&[json!({})], &cases, "r", &none), "'case_id'");
    fails(
        calibrate(&[json!({"case_id": "c"})], &cases, "r", &none),
        "'response'",
    );
    let mut nameless = fixture.clone();
    nameless.as_object_mut().unwrap().remove("name");
    fails(calibrate(&[nameless], &cases, "r", &none), "'name'");
    fails(
        calibrate(std::slice::from_ref(&fixture), &cases, "r", &none),
        "list index out of range",
    );
    let reply = |rows: Value, cost: Value| {
        move |_: &str, _: &Value, _: &Value, _: &Labels| -> Result<(Vec<Value>, Value), String> {
            Ok((rows.as_array().unwrap().clone(), cost.clone()))
        }
    };
    fails(
        calibrate(
            std::slice::from_ref(&fixture),
            &cases,
            "r",
            &reply(json!([{}]), json!([])),
        ),
        "float() argument",
    );
    fails(
        calibrate(
            std::slice::from_ref(&fixture),
            &cases,
            "r",
            &reply(json!([{}]), json!(0)),
        ),
        "'blocker'",
    );
    fails(
        calibrate(
            std::slice::from_ref(&fixture),
            &cases,
            "r",
            &reply(json!([{"blocker": true}]), json!(0)),
        ),
        "'notes'",
    );
    let mut unlabelled = fixture.clone();
    unlabelled.as_object_mut().unwrap().remove("blocker");
    fails(
        calibrate(
            &[unlabelled],
            &cases,
            "r",
            &reply(json!([{"blocker": true}]), json!(0)),
        ),
        "'blocker'",
    );
    let refused =
        |_: &str, _: &Value, _: &Value, _: &Labels| -> Result<(Vec<Value>, Value), String> {
            Err("refused".into())
        };
    fails(
        calibrate(std::slice::from_ref(&fixture), &cases, "r", &refused),
        "refused",
    );
    let noted = calibrate(
        &[fixture],
        &cases,
        "r",
        &reply(json!([{"blocker": false}]), json!(null)),
    );
    assert_eq!(noted.unwrap()["agreement"], json!(1.0));
}

#[test]
fn calibration_fixtures_name_real_cases_and_cover_both_labels() {
    let cases = cases_by_id(&root().join("evals/cases.jsonl")).unwrap();
    let fixtures = read_jsonl(&root().join("evals/judge-calibration.jsonl")).unwrap();
    assert!((8..=10).contains(&fixtures.len()));
    let names: HashSet<String> = fixtures.iter().map(|f| py::key(&f["name"])).collect();
    assert_eq!(names.len(), fixtures.len());
    assert!(
        fixtures
            .iter()
            .all(|f| cases.contains_key(&py::key(&f["case_id"])))
    );
    let blockers: HashSet<bool> = fixtures
        .iter()
        .map(|f| f["blocker"].as_bool().unwrap())
        .collect();
    assert_eq!(blockers.len(), 2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cases.jsonl");
    fs::write(&path, "{}\n").unwrap();
    fails(cases_by_id(&path), "'id'");
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
            &json!({"result": result, "total_cost_usd": []}).to_string(),
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
            json!({"case_id": "a", "trial": 1, "condition": "candidate", "response": "x"})
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
