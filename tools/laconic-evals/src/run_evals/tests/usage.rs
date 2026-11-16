// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

#[test]
fn usage_requires_a_separate_reference_and_valid_model_metadata() {
    let row = usage_row("probe", "candidate", "102", json!(0.1), json!({}));
    let mut blank = row.clone();
    blank["model"] = json!(" ");
    fails(
        summarize_usage(std::slice::from_ref(&row), "candidate"),
        "reference must be a condition other than candidate",
    );
    fails(
        summarize_usage(&[row], "baseline"),
        "include baseline and candidate",
    );
    fails(
        summarize_usage(&[blank], "baseline"),
        "model must be a non-empty string",
    );
}

#[test]
fn usage_summary_reports_token_and_cost_deltas() {
    let rows = [
        usage_row(
            "direct-answer",
            "baseline",
            "abcdefghij",
            json!(1.0),
            json!({"input_tokens": 100, "cache_creation_input_tokens": 20,
                "cache_read_input_tokens": 10, "output_tokens": 80}),
        ),
        usage_row(
            "medical-boundary",
            "baseline",
            "abcdef",
            json!(0.5),
            json!({"input_tokens": 50, "output_tokens": 20}),
        ),
        usage_row(
            "direct-answer",
            "candidate",
            "abcd",
            json!(0.6),
            json!({"input_tokens": 110, "cached_input_tokens": 10, "output_tokens": 40}),
        ),
        usage_row(
            "medical-boundary",
            "candidate",
            "wxyz",
            json!(0.3),
            json!({"input_tokens": 40, "output_tokens": 10}),
        ),
    ];
    let summary = summarize_usage(&rows, "baseline").unwrap();
    assert_eq!(summary["runner"], "claude");
    assert_eq!(summary["conditions"]["baseline"]["input_tokens"], 180);
    assert_eq!(summary["conditions"]["candidate"]["output_tokens"], 50);
    near(
        25.0,
        &summary["conditions"]["candidate"]["mean_output_tokens"],
    );
    let delta = &summary["delta"]["candidate"];
    assert_eq!(delta["output_tokens"], -50);
    near(-50.0, &delta["output_tokens_pct"]);
    near(-0.6, &delta["cost_usd"]);
    near(-40.0, &delta["cost_usd_pct"]);
    near(-50.0, &delta["mean_response_chars_pct"]);
}

#[test]
fn usage_summary_rejects_mixed_runners() {
    let mut rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "b", Value::Null, json!({})),
    ];
    rows[1]["runner"] = json!("codex");
    fails(
        summarize_usage(&rows, "baseline"),
        "Responses must use the same runner; found: claude, codex",
    );
}

#[test]
fn usage_summary_rejects_unpaired_conditions() {
    let rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("medical-boundary", "baseline", "b", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "c", json!(0.1), json!({})),
    ];
    fails(
        summarize_usage(&rows, "baseline"),
        "not judged on the same rows",
    );
}

#[test]
fn usage_summary_marks_unreported_cost() {
    let rows = [
        usage_row("direct-answer", "baseline", "a", json!(0.1), json!({})),
        usage_row("direct-answer", "candidate", "b", Value::Null, json!({})),
    ];
    let summary = summarize_usage(&rows, "baseline").unwrap();
    let candidate = &summary["conditions"]["candidate"];
    assert!(candidate["input_tokens"].is_null());
    assert!(candidate["mean_input_tokens"].is_null());
    assert!(candidate["output_tokens"].is_null());
    assert!(candidate["cost_usd"].is_null());
    assert_eq!(candidate["cost_usd_unreported_rows"], 1);
    assert!(summary["delta"]["candidate"]["cost_usd"].is_null());
    assert!(summary["delta"]["candidate"]["cost_usd_pct"].is_null());
}

#[test]
fn usage_tokens_handles_both_runner_shapes() {
    assert_eq!(
        usage_tokens(
            &json!({"input_tokens": 10, "cache_creation_input_tokens": 2,
            "cache_read_input_tokens": 3, "output_tokens": 4})
        ),
        Ok((Some(15), Some(4)))
    );
    assert_eq!(
        usage_tokens(&json!({"input_tokens": 12, "cached_input_tokens": 5, "output_tokens": 7})),
        Ok((Some(12), Some(7)))
    );
    assert_eq!(usage_tokens(&json!({})), Ok((None, None)));
}

#[test]
fn usage_rejects_invalid_measurements() {
    // JSON has no NaN or infinity: such costs stop at reading the file.
    for cost in [json!(-1), json!(true), json!(false), json!("0.1")] {
        let rows =
            ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", cost.clone(), json!({})));
        fails(summarize_usage(&rows, "baseline"), "cost_usd");
    }
    for key in [
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
        "cached_input_tokens",
    ] {
        for value in [json!(-1), json!(true), json!(1.5), json!("10")] {
            fails(usage_tokens(&json!({ key: value })), key);
        }
    }
    fails(usage_tokens(&json!([])), "usage");
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!([])));
    fails(
        summarize_usage(&rows, "baseline"),
        "usage must be an object or null",
    );
}

#[test]
fn usage_rejects_overflowing_totals() {
    let rows: Vec<Value> = ["a", "b"]
        .iter()
        .flat_map(|case| {
            ["baseline", "candidate"].map(|c| usage_row(case, c, "ok", json!(1e308), json!({})))
        })
        .collect();
    fails(
        summarize_usage(&rows, "baseline"),
        "cost total is not finite",
    );
    let huge = json!({"input_tokens": u64::MAX, "output_tokens": 0});
    let rows: Vec<Value> = ["a", "b"]
        .iter()
        .flat_map(|case| {
            ["baseline", "candidate"].map(|c| usage_row(case, c, "ok", json!(0), huge.clone()))
        })
        .collect();
    fails(summarize_usage(&rows, "baseline"), "is too large");
    let mut rows =
        ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(1e-300), json!({})));
    rows[1]["cost_usd"] = json!(1e300);
    fails(
        summarize_usage(&rows, "baseline"),
        "Out of range float values are not JSON compliant",
    );
    let big = json!({"input_tokens": u64::MAX, "output_tokens": 0});
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), big.clone()));
    assert_eq!(
        summarize_usage(&rows, "baseline").unwrap()["conditions"]["candidate"]["input_tokens"],
        json!(u64::MAX)
    );
}

#[test]
fn usage_distinguishes_missing_counts_from_zero() {
    assert_eq!(usage_tokens(&Value::Null), Ok((None, None)));
    assert_eq!(
        usage_tokens(&json!({"cache_read_input_tokens": 10, "output_tokens": 2})),
        Ok((None, Some(2)))
    );
    assert_eq!(
        usage_tokens(&json!({"input_tokens": 10, "cache_read_input_tokens": null,
            "output_tokens": 2})),
        Ok((None, Some(2)))
    );
    let mut rows = ["baseline", "candidate"].map(|c| {
        usage_row(
            "a",
            c,
            "",
            json!(0),
            json!({"input_tokens": 0, "output_tokens": 0}),
        )
    });
    let result = summarize_usage(&rows, "baseline").unwrap();
    assert_eq!(result["conditions"]["candidate"]["input_tokens"], 0);
    assert_eq!(result["delta"]["candidate"]["cost_usd"].as_f64(), Some(0.0));
    assert!(result["delta"]["candidate"]["cost_usd_pct"].is_null());
    rows[1]["usage"] = Value::Null;
    let result = summarize_usage(&rows, "baseline").unwrap();
    assert!(result["delta"]["candidate"]["input_tokens"].is_null());
    rows[1].as_object_mut().unwrap().remove("usage");
    assert!(
        summarize_usage(&rows, "baseline").unwrap()["delta"]["candidate"]["input_tokens"].is_null()
    );
}

#[test]
fn usage_rejects_duplicate_or_invalid_rows() {
    let rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!({})));
    let mut repeated = rows.to_vec();
    repeated.push(rows[0].clone());
    fails(summarize_usage(&repeated, "baseline"), "duplicate");
    for (field, value, expected) in [
        ("condition", json!("unknown"), "unsupported condition"),
        ("case_id", Value::Null, "case_id must be"),
        ("trial", json!(true), "trial must be"),
        ("trial", json!(0), "trial must be"),
        ("runner", json!(""), "runner must be"),
        ("response", Value::Null, "response must be"),
    ] {
        let mut invalid = rows.clone();
        invalid[1][field] = value;
        fails(summarize_usage(&invalid, "baseline"), expected);
    }
}

#[test]
fn usage_checks_model_metadata_when_available() {
    let mut rows = ["baseline", "candidate"].map(|c| usage_row("a", c, "ok", json!(0), json!({})));
    assert!(summarize_usage(&rows, "baseline").unwrap()["model"].is_null());
    rows[0]["model"] = json!("model-a");
    fails(summarize_usage(&rows, "baseline"), "same model");
    rows[1]["model"] = json!("model-b");
    fails(summarize_usage(&rows, "baseline"), "same model");
    rows[1]["model"] = json!("model-a");
    assert_eq!(
        summarize_usage(&rows, "baseline").unwrap()["model"],
        "model-a"
    );
}
