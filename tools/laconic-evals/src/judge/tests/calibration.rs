// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

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
            &reply(json!([{}]), json!([1])),
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
