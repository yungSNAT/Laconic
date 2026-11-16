// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

/// A new Git repository in a fresh temporary directory.
fn repository(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("laconic-hook-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let ok = Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["init", "-q"])
        .status()
        .unwrap()
        .success();
    assert!(ok);
    dir
}

fn event(dir: &Path, active: bool) -> String {
    mcp::to_string(&mcp::object(vec![
        ("hook_event_name", mcp::text("Stop")),
        ("cwd", mcp::text(dir.to_str().unwrap())),
        ("stop_hook_active", Json::Bool(active)),
    ]))
}

/// The block decision's reason.
fn reason(output: &str) -> String {
    let decision = mcp::parse(output).unwrap();
    assert_eq!(
        decision.get("decision").and_then(Json::as_str),
        Some("block")
    );
    decision
        .get("reason")
        .and_then(Json::as_str)
        .unwrap()
        .to_owned()
}

#[test]
fn blocks_once_on_failing_findings_in_uncommitted_markdown() {
    let dir = repository("block");
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(dir.join("docs/a.md"), "Pick a color.\n").unwrap();
    fs::write(dir.join("b.txt"), "Pick a color.\n").unwrap();
    fs::write(dir.join("c.MARKDOWN"), "It is done \u{2014} for now.\n").unwrap();
    fs::write(dir.join("clean.md"), "Pick a colour.\n").unwrap();
    // From a subdirectory, so the hook finds the top level.
    let reason = reason(&stop(&event(&dir.join("docs"), false)).unwrap().unwrap());
    assert!(
        reason.starts_with("laconic-check lint found 2 candidates"),
        "{reason}"
    );
    assert!(
        reason.contains("\ndocs/a.md:1:8: us-spelling: "),
        "{reason}"
    );
    assert!(reason.contains("\nc.MARKDOWN:1:12: em-dash: "), "{reason}");
    assert!(
        !reason.contains("b.txt") && !reason.contains("clean.md"),
        "{reason}"
    );
    // The turn already continued for a Stop hook, so it may end.
    assert_eq!(stop(&event(&dir, true)), Ok(None));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn applies_the_project_file_and_skips_advisory_findings_and_deleted_files() {
    let dir = repository("settings");
    let commit = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(["-c", "core.autocrlf=false"])
            .args(args)
            .status()
            .unwrap()
            .success();
        assert!(ok, "{args:?}");
    };
    fs::write(dir.join("gone.md"), "Pick a color.\n").unwrap();
    commit(&["add", "gone.md"]);
    commit(&["commit", "-q", "-m", "add"]);
    fs::remove_file(dir.join("gone.md")).unwrap();
    // A stacked hedge is advisory and never fails lint.
    let text = "It could potentially rain. Pick a color.\n";
    assert!(lint::lint(text).iter().any(|f| f.rule.rule().advisory));
    fs::write(dir.join("a.md"), text).unwrap();
    let reason = reason(&stop(&event(&dir, false)).unwrap().unwrap());
    assert!(
        reason.starts_with("laconic-check lint found 1 candidate in"),
        "{reason}"
    );
    fs::write(dir.join(PROJECT_FILE), "accept = color\n").unwrap();
    assert_eq!(stop(&event(&dir, false)), Ok(None));
    fs::write(dir.join(PROJECT_FILE), "max-findings = 1\n").unwrap();
    assert_eq!(stop(&event(&dir, false)), Ok(None));
    fs::write(dir.join(PROJECT_FILE), "colour = yes\n").unwrap();
    let error = stop(&event(&dir, false)).unwrap_err();
    assert!(error.ends_with(":1: unknown key 'colour'"), "{error}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn lists_fifty_findings_and_counts_the_rest() {
    let dir = repository("many");
    fs::write(dir.join("a.md"), "Pick a color.\n\n".repeat(52)).unwrap();
    let reason = reason(&stop(&event(&dir, false)).unwrap().unwrap());
    assert_eq!(reason.matches(": us-spelling: ").count(), LISTED);
    assert!(reason.ends_with("\nand 2 more\n"), "{reason}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn says_nothing_where_git_finds_no_repository_and_refuses_input_that_is_not_an_object() {
    // A directory that does not exist, as the temporary directory may be inside a repository.
    let missing = std::env::temp_dir().join(format!("laconic-hook-none-{}", std::process::id()));
    assert_eq!(stop(&event(&missing, false)), Ok(None));
    for input in ["", "[]", "{"] {
        assert_eq!(
            stop(input),
            Err("standard input is not a JSON object".to_owned())
        );
    }
}

#[test]
fn reads_markdown_paths_from_porcelain_status() {
    let status = b" M docs/a.md\0?? b.markdown\0 D gone.md\0D  staged-gone.md\0A  c.MD\0?? d.txt\0";
    assert_eq!(markdown(status), ["docs/a.md", "b.markdown", "c.MD"]);
}
