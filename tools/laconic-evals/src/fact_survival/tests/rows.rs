// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

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

#[test]
fn every_failed_row_is_costed_and_one_failure_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let call = |_: &str| -> Result<(Value, f64), String> { Err("judge exited 1".into()) };
    let error = judge_rows(
        &rows(),
        &ledgers(),
        &sources_of(&[("c", "Source.")]),
        &dir.path().join("scores.jsonl"),
        &call,
        2,
        12.0,
    )
    .unwrap_err();
    assert!(error.ends_with(": judge exited 1"), "{error}");
    let overhead = read_jsonl(&dir.path().join("overhead.jsonl")).unwrap();
    assert_eq!(overhead.len(), 2);
    assert!(overhead.iter().all(|row| row["stage"] == "failed-judge"));
}
