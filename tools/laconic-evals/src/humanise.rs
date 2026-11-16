// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Scores humanising responses with deterministic checkers, without a judge.
//!
//! For each response to a case in humanise-cases.jsonl, it measures the rewrite against the
//! source text in the prompt:
//!
//! - length: rewrite words as a share of source words;
//! - laconic-check lint findings and `preserve` drops (numbers, code, URLs and meaning words
//!   such as 'not' that the rewrite lost);
//! - runs of bold-label bullets, counted amongst the lint findings;
//! - optionally, the avoid-ai-writing detector's 0 to 100 score and finding count (`--avoid
//!   <path to bin/avoid-ai-writing.js>`), and dslop's finding count (`--dslop`, run through
//!   uvx with evals/humanise-dslop.toml).
//!
//! The same checks run on the source texts, so each condition can be read against where it
//! started. Every checker has a house view; a finding is a candidate.

use crate::cases::{parse, read_text};
use crate::fact_audit::Exec;
use crate::proc::{self, Input};
use crate::py::{self, Num};
use fancy_regex::Regex;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

static FINDING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^\S.*:\d+:\d+: ").unwrap());
static BOLD_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\S.*:\d+:\d+: bold-label: ").unwrap());
/// Pattern findings carry a position ('<stdin>:1:17 em-dash'); whole-text rhythm metrics do
/// not ('<stdin> sentence-length-cv=0.19 (threshold 0.30)').
pub static DSLOP_FINDING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<stdin>(?::\d+:\d+)? [a-z]").unwrap());
static DROPPED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^dropped ").unwrap());

/// Named measurements of one text, in the order they were taken.
pub type Metrics = Vec<(String, Num)>;

/// The text to humanise: everything after the instruction paragraph.
pub fn source_text(prompt: &str) -> Result<&str, String> {
    prompt
        .split_once("\n\n")
        .map(|(_, text)| text)
        .ok_or_else(|| "a prompt has no text after its instruction paragraph".into())
}

/// `len(text.split())`.
pub fn words(text: &str) -> usize {
    py::words(text).count()
}

fn count(pattern: &Regex, text: &str) -> i64 {
    i64::try_from(pattern.find_iter(text).count()).expect("a small count")
}

/// The production `Exec` for checkers: no working directory, 120 seconds.
pub fn exec(command: &[String], input: &Input) -> Result<proc::Output, proc::Failure> {
    proc::run(command, input, None, Some(Duration::from_secs(120)))
}

/// A checker's stdout. Exit 0 is success. With `findings`, exit 1 is success only when stdout
/// holds a finding, because 1 also reports an operational failure (uvx exits 1 when it cannot
/// launch). Any other status is an error rather than counting as zero findings.
pub fn run(
    exec: Exec,
    command: &[&str],
    text: Option<&str>,
    findings: Option<&Regex>,
) -> Result<String, String> {
    let command: Vec<String> = command.iter().map(|s| (*s).to_owned()).collect();
    let input = text.map_or(Input::Inherit, |t| Input::Bytes(t.as_bytes()));
    let done = exec(&command, &input).map_err(|e| e.message(&command[0]))?;
    let stdout = proc::text(&done.stdout)?;
    let found = done.code == 1 && findings.is_some_and(|f| f.is_match(&stdout).unwrap_or(false));
    if done.code != 0 && !found {
        let stderr = proc::text(&done.stderr)?;
        return Err(format!(
            "{} exited {}: {}",
            command.join(" "),
            done.code,
            py::strip(&stderr)
        ));
    }
    Ok(stdout)
}

/// laconic-check's lint report on `text`, every finding as a warning.
pub fn lint_output(exec: Exec, text: &str) -> Result<String, String> {
    run(
        exec,
        &["laconic-check", "lint", "--warn", "-"],
        Some(text),
        None,
    )
}

/// Protected items the rewrite dropped; preserve exits 1 when it finds any.
pub fn preserve_drops(exec: Exec, source: &str, rewrite: &str) -> Result<i64, String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let (original, edited) = (dir.path().join("original.md"), dir.path().join("edited.md"));
    fs::write(&original, source).map_err(|e| e.to_string())?;
    fs::write(&edited, rewrite).map_err(|e| e.to_string())?;
    let (original, edited) = (original.display().to_string(), edited.display().to_string());
    let out = run(
        exec,
        &["laconic-check", "preserve", &original, &edited],
        None,
        Some(&DROPPED),
    )?;
    Ok(count(&DROPPED, &out))
}

/// A JSON number as Python reads it: an int or a float.
fn number(value: &Value) -> Result<Num, String> {
    value
        .as_i64()
        .map(Num::Int)
        .or_else(|| value.as_f64().map(Num::Float))
        .ok_or_else(|| format!("not a number: {}", py::repr(value)))
}

/// The checkers to run on each text. dslop silently falls back to its defaults when its
/// config file is missing or is not valid TOML, so both are checked before measuring anything.
pub struct Checkers<'a> {
    pub avoid: Option<&'a Path>,
    pub dslop: Option<&'a Path>,
}

impl Checkers<'_> {
    pub fn check(&self) -> Result<(), String> {
        if let Some(config) = self.dslop {
            if !config.is_file() {
                return Err(format!("dslop config not found: {}", config.display()));
            }
            toml::from_str::<toml::Table>(&read_text(config)?)
                .map_err(|e| format!("{}: {e}", config.display()))?;
        }
        Ok(())
    }

    /// Every checker's measurements of `text`.
    pub fn measure(&self, exec: Exec, text: &str) -> Result<Metrics, String> {
        let lint = lint_output(exec, text)?;
        let mut result = vec![
            ("lint".to_owned(), Num::Int(count(&FINDING, &lint))),
            (
                "bold_label_runs".to_owned(),
                Num::Int(count(&BOLD_LABEL, &lint)),
            ),
        ];
        if let Some(avoid) = self.avoid {
            let avoid = avoid.display().to_string();
            let out = run(
                exec,
                &["node", &avoid, "--context", "general"],
                Some(text),
                None,
            )?;
            let report = parse(&out)?;
            let score = py::get(&report, "score").ok_or("the detector reported no score")?;
            let issues = py::get(&report, "issues").and_then(Value::as_array);
            let issues = issues.ok_or("the detector reported no issues list")?;
            result.push(("avoid_score".to_owned(), number(score)?));
            let found = i64::try_from(issues.len()).expect("a small count");
            result.push(("avoid_findings".to_owned(), Num::Int(found)));
        }
        if let Some(config) = self.dslop {
            let config = config.display().to_string();
            let out = run(
                exec,
                &["uvx", "dslop", "--config", &config, "-"],
                Some(text),
                Some(&DSLOP_FINDING),
            )?;
            result.push(("dslop".to_owned(), Num::Int(count(&DSLOP_FINDING, &out))));
        }
        Ok(result)
    }
}

/// Measures one text.
pub type Measure<'a> = &'a dyn Fn(&str) -> Result<Metrics, String>;
/// Counts what a rewrite dropped from its source.
pub type Drops<'a> = &'a dyn Fn(&str, &str) -> Result<i64, String>;

/// Mean metrics per condition, plus a 'source' row for the original texts.
pub fn summarise(
    rows: &[Value],
    cases: &BTreeMap<String, Value>,
    measure: Measure,
    drops: Drops,
) -> Result<Value, String> {
    let prompt = |case_id: &Value| -> Result<&str, String> {
        let case = &cases[&py::key(case_id)];
        source_text(py::text(case, "prompt")?)
    };
    let mut per: BTreeMap<String, Vec<Metrics>> = BTreeMap::new();
    let mut ids: Vec<&Value> = rows.iter().map(|row| &row["case_id"]).collect();
    ids.sort_by(|a, b| py::compare(a, b).unwrap_or(std::cmp::Ordering::Equal));
    ids.dedup_by(|a, b| py::key(a) == py::key(b));
    for case_id in ids {
        let mut metrics = measure(prompt(case_id)?)?;
        metrics.push(("length".to_owned(), Num::Float(1.0)));
        metrics.push(("drops".to_owned(), Num::Int(0)));
        per.entry("source".to_owned()).or_default().push(metrics);
    }
    for row in rows {
        let source = prompt(&row["case_id"])?;
        let rewrite = py::text(row, "response")?;
        let mut metrics = measure(rewrite)?;
        if words(source) == 0 {
            return Err("division by zero: a source text has no words".into());
        }
        #[expect(clippy::cast_precision_loss, reason = "word counts are below 2**53")]
        let length = words(rewrite) as f64 / words(source) as f64;
        metrics.push(("length".to_owned(), Num::Float(length)));
        metrics.push(("drops".to_owned(), Num::Int(drops(source, rewrite)?)));
        per.entry(py::text(row, "condition")?.to_owned())
            .or_default()
            .push(metrics);
    }
    let mut summary = Map::new();
    for (condition, items) in per {
        let mut entry = Map::new();
        entry.insert("rows".into(), Value::from(items.len()));
        for (name, _) in &items[0] {
            let values = items
                .iter()
                .map(|item| item.iter().find(|(n, _)| n == name).map(|(_, v)| *v));
            let values: Option<Vec<Num>> = values.collect();
            let values = values.ok_or_else(|| format!("a {condition} row has no {name}"))?;
            let mean = py::mean(values).expect("a row");
            entry.insert(name.clone(), py::round_num(mean, 3).to_value());
        }
        summary.insert(condition, Value::Object(entry));
    }
    Ok(Value::Object(summary))
}

/// The command: reads the cases and responses and returns the summary as printed.
pub fn command(
    responses: &Path,
    cases: &Path,
    checkers: &Checkers,
    exec: Exec,
) -> Result<String, String> {
    checkers.check()?;
    let cases: BTreeMap<String, Value> = crate::cases::load_cases(cases)?
        .into_iter()
        .map(|case| (py::key(&case["id"]), case))
        .collect();
    let rows: Vec<Value> = crate::cases::read_jsonl(responses)?
        .into_iter()
        .filter(|row| cases.contains_key(&py::key(&row["case_id"])))
        .collect();
    let measure = |text: &str| checkers.measure(exec, text);
    let drops = |source: &str, rewrite: &str| preserve_drops(exec, source, rewrite);
    Ok(py::dumps_pretty(&summarise(
        &rows, &cases, &measure, &drops,
    )?))
}

#[cfg(test)]
mod tests;
