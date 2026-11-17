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
