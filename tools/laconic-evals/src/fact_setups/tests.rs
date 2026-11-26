// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::testing::{Recorder, done};
use std::sync::Mutex;

/// A stand-in for git: a clone makes the folders each rival's skill lives in.
fn fake_git(calls: &Mutex<Vec<Vec<String>>>) -> impl Fn(&[String]) -> Result<(), String> + '_ {
    move |command: &[String]| {
        calls.lock().unwrap().push(command.to_vec());
        if command[1] == "clone" {
            let target = Path::new(&command[6]);
            for inner in [
                "",
                "plugins/avoid-ai-writing",
                "plugins/caveman/skills/caveman",
            ] {
                fs::create_dir_all(target.join(inner).join(".git")).unwrap();
                fs::write(target.join(inner).join("SKILL.md"), "skill").unwrap();
            }
        }
        Ok(())
    }
}

#[test]
fn setup_fetches_each_rival_once_and_writes_the_runner_config() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path().join("facts");
    let calls = Mutex::new(Vec::new());
    let git = fake_git(&calls);
    let names = setup(&here, "https://example.test", &git).unwrap();
    assert_eq!(
        names,
        "laconic-prose\nlaconic-session\nhumanizer\navoid\nstop-slop\ncaveman\nconcise-control\nno-skill"
    );
    drop(git);
    let calls = calls.into_inner().unwrap();
    assert_eq!(calls.len(), 8);
    let target = here.join("src").join("humanizer").display().to_string();
    assert_eq!(
        calls[0],
        strings(&[
            "git",
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            "https://example.test/blader/humanizer",
            &target
        ])
    );
    assert_eq!(
        calls[1],
        strings(&[
            "git",
            "-C",
            &target,
            "-c",
            "core.longpaths=true",
            "checkout",
            "-q",
            RIVALS[0].2
        ])
    );
    let setups = here.join("setups");
    assert_eq!(
        fs::read_to_string(setups.join("humanizer-plugin/.claude-plugin/plugin.json")).unwrap(),
        r#"{"name": "humanizer", "version": "0.0.0"}"#
    );
    assert!(
        setups
            .join("humanizer-plugin/skills/humanizer/SKILL.md")
            .is_file()
    );
    assert!(
        !setups
            .join("humanizer-plugin/skills/humanizer/.git")
            .exists()
    );
    assert!(
        setups
            .join("laconic-prose-plugin/skills/laconic-prose/SKILL.md")
            .is_file()
    );
    assert!(
        setups
            .join("caveman-plugin/skills/caveman/SKILL.md")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(setups.join("avoid-system.md")).unwrap(),
        "For any request to rewrite, humanise or edit text, use the avoid-ai-writing skill.\n"
    );
    let config = parse(&fs::read_to_string(here.join("runners.json")).unwrap()).unwrap();
    let avoid = &config["avoid"];
    assert_eq!(avoid["response_format"], "claude-json");
    assert_eq!(avoid["budget_flag"], "--max-budget-usd");
    let folder = here
        .join("src")
        .join("avoid")
        .join("plugins")
        .join("avoid-ai-writing")
        .display()
        .to_string();
    let prompt = setups.join("avoid-system.md").display().to_string();
    let mut expected = strings(&BASE);
    expected.extend(strings(&[
        "--plugin-dir",
        &folder,
        "--add-dir",
        &folder,
        "--tools",
        "Skill,Read",
        "--allowedTools",
        "Skill",
        "--append-system-prompt-file",
        &prompt,
    ]));
    assert_eq!(avoid["command"], json!(expected));
    assert_eq!(
        config["no-skill"]["command"],
        json!([strings(&BASE), strings(&["--tools", ""])].concat())
    );
    let control = config["concise-control"]["command"].as_array().unwrap();
    assert!(
        control
            .last()
            .unwrap()
            .as_str()
            .unwrap()
            .ends_with("terse.md")
    );
    // A second run keeps the clones and rebuilds the setups.
    let again = Mutex::new(Vec::new());
    setup(&here, "https://example.test", &fake_git(&again)).unwrap();
    assert!(again.into_inner().unwrap().is_empty());
}

#[test]
fn setup_errors_stop_it() {
    let dir = tempfile::tempdir().unwrap();
    let failing = |_: &[String]| -> Result<(), String> { Err("git failed".to_owned()) };
    assert_eq!(
        setup(&dir.path().join("a"), "x", &failing),
        Err("git failed".into())
    );
    let none = |_: &[String]| Ok(());
    assert!(
        setup(&dir.path().join("b"), "x", &none).is_err(),
        "a clone without the rival's skill"
    );
    let file = dir.path().join("file");
    fs::write(&file, "").unwrap();
    assert!(setup(&file, "x", &none).is_err());
    fs::create_dir_all(dir.path().join("c")).unwrap();
    fs::write(dir.path().join("c").join("src"), "").unwrap();
    assert!(setup(&dir.path().join("c"), "x", &none).is_err());
    let calls = Mutex::new(Vec::new());
    let git = fake_git(&calls);
    let here = dir.path().join("d");
    setup(&here, "x", &git).unwrap();
    fs::remove_file(here.join("runners.json")).unwrap();
    fs::create_dir(here.join("runners.json")).unwrap();
    assert!(setup(&here, "x", &git).is_err());
}

#[test]
fn plugins_and_runners_refuse_to_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("skill");
    fs::create_dir_all(skill.join("nested")).unwrap();
    fs::write(skill.join("nested").join("a.md"), "a").unwrap();
    plugin(dir.path(), "p", "s", &skill).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("p-plugin/skills/s/nested/a.md")).unwrap(),
        "a"
    );
    assert!(plugin(dir.path(), "p", "s", &skill).is_err());
    fs::remove_dir_all(dir.path().join("p-plugin/.claude-plugin")).unwrap();
    assert!(plugin(dir.path(), "p", "s", &skill).is_err());
    assert!(plugin(dir.path(), "q", "s", &dir.path().join("missing")).is_err());
    fs::write(dir.path().join("r-plugin"), "").unwrap();
    assert!(plugin(dir.path(), "r", "s", &skill).is_err());
    let missing = dir.path().join("none");
    assert!(runners(&missing, &[("a", missing.clone(), "a")]).is_err());
}

#[test]
fn copying_names_a_folder_it_cannot_make() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("file"), "").unwrap();
    let to = dir.path().join("file/copy");
    let error = copy_tree(dir.path(), &to).unwrap_err();
    assert!(error.starts_with(&to.display().to_string()), "{error}");
}

#[cfg(unix)]
#[test]
fn copying_reports_a_link_to_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let skill = dir.path().join("skill");
    fs::create_dir_all(&skill).unwrap();
    std::os::unix::fs::symlink(dir.path().join("gone"), skill.join("link")).unwrap();
    assert!(plugin(dir.path(), "q", "s", &skill).is_err());
}

#[test]
fn skills_used_reads_skill_calls_from_the_stream() {
    let stream = [
        r#"{"type":"system"}"#,
        "not json",
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "plugin:laconic-prose"}}]}}"#,
        r#"{"message": {"content": [{"type": "tool_use", "name": "Read", "input": {}}, "text"]}}"#,
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {}}]}}"#,
        r#"{"message": {"content": "text"}}"#,
        r#"{"message": null}"#,
        r#"  {"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "indented"}}]}}"#,
    ]
    .join("\n");
    assert_eq!(
        skills_used(&stream),
        Ok(BTreeSet::from([
            "laconic-prose".to_owned(),
            "indented".to_owned(),
            String::new()
        ]))
    );
    for bad in [
        "{broken",
        "  {broken",
        r#"{"message": 3}"#,
        r#"{"message": {"content": 3}}"#,
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill"}]}}"#,
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": "x"}]}}"#,
    ] {
        assert!(skills_used(bad).is_err(), "{bad}");
    }
}

fn write_config(here: &Path, config: &Value) {
    fs::create_dir_all(here).unwrap();
    fs::write(here.join("runners.json"), config.to_string()).unwrap();
}

#[test]
fn preflight_calls_each_skill_setup_and_reports_what_it_called() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    write_config(
        here,
        &json!({
            "laconic-prose": {"command": ["claude", "--output-format", "json", "--plugin-dir", "p"]},
            "humanizer": {"command": ["claude", "--plugin-dir", "h"]},
            "no-skill": {"command": ["claude", "--tools", ""]},
        }),
    );
    fs::write(
        here.join("overhead.jsonl"),
        "{\"stage\": \"calibrate\", \"cost_usd\": 1.0}\n",
    )
    .unwrap();
    let recorder = Recorder::new();
    let long = "x".repeat(450);
    let stream = format!(
        "{}\n{{\"type\":\"result\",\"result\":\"{long}\",\"total_cost_usd\":0.25}}\n{{\"type\":\"result\",\"total_cost_usd\":\"0.5\"}}\n",
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "laconic-prose"}}]}}"#
    );
    let fake = |command: &[String], input: &Input, timeout: Duration| {
        recorder.record(command, input);
        assert_eq!(timeout, Duration::from_secs(600));
        if command[1] == "--plugin-dir" {
            done(0, "{\"type\":\"result\",\"result\":[1]}\n", "")
        } else {
            done(0, &stream, "")
        }
    };
    let mut printed = Vec::new();
    assert_eq!(preflight(here, &fake, &mut printed), Ok(1));
    let calls = recorder.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].command,
        strings(&[
            "claude",
            "--output-format",
            "stream-json",
            "--verbose",
            "--plugin-dir",
            "p"
        ])
    );
    let prompt = calls[0].input.as_deref().unwrap_or_default();
    assert!(prompt.starts_with("Rewrite this text for a busy Australian reader."));
    let printed = String::from_utf8(printed).unwrap();
    let first = format!(
        "laconic-prose: wanted laconic-prose, called ['laconic-prose']\n{}\n\n",
        "x".repeat(400)
    );
    assert_eq!(
        printed,
        format!("{first}humanizer: wanted humanizer, called []\n[1]\n\n")
    );
    let overhead = fs::read_to_string(here.join("overhead.jsonl")).unwrap();
    assert!(
        overhead.ends_with("{\"stage\": \"preflight\", \"cost_usd\": 0.75}\n"),
        "{overhead}"
    );
}

#[test]
fn preflight_passes_when_every_setup_calls_its_skill() {
    let dir = tempfile::tempdir().unwrap();
    write_config(
        dir.path(),
        &json!({"caveman": {"command": ["claude", "--plugin-dir", "c"]}}),
    );
    let fake = |_: &[String], _: &Input, _: Duration| {
        done(
            0,
            r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "caveman"}}]}}"#,
            "",
        )
    };
    let mut printed = Vec::new();
    assert_eq!(preflight(dir.path(), &fake, &mut printed), Ok(0));
    assert_eq!(
        String::from_utf8(printed).unwrap(),
        "caveman: wanted caveman, called ['caveman']\n\n\n"
    );
    let mut closed: &mut [u8] = &mut [];
    assert_eq!(
        preflight(dir.path(), &fake, &mut closed),
        Err("failed to write whole buffer".into())
    );
}

#[test]
fn preflight_reads_result_events_whatever_their_spacing_or_key_order() {
    let dir = tempfile::tempdir().unwrap();
    write_config(
        dir.path(),
        &json!({"avoid": {"command": ["claude", "--plugin-dir", "a"]}}),
    );
    let stream = [
        r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "avoid-ai-writing"}}]}}"#,
        "progress text",
        r#"{"subtype": "success", "type": "result", "result": "Done.", "total_cost_usd": 0.5}"#,
        r#"  {"type" : "result", "total_cost_usd": 0.25}"#,
        r#"{"type": "assistant", "result": "not a result"}"#,
    ]
    .join("\n");
    let fake = |_: &[String], _: &Input, _: Duration| done(0, &stream, "");
    let mut printed = Vec::new();
    assert_eq!(preflight(dir.path(), &fake, &mut printed), Ok(0));
    assert_eq!(
        String::from_utf8(printed).unwrap(),
        "avoid: wanted avoid-ai-writing, called ['avoid-ai-writing']\nDone.\n\n"
    );
    let overhead = fs::read_to_string(dir.path().join("overhead.jsonl")).unwrap();
    assert!(
        overhead.ends_with("{\"stage\": \"preflight\", \"cost_usd\": 0.75}\n"),
        "{overhead}"
    );
}

#[test]
fn preflight_rejects_an_unsuccessful_runner_and_records_complete_costs() {
    let dir = tempfile::tempdir().unwrap();
    write_config(
        dir.path(),
        &json!({"avoid": {"command": ["claude", "--plugin-dir", "a"]}}),
    );
    let stream = concat!(
        r#"{"message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"avoid-ai-writing"}}]}}"#,
        "\n",
        r#"{"type":"result","total_cost_usd":0.75}"#
    );
    for tail in [b"".as_slice(), b"\n{\"type\":", b"\n{\"result\":\"\xc3"] {
        let partial = |_: &[String], _: &Input, _: Duration| {
            let mut output = done(3, stream, "")?;
            output.stdout.extend_from_slice(tail);
            Ok(output)
        };
        let mut printed = Vec::new();
        assert_eq!(preflight(dir.path(), &partial, &mut printed), Ok(1));
        assert!(
            String::from_utf8(printed)
                .unwrap()
                .contains("avoid: runner exited with status 3")
        );
    }
    let overhead = fs::read_to_string(dir.path().join("overhead.jsonl")).unwrap();
    assert_eq!(overhead.lines().count(), 3);
    assert!(
        overhead
            .lines()
            .all(|line| line.ends_with("\"cost_usd\": 0.75}"))
    );
}

#[test]
fn preflight_fails_a_setup_with_no_known_skill_and_checks_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    write_config(
        dir.path(),
        &json!({
            "mine": {"command": ["claude", "--plugin-dir", "m"]},
            "caveman": {"command": ["claude", "--plugin-dir", "c"]},
        }),
    );
    let recorder = Recorder::new();
    let fake = |command: &[String], input: &Input, _: Duration| {
        recorder.record(command, input);
        done(
            0,
            r#"{"message": {"content": [{"type": "tool_use", "name": "Skill", "input": {"skill": "caveman"}}]}}"#,
            "",
        )
    };
    let mut printed = Vec::new();
    assert_eq!(preflight(dir.path(), &fake, &mut printed), Ok(1));
    // The unknown setup is not run, so it costs nothing.
    assert_eq!(recorder.calls().len(), 1);
    assert_eq!(
        String::from_utf8(printed).unwrap(),
        "mine: no skill is known for this setup\n\ncaveman: wanted caveman, called ['caveman']\n\n\n"
    );
    // A closed output stops it at the unknown setup's line, before any call.
    let mut closed: &mut [u8] = &mut [];
    assert_eq!(
        preflight(dir.path(), &fake, &mut closed),
        Err("failed to write whole buffer".into())
    );
    assert_eq!(recorder.calls().len(), 1);
}

#[test]
fn preflight_errors_stop_it() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    let quiet = |_: &[String], _: &Input, _: Duration| done(0, "", "");
    let run = |config: Value, exec: Exec| {
        write_config(here, &config);
        preflight(here, exec, &mut Vec::new())
    };
    assert!(preflight(&here.join("missing"), &quiet, &mut Vec::new()).is_err());
    assert!(run(json!([]), &quiet).is_err());
    assert!(run(json!({"a": {}}), &quiet).is_err());
    assert!(run(json!({"a": {"command": "x"}}), &quiet).is_err());
    let gone =
        |_: &[String], _: &Input, _: Duration| -> Result<Output, Failure> { Err(Failure::Timeout) };
    assert!(
        run(
            json!({"avoid": {"command": ["claude", "--plugin-dir"]}}),
            &gone
        )
        .is_err()
    );
    let garbled = |_: &[String], _: &Input, _: Duration| -> Result<Output, Failure> {
        Ok(Output {
            code: 0,
            stdout: vec![0xff],
            stderr: Vec::new(),
        })
    };
    assert!(
        run(
            json!({"avoid": {"command": ["claude", "--plugin-dir"]}}),
            &garbled
        )
        .is_err()
    );
    let bad_stream = |_: &[String], _: &Input, _: Duration| done(0, "{broken", "");
    assert!(
        run(
            json!({"avoid": {"command": ["claude", "--plugin-dir"]}}),
            &bad_stream
        )
        .is_err()
    );
    let bad_result =
        |_: &[String], _: &Input, _: Duration| done(0, "{\"type\":\"result\", broken", "");
    assert!(
        run(
            json!({"avoid": {"command": ["claude", "--plugin-dir"]}}),
            &bad_result
        )
        .is_err()
    );
    let bad_cost = |_: &[String], _: &Input, _: Duration| {
        done(0, "{\"type\":\"result\",\"total_cost_usd\":\"x\"}", "")
    };
    assert!(
        run(
            json!({"avoid": {"command": ["claude", "--plugin-dir"]}}),
            &bad_cost
        )
        .is_err()
    );
    fs::create_dir_all(here.join("overhead.jsonl")).unwrap();
    assert!(run(json!({}), &quiet).is_err());
}

#[cfg(unix)]
#[test]
fn check_call_runs_commands_and_reports_failures() {
    assert_eq!(check_call(&strings(&["true"])), Ok(()));
    let error = check_call(&strings(&["sh", "-c", "exit 3"])).unwrap_err();
    assert_eq!(
        error,
        "Command ['sh', '-c', 'exit 3'] returned exit status: 3"
    );
    assert!(check_call(&strings(&["/nonexistent/command"])).is_err());
    let out = exec(
        &strings(&["cat"]),
        &Input::Bytes(b"x"),
        Duration::from_secs(30),
    )
    .unwrap();
    assert_eq!(out.stdout, b"x");
}

#[test]
fn folders_resolve_to_absolute_paths() {
    let resolved = resolve(Path::new("results")).unwrap();
    assert!(resolved.is_absolute() && resolved.ends_with("results"));
    assert!(resolve(Path::new("")).is_err());
}
