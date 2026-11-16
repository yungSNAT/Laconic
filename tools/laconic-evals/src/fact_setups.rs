// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Fetch rival skills at pinned commits and write the fact-survival runner config.
//!
//! Run: `laconic-evals fact-setups evals/results/<new folder>`
//! Then: `laconic-evals fact-setups --preflight evals/results/<that folder>`

use crate::cases::{parse, read_text, root};
use crate::fact_survival::to_float;
use crate::proc::{self, Failure, Input, Output};
use crate::py;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const RIVALS: [(&str, &str, &str); 4] = [
    (
        "humanizer",
        "blader/humanizer",
        "0ef9fe5bc3572734fd3ae6c448fe00a30cd5dcf0",
    ),
    (
        "avoid",
        "conorbronsdon/avoid-ai-writing",
        "9cd07215982eca9736e14caaa63241c0264856f3",
    ),
    (
        "stop-slop",
        "hardikpandya/stop-slop",
        "8da1f030185bdfe8471220585162991eaeb970e9",
    ),
    (
        "caveman",
        "JuliusBrussee/caveman",
        "9b68f7ed12b05e7efaff18e65c5a3fc2849fd0a4",
    ),
];
pub const SKILLS: [(&str, &str); 6] = [
    ("laconic-prose", "laconic-prose"),
    ("laconic-session", "laconic"),
    ("humanizer", "humanizer"),
    ("avoid", "avoid-ai-writing"),
    ("stop-slop", "stop-slop"),
    ("caveman", "caveman"),
];
const BASE: [&str; 10] = [
    "claude",
    "--print",
    "--output-format",
    "json",
    "--no-session-persistence",
    "--strict-mcp-config",
    "--setting-sources",
    "",
    "--model",
    "claude-opus-5-5",
];

/// Runs a command with the terminal's input and output; a failure is an error.
pub type CheckCall<'a> = &'a dyn Fn(&[String]) -> Result<(), String>;

/// The production `CheckCall`, as Python's `subprocess.run(..., check=True)`.
pub fn check_call(command: &[String]) -> Result<(), String> {
    let status = Command::new(&command[0]).args(&command[1..]).status();
    let status = status.map_err(|e| format!("{}: {e}", command[0]))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Command {} returned {status}",
            py::repr(&json!(command))
        ))
    }
}

fn strings(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|&part| part.to_owned()).collect()
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A rival's checkout at its pinned commit, cloned from `base` (https://github.com) unless
/// already present.
pub fn fetch(
    src: &Path,
    name: &str,
    repo: &str,
    commit: &str,
    base: &str,
    check: CheckCall,
) -> Result<PathBuf, String> {
    let target = src.join(name);
    if !target.exists() {
        let target = target.display().to_string();
        let url = format!("{base}/{repo}");
        check(&strings(&[
            "git",
            "clone",
            "-q",
            "--filter=blob:none",
            "--no-checkout",
            &url,
            &target,
        ]))?;
        check(&strings(&[
            "git",
            "-C",
            &target,
            "-c",
            "core.longpaths=true",
            "checkout",
            "-q",
            commit,
        ]))?;
    }
    Ok(target)
}

/// Copies a folder as `shutil.copytree` with `ignore_patterns(".git")`: every entry named
/// .git is skipped, and a link is copied as what it points to.
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let fail = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    fs::create_dir_all(to).map_err(|e| fail(to, e))?;
    let mut entries: Vec<_> = fs::read_dir(from)
        .and_then(Iterator::collect)
        .map_err(|e| fail(from, e))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries
        .into_iter()
        .filter(|entry| entry.file_name() != ".git")
    {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| fail(&source, e))?;
        }
    }
    Ok(())
}

/// A one-skill plugin, so a setup can reach nothing but its own skill.
pub fn plugin(setups: &Path, name: &str, skill: &str, path: &Path) -> Result<PathBuf, String> {
    let folder = setups.join(format!("{name}-plugin"));
    let manifest = folder.join(".claude-plugin");
    if manifest.exists() {
        return Err(format!("{}: already exists", manifest.display()));
    }
    fs::create_dir_all(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let meta = py::dumps(
        &py::object([("name", Value::from(name)), ("version", json!("0.0.0"))]),
        None,
        true,
    );
    write(&manifest.join("plugin.json"), &meta)?;
    let target = folder.join("skills").join(skill);
    if target.exists() {
        return Err(format!("{}: already exists", target.display()));
    }
    copy_tree(path, &target)?;
    Ok(folder)
}

/// The runner config: one Claude Code command per setup.
pub fn runners(
    setups: &Path,
    skills: &[(&str, PathBuf, &str)],
) -> Result<Map<String, Value>, String> {
    let base = strings(&BASE);
    let mut config: Vec<(String, Vec<String>)> = Vec::new();
    for (name, folder, skill) in skills {
        let prompt = setups.join(format!("{name}-system.md"));
        write(
            &prompt,
            &format!("For any request to rewrite, humanise or edit text, use the {skill} skill.\n"),
        )?;
        let (folder, prompt) = (folder.display().to_string(), prompt.display().to_string());
        let extra = [
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
        ];
        config.push(((*name).to_owned(), [base.clone(), strings(&extra)].concat()));
    }
    let terse = root()
        .join("evals")
        .join("controls")
        .join("terse.md")
        .display()
        .to_string();
    config.push((
        "concise-control".into(),
        [
            base.clone(),
            strings(&["--tools", "", "--append-system-prompt-file", &terse]),
        ]
        .concat(),
    ));
    config.push((
        "no-skill".into(),
        [base, strings(&["--tools", ""])].concat(),
    ));
    Ok(config
        .into_iter()
        .map(|(name, command)| {
            let runner = py::object([
                ("command", Value::from(command)),
                ("response_format", json!("claude-json")),
                ("budget_flag", json!("--max-budget-usd")),
            ]);
            (name, runner)
        })
        .collect())
}

/// The error for iterating a value that cannot be iterated.
fn not_iterable(value: &Value) -> String {
    format!("{} is not iterable", py::repr(value))
}

/// Skill names called in a Claude stream-json transcript.
pub fn skills_used(stream: &str) -> Result<BTreeSet<String>, String> {
    let mut used = BTreeSet::new();
    for line in py::split_lines(stream) {
        if !line.starts_with('{') {
            continue;
        }
        let event = parse(line)?;
        let message = py::get(&event, "message").filter(|m| py::truthy(m));
        if message.is_some_and(|m| !m.is_object()) {
            return Err(format!(
                "{} has no attribute 'get'",
                py::repr(message.expect("a message"))
            ));
        }
        let content = message
            .and_then(|m| py::get(m, "content"))
            .filter(|c| py::truthy(c));
        let blocks = match content {
            None | Some(Value::String(_) | Value::Object(_)) => &[][..],
            Some(Value::Array(blocks)) => blocks.as_slice(),
            Some(other) => return Err(not_iterable(other)),
        };
        for block in blocks {
            if py::get(block, "type").is_some_and(|t| t == "tool_use")
                && py::get(block, "name").is_some_and(|n| n == "Skill")
            {
                let input = py::field(block, "input")?;
                if !input.is_object() {
                    return Err(format!("{} has no attribute 'get'", py::repr(input)));
                }
                let skill = py::get(input, "skill").map(py::to_str).unwrap_or_default();
                used.insert(skill.rsplit(':').next().expect("a part").to_owned());
            }
        }
    }
    Ok(used)
}

/// Runs a command with input and a timeout.
pub type Exec<'a> = &'a dyn Fn(&[String], &Input, Duration) -> Result<Output, Failure>;

/// The production `Exec`.
pub fn exec(command: &[String], input: &Input, timeout: Duration) -> Result<Output, Failure> {
    proc::run(command, input, None, Some(timeout))
}

/// One call per skill setup, proving it loads its own skill. Prints each reply to read.
/// Returns the exit status: 1 when a setup did not call its skill.
pub fn preflight(here: &Path, exec: Exec, stdout: &mut dyn Write) -> Result<i32, String> {
    let config = parse(&read_text(&here.join("runners.json"))?)?;
    let config = config.as_object().ok_or("runners.json is not an object")?;
    let cases = read_text(&root().join("evals").join("fact-cases.jsonl"))?;
    let first = py::split_lines(&cases)
        .into_iter()
        .next()
        .ok_or("list index out of range")?;
    let prompt = py::text(&parse(first)?, "prompt")?.to_owned();
    let (mut failures, mut cost) = (0, 0.0);
    for (name, runner) in config {
        let command = py::field(runner, "command")?
            .as_array()
            .ok_or("a runner's command is not a list")?;
        if !command.iter().any(|arg| arg == "--plugin-dir") {
            continue;
        }
        let command: Vec<String> = command
            .iter()
            .flat_map(|arg| {
                if arg == "json" {
                    vec!["stream-json".to_owned(), "--verbose".to_owned()]
                } else {
                    vec![py::to_str(arg)]
                }
            })
            .collect();
        let done = exec(
            &command,
            &Input::Bytes(prompt.as_bytes()),
            Duration::from_secs(600),
        )
        .map_err(|e| e.message(&command[0]))?;
        let out = proc::text(&done.stdout)?;
        let used = skills_used(&out)?;
        let wanted = SKILLS
            .iter()
            .find(|(setup, _)| setup == name)
            .map(|(_, skill)| *skill);
        let wanted = wanted.ok_or_else(|| py::str_repr(name))?;
        failures += i32::from(!used.contains(wanted));
        let mut results = Vec::new();
        for line in py::split_lines(&out)
            .into_iter()
            .filter(|line| line.starts_with(r#"{"type":"result""#))
        {
            results.push(parse(line)?);
        }
        let costs = results
            .iter()
            .map(|result| to_float(py::get(result, "total_cost_usd")))
            .collect::<Result<Vec<f64>, String>>()?;
        cost += py::sum(costs);
        let reply = results.first().and_then(|result| py::get(result, "result"));
        let reply = match reply {
            Some(Value::String(text)) => text.chars().take(400).collect(),
            Some(other) => py::to_str(other),
            None => String::new(),
        };
        let called: Vec<Value> = used
            .iter()
            .map(|skill| Value::from(skill.as_str()))
            .collect();
        let called = py::repr(&Value::from(called));
        writeln!(
            stdout,
            "{name}: wanted {wanted}, called {called}\n{reply}\n"
        )
        .map_err(|e| e.to_string())?;
    }
    let line = py::dumps(
        &json!({"stage": "preflight", "cost_usd": py::float(cost)}),
        None,
        true,
    );
    let overhead = here.join("overhead.jsonl");
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&overhead)
        .and_then(|mut sink| sink.write_all(format!("{line}\n").as_bytes()))
        .map_err(|e| format!("{}: {e}", overhead.display()))?;
    Ok(i32::from(failures > 0))
}

/// Fetches the rivals into `here/src`, builds `here/setups` afresh and writes
/// `here/runners.json`. Returns the setup names as printed, one a line.
pub fn setup(here: &Path, base: &str, check: CheckCall) -> Result<String, String> {
    let (src, setups) = (here.join("src"), here.join("setups"));
    let _ = fs::remove_dir_all(&setups);
    fs::create_dir_all(&setups).map_err(|e| format!("{}: {e}", setups.display()))?;
    fs::create_dir_all(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    let mut rival = std::collections::BTreeMap::new();
    for (name, repo, commit) in RIVALS {
        rival.insert(name, fetch(&src, name, repo, commit, base, check)?);
    }
    let skills_dir = root().join("skills");
    let skills = [
        (
            "laconic-prose",
            plugin(
                &setups,
                "laconic-prose",
                "laconic-prose",
                &skills_dir.join("laconic-prose"),
            )?,
            "laconic-prose",
        ),
        (
            "laconic-session",
            plugin(
                &setups,
                "laconic-session",
                "laconic",
                &skills_dir.join("laconic"),
            )?,
            "laconic",
        ),
        (
            "humanizer",
            plugin(&setups, "humanizer", "humanizer", &rival["humanizer"])?,
            "humanizer",
        ),
        (
            "avoid",
            rival["avoid"].join("plugins").join("avoid-ai-writing"),
            "avoid-ai-writing",
        ),
        (
            "stop-slop",
            plugin(&setups, "stop-slop", "stop-slop", &rival["stop-slop"])?,
            "stop-slop",
        ),
        (
            "caveman",
            plugin(
                &setups,
                "caveman",
                "caveman",
                &rival["caveman"]
                    .join("plugins")
                    .join("caveman")
                    .join("skills")
                    .join("caveman"),
            )?,
            "caveman",
        ),
    ];
    let config = runners(&setups, &skills)?;
    write(
        &here.join("runners.json"),
        &py::dumps_pretty(&Value::Object(config.clone())),
    )?;
    Ok(config.keys().cloned().collect::<Vec<_>>().join("\n"))
}

/// `Path.resolve()` for a folder that may not exist yet: an absolute path.
pub fn resolve(folder: &Path) -> Result<PathBuf, String> {
    std::path::absolute(folder).map_err(|e| format!("{}: {e}", folder.display()))
}

#[cfg(test)]
mod tests;
