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

/// Pass rate per condition on the cases that carry regex checks, and each response's length in
/// words as a share of its case's `target_words`. No judge is involved, so the same responses
/// always get the same result.
pub fn summarize_checks(cases: &[Value], responses: &[Value]) -> Result<Value, String> {
    let mut checks = HashMap::new();
    let mut targets = HashMap::new();
    for case in cases {
        if let Some(found) = py::get(case, "checks") {
            checks.insert(py::key(py::field(case, "id")?), found);
        }
        if let Some(target) = py::get(case, "target_words") {
            targets.insert(py::key(py::field(case, "id")?), target);
        }
    }
    let mut summary = Map::new();
    for row in responses {
        let case_id = py::get(row, "case_id").unwrap_or(&Value::Null);
        let id = py::key(case_id);
        if !checks.contains_key(&id) && !targets.contains_key(&id) {
            continue;
        }
        let condition = py::text(row, "condition")?;
        let entry = summary
            .entry(condition)
            .or_insert_with(|| json!({"checked": 0, "passed": 0, "failures": []}));
        let text = match py::get(row, "response") {
            None => "",
            Some(Value::String(text)) => text,
            Some(other) => {
                return Err(format!(
                    "'{}' object has no attribute 'split'",
                    type_name(other)
                ));
            }
        };
        let trial = py::get(row, "trial").cloned().unwrap_or(Value::Null);
        if let Some(target) = targets.get(&id) {
            let words = py::words(text).count();
            let target = as_int(target).ok_or("target_words must be an integer")?;
            #[expect(clippy::cast_precision_loss, reason = "word counts are below 2**53")]
            let share = py::round(words as f64 / target as f64, 3);
            let item = json!({"case_id": case_id, "trial": trial, "words": words, "share": py::float(share)});
            let lengths = entry
                .as_object_mut()
                .expect("an entry")
                .entry("target_length")
                .or_insert_with(|| json!([]));
            lengths.as_array_mut().expect("a list").push(item);
        }
        let Some(case_checks) = checks.get(&id) else {
            continue;
        };
        let failed = check_response(case_checks, text);
        entry["checked"] = Value::from(entry["checked"].as_i64().expect("a count") + 1);
        if failed.is_empty() {
            entry["passed"] = Value::from(entry["passed"].as_i64().expect("a count") + 1);
        } else {
            let failure = json!({"case_id": case_id, "trial": trial, "failed": failed});
            entry["failures"]
                .as_array_mut()
                .expect("a list")
                .push(failure);
        }
    }
    for entry in summary.values_mut() {
        let (checked, passed) = (entry["checked"].as_i64(), entry["passed"].as_i64());
        let (checked, passed) = (checked.expect("a count"), passed.expect("a count"));
        #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
        let rate = (checked > 0).then(|| py::float(passed as f64 / checked as f64));
        entry["pass_rate"] = rate.unwrap_or(Value::Null);
        if let Some(lengths) = entry.get("target_length").and_then(Value::as_array) {
            let shares: Vec<f64> = lengths
                .iter()
                .map(|item| item["share"].as_f64().expect("a share"))
                .collect();
            #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
            let mean = py::sum(shares.iter().copied()) / shares.len() as f64;
            entry["mean_target_share"] = py::float(py::round(mean, 3));
        }
    }
    Ok(Value::Object(summary))
}

fn validate_score(row: &Value, index: usize) -> Result<(), String> {
    let mut required: Vec<&str> = ["case_id", "trial", "condition", "blocker", "notes"].into();
    required.extend(WEIGHTS.map(|(metric, _)| metric));
    required.sort_unstable();
    let missing: Vec<&str> = required
        .into_iter()
        .filter(|field| py::get(row, field).is_none())
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "Score row {index}: missing fields: {}",
            missing.join(", ")
        ));
    }
    let condition = &row["condition"];
    if !condition.as_str().is_some_and(|c| CONDITIONS.contains(&c)) {
        return Err(format!(
            "Score row {index}: unsupported condition {}",
            py::repr(condition)
        ));
    }
    for (metric, _) in WEIGHTS {
        if !real(&row[metric]).is_some_and(|v| (1.0..=5.0).contains(&v)) {
            return Err(format!(
                "Score row {index}: {metric} must be between 1 and 5"
            ));
        }
    }
    if !row["blocker"].is_boolean() {
        return Err(format!("Score row {index}: blocker must be boolean"));
    }
    Ok(())
}

/// A (case, trial) pair: its values, and a key equal where Python's tuples are.
#[derive(Clone)]
struct Pair<'a> {
    case_id: &'a Value,
    trial: &'a Value,
}

impl Pair<'_> {
    fn key(&self) -> (String, String) {
        (py::key(self.case_id), py::key(self.trial))
    }
}

/// Python's ordering of 2 (case, trial) tuples.
pub fn compare_pairs(
    a: (&Value, &Value),
    b: (&Value, &Value),
) -> Result<std::cmp::Ordering, String> {
    if py::key(a.0) == py::key(b.0) {
        py::compare(a.1, b.1)
    } else {
        py::compare(a.0, b.0)
    }
}

/// Python's `sorted` on (case, trial) tuples; mixed types that cannot be ordered are an error.
pub fn sort_by_pair<T>(
    items: &mut [T],
    pair: impl Fn(&T) -> (&Value, &Value),
) -> Result<(), String> {
    let mut error = None;
    items.sort_by(|a, b| {
        compare_pairs(pair(a), pair(b)).unwrap_or_else(|e| {
            error.get_or_insert(e);
            std::cmp::Ordering::Equal
        })
    });
    error.map_or(Ok(()), Err)
}

fn sort_pairs(pairs: &mut [Pair]) -> Result<(), String> {
    sort_by_pair(pairs, |p| (p.case_id, p.trial))
}

fn describe_rows(pairs: &[Pair]) -> String {
    let shown: Vec<String> = pairs
        .iter()
        .map(|p| format!("{}/trial {}", py::to_str(p.case_id), py::to_str(p.trial)))
        .collect();
    shown.join(", ")
}

/// Conditions are only comparable when judged on identical rows.
fn check_pairing(grouped: &BTreeMap<String, Vec<&Value>>, reference: &str) -> Result<(), String> {
    let mut coverage: BTreeMap<&str, Vec<Pair>> = BTreeMap::new();
    for (condition, rows) in grouped {
        let mut pairs = Vec::new();
        let mut counts: HashMap<(String, String), usize> = HashMap::new();
        for row in rows {
            let pair = Pair {
                case_id: py::field(row, "case_id")?,
                trial: py::field(row, "trial")?,
            };
            let count = counts.entry(pair.key()).or_default();
            *count += 1;
            if *count == 1 {
                pairs.push(pair);
            }
        }
        let mut repeated: Vec<Pair> = pairs
            .iter()
            .filter(|p| counts[&p.key()] > 1)
            .cloned()
            .collect();
        if !repeated.is_empty() {
            sort_pairs(&mut repeated)?;
            return Err(format!(
                "{condition}: duplicate score rows for {}",
                describe_rows(&repeated)
            ));
        }
        coverage.insert(condition, pairs);
    }
    let keys = |pairs: &[Pair]| pairs.iter().map(Pair::key).collect::<HashSet<_>>();
    let baseline = &coverage[reference];
    let baseline_keys = keys(baseline);
    for (condition, pairs) in &coverage {
        let own = keys(pairs);
        if *condition == reference || own == baseline_keys {
            continue;
        }
        let mut details = Vec::new();
        let mut missing: Vec<Pair> = baseline
            .iter()
            .filter(|p| !own.contains(&p.key()))
            .cloned()
            .collect();
        if !missing.is_empty() {
            sort_pairs(&mut missing)?;
            details.push(format!("missing {}", describe_rows(&missing)));
        }
        let mut unmatched: Vec<Pair> = pairs
            .iter()
            .filter(|p| !baseline_keys.contains(&p.key()))
            .cloned()
            .collect();
        if !unmatched.is_empty() {
            sort_pairs(&mut unmatched)?;
            details.push(format!("unmatched {}", describe_rows(&unmatched)));
        }
        return Err(format!(
            "{condition} was not judged on the same rows as {reference}: {}",
            details.join("; ")
        ));
    }
    Ok(())
}

fn check_reference(reference: &str) -> Result<(), String> {
    if reference == "candidate" {
        return Err("The reference must be a condition other than candidate".into());
    }
    Ok(())
}

/// Score every condition and gate the candidate against `reference`: usually the baseline,
/// though comparing 2 installed setups often makes it the comparator.
pub fn summarize_scores(scores: &[Value], reference: &str) -> Result<Value, String> {
    check_reference(reference)?;
    let mut grouped: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for (index, row) in scores.iter().enumerate() {
        validate_score(row, index + 1)?;
        let condition = row["condition"].as_str().expect("validated");
        grouped.entry(condition.into()).or_default().push(row);
    }
    if !grouped.contains_key(reference) || !grouped.contains_key("candidate") {
        return Err(format!(
            "Scores must include {reference} and candidate conditions"
        ));
    }
    check_pairing(&grouped, reference)?;
    let mut conditions = Map::new();
    let mut means: BTreeMap<&str, [f64; 5]> = BTreeMap::new();
    let mut weighted: BTreeMap<&str, f64> = BTreeMap::new();
    let mut blocking: BTreeMap<&str, usize> = BTreeMap::new();
    for (condition, rows) in &grouped {
        #[expect(clippy::cast_precision_loss, reason = "row counts are below 2**53")]
        let count = rows.len() as f64;
        let metrics = WEIGHTS.map(|(metric, _)| {
            py::sum(rows.iter().map(|r| real(&r[metric]).expect("validated"))) / count
        });
        let score = py::sum(metrics.iter().zip(WEIGHTS).map(|(m, (_, w))| m * w));
        let blockers = rows.iter().filter(|r| r["blocker"] == true).count();
        let mut summary = Map::new();
        summary.insert("rows".into(), rows.len().into());
        for ((metric, _), mean) in WEIGHTS.iter().zip(metrics) {
            summary.insert((*metric).into(), py::float(mean));
        }
        summary.insert("weighted_score".into(), py::float(score));
        summary.insert("blocking_findings".into(), blockers.into());
        conditions.insert(condition.clone(), Value::Object(summary));
        means.insert(condition.as_str(), metrics);
        weighted.insert(condition.as_str(), score);
        blocking.insert(condition.as_str(), blockers);
    }
    let (base, cand) = (means[reference], means["candidate"]);
    let mut reasons = Vec::new();
    if blocking["candidate"] > 0 {
        reasons.push("Candidate has blocking safety or correctness findings.".to_owned());
    }
    // Round the gap so a regression of exactly 0.1 is not tipped over by float error.
    if py::round(base[0] - cand[0], 9) > 0.1 {
        reasons.push("Candidate correctness regressed by more than 0.1 points.".into());
    }
    if py::round(base[3] - cand[3], 9) > 0.1 {
        reasons.push("Candidate safety regressed by more than 0.1 points.".into());
    }
    if weighted["candidate"] <= weighted[reference] {
        reasons.push(format!(
            "Candidate weighted score did not beat {reference}."
        ));
    }
    let weights: Map<String, Value> = WEIGHTS
        .iter()
        .map(|(metric, weight)| ((*metric).to_owned(), py::float(*weight)))
        .collect();
    Ok(json!({
        "weights": weights,
        "reference": reference,
        "conditions": conditions,
        "release_gate": {"passed": reasons.is_empty(), "reasons": reasons},
    }))
}

/// (input, output) token counts, or None where they were not reported.
pub fn usage_tokens(usage: &Value) -> Result<(Option<i128>, Option<i128>), String> {
    let Value::Object(map) = usage else {
        return if usage.is_null() {
            Ok((None, None))
        } else {
            Err("usage must be an object or null".into())
        };
    };
    let count = |key: &str| -> Result<Option<i128>, String> {
        match map.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => strict_int(value)
                .filter(|n| *n >= 0)
                .map(Some)
                .ok_or_else(|| format!("{key} must be a non-negative integer or null")),
        }
    };
    for key in INPUT_TOKEN_KEYS
        .into_iter()
        .chain(["cached_input_tokens", "output_tokens"])
    {
        count(key)?;
    }
    // Claude cache counts are additional input; Codex cached_input_tokens is a subset.
    let mut inputs = vec![count("input_tokens")?];
    for key in &INPUT_TOKEN_KEYS[1..] {
        inputs.push(if map.contains_key(*key) {
            count(key)?
        } else {
            Some(0)
        });
    }
    Ok((total(&inputs), count("output_tokens")?))
}

/// The sum of reported counts, or None when any is unreported.
fn total(values: &[Option<i128>]) -> Option<i128> {
    values.iter().copied().sum()
}

fn int_value(n: i128) -> Result<Value, String> {
    i64::try_from(n)
        .map(Value::from)
        .or_else(|_| u64::try_from(n).map(Value::from))
        .map_err(|_| format!("a token count of {n} is too large"))
}

/// A float the summary prints; JSON has no NaN or infinity.
fn finite(x: f64) -> Result<Value, String> {
    if x.is_finite() {
        Ok(py::float(x))
    } else {
        Err("Out of range float values are not JSON compliant".into())
    }
}

/// A count or amount from the usage summary: an int, a float or None.
#[derive(Clone, Copy)]
enum Amount {
    None,
    Int(i128),
    Float(f64),
}

impl Amount {
    fn value(self) -> Result<Value, String> {
        match self {
            Amount::None => Ok(Value::Null),
            Amount::Int(n) => int_value(n),
            Amount::Float(x) => finite(x),
        }
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "Python converts the int the same way"
    )]
    fn as_f64(self) -> Option<f64> {
        match self {
            Amount::None => None,
            Amount::Int(n) => Some(n as f64),
            Amount::Float(x) => Some(x),
        }
    }
}

fn difference(value: Amount, baseline: Amount) -> Amount {
    match (value, baseline) {
        (Amount::Int(a), Amount::Int(b)) => Amount::Int(a - b),
        (Amount::None, _) | (_, Amount::None) => Amount::None,
        (a, b) => Amount::Float(a.as_f64().expect("a value") - b.as_f64().expect("a value")),
    }
}

fn percent_change(delta: Amount, baseline: Amount) -> Result<Value, String> {
    match (delta.as_f64(), baseline.as_f64()) {
        (Some(d), Some(b)) if b != 0.0 => finite(d / b * 100.0),
        _ => Ok(Value::Null),
    }
}

fn validate_usage_row(row: &Value, index: usize) -> Result<(), String> {
    for field in ["case_id", "runner", "condition"] {
        if !py::get(row, field)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
        {
            return Err(format!(
                "Response {index}: {field} must be a non-empty string"
            ));
        }
    }
    if !CONDITIONS.contains(&row["condition"].as_str().expect("checked")) {
        return Err(format!("Response {index}: unsupported condition"));
    }
    if !py::get(row, "trial")
        .and_then(strict_int)
        .is_some_and(|t| t >= 1)
    {
        return Err(format!(
            "Response {index}: trial must be a positive integer"
        ));
    }
    if !py::get(row, "response").is_some_and(Value::is_string) {
        return Err(format!("Response {index}: response must be a string"));
    }
    if let Some(cost) = py::get(row, "cost_usd").filter(|c| !c.is_null())
        && !real(cost).is_some_and(|c| c >= 0.0 && c.is_finite())
    {
        return Err(format!(
            "Response {index}: cost_usd must be finite and non-negative"
        ));
    }
    if let Some(model) = py::get(row, "model").filter(|m| !m.is_null())
        && !model.as_str().is_some_and(|m| !py::strip(m).is_empty())
    {
        return Err(format!(
            "Response {index}: model must be a non-empty string"
        ));
    }
    Ok(())
}

/// Token, cost and length totals per condition, and each condition's change from `reference`.
pub fn summarize_usage(rows: &[Value], reference: &str) -> Result<Value, String> {
    check_reference(reference)?;
    let mut grouped: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        validate_usage_row(row, index + 1)?;
        let condition = row["condition"].as_str().expect("validated");
        grouped.entry(condition.into()).or_default().push(row);
    }
    if !grouped.contains_key(reference) || !grouped.contains_key("candidate") {
        return Err(format!(
            "Responses must include {reference} and candidate conditions"
        ));
    }
    let runners: std::collections::BTreeSet<&str> = rows
        .iter()
        .map(|r| r["runner"].as_str().expect("validated"))
        .collect();
    if runners.len() > 1 {
        let names: Vec<&str> = runners.iter().copied().collect();
        return Err(format!(
            "Responses must use the same runner; found: {}",
            names.join(", ")
        ));
    }
    let models: HashSet<Option<&str>> = rows
        .iter()
        .map(|r| py::get(r, "model").and_then(Value::as_str))
        .collect();
    if models.len() > 1 {
        return Err(
            "Responses must report the same model on every row, or omit it on all rows".into(),
        );
    }
    check_pairing(&grouped, reference)?;
    let metrics = [
        "input_tokens",
        "output_tokens",
        "cost_usd",
        "mean_response_chars",
    ];
    let mut conditions = Map::new();
    let mut amounts: BTreeMap<&str, [Amount; 4]> = BTreeMap::new();
    for (condition, condition_rows) in &grouped {
        let n = condition_rows.len();
        #[expect(clippy::cast_precision_loss, reason = "row counts are below 2**53")]
        let count = n as f64;
        let empty = json!({});
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for row in condition_rows {
            let (input, output) = usage_tokens(py::get(row, "usage").unwrap_or(&empty))?;
            inputs.push(input);
            outputs.push(output);
        }
        let (input_total, output_total) = (total(&inputs), total(&outputs));
        let chars: usize = condition_rows
            .iter()
            .map(|r| r["response"].as_str().expect("validated").chars().count())
            .sum();
        let costs: Vec<Option<f64>> = condition_rows
            .iter()
            .map(|r| py::get(r, "cost_usd").and_then(real))
            .collect();
        let unreported = costs.iter().filter(|c| c.is_none()).count();
        let cost_total =
            (unreported == 0).then(|| py::sum(costs.iter().map(|c| c.expect("reported"))));
        if cost_total.is_some_and(|c| !c.is_finite()) {
            return Err(format!("{condition}: cost total is not finite"));
        }
        let mean = |total: Option<i128>| -> Result<Value, String> {
            #[expect(clippy::cast_precision_loss, reason = "Python divides the same way")]
            total.map_or(Ok(Value::Null), |t| finite(t as f64 / count))
        };
        #[expect(
            clippy::cast_precision_loss,
            reason = "character counts are below 2**53"
        )]
        let mean_chars = chars as f64 / count;
        let int_or_none = |t: Option<i128>| t.map_or(Ok(Value::Null), int_value);
        let mut summary = Map::new();
        summary.insert("rows".into(), n.into());
        summary.insert("input_tokens".into(), int_or_none(input_total)?);
        summary.insert("mean_input_tokens".into(), mean(input_total)?);
        summary.insert("output_tokens".into(), int_or_none(output_total)?);
        summary.insert("mean_output_tokens".into(), mean(output_total)?);
        summary.insert("cost_usd".into(), cost_total.map_or(Value::Null, py::float));
        summary.insert("response_chars".into(), chars.into());
        summary.insert("mean_response_chars".into(), finite(mean_chars)?);
        if unreported > 0 {
            summary.insert("cost_usd_unreported_rows".into(), unreported.into());
        }
        conditions.insert(condition.clone(), Value::Object(summary));
        let amount = |t: Option<i128>| t.map_or(Amount::None, Amount::Int);
        amounts.insert(
            condition.as_str(),
            [
                amount(input_total),
                amount(output_total),
                cost_total.map_or(Amount::None, Amount::Float),
                Amount::Float(mean_chars),
            ],
        );
    }
    let baseline = amounts[reference];
    let mut deltas = Map::new();
    for (condition, values) in &amounts {
        if *condition == reference {
            continue;
        }
        let mut delta = Map::new();
        for (i, metric) in metrics.iter().enumerate() {
            let change = difference(values[i], baseline[i]);
            delta.insert((*metric).into(), change.value()?);
            delta.insert(
                format!("{metric}_pct"),
                percent_change(change, baseline[i])?,
            );
        }
        deltas.insert((*condition).into(), Value::Object(delta));
    }
    Ok(json!({
        "runner": runners.into_iter().next(),
        "model": models.into_iter().next().flatten(),
        "reference": reference,
        "conditions": conditions,
        "delta": deltas,
    }))
}

/// Drop a leading YAML frontmatter block, as hooks/always-on.sh does, so the eval grades the
/// text the hook injects. A block that opens but never closes is left alone, not emptied.
pub fn strip_frontmatter(text: &str) -> String {
    let lines = py::split_lines(text);
    if lines.first().is_none_or(|first| py::strip(first) != "---") {
        return text.to_owned();
    }
    for index in 1..lines.len() {
        if py::strip(lines[index]) == "---" {
            return lines[index + 1..]
                .join("\n")
                .trim_start_matches('\n')
                .to_owned();
        }
    }
    text.to_owned()
}

/// The prompt a condition sends. Without injection the runner itself supplies the setup
/// (plugins, rules files), so whole installations can be compared under their condition names.
pub fn condition_prompt(
    task: &str,
    condition: &str,
    skill: Option<&Path>,
    inject: bool,
) -> Result<String, String> {
    if condition == "baseline" || !inject {
        return Ok(task.to_owned());
    }
    let skill = skill
        .ok_or_else(|| format!("--condition-skill is required for the {condition} condition"))?;
    let instructions = strip_frontmatter(&read_text(skill)?);
    Ok(format!(
        "Follow the response-style skill below while completing the task. \
         Do not discuss or quote the skill.\n\n\
         <response_style>\n{instructions}\n</response_style>\n\n<task>\n{task}\n</task>"
    ))
}

fn no_get(value: &Value) -> Fault {
    Fault::Other(format!(
        "'{}' object has no attribute 'get'",
        type_name(value)
    ))
}

/// A runner's reply: its text, token usage and reported cost (None when not reported).
pub fn parse_response(output: &str, format: &Value) -> Result<(String, Value, Value), Fault> {
    match format.as_str() {
        Some("text") => Ok((py::strip(output).to_owned(), json!({}), Value::Null)),
        Some("claude-json") => {
            let body = output.trim_start_matches(py::is_space);
            let payload = serde_json::Deserializer::from_str(body)
                .into_iter::<Value>()
                .next()
                .unwrap_or_else(|| serde_json::from_str("")) // the end-of-input error
                .map_err(|e| Fault::Value(e.to_string()))?;
            let Value::Object(map) = &payload else {
                return Err(no_get(&payload));
            };
            let result = map.get("result").map_or_else(String::new, py::to_str);
            let usage = map
                .get("usage")
                .filter(|u| py::truthy(u))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let cost = map.get("total_cost_usd").cloned().unwrap_or(Value::Null);
            Ok((py::strip(&result).to_owned(), usage, cost))
        }
        Some("codex-jsonl") => {
            let mut events = Vec::new();
            for line in py::split_lines(output) {
                if !py::strip(line).is_empty() {
                    events.push(
                        serde_json::from_str::<Value>(line)
                            .map_err(|e| Fault::Value(e.to_string()))?,
                    );
                }
            }
            let mut text = Value::String(String::new());
            let mut usage = json!({});
            for event in &events {
                let Value::Object(map) = event else {
                    return Err(no_get(event));
                };
                let empty = json!({});
                let item = map.get("item").unwrap_or(&empty);
                let kind = map.get("type").and_then(Value::as_str);
                if kind == Some("item.completed") {
                    let Value::Object(fields) = item else {
                        return Err(no_get(item));
                    };
                    if fields.get("type").and_then(Value::as_str) == Some("agent_message") {
                        text = fields.get("text").cloned().unwrap_or(text);
                    }
                }
                if kind == Some("turn.completed") {
                    usage = map.get("usage").cloned().unwrap_or(usage);
                }
            }
            Ok((py::strip(&py::to_str(&text)).to_owned(), usage, Value::Null))
        }
        _ => Err(Fault::Value(format!(
            "Unsupported response format: {}",
            py::to_str(format)
        ))),
    }
}

/// The model a runner command asks for, only for a directly invoked CLI recognised here: a
/// wrapper's `-m` may select a Python module, not a model.
pub fn runner_model(command: &[String]) -> Option<String> {
    let executable = Path::new(command.first()?)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())?;
    if executable != "claude" && executable != "codex" {
        return None;
    }
    let mut model = None;
    let mut arguments = command[1..].iter();
    while let Some(arg) = arguments.next() {
        if arg == "--" {
            break;
        }
        if arg == "--model" || (executable == "codex" && arg == "-m") {
            model = arguments.next().cloned();
        } else if let Some(value) = arg.strip_prefix("--model=") {
            model = Some(value.to_owned());
        }
    }
    model.filter(|m| !m.is_empty() && !m.starts_with('-'))
}

/// `shlex.quote`.
pub fn shell_quote(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c);
    if word.is_empty() {
        "''".into()
    } else if word.chars().all(safe) {
        word.into()
    } else {
        format!("'{}'", word.replace('\'', "'\"'\"'"))
    }
}

/// `format(x, "g")`: 6 significant digits, trailing zeros dropped, exponent form below 1e-4
/// or from 1e6.
pub fn format_g(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "nan".into()
        } else if x > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let scientific = format!("{x:.5e}");
    let (mantissa, exponent) = scientific.split_once('e').expect("an exponent");
    let exponent: i32 = exponent.parse().expect("an exponent");
    let trim = |digits: &str| -> String {
        if digits.contains('.') {
            digits
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_owned()
        } else {
            digits.to_owned()
        }
    };
    if (-4..6).contains(&exponent) {
        let places = usize::try_from(5 - exponent).expect("0 to 9 places");
        trim(&format!("{x:.places$}"))
    } else {
        let sign = if exponent < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mantissa), exponent.abs())
    }
}

/// One model call, with a hang turned into an ordinary failed attempt (exit status 124): a CLI
/// that silently waits on provider retries would otherwise stall the whole condition. Output
/// that is not UTF-8 is a `Fault::Value`, as Python's UnicodeDecodeError was a ValueError; a
/// program that cannot start is a `Fault::Other`.
pub fn run_with_timeout(
    exec: Exec,
    command: &[String],
    input: &Input,
    cwd: &Path,
    timeout: f64,
) -> Result<(i32, String, String), Fault> {
    let limit = Duration::from_secs_f64(timeout);
    match exec(command, input, Some(cwd), Some(limit)) {
        Ok(done) => {
            let decode = |bytes: &[u8]| proc::text(bytes).map_err(Fault::Value);
            Ok((done.code, decode(&done.stdout)?, decode(&done.stderr)?))
        }
        Err(Failure::Timeout) => Ok((
            124,
            String::new(),
            format!("Timed out after {} seconds", format_g(timeout)),
        )),
        Err(spawn) => Err(Fault::Other(spawn.message(&command[0]))),
    }
}

/// An empty scratch directory for one runner call. Agent CLIs adopt their working directory as
/// project context: run one inside this checkout and it answers by inspecting the harness
/// instead of the task, and not equally across conditions.
pub fn neutral_cwd(prefix: &str) -> Result<tempfile::TempDir, String> {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .map_err(|e| e.to_string())
}

/// The pause before retry `attempt + 1`: Python's `min(2**attempt, 5)` seconds.
pub fn backoff(attempt: i64) -> Duration {
    Duration::from_secs(1 << attempt.clamp(0, 3)).min(Duration::from_secs(5))
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
    let prompt = condition_prompt(
        task,
        &args.condition,
        args.condition_skill.as_deref(),
        !args.no_injection,
    )?;
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
                py::print(
                    out,
                    &format!("skip completed {} trial {trial}: {id}", args.condition),
                )?;
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
