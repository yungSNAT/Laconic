// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The arithmetic and the refusals of the record steps. `src/tests/record.rs` checks their
//! output against what the Python they replace printed.

use super::*;
use crate::testing::write_rows;

#[test]
fn half_up_rounds_the_shortest_decimal_as_python_does() {
    // Python's str(Decimal(repr(x)).quantize(Decimal(1).scaleb(-places), ROUND_HALF_UP)).
    for (x, places, text) in [
        (Num::Float(2.675), 2, "2.68"),
        (Num::Float(0.125), 2, "0.13"),
        (Num::Float(-0.005), 2, "-0.01"),
        (Num::Float(-0.001), 2, "-0.00"),
        (Num::Float(9.995), 2, "10.00"),
        (Num::Float(1e-7), 2, "0.00"),
        (Num::Float(1e16), 0, "10000000000000000"),
        (Num::Int(42), 2, "42.00"),
        (Num::Float(0.5), 0, "1"),
        (Num::Float(99.5), 0, "100"),
        (Num::Float(0.0), 4, "0.0000"),
        (Num::Float(-0.0), 2, "-0.00"),
        (Num::Float(1.5e-7), 6, "0.000000"),
        (Num::Float(123.456), 1, "123.5"),
        (Num::Float(-2.5), 0, "-3"),
        (Num::Float(0.0049), 2, "0.00"),
        (Num::Float(1e22), 2, "10000000000000000000000.00"),
        (Num::Int(-7), 0, "-7"),
    ] {
        assert_eq!(half_up(x, places).unwrap(), text, "{x:?} {places}");
    }
    assert_eq!(
        half_up(Num::Float(f64::INFINITY), 2).unwrap_err(),
        "cannot round inf"
    );
}

#[test]
#[should_panic(expected = "exponent past 6 places")]
fn half_up_refuses_places_python_would_write_with_an_exponent() {
    let _ = half_up(Num::Float(1.5e-7), 7);
}

#[test]
fn sum_adds_as_cpython_does() {
    let float = |n: Num| match n {
        Num::Float(f) => f,
        Num::Int(i) => panic!("int {i}"),
    };
    assert!(matches!(sum([]), Num::Int(0)));
    assert!(matches!(sum([Num::Int(3), Num::Int(4)]), Num::Int(7)));
    assert_eq!(float(sum([Num::Int(1), Num::Float(2.5), Num::Int(3)])), 6.5);
    // Python 3.12 and later: 1.0, where adding in turn gives 0.9999999999999999.
    assert_eq!(float(sum([Num::Float(0.1); 10])), 1.0);
    assert_eq!(
        float(sum([Num::Float(0.1), Num::Float(1e10), Num::Float(0.2)])),
        10_000_000_000.3
    );
    assert_eq!(
        float(sum([
            Num::Float(1e100),
            Num::Float(1.0),
            Num::Float(-1e100)
        ])),
        1.0
    );
    assert_eq!(
        float(sum([
            Num::Float(1e308),
            Num::Float(1e308),
            Num::Float(-1e308)
        ])),
        f64::INFINITY
    );
}

#[test]
fn numbers_are_read_as_python_adds_them() {
    assert!(matches!(num(&json!(true)), Ok(Num::Int(1))));
    assert!(matches!(num(&json!(u64::MAX)), Ok(Num::Float(f)) if f == 1.8446744073709552e19));
    assert_eq!(
        num(&json!("1")).unwrap_err(),
        "unsupported operand: '1' is not a number"
    );
    assert_eq!(
        divide(Num::Int(1), Num::Float(0.0)).unwrap_err(),
        "division by zero"
    );
}

#[test]
fn dict_keys_and_counts_follow_json_dumps() {
    assert_eq!(json_key(&json!("a")).unwrap(), "a");
    assert_eq!(json_key(&json!(4)).unwrap(), "4");
    assert_eq!(json_key(&Value::Null).unwrap(), "null");
    assert_eq!(json_key(&json!(true)).unwrap(), "true");
    assert_eq!(json_key(&json!([1])).unwrap_err(), "unhashable: [1]");
    assert_eq!(
        counter([json!("b"), json!(2), json!("b")]).unwrap(),
        json!({"b": 2, "2": 1})
    );
    assert!(counter([json!({})]).is_err());
}

#[test]
fn sorting_refuses_values_python_cannot_compare() {
    assert!(sorted_set(vec![json!(1), json!("a")]).is_err());
    assert_eq!(
        sorted_set(vec![json!("b"), json!("a"), json!("b")]).unwrap(),
        vec![json!("a"), json!("b")]
    );
}

#[test]
fn regex_checks_must_be_lists_of_strings() {
    let case = |checks| json!({"id": "c", "checks": checks});
    assert_eq!(
        regex_failed(&case(json!({"must_include": "x"})), "x").unwrap_err(),
        "must_include is not a list"
    );
    assert_eq!(
        regex_failed(&case(json!({"must_not_include": [1]})), "x").unwrap_err(),
        "must_not_include holds a non-string"
    );
    // Python's re.search raises on a pattern it cannot compile, and `.get` on checks that
    // are not a dict.
    assert!(regex_failed(&case(json!({"must_not_include": ["["]})), "x").is_err());
    // A search past the backtracking limit, where Python would run on, is refused too.
    let runaway = case(json!({"must_include": [r"^(a|a)*\1b$"]}));
    assert!(regex_failed(&runaway, &"a".repeat(40)).is_err());
    assert_eq!(
        regex_failed(&case(json!("invalid")), "x").unwrap_err(),
        "checks is 'invalid'"
    );
}

#[test]
fn usage_that_is_not_a_dict_is_refused() {
    // `(r.get("usage") or {}).get(...)`: falsy usage reads as no usage; any other non-dict raises.
    assert_eq!(usage_field(Some(&json!("")), "output_tokens").unwrap(), 0);
    assert_eq!(usage_field(None, "output_tokens").unwrap(), 0);
    assert_eq!(
        usage_field(Some(&json!({"output_tokens": 7})), "output_tokens").unwrap(),
        7
    );
    assert_eq!(
        usage_field(Some(&json!("invalid")), "output_tokens").unwrap_err(),
        "usage is 'invalid'"
    );
}

#[test]
fn counts_refuse_values_json_would_write_as_one_key() {
    // json.dumps writes None and 'null' as 2 keys spelt "null"; a JSON object cannot hold both.
    assert!(counter([Value::Null, json!("null")]).is_err());
    assert!(counter([json!(1), json!("1")]).is_err());
    // 1 and 1.0 are one Counter key in Python.
    assert_eq!(counter([json!(1), json!(1.0)]).unwrap(), json!({"1": 2}));
}

#[test]
fn a_repeated_id_keeps_its_first_place_and_its_last_row() {
    let rows = vec![
        json!({"id": "a", "v": 1}),
        json!({"id": "b"}),
        json!({"id": "a", "v": 2}),
    ];
    let map = by_id(rows, id_field).unwrap();
    assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(map["a"]["v"], 2);
}

#[test]
fn an_unknown_run_or_a_path_that_is_not_text_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        setups("other", dir.path()).unwrap_err(),
        "unknown run other; expected one of concise-style-2026-11-17, token-cost-2026-11-17, rival-rewrites-2026-11-17"
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let odd = dir.path().join(std::ffi::OsStr::from_bytes(b"\xff"));
        assert!(
            setups(RUNS[0], &odd)
                .unwrap_err()
                .ends_with("not valid UTF-8")
        );
    }
}

#[test]
fn style_rows_need_their_case_and_scores() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    let answer = json!({"case_id": "c1", "trial": 1, "condition": "candidate", "runner": "r", "response": "x"});
    write_rows(&w.join("cases.jsonl"), &[json!({"id": "c2"})]);
    write_rows(&w.join("responses.jsonl"), &[answer]);
    fs::create_dir_all(w.join("lint")).unwrap();
    fs::write(w.join("lint/cold-0"), "1").unwrap();
    assert_eq!(style_results(w).unwrap_err(), "no case 'c1'");
    write_rows(&w.join("cases.jsonl"), &[json!({"id": "c1"})]);
    assert_eq!(
        style_results(w).unwrap_err(),
        r#"scores.jsonl: no score for ["c1",1,"candidate"]"#
    );
}

#[test]
fn answers_and_style_rows_refuse_what_they_cannot_read_or_write() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    let answer = json!({"case_id": "c1", "trial": 1, "condition": "candidate", "runner": "r", "response": "x"});
    write_rows(&w.join("responses.jsonl"), std::slice::from_ref(&answer));
    // A file where the answers folder goes, then a folder where the answer file goes.
    fs::write(w.join("answers"), "").unwrap();
    assert!(write_answers(w).unwrap_err().contains("answers"));
    fs::remove_file(w.join("answers")).unwrap();
    fs::create_dir_all(w.join("answers/cold-0.txt")).unwrap();
    assert!(write_answers(w).unwrap_err().contains("cold-0.txt"));

    write_rows(&w.join("cases.jsonl"), &[json!({"name": "c1"})]);
    assert_eq!(style_results(w).unwrap_err(), "a row has no 'id'");
    write_rows(&w.join("cases.jsonl"), &[json!({"id": "c1"})]);
    fs::create_dir_all(w.join("lint")).unwrap();
    fs::write(w.join("lint/cold-0"), "1").unwrap();
    let mut score = answer;
    for d in DIMENSIONS.iter().chain(&["blocker"]) {
        score[*d] = json!(1);
    }
    write_rows(&w.join("scores.jsonl"), &[score]);
    assert_eq!(
        style_results(w).unwrap_err(),
        r#"correctness.jsonl: no score for ["c1",1,"candidate"]"#
    );
}

/// A cold row of the Concise run with every field the summary reads.
fn style_row(case: &str, condition: &str) -> Value {
    json!({"run": "cold", "case_id": case, "trial": 1, "condition": condition, "setup": "s",
        "input_tokens": 1, "cache_write_tokens": 0, "cache_read_tokens": 0.5, "output_tokens": 2,
        "cost_usd": 0.1, "characters": 3, "lint_clean": true, "regex_failed": null,
        "house": {"correctness": 4, "autonomy": 4, "actionability": 4, "safety": 4,
            "concision": 4, "blocker": false},
        "correctness_only": {"correctness": 4, "blocker": false}})
}

#[test]
fn the_style_summary_refuses_rows_it_cannot_total() {
    assert_eq!(style_summary(&[]).unwrap(), "");
    let mut no_trial = style_row("c1", "candidate");
    no_trial.as_object_mut().unwrap().remove("trial");
    assert_eq!(
        style_summary(&[no_trial]).unwrap_err(),
        "a row has no 'trial'"
    );
    // The other condition has no row for c2.
    let rows = [
        style_row("c1", "candidate"),
        style_row("c2", "candidate"),
        style_row("c1", "other"),
    ];
    assert_eq!(
        style_summary(&rows).unwrap_err(),
        r#"no row for ["c2",1,"other"]"#
    );
    let mut no_condition = style_row("c1", "candidate");
    no_condition.as_object_mut().unwrap().remove("condition");
    assert_eq!(
        style_summary(&[no_condition]).unwrap_err(),
        "a row has no 'condition'"
    );
    let empty = json!({});
    assert_eq!(weighted(&empty).unwrap_err(), "a row has no 'house'");
    assert_eq!(
        total_of(&[&empty], lint_clean).unwrap_err(),
        "a row has no 'lint_clean'"
    );
    assert_eq!(
        mean_of(&[&empty], cost, 2).unwrap_err(),
        "a row has no 'cost_usd'"
    );
    assert!(matches!(add(Num::Int(1), Num::Float(0.5)), Num::Float(f) if f == 1.5));
}

#[test]
fn rewrite_steps_refuse_malformed_input() {
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path();
    write_rows(
        &w.join("aihb-2026-10.jsonl"),
        &[json!({"sample_id": "1", "input": "Text."})],
    );
    for s in MODEL_REWRITERS {
        write_rows(&w.join(format!("run-{s}.jsonl")), &[]);
    }
    fs::create_dir_all(w.join("patina-out")).unwrap();
    fs::write(w.join("patina-out/1.json"), "[]").unwrap();
    assert_eq!(
        rewrite_pairs(w).unwrap_err(),
        "patina-out/1.json is not a JSON object"
    );
    fs::write(w.join("patina-out/1.json"), r#"{"verification": "ok"}"#).unwrap();
    assert_eq!(
        rewrite_pairs(w).unwrap_err(),
        "patina-out/1.json: verification is 'ok'"
    );
    write_rows(
        &w.join("run-laconic.jsonl"),
        &[json!({"case_id": "nodash", "response": "x"})],
    );
    assert_eq!(rewrite_pairs(w).unwrap_err(), "case id 'nodash' has no '-'");
    write_rows(
        &w.join("run-laconic.jsonl"),
        &[json!({"case_id": "aihb-2026-10-2", "response": "x"})],
    );
    assert_eq!(rewrite_pairs(w).unwrap_err(), "no source text '2'");
    // Valid input, but a file where the preserve folders go.
    write_rows(&w.join("run-laconic.jsonl"), &[]);
    fs::write(w.join("patina-out/1.json"), "{}").unwrap();
    fs::write(w.join("preserve"), "").unwrap();
    assert!(rewrite_pairs(w).unwrap_err().contains("patina-1"));
    fs::remove_file(w.join("preserve")).unwrap();

    write_rows(
        &w.join("units-laconic.jsonl"),
        &[json!({"item_id": "1", "units": {}})],
    );
    write_rows(&w.join("run-laconic.jsonl"), &[]);
    assert_eq!(rewrite_results(w).unwrap_err(), "units is {}");
    write_rows(&w.join("units-laconic.jsonl"), &[json!({"item_id": "1"})]);
    assert_eq!(
        rewrite_results(w).unwrap_err(),
        "pairs-laconic.jsonl has no '1'"
    );
    write_rows(&w.join("units-laconic.jsonl"), &[]);
    write_rows(&w.join("units-patina.jsonl"), &[json!({"item_id": "1"})]);
    write_rows(
        &w.join("pairs-patina.jsonl"),
        &[json!({"id": "1", "source": "a b", "rewrite": "a"})],
    );
    fs::create_dir_all(w.join("preserve/patina-1")).unwrap();
    fs::write(w.join("preserve/patina-1/fails"), "0").unwrap();
    write_rows(&w.join("patina-verify.jsonl"), &[]);
    assert_eq!(
        rewrite_results(w).unwrap_err(),
        "patina-verify.jsonl has no '1'"
    );
}

#[test]
fn rewrite_summary_refuses_rows_it_cannot_total() {
    let row = json!({"rewriter": "patina", "sample_id": "1", "units": 1, "units_altered": 0,
        "altered_groups": [], "preserve_fails": false, "source_words": 2, "rewrite_words": 1});
    let with = |field: &str, value: Value| {
        let mut r = row.clone();
        r[field] = value;
        r
    };
    assert_eq!(
        rewrite_summary(&[with("source_words", json!(0))]).unwrap_err(),
        "division by zero"
    );
    let mut altered = with("units_altered", json!(1));
    altered.as_object_mut().unwrap().remove("sample_id");
    assert_eq!(
        rewrite_summary(&[altered]).unwrap_err(),
        "a row has no 'sample_id'"
    );
    assert_eq!(
        rewrite_summary(&[with("cost_usd", json!("x"))]).unwrap_err(),
        "unsupported operand: 'x' is not a number"
    );
    // A cost of 0 or none counts as 0; patina's rows then need their --verify result.
    assert_eq!(
        rewrite_summary(&[with("cost_usd", json!(0.5)), with("cost_usd", Value::Null)])
            .unwrap_err(),
        "a row has no 'patina_verify'"
    );
}

#[test]
fn rewrite_summary_needs_lists_of_groups() {
    let row = json!({"rewriter": "laconic", "sample_id": "1", "units": 1, "units_altered": 1,
        "altered_groups": "number", "preserve_fails": false, "source_words": 2, "rewrite_words": 1});
    assert_eq!(
        rewrite_summary(&[row]).unwrap_err(),
        "'number' is not a list"
    );
}
