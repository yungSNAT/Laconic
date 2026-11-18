// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The steps of the dated benchmark runs in `evals/runs/`, which each run's `run.sh` calls:
//! the runner configurations, the answer files, one row per answer or rewrite, and the
//! published figures recomputed from those rows. Each writes the bytes the run's Python wrote.

use crate::cases::{self, read_jsonl, read_text};
use crate::py;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

mod arith;
mod rewrite;
mod style;

use arith::*;
pub use rewrite::{rewrite_pairs, rewrite_results, rewrite_summary};
pub use style::style_summary;

/// The run folders `record setups` writes configurations for.
pub const RUNS: [&str; 3] = [
    "concise-style-2026-11-17",
    "token-cost-2026-11-17",
    "rival-rewrites-2026-11-17",
];

/// The Claude Code flags every run used, after the program, ending `--tools ""`.
const FLAGS: [&str; 14] = [
    "--disable-slash-commands",
    "--print",
    "--output-format",
    "json",
    "--no-session-persistence",
    "--strict-mcp-config",
    "--setting-sources",
    "",
    "--model",
    "claude-opus-5-5",
    "--effort",
    "high",
    "--tools",
    "",
];

/// One runner configuration: `program`, the flags and `extra`.
fn runner(program: &str, extra: &[String]) -> Value {
    let command: Vec<String> = [program]
        .iter()
        .chain(FLAGS.iter())
        .map(|s| (*s).to_owned())
        .chain(extra.iter().cloned())
        .collect();
    json!({"command": command, "response_format": "claude-json", "budget_flag": "--max-budget-usd"})
}

fn prompt_file(path: String) -> Vec<String> {
    vec!["--append-system-prompt-file".into(), path]
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    cases::write(path, text)
}

/// Writes the files `run`'s `run.sh` asks for into the work directory `work`, which the
/// commands name by its path as given.
pub fn setups(run: &str, work: &Path) -> Result<(), String> {
    let w = work
        .to_str()
        .ok_or_else(|| format!("{}: not valid UTF-8", work.display()))?;
    match run {
        "concise-style-2026-11-17" => {
            let none = prompt_file(format!("{w}/arm-none.md"));
            let mut concise = vec![
                "--settings".to_owned(),
                r#"{"outputStyle":"Concise"}"#.into(),
            ];
            concise.extend(none.clone());
            let setups = json!({
                "claude": runner("claude", &none),
                "setup-laconic": runner("claude", &prompt_file(format!("{w}/arm-laconic.md"))),
                "setup-concise": runner("claude", &concise),
                "setup-terse": runner("claude", &prompt_file(format!("{w}/arm-terse.md"))),
            });
            write(
                &work.join("setups.json"),
                &(py::dumps_pretty(&setups) + "\n"),
            )
        }
        "token-cost-2026-11-17" => {
            let fixed = format!("{w}/claude-fixed-cwd");
            for (name, program) in [("setups.json", "claude"), ("setups-warm.json", &fixed)] {
                let arm = |arm: &str| runner(program, &prompt_file(format!("{w}/arm-{arm}.md")));
                let setups = json!({
                    "claude": arm("none"),
                    "setup-laconic": arm("laconic"),
                    "setup-caveman": arm("caveman"),
                    "setup-chisle": arm("chisle"),
                });
                write(&work.join(name), &py::dumps_pretty(&setups))?;
            }
            Ok(())
        }
        "rival-rewrites-2026-11-17" => rewrite_setups(w, work),
        other => Err(format!(
            "unknown run {other}; expected one of {}",
            RUNS.join(", ")
        )),
    }
}

/// The rewrite run's runner configuration, its cases and patina's input files, from the
/// benchmark's texts in `aihb-2026-10.jsonl`.
fn rewrite_setups(w: &str, work: &Path) -> Result<(), String> {
    let program = format!("{w}/claude-fixed-cwd");
    let setups: Map<String, Value> = ["laconic", "humanizer", "avoid", "work", "plain"]
        .iter()
        .map(|name| {
            let file = prompt_file(format!("{w}/arms/{name}.md"));
            ((*name).to_owned(), runner(&program, &file))
        })
        .collect();
    write(
        &work.join("setups.json"),
        &py::dumps_pretty(&Value::Object(setups)),
    )?;
    let ask = "Rewrite this text so it reads as if a person wrote it. Reply with the rewritten text only.";
    let mut lines = String::new();
    for text in read_jsonl(&work.join("aihb-2026-10.jsonl"))? {
        let id = py::text(&text, "sample_id")?;
        let input = py::text(&text, "input")?;
        let case = json!({
            "id": format!("aihb-2026-10-{id}"),
            "category": "rewrite",
            "risk": "low",
            "prompt": format!("{ask}\n\n{input}"),
            "criteria": ["Keeps every fact of the source."],
        });
        lines.push_str(&py::dumps(&case, None, true));
        lines.push('\n');
        write(
            &work.join(format!("patina-in/{id}.md")),
            &format!("{input}\n"),
        )?;
    }
    write(&work.join("cases.jsonl"), &lines)
}

/// The rows of `path`, or none when it does not exist.
fn rows_if_any(path: &Path) -> Result<Vec<Value>, String> {
    if path.exists() {
        read_jsonl(path)
    } else {
        Ok(Vec::new())
    }
}

/// `(run, n, row)` for every answer: the cold calls in `responses.jsonl`, then the warm ones
/// in `warm.jsonl`, each numbered from 0.
fn answers(work: &Path) -> Result<Vec<(&'static str, usize, Value)>, String> {
    let mut all = Vec::new();
    for (run, name) in [("cold", "responses.jsonl"), ("warm", "warm.jsonl")] {
        for (n, row) in rows_if_any(&work.join(name))?.into_iter().enumerate() {
            all.push((run, n, row));
        }
    }
    Ok(all)
}

/// Writes each answer to `answers/<run>-<n>.txt` for `run.sh` to lint.
pub fn write_answers(work: &Path) -> Result<(), String> {
    let folder = work.join("answers");
    fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    for (run, n, row) in answers(work)? {
        write(
            &folder.join(format!("{run}-{n}.txt")),
            py::text(&row, "response")?,
        )?;
    }
    Ok(())
}

/// Whether the one-line file at `path` holds '1'.
fn flag(path: &Path) -> Result<bool, String> {
    Ok(py::strip(&read_text(path)?) == "1")
}

/// A row's key: case, trial and condition.
fn answer_key(row: &Value) -> Result<String, String> {
    Ok(py::key(&json!([
        py::field(row, "case_id")?,
        py::field(row, "trial")?,
        py::field(row, "condition")?
    ])))
}

fn by_answer(rows: Vec<Value>) -> Result<HashMap<String, Value>, String> {
    rows.into_iter()
        .map(|row| Ok((answer_key(&row)?, row)))
        .collect()
}

const DIMENSIONS: [&str; 5] = [
    "correctness",
    "autonomy",
    "actionability",
    "safety",
    "concision",
];

/// `d.get(key, default)` on a dict, or on `{}` when `d` is falsy, as `r.get("usage") or {}`.
/// Any other truthy value raises in Python, as `.get` does on it.
fn usage_field(usage: Option<&Value>, key: &str) -> Result<Value, String> {
    match usage.filter(|u| py::truthy(u)) {
        None => Ok(json!(0)),
        Some(Value::Object(u)) => Ok(u.get(key).cloned().unwrap_or(json!(0))),
        Some(other) => Err(format!("usage is {}", py::repr(other))),
    }
}

/// `re.search(pattern, text, re.IGNORECASE)`, raising on a pattern that cannot compile or run.
fn search(pattern: &str, text: &str) -> Result<bool, String> {
    cases::compile(pattern)?
        .is_match(text)
        .map_err(|e| e.to_string())
}

/// The case's regex checks the answer fails, or null when the case has none.
fn regex_failed(case: &Value, text: &str) -> Result<Value, String> {
    let Some(checks) = py::get(case, "checks").filter(|c| py::truthy(c)) else {
        return Ok(Value::Null);
    };
    if !checks.is_object() {
        return Err(format!("checks is {}", py::repr(checks)));
    }
    let patterns = |field| -> Result<Vec<&str>, String> {
        match py::get(checks, field) {
            None => Ok(Vec::new()),
            Some(Value::Array(items)) => items
                .iter()
                .map(|p| p.as_str().ok_or(format!("{field} holds a non-string")))
                .collect(),
            Some(_) => Err(format!("{field} is not a list")),
        }
    };
    let mut failed: Vec<String> = Vec::new();
    for p in patterns("must_include")? {
        if !search(p, text)? {
            failed.push(format!("missing {}", py::str_repr(p)));
        }
    }
    for p in patterns("must_not_include")? {
        if search(p, text)? {
            failed.push(format!("has {}", py::str_repr(p)));
        }
    }
    Ok(json!(failed))
}

/// One row per answer of the Concise and token-cost runs, without the answer text.
pub fn style_results(work: &Path) -> Result<String, String> {
    let cases: HashMap<String, Value> = read_jsonl(&work.join("cases.jsonl"))?
        .into_iter()
        .map(|c| Ok((py::text(&c, "id")?.to_owned(), c)))
        .collect::<Result<_, String>>()?;
    let house = by_answer(rows_if_any(&work.join("scores.jsonl"))?)?;
    let plain = by_answer(rows_if_any(&work.join("correctness.jsonl"))?)?;
    let mut out = String::new();
    for (run, n, r) in answers(work)? {
        let key = answer_key(&r)?;
        let case_id = py::text(&r, "case_id")?;
        let response = py::text(&r, "response")?;
        let usage = py::get(&r, "usage");
        let case = cases
            .get(case_id)
            .ok_or_else(|| format!("no case {}", py::str_repr(case_id)))?;
        let mut row = json!({
            "run": run,
            "case_id": case_id,
            "trial": py::field(&r, "trial")?,
            "condition": py::field(&r, "condition")?,
            "setup": py::field(&r, "runner")?,
            "input_tokens": usage_field(usage, "input_tokens")?,
            "cache_write_tokens": usage_field(usage, "cache_creation_input_tokens")?,
            "cache_read_tokens": usage_field(usage, "cache_read_input_tokens")?,
            "output_tokens": usage_field(usage, "output_tokens")?,
            "cost_usd": py::get(&r, "cost_usd").cloned().unwrap_or(Value::Null),
            "characters": response.chars().count(),
            "lint_clean": flag(&work.join(format!("lint/{run}-{n}")))?,
            "regex_failed": regex_failed(case, response)?,
        });
        if run == "cold" {
            let missing = |file: &str| format!("{file}: no score for {key}");
            let h = house.get(&key).ok_or_else(|| missing("scores.jsonl"))?;
            let p = plain
                .get(&key)
                .ok_or_else(|| missing("correctness.jsonl"))?;
            let mut scores = Map::new();
            for d in DIMENSIONS.iter().chain(&["blocker"]) {
                scores.insert((*d).to_owned(), py::field(h, d)?.clone());
            }
            row["house"] = Value::Object(scores);
            row["correctness_only"] = json!({
                "correctness": py::field(p, "correctness")?,
                "blocker": py::field(p, "blocker")?,
            });
        }
        out.push_str(&py::dumps(&row, None, true));
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
