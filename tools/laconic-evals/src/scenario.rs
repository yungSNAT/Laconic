// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Captures a resumed Claude conversation for the judge and score commands: `scenario`.

use crate::cases::{append, load_cases, read_text, validate_cases};
use crate::proc::{Failure, Input};
use crate::py;
use crate::run_evals::{Exec, condition_prompt, strip_frontmatter};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CALL_TIMEOUT: Duration = Duration::from_secs(120);

/// Model ids and aliases such as opus[1m] or claude-opus-4-5@20251101 (Vertex); never an
/// option or a space.
pub fn valid_model(model: &str) -> bool {
    let mut chars = model.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || "._:/@[]-".contains(c))
}

/// The one case in a scenario folder's `case.jsonl`, with at least 2 turns of distinct ids.
pub fn load_scenario(directory: &Path) -> Result<Value, String> {
    let cases = load_cases(&directory.join("case.jsonl"))?;
    let errors = validate_cases(&cases);
    if cases.len() != 1 || !errors.is_empty() {
        return Err(format!(
            "Expected one valid scenario case: {}",
            errors.join("; ")
        ));
    }
    let case = cases.into_iter().next().expect("one case");
    let turns = py::get(&case, "turns").and_then(Value::as_array);
    let turns = turns
        .filter(|t| t.len() >= 2)
        .ok_or("Scenario needs at least two turns")?;
    let mut seen = HashSet::new();
    for turn in turns {
        let text = |key| {
            py::get(turn, key)
                .and_then(Value::as_str)
                .filter(|s| !py::strip(s).is_empty())
        };
        let (Some(id), Some(_)) = (text("id"), text("prompt")) else {
            return Err("Each turn needs a non-empty id and prompt".into());
        };
        if !seen.insert(id) {
            return Err(format!("Duplicate turn id: {id}"));
        }
    }
    Ok(case)
}

/// Bytes decoded as UTF-8, with each invalid byte as `\xNN`: Python's `backslashreplace`.
fn backslash_replace(bytes: &[u8]) -> String {
    let mut text = String::new();
    for chunk in bytes.utf8_chunks() {
        text.push_str(chunk.valid());
        for byte in chunk.invalid() {
            text.push_str(&format!("\\x{byte:02x}"));
        }
    }
    text
}

/// One claude call: its response and reported cost. An unusable reply is an error that keeps
/// every cost reported so far.
pub fn call_claude(
    exec: Exec,
    command: &[String],
    workdir: &Path,
    spent: f64,
    prompt: Option<&str>,
    timeout: Duration,
) -> Result<(String, f64), String> {
    // Prompt bytes stay outside argv and keep their newlines on Windows.
    let input = prompt.map_or(Input::Null, |p| Input::Bytes(p.as_bytes()));
    let result = match exec(command, &input, Some(workdir), Some(timeout)) {
        Ok(result) => result,
        Err(Failure::Timeout) => {
            return Err(format!(
                "Claude timed out; call cost unavailable; reported so far: ${spent:.6}"
            ));
        }
        Err(failure) => return Err(failure.message(&command[0])),
    };
    let stderr = backslash_replace(&result.stderr);
    let stdout = result
        .stdout
        .strip_prefix(b"\xef\xbb\xbf")
        .unwrap_or(&result.stdout);
    let parsed = std::str::from_utf8(stdout)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_json::from_str::<Value>(text).map_err(|e| e.to_string()))
        .and_then(|payload| {
            let cost = py::get(&payload, "total_cost_usd");
            match cost.filter(|c| c.is_number()).and_then(Value::as_f64) {
                Some(c) if c >= 0.0 && c.is_finite() => Ok((payload, c)),
                _ => Err("invalid total_cost_usd".to_owned()),
            }
        });
    let (payload, cost) = parsed.map_err(|error| {
        let detail = if py::strip(&stderr).is_empty() {
            error
        } else {
            py::strip(&stderr).to_owned()
        };
        format!(
            "Claude returned invalid JSON or cost ({detail}); call cost unavailable; \
             reported so far: ${spent:.6}"
        )
    })?;
    let response = py::get(&payload, "result");
    let text = response
        .and_then(Value::as_str)
        .filter(|r| !py::strip(r).is_empty());
    let failed = result.code != 0
        || py::get(&payload, "is_error").is_some_and(py::truthy)
        || py::get(&payload, "subtype").is_some_and(|s| py::to_str(s).starts_with("error"));
    match text {
        Some(text) if !failed => Ok((py::strip(text).to_owned(), cost)),
        _ => {
            let detail = if !py::strip(&stderr).is_empty() {
                py::strip(&stderr).to_owned()
            } else {
                match response.filter(|r| py::truthy(r)) {
                    Some(r) => py::to_str(r),
                    None => "missing response".into(),
                }
            };
            Err(format!(
                "Claude failed: {detail}; cumulative reported cost: ${:.6}",
                spent + cost
            ))
        }
    }
}

/// The options of `scenario run`.
pub struct ScenarioArgs {
    pub scenario: PathBuf,
    pub condition: String,
    pub condition_skill: Option<PathBuf>,
    pub model: String,
    pub trial: i64,
    pub budget_usd: f64,
    pub output: PathBuf,
}

/// Runs every turn of a scenario in one claude session and saves the conversation as one
/// response row, ungraded. A failure leaves the output file created but empty.
pub fn run_scenario(
    args: &ScenarioArgs,
    exec: Exec,
    session: &dyn Fn() -> String,
    timeout: Duration,
    out: &mut dyn Write,
) -> Result<(), String> {
    let case = load_scenario(&args.scenario)?;
    if !(args.budget_usd > 0.0 && args.budget_usd <= 25.0) || args.trial < 1 {
        return Err("Budget must be in (0, 25]; trial must be positive".into());
    }
    if !valid_model(&args.model) {
        return Err(
            "Model must start with a letter or digit and use only letters, digits \
                    and . _ : / @ [ ] -"
                .into(),
        );
    }
    let turns = case["turns"].as_array().expect("validated");
    let first = turns[0]["prompt"].as_str().expect("validated");
    let first_prompt = condition_prompt(
        first,
        &args.condition,
        args.condition_skill.as_deref(),
        true,
    )?;
    if args.condition == "candidate" {
        let skill = args
            .condition_skill
            .as_deref()
            .expect("checked by condition_prompt");
        if py::strip(&strip_frontmatter(&read_text(skill)?)).is_empty() {
            return Err("Candidate skill cannot be empty".into());
        }
    }
    let session_id = session();
    let mut spent = 0.0;
    let mut transcript = Vec::new();
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    // An existing output is refused before any call.
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.output)
        .map_err(|e| format!("{}: {e}", args.output.display()))?;
    let workdir = crate::run_evals::neutral_cwd("scenario-eval-")?;
    for (index, turn) in turns.iter().enumerate() {
        // Round down so the CLI never receives more than the remaining budget.
        let remaining = ((args.budget_usd - spent) * 1_000_000.0).floor() / 1_000_000.0;
        if remaining <= 0.0 {
            return Err(format!("Budget exhausted; reported cost: ${spent:.6}"));
        }
        let mut command: Vec<String> = [
            "claude",
            "--safe-mode",
            "--strict-mcp-config",
            "--print",
            "--output-format",
            "json",
        ]
        .map(str::to_owned)
        .into();
        command.extend([
            format!("--model={}", args.model),
            "--tools".into(),
            String::new(),
            if index == 0 {
                "--session-id"
            } else {
                "--resume"
            }
            .into(),
            session_id.clone(),
            "--max-budget-usd".into(),
            format!("{remaining:.6}"),
        ]);
        let prompt = if index == 0 {
            first_prompt.as_str()
        } else {
            turn["prompt"].as_str().expect("validated")
        };
        let (response, cost) =
            call_claude(exec, &command, workdir.path(), spent, Some(prompt), timeout)?;
        spent += cost;
        if spent > args.budget_usd {
            return Err(format!("Budget exceeded; reported cost: ${spent:.6}"));
        }
        transcript
            .push(json!({"turn_id": turn["id"], "prompt": turn["prompt"], "response": response}));
    }
    let row = json!({
        "case_id": case["id"],
        "trial": args.trial,
        "condition": args.condition,
        "runner": format!("claude:{}", args.model),
        "session_id": session_id,
        "response": py::dumps(&Value::from(transcript), None, false),
        "cost_usd": py::float(spent),
    });
    append(&args.output, &format!("{}\n", py::dumps_row(&row)))?;
    writeln!(
        out,
        "Conversation saved to {}; reported cost: ${spent:.6}. Not graded.",
        args.output.display()
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
