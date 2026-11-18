// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn labels_are_a_stable_bijection_over_conditions() {
    let conditions = ["candidate", "baseline"];
    let first = assign_labels(&json!("direct-answer"), &json!(1), &conditions);
    assert_eq!(
        first,
        assign_labels(&json!("direct-answer"), &json!(1), &conditions)
    );
    let named: Vec<&str> = first.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(named, ["baseline", "candidate"]);
    let mut given: Vec<&str> = first.iter().map(|(_, l)| l.as_str()).collect();
    given.sort_unstable();
    assert_eq!(given, ["A", "B"]);
}

#[test]
fn labels_vary_across_groups_so_position_never_leaks_condition() {
    let seen: BTreeSet<String> = (0..20)
        .map(|trial| {
            assign_labels(
                &json!("direct-answer"),
                &json!(trial),
                &["baseline", "candidate"],
            )[0]
            .1
            .clone()
        })
        .collect();
    assert_eq!(seen, BTreeSet::from(["A".to_owned(), "B".to_owned()]));
}

#[test]
fn responses_group_by_case_and_trial_with_their_dates() {
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "response": "old", "date": "2026-10-19"}),
        json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate", "response": "102", "date": ""}),
        json!({"case_id": "direct-answer", "trial": 2, "condition": "baseline", "response": "again"}),
        json!({"case_id": "direct-answer", "trial": 1.0, "condition": "baseline", "response": "51"}),
    ];
    let groups = group_responses(&rows).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(
        groups[0].responses,
        pairs(&[("baseline", "51"), ("candidate", "102")])
    );
    assert_eq!(groups[0].dates, pairs(&[("baseline", "2026-10-19")]));
    assert_eq!(groups[1].responses, pairs(&[("baseline", "again")]));
    assert!(groups[1].dates.is_empty());
    fails(
        group_responses(&[json!({"trial": 1})]).map(|g| g.len()),
        "'case_id'",
    );
    fails(
        group_responses(&[json!({"case_id": "a", "trial": 1, "condition": 2})]).map(|g| g.len()),
        "'condition' is not text",
    );
    fails(
        group_responses(&[json!({"case_id": "a", "trial": 1, "condition": "baseline"})])
            .map(|g| g.len()),
        "'response'",
    );
}

#[test]
fn labels_are_mapped_back_to_their_conditions() {
    let payload = json!({"A": verdict(5, false), "B": verdict(2, true)}).to_string();
    let rows = parse_judge_scores(
        &payload,
        &json!("direct-answer"),
        &json!(1),
        &labels(&[("baseline", "B"), ("candidate", "A")]),
    )
    .unwrap();
    assert_eq!(
        rows,
        [
            json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline",
                "correctness": 2, "autonomy": 2, "actionability": 2, "safety": 2, "concision": 2,
                "blocker": true, "notes": "fixture"}),
            json!({"case_id": "direct-answer", "trial": 1, "condition": "candidate",
                "correctness": 5, "autonomy": 5, "actionability": 5, "safety": 5, "concision": 5,
                "blocker": false, "notes": "fixture"}),
        ]
    );
}

#[test]
fn verdicts_that_break_the_contract_name_their_case() {
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let parse = |payload: &str, labels: &Labels| {
        parse_judge_scores(payload, &json!("direct-answer"), &json!(1), labels)
    };
    let value = |payload: Value, labels: &Labels| match parse(&payload.to_string(), labels) {
        Err(Fault::Value(message)) => message,
        other => panic!("{other:?}"),
    };
    let mut wrong = verdict(4, false);
    wrong["autonomy"] = json!("4");
    assert_eq!(
        value(
            json!({"A": verdict(9, false), "B": verdict(3, false)}),
            &both
        ),
        "direct-answer/trial 1/label A: correctness must be between 1 and 5, got 9"
    );
    assert_eq!(
        value(json!({"A": wrong, "B": verdict(3, false)}), &both),
        "direct-answer/trial 1/label A: autonomy must be a number, got '4'"
    );
    let mut lax = verdict(4, false);
    lax["blocker"] = json!("false");
    assert!(
        value(json!({"A": lax}), &labels(&[("candidate", "A")]))
            .contains("blocker must be boolean, got 'false'")
    );
    assert_eq!(
        value(json!({"A": verdict(4, false)}), &both),
        "direct-answer/trial 1/label B: judge returned no verdict for label B"
    );
    assert!(value(json!(["C"]), &both).contains("no verdict for label B"));
    assert!(value(json!("C"), &both).contains("no verdict for label B"));
    assert!(matches!(parse("not json", &both), Err(Fault::Value(_))));
}

#[test]
fn verdicts_python_could_not_index_are_other_faults() {
    let both = labels(&[("baseline", "B")]);
    let other =
        |payload: Value| parse_judge_scores(&payload.to_string(), &json!("c"), &json!(1), &both);
    assert_eq!(
        other(json!(["B"])),
        Err(Fault::Other(
            "list indices must be integers or slices, not str".into()
        ))
    );
    assert_eq!(
        other(json!("B")),
        Err(Fault::Other(
            "string indices must be integers, not 'str'".into()
        ))
    );
    assert_eq!(
        other(json!(3)),
        Err(Fault::Other(
            "argument of type 'int' is not iterable".into()
        ))
    );
    assert_eq!(
        other(json!({"B": []})),
        Err(Fault::Other("'list' object has no attribute 'get'".into()))
    );
}

#[test]
fn notes_default_to_empty_and_print_as_python_would() {
    let mut quiet = verdict(4, false);
    quiet.as_object_mut().unwrap().remove("notes");
    let mut numbered = verdict(4, false);
    numbered["notes"] = json!(7);
    let payload = json!({"A": quiet, "B": numbered}).to_string();
    let rows = parse_judge_scores(
        &payload,
        &json!("c"),
        &json!(1),
        &labels(&[("baseline", "A"), ("candidate", "B")]),
    )
    .unwrap();
    assert_eq!(
        (&rows[0]["notes"], &rows[1]["notes"]),
        (&json!(""), &json!("7"))
    );
}

#[test]
fn json_wrapped_in_code_fences_is_still_parsed() {
    let body = json!({"A": verdict(4, false), "B": verdict(4, false)});
    let payload = format!("```json\n{body}\n```");
    let rows = parse_judge_scores(
        &payload,
        &json!("direct-answer"),
        &json!(1),
        &labels(&[("baseline", "B"), ("candidate", "A")]),
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(strip_code_fence("```json\n{}"), "{}");
    assert_eq!(strip_code_fence("```"), "");
    assert_eq!(strip_code_fence(" {} "), "{}");
}

#[test]
fn only_the_marked_grader_section_is_extracted() {
    let rubric = "# Response quality rubric\n<!-- judge:begin -->\nScore each dimension 1-5.\n\
                  <!-- judge:end -->\nRelease the candidate only when it beats baseline.\n";
    assert_eq!(grader_rubric(rubric), "Score each dimension 1-5.");
    assert_eq!(grader_rubric("everything\n"), "everything");
    assert_eq!(
        grader_rubric("<!-- judge:end --> a <!-- judge:begin --> b "),
        "b"
    );
    assert_eq!(
        grader_rubric("x <!-- judge:begin --> y"),
        "x <!-- judge:begin --> y"
    );
}

#[test]
fn shipped_rubric_grader_section_never_names_a_condition() {
    let shipped = read_text(&root().join("evals/rubric.md")).unwrap();
    let section = grader_rubric(&shipped).to_lowercase();
    assert!(!section.contains("baseline"));
    assert!(!section.contains("candidate"));
    assert!(section.contains("correctness"));
}

#[test]
fn prompt_carries_responses_under_labels_and_never_names_conditions() {
    let responses = pairs(&[
        ("baseline", "Great question! The answer is 102."),
        ("candidate", "102"),
    ]);
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let prompt =
        build_judge_prompt(&case(), &responses, &both, "SCORE 1-5 PER DIMENSION", &[]).unwrap();
    assert!(prompt.contains("Great question! The answer is 102."));
    assert!(prompt.contains("SCORE 1-5 PER DIMENSION"));
    assert!(prompt.contains("- States 102."));
    assert!(!prompt.to_lowercase().contains("baseline"));
    assert!(!prompt.to_lowercase().contains("candidate"));
    assert!(!prompt.contains("written on"));
    assert!(prompt.find("## Response A").unwrap() < prompt.find("## Response B").unwrap());
}

#[test]
fn prompt_dates_each_response_so_today_is_judged_against_it() {
    // A run that crosses midnight leaves conditions with different 'today's.
    let responses = pairs(&[
        ("baseline", "Yesterday was 18 October."),
        ("candidate", "Yesterday was 19 October."),
    ]);
    let both = labels(&[("baseline", "B"), ("candidate", "A")]);
    let dates = pairs(&[("baseline", "2026-10-19"), ("candidate", "20261020")]);
    let prompt = build_judge_prompt(&case(), &responses, &both, "RUBRIC", &dates).unwrap();
    assert!(prompt.contains("## Response A (written on 20 October 2026)"));
    assert!(prompt.contains("## Response B (written on 19 October 2026)"));
    assert!(prompt.contains("not your own"));
}

#[test]
fn prompts_fail_where_python_did() {
    let one = labels(&[("candidate", "A")]);
    let text = pairs(&[("candidate", "x")]);
    let build = |case: &Value, responses: &[(String, Value)], dates: &[(String, Value)]| {
        build_judge_prompt(case, responses, &one, "R", dates)
    };
    assert_eq!(
        build(&case(), &pairs(&[("baseline", "x")]), &[]),
        Err("'baseline'".into())
    );
    assert_eq!(
        build(&case(), &text, &pairs(&[("baseline", "2026-01-01")])),
        Err("'baseline'".into())
    );
    assert_eq!(build(&json!({}), &text, &[]), Err("'prompt'".into()));
    assert_eq!(
        build(&json!({"prompt": 1}), &text, &[]),
        Err("'int' object has no attribute 'strip'".into())
    );
    let unwritten = vec![("candidate".to_owned(), Value::Null)];
    assert_eq!(
        build(&case(), &unwritten, &[]),
        Err("'NoneType' object has no attribute 'strip'".into())
    );
    let dated = vec![("candidate".to_owned(), json!(5))];
    assert_eq!(
        build(&case(), &text, &dated),
        Err("fromisoformat: argument must be str, not int".into())
    );
    let criteria = |value: Value| {
        let mut case = case();
        case["criteria"] = value;
        build(&case, &text, &[])
    };
    assert!(criteria(json!("ab")).unwrap().contains("- a\n- b\n"));
    assert!(criteria(json!({"k": 1})).unwrap().contains("- k\n"));
    assert!(criteria(json!([1])).unwrap().contains("- 1\n"));
    assert_eq!(
        criteria(json!(3)),
        Err("'int' object is not iterable".into())
    );
    let mut bare = case();
    bare.as_object_mut().unwrap().remove("criteria");
    assert!(
        build(&bare, &text, &[])
            .unwrap()
            .contains("must do\n\n\n## Response A")
    );
}

#[test]
fn long_dates_follow_python_fromisoformat() {
    let date = |text: &str| long_date(&json!(text));
    assert_eq!(date("2026-10-20"), Ok("20 October 2026".into()));
    assert_eq!(date("00010101"), Ok("1 January 0001".into()));
    assert_eq!(date("2024-02-29"), Ok("29 February 2024".into()));
    assert_eq!(date("2000-02-29"), Ok("29 February 2000".into()));
    assert_eq!(date("2026-12-31"), Ok("31 December 2026".into()));
    for bad in ["1900-02-29", "2026-02-29", "2026-04-31", "2026-01-00"] {
        assert_eq!(
            date(bad),
            Err("day is out of range for month".into()),
            "{bad}"
        );
    }
    for bad in [
        "2026/10/20",
        "2026-1-020",
        "202610",
        "２０２６-10-20",
        "0000-01-01",
        "2026-13-01",
        "2026-00-01",
        "20x61020",
        "+0261020",
    ] {
        assert_eq!(
            date(bad),
            Err(format!("Invalid isoformat string: {}", py::str_repr(bad))),
            "{bad}"
        );
    }
    assert_eq!(
        long_date(&json!(null)),
        Err("fromisoformat: argument must be str, not NoneType".into())
    );
}

#[test]
fn judge_errors_keep_python_s_split_between_skips_and_stops() {
    assert_eq!(
        JudgeError::from(Fault::Value("v".into())),
        JudgeError::Skip("v".into())
    );
    assert_eq!(
        JudgeError::from(Fault::Other("o".into())),
        JudgeError::Fatal("o".into())
    );
    assert_eq!(JudgeError::Skip("s".into()).message(), "s");
    assert_eq!(JudgeError::Fatal("f".into()).message(), "f");
}
