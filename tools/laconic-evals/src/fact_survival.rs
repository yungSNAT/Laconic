// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Measure whether rewrites keep each fact and qualifier in a source text.
//!
//! Ledgers (`fact-ledgers.jsonl`) are written from the sources alone, before any rewrite
//! exists. A blind judge sees one rewrite and the ledger items, never the setup's name, and
//! must quote the rewrite for every item it calls kept. A quote that is not in the rewrite
//! counts as dropped, so a judge cannot invent support.

use crate::cases::{append, parse, read_text};
use crate::fact_audit::Exec;
use crate::humanise::{source_text, words};
use crate::proc::{self, Input};
use crate::py::{self, Num, field, text, type_name};
use fancy_regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, mpsc};
use std::thread;
use std::time::Duration;

pub const INSTRUCTION: &str =
    "Rewrite this text for a busy Australian reader. Reply with the rewritten text only.";
pub const JUDGE_COMMAND: [&str; 12] = [
    "claude",
    "--print",
    "--output-format",
    "json",
    "--no-session-persistence",
    "--strict-mcp-config",
    "--setting-sources",
    "",
    "--tools",
    "",
    "--model",
    "claude-sonnet-5-5",
];
pub const VERDICTS: [&str; 3] = ["kept", "hardened", "dropped"];
pub const SETUPS: [&str; 8] = [
    "laconic-prose",
    "laconic-session",
    "humanizer",
    "avoid",
    "stop-slop",
    "caveman",
    "concise-control",
    "no-skill",
];
static ADDED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^added ").unwrap());

/// Ledger items per case id.
pub type Ledgers = BTreeMap<String, Vec<Value>>;
/// Source texts per case id.
pub type Sources = BTreeMap<String, String>;
/// One judge call: the reply's `result` and what the call cost.
pub type Call<'a> = &'a (dyn Fn(&str) -> Result<(Value, f64), String> + Sync);
/// Protected items a rewrite added, as preserve reports them.
pub type Added<'a> = &'a dyn Fn(&str, &str) -> Result<i64, String>;

/// A row's retries ran out; carries what those calls cost.
#[derive(Debug, PartialEq)]
pub struct JudgeFailed {
    pub message: String,
    pub cost: f64,
}

/// Why a judge reply could not be read. Python retried a ValueError, KeyError or
/// AttributeError; any other error (a TypeError) ended the row at once without its cost.
#[derive(Debug, PartialEq)]
pub enum ReplyError {
    Retry(String),
    Fatal(String),
}

impl ReplyError {
    fn message(self) -> String {
        match self {
            ReplyError::Retry(message) | ReplyError::Fatal(message) => message,
        }
    }
}

/// The rows of a JSON Lines file, skipping blank lines.
pub fn read_jsonl(path: &Path) -> Result<Vec<Value>, String> {
    let text = read_text(path)?;
    py::split_lines(&text)
        .into_iter()
        .filter(|line| !py::strip(line).is_empty())
        .map(|line| parse(line).map_err(|e| format!("{}: {e}", path.display())))
        .collect()
}

/// A humanise case id as its fact case id.
fn fact_id(id: &str) -> String {
    id.replacen("hum-", "fact-", 1)
}

/// The same source texts under one instruction no setup is named in.
pub fn build_cases(sources: &[Value]) -> Result<Vec<Value>, String> {
    sources
        .iter()
        .map(|case| {
            let id = fact_id(text(case, "id")?);
            let prompt = format!("{INSTRUCTION}\n\n{}", source_text(text(case, "prompt")?)?);
            Ok(py::object([
                ("id", Value::from(id)),
                ("category", json!("rewrite")),
                ("prompt", Value::from(prompt)),
                ("risk", field(case, "risk")?.clone()),
                (
                    "criteria",
                    json!(["Keeps every fact and qualifier in the ledger."]),
                ),
            ]))
        })
        .collect()
}

pub fn load_ledgers(path: &Path) -> Result<Ledgers, String> {
    let mut ledgers = Ledgers::new();
    for row in read_jsonl(path)? {
        let case_id = fact_id(text(&row, "case_id")?);
        if ledgers.contains_key(&case_id) {
            return Err(format!("duplicate ledger: {case_id}"));
        }
        let items = field(&row, "items")?
            .as_array()
            .ok_or("a ledger's items are not a list")?;
        for item in items {
            let kind = field(item, "kind")?.as_str();
            if !matches!(kind, Some("fact" | "qualifier"))
                || py::strip(text(item, "text")?).is_empty()
            {
                return Err(format!("bad item in {case_id}: {}", py::repr(item)));
            }
        }
        ledgers.insert(case_id, items.clone());
    }
    Ok(ledgers)
}

pub fn judge_prompt(source: &str, rewrite: &str, items: &[Value]) -> Result<String, String> {
    let lines: Result<Vec<String>, String> = items
        .iter()
        .map(|item| {
            let part = |name| field(item, name).map(py::to_str);
            Ok(format!(
                "{} ({}): {}",
                part("id")?,
                part("kind")?,
                part("text")?
            ))
        })
        .collect();
    let listing = lines?.join("\n");
    Ok(format!(
        "You check whether a rewritten text still carries specific claims. Judge only the \
         text below; ignore its style and length.\n\n\
         For each item return one verdict:\n\
         - kept: the text carries the claim, in any wording, including its condition or hedge. \
         'three' and '3' are the same.\n\
         - hardened: the text carries the claim but states it more strongly or more widely \
         than the item, because a hedge, condition, exception or limit is gone.\n\
         - dropped: the text does not carry the claim.\n\n\
         For kept and hardened, give `quote`: the shortest exact passage of the text that \
         carries the claim, copied character for character. For dropped, give an empty quote.\n\n\
         Also list under `invented` each exact passage of the text that states a material fact, \
         figure, promise or experience the source does not support. Ignore rewording, tone and \
         reasonable inference.\n\n\
         Reply with JSON only: {{\"items\": [{{\"id\": \"f1\", \"verdict\": \"kept\", \"quote\": \"...\"}}], \"invented\": [\"...\"]}}\n\n\
         <source>\n{source}\n</source>\n\n<text>\n{rewrite}\n</text>\n\nItems:\n{listing}\n"
    ))
}

/// Curly quotes as straight ones, whitespace runs as one space, then Unicode full case
/// folding, as Python's `str.casefold`.
pub fn normalise(text: &str) -> String {
    let straight: String = text
        .chars()
        .map(|c| match c {
            '\u{2018}' | '\u{2019}' => '\'',
            '\u{201c}' | '\u{201d}' => '"',
            other => other,
        })
        .collect();
    let joined = py::words(&straight).collect::<Vec<_>>().join(" ");
    icu_casemap::CaseMapper::new()
        .fold_string(&joined)
        .into_owned()
}

/// The reply's ids mapped to their rows, as `{row.get("id"): row for row in parsed["items"]}`.
fn given_rows(parsed: &Value) -> Result<BTreeMap<String, &Value>, ReplyError> {
    let items = py::get(parsed, "items").ok_or_else(|| ReplyError::Retry("'items'".into()))?;
    let rows: Vec<&Value> = match items {
        Value::Array(rows) => rows.iter().collect(),
        Value::String(s) if s.is_empty() => Vec::new(),
        Value::Object(map) if map.is_empty() => Vec::new(),
        Value::String(_) | Value::Object(_) => {
            return Err(ReplyError::Retry(
                "'str' object has no attribute 'get'".into(),
            ));
        }
        other => {
            let message = format!("'{}' object is not iterable", type_name(other));
            return Err(ReplyError::Fatal(message));
        }
    };
    let mut given = BTreeMap::new();
    for row in rows {
        let Value::Object(map) = row else {
            let message = format!("'{}' object has no attribute 'get'", type_name(row));
            return Err(ReplyError::Retry(message));
        };
        let id = map.get("id").unwrap_or(&Value::Null);
        if id.is_array() || id.is_object() {
            return Err(ReplyError::Fatal(format!(
                "unhashable type: '{}'",
                type_name(id)
            )));
        }
        given.insert(py::key(id), row);
    }
    Ok(given)
}

/// Verdicts and verified invented passages. An unverifiable quote or a missing item is dropped.
pub fn parse_reply(
    reply: &Value,
    rewrite: &str,
    items: &[Value],
) -> Result<(Vec<Value>, Vec<String>), ReplyError> {
    let Value::String(reply) = reply else {
        let message = format!(
            "expected string or bytes-like object, got '{}'",
            type_name(reply)
        );
        return Err(ReplyError::Fatal(message));
    };
    let span = reply
        .find('{')
        .zip(reply.rfind('}'))
        .filter(|(start, end)| start < end);
    let (start, end) = span.ok_or_else(|| ReplyError::Retry("judge reply holds no JSON".into()))?;
    let parsed = parse(&reply[start..=end]).map_err(ReplyError::Retry)?;
    let given = given_rows(&parsed)?;
    let body = normalise(rewrite);
    let empty = Value::Object(Map::new());
    let mut verdicts = Vec::new();
    for item in items {
        let id = field(item, "id").map_err(ReplyError::Retry)?;
        let row = given.get(&py::key(id)).copied().unwrap_or(&empty);
        let verdict = py::get(row, "verdict")
            .cloned()
            .unwrap_or_else(|| json!("dropped"));
        let verdict = match verdict {
            Value::String(v) if VERDICTS.contains(&v.as_str()) => v,
            Value::Array(_) | Value::Object(_) => {
                let message = format!("unhashable type: '{}'", type_name(&verdict));
                return Err(ReplyError::Fatal(message));
            }
            other => {
                return Err(ReplyError::Retry(format!(
                    "unknown verdict: {}",
                    py::to_str(&other)
                )));
            }
        };
        let quote = py::get(row, "quote")
            .filter(|q| py::truthy(q))
            .map(py::to_str)
            .unwrap_or_default();
        let unverified = verdict != "dropped"
            && (py::strip(&quote).is_empty() || !body.contains(&normalise(&quote)));
        verdicts.push(py::object([
            ("id", id.clone()),
            (
                "kind",
                field(item, "kind").map_err(ReplyError::Retry)?.clone(),
            ),
            (
                "verdict",
                Value::from(if unverified {
                    "dropped"
                } else {
                    verdict.as_str()
                }),
            ),
            ("judge_verdict", Value::from(verdict.as_str())),
            ("unverified_quote", Value::from(unverified)),
            ("quote", Value::from(quote)),
        ]));
    }
    let Some(Value::Array(invented)) = py::get(&parsed, "invented") else {
        return Err(ReplyError::Retry("judge reply has no invented list".into()));
    };
    let mut seen = HashSet::new();
    let found = invented
        .iter()
        .filter_map(Value::as_str)
        .filter(|q| !py::strip(q).is_empty())
        .map(normalise)
        .filter(|q| seen.insert(q.clone()))
        .filter(|q| body.contains(q.as_str()))
        .collect();
    Ok((verdicts, found))
}

/// Python's `float(value or 0)` for a JSON value.
pub fn to_float(value: Option<&Value>) -> Result<f64, String> {
    match value.filter(|v| py::truthy(v)) {
        None => Ok(0.0),
        Some(Value::Bool(_)) => Ok(1.0),
        Some(Value::Number(n)) => Ok(n.as_f64().expect("a JSON number")),
        Some(Value::String(s)) => py::strip(s)
            .parse()
            .map_err(|_| format!("could not convert string to float: {}", py::str_repr(s))),
        Some(other) => Err(format!(
            "float() argument must be a string or a real number, not '{}'",
            type_name(other)
        )),
    }
}

/// One judge call through the claude CLI: its `result` and its reported cost.
pub fn call_judge(
    prompt: &str,
    command: &[String],
    timeout: Duration,
) -> Result<(Value, f64), String> {
    let done = proc::run(
        command,
        &Input::Bytes(prompt.as_bytes()),
        None,
        Some(timeout),
    )
    .map_err(|e| e.message(&command[0]))?;
    let stdout = proc::text(&done.stdout)?;
    if done.code != 0 {
        let stderr = proc::text(&done.stderr)?;
        let shown: String = py::strip(&stderr).chars().take(300).collect();
        return Err(format!("judge exited {}: {shown}", done.code));
    }
    let reply = parse(&stdout)?;
    let result = py::get(&reply, "result").ok_or("the judge's reply has no 'result'")?;
    Ok((result.clone(), to_float(py::get(&reply, "total_cost_usd"))?))
}

/// The resume key of a row: its setup, case and trial, equal where Python's tuples are.
fn row_key(row: &Value, setup: &str) -> Result<[String; 3], String> {
    Ok([
        py::key(field(row, setup)?),
        py::key(field(row, "case_id")?),
        py::key(field(row, "trial")?),
    ])
}

/// Judges one row, retrying a reply it cannot read up to 3 calls in all.
fn judge_one(
    row: &Value,
    ledgers: &Ledgers,
    sources: &Sources,
    call: Call,
) -> Result<Value, JudgeFailed> {
    let (runner, case_id) = (&row["runner"], &row["case_id"]);
    let label = format!("{} {}", py::to_str(runner), py::to_str(case_id));
    let items = &ledgers[case_id.as_str().expect("a case with a ledger")];
    let failed = |error: String, cost: f64| JudgeFailed {
        message: format!("{label}: {error}"),
        cost,
    };
    // The prompt's inputs are read before each call, as Python built its arguments.
    let ask = || -> Result<(Value, f64, &str), String> {
        let source = sources.get(case_id.as_str().expect("a case with a ledger"));
        let source = source.ok_or_else(|| py::str_repr(&py::to_str(case_id)))?;
        let response = text(row, "response")?;
        let (reply, spent) = call(&judge_prompt(source, response, items)?)?;
        Ok((reply, spent, response))
    };
    let mut cost = 0.0;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let (reply, spent, response) = ask().map_err(|error| failed(error, cost))?;
        cost += spent;
        match parse_reply(&reply, response, items) {
            Ok((verdicts, invented)) => {
                return Ok(py::object([
                    ("setup", runner.clone()),
                    ("case_id", case_id.clone()),
                    ("trial", row["trial"].clone()),
                    ("cost", py::float(cost)),
                    ("verdicts", Value::from(verdicts)),
                    ("invented", Value::from(invented)),
                ]));
            }
            Err(ReplyError::Fatal(error)) => return Err(failed(error, 0.0)),
            Err(ReplyError::Retry(error)) if attempt == 3 => return Err(failed(error, cost)),
            Err(ReplyError::Retry(_)) => {}
        }
    }
}

/// Judge rows not yet in `output`; resumable. Returns the cost of this run.
///
/// Up to `jobs` rows are judged at once, in a fixed shuffled order that spreads a budget stop
/// across setups. Once the cost reaches the budget, or a row fails, no new row starts; rows
/// already running finish and are saved. A failed row's cost goes to overhead.jsonl beside
/// `output`, and the first failure is returned once every running row has finished.
pub fn judge_rows<'a>(
    rows: &'a [Value],
    ledgers: &Ledgers,
    sources: &Sources,
    output: &Path,
    call: Call,
    jobs: i64,
    budget_usd: f64,
) -> Result<f64, String> {
    let mut done = HashSet::new();
    if output.exists() {
        for row in read_jsonl(output)? {
            done.insert(row_key(&row, "setup")?);
        }
    }
    let mut todo = Vec::new();
    for row in rows {
        if !done.contains(&row_key(row, "runner")?) {
            todo.push(row);
        }
    }
    let mut missing: Vec<&Value> = todo
        .iter()
        .map(|row| &row["case_id"])
        .filter(|id| !id.as_str().is_some_and(|id| ledgers.contains_key(id)))
        .collect();
    missing.sort_by(|a, b| py::compare(a, b).unwrap_or(std::cmp::Ordering::Equal));
    missing.dedup_by(|a, b| py::key(a) == py::key(b));
    if !missing.is_empty() {
        let shown: Vec<Value> = missing.into_iter().cloned().collect();
        return Err(format!("no ledger for: {}", py::repr(&Value::from(shown))));
    }
    py::Random::new(0).shuffle(&mut todo);
    let jobs = usize::try_from(jobs).ok().filter(|&jobs| jobs > 0);
    let jobs = jobs.ok_or("max_workers must be greater than 0")?;
    append(output, "")?;
    let overhead = output
        .parent()
        .unwrap_or(Path::new(""))
        .join("overhead.jsonl");
    let (sender, receiver) = mpsc::channel();
    let mut pending = todo.into_iter();
    let mut spent = 0.0;
    let mut failure: Option<String> = None;
    let mut error = None;
    thread::scope(|scope| {
        let submit = |row: &'a Value| {
            let sender = sender.clone();
            scope.spawn(move || sender.send(judge_one(row, ledgers, sources, call)));
        };
        let mut running = 0;
        for row in pending.by_ref().take(jobs) {
            submit(row);
            running += 1;
        }
        while running > 0 {
            running -= 1;
            let written = match receiver.recv().expect("a running row reports") {
                Err(failed) => {
                    if failure.is_none() {
                        failure = Some(failed.message);
                    }
                    let line = json!({"stage": "failed-judge", "cost_usd": py::float(failed.cost)});
                    append(&overhead, &format!("{}\n", py::dumps(&line, None, true)))
                }
                Ok(result) => {
                    spent += result["cost"].as_f64().expect("a cost");
                    let written = append(output, &format!("{}\n", py::dumps_row(&result)));
                    if written.is_ok()
                        && spent < budget_usd
                        && failure.is_none()
                        && let Some(row) = pending.next()
                    {
                        submit(row);
                        running += 1;
                    }
                    written
                }
            };
            if let Err(e) = written {
                error = Some(e);
                break;
            }
        }
    });
    match (error, failure) {
        (Some(error), _) | (None, Some(error)) => Err(error),
        (None, None) => Ok(spent),
    }
}

/// The judge against hand-labelled rewrites. '@source' stands for a verbatim copy.
pub fn calibrate(
    fixtures: &[Value],
    ledgers: &Ledgers,
    sources: &Sources,
    call: Call,
) -> Result<Value, String> {
    let (mut errors, mut compared, mut cost) = (Vec::new(), 0u64, 0.0);
    for fixture in fixtures {
        let case_id = field(fixture, "case_id")?;
        let missing = || format!("no case {}", py::repr(case_id));
        let source = case_id
            .as_str()
            .and_then(|id| sources.get(id))
            .ok_or_else(missing)?;
        let rewrite = match field(fixture, "rewrite")? {
            Value::String(r) if r == "@source" => source.as_str(),
            _ => text(fixture, "rewrite")?,
        };
        let items = case_id
            .as_str()
            .and_then(|id| ledgers.get(id))
            .ok_or_else(missing)?;
        let (reply, spent) = call(&judge_prompt(source, rewrite, items)?)?;
        cost += spent;
        let (verdicts, invented) =
            parse_reply(&reply, rewrite, items).map_err(ReplyError::message)?;
        for verdict in verdicts {
            let expected = field(fixture, "expected")?;
            if !expected.is_object() {
                return Err(format!(
                    "'expected' is not an object in {}",
                    py::repr(fixture)
                ));
            }
            let default = field(fixture, "default")?;
            let given = verdict["id"].as_str().and_then(|id| py::get(expected, id));
            let expected = given.unwrap_or(default);
            compared += 1;
            if &verdict["verdict"] != expected {
                errors.push(py::object([
                    ("fixture", field(fixture, "name")?.clone()),
                    ("item", verdict["id"].clone()),
                    ("expected", expected.clone()),
                    ("got", verdict["verdict"].clone()),
                ]));
            }
        }
        compared += 1;
        let wanted = field(fixture, "invented")?;
        if py::key(&Value::from(!invented.is_empty())) != py::key(wanted) {
            errors.push(py::object([
                ("fixture", field(fixture, "name")?.clone()),
                ("item", json!("invented")),
                ("expected", wanted.clone()),
                ("got", Value::from(invented)),
            ]));
        }
    }
    if compared == 0 {
        return Err("division by zero: no calibration fixtures".into());
    }
    #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
    let agreement = 1.0 - errors.len() as f64 / compared as f64;
    Ok(py::object([
        ("compared", Value::from(compared)),
        ("agreement", py::float(agreement)),
        ("errors", Value::from(errors)),
        ("cost_usd", py::float(cost)),
    ]))
}

/// A file name as the platform's glob and sort compare it.
#[cfg(not(windows))]
fn fold(name: &str) -> String {
    name.to_owned()
}

#[cfg(windows)]
fn fold(name: &str) -> String {
    name.to_lowercase()
}

/// `sorted(folder.glob("run-*.jsonl"))`: an unreadable folder has none.
fn run_files(folder: &Path) -> Vec<PathBuf> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            let name = fold(name);
            name.len() >= 10 && name.starts_with("run-") && name.ends_with(".jsonl")
        })
        .collect();
    names.sort_by_key(|name| fold(name));
    names.into_iter().map(|name| folder.join(name)).collect()
}

/// Reported cost of every stage so far: overhead, generation and judging.
pub fn total_spend(folder: &Path) -> Result<f64, String> {
    let mut total = 0.0;
    let mut paths = run_files(folder);
    paths.extend([folder.join("overhead.jsonl"), folder.join("scores.jsonl")]);
    for path in paths.into_iter().filter(|path| path.exists()) {
        let mut costs = Vec::new();
        for row in read_jsonl(&path)? {
            if !row.is_object() {
                return Err(format!("{}: a row is not an object", path.display()));
            }
            let cost = py::get(&row, "cost_usd")
                .filter(|c| py::truthy(c))
                .or_else(|| py::get(&row, "cost"));
            costs.push(to_float(cost)?);
        }
        total += py::sum(costs);
    }
    Ok(total)
}

/// The production preserve call for `summarise`: how many items the rewrite added.
pub fn added_count(exec: Exec, source: &str, rewrite: &str) -> Result<i64, String> {
    let (_dir, files) = crate::fact_audit::preserve_files(source, rewrite)?;
    let command: Vec<String> = ["laconic-check", "preserve"]
        .into_iter()
        .map(str::to_owned)
        .chain(files)
        .collect();
    let done = exec(&command, &Input::Inherit).map_err(|e| e.message("laconic-check"))?;
    let stdout = proc::text(&done.stdout)?;
    if done.code != 0 && done.code != 1 {
        let stderr = proc::text(&done.stderr)?;
        return Err(format!(
            "preserve exited {}: {}",
            done.code,
            py::strip(&stderr)
        ));
    }
    Ok(i64::try_from(ADDED.find_iter(&stdout).count()).expect("a small count"))
}

fn rate(verdicts: &[&Value], kind: &str) -> Result<Value, String> {
    let mut chosen = Vec::new();
    for verdict in verdicts {
        if field(verdict, "kind")? == kind {
            chosen.push(field(verdict, "verdict")? == "kept");
        }
    }
    let kept = chosen.iter().filter(|&&kept| kept).count();
    #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
    let rate = (!chosen.is_empty()).then(|| py::float(kept as f64 / chosen.len() as f64));
    Ok(rate.unwrap_or(Value::Null))
}

/// Per-text survival rates for each case.
pub type PerCase = BTreeMap<String, Vec<f64>>;

/// Mean a minus b, with a 95% bootstrap interval that resamples whole texts.
pub fn paired_interval(
    a: &PerCase,
    b: &PerCase,
    rounds: usize,
    seed: u32,
) -> Result<(f64, f64, f64), String> {
    let cases: Vec<&String> = a.keys().filter(|case| b.contains_key(*case)).collect();
    if cases.is_empty() {
        return Err("no shared cases".into());
    }
    let mean = |values: &[f64]| py::mean_f64(values.iter().copied()).expect("a value");
    let gaps: BTreeMap<&String, f64> = cases
        .iter()
        .map(|&case| (case, mean(&a[case]) - mean(&b[case])))
        .collect();
    let mut rng = py::Random::new(seed);
    let mut draws: Vec<f64> = (0..rounds)
        .map(|_| {
            let picked: Vec<f64> = cases.iter().map(|_| gaps[*rng.choice(&cases)]).collect();
            mean(&picked)
        })
        .collect();
    draws.sort_by(|x, y| x.partial_cmp(y).expect("finite gaps"));
    #[expect(clippy::cast_precision_loss, reason = "rounds are below 2**53")]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "int() of a small positive float"
    )]
    let at = |share: f64| {
        draws
            .get((share * rounds as f64) as usize)
            .copied()
            .ok_or("list index out of range")
    };
    let all: Vec<f64> = gaps.values().copied().collect();
    Ok((mean(&all), at(0.025)?, at(0.975)?))
}

/// A setup's coverage: its (case, trial) pairs, sorted by key so equal multisets compare equal.
fn coverage<'a>(rows: impl IntoIterator<Item = &'a Value>) -> Result<Vec<[String; 2]>, String> {
    let mut pairs = rows
        .into_iter()
        .map(|row| {
            Ok([
                py::key(field(row, "case_id")?),
                py::key(field(row, "trial")?),
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    pairs.sort();
    Ok(pairs)
}

/// The responses of one setup; a response without a runner is an error.
fn answers_of<'a>(responses: &'a [Value], setup: &str) -> Result<Vec<&'a Value>, String> {
    let mut answers = Vec::new();
    for response in responses {
        if field(response, "runner")? == setup {
            answers.push(response);
        }
    }
    Ok(answers)
}

fn length_share(answers: &[&Value], sources: &Sources) -> Result<f64, String> {
    let mut shares = Vec::new();
    for answer in answers {
        let source = &sources[text(answer, "case_id")?];
        if words(source) == 0 {
            return Err("division by zero: a source text has no words".into());
        }
        #[expect(clippy::cast_precision_loss, reason = "word counts are below 2**53")]
        shares.push(words(text(answer, "response")?) as f64 / words(source) as f64);
    }
    Ok(py::mean_f64(shares).expect("a response"))
}

fn length_of(value: &Value) -> Result<i64, String> {
    let length = match value {
        Value::Array(items) => items.len(),
        Value::Object(map) => map.len(),
        Value::String(s) => s.chars().count(),
        other => {
            return Err(format!(
                "object of type '{}' has no len()",
                type_name(other)
            ));
        }
    };
    Ok(i64::try_from(length).expect("a small length"))
}

/// One setup's summary, without its reference gap.
fn summarise_setup(
    rows: &[&Value],
    answers: &[&Value],
    sources: &Sources,
    added: Added,
) -> Result<(Value, PerCase), String> {
    let mut verdicts = Vec::new();
    let mut cases = PerCase::new();
    for row in rows {
        let marks = field(row, "verdicts")?
            .as_array()
            .ok_or("a row's verdicts are not a list")?;
        let kept = marks
            .iter()
            .map(|v| Ok(field(v, "verdict")? == "kept"))
            .collect::<Result<Vec<bool>, String>>()?;
        if kept.is_empty() {
            return Err("division by zero: a row has no verdicts".into());
        }
        #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
        let share = kept.iter().filter(|&&k| k).count() as f64 / kept.len() as f64;
        cases
            .entry(text(row, "case_id")?.to_owned())
            .or_default()
            .push(share);
        verdicts.extend(marks);
    }
    let count = |verdict: &str| verdicts.iter().filter(|v| v["verdict"] == verdict).count();
    let mut unverified = 0;
    for verdict in &verdicts {
        let flag = field(verdict, "unverified_quote")?
            .as_bool()
            .ok_or("an unverified_quote is not a boolean")?;
        unverified += usize::from(flag);
    }
    let by_text = cases
        .values()
        .map(|values| py::mean_f64(values.iter().copied()).expect("a row"));
    let invented = rows
        .iter()
        .map(|row| field(row, "invented").and_then(length_of).map(Num::Int));
    let invented = invented.collect::<Result<Vec<Num>, String>>()?;
    let mut added_counts = Vec::new();
    let mut costs = Vec::new();
    for answer in answers {
        added_counts.push(Num::Int(added(
            &sources[text(answer, "case_id")?],
            text(answer, "response")?,
        )?));
        costs.push(to_float(py::get(answer, "cost_usd"))?);
    }
    let summary = py::object([
        ("texts", Value::from(rows.len())),
        (
            "survival_by_text",
            py::float(py::mean_f64(by_text).expect("a case")),
        ),
        ("fact_survival", rate(&verdicts, "fact")?),
        ("qualifier_survival", rate(&verdicts, "qualifier")?),
        ("hardened", Value::from(count("hardened"))),
        ("dropped", Value::from(count("dropped"))),
        ("unverified_quotes", Value::from(unverified)),
        ("items", Value::from(verdicts.len())),
        ("length_share", py::float(length_share(answers, sources)?)),
        (
            "invented_per_text",
            py::mean(invented).expect("a row").to_value(),
        ),
        (
            "added_per_text",
            py::mean(added_counts).expect("a response").to_value(),
        ),
        ("generation_cost", py::float(py::sum(costs))),
    ]);
    Ok((summary, cases))
}

/// Each setup's survival rates, counts, length and cost, with its gap to the reference setup.
/// Every setup must cover every (case, trial) once, in both scores and responses.
pub fn summarise(
    scores: &[Value],
    responses: &[Value],
    sources: &Sources,
    reference: &str,
    added: Added,
    trials: i64,
    setups: Option<&BTreeSet<String>>,
) -> Result<Value, String> {
    let mut by_setup: Vec<(String, Vec<&Value>)> = Vec::new();
    for score in scores {
        let setup = text(score, "setup")?;
        match by_setup.iter_mut().find(|(name, _)| name == setup) {
            Some((_, rows)) => rows.push(score),
            None => by_setup.push((setup.to_owned(), vec![score])),
        }
    }
    if !by_setup.iter().any(|(name, _)| name == reference) {
        return Err(format!("reference setup has no scores: {reference}"));
    }
    let names: BTreeSet<String> = by_setup.iter().map(|(name, _)| name.clone()).collect();
    if let Some(setups) = setups.filter(|&setups| *setups != names) {
        let differ: Vec<Value> = names
            .symmetric_difference(setups)
            .map(|n| Value::from(n.as_str()))
            .collect();
        return Err(format!(
            "setups differ from the plan: {}",
            py::repr(&Value::from(differ))
        ));
    }
    let mut expected: Vec<[String; 2]> = sources
        .keys()
        .flat_map(|case| {
            (1..=trials).map(move |trial| [py::key(&Value::from(case.as_str())), trial.to_string()])
        })
        .collect();
    expected.sort();
    for (setup, rows) in &by_setup {
        let answered = coverage(answers_of(responses, setup)?)?;
        if coverage(rows.iter().copied())? != expected || answered != expected {
            return Err(format!(
                "incomplete, duplicate or unequal coverage: {setup}"
            ));
        }
    }
    by_setup.sort_by(|(a, _), (b, _)| a.cmp(b));
    let mut summary = Map::new();
    let mut per_case = BTreeMap::new();
    for (setup, rows) in &by_setup {
        let (entry, cases) = summarise_setup(rows, &answers_of(responses, setup)?, sources, added)?;
        summary.insert(setup.clone(), entry);
        per_case.insert(setup.as_str(), cases);
    }
    for (setup, cases) in &per_case {
        if *setup != reference {
            let (gap, low, high) = paired_interval(&per_case[reference], cases, 10_000, 0)?;
            let gap = py::object([
                ("mean", py::float(gap)),
                ("low", py::float(low)),
                ("high", py::float(high)),
            ]);
            summary[*setup]["reference_gap"] = gap;
        }
    }
    Ok(Value::Object(summary))
}

/// Source texts by case id.
pub fn sources(path: &Path) -> Result<Sources, String> {
    let mut sources = Sources::new();
    for case in read_jsonl(path)? {
        sources.insert(
            text(&case, "id")?.to_owned(),
            source_text(text(&case, "prompt")?)?.to_owned(),
        );
    }
    Ok(sources)
}

/// Python's `f"{x:.2f}"`.
pub fn money(x: f64) -> String {
    if x.is_nan() {
        "nan".into()
    } else {
        format!("{x:.2}")
    }
}

#[cfg(test)]
mod tests;
