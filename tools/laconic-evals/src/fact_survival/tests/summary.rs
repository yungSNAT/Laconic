// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

fn marks(pairs: &[(&str, &str)]) -> Value {
    pairs
        .iter()
        .map(|(kind, verdict)| json!({"kind": kind, "verdict": verdict, "unverified_quote": false}))
        .collect()
}

fn score(setup: &str, case: &str, trial: i64, verdicts: Value, invented: Value) -> Value {
    json!({"setup": setup, "case_id": case, "trial": trial, "verdicts": verdicts, "invented": invented})
}

fn answer(setup: &str, case: &str, trial: i64, response: &str) -> Value {
    json!({"runner": setup, "case_id": case, "trial": trial, "response": response})
}

fn no_added(_: &str, _: &str) -> Result<i64, String> {
    Ok(0)
}

#[test]
fn unequal_coverage_is_an_error_not_a_silent_intersection() {
    let kept = marks(&[("fact", "kept")]);
    let scores = [
        score("a", "c", 1, kept.clone(), json!([])),
        score("a", "d", 1, kept.clone(), json!([])),
        score("b", "c", 1, kept, json!([])),
    ];
    let responses: Vec<Value> = scores
        .iter()
        .map(|s| {
            answer(
                s["setup"].as_str().unwrap(),
                s["case_id"].as_str().unwrap(),
                1,
                "x",
            )
        })
        .collect();
    let error = summarise(
        &scores,
        &responses,
        &sources_of(&[("c", "x"), ("d", "x")]),
        "a",
        &no_added,
        1,
        None,
    );
    assert_eq!(
        error,
        Err("incomplete, duplicate or unequal coverage: b".into())
    );
}

#[test]
fn a_setup_missing_from_the_plan_is_an_error() {
    let one = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    let plan = BTreeSet::from(["a".to_owned(), "b".to_owned()]);
    let error = summarise(
        &[one],
        &[answer("a", "c", 1, "x")],
        &sources_of(&[("c", "x")]),
        "a",
        &no_added,
        1,
        Some(&plan),
    );
    assert_eq!(error, Err("setups differ from the plan: ['b']".into()));
}

#[test]
fn a_duplicate_row_or_an_unrun_trial_is_an_error() {
    let one = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    let reply = answer("a", "c", 1, "x");
    let sources = sources_of(&[("c", "x")]);
    for (scores, responses, trials) in [
        (
            vec![one.clone(), one.clone()],
            vec![reply.clone(), reply.clone()],
            1,
        ),
        (vec![one], vec![reply], 2),
    ] {
        assert!(summarise(&scores, &responses, &sources, "a", &no_added, trials, None).is_err());
    }
}

#[test]
fn rates_count_kept_only_and_gap_is_reference_minus_setup() {
    let scores = [
        score(
            "a",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "kept")]),
            json!(["x"]),
        ),
        score(
            "b",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "hardened")]),
            json!([]),
        ),
    ];
    let responses = [
        answer("a", "c", 1, "one two"),
        answer("b", "c", 1, "one two three four"),
    ];
    let summary = summarise(
        &scores,
        &responses,
        &sources_of(&[("c", "one two three four")]),
        "a",
        &no_added,
        1,
        None,
    )
    .unwrap();
    assert_eq!(summary["a"]["qualifier_survival"], 1.0);
    assert_eq!(summary["b"]["qualifier_survival"], 0.0);
    assert_eq!(summary["b"]["hardened"], 1);
    assert_eq!(summary["a"]["length_share"], 0.5);
    assert_eq!(summary["b"]["reference_gap"]["mean"], 0.5);
    assert_eq!(
        summary["a"]["survival_by_text"].as_f64().unwrap()
            - summary["b"]["survival_by_text"].as_f64().unwrap(),
        0.5
    );
    assert_eq!(summary["a"]["invented_per_text"], 1);
}

#[test]
fn the_summary_prints_as_python_printed_it() {
    let mut answers = Vec::new();
    let mut scores = Vec::new();
    for (setup, case, trial, verdicts, invented, cost) in [
        (
            "a",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "kept")]),
            json!(["x"]),
            json!(0.1),
        ),
        (
            "a",
            "c",
            2,
            marks(&[("fact", "dropped"), ("qualifier", "kept")]),
            json!([]),
            json!(0.2),
        ),
        (
            "a",
            "d",
            1,
            marks(&[("fact", "kept")]),
            json!(["y", "z"]),
            json!("0.3"),
        ),
        (
            "a",
            "d",
            2,
            marks(&[("fact", "kept")]),
            json!([]),
            json!(null),
        ),
        (
            "b",
            "c",
            1,
            marks(&[("fact", "kept"), ("qualifier", "hardened")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "c",
            2,
            marks(&[("fact", "kept"), ("qualifier", "dropped")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "d",
            1,
            marks(&[("fact", "dropped")]),
            json!([]),
            json!(0.1),
        ),
        (
            "b",
            "d",
            2,
            marks(&[("fact", "kept")]),
            json!([]),
            json!(0.1),
        ),
    ] {
        scores.push(score(setup, case, trial, verdicts, invented));
        let mut reply = answer(
            setup,
            case,
            trial,
            if setup == "a" {
                "one two"
            } else {
                "one two three"
            },
        );
        reply["cost_usd"] = cost;
        answers.push(reply);
    }
    let sources = sources_of(&[("c", "one two three four"), ("d", "one two three")]);
    let added = |_: &str, rewrite: &str| Ok(i64::try_from(rewrite.len() % 3).unwrap());
    let summary = summarise(&scores, &answers, &sources, "a", &added, 2, None).unwrap();
    // The text Python's json.dumps(summary, indent=2) printed for the same inputs.
    let python = include_str!("../summary.json");
    assert_eq!(py::dumps_pretty(&summary), python.trim_end());
}

#[test]
fn summaries_reject_malformed_scores() {
    let sources = sources_of(&[("c", "one")]);
    let reply = [answer("a", "c", 1, "one")];
    let summary = |score: Value| summarise(&[score], &reply, &sources, "a", &no_added, 1, None);
    assert_eq!(
        summary(score("b", "c", 1, marks(&[]), json!([]))),
        Err("reference setup has no scores: a".into())
    );
    assert!(summary(score("a", "c", 1, marks(&[]), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, json!({}), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, marks(&[("fact", "kept")]), json!(3))).is_err());
    let mut unflagged = score("a", "c", 1, marks(&[("fact", "kept")]), json!([]));
    unflagged["verdicts"][0]["unverified_quote"] = json!(0);
    assert!(summary(unflagged).is_err());
    let mut nameless = score("a", "c", 1, marks(&[]), json!([]));
    nameless["setup"] = json!(1);
    assert!(summary(nameless).is_err());
    assert!(summary(score("a", "c", 1, json!([{"verdict": "kept"}]), json!([]))).is_err());
    assert!(summary(score("a", "c", 1, json!([{"kind": "fact"}]), json!([]))).is_err());
    let counted = summary(score("a", "c", 1, marks(&[("fact", "kept")]), json!("ab"))).unwrap();
    assert_eq!(counted["a"]["invented_per_text"], 2);
    let counted = summary(score(
        "a",
        "c",
        1,
        marks(&[("fact", "kept")]),
        json!({"k": 1}),
    ))
    .unwrap();
    assert_eq!(counted["a"]["invented_per_text"], 1);
    let blank = sources_of(&[("c", " ")]);
    let empty = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &reply,
        &blank,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(empty.is_err());
    let failing = |_: &str, _: &str| Err("preserve exited 2".to_owned());
    let added = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &reply,
        &sources,
        "a",
        &failing,
        1,
        None,
    );
    assert_eq!(added, Err("preserve exited 2".into()));
    let mut unpriced = reply[0].clone();
    unpriced["cost_usd"] = json!([1]);
    let priced = summarise(
        &[score("a", "c", 1, marks(&[("fact", "kept")]), json!([]))],
        &[unpriced],
        &sources,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(priced.is_err());
    let runnerless = summarise(
        &[score("a", "c", 1, marks(&[]), json!([]))],
        &[json!({})],
        &sources,
        "a",
        &no_added,
        1,
        None,
    );
    assert!(runnerless.is_err());
}

#[test]
fn paired_intervals_need_shared_cases_and_rounds() {
    let a = PerCase::from([("c".to_owned(), vec![1.0])]);
    let b = PerCase::from([("d".to_owned(), vec![1.0])]);
    assert_eq!(
        paired_interval(&a, &b, 10, 0),
        Err("no shared cases".into())
    );
    assert_eq!(
        paired_interval(&a, &a, 0, 0),
        Err("list index out of range".into())
    );
    assert_eq!(paired_interval(&a, &a, 1, 0), Ok((0.0, 0.0, 0.0)));
    assert_eq!(paired_interval(&a, &a, 100, 0), Ok((0.0, 0.0, 0.0)));
}

#[test]
fn counts_generation_overhead_and_judging() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    fs::write(
        folder.join("run-a.jsonl"),
        "{\"cost_usd\": 1.5}\n{\"cost_usd\": null}\n",
    )
    .unwrap();
    fs::write(
        folder.join("overhead.jsonl"),
        "{\"stage\": \"preflight\", \"cost_usd\": 0.5}\n",
    )
    .unwrap();
    fs::write(folder.join("scores.jsonl"), "{\"cost\": 2.0}\n").unwrap();
    fs::write(folder.join("responses.jsonl"), "{\"cost_usd\": 99}\n").unwrap();
    fs::write(folder.join("run-.json"), "{\"cost_usd\": 99}\n").unwrap();
    assert_eq!(total_spend(folder), Ok(4.0));
    fs::write(folder.join("run-b.jsonl"), "[]\n").unwrap();
    assert!(total_spend(folder).is_err());
    assert_eq!(total_spend(&folder.join("missing")), Ok(0.0));
}

#[test]
fn spend_sums_each_file_as_python_did() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    // Python 3.12+: 0.1 + 0.2 + 0.3 is summed exactly within a file, 0.6000000000000001 across files.
    fs::write(
        folder.join("run-b.jsonl"),
        "{\"cost_usd\": 0.1}\n{\"cost_usd\": 0.2}\n{\"cost_usd\": 0.3}\n",
    )
    .unwrap();
    assert_eq!(total_spend(folder), Ok(0.6));
    fs::write(folder.join("run-a.jsonl"), "{\"cost_usd\": \" 1e-1 \"}\n").unwrap();
    assert_eq!(total_spend(folder), Ok(0.7));
    assert_eq!(money(0.125), "0.12");
    assert_eq!(money(f64::NAN), "nan");
}

#[test]
fn costs_read_as_python_float_reads_them() {
    assert_eq!(to_float(None), Ok(0.0));
    assert_eq!(to_float(Some(&json!(true))), Ok(1.0));
    assert_eq!(to_float(Some(&json!(""))), Ok(0.0));
    assert_eq!(to_float(Some(&json!(2))), Ok(2.0));
    assert_eq!(
        to_float(Some(&json!("x"))),
        Err("could not convert string to float: 'x'".into())
    );
    assert!(to_float(Some(&json!([1]))).is_err());
}

#[test]
fn added_counts_preserves_added_lines() {
    let exec = |command: &[String], input: &Input| {
        assert!(matches!(input, Input::Inherit));
        assert_eq!(fs::read_to_string(&command[3]).unwrap(), "rewrite");
        done(
            1,
            "added number: 4\ncheck added name: Jane\nadded name: Ann\n",
            "",
        )
    };
    assert_eq!(added_count(&exec, "source", "rewrite"), Ok(2));
    assert_eq!(
        added_count(&|_: &[String], _: &Input| done(0, "", ""), "s", "r"),
        Ok(0)
    );
    let error = added_count(&|_: &[String], _: &Input| done(2, "", "bad\n"), "s", "r");
    assert_eq!(error, Err("preserve exited 2: bad".into()));
    let missing =
        |_: &[String], _: &Input| -> Result<Output, Failure> { Err(Failure::Spawn("gone".into())) };
    assert!(added_count(&missing, "s", "r").is_err());
}

#[test]
fn a_response_without_a_trial_is_an_error() {
    let kept = marks(&[("fact", "kept")]);
    let error = summarise(
        &[score("a", "c", 1, kept, json!([]))],
        &[json!({"runner": "a", "case_id": "c", "response": "x"})],
        &sources_of(&[("c", "x")]),
        "a",
        &no_added,
        1,
        None,
    );
    assert_eq!(error, Err("a row has no 'trial'".into()));
}
