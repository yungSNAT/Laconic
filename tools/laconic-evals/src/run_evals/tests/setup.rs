// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

#[test]
fn condition_prompt_injects_the_skill_body_without_frontmatter() {
    // hooks/always-on.sh strips the YAML frontmatter before injecting the ruleset, so the
    // eval must grade the same text that actually ships.
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    fs::write(
        &skill,
        "---\nname: laconic\ndisable-model-invocation: true\nmetadata:\n  hermes:\n    \
         tags: [ADHD]\n---\n\n# laconic\n\nLead with the next action.\n",
    )
    .unwrap();
    let prompt = condition_prompt("Fix the bug.", "candidate", Some(&skill), true).unwrap();
    assert!(prompt.contains("Lead with the next action."));
    assert!(prompt.contains("<task>\nFix the bug.\n</task>"));
    assert!(!prompt.contains("disable-model-invocation"));
    assert!(!prompt.contains("hermes"));
}

#[test]
fn condition_prompt_keeps_a_skill_body_that_has_no_frontmatter() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("SKILL.md");
    fs::write(
        &skill,
        "# No frontmatter here\n\nLead with the next action.\n",
    )
    .unwrap();
    let prompt = condition_prompt("Fix the bug.", "candidate", Some(&skill), true).unwrap();
    assert!(prompt.contains("# No frontmatter here"));
    assert_eq!(
        condition_prompt("Fix the bug.", "baseline", Some(&skill), true).unwrap(),
        "Fix the bug."
    );
}

#[test]
fn setup_comparison_sends_the_task_unchanged_and_injection_needs_a_skill() {
    assert_eq!(
        condition_prompt("Task", "candidate", None, false).unwrap(),
        "Task"
    );
    assert_eq!(
        condition_prompt("Task", "candidate", None, true),
        Err("--condition-skill is required for the candidate condition".into())
    );
}

#[test]
fn completed_keys_support_resuming_partial_runs() {
    let rows = [
        json!({"case_id": "direct-answer", "trial": 1, "condition": "baseline", "runner": "claude"}),
        json!({"case_id": "direct-answer", "trial": 2, "condition": "baseline"}),
    ];
    let expected: HashSet<_> = [(
        "direct-answer".to_owned(),
        1,
        "baseline".to_owned(),
        "claude".to_owned(),
    )]
    .into();
    assert_eq!(completed_keys(&rows), expected);
}

#[test]
fn runner_configs_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("runners.json");
    let load = |config: Value| {
        fs::write(&path, config.to_string()).unwrap();
        load_runner(&path, "stub").map(|r| (r.command, r.format, r.budget_flag))
    };
    fails(load(json!({})), "no runner 'stub' in");
    fails(load(json!({"stub": {}})), "'command'");
    for command in [json!([]), json!("x"), json!([1])] {
        fails(
            load(json!({"stub": {"command": command}})),
            "runner 'stub': command must be a non-empty list of strings",
        );
    }
    fails(
        load(json!({"stub": {"command": ["x"], "budget_flag": 1}})),
        "runner 'stub': budget_flag must be a string",
    );
    assert_eq!(
        load(json!({"stub": {"command": ["x"], "budget_flag": ""}})),
        Ok((vec!["x".to_owned()], json!("text"), None))
    );
}

#[test]
fn format_g_and_shell_quote_follow_python() {
    let cases = [
        (f64::NAN, "nan"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
        (0.0, "0"),
        (-0.0, "-0"),
        (0.5, "0.5"),
        (300.0, "300"),
        (0.0001, "0.0001"),
        (1e-5, "1e-05"),
        (123_456.0, "123456"),
        (1e6, "1e+06"),
        (123_456_789.0, "1.23457e+08"),
        (2.5e-7, "2.5e-07"),
    ];
    for (x, expected) in cases {
        assert_eq!(format_g(x), expected, "{x}");
    }
    assert_eq!(shell_quote(""), "''");
    assert_eq!(shell_quote("a-b/c.d"), "a-b/c.d");
    assert_eq!(shell_quote("it's"), "'it'\"'\"'s'");
}

#[test]
fn neutral_directories_are_fresh_and_outside_the_repository() {
    // An agent CLI adopts its working directory as project context.
    let first = neutral_cwd("eval-neutral-cwd-").unwrap();
    let place = first.path().canonicalize().unwrap();
    assert!(!place.starts_with(root().canonicalize().unwrap()));
    assert_eq!(fs::read_dir(&place).unwrap().count(), 0);
    fs::write(place.join("state-from-prior-run"), "not reusable").unwrap();
    let second = neutral_cwd("eval-neutral-cwd-").unwrap();
    assert_ne!(second.path(), first.path());
    assert_eq!(fs::read_dir(second.path()).unwrap().count(), 0);
    assert!(neutral_cwd("no-such-folder/eval-").is_err());
}

#[test]
fn validate_and_plan_report_the_catalogue() {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases.jsonl");
    fs::write(&cases, "{}\n").unwrap();
    let mut out = Vec::new();
    assert_eq!(validate(&cases, &mut out), Ok(1));
    assert!(out.is_empty());
    fails(plan(&cases, 1, false), "Case 1: missing fields");
    let shipped = root().join("evals/cases.jsonl");
    assert_eq!(validate(&shipped, &mut out), Ok(0));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Evaluation cases are valid.\n"
    );
    let lines = plan(&shipped, 1, false).unwrap();
    let first = lines.lines().next().unwrap();
    assert_eq!(
        first,
        r#"{"case_id": "direct-answer", "trial": 1, "condition": "baseline"}"#
    );
    assert!(!lines.contains("comparator"));
}
