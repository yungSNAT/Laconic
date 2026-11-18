// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

/// A new Git repository in a fresh temporary directory.
fn repository(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("laconic-hook-{}-{name}", std::process::id()));
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
    assert!(lint::lint(text).iter().any(|f| f.rule.advisory()));
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
fn reports_the_project_files_own_rules() {
    let dir = repository("rules");
    fs::write(
        dir.join("a.md"),
        "We utilise it.
",
    )
    .unwrap();
    fs::write(
        dir.join(PROJECT_FILE),
        "[rules]
plain: utilise = use
",
    )
    .unwrap();
    let reason = reason(&stop(&event(&dir, false)).unwrap().unwrap());
    assert!(
        reason.contains(
            "
a.md:1:4: project/plain: 'utilise': the project file prefers 'use'
"
        ),
        "{reason}"
    );
    fs::write(
        dir.join(PROJECT_FILE),
        "disable = project/plain
[rules]
plain: utilise = use
",
    )
    .unwrap();
    assert_eq!(stop(&event(&dir, false)), Ok(None));
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
fn refuses_input_that_is_not_an_object() {
    for input in ["", "[]", "{"] {
        assert_eq!(
            stop(input),
            Err("standard input is not a JSON object".to_owned())
        );
    }
}

// Outside a repository the hook says nothing: tests/cli.rs checks that with Git's ceiling
// directories, as the temporary directory may itself be inside a repository.
#[test]
fn a_git_failure_other_than_finding_no_repository_is_an_error() {
    let broken = std::env::temp_dir().join(format!("laconic-hook-broken-{}", std::process::id()));
    fs::create_dir_all(&broken).unwrap();
    fs::write(broken.join(".git"), "not a gitfile\n").unwrap();
    let error = stop(&event(&broken, false)).unwrap_err();
    assert!(error.starts_with("git rev-parse: fatal: "), "{error}");
    let missing = broken.join("missing");
    let error = stop(&event(&missing, false)).unwrap_err();
    assert!(error.starts_with("git rev-parse: fatal: "), "{error}");
    let _ = fs::remove_dir_all(&broken);
}

#[test]
fn a_markdown_file_it_cannot_read_is_an_error_whatever_max_findings_allows() {
    let dir = repository("unread");
    fs::write(dir.join(PROJECT_FILE), "max-findings = 1\n").unwrap();
    fs::write(dir.join("latin1.md"), b"Caf\xe9\n").unwrap();
    let error = stop(&event(&dir, false)).unwrap_err();
    assert!(error.starts_with("latin1.md: "), "{error}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reads_markdown_paths_from_porcelain_status() {
    let status = b" M docs/a.md\0?? b.markdown\0 D gone.md\0D  staged-gone.md\0A  c.MD\0?? d.txt\0";
    let paths = ["docs/a.md", "b.markdown", "c.MD"].map(PathBuf::from);
    assert_eq!(markdown(status), Ok(paths.to_vec()));
}

#[cfg(unix)]
#[test]
fn keeps_a_path_that_is_not_utf8_and_a_top_level_name_that_ends_in_a_newline() {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9.md");
    assert_eq!(markdown(b"?? caf\xe9.md\0"), Ok(vec![PathBuf::from(name)]));
    let dir = repository("newline\n");
    fs::write(dir.join("a.md"), "Pick a color.\n").unwrap();
    let reason = reason(&stop(&event(&dir, false)).unwrap().unwrap());
    assert!(reason.contains("\na.md:1:8: us-spelling: "), "{reason}");
    let _ = fs::remove_dir_all(&dir);
}

// macOS file systems refuse a file name that is not UTF-8.
#[cfg(target_os = "linux")]
#[test]
fn lints_a_file_whose_name_is_not_utf8() {
    use std::os::unix::ffi::OsStrExt;
    let dir = repository("bytes");
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9.md");
    fs::write(dir.join(name), "Pick a color.\n").unwrap();
    let reason = reason(&stop(&event(&dir, false)).unwrap().unwrap());
    assert!(reason.contains(".md:1:8: us-spelling: "), "{reason}");
    let _ = fs::remove_dir_all(&dir);
}

#[cfg(not(unix))]
#[test]
fn refuses_a_path_that_is_not_utf8() {
    let error = markdown(b"?? caf\xe9.md\0").unwrap_err();
    assert!(
        error.starts_with("git printed a path that is not UTF-8"),
        "{error}"
    );
}
