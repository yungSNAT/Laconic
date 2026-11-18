// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::testing::{Recorder, done};
use serde_json::json;
use std::sync::Mutex;

/// A minimal correct fix per task, proving each benchmark task can be solved.
#[cfg(unix)]
const FIXES: [(&str, &str, &str, &str); 5] = [
    (
        "pagination",
        "app/pages.py",
        "start = number * size",
        "start = (number - 1) * size",
    ),
    (
        "gst-rounding",
        "app/gst.py",
        "return round(Decimal(amount) * Decimal('0.1'), 2)",
        "return (Decimal(amount) * Decimal('0.1')).quantize(Decimal('0.01'), rounding='ROUND_HALF_UP')",
    ),
    ("au-dates", "app/dates.py", "'%m/%d/%Y'", "'%d/%m/%Y'"),
    (
        "none-guard",
        "app/names.py",
        "return ''.join(",
        "return '' if not name else ''.join(",
    ),
    (
        "needle-in-large-file",
        "app/helpers.py",
        "return gross - fee",
        "return max(gross - fee, 0)",
    ),
];

#[cfg(unix)]
#[test]
fn every_task_starts_failing_and_its_fix_passes() {
    let names: Vec<&str> = TASKS.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, FIXES.map(|(task, ..)| task));
    let python = python(&crate::fact_audit::which).unwrap();
    for (task, rel, old, new) in FIXES {
        let dir = tempfile::tempdir().unwrap();
        build(task, dir.path()).unwrap();
        assert!(!tests_pass(&python, dir.path(), &run_in).unwrap(), "{task}");
        let path = dir.path().join(rel);
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches(old).count(), 1, "{task}");
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, text.replace(old, new)).unwrap();
        // The fix keeps the file's time, as an edit within the same second does; au-dates'
        // also keeps its size, so bytecode cached by the first run would still look current.
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(modified).unwrap();
        assert!(tests_pass(&python, dir.path(), &run_in).unwrap(), "{task}");
    }
}

#[test]
fn python_is_python3_then_python() {
    let only = |name: &str| (name == "python").then(|| PathBuf::from("py"));
    assert_eq!(python(&only), Ok(PathBuf::from("py")));
    assert!(python(&|_| None).is_err());
}

#[test]
fn the_fixtures_are_the_ones_python_built() {
    let (_, files) = &TASKS[4];
    let helpers = &files[0].1;
    assert!(helpers.starts_with(
        "def helper_1(value):\n    \"\"\"Return value scaled by 1.\"\"\"\n    return value * 1\n\n"
    ));
    assert!(helpers.contains("def helper_159(value):"));
    assert!(!helpers.contains("def helper_160("));
    assert_eq!(
        build("missing", Path::new(".")),
        Err("no task missing".into())
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file");
    fs::write(&file, "").unwrap();
    let (task, files) = &TASKS[0];
    let error = build(task, &file).unwrap_err();
    let first = file.join(&files[0].0);
    assert!(
        error.starts_with(&format!("{}: ", first.display())),
        "{error}"
    );
}

#[test]
fn main_reports_a_closed_standard_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    let before = std::cell::Cell::new(true);
    let tests = |_: &[String], _: &Input, _: &Path, _: Duration| {
        done(i32::from(before.replace(!before.get())), "", "")
    };
    let session = |_: &Path, _: &str, _: &Path| Ok(json!({"turns": 1}));
    let mut closed: &mut [u8] = &mut [];
    let error = command(
        &args(&out),
        Some(Path::new("claude")),
        Path::new("python3"),
        &session,
        &tests,
        &mut closed,
    );
    assert_eq!(error, Err("failed to write whole buffer".into()));
    assert_eq!(fs::read_to_string(&out).unwrap().lines().count(), 1);
}

fn args(out: &Path) -> Vec<String> {
    ["a.md", "b.md", "1"]
        .map(str::to_owned)
        .into_iter()
        .chain([out.display().to_string()])
        .collect()
}

fn no_session(_: &Path, _: &str, _: &Path) -> Result<Value, String> {
    panic!("no session expected")
}

fn no_run(_: &[String], _: &Input, _: &Path, _: Duration) -> Result<Output, Failure> {
    panic!("no command expected")
}

#[test]
fn main_stops_without_the_claude_cli() {
    let error = command(
        &args(Path::new("out.jsonl")),
        None,
        Path::new("python3"),
        &no_session,
        &no_run,
        &mut Vec::new(),
    );
    assert_eq!(
        error,
        Err("agent-bench: the claude CLI is not on PATH".into())
    );
    let error = command(
        &[],
        Some(Path::new("claude")),
        Path::new("python3"),
        &no_session,
        &no_run,
        &mut Vec::new(),
    );
    assert_eq!(error, Err(USAGE.into()));
    let mut bad = args(Path::new("out.jsonl"));
    bad[2] = "two".into();
    let error = command(
        &bad,
        Some(Path::new("claude")),
        Path::new("python3"),
        &no_session,
        &no_run,
        &mut Vec::new(),
    );
    assert_eq!(
        error,
        Err("invalid literal for int() with base 10: 'two'".into())
    );
}

#[test]
fn run_reads_turns_tokens_cost_and_denials() {
    let payload = json!({
        "num_turns": 4, "total_cost_usd": 0.09,
        "usage": {"input_tokens": 10, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 1000, "output_tokens": 50},
        "permission_denials": [{"tool": "Bash"}],
    });
    let recorder = Recorder::new();
    let fake = |command: &[String], input: &Input, _: &Path, timeout: Duration| {
        recorder.record(command, input);
        assert_eq!(timeout, Duration::from_secs(600));
        done(0, &payload.to_string(), "")
    };
    let result = run(Path::new("claude"), "rules.md", Path::new("."), &fake).unwrap();
    assert_eq!(
        py::dumps(&result, None, true),
        r#"{"turns": 4, "cost": 0.09, "input_tokens": 1110, "output_tokens": 50, "denials": 1}"#
    );
    let call = &recorder.calls()[0];
    assert!(call.command.iter().any(|arg| arg == "--max-budget-usd"));
    assert_eq!(call.command.last().map(String::as_str), Some(PROMPT));
    assert!(
        call.input.is_none() && !call.inherited,
        "stdin is the null device"
    );
}

#[test]
fn run_reports_unparseable_output_and_timeouts() {
    let reply = |stdout: &'static [u8]| {
        move |_: &[String], _: &Input, _: &Path, _: Duration| {
            Ok(Output {
                code: 1,
                stdout: stdout.to_vec(),
                stderr: Vec::new(),
            })
        }
    };
    let session = |exec: Run| super::run(Path::new("claude"), "rules.md", Path::new("."), exec);
    assert_eq!(
        session(&reply(b"not json")),
        Ok(json!({"error": "JSONDecodeError"}))
    );
    assert_eq!(
        session(&reply(b"\xff")),
        Ok(json!({"error": "UnicodeDecodeError"}))
    );
    let slow = |_: &[String], _: &Input, _: &Path, _: Duration| -> Result<Output, Failure> {
        Err(Failure::Timeout)
    };
    assert_eq!(session(&slow), Ok(json!({"error": "TimeoutExpired"})));
    let missing = |_: &[String], _: &Input, _: &Path, _: Duration| -> Result<Output, Failure> {
        Err(Failure::Spawn("not found".into()))
    };
    assert!(session(&missing).is_err());
    assert!(session(&reply(b"[]")).is_err());
    assert!(session(&reply(br#"{"usage": [1]}"#)).is_err());
    assert!(session(&reply(br#"{"usage": {"input_tokens": "x"}}"#)).is_err());
    assert!(session(&reply(br#"{"permission_denials": 3}"#)).is_err());
    // Python's exact int sum: a total past 64 bits is refused, never wrapped or rounded.
    assert_eq!(
        session(&reply(
            br#"{"usage": {"input_tokens": 5000000000000000000, "cache_read_input_tokens": 5000000000000000000}}"#
        )),
        Err("the input token total is out of range".into())
    );
    assert_eq!(
        session(&reply(
            br#"{"usage": {"input_tokens": 10000000000000000000}}"#
        )),
        Err("the token count 10000000000000000000 is out of range".into())
    );
}

#[test]
fn run_sums_tokens_and_counts_denials_as_python_did() {
    let reply = |stdout: &'static str| {
        move |_: &[String], _: &Input, _: &Path, _: Duration| done(0, stdout, "")
    };
    let session =
        |exec: Run| super::run(Path::new("claude"), "rules.md", Path::new("."), exec).unwrap();
    let floats = session(&reply(
        r#"{"usage": {"input_tokens": 1.5, "cache_read_input_tokens": null}, "permission_denials": {"a": 1}}"#,
    ));
    assert_eq!(
        py::dumps(&floats, None, true),
        r#"{"turns": null, "cost": null, "input_tokens": 1.5, "output_tokens": null, "denials": 1}"#
    );
    // Python adds the ints exactly, then the float to their total: 18014398509481992.0, where
    // adding every count as a float gives 18014398509481988.0.
    let mixed = session(&reply(
        r#"{"usage": {"input_tokens": 9007199254740993, "cache_creation_input_tokens": 9007199254740997, "cache_read_input_tokens": 1.5}}"#,
    ));
    assert_eq!(mixed["input_tokens"], json!(18_014_398_509_481_992.0));
    // After the first float, Python keeps adding with its compensated float sum: 1e16 + 1 + 1
    // is 10000000000000002.0, where adding in turn would round each 1 away.
    let after = session(&reply(
        r#"{"usage": {"input_tokens": 1e16, "cache_creation_input_tokens": 1, "cache_read_input_tokens": 1}}"#,
    ));
    assert_eq!(after["input_tokens"], json!(10_000_000_000_000_002.0));
    let text = session(&reply(r#"{"usage": null, "permission_denials": "ab"}"#));
    assert_eq!(text["denials"], 2);
    assert_eq!(text["input_tokens"], 0);
    assert_eq!(session(&reply("{}"))["denials"], 0);
}

#[test]
fn main_resumes_and_skips_finished_runs() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    fs::write(
        &out,
        "{\"task\": \"pagination\", \"setup\": \"A\", \"trial\": 1}\n",
    )
    .unwrap();
    let sessions = Mutex::new(Vec::new());
    let session = |_: &Path, rules: &str, _: &Path| -> Result<Value, String> {
        sessions.lock().unwrap().push(rules.to_owned());
        Ok(json!({"turns": 1}))
    };
    // Each repository fails its tests before the session and passes them after.
    let before = std::cell::Cell::new(true);
    let tests = |_: &[String], input: &Input, _: &Path, _: Duration| {
        assert!(matches!(input, Input::Inherit));
        done(i32::from(before.replace(!before.get())), "", "")
    };
    let mut printed = Vec::new();
    command(
        &args(&out),
        Some(Path::new("claude")),
        Path::new("python3"),
        &session,
        &tests,
        &mut printed,
    )
    .unwrap();
    let sessions = sessions.into_inner().unwrap();
    assert_eq!(sessions.len(), 2 * TASKS.len() - 1);
    assert_eq!(sessions[..3], ["b.md", "a.md", "b.md"]);
    let text = fs::read_to_string(&out).unwrap();
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 2 * TASKS.len());
    assert_eq!(
        rows[1],
        r#"{"turns": 1, "task": "pagination", "setup": "B", "trial": 1, "passed": true}"#
    );
    assert_eq!(
        String::from_utf8(printed).unwrap(),
        format!("{}\n", rows[1..].join("\n"))
    );
}

#[test]
fn main_stops_on_a_task_that_already_passes_or_a_bad_resume_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.jsonl");
    let passing = |_: &[String], _: &Input, _: &Path, _: Duration| done(0, "", "");
    let error = command(
        &args(&out),
        Some(Path::new("claude")),
        Path::new("python3"),
        &no_session,
        &passing,
        &mut Vec::new(),
    );
    assert_eq!(error, Err("pagination should start failing".into()));
    let missing = |_: &[String], _: &Input, _: &Path, _: Duration| -> Result<Output, Failure> {
        Err(Failure::Spawn("gone".into()))
    };
    assert!(
        command(
            &args(&out),
            Some(Path::new("claude")),
            Path::new("python3"),
            &no_session,
            &missing,
            &mut Vec::new()
        )
        .is_err()
    );
    let failing = |_: &[String], _: &Input, _: &Path, _: Duration| done(1, "", "");
    let broken =
        |_: &Path, _: &str, _: &Path| -> Result<Value, String> { Err("session failed".to_owned()) };
    assert_eq!(
        command(
            &args(&out),
            Some(Path::new("claude")),
            Path::new("python3"),
            &broken,
            &failing,
            &mut Vec::new()
        ),
        Err("session failed".into())
    );
    fs::write(&out, "\n").unwrap();
    assert!(
        command(
            &args(&out),
            Some(Path::new("claude")),
            Path::new("python3"),
            &no_session,
            &no_run,
            &mut Vec::new()
        )
        .is_err()
    );
    fs::write(&out, "{}\n").unwrap();
    assert!(
        command(
            &args(&out),
            Some(Path::new("claude")),
            Path::new("python3"),
            &no_session,
            &no_run,
            &mut Vec::new()
        )
        .is_err()
    );
    let folder = dir.path().join("folder");
    fs::create_dir(&folder).unwrap();
    assert!(
        command(
            &args(&folder),
            Some(Path::new("claude")),
            Path::new("python3"),
            &no_session,
            &no_run,
            &mut Vec::new()
        )
        .is_err()
    );
    let jammed = dir.path().join("jammed.jsonl");
    let session = |_: &Path, _: &str, _: &Path| -> Result<Value, String> { Ok(json!({})) };
    let first = std::cell::Cell::new(true);
    let tests = |_: &[String], _: &Input, _: &Path, _: Duration| {
        // Fails before the session, passes after it, then makes the output a folder.
        let before = first.replace(!first.get());
        if !before {
            fs::create_dir_all(&jammed).unwrap();
        }
        done(i32::from(before), "", "")
    };
    assert!(
        command(
            &args(&jammed),
            Some(Path::new("claude")),
            Path::new("python3"),
            &session,
            &tests,
            &mut Vec::new()
        )
        .is_err()
    );
}

#[test]
fn the_stand_ins_refuse_every_call() {
    let refused =
        |call: &dyn Fn()| std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).is_err();
    assert!(refused(&|| drop(no_session(
        Path::new("c"),
        "r",
        Path::new(".")
    ))));
    assert!(refused(&|| drop(no_run(
        &[],
        &Input::Null,
        Path::new("."),
        Duration::ZERO
    ))));
}

#[cfg(unix)]
#[test]
fn commands_run_in_the_folder_with_their_input() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_in(
        &["pwd".to_owned()],
        &Input::Null,
        dir.path(),
        Duration::from_secs(30),
    )
    .unwrap();
    let shown = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        fs::canonicalize(shown.trim()).unwrap(),
        fs::canonicalize(dir.path()).unwrap()
    );
}
