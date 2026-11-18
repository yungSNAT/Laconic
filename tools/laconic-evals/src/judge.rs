// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! Blind-judges paired evaluation responses and writes scorable rows: `judge`.

use crate::cases::{append, load_cases, read_jsonl, read_text};
use crate::fact_survival::to_float;
use crate::proc::Input;
use crate::py::{self, type_name};
use crate::run_evals::{
    Exec, Fault, Runner, Sleep, backoff, load_runner, neutral_cwd, parse_response,
    run_with_timeout, sort_by_pair,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

mod prompt;

pub use prompt::*;

const LABELS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIMENSIONS: [&str; 5] = [
    "correctness",
    "autonomy",
    "actionability",
    "safety",
    "concision",
];
const JUDGE_BEGIN: &str = "<!-- judge:begin -->";
const JUDGE_END: &str = "<!-- judge:end -->";
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Condition and blind label pairs, sorted by condition.
pub type Labels = Vec<(String, String)>;

/// Why a group went unjudged. Python skipped a group on a ValueError or RuntimeError (a failed
/// runner, a reply it could not read) and stopped the run on anything else.
#[derive(Debug, PartialEq)]
pub enum JudgeError {
    Skip(String),
    Fatal(String),
}

impl From<Fault> for JudgeError {
    fn from(fault: Fault) -> Self {
        match fault {
            Fault::Value(message) => JudgeError::Skip(message),
            Fault::Other(message) => JudgeError::Fatal(message),
        }
    }
}

impl JudgeError {
    pub fn message(self) -> String {
        match self {
            JudgeError::Skip(message) | JudgeError::Fatal(message) => message,
        }
    }
}

/// The responses of one (case, trial) group, by condition in the order first seen.
pub struct Group {
    pub case_id: Value,
    pub trial: Value,
    pub responses: Vec<(String, Value)>,
    pub dates: Vec<(String, Value)>,
}

impl Group {
    fn key(&self) -> (String, String) {
        (py::key(&self.case_id), py::key(&self.trial))
    }

    fn conditions(&self) -> BTreeSet<&str> {
        self.responses.iter().map(|(c, _)| c.as_str()).collect()
    }
}

/// Responses grouped by (case, trial), each with its recorded generation dates.
pub fn group_responses(rows: &[Value]) -> Result<Vec<Group>, String> {
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<(String, String), usize> = HashMap::new();
    for row in rows {
        let (case_id, trial) = (py::field(row, "case_id")?, py::field(row, "trial")?);
        let condition = py::text(row, "condition")?.to_owned();
        let response = py::field(row, "response")?.clone();
        let at = *index
            .entry((py::key(case_id), py::key(trial)))
            .or_insert_with(|| {
                groups.push(Group {
                    case_id: case_id.clone(),
                    trial: trial.clone(),
                    responses: Vec::new(),
                    dates: Vec::new(),
                });
                groups.len() - 1
            });
        let set = |list: &mut Vec<(String, Value)>, value: Value| match list
            .iter_mut()
            .find(|(c, _)| *c == condition)
        {
            Some(entry) => entry.1 = value,
            None => list.push((condition.clone(), value)),
        };
        set(&mut groups[at].responses, response);
        if let Some(day) = py::get(row, "date").filter(|d| py::truthy(d)) {
            set(&mut groups[at].dates, day.clone());
        }
    }
    Ok(groups)
}

/// A blind label for each condition of one group. The permutation comes from a digest of the
/// group key, not a random source: reruns give the same labels, so a partly judged file stays
/// consistent, yet the order varies by group, so position never reveals the condition.
pub fn assign_labels(case_id: &Value, trial: &Value, conditions: &[&str]) -> Labels {
    let mut ordered: Vec<&str> = conditions.to_vec();
    ordered.sort_unstable();
    let mut labels: Vec<char> = LABELS.chars().take(ordered.len()).collect();
    let seed = format!("{}\0{}", py::to_str(case_id), py::to_str(trial));
    let digest = Sha256::digest(seed.as_bytes());
    for index in (1..labels.len()).rev() {
        let swap = usize::from(digest[index % digest.len()]) % (index + 1);
        labels.swap(index, swap);
    }
    ordered
        .into_iter()
        .zip(labels)
        .map(|(condition, label)| (condition.to_owned(), label.to_string()))
        .collect()
}

/// How judge calls are made: the runner, its retries and timeout, and the outside world.
pub struct Caller<'a> {
    pub exec: Exec<'a>,
    pub sleep: Sleep<'a>,
    pub runner: &'a Runner,
    pub retries: i64,
    pub call_timeout: f64,
}

impl Caller<'_> {
    /// Calls the judge runner with the prompt on standard input, retrying a failed run. The
    /// prompt goes in on stdin rather than as a last argument: a runner command ending in an
    /// option that takes a value (the claude runner ends in `--tools ""`) would take the prompt
    /// as that value. Stdin also avoids argument length limits.
    pub fn invoke(&self, prompt: &str) -> Result<(String, Value), JudgeError> {
        let mut completed = None;
        for attempt in 0..=self.retries {
            let cwd = neutral_cwd("eval-neutral-cwd-").map_err(JudgeError::Fatal)?;
            let input = Input::Bytes(prompt.as_bytes());
            let done = run_with_timeout(
                self.exec,
                &self.runner.command,
                &input,
                cwd.path(),
                self.call_timeout,
            )?;
            let ok = done.0 == 0;
            completed = Some(done);
            if ok {
                break;
            }
            if attempt < self.retries {
                (self.sleep)(backoff(attempt));
            }
        }
        let (code, stdout, stderr) =
            completed.ok_or_else(|| JudgeError::Fatal("--retries must not be negative".into()))?;
        if code != 0 {
            let detail = if py::strip(&stderr).is_empty() {
                py::strip(&stdout)
            } else {
                py::strip(&stderr)
            };
            return Err(JudgeError::Skip(format!(
                "Judge runner failed after {} attempts:\n{detail}",
                self.retries + 1
            )));
        }
        let (text, _, cost) = parse_response(&stdout, &self.runner.format)?;
        Ok((text, cost))
    }

    /// Judges one group, retrying a malformed verdict as often as a failed run: a grader that
    /// drops a field or wraps the JSON in prose has failed transiently, not for good.
    pub fn judge(
        &self,
        prompt: &str,
        case_id: &Value,
        trial: &Value,
        labels: &Labels,
    ) -> Result<(Vec<Value>, Value), JudgeError> {
        let mut last = None;
        for attempt in 0..=self.retries {
            let (text, cost) = self.invoke(prompt)?;
            match parse_judge_scores(&text, case_id, trial, labels) {
                Ok(rows) => return Ok((rows, cost)),
                Err(Fault::Value(error)) => last = Some(error),
                Err(Fault::Other(error)) => return Err(JudgeError::Fatal(error)),
            }
            if attempt < self.retries {
                (self.sleep)(backoff(attempt));
            }
        }
        Err(last.map_or_else(
            || JudgeError::Fatal("--retries must not be negative".into()),
            JudgeError::Skip,
        ))
    }
}

/// One judge call for calibration: the prompt, the fixture's case and name, and the labels.
pub type Judge<'a> =
    &'a dyn Fn(&str, &Value, &Value, &Labels) -> Result<(Vec<Value>, Value), String>;

/// The judge's `blocker` against hand-labelled responses, one response per call.
pub fn calibrate(
    fixtures: &[Value],
    cases: &Cases,
    rubric: &str,
    judge: Judge,
) -> Result<Value, String> {
    if fixtures.is_empty() {
        return Err("no calibration fixtures to judge".into());
    }
    let mut errors = Vec::new();
    let mut cost = 0.0;
    let labels: Labels = vec![("fixture".into(), "A".into())];
    for fixture in fixtures {
        let case_id = py::field(fixture, "case_id")?;
        let case = cases
            .get(&py::key(case_id))
            .ok_or_else(|| py::repr(case_id))?;
        let response = vec![(
            "fixture".to_owned(),
            py::field(fixture, "response")?.clone(),
        )];
        let prompt = build_judge_prompt(case, &response, &labels, rubric, &[])?;
        let (rows, spent) = judge(&prompt, case_id, py::field(fixture, "name")?, &labels)?;
        cost += to_float(Some(&spent))?;
        let row = rows.first().ok_or("list index out of range")?;
        let got = py::field(row, "blocker")?;
        let expected = py::field(fixture, "blocker")?;
        if py::key(got) != py::key(expected) {
            errors.push(
                json!({"fixture": fixture["name"], "expected": expected, "got": got,
                               "notes": py::field(row, "notes")?}),
            );
        }
    }
    #[expect(clippy::cast_precision_loss, reason = "fixture counts are below 2**53")]
    let agreement = 1.0 - errors.len() as f64 / fixtures.len() as f64;
    Ok(
        json!({"compared": fixtures.len(), "agreement": py::float(agreement), "errors": errors,
              "cost_usd": py::float(cost)}),
    )
}

/// Cases by id.
pub type Cases = HashMap<String, Value>;

/// The cases in a catalogue by id, unvalidated, as the judge read them.
pub fn cases_by_id(path: &Path) -> Result<Cases, String> {
    let mut cases = HashMap::new();
    for case in load_cases(path)? {
        cases.insert(py::key(py::field(&case, "id")?), case);
    }
    Ok(cases)
}

/// The options of `judge`.
pub struct JudgeArgs {
    pub calibrate: Option<PathBuf>,
    pub responses: Option<PathBuf>,
    pub cases: PathBuf,
    pub rubric: PathBuf,
    pub runner_config: PathBuf,
    pub runner: String,
    pub conditions: Vec<String>,
    pub retries: i64,
    pub call_timeout: f64,
    pub output: Option<PathBuf>,
}

/// `judge`: blind-judge the response groups, or calibrate against labelled responses.
/// Returns 1 when a group went unjudged.
pub fn command(
    args: &JudgeArgs,
    exec: Exec,
    sleep: Sleep,
    out: &mut dyn Write,
) -> Result<u8, String> {
    let rubric = grader_rubric(&read_text(&args.rubric)?);
    let cases = cases_by_id(&args.cases)?;
    let runner = load_runner(&args.runner_config, &args.runner)?;
    let caller = Caller {
        exec,
        sleep,
        runner: &runner,
        retries: args.retries,
        call_timeout: args.call_timeout,
    };
    let group = |prompt: &str, case_id: &Value, trial: &Value, labels: &Labels| {
        caller.judge(prompt, case_id, trial, labels)
    };
    if let Some(fixtures) = &args.calibrate {
        let judge = |prompt: &str, case_id: &Value, name: &Value, labels: &Labels| {
            group(prompt, case_id, name, labels).map_err(JudgeError::message)
        };
        let result = calibrate(&read_jsonl(fixtures)?, &cases, &rubric, &judge)?;
        py::print(out, &py::dumps_pretty(&result))?;
        return Ok(0);
    }
    let (Some(responses), Some(output)) = (&args.responses, &args.output) else {
        return Err("--responses and --output are required unless --calibrate is given".into());
    };
    let rows = read_jsonl(responses)?;
    if rows.is_empty() {
        return Err(format!("{}: no responses to judge", responses.display()));
    }
    let groups = group_responses(&rows)?;
    let required: BTreeSet<&str> = args.conditions.iter().map(String::as_str).collect();
    let observed: BTreeSet<&str> = rows
        .iter()
        .map(|r| r["condition"].as_str().expect("grouped"))
        .collect();
    let missing: Vec<&str> = required.difference(&observed).copied().collect();
    let unexpected: Vec<&str> = observed.difference(&required).copied().collect();
    if !missing.is_empty() || !unexpected.is_empty() {
        let mut details = Vec::new();
        if !missing.is_empty() {
            details.push(format!(
                "missing required condition(s): {}",
                missing.join(", ")
            ));
        }
        if !unexpected.is_empty() {
            details.push(format!(
                "unexpected condition(s): {}",
                unexpected.join(", ")
            ));
        }
        return Err(format!(
            "Response conditions do not match --conditions: {}",
            details.join("; ")
        ));
    }
    let (mut complete, mut incomplete): (Vec<&Group>, Vec<&Group>) = groups
        .iter()
        .partition(|g| required.is_subset(&g.conditions()));
    sort_by_pair(&mut incomplete, |g| (&g.case_id, &g.trial))?;
    for g in incomplete {
        let lacking: Vec<&str> = required.difference(&g.conditions()).copied().collect();
        eprintln!(
            "skip {}/trial {}: missing {}",
            py::to_str(&g.case_id),
            py::to_str(&g.trial),
            lacking.join(", ")
        );
    }
    let mut judged = HashSet::new();
    if output.exists() {
        for row in read_jsonl(output)? {
            judged.insert((
                py::key(py::field(&row, "case_id")?),
                py::key(py::field(&row, "trial")?),
            ));
        }
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    append(output, "")?;
    sort_by_pair(&mut complete, |g| (&g.case_id, &g.trial))?;
    let mut total_cost = 0.0;
    let mut skipped = Vec::new();
    for g in complete {
        let place = format!("{}/trial {}", py::to_str(&g.case_id), py::to_str(&g.trial));
        if judged.contains(&g.key()) {
            py::print(out, &format!("skip judged {place}"))?;
            continue;
        }
        let case = cases.get(&py::key(&g.case_id)).ok_or_else(|| {
            format!(
                "{}: response has no matching case in {}",
                py::to_str(&g.case_id),
                args.cases.display()
            )
        })?;
        let conditions: Vec<&str> = g.conditions().into_iter().collect();
        let labels = assign_labels(&g.case_id, &g.trial, &conditions);
        let prompt = build_judge_prompt(case, &g.responses, &labels, &rubric, &g.dates)?;
        let (scored, cost) = match group(&prompt, &g.case_id, &g.trial, &labels) {
            Ok(result) => result,
            // One unusable verdict must not discard the groups already written or the ones
            // still queued behind it.
            Err(JudgeError::Skip(error)) => {
                eprintln!("skip {place}: {error}");
                skipped.push(place);
                continue;
            }
            Err(JudgeError::Fatal(error)) => return Err(error),
        };
        total_cost += to_float(Some(&cost))?;
        let lines: String = scored
            .iter()
            .map(|row| format!("{}\n", py::dumps_row(row)))
            .collect();
        append(output, &lines)?;
        py::print(out, &format!("judged {place}"))?;
    }
    if total_cost != 0.0 {
        py::print(out, &format!("Reported judge cost: ${total_cost:.4}"))?;
    }
    if !skipped.is_empty() {
        eprintln!(
            "{} group(s) went unjudged: {}",
            skipped.len(),
            skipped.join(", ")
        );
        return Ok(1);
    }
    Ok(0)
}

#[cfg(test)]
mod tests;
