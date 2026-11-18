// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn judge_resumes_and_prints_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let (responses, output) = (
        dir.path().join("responses.jsonl"),
        dir.path().join("scores.jsonl"),
    );
    let case = first_case();
    write_rows(
        &responses,
        &[
            json!({"runner": "a", "case_id": case, "trial": 1, "response": "x"}),
            json!({"runner": "b", "case_id": case, "trial": 1, "response": "y"}),
        ],
    );
    // A finished row ends its line, as the command writes it.
    let finished = json!({"setup": "a", "case_id": case, "trial": 1, "cost": 0.5});
    fs::write(&output, format!("{finished}\n")).unwrap();
    let timeouts = Mutex::new(Vec::new());
    let judge = |_: &str, timeout: Duration| {
        timeouts.lock().unwrap().push(timeout);
        Ok((Value::from(r#"{"items": [], "invented": []}"#), 0.125))
    };
    let tools = Tools {
        judge: &judge,
        ..NONE
    };
    let args = [
        "fact-survival",
        "judge",
        &arg(&responses),
        "--output",
        &arg(&output),
        "--call-timeout",
        "7.5",
    ];
    assert_eq!(call(&args, &tools), (Ok(0), "judge cost: $0.12\n".into()));
    assert_eq!(*timeouts.lock().unwrap(), [Duration::from_millis(7500)]);
    assert!(fails_on_a_closed_output(&args, &tools));
    let down = |_: &str, _: Duration| Err("judge down".to_owned());
    let broken = Tools {
        judge: &down,
        ..NONE
    };
    let fresh = arg(&dir.path().join("fresh.jsonl"));
    let (status, _) = call(
        &[
            "fact-survival",
            "judge",
            &arg(&responses),
            "--output",
            &fresh,
        ],
        &broken,
    );
    assert!(
        matches!(&status, Err(Stop::Failed(m)) if m.contains("judge down")),
        "{status:?}"
    );
    let (status, _) = call(
        &[
            "fact-survival",
            "judge",
            "r",
            "--output",
            "o",
            "--call-timeout=-1",
        ],
        &NONE,
    );
    assert!(matches!(status, Err(Stop::Usage(_))));
    let (status, _) = call(
        &[
            "fact-survival",
            "judge",
            &arg(&dir.path().join("none")),
            "--output",
            "o",
        ],
        &NONE,
    );
    assert!(matches!(status, Err(Stop::Failed(_))));
}

#[test]
fn calibrate_prints_its_result_and_can_record_its_cost() {
    let dir = tempfile::tempdir().unwrap();
    let fixtures = dir.path().join("fixtures.jsonl");
    let case = first_case();
    write_rows(
        &fixtures,
        &[
            json!({"name": "copy", "case_id": case, "rewrite": "@source", "default": "dropped", "expected": {}, "invented": false}),
        ],
    );
    let judge = |_: &str, timeout: Duration| {
        assert_eq!(timeout, Duration::from_secs(300));
        Ok((Value::from(r#"{"items": [], "invented": []}"#), 0.25))
    };
    let tools = Tools {
        judge: &judge,
        ..NONE
    };
    let overhead = dir.path().join("overhead.jsonl");
    let (status, printed) = call(
        &[
            "fact-survival",
            "calibrate",
            "--fixtures",
            &arg(&fixtures),
            "--overhead",
            &arg(&overhead),
        ],
        &tools,
    );
    assert_eq!(status, Ok(0));
    assert!(printed.starts_with("{\n  \"compared\": "), "{printed}");
    assert!(printed.ends_with("  \"cost_usd\": 0.25\n}\n"), "{printed}");
    assert_eq!(
        fs::read_to_string(&overhead).unwrap(),
        "{\"stage\": \"calibrate\", \"cost_usd\": 0.25}\n"
    );
    let (status, _) = call(
        &["fact-survival", "calibrate", "--fixtures", &arg(&fixtures)],
        &tools,
    );
    assert_eq!(status, Ok(0));
    let down = |_: &str, _: Duration| Err("judge down".to_owned());
    let broken = Tools {
        judge: &down,
        ..NONE
    };
    let (status, _) = call(
        &["fact-survival", "calibrate", "--fixtures", &arg(&fixtures)],
        &broken,
    );
    assert_eq!(status, Err(Stop::Failed("judge down".into())));
    let jammed = dir.path().join("jammed");
    fs::create_dir(&jammed).unwrap();
    let (status, _) = call(
        &[
            "fact-survival",
            "calibrate",
            "--fixtures",
            &arg(&fixtures),
            "--overhead",
            &arg(&jammed),
        ],
        &tools,
    );
    assert!(status.is_err());
}

#[test]
fn spend_exits_1_once_the_cap_is_reached() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("scores.jsonl"), "{\"cost\": 2.5}\n").unwrap();
    let folder = arg(dir.path());
    assert_eq!(
        call(&["fact-survival", "spend", &folder, "--cap", "3"], &NONE),
        (Ok(0), "reported cost: $2.50 of $3.00\n".into())
    );
    assert_eq!(
        call(&["fact-survival", "spend", &folder, "--cap", "2.5"], &NONE),
        (Ok(1), "reported cost: $2.50 of $2.50\n".into())
    );
    assert!(fails_on_a_closed_output(
        &["fact-survival", "spend", &folder, "--cap", "3"],
        &NONE
    ));
    fs::write(dir.path().join("scores.jsonl"), "[]\n").unwrap();
    assert!(
        call(&["fact-survival", "spend", &folder, "--cap", "3"], &NONE)
            .0
            .is_err()
    );
}

#[test]
fn report_summarises_every_planned_setup() {
    let dir = tempfile::tempdir().unwrap();
    let (cases, scores, responses) = (
        dir.path().join("cases.jsonl"),
        dir.path().join("scores.jsonl"),
        dir.path().join("responses.jsonl"),
    );
    write_rows(&cases, &[json!({"id": "c", "prompt": "Do.\n\none two"})]);
    let marks = json!([{"kind": "fact", "verdict": "kept", "unverified_quote": false}]);
    let setups = fact_survival::SETUPS;
    write_rows(
        &scores,
        &setups.map(
            |s| json!({"setup": s, "case_id": "c", "trial": 1, "verdicts": marks, "invented": []}),
        ),
    );
    write_rows(
        &responses,
        &setups.map(|s| json!({"runner": s, "case_id": "c", "trial": 1, "response": "one"})),
    );
    let checker = |command: &[String], _: &Input| {
        assert_eq!(command[..2], ["laconic-check", "preserve"]);
        done(0, "added number: 3\n", "")
    };
    let tools = Tools {
        checker: &checker,
        ..NONE
    };
    let args = [
        "fact-survival",
        "report",
        &arg(&scores),
        &arg(&responses),
        "--cases",
        &arg(&cases),
        "--reference",
        "laconic-prose",
        "--trials",
        "1",
    ];
    let (status, printed) = call(&args, &tools);
    assert_eq!(status, Ok(0));
    let summary = crate::cases::parse(&printed).unwrap();
    assert_eq!(summary.as_object().unwrap().len(), 8);
    assert_eq!(summary["no-skill"]["added_per_text"], 1);
    let (status, _) = call(
        &[
            "fact-survival",
            "report",
            &arg(&scores),
            &arg(&responses),
            "--cases",
            &arg(&cases),
            "--reference",
            "x",
        ],
        &tools,
    );
    assert!(status.is_err());
}
