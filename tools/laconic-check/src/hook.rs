// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! `laconic-check hook`: a Stop hook for Claude Code and Codex. When the agent is about to end
//! its turn, it lints the repository's uncommitted Markdown and, if `lint` would fail, asks the
//! agent to look at the findings first. It asks once a turn: when the agent is already
//! continuing because of a Stop hook, it lets the turn end.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::lint;
use crate::mcp::{self, Json};
use crate::settings::{PROJECT_FILE, Settings};

/// The findings a block decision lists; it gives the number of the rest.
const LISTED: usize = 50;

/// What the hook prints for a Stop event, if anything: a block decision with `lint`'s failing
/// findings in the uncommitted Markdown of the repository that holds the event's `cwd`.
/// Nothing when the turn already continued for a Stop hook, outside a Git repository, or when
/// `lint` would pass, which applies the `.laconic-check` file at the repository's top level.
pub fn stop(input: &str) -> Result<Option<String>, String> {
    let event = mcp::parse(input)
        .filter(|event| matches!(event, Json::Object(_)))
        .ok_or("standard input is not a JSON object")?;
    if event.get("stop_hook_active") == Some(&Json::Bool(true)) {
        return Ok(None);
    }
    let cwd = match event.get("cwd").and_then(Json::as_str) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_dir().map_err(|e| format!("current directory: {e}"))?,
    };
    let Some(top) = git(&cwd, &["rev-parse", "--show-toplevel"])? else {
        return Ok(None);
    };
    let root = PathBuf::from(String::from_utf8_lossy(&top).trim_end_matches(['\n', '\r']));
    let settings = settings(&root)?;
    let status = git(
        &root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--no-renames",
        ],
    )?
    .ok_or("git status failed")?;
    let failing: Vec<String> = markdown(&status)
        .iter()
        .flat_map(|path| failing(&root, path, &settings))
        .collect();
    Ok((failing.len() > settings.max_findings).then(|| decision(&failing)))
}

/// `lint`'s failing findings in one file, one line each as `lint` prints them, or the error
/// that kept the file from being read.
fn failing(root: &Path, path: &str, settings: &Settings) -> Vec<String> {
    let text = match fs::read_to_string(root.join(path)) {
        Ok(text) => text,
        Err(e) => return vec![format!("{path}: {e}")],
    };
    let lines: Vec<&str> = text.lines().collect();
    lint::lint(&text)
        .into_iter()
        .filter(|f| {
            let line = f.line.checked_sub(1).and_then(|i| lines.get(i)).copied();
            !f.rule.rule().advisory && settings.keeps(f, line.unwrap_or(""))
        })
        .map(|f| {
            format!(
                "{path}:{}:{}: {}: {}",
                f.line,
                f.column,
                f.rule.name(),
                f.message
            )
        })
        .collect()
}

/// The block decision for these findings, listing the first LISTED.
fn decision(failing: &[String]) -> String {
    let mut reason = format!(
        "laconic-check lint found {} in uncommitted Markdown. Each is a reason to look, not \
         proof of a defect; each rule is described under its name in {}. Fix the ones that are \
         real before you finish; this check runs once a turn.\n",
        match failing.len() {
            1 => "1 candidate".to_owned(),
            n => format!("{n} candidates"),
        },
        lint::RULES_URL
    );
    for line in failing.iter().take(LISTED) {
        reason.push_str(line);
        reason.push('\n');
    }
    if failing.len() > LISTED {
        reason.push_str(&format!("and {} more\n", failing.len() - LISTED));
    }
    mcp::to_string(&mcp::object(vec![
        ("decision", mcp::text("block")),
        ("reason", mcp::text(&reason)),
    ]))
}

/// Git's output in `dir`, or `None` when Git fails there, as it does outside a repository. It
/// runs with the file system monitor off, so a checkout's settings start no other program for
/// these commands, and without optional locks, so it never writes the index.
fn git(dir: &Path, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "core.fsmonitor="])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("git: {e}"))?;
    Ok(out.status.success().then_some(out.stdout))
}

/// The project file at the repository's top level, or the default settings without one.
fn settings(root: &Path) -> Result<Settings, String> {
    let file = root.join(PROJECT_FILE);
    if !file.is_file() {
        return Ok(Settings::default());
    }
    let shown = file.display().to_string();
    let body = fs::read_to_string(&file).map_err(|e| format!("{shown}: {e}"))?;
    Settings::parse(&shown, &body)
}

/// The Markdown paths, relative to the top level, that `git status --porcelain=v1 -z` lists as
/// added, changed or untracked; a deleted file has nothing to lint.
// shortcut: this is all uncommitted Markdown, not only what the agent changed this turn, which
// would need the transcript; a user's own unfinished edits are linted too.
fn markdown(status: &[u8]) -> Vec<String> {
    status
        .split(|&b| b == 0)
        .filter_map(|entry| {
            let (code, path) = (entry.get(..2)?, entry.get(3..)?);
            let path = String::from_utf8_lossy(path).into_owned();
            let lower = path.to_lowercase();
            let wanted = lower.ends_with(".md") || lower.ends_with(".markdown");
            (wanted && !code.contains(&b'D')).then_some(path)
        })
        .collect()
}

#[cfg(test)]
mod tests;
