// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::cases::{load_cases, root, validate_cases};
use crate::proc::{Failure, Output};
use crate::testing::done;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

fn items() -> Vec<Value> {
    vec![
        json!({"id": "f1", "kind": "fact", "text": "GST payable is $4,812."}),
        json!({"id": "q1", "kind": "qualifier", "text": "Lodgment is likely by Friday."}),
    ]
}

fn reply(rows: Value, invented: Value) -> Value {
    Value::from(json!({"items": rows, "invented": invented}).to_string())
}

fn verdicts(reply: &Value, rewrite: &str) -> Vec<String> {
    let (verdicts, _) = parse_reply(reply, rewrite, &items()).unwrap();
    verdicts
        .iter()
        .map(|v| v["verdict"].as_str().unwrap().to_owned())
        .collect()
}

fn ledgers() -> Ledgers {
    Ledgers::from([("c".to_owned(), items())])
}

fn sources_of(pairs: &[(&str, &str)]) -> Sources {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn rows() -> Vec<Value> {
    vec![
        json!({"runner": "a", "case_id": "c", "trial": 1, "response": "GST is $4,812 and likely by Friday."}),
        json!({"runner": "b", "case_id": "c", "trial": 1, "response": "Nothing."}),
    ]
}

fn empty_reply() -> Value {
    reply(json!([]), json!([]))
}

fn setups(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .map(|row| row["setup"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn keeps_a_verified_quote_across_case_quotes_and_wrapping() {
    let rewrite = "GST payable:\n$4,812. It\u{2019}s likely by Friday.";
    let reply = reply(
        json!([{"id": "f1", "verdict": "kept", "quote": "gst payable: $4,812"},
               {"id": "q1", "verdict": "kept", "quote": "it's likely by friday"}]),
        json!([]),
    );
    assert_eq!(verdicts(&reply, rewrite), ["kept", "kept"]);
}

#[test]
fn a_quote_not_in_the_rewrite_counts_as_dropped() {
    let reply = reply(
        json!([{"id": "f1", "verdict": "kept", "quote": "$4,812"}, {"id": "q1", "verdict": "hardened", "quote": ""}]),
        json!([]),
    );
    let (verdicts, _) = parse_reply(&reply, "No figures here.", &items()).unwrap();
    assert!(
        verdicts
            .iter()
            .all(|v| v["verdict"] == "dropped" && v["unverified_quote"] == true)
    );
}

#[test]
fn a_missing_item_counts_as_dropped_and_hardened_is_kept_apart() {
    let reply = reply(
        json!([{"id": "q1", "verdict": "hardened", "quote": "by Friday"}]),
        json!([]),
    );
    assert_eq!(verdicts(&reply, "Due by Friday."), ["dropped", "hardened"]);
}

#[test]
fn verdict_rows_keep_the_judges_word_and_quote() {
    let reply = reply(
        json!([{"id": "f1", "verdict": "kept", "quote": 4812}]),
        json!([]),
    );
    let (verdicts, _) = parse_reply(&reply, "It is 4812.", &items()).unwrap();
    assert_eq!(
        verdicts[0],
        json!({"id": "f1", "kind": "fact", "verdict": "kept", "judge_verdict": "kept",
               "unverified_quote": false, "quote": "4812"})
    );
}

#[test]
fn an_unknown_verdict_is_an_error_not_a_pass() {
    let reply = reply(
        json!([{"id": "f1", "verdict": "fine", "quote": "x"}]),
        json!([]),
    );
    assert_eq!(
        parse_reply(&reply, "x", &items()),
        Err(ReplyError::Retry("unknown verdict: fine".into()))
    );
    let null = reply_with(json!([{"id": "f1", "verdict": null}]));
    assert_eq!(
        parse_reply(&null, "x", &items()),
        Err(ReplyError::Retry("unknown verdict: None".into()))
    );
}

fn reply_with(rows: Value) -> Value {
    reply(rows, json!([]))
}

#[test]
fn replies_python_retried_are_retried_and_others_end_the_row() {
    let read = |reply: &str| parse_reply(&Value::from(reply), "x", &items()).map(|_| ());
    for text in [
        "no braces",
        "} {",
        "{not json}",
        r#"{"invented": []}"#,
        r#"{"items": "x", "invented": []}"#,
        r#"{"items": {"a": 1}, "invented": []}"#,
        r#"{"items": [3], "invented": []}"#,
        r#"{"items": []}"#,
        r#"{"items": [], "invented": "x"}"#,
    ] {
        assert!(matches!(read(text), Err(ReplyError::Retry(_))), "{text}");
    }
    for text in [
        r#"{"items": 3, "invented": []}"#,
        r#"{"items": null}"#,
        r#"{"items": [{"id": [1]}]}"#,
        r#"{"items": [{"id": "f1", "verdict": ["kept"]}], "invented": []}"#,
    ] {
        assert!(matches!(read(text), Err(ReplyError::Fatal(_))), "{text}");
    }
    assert!(matches!(
        parse_reply(&json!(null), "x", &items()),
        Err(ReplyError::Fatal(_))
    ));
    for text in [
        r#"{"items": "", "invented": []}"#,
        r#"{"items": {}, "invented": []}"#,
    ] {
        assert_eq!(verdicts(&Value::from(text), "x"), ["dropped", "dropped"]);
    }
    let no_id = vec![json!({"kind": "fact"})];
    assert!(parse_reply(&empty_reply(), "x", &no_id).is_err());
    let no_kind = vec![json!({"id": "f1"})];
    assert!(parse_reply(&empty_reply(), "x", &no_kind).is_err());
}

#[test]
fn the_judge_prompt_names_no_setup() {
    let prompt = judge_prompt("Source.", "Some text.", &items())
        .unwrap()
        .to_lowercase();
    for name in ["laconic", "humanizer", "caveman", "setup"] {
        assert!(!prompt.contains(name), "{name}");
    }
    assert!(prompt.contains("items:\nf1 (fact): gst payable is $4,812.\nq1 (qualifier): "));
    assert!(judge_prompt("s", "t", &[json!({"id": "f1"})]).is_err());
}

#[test]
fn only_invented_passages_found_in_the_rewrite_count() {
    let (_, invented) = parse_reply(
        &reply(json!([]), json!(["costs $9", "not there", "", 4])),
        "It costs $9.",
        &items(),
    )
    .unwrap();
    assert_eq!(invented, ["costs $9"]);
}

#[test]
fn repeated_invented_passages_count_once() {
    let (_, invented) = parse_reply(
        &reply(json!([]), json!(["costs $9", "Costs  $9"])),
        "It costs $9.",
        &items(),
    )
    .unwrap();
    assert_eq!(invented, ["costs $9"]);
}

#[test]
fn a_reply_without_an_invented_list_is_an_error_not_a_zero() {
    let reply = Value::from(json!({"items": []}).to_string());
    assert!(parse_reply(&reply, "Text.", &items()).is_err());
}

#[test]
fn normalising_folds_case_as_python_casefold() {
    // Python: '‘Straße’  ﬁne Σς'.casefold() after the quote and
    // space steps gives "'strasse' fine σσ".
    assert_eq!(
        normalise("\u{2018}Stra\u{df}e\u{2019}  \u{fb01}ne \u{3a3}\u{3c2}"),
        "'strasse' fine \u{3c3}\u{3c3}"
    );
    assert_eq!(normalise("\u{201c}A\u{201d}\n\tB"), "\"a\" b");
}

#[test]
fn a_verbatim_copy_reaches_the_judge_and_disagreements_are_counted() {
    let fixtures = [
        json!({"name": "copy", "case_id": "c", "rewrite": "@source", "default": "kept", "expected": {}, "invented": false}),
    ];
    let prompts = Mutex::new(Vec::new());
    let call = |prompt: &str| {
        prompts.lock().unwrap().push(prompt.to_owned());
        Ok((
            reply(
                json!([{"id": "f1", "verdict": "kept", "quote": "GST"}, {"id": "q1", "verdict": "dropped", "quote": ""}]),
                json!([]),
            ),
            0.0,
        ))
    };
    let result = calibrate(
        &fixtures,
        &ledgers(),
        &sources_of(&[("c", "GST is due.")]),
        &call,
    )
    .unwrap();
    assert!(prompts.lock().unwrap()[0].contains("<text>\nGST is due.\n</text>"));
    assert_eq!(result["compared"], 3);
    let errors: Vec<_> = result["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["item"].clone(), e["expected"].clone(), e["got"].clone()))
        .collect();
    assert_eq!(errors, [(json!("q1"), json!("kept"), json!("dropped"))]);
    assert_eq!(
        py::dumps(&result, None, true),
        r#"{"compared": 3, "agreement": 0.6666666666666667, "errors": [{"fixture": "copy", "item": "q1", "expected": "kept", "got": "dropped"}], "cost_usd": 0.0}"#
    );
}

#[test]
fn calibration_compares_expected_items_and_invented_passages() {
    let fixture = json!({"name": "own", "case_id": "c", "rewrite": "It costs $9.", "default": "dropped",
                         "expected": {"f1": "kept"}, "invented": 1});
    let call = |_: &str| {
        Ok((
            reply(
                json!([{"id": "f1", "verdict": "kept", "quote": "costs"}]),
                json!(["costs $9"]),
            ),
            0.5,
        ))
    };
    let result = calibrate(
        std::slice::from_ref(&fixture),
        &ledgers(),
        &sources_of(&[("c", "s")]),
        &call,
    )
    .unwrap();
    assert_eq!(result["errors"], json!([]));
    let mut wrong = fixture;
    wrong["invented"] = json!(false);
    let result = calibrate(&[wrong], &ledgers(), &sources_of(&[("c", "s")]), &call).unwrap();
    assert_eq!(
        result["errors"],
        json!([{"fixture": "own", "item": "invented", "expected": false, "got": ["costs $9"]}])
    );
}

#[test]
fn calibration_errors_stop_the_run() {
    let call = |_: &str| Ok((empty_reply(), 0.0));
    let sources = sources_of(&[("c", "s")]);
    let base = json!({"name": "n", "case_id": "c", "rewrite": "r", "default": "kept", "expected": {}, "invented": false});
    let with = |name: &str, value: Value| {
        let mut fixture = base.clone();
        fixture[name] = value;
        fixture
    };
    for fixture in [
        with("case_id", json!("d")),
        with("rewrite", json!(3)),
        with("expected", json!([])),
    ] {
        assert!(
            calibrate(std::slice::from_ref(&fixture), &ledgers(), &sources, &call).is_err(),
            "{fixture}"
        );
    }
    assert!(
        calibrate(
            std::slice::from_ref(&base),
            &Ledgers::new(),
            &sources,
            &call
        )
        .is_err()
    );
    let failing = |_: &str| Err("judge exited 1".to_owned());
    assert_eq!(
        calibrate(std::slice::from_ref(&base), &ledgers(), &sources, &failing),
        Err("judge exited 1".into())
    );
    let unreadable = |_: &str| Ok((json!("no json"), 0.0));
    assert!(calibrate(&[base], &ledgers(), &sources, &unreadable).is_err());
    assert!(calibrate(&[], &ledgers(), &sources, &call).is_err());
}

#[test]
fn resumes_without_judging_a_row_twice() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    let calls = AtomicUsize::new(0);
    let call = |_: &str| {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok((empty_reply(), 0.1))
    };
    let sources = sources_of(&[("c", "Source.")]);
    judge_rows(&rows(), &ledgers(), &sources, &output, &call, 2, 12.0).unwrap();
    judge_rows(&rows(), &ledgers(), &sources, &output, &call, 2, 12.0).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(read_jsonl(&output).unwrap().len(), 2);
}

#[test]
fn a_retried_judge_call_still_counts_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    let replies = Mutex::new(vec![(empty_reply(), 0.25), (json!("not json"), 0.5)]);
    let call = |_: &str| Ok(replies.lock().unwrap().pop().unwrap());
    let spent = judge_rows(
        &rows()[..1],
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &output,
        &call,
        4,
        12.0,
    )
    .unwrap();
    assert!((spent - 0.75).abs() < f64::EPSILON);
    let saved = read_jsonl(&output).unwrap();
    assert_eq!(saved[0]["cost"], 0.75);
    assert_eq!(saved[0]["setup"], "a");
}

#[test]
fn a_row_that_exhausts_its_retries_records_its_cost_for_spend() {
    let dir = tempfile::tempdir().unwrap();
    let call = |_: &str| Ok((json!("not json"), 0.5));
    let error = judge_rows(
        &rows()[..1],
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &dir.path().join("scores.jsonl"),
        &call,
        4,
        12.0,
    );
    assert_eq!(error, Err("a c: judge reply holds no JSON".into()));
    assert_eq!(total_spend(dir.path()), Ok(1.5));
    let overhead = fs::read_to_string(dir.path().join("overhead.jsonl")).unwrap();
    assert_eq!(
        overhead,
        "{\"stage\": \"failed-judge\", \"cost_usd\": 1.5}\n"
    );
}

#[test]
fn a_reply_python_could_not_retry_loses_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let call = |_: &str| Ok((json!(null), 0.5));
    let error = judge_rows(
        &rows()[..1],
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &dir.path().join("scores.jsonl"),
        &call,
        4,
        12.0,
    );
    assert!(error.unwrap_err().starts_with("a c: expected string"));
    assert_eq!(total_spend(dir.path()), Ok(0.0));
}

#[test]
fn rows_running_beside_a_failure_are_saved_and_costed() {
    let dir = tempfile::tempdir().unwrap();
    let failing = AtomicUsize::new(0);
    let call = |prompt: &str| {
        if prompt.contains("Nothing.") {
            failing.fetch_add(1, Ordering::SeqCst);
            return Ok((json!("not json"), 0.5));
        }
        let start = Instant::now();
        loop {
            thread::sleep(Duration::from_millis(5));
            if failing.load(Ordering::SeqCst) >= 3 || start.elapsed() > Duration::from_secs(10) {
                break;
            }
        }
        thread::sleep(Duration::from_millis(200)); // Lets the failure surface before this row finishes.
        Ok((empty_reply(), 0.25))
    };
    let result = judge_rows(
        &rows(),
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &dir.path().join("scores.jsonl"),
        &call,
        2,
        12.0,
    );
    assert!(result.is_err());
    assert_eq!(
        setups(&read_jsonl(&dir.path().join("scores.jsonl")).unwrap()),
        ["a"]
    );
    assert_eq!(total_spend(dir.path()), Ok(1.75));
}

#[test]
fn a_process_failure_after_a_paid_retry_keeps_that_cost() {
    let dir = tempfile::tempdir().unwrap();
    let replies = Mutex::new(vec![(json!("not json"), 0.5)]);
    let call = |_: &str| {
        replies
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| "judge exited 1".to_owned())
    };
    let error = judge_rows(
        &rows()[..1],
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &dir.path().join("scores.jsonl"),
        &call,
        4,
        12.0,
    );
    assert_eq!(error, Err("a c: judge exited 1".into()));
    assert_eq!(total_spend(dir.path()), Ok(0.5));
}

#[test]
fn stops_submitting_at_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    let rows: Vec<Value> = (0..10)
        .map(|n| {
            let mut row = rows()[0].clone();
            row["trial"] = json!(n);
            row
        })
        .collect();
    let call = |_: &str| Ok((empty_reply(), 1.0));
    judge_rows(
        &rows,
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &output,
        &call,
        1,
        3.0,
    )
    .unwrap();
    assert_eq!(read_jsonl(&output).unwrap().len(), 3);
}

#[test]
fn rows_are_judged_in_pythons_shuffled_order() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    let rows: Vec<Value> = (0..5)
        .map(|n| {
            let mut row = rows()[0].clone();
            row["trial"] = json!(n);
            row
        })
        .collect();
    let call = |_: &str| Ok((empty_reply(), 0.0));
    judge_rows(
        &rows,
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &output,
        &call,
        1,
        12.0,
    )
    .unwrap();
    let trials: Vec<i64> = read_jsonl(&output)
        .unwrap()
        .iter()
        .map(|row| row["trial"].as_i64().unwrap())
        .collect();
    // Python: random.Random(0).shuffle(list(range(5))) gives [2, 1, 0, 4, 3].
    assert_eq!(trials, [2, 1, 0, 4, 3]);
}

#[test]
fn a_case_without_a_ledger_fails_before_any_call() {
    let dir = tempfile::tempdir().unwrap();
    let call = |_: &str| -> Result<(Value, f64), String> { panic!("called") };
    let error = judge_rows(
        &rows(),
        &Ledgers::new(),
        &Sources::new(),
        &dir.path().join("s.jsonl"),
        &call,
        4,
        12.0,
    );
    assert_eq!(error, Err("no ledger for: ['c']".into()));
    assert!(!dir.path().join("s.jsonl").exists());
}

#[test]
fn judging_rejects_bad_settings_and_rows() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    let call = |_: &str| Ok((empty_reply(), 0.0));
    let sources = sources_of(&[("c", "Source.")]);
    assert!(judge_rows(&rows(), &ledgers(), &sources, &output, &call, 0, 12.0).is_err());
    assert!(
        judge_rows(
            &[json!({"case_id": "c"})],
            &ledgers(),
            &sources,
            &output,
            &call,
            1,
            12.0
        )
        .is_err()
    );
    let error = judge_rows(
        &rows()[..1],
        &ledgers(),
        &Sources::new(),
        &output,
        &call,
        1,
        12.0,
    );
    assert_eq!(error, Err("a c: 'c'".into()));
    let mut silent = rows()[0].clone();
    silent["response"] = json!(null);
    assert!(judge_rows(&[silent], &ledgers(), &sources, &output, &call, 1, 12.0).is_err());
    fs::write(&output, "{\"case_id\": \"c\"}\n").unwrap();
    assert!(judge_rows(&rows(), &ledgers(), &sources, &output, &call, 1, 12.0).is_err());
    let folder = dir.path().join("folder");
    fs::create_dir(&folder).unwrap();
    assert!(judge_rows(&rows(), &ledgers(), &sources, &folder, &call, 1, 12.0).is_err());
}

#[test]
fn write_failures_end_the_run_once_running_rows_finish() {
    let dir = tempfile::tempdir().unwrap();
    let sources = sources_of(&[("c", "Source.")]);
    let jammed = dir.path().join("jammed");
    fs::create_dir_all(jammed.join("overhead.jsonl")).unwrap();
    let failing = |_: &str| Err("down".to_owned());
    let error = judge_rows(
        &rows(),
        &ledgers(),
        &sources,
        &jammed.join("scores.jsonl"),
        &failing,
        1,
        12.0,
    );
    assert!(error.unwrap_err().contains("overhead.jsonl"));
}

#[test]
fn a_row_already_judged_is_not_judged_again_after_a_write() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("scores.jsonl");
    fs::write(
        &output,
        "{\"setup\": \"a\", \"case_id\": \"c\", \"trial\": 1.0}\n\n",
    )
    .unwrap();
    let call = |_: &str| Ok((empty_reply(), 0.0));
    judge_rows(
        &rows(),
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &output,
        &call,
        1,
        12.0,
    )
    .unwrap();
    assert_eq!(setups(&read_jsonl(&output).unwrap()[1..]), ["b"]);
}

fn marks(pairs: &[(&str, &str)]) -> Value {
    pairs
        .iter()
        .map(|(kind, verdict)| json!({"kind": kind, "verdict": verdict, "unverified_quote": false}))
        .collect()
}

fn score(setup: &str, case: &str, trial: i64, verdicts: Value, invented: Value) -> Value {
    json!({"setup": setup, "case_id": case, "trial": trial, "verdicts": verdicts, "invented": invented})
}

fn answer(setup: &str, case: &str, trial: i64, response: &str) -> Value {
    json!({"runner": setup, "case_id": case, "trial": trial, "response": response})
}

fn no_added(_: &str, _: &str) -> Result<i64, String> {
    Ok(0)
}

#[test]
fn unequal_coverage_is_an_error_not_a_silent_intersection() {
    let kept = marks(&[("fact", "kept")]);
    let scores = [
        score("a", "c", 1, kept.clone(), json!([])),
        score("a", "d", 1, kept.clone(), json!([])),
        score("b", "c", 1, kept, json!([])),
    ];
    let responses: Vec<Value> = scores
        .iter()
        .map(|s| {
            answer(
                s["setup"].as_str().unwrap(),
                s["case_id"].as_str().unwrap(),
                1,
                "x",
            )
        })
        .collect();
    let error = summarise(
        &scores,
        &responses,
        &sources_of(&[("c", "x"), ("d", "x")]),
        "a",
        &no_added,
        1,
        None,
    );
    assert_eq!(
        error,
        Err("incomplete, duplicate or unequal coverage: b".into())
    );
}

#[test]
fn a_setup_missing_from_the_plan_is_an_error() {
    let one = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    let plan = BTreeSet::from(["a".to_owned(), "b".to_owned()]);
    let error = summarise(
        &[one],
        &[answer("a", "c", 1, "x")],
        &sources_of(&[("c", "x")]),
        "a",
        &no_added,
        1,
        Some(&plan),
    );
    assert_eq!(error, Err("setups differ from the plan: ['b']".into()));
}

#[test]
fn a_duplicate_row_or_an_unrun_trial_is_an_error() {
    let one = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    let reply = answer("a", "c", 1, "x");
    let sources = sources_of(&[("c", "x")]);
    for (scores, responses, trials) in [
        (
            vec![one.clone(), one.clone()],
            vec![reply.clone(), reply.clone()],
            1,
        ),
        (vec![one], vec![reply], 2),
    ] {
        assert!(summarise(&scores, &responses, &sources, "a", &no_added, trials, None).is_err());
    }
}

#[test]
fn rates_count_kept_only_and_gap_is_reference_minus_setup() {
    let scores = [
        score(
            "a",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "kept")]),
            json!(["x"]),
        ),
        score(
            "b",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "hardened")]),
            json!([]),
        ),
    ];
    let responses = [
        answer("a", "c", 1, "one two"),
        answer("b", "c", 1, "one two three four"),
    ];
    let summary = summarise(
        &scores,
        &responses,
        &sources_of(&[("c", "one two three four")]),
        "a",
        &no_added,
        1,
        None,
    )
    .unwrap();
    assert_eq!(summary["a"]["qualifier_survival"], 1.0);
    assert_eq!(summary["b"]["qualifier_survival"], 0.0);
    assert_eq!(summary["b"]["hardened"], 1);
    assert_eq!(summary["a"]["length_share"], 0.5);
    assert_eq!(summary["b"]["reference_gap"]["mean"], 0.5);
    assert_eq!(
        summary["a"]["survival_by_text"].as_f64().unwrap()
            - summary["b"]["survival_by_text"].as_f64().unwrap(),
        0.5
    );
    assert_eq!(summary["a"]["invented_per_text"], 1);
}

#[test]
fn the_summary_prints_as_python_printed_it() {
    let mut answers = Vec::new();
    let mut scores = Vec::new();
    for (setup, case, trial, verdicts, invented, cost) in [
        (
            "a",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "kept")]),
            json!(["x"]),
            json!(0.1),
        ),
        (
            "a",
            "c",
            2,
            marks(&[("fact", "dropped"), ("qualifier", "kept")]),
            json!([]),
            json!(0.2),
        ),
        (
            "a",
            "d",
            1,
            marks(&[("fact", "kept")]),
            json!(["y", "z"]),
            json!("0.3"),
        ),
        (
            "a",
            "d",
            2,
            marks(&[("fact", "kept")]),
            json!([]),
            json!(null),
        ),
        (
            "b",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "hardened")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "c",
            2,
            marks(&[("fact", "kept"), ("qualifier", "dropped")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "d",
            1,
            marks(&[("fact", "dropped")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "d",
            2,
            marks(&[("fact", "kept")]),
            json!([]),
            json!(0.1),
        ),
    ] {
        scores.push(score(setup, case, trial, verdicts, invented));
        let mut reply = answer(
            setup,
            case,
            trial,
            if setup == "a" {
                "one two"
            } else {
                "one two three"
            },
        );
        reply["cost_usd"] = cost;
        answers.push(reply);
    }
    let sources = sources_of(&[("c", "one two three four"), ("d", "one two three")]);
    let added = |_: &str, rewrite: &str| Ok(i64::try_from(rewrite.len() % 3).unwrap());
    let summary = summarise(&scores, &answers, &sources, "a", &added, 2, None).unwrap();
    // The text Python's json.dumps(summary, indent=2) printed for the same inputs.
    let python = include_str!("summary.json");
    assert_eq!(py::dumps_pretty(&summary), python.trim_end());
}

#[test]
fn summaries_reject_malformed_scores() {
    let sources = sources_of(&[("c", "one")]);
    let reply = [answer("a", "c", 1, "one")];
    let summary = |score: Value| summarise(&[score], &reply, &sources, "a", &no_added, 1, None);
    assert_eq!(
        summary(score("b", "c", 1, marks(&[]), json!([]))),
        Err("reference setup has no scores: a".into())
    );
    assert!(summary(score("a", "c", 1, marks(&[]), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, json!({}), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, marks(&[("fact", "kept")]), json!(3))).is_err());
    let mut unflagged = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    unflagged["verdicts"][0]["unverified_quote"] = json!(0);
    assert!(summary(unflagged).is_err());
    let mut nameless = score("a", "c", 1, marks(&[]), json!([]));
    nameless["setup"] = json!(1);
    assert!(summary(nameless).is_err());
    assert!(summary(score("a", "c", 1, json!([{"verdict": "kept"}]), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, json!([{"kind": "fact"}]), json!([]))).is_err());
    let counted = summary(score("a", "c", 1, marks(&[("fact", "kept")]), json!("ab"))).unwrap();
    assert_eq!(counted["a"]["invented_per_text"], 2);
    let counted = summary(score(
        "a",
        "c",
        1,
        marks(&[("fact", "kept")]),
        json!({"k": 1}),
    ))
    .unwrap();
    assert_eq!(counted["a"]["invented_per_text"], 1);
    let blank = sources_of(&[("c", " ")]);
    let empty = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &reply,
        &blank,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(empty.is_err());
    let failing = |_: &str, _: &str| Err("preserve exited 2".to_owned());
    let added = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &reply,
        &sources,
        "a",
        &failing,
        1,
        None,
    );
    assert_eq!(added, Err("preserve exited 2".into()));
    let mut unpriced = reply[0].clone();
    unpriced["cost_usd"] = json!([1]);
    let priced = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &[unpriced],
        &sources,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(priced.is_err());
    let runnerless = summarise(
        &[score("a", "c", 1, marks(&[]), json!([]))],
        &[json!({})],
        &sources,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(runnerless.is_err());
}

#[test]
fn paired_intervals_need_shared_cases_and_rounds() {
    let a = PerCase::from([("c".to_owned(), vec![1.0])]);
    let b = PerCase::from([("d".to_owned(), vec![1.0])]);
    assert_eq!(
        paired_interval(&a, &b, 10, 0),
        Err("no shared cases".into())
    );
    assert_eq!(
        paired_interval(&a, &a, 0, 0),
        Err("list index out of range".into())
    );
    assert_eq!(paired_interval(&a, &a, 1, 0), Ok((0.0, 0.0, 0.0)));
    assert_eq!(paired_interval(&a, &a, 100, 0), Ok((0.0, 0.0, 0.0)));
}

#[test]
fn counts_generation_overhead_and_judging() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    fs::write(
        folder.join("run-a.jsonl"),
        "{\"cost_usd\": 1.5}\n{\"cost_usd\": null}\n",
    )
    .unwrap();
    fs::write(
        folder.join("overhead.jsonl"),
        "{\"stage\": \"preflight\", \"cost_usd\": 0.5}\n",
    )
    .unwrap();
    fs::write(folder.join("scores.jsonl"), "{\"cost\": 2.0}\n").unwrap();
    fs::write(folder.join("responses.jsonl"), "{\"cost_usd\": 99}\n").unwrap();
    fs::write(folder.join("run-.json"), "{\"cost_usd\": 99}\n").unwrap();
    assert_eq!(total_spend(folder), Ok(4.0));
    fs::write(folder.join("run-b.jsonl"), "[]\n").unwrap();
    assert!(total_spend(folder).is_err());
    assert_eq!(total_spend(&folder.join("missing")), Ok(0.0));
}

#[test]
fn spend_sums_each_file_as_python_did() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    // Python 3.12+: 0.1 + 0.2 + 0.3 is summed exactly within a file, 0.6000000000000001 across files.
    fs::write(
        folder.join("run-b.jsonl"),
        "{\"cost_usd\": 0.1}\n{\"cost_usd\": 0.2}\n{\"cost_usd\": 0.3}\n",
    )
    .unwrap();
    assert_eq!(total_spend(folder), Ok(0.6));
    fs::write(folder.join("run-a.jsonl"), "{\"cost_usd\": \" 1e-1 \"}\n").unwrap();
    assert_eq!(total_spend(folder), Ok(0.7));
    assert_eq!(money(0.125), "0.12");
    assert_eq!(money(f64::NAN), "nan");
}

#[test]
fn costs_read_as_python_float_reads_them() {
    assert_eq!(to_float(None), Ok(0.0));
    assert_eq!(to_float(Some(&json!(true))), Ok(1.0));
    assert_eq!(to_float(Some(&json!(""))), Ok(0.0));
    assert_eq!(to_float(Some(&json!(2))), Ok(2.0));
    assert_eq!(
        to_float(Some(&json!("x"))),
        Err("could not convert string to float: 'x'".into())
    );
    assert!(to_float(Some(&json!([1]))).is_err());
}

#[test]
fn added_counts_preserves_added_lines() {
    let exec = |command: &[String], input: &Input| {
        assert!(matches!(input, Input::Inherit));
        assert_eq!(fs::read_to_string(&command[3]).unwrap(), "rewrite");
        done(
            1,
            "added number: 4\ncheck added name: Jane\nadded name: Ann\n",
            "",
        )
    };
    assert_eq!(added_count(&exec, "source", "rewrite"), Ok(2));
    assert_eq!(
        added_count(&|_: &[String], _: &Input| done(0, "", ""), "s", "r"),
        Ok(0)
    );
    let error = added_count(&|_: &[String], _: &Input| done(2, "", "bad\n"), "s", "r");
    assert_eq!(error, Err("preserve exited 2: bad".into()));
    let missing =
        |_: &[String], _: &Input| -> Result<Output, Failure> { Err(Failure::Spawn("gone".into())) };
    assert!(added_count(&missing, "s", "r").is_err());
}

#[cfg(unix)]
#[test]
fn the_judge_runs_the_command_with_the_prompt_on_its_input() {
    let dir = tempfile::tempdir().unwrap();
    let judge = crate::testing::script(
        dir.path(),
        "judge",
        "cat > \"$0.in\"; printf '{\"result\": \"ok\", \"total_cost_usd\": 0.5}'",
    );
    let command = [judge.display().to_string()];
    assert_eq!(
        call_judge("prompt", &command, Duration::from_secs(30)),
        Ok((json!("ok"), 0.5))
    );
    assert_eq!(
        fs::read_to_string(format!("{}.in", judge.display())).unwrap(),
        "prompt"
    );
    let failing = crate::testing::script(
        dir.path(),
        "failing",
        &format!("printf '{}' >&2; exit 3", "x".repeat(400)),
    );
    let error = call_judge(
        "p",
        &[failing.display().to_string()],
        Duration::from_secs(30),
    )
    .unwrap_err();
    assert_eq!(error, format!("judge exited 3: {}", "x".repeat(300)));
    let bare = crate::testing::script(dir.path(), "bare", "printf '{}'");
    assert!(call_judge("p", &[bare.display().to_string()], Duration::from_secs(30)).is_err());
    let missing = [dir.path().join("none").display().to_string()];
    assert!(call_judge("p", &missing, Duration::from_secs(30)).is_err());
}

#[test]
fn ledgers_reject_duplicates_and_bad_items() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledgers.jsonl");
    let write = |rows: &[Value]| crate::testing::write_rows(&path, rows);
    let item = json!({"id": "f1", "kind": "fact", "text": "x"});
    write(&[
        json!({"case_id": "hum-a", "items": [item]}),
        json!({"case_id": "fact-a", "items": []}),
    ]);
    assert_eq!(load_ledgers(&path), Err("duplicate ledger: fact-a".into()));
    write(&[json!({"case_id": "hum-a", "items": [{"id": "f1", "kind": "claim", "text": "x"}]})]);
    assert_eq!(
        load_ledgers(&path),
        Err("bad item in fact-a: {'id': 'f1', 'kind': 'claim', 'text': 'x'}".into())
    );
    write(&[json!({"case_id": "hum-a", "items": [{"id": "f1", "kind": "fact", "text": " "}]})]);
    assert!(load_ledgers(&path).is_err());
    write(&[json!({"case_id": "hum-a", "items": {}})]);
    assert!(load_ledgers(&path).is_err());
    fs::write(&path, "{\n").unwrap();
    assert!(load_ledgers(&path).is_err());
}

#[test]
fn sources_and_cases_need_text_ids_and_prompts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cases.jsonl");
    crate::testing::write_rows(&path, &[json!({"id": "hum-a", "prompt": "Do.\n\nText."})]);
    assert_eq!(sources(&path), Ok(sources_of(&[("hum-a", "Text.")])));
    crate::testing::write_rows(&path, &[json!({"id": 1, "prompt": "Do.\n\nText."})]);
    assert!(sources(&path).is_err());
    assert!(build_cases(&[json!({"id": "hum-a", "prompt": "Do.\n\nText."})]).is_err());
    assert!(build_cases(&[json!({"id": "hum-a", "prompt": "Do."})]).is_err());
}

#[test]
fn cases_are_valid_and_every_case_has_a_ledger() {
    let cases = load_cases(&root().join("evals/fact-cases.jsonl")).unwrap();
    assert!(validate_cases(&cases).is_empty());
    let ledgers = load_ledgers(&root().join("evals/fact-ledgers.jsonl")).unwrap();
    let ids: BTreeSet<&str> = cases
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ledgers.keys().map(String::as_str).collect());
}

#[test]
fn calibration_fixtures_name_real_cases_items_and_verdicts() {
    let ledgers = load_ledgers(&root().join("evals/fact-ledgers.jsonl")).unwrap();
    let fixtures = read_jsonl(&root().join("evals/fact-calibration.jsonl")).unwrap();
    assert!(fixtures.len() >= 6);
    for fixture in &fixtures {
        let ids: BTreeSet<&str> = ledgers[fixture["case_id"].as_str().unwrap()]
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect();
        let expected = fixture["expected"].as_object().unwrap();
        assert!(
            expected.keys().all(|id| ids.contains(id.as_str())),
            "{fixture}"
        );
        let named = expected.values().chain([&fixture["default"]]);
        assert!(
            named
                .into_iter()
                .all(|v| VERDICTS.contains(&v.as_str().unwrap())),
            "{fixture}"
        );
    }
}

#[test]
fn cases_match_their_sources() {
    let built =
        build_cases(&read_jsonl(&root().join("evals/humanise-cases.jsonl")).unwrap()).unwrap();
    assert_eq!(
        built,
        read_jsonl(&root().join("evals/fact-cases.jsonl")).unwrap()
    );
    let lines: Vec<String> = built.iter().map(py::dumps_row).collect();
    let committed = fs::read_to_string(root().join("evals/fact-cases.jsonl")).unwrap();
    assert_eq!(
        format!("{}\n", lines.join("\n")),
        py::universal_newlines(&committed)
    );
}
