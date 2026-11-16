// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Validates, runs and scores paired response-quality evaluations: `validate`, `plan`, `run`,
//! `checks`, `score` and `measure`.

use crate::cases::{append, check_response, load_cases, read_jsonl, read_text, validate_cases};
use crate::fact_survival::to_float;
use crate::proc::{self, Failure, Input, Output};
use crate::py::{self, type_name};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod response;
mod scores;
mod usage;

pub use response::*;
pub use scores::*;
pub use usage::*;

pub const WEIGHTS: [(&str, f64); 5] = [
    ("correctness", 0.35),
    ("autonomy", 0.25),
    ("actionability", 0.20),
    ("safety", 0.10),
    ("concision", 0.10),
];
/// The conditions, in sorted order.
pub const CONDITIONS: [&str; 3] = ["baseline", "candidate", "comparator"];
const INPUT_TOKEN_KEYS: [&str; 3] = [
    "input_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
];

/// One runner call: the command, what it reads on standard input, its working directory and
/// timeout. `proc::run` in production.
pub type Exec<'a> = &'a (
        dyn Fn(&[String], &Input, Option<&Path>, Option<Duration>) -> Result<Output, Failure> + Sync
    );
/// The pause between retries.
pub type Sleep<'a> = &'a (dyn Fn(Duration) + Sync);

/// Why a runner's output could not be read. Python caught a ValueError (bad JSON, an unknown
/// format) in places; anything else (a TypeError or AttributeError) always ended the run.
#[derive(Debug, PartialEq)]
pub enum Fault {
    Value(String),
    Other(String),
}

impl Fault {
    pub fn message(self) -> String {
        match self {
            Fault::Value(message) | Fault::Other(message) => message,
        }
    }
}

/// An int from JSON as Python sees one: a bool counts as 0 or 1, a float does not.
fn as_int(value: &Value) -> Option<i128> {
    match value {
        Value::Bool(b) => Some(i128::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(i128::from)
            .or_else(|| n.as_u64().map(i128::from)),
        _ => None,
    }
}

/// An int that is not a bool.
fn strict_int(value: &Value) -> Option<i128> {
    if value.is_boolean() {
        None
    } else {
        as_int(value)
    }
}

/// An int or float that is not a bool.
fn real(value: &Value) -> Option<f64> {
    if value.is_number() {
        value.as_f64()
    } else {
        None
    }
}

/// The (case, trial, condition, runner) keys of rows already recorded.
pub fn completed_keys(rows: &[Value]) -> HashSet<(String, i128, String, String)> {
    let mut keys = HashSet::new();
    for row in rows {
        let text = |name| py::get(row, name).and_then(Value::as_str);
        let trial = py::get(row, "trial").and_then(as_int);
        if let (Some(case_id), Some(trial), Some(condition), Some(runner)) =
            (text("case_id"), trial, text("condition"), text("runner"))
        {
            keys.insert((case_id.into(), trial, condition.into(), runner.into()));
        }
    }
    keys
}

/// The options of `run`.
pub struct RunArgs {
    pub cases: PathBuf,
    pub runner_config: PathBuf,
    pub runner: String,
    pub condition: String,
    pub condition_skill: Option<PathBuf>,
    pub case: Vec<String>,
    pub trials: i64,
    pub retries: i64,
    pub no_injection: bool,
    pub call_timeout: f64,
    pub jobs: i64,
    pub budget_usd: f64,
    pub allow_unmetered: bool,
    pub output: PathBuf,
}

/// The outside world `run` touches, so tests can stand in for it.
pub struct World<'a> {
    pub exec: Exec<'a>,
    pub sleep: Sleep<'a>,
    /// Today's date in ISO form, in local time.
    pub today: &'a (dyn Fn() -> String + Sync),
}

/// A runner from a runner config: its command and the format of its replies.
pub struct Runner {
    pub command: Vec<String>,
    pub format: Value,
    pub budget_flag: Option<String>,
}

/// The named runner in a runner config file.
pub fn load_runner(path: &Path, name: &str) -> Result<Runner, String> {
    let config: Value = crate::cases::parse(&read_text(path)?)?;
    let runner = py::get(&config, name)
        .ok_or_else(|| format!("no runner {} in {}", py::str_repr(name), path.display()))?;
    let command = py::field(runner, "command")?
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|i| i.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
        })
        .filter(|c| !c.is_empty())
        .ok_or_else(|| {
            format!(
                "runner {}: command must be a non-empty list of strings",
                py::str_repr(name)
            )
        })?;
    let format = py::get(runner, "response_format")
        .cloned()
        .unwrap_or_else(|| Value::from("text"));
    let budget_flag = match py::get(runner, "budget_flag").filter(|f| py::truthy(f)) {
        None => None,
        Some(Value::String(flag)) => Some(flag.clone()),
        Some(_) => {
            return Err(format!(
                "runner {}: budget_flag must be a string",
                py::str_repr(name)
            ));
        }
    };
    Ok(Runner {
        command,
        format,
        budget_flag,
    })
}

/// Calls the runner until it succeeds or the retries run out; the error is the failure Python
/// raised (the last reply's detail).
fn call(
    args: &RunArgs,
    runner: &Runner,
    model: Option<&str>,
    world: &World,
    case: &Value,
    trial: i64,
    remaining: f64,
) -> Result<Value, String> {
    let task = py::text(case, "prompt")?;
    let skill = args.condition_skill.as_deref();
    let prompt = condition_prompt(task, &args.condition, skill, !args.no_injection)?;
    let mut invocation = runner.command.clone();
    if let Some(flag) = &runner.budget_flag {
        invocation.extend([flag.clone(), format!("{remaining:.4}")]);
    }
    invocation.push(prompt);
    // The model reads today's date from its environment, so 'today' and 'yesterday' in a
    // response mean this date, which the judge needs.
    let called_on = (world.today)();
    let mut completed = None;
    for attempt in 0..=args.retries {
        let cwd = neutral_cwd("eval-neutral-cwd-")?;
        let done = run_with_timeout(
            world.exec,
            &invocation,
            &Input::Null,
            cwd.path(),
            args.call_timeout,
        )
        .map_err(Fault::message)?;
        let ok = done.0 == 0;
        completed = Some(done);
        if ok {
            break;
        }
        if attempt < args.retries {
            (world.sleep)(backoff(attempt));
        }
    }
    let (code, stdout, stderr) = completed.ok_or("--retries must not be negative")?;
    if code != 0 {
        let mut detail = if py::strip(&stderr).is_empty() {
            py::strip(&stdout)
        } else {
            py::strip(&stderr)
        }
        .to_owned();
        if !py::strip(&stdout).is_empty() {
            match parse_response(&stdout, &runner.format) {
                Ok((text, _, _)) if !text.is_empty() => detail = text,
                Ok(_) | Err(Fault::Value(_)) => {}
                Err(Fault::Other(error)) => return Err(error),
            }
        }
        let shown: Vec<String> = invocation[..invocation.len() - 1]
            .iter()
            .map(|w| shell_quote(w))
            .collect();
        return Err(format!(
            "Runner failed after {} attempts ({}):\n{detail}",
            args.retries + 1,
            shown.join(" ")
        ));
    }
    let (text, usage, cost) = parse_response(&stdout, &runner.format).map_err(Fault::message)?;
    if cost.is_null() && !args.allow_unmetered {
        return Err(
            "Runner did not report dollar cost; rerun with --allow-unmetered only when \
                    the provider has a separate hard spending cap."
                .into(),
        );
    }
    Ok(py::object([
        ("case_id", case["id"].clone()),
        ("trial", Value::from(trial)),
        ("condition", Value::from(args.condition.as_str())),
        ("runner", Value::from(args.runner.as_str())),
        ("response", Value::from(text)),
        ("usage", usage),
        ("cost_usd", cost),
        ("model", model.map_or(Value::Null, Value::from)),
        ("date", Value::from(called_on)),
    ]))
}

/// Runs one condition over the cases, appending a row per completed call to the output;
/// resumable. Returns the exit status: 2 when the budget ran out with calls left.
pub fn run_evaluations(args: &RunArgs, world: &World, out: &mut dyn Write) -> Result<u8, String> {
    let cases = load_cases(&args.cases)?;
    let errors = validate_cases(&cases);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let ids: HashSet<&str> = cases.iter().filter_map(|c| c["id"].as_str()).collect();
    let mut unknown: Vec<&str> = args
        .case
        .iter()
        .map(String::as_str)
        .filter(|c| !ids.contains(c))
        .collect();
    unknown.sort_unstable();
    unknown.dedup();
    if !unknown.is_empty() {
        return Err(format!(
            "--case matched no evaluation case: {}",
            unknown.join(", ")
        ));
    }
    let runner = load_runner(&args.runner_config, &args.runner)?;
    if runner.format != "claude-json" && !args.allow_unmetered {
        return Err(format!(
            "The {} response format never reports dollar cost; rerun with --allow-unmetered \
             only when the provider has a separate hard spending cap.",
            py::repr(&runner.format)
        ));
    }
    let prior = if args.output.exists() {
        read_jsonl(&args.output)?
    } else {
        Vec::new()
    };
    let done = completed_keys(&prior);
    let mut costs = Vec::new();
    for row in &prior {
        if py::get(row, "condition").and_then(Value::as_str) == Some(args.condition.as_str())
            && py::get(row, "runner").and_then(Value::as_str) == Some(args.runner.as_str())
        {
            costs.push(to_float(py::get(row, "cost_usd"))?);
        }
    }
    let mut reported_cost = py::sum(costs);
    if !(args.budget_usd > 0.0 && args.budget_usd <= 25.0) {
        return Err("--budget-usd must be greater than 0 and no more than 25".into());
    }
    let jobs = usize::try_from(args.jobs).ok().filter(|&jobs| jobs >= 1);
    let jobs = jobs.ok_or("--jobs must be at least 1")?;
    // The requested model, not an attestation of the provider's resolved backend.
    let model = runner_model(&runner.command);
    let mut pending = Vec::new();
    for trial in 1..=args.trials {
        for case in &cases {
            let id = case["id"].as_str().expect("validated");
            if !args.case.is_empty() && !args.case.iter().any(|c| c == id) {
                continue;
            }
            if done.contains(&(
                id.to_owned(),
                i128::from(trial),
                args.condition.clone(),
                args.runner.clone(),
            )) {
                let skip = format!("skip completed {} trial {trial}: {id}", args.condition);
                py::print(out, &skip)?;
                continue;
            }
            pending.push((case, trial));
        }
    }
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    append(&args.output, "")?;
    // shortcut: the budget is a soft threshold. No new call is submitted once recorded cost
    // reaches it, but up to N - 1 calls may still be running; it is not a hard cap. Reserve
    // per-call cost before submitting if a run overshoots materially.
    let mut queue = pending.into_iter();
    let mut exhausted = false;
    // After a failure, calls already running still finish and are saved, so a resumed run
    // neither repeats nor loses their responses and costs.
    let mut failure: Option<String> = None;
    let mut stop: Option<String> = None;
    let model = model.as_deref();
    thread::scope(|scope| {
        let (sender, receiver) = mpsc::channel();
        let mut running = 0;
        loop {
            while failure.is_none() && stop.is_none() && !exhausted && running < jobs {
                let remaining = args.budget_usd - reported_cost;
                if remaining <= 0.0 {
                    exhausted = true;
                    break;
                }
                let Some((case, trial)) = queue.next() else {
                    break;
                };
                let sender = sender.clone();
                let runner = &runner;
                scope.spawn(move || {
                    sender.send(call(args, runner, model, world, case, trial, remaining))
                });
                running += 1;
            }
            if running == 0 {
                break;
            }
            let mut finished = vec![receiver.recv().expect("a running call reports")];
            finished.extend(receiver.try_iter());
            running -= finished.len();
            for result in finished {
                if stop.is_some() {
                    continue;
                }
                let row = match result {
                    Ok(row) => row,
                    Err(error) => {
                        failure.get_or_insert(error);
                        continue;
                    }
                };
                let written = to_float(Some(&row["cost_usd"])).and_then(|cost| {
                    reported_cost += cost;
                    append(&args.output, &format!("{}\n", py::dumps_row(&row)))?;
                    py::print(
                        out,
                        &format!(
                            "{} trial {}: {}",
                            args.condition,
                            row["trial"],
                            py::to_str(&row["case_id"])
                        ),
                    )
                });
                if let Err(error) = written {
                    stop = Some(error);
                }
            }
        }
    });
    if let Some(error) = stop.or(failure) {
        return Err(error);
    }
    if exhausted && queue.next().is_some() {
        eprintln!("Budget exhausted; stopping.");
        return Ok(2);
    }
    py::print(out, &format!("Reported cost: ${reported_cost:.4}"))?;
    Ok(0)
}

/// `validate`: the catalogue's problems on standard error, or a confirmation.
pub fn validate(cases: &Path, out: &mut dyn Write) -> Result<u8, String> {
    let errors = validate_cases(&load_cases(cases)?);
    if errors.is_empty() {
        py::print(out, "Evaluation cases are valid.")?;
        return Ok(0);
    }
    for error in errors {
        eprintln!("ERROR: {error}");
    }
    Ok(1)
}

/// The cases of a catalogue that must be valid.
pub fn valid_cases(path: &Path) -> Result<Vec<Value>, String> {
    let cases = load_cases(path)?;
    let errors = validate_cases(&cases);
    if errors.is_empty() {
        Ok(cases)
    } else {
        Err(errors.join("\n"))
    }
}

/// `plan`: the paired run matrix as JSON Lines.
pub fn plan(cases: &Path, trials: i64, include_comparator: bool) -> Result<String, String> {
    let cases = valid_cases(cases)?;
    let conditions = if include_comparator {
        &CONDITIONS[..]
    } else {
        &CONDITIONS[..2]
    };
    let mut lines = String::new();
    for trial in 1..=trials {
        for case in &cases {
            for condition in conditions {
                let row = json!({"case_id": case["id"], "trial": trial, "condition": condition});
                lines.push_str(&py::dumps(&row, None, true));
                lines.push('\n');
            }
        }
    }
    Ok(lines)
}

#[cfg(test)]
mod tests;
