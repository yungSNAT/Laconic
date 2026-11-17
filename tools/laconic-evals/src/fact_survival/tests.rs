// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;
use crate::cases::{load_cases, root, validate_cases};
use crate::proc::{Failure, Output};
use crate::testing::done;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

mod files;
mod rows;
mod summary;

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
fn normalising_folds_with_python_3_15s_unicode_17_data() {
    // Unicode 18 gave these case foldings; Python 3.15 (Unicode 17) leaves each unchanged,
    // so a quote differing from its source only here must not match.
    let unicode_18 = [
        0xA7DD, 0xA7E2, 0xAB6C, 0xAB6D, 0x1DF40, 0x1DF48, 0x1DF4A, 0x1DF4D, 0x1DF51, 0x1DF68,
        0x1DF6A, 0x1DF6C, 0x1DF6E, 0x1DF72, 0x1DF74, 0x1DF76, 0x1DF78, 0x1DF7A, 0x1DF7C, 0x1DF7E,
        0x1DF95,
    ];
    for code in unicode_18 {
        let text = char::from_u32(code).unwrap().to_string();
        assert_eq!(normalise(&text), text, "U+{code:04X}");
    }
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
fn a_reply_that_ends_the_row_still_counts_its_cost() {
    // The Python script recorded 0 here, losing a paid call from the spend total.
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
    assert_eq!(total_spend(dir.path()), Ok(0.5));
    let overhead = fs::read_to_string(dir.path().join("overhead.jsonl")).unwrap();
    assert_eq!(
        overhead,
        "{\"stage\": \"failed-judge\", \"cost_usd\": 0.5}\n"
    );
}
