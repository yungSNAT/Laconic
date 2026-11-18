// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

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
