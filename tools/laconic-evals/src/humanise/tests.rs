// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::cases::{load_cases, root, validate_cases};
use crate::proc::{Failure, Output};
use crate::testing::{Recorder, done};
use serde_json::json;
use std::fs;

fn reply(
    code: i32,
    stdout: &'static str,
    stderr: &'static str,
) -> impl Fn(&[String], &Input) -> Result<Output, Failure> {
    move |_: &[String], _: &Input| done(code, stdout, stderr)
}

fn cases() -> BTreeMap<String, Value> {
    BTreeMap::from([(
        py::key(&json!("c1")),
        json!({"prompt": "Do it.\n\none two three four"}),
    )])
}

#[test]
fn humanise_cases_are_valid_and_every_case_has_checks() {
    let cases = load_cases(&root().join("evals/humanise-cases.jsonl")).unwrap();
    assert!(validate_cases(&cases).is_empty());
    assert!(cases.len() >= 20);
    for case in &cases {
        assert!(py::get(case, "checks").is_some(), "{}", case["id"]);
        let prompt = case["prompt"].as_str().unwrap();
        assert!(
            !py::strip(source_text(prompt).unwrap()).is_empty(),
            "{}",
            case["id"]
        );
    }
}

#[test]
fn source_text_drops_only_the_instruction_paragraph() {
    let prompt = "Humanise this.\n\nFirst paragraph.\n\nSecond paragraph.";
    assert_eq!(
        source_text(prompt),
        Ok("First paragraph.\n\nSecond paragraph.")
    );
    assert!(source_text("Only an instruction.").is_err());
}

#[test]
fn summary_reports_source_and_condition_means() {
    let rows = [
        json!({"case_id": "c1", "condition": "candidate", "response": "one two"}),
        json!({"case_id": "c1", "condition": "candidate", "response": "one two three four"}),
        json!({"case_id": "c1", "condition": "comparator", "response": "one two three four five six seven eight"}),
    ];
    let measure = |text: &str| -> Result<Metrics, String> {
        let lint = i64::try_from(text.matches("two").count()).unwrap();
        Ok(vec![("lint".to_owned(), Num::Int(lint))])
    };
    let drops = |_: &str, rewrite: &str| -> Result<i64, String> {
        Ok(4 - i64::try_from(words(rewrite).min(4)).unwrap())
    };
    let summary = summarise(&rows, &cases(), &measure, &drops).unwrap();
    // The text Python's json.dumps(summary, indent=2) printed for the same inputs.
    let expected = "{\n  \"candidate\": {\n    \"rows\": 2,\n    \"lint\": 1,\n    \"length\": 0.75,\n    \"drops\": 1\n  },\n  \
                    \"comparator\": {\n    \"rows\": 1,\n    \"lint\": 1,\n    \"length\": 2.0,\n    \"drops\": 0\n  },\n  \
                    \"source\": {\n    \"rows\": 1,\n    \"lint\": 1,\n    \"length\": 1.0,\n    \"drops\": 0\n  }\n}";
    assert_eq!(py::dumps_pretty(&summary), expected);
}

#[test]
fn summary_means_round_floats_to_3_places() {
    let rows = [
        json!({"case_id": "c1", "condition": "a", "response": "one two three"}),
        json!({"case_id": "c1", "condition": "a", "response": "x"}),
    ];
    #[expect(clippy::cast_precision_loss, reason = "short texts")]
    let measure = |text: &str| -> Result<Metrics, String> {
        Ok(vec![(
            "lint".to_owned(),
            Num::Float(text.len() as f64 / 7.0),
        )])
    };
    let summary = summarise(&rows, &cases(), &measure, &|_: &str,
                                                         _: &str|
     -> Result<i64, String> {
        Ok(1)
    })
    .unwrap();
    // Python: lint 1.0 for 'a' and 2.571 for the source.
    assert_eq!(
        py::dumps(&summary, None, true),
        r#"{"a": {"rows": 2, "lint": 1.0, "length": 0.5, "drops": 1}, "source": {"rows": 1, "lint": 2.571, "length": 1.0, "drops": 0}}"#
    );
}

#[test]
fn summary_rejects_rows_it_cannot_measure() {
    let ok = |_: &str| -> Result<Metrics, String> { Ok(vec![("lint".to_owned(), Num::Int(0))]) };
    let none = |_: &str, _: &str| -> Result<i64, String> { Ok(0) };
    let row =
        |condition: Value| json!({"case_id": "c1", "condition": condition, "response": "one"});
    let error = summarise(&[row(json!(3))], &cases(), &ok, &none).unwrap_err();
    assert_eq!(
        error,
        r#"'condition' is not text in {'case_id': 'c1', 'condition': 3, 'response': 'one'}"#
    );
    let empty = BTreeMap::from([(py::key(&json!("c1")), json!({"prompt": "Do it.\n\n "}))]);
    let error = summarise(&[row(json!("a"))], &empty, &ok, &none).unwrap_err();
    assert_eq!(error, "division by zero: a source text has no words");
    let uneven = |text: &str| -> Result<Metrics, String> {
        let name = if text == "one" { "other" } else { "lint" };
        Ok(vec![(name.to_owned(), Num::Int(0))])
    };
    let error = summarise(&[row(json!("source"))], &cases(), &uneven, &none).unwrap_err();
    assert_eq!(error, "a source row has no lint");
}

#[test]
fn checker_failures_are_errors_instead_of_counting_as_clean() {
    let failed = reply(2, "", "cannot read input\n");
    assert!(
        lint_output(&failed, "text")
            .unwrap_err()
            .ends_with("exited 2: cannot read input")
    );
    assert!(
        preserve_drops(&failed, "a", "b")
            .unwrap_err()
            .contains("exited 2")
    );
    assert_eq!(
        preserve_drops(&reply(1, "dropped number: 12\n", ""), "a 12", "a"),
        Ok(1)
    );
    let missing = Checkers {
        avoid: None,
        dslop: Some(Path::new("missing-dslop.toml")),
    };
    assert_eq!(
        missing.check().unwrap_err(),
        "dslop config not found: missing-dslop.toml"
    );
    // Exit 1 without a finding is an operational failure (uvx exits 1 when it cannot launch).
    let launch = reply(1, "", "No solution found");
    assert!(
        preserve_drops(&launch, "a", "b")
            .unwrap_err()
            .ends_with("exited 1: No solution found")
    );
    let error = run(
        &launch,
        &["uvx", "dslop", "-"],
        Some("text"),
        Some(&DSLOP_FINDING),
    )
    .unwrap_err();
    assert_eq!(error, "uvx dslop - exited 1: No solution found");
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("dslop.toml");
    fs::write(&bad, "[patterns\nbroken = \n").unwrap();
    let broken = Checkers {
        avoid: None,
        dslop: Some(&bad),
    };
    assert!(
        broken
            .check()
            .unwrap_err()
            .starts_with(&format!("{}: ", bad.display()))
    );
    let spawn = |_: &[String], _: &Input| -> Result<Output, Failure> {
        Err(Failure::Spawn("not found".into()))
    };
    assert!(lint_output(&spawn, "text").is_err());
    assert!(lint_output(&reply(0, "", ""), "text").is_ok());
}

#[test]
fn checker_output_must_be_utf8() {
    let bad = |_: &[String], _: &Input| -> Result<Output, Failure> {
        Ok(Output {
            code: 0,
            stdout: vec![0xff],
            stderr: Vec::new(),
        })
    };
    assert!(lint_output(&bad, "text").is_err());
    let bad = |_: &[String], _: &Input| -> Result<Output, Failure> {
        Ok(Output {
            code: 2,
            stdout: Vec::new(),
            stderr: vec![0xff],
        })
    };
    assert!(lint_output(&bad, "text").is_err());
}

#[test]
fn dslop_pattern_counts_positioned_findings_and_rhythm_metrics() {
    let out = "  <stdin>:1:17 em-dash\n  <stdin> sentence-length-cv=0.19 (threshold 0.30)\n\nfix:\n  em-dash: rewrite\n";
    assert_eq!(count(&DSLOP_FINDING, out), 2);
}

#[test]
fn bold_label_runs_are_counted_apart_from_other_findings() {
    let out = "-:1:5: em-dash: em dash in original prose\n-:3:1: bold-label: 3 or more bullets\n";
    let metrics = Checkers {
        avoid: None,
        dslop: None,
    }
    .measure(&reply(0, out, ""), "text")
    .unwrap();
    let expected = [
        ("lint".to_owned(), Num::Int(2)),
        ("bold_label_runs".to_owned(), Num::Int(1)),
    ];
    assert_eq!(metrics, expected);
}

#[test]
fn every_checker_gets_the_text_and_reports_its_counts() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("dslop.toml");
    fs::write(&config, "[patterns]\n").unwrap();
    let recorder = Recorder::new();
    let exec = |command: &[String], input: &Input| {
        recorder.record(command, input);
        match command[0].as_str() {
            "node" => done(0, r#"{"score": 37.5, "issues": [{}, {}]}"#, ""),
            "uvx" => done(1, "  <stdin>:1:2 em-dash\n", ""),
            _ => done(0, "", ""),
        }
    };
    let avoid = Path::new("avoid.js");
    let checkers = Checkers {
        avoid: Some(avoid),
        dslop: Some(&config),
    };
    checkers.check().unwrap();
    let metrics = checkers.measure(&exec, "text").unwrap();
    let names: Vec<&str> = metrics.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "lint",
            "bold_label_runs",
            "avoid_score",
            "avoid_findings",
            "dslop"
        ]
    );
    assert_eq!(metrics[2].1, Num::Float(37.5));
    assert_eq!(metrics[3].1, Num::Int(2));
    assert_eq!(metrics[4].1, Num::Int(1));
    let calls = recorder.calls();
    assert_eq!(
        calls[1].command,
        ["node", "avoid.js", "--context", "general"]
    );
    assert_eq!(calls[2].command[..3], ["uvx", "dslop", "--config"]);
    assert!(
        calls
            .iter()
            .all(|call| call.input.as_deref() == Some("text"))
    );
}

#[test]
fn a_detector_report_without_its_fields_is_an_error() {
    let checkers = Checkers {
        avoid: Some(Path::new("avoid.js")),
        dslop: None,
    };
    for (report, error) in [
        (r#"{"issues": []}"#, "the detector reported no score"),
        (r#"{"score": 1}"#, "the detector reported no issues list"),
        (r#"{"score": "high", "issues": []}"#, "not a number: 'high'"),
    ] {
        let exec = |command: &[String], _: &Input| {
            if command[0] == "node" {
                done(0, report, "")
            } else {
                done(0, "", "")
            }
        };
        assert_eq!(checkers.measure(&exec, "text").unwrap_err(), error);
    }
}

#[test]
fn a_detector_that_fails_stops_the_measurement() {
    for (checkers, detector) in [
        (
            Checkers {
                avoid: Some(Path::new("avoid.js")),
                dslop: None,
            },
            "node",
        ),
        (
            Checkers {
                avoid: None,
                dslop: Some(Path::new("dslop.toml")),
            },
            "uvx",
        ),
    ] {
        let exec = |command: &[String], _: &Input| {
            if command[0] == detector {
                done(2, "", "crashed")
            } else {
                done(0, "", "")
            }
        };
        let error = checkers.measure(&exec, "text").unwrap_err();
        assert!(error.starts_with(detector), "{error}");
        assert!(error.ends_with("exited 2: crashed"), "{error}");
    }
}

#[test]
fn preserve_gets_both_texts_as_files_and_no_input() {
    let recorder = Recorder::new();
    let exec = |command: &[String], input: &Input| {
        recorder.record(command, input);
        let (original, edited) = (
            fs::read_to_string(&command[2]).unwrap(),
            fs::read_to_string(&command[3]).unwrap(),
        );
        assert_eq!((original.as_str(), edited.as_str()), ("a 12", "a"));
        done(1, "dropped number: 12\ndropped name: Ann\n", "")
    };
    assert_eq!(preserve_drops(&exec, "a 12", "a"), Ok(2));
    let call = &recorder.calls()[0];
    assert_eq!(call.command[..2], ["laconic-check", "preserve"]);
    assert!(call.inherited);
}

#[test]
fn command_measures_only_rows_for_known_cases() {
    let dir = tempfile::tempdir().unwrap();
    let (cases, responses) = (
        dir.path().join("cases.jsonl"),
        dir.path().join("responses.jsonl"),
    );
    crate::testing::write_rows(
        &cases,
        &[json!({"id": "c1", "prompt": "Do it.\n\none two"})],
    );
    crate::testing::write_rows(
        &responses,
        &[
            json!({"case_id": "c1", "condition": "laconic", "response": "one"}),
            json!({"case_id": "other", "condition": "laconic", "response": "ignored"}),
        ],
    );
    let exec = |command: &[String], _: &Input| {
        if command[1] == "lint" {
            done(0, "-:1:1: em-dash: x\n", "")
        } else {
            done(0, "", "")
        }
    };
    let none = Checkers {
        avoid: None,
        dslop: None,
    };
    let printed = command(&responses, &cases, &none, &exec).unwrap();
    let summary = parse(&printed).unwrap();
    assert_eq!(
        summary["laconic"],
        json!({"rows": 1, "lint": 1, "bold_label_runs": 0, "length": 0.5, "drops": 0})
    );
    let missing = Checkers {
        avoid: None,
        dslop: Some(Path::new("missing.toml")),
    };
    assert!(command(&responses, &cases, &missing, &exec).is_err());
    assert!(command(&responses, &dir.path().join("none.jsonl"), &none, &exec).is_err());
    assert!(command(&dir.path().join("none.jsonl"), &cases, &none, &exec).is_err());
    let failing = |_: &[String], _: &Input| done(2, "", "broken");
    assert!(command(&responses, &cases, &none, &failing).is_err());
}

#[cfg(unix)]
#[test]
fn exec_runs_the_checker_with_its_input() {
    let out = exec(&["cat".to_owned()], &Input::Bytes(b"text")).unwrap();
    assert_eq!((out.code, out.stdout.as_slice()), (0, b"text".as_slice()));
}

#[test]
fn lint_reads_laconic_check_findings() {
    // Runs where laconic-check is on the PATH, as in CI.
    if crate::fact_audit::which("laconic-check").is_some() {
        let lint = |text: &str| count(&FINDING, &lint_output(&exec, text).unwrap());
        assert!(lint("We analyzed the color.\n") >= 2);
        assert_eq!(lint("We analysed the colour.\n"), 0);
    }
}
