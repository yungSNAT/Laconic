// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::num::round;
use super::*;
use serde_json::json;

#[test]
fn float_repr_matches_python() {
    for (x, text) in [
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (1.0, "1.0"),
        (0.1, "0.1"),
        (1e-4, "0.0001"),
        (1e-5, "1e-05"),
        (1.5e-7, "1.5e-07"),
        (1e15, "1000000000000000.0"),
        (1e16, "1e+16"),
        (1.2345e20, "1.2345e+20"),
        (123.456, "123.456"),
        (-2.5, "-2.5"),
        (5e-324, "5e-324"),
        (1.7976931348623157e308, "1.7976931348623157e+308"),
        (f64::NAN, "nan"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
    ] {
        assert_eq!(float_repr(x), text, "{x}");
    }
}

#[test]
fn dumps_matches_python_spacing_and_escapes() {
    let value =
        json!({"a": [1, 2.0, "é\u{1f600}\n\u{7f}"], "b": {}, "c": [], "d": null, "e": true});
    assert_eq!(
        dumps_row(&value),
        "{\"a\": [1, 2.0, \"é\u{1f600}\\n\u{7f}\"], \"b\": {}, \"c\": [], \"d\": null, \"e\": true}"
    );
    assert_eq!(
        dumps_pretty(&value),
        "{\n  \"a\": [\n    1,\n    2.0,\n    \"\\u00e9\\ud83d\\ude00\\n\\u007f\"\n  ],\n  \"b\": {},\n  \"c\": [],\n  \"d\": null,\n  \"e\": true\n}"
    );
    assert_eq!(
        dumps(&json!("\u{8}\u{c}\t\r\"\\\u{1}"), None, true),
        "\"\\b\\f\\t\\r\\\"\\\\\\u0001\""
    );
}

#[test]
fn repr_matches_python() {
    assert_eq!(
        repr(&json!([null, true, 1, 1.5, "it's", "a\"b'c", {"k": "v\n"}])),
        "[None, True, 1, 1.5, \"it's\", 'a\"b\\'c', {'k': 'v\\n'}]"
    );
    assert_eq!(str_repr("\u{1}\u{85}x\\\t\r"), "'\\x01\\x85x\\\\\\t\\r'");
    assert_eq!(to_str(&json!("plain")), "plain");
    assert_eq!(to_str(&json!(2)), "2");
}

#[test]
fn truthiness_matches_python() {
    for (value, expected) in [
        (json!(null), false),
        (json!(false), false),
        (json!(0), false),
        (json!(0.0), false),
        (json!(""), false),
        (json!([]), false),
        (json!({}), false),
        (json!(true), true),
        (json!(2), true),
        (json!("x"), true),
        (json!([0]), true),
        (json!({"k": 0}), true),
    ] {
        assert_eq!(truthy(&value), expected, "{value}");
    }
    assert_eq!(get(&json!([1]), "k"), None);
}

#[test]
fn compare_orders_as_python_sorted_does() {
    assert_eq!(compare(&json!("a"), &json!("b")), Ok(Ordering::Less));
    assert_eq!(compare(&json!(2), &json!(10)), Ok(Ordering::Less));
    assert_eq!(compare(&json!(1.5), &json!(1)), Ok(Ordering::Greater));
    assert_eq!(compare(&json!(true), &json!(1)), Ok(Ordering::Equal));
    assert_eq!(
        compare(&json!(["a", 2]), &json!(["a", 10])),
        Ok(Ordering::Less)
    );
    assert_eq!(compare(&json!(["a"]), &json!(["a", 1])), Ok(Ordering::Less));
    assert!(
        compare(&json!("a"), &json!(1))
            .unwrap_err()
            .contains("not supported")
    );
}

#[test]
fn keys_equal_where_python_values_are_equal() {
    assert_eq!(key(&json!(1)), key(&json!(1.0)));
    assert_ne!(key(&json!(1)), key(&json!(1.5)));
    assert_eq!(key(&json!(1.5)), "1.5");
    assert_eq!(key(&json!(["a", 2.0])), key(&json!(["a", 2])));
    assert_eq!(key(&json!(1e300)), "1e+300");
    assert_eq!(key(&json!(true)), key(&json!(1)));
    assert_eq!(key(&json!(-0.0)), key(&json!(0)));
}

#[test]
fn text_helpers_match_python() {
    assert_eq!(
        words(" a\u{1c}b\u{a0}c ").collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(strip("\u{2028} x \u{1f}"), "x");
    assert_eq!(
        split_lines("a\r\nb\rc\u{2028}d\u{85}e\n"),
        ["a", "b", "c", "d", "e"]
    );
    assert_eq!(universal_newlines("a\r\nb\rc\n"), "a\nb\nc\n");
}

#[test]
fn sum_uses_neumaier_compensation() {
    // Naive left-to-right addition gives 0.0 here; CPython 3.12 and later give 2.0.
    assert_eq!(sum([1.0, 1e100, 1.0, -1e100]), 2.0);
    assert_eq!(sum([0.1; 10]), 1.0);
    assert_eq!(sum([]), 0.0);
    assert!(sum([-0.0]).is_sign_positive(), "0 + -0.0 is 0.0");
    assert_eq!(sum([1e308, 1e308]), f64::INFINITY);
}

#[test]
fn mean_is_exact_and_keeps_ints() {
    assert_eq!(mean([Num::Int(1), Num::Int(3)]), Some(Num::Int(2)));
    assert_eq!(mean([Num::Int(1), Num::Int(2)]), Some(Num::Float(1.5)));
    assert_eq!(mean([Num::Int(2), Num::Float(2.0)]), Some(Num::Float(2.0)));
    assert_eq!(mean([]), None);
    // The exact mean of 3 tenths is 0.1 as written, which naive summation misses.
    assert_eq!(mean_f64([0.1, 0.1, 0.1]), Some(0.1));
    assert_eq!(mean_f64([1e308, 1e308]), Some(1e308));
    assert_eq!(mean_f64([-0.5, -0.25]), Some(-0.375));
    // Half the smallest float is a tie, which rounds to the even 0.0, as Python gives.
    assert_eq!(mean_f64([5e-324, 0.0]), Some(0.0));
    assert_eq!(mean_f64([5e-324, 5e-324, 0.0]), Some(5e-324));
    assert_eq!(mean_f64([-0.0]), Some(0.0));
    assert_eq!(
        mean([Num::Int(i64::MAX), Num::Int(i64::MAX)]),
        Some(Num::Int(i64::MAX))
    );
    assert_eq!(Num::Float(2.5).as_f64(), 2.5);
    assert_eq!(Num::Int(2).to_value(), json!(2));
    assert_eq!(float(f64::NAN), Value::Null);
}

#[test]
fn round_is_half_even_on_the_exact_value() {
    assert_eq!(round(0.125, 2), 0.12);
    assert_eq!(round(0.375, 2), 0.38);
    assert_eq!(round(2.675, 2), 2.67, "2.675 is stored below the half");
    assert_eq!(round(1.0 / 3.0, 3), 0.333);
    assert!(round(f64::NAN, 3).is_nan());
    assert_eq!(round_num(Num::Int(7), 3), Num::Int(7));
    assert_eq!(round_num(Num::Float(0.5555), 3), Num::Float(0.555));
}

#[test]
fn random_matches_python_seeded_with_zero() {
    // Python: r = random.Random(0); [r.getrandbits(32) for _ in range(3)]
    let mut random = Random::new(0);
    let draws: Vec<u32> = (0..3).map(|_| random.next_u32()).collect();
    assert_eq!(draws, [3_626_764_237, 1_654_615_998, 3_255_389_356]);
    // Python: x = list(range(10)); random.Random(0).shuffle(x)
    let mut items: Vec<u32> = (0..10).collect();
    Random::new(0).shuffle(&mut items);
    assert_eq!(items, [7, 8, 1, 5, 3, 4, 2, 0, 9, 6]);
    // Python: r = random.Random(0); [r.choice('abcde') for _ in range(5)]
    let mut random = Random::new(0);
    let letters = ['a', 'b', 'c', 'd', 'e'];
    let picks: String = (0..5).map(|_| *random.choice(&letters)).collect();
    assert_eq!(picks, "ddace");
}

#[test]
fn a_row_reads_back_as_one_line() {
    // `split_lines` ends lines at these too, so a raw one would split a row on resume.
    let value = json!({"response": "a\u{85}b\u{2028}c\u{2029}d"});
    let row = dumps_row(&value);
    let expected = r#"{"response": "a_0085b_2028c_2029d"}"#.replace('_', "\\u");
    assert_eq!(row, expected);
    assert_eq!(split_lines(&row), [row.as_str()]);
    assert_eq!(serde_json::from_str::<Value>(&row).unwrap(), value);
}

#[test]
fn object_keeps_insertion_order() {
    let value = object([("b", json!(1)), ("a", json!(2))]);
    assert_eq!(dumps_row(&value), "{\"b\": 1, \"a\": 2}");
}
