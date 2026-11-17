// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The steps of the dated benchmark runs in `evals/runs/`, which each run's `run.sh` calls:
//! the runner configurations, the answer files, one row per answer or rewrite, and the
//! published figures recomputed from those rows. Each writes the bytes the run's Python wrote.

use crate::cases::{self, read_jsonl, read_text};
use crate::py::{self, Num, Random};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The run folders `record setups` writes configurations for.
pub const RUNS: [&str; 3] = [
    "concise-style-2026-11-17",
    "token-cost-2026-11-17",
    "rival-rewrites-2026-11-17",
];

/// The Claude Code flags every run used, after the program.
const FLAGS: [&str; 13] = [
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
];

/// One runner configuration: `program`, the flags (ending `--tools ""`) and `extra`.
fn runner(program: &str, extra: &[String]) -> Value {
    let command: Vec<String> = [program]
        .iter()
        .chain(FLAGS.iter())
        .map(|s| (*s).to_owned())
        .chain([String::new()])
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
fn usage_field(usage: Option<&Value>, key: &str) -> Value {
    usage
        .filter(|u| py::truthy(u))
        .and_then(|u| py::get(u, key))
        .cloned()
        .unwrap_or(json!(0))
}

/// The case's regex checks the answer fails, or null when the case has none.
fn regex_failed(case: &Value, text: &str) -> Result<Value, String> {
    let Some(checks) = py::get(case, "checks").filter(|c| py::truthy(c)) else {
        return Ok(Value::Null);
    };
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
        if !cases::found(p, text) {
            failed.push(format!("missing {}", py::str_repr(p)));
        }
    }
    for p in patterns("must_not_include")? {
        if cases::found(p, text) {
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
            "input_tokens": usage_field(usage, "input_tokens"),
            "cache_write_tokens": usage_field(usage, "cache_creation_input_tokens"),
            "cache_read_tokens": usage_field(usage, "cache_read_input_tokens"),
            "output_tokens": usage_field(usage, "output_tokens"),
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

/// A JSON number or bool as Python adds it: bools are the ints 0 and 1.
fn num(value: &Value) -> Result<Num, String> {
    match value {
        Value::Bool(b) => Ok(Num::Int(i64::from(*b))),
        Value::Number(n) => match n.as_i64() {
            Some(i) => Ok(Num::Int(i)),
            None => n
                .as_f64()
                .map(Num::Float)
                .ok_or(format!("{n} is out of range")),
        },
        other => Err(format!(
            "unsupported operand: {} is not a number",
            py::repr(other)
        )),
    }
}

fn num_at(row: &Value, path: &[&str]) -> Result<Num, String> {
    let mut value = row;
    for key in path {
        value = py::field(value, key)?;
    }
    num(value)
}

/// `a / b` as Python's true division.
fn divide(a: Num, b: Num) -> Result<f64, String> {
    if b.as_f64() == 0.0 {
        return Err("division by zero".into());
    }
    Ok(a.as_f64() / b.as_f64())
}

/// `Decimal(repr(x)).quantize(Decimal(1).scaleb(-places), ROUND_HALF_UP)`, as text. Up to 6
/// places, where Python writes no exponent.
fn half_up(x: Num, places: usize) -> Result<String, String> {
    assert!(places <= 6, "Python writes an exponent past 6 places");
    let text = match x {
        Num::Int(i) => i.to_string(),
        Num::Float(f) if f.is_finite() => py::float_repr(f),
        Num::Float(f) => return Err(format!("cannot round {}", py::float_repr(f))),
    };
    let (negative, text) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |t| (true, t));
    let (mantissa, exponent) = match text.split_once('e') {
        Some((m, e)) => (m, e.parse::<i64>().expect("repr writes an integer exponent")),
        None => (text, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    // The value is `digits` times 10 to the power `scale`.
    let mut digits: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes())
        .map(|b| b - b'0')
        .collect();
    let scale = exponent - i64::try_from(fraction.len()).expect("short");
    let target = -i64::try_from(places).expect("few places");
    if scale >= target {
        digits.extend(std::iter::repeat_n(
            0,
            usize::try_from(scale - target).expect("small"),
        ));
    } else {
        let drop = usize::try_from(target - scale).expect("small");
        let keep = digits.len().saturating_sub(drop);
        let round_up = drop <= digits.len() && digits[keep] >= 5;
        digits.truncate(keep);
        if round_up {
            match digits.iter().rposition(|d| *d != 9) {
                Some(i) => {
                    digits[i] += 1;
                    digits[i + 1..].fill(0);
                }
                None => {
                    digits.fill(0);
                    digits.insert(0, 1);
                }
            }
        }
    }
    let leading = digits.iter().take_while(|d| **d == 0).count();
    digits.drain(..leading.min(digits.len().saturating_sub(places + 1)));
    while digits.len() < places + 1 {
        digits.insert(0, 0);
    }
    let point = digits.len() - places;
    let mut out = String::from(if negative { "-" } else { "" });
    for (i, d) in digits.iter().enumerate() {
        if i == point {
            out.push('.');
        }
        out.push(char::from(b'0' + d));
    }
    Ok(out)
}

/// `a + b` with Python's `+`: ints stay ints, otherwise a plain float addition.
fn add(a: Num, b: Num) -> Num {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => Num::Int(x + y),
        (x, y) => Num::Float(x.as_f64() + y.as_f64()),
    }
}

/// `sum()` as CPython adds ints and floats: ints exactly until the first float, which is added
/// to their total, then floats by Neumaier's summation with any later int added plainly.
fn sum(values: impl IntoIterator<Item = Num>) -> Num {
    let mut values = values.into_iter();
    let mut whole = 0i64;
    let first = loop {
        match values.next() {
            None => return Num::Int(whole),
            Some(Num::Int(i)) => whole += i,
            Some(Num::Float(f)) => break f,
        }
    };
    let mut total = Num::Int(whole).as_f64() + first;
    let mut compensation = 0.0;
    for value in values {
        match value {
            Num::Float(x) => {
                let t = total + x;
                if total.abs() >= x.abs() {
                    compensation += (total - t) + x;
                } else {
                    compensation += (x - t) + total;
                }
                total = t;
            }
            Num::Int(_) => total += value.as_f64(),
        }
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    Num::Float(total)
}

/// `sum(xs) / len(xs)`.
fn mean(values: &[Num]) -> Result<f64, String> {
    let count = i64::try_from(values.len()).expect("few rows");
    divide(sum(values.iter().copied()), Num::Int(count))
}

/// evals/rubric.md's weights.
const WEIGHTS: [(&str, f64); 5] = [
    ("correctness", 0.35),
    ("autonomy", 0.25),
    ("actionability", 0.2),
    ("safety", 0.1),
    ("concision", 0.1),
];

/// A measure of one result row.
type Measure = fn(&Value) -> Result<Num, String>;

/// A row's weighted score under evals/rubric.md.
fn weighted(row: &Value) -> Result<Num, String> {
    let terms = WEIGHTS
        .iter()
        .map(|(d, w)| Ok(Num::Float(num_at(row, &["house", d])?.as_f64() * w)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(sum(terms))
}

fn output_tokens(row: &Value) -> Result<Num, String> {
    num_at(row, &["output_tokens"])
}

fn cost(row: &Value) -> Result<Num, String> {
    num_at(row, &["cost_usd"])
}

fn input_tokens(row: &Value) -> Result<Num, String> {
    let part = |name| num_at(row, &[name]);
    Ok(add(
        add(part("input_tokens")?, part("cache_write_tokens")?),
        part("cache_read_tokens")?,
    ))
}

fn cache_write(row: &Value) -> Result<Num, String> {
    num_at(row, &["cache_write_tokens"])
}

fn cache_read(row: &Value) -> Result<Num, String> {
    num_at(row, &["cache_read_tokens"])
}

fn characters(row: &Value) -> Result<Num, String> {
    num_at(row, &["characters"])
}

fn house_correctness(row: &Value) -> Result<Num, String> {
    num_at(row, &["house", "correctness"])
}

fn correctness_only(row: &Value) -> Result<Num, String> {
    num_at(row, &["correctness_only", "correctness"])
}

/// One run's rows by case, trial and condition, and its trials in order.
struct Run<'a> {
    index: HashMap<String, &'a Value>,
    trials: Vec<Value>,
}

impl Run<'_> {
    /// The sum of `value` over `cases` and the trials for `condition`, cases outermost.
    fn total(&self, condition: &str, cases: &[&Value], value: Measure) -> Result<Num, String> {
        let mut values = Vec::new();
        for case in cases {
            for trial in &self.trials {
                let key = py::key(&json!([case, trial, condition]));
                let row = self
                    .index
                    .get(&key)
                    .ok_or_else(|| format!("no row for {key}"))?;
                values.push(value(row)?);
            }
        }
        Ok(sum(values))
    }

    fn ratio(&self, pair: (&str, &str), value: Measure, cases: &[&Value]) -> Result<f64, String> {
        divide(
            self.total(pair.0, cases, value)?,
            self.total(pair.1, cases, value)?,
        )
    }

    fn difference(
        &self,
        pair: (&str, &str),
        value: Measure,
        cases: &[&Value],
    ) -> Result<f64, String> {
        let gap = match (
            self.total(pair.0, cases, value)?,
            self.total(pair.1, cases, value)?,
        ) {
            (Num::Int(a), Num::Int(b)) => Num::Int(a - b),
            (a, b) => Num::Float(a.as_f64() - b.as_f64()),
        };
        let answers = i64::try_from(self.trials.len() * cases.len()).expect("few rows");
        divide(gap, Num::Int(answers))
    }
}

type Statistic<'a> = &'a dyn Fn(&[&Value]) -> Result<f64, String>;

/// The statistic on `cases`, then the 251st and 9,751st of 10,000 sorted values on cases
/// drawn with replacement by a fresh `random.Random(2282)`.
fn interval(stat: Statistic, cases: &[&Value]) -> Result<[f64; 3], String> {
    let mut rng = Random::new(2282);
    let mut draws = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let sample: Vec<&Value> = cases.iter().map(|_| *rng.choice(cases)).collect();
        draws.push(stat(&sample)?);
    }
    // A stable sort, as Python's: -0.0 and 0.0 keep their order.
    draws.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok([stat(cases)?, draws[250], draws[9750]])
}

/// One interval line: the estimate and its interval to 2 places, signed for a difference.
fn show(out: &mut String, label: &str, values: [f64; 3], sign: bool) -> Result<(), String> {
    let mut texts = Vec::new();
    for x in values {
        let text = half_up(Num::Float(x), 2)?;
        texts.push(if sign && x >= 0.0 {
            format!("+{text}")
        } else {
            text
        });
    }
    let _ = writeln!(
        out,
        "  {label}: {} ({} to {})",
        texts[0], texts[1], texts[2]
    );
    Ok(())
}

/// The sum of `value` over `rows`: a count for bools.
fn total_of(rows: &[&Value], value: Measure) -> Result<Num, String> {
    Ok(sum(rows
        .iter()
        .map(|r| value(r))
        .collect::<Result<Vec<_>, String>>()?))
}

/// The mean of `value` over `rows`, rounded half up to `places`.
fn mean_of(rows: &[&Value], value: Measure, places: usize) -> Result<Value, String> {
    let values = rows
        .iter()
        .map(|r| value(r))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!(half_up(Num::Float(mean(&values)?), places)?))
}

fn lint_clean(row: &Value) -> Result<Num, String> {
    num_at(row, &["lint_clean"])
}

fn house_blocker(row: &Value) -> Result<Num, String> {
    num_at(row, &["house", "blocker"])
}

fn correctness_blocker(row: &Value) -> Result<Num, String> {
    num_at(row, &["correctness_only", "blocker"])
}

/// `sorted(set(values))`, as Python orders and compares them.
fn sorted_set(mut values: Vec<Value>) -> Result<Vec<Value>, String> {
    let mut error = None;
    values.sort_by(|a, b| {
        py::compare(a, b).unwrap_or_else(|e| {
            error.get_or_insert(e);
            std::cmp::Ordering::Equal
        })
    });
    if let Some(e) = error {
        return Err(e);
    }
    values.dedup_by(|a, b| py::key(a) == py::key(b));
    Ok(values)
}

/// One condition's line of figures for one run.
fn condition_line(rs: &[&Value], cold: bool) -> Result<Map<String, Value>, String> {
    let mut line = Map::new();
    line.insert("answers".into(), json!(rs.len()));
    let means: [(&str, Measure, usize); 6] = [
        ("mean output tokens", output_tokens, 0),
        ("mean input tokens, cache included", input_tokens, 0),
        ("mean cache write", cache_write, 0),
        ("mean cache read", cache_read, 0),
        ("mean cost US$", cost, 4),
        ("mean characters", characters, 0),
    ];
    for (label, value, places) in means {
        line.insert(label.into(), mean_of(rs, value, places)?);
    }
    let clean = total_of(rs, lint_clean)?.to_value();
    line.insert(
        "lint clean".into(),
        json!(format!("{clean} of {}", rs.len())),
    );
    let mut checked = 0;
    let mut passed = 0;
    for r in rs {
        let failed = py::field(r, "regex_failed")?;
        if !failed.is_null() {
            checked += 1;
            passed += usize::from(!py::truthy(failed));
        }
    }
    line.insert(
        "regex checks passed".into(),
        json!(format!("{passed} of {checked}")),
    );
    if cold {
        line.insert(
            "weighted score, evals/rubric.md".into(),
            mean_of(rs, weighted, 2)?,
        );
        line.insert(
            "correctness, evals/rubric.md".into(),
            mean_of(rs, house_correctness, 2)?,
        );
        line.insert(
            "blocking findings, evals/rubric.md".into(),
            total_of(rs, house_blocker)?.to_value(),
        );
        line.insert(
            "correctness, evals/rubric-correctness.md".into(),
            mean_of(rs, correctness_only, 2)?,
        );
        line.insert(
            "blocking findings, evals/rubric-correctness.md".into(),
            total_of(rs, correctness_blocker)?.to_value(),
        );
    }
    Ok(line)
}

/// Every figure the Concise and token-cost runs publish, recomputed from their rows: for the
/// cold calls, then the warm ones if any, each condition's means and counts, then each pair
/// of conditions' ratios and differences with their 95% intervals.
pub fn style_summary(rows: &[Value]) -> Result<String, String> {
    let mut conditions = vec![json!("candidate")];
    let all: Vec<Value> = rows
        .iter()
        .map(|r| py::field(r, "condition").cloned())
        .collect::<Result<_, _>>()?;
    conditions.extend(sorted_set(all)?.into_iter().filter(|c| *c != "candidate"));
    let mut setups = HashMap::new();
    for r in rows {
        setups.insert(
            py::key(py::field(r, "condition")?),
            py::to_str(py::field(r, "setup")?),
        );
    }
    let mut out = String::new();
    for run in ["cold", "warm"] {
        let in_run: Vec<&Value> = rows.iter().filter(|r| r["run"] == run).collect();
        if in_run.is_empty() {
            continue;
        }
        let _ = writeln!(out, "== {run} calls");
        for cond in &conditions {
            let rs: Vec<&Value> = in_run
                .iter()
                .copied()
                .filter(|r| &r["condition"] == cond)
                .collect();
            let line = condition_line(&rs, run == "cold")?;
            let _ = writeln!(
                out,
                "{} ({}): {}",
                py::to_str(cond),
                setups[&py::key(cond)],
                py::dumps(&Value::Object(line), None, true)
            );
        }
        // answer_key refuses a row without a case, trial or condition, so the columns are there.
        let index = in_run
            .iter()
            .map(|r| Ok((answer_key(r)?, *r)))
            .collect::<Result<HashMap<_, _>, String>>()?;
        let column = |name| sorted_set(in_run.iter().map(|r| r[name].clone()).collect());
        let trials = column("trial")?;
        let cases = column("case_id")?;
        let cases: Vec<&Value> = cases.iter().collect();
        let data = Run { index, trials };
        let ratios: [(&str, Measure); 2] = [
            ("output tokens, ratio", output_tokens),
            ("cost, ratio", cost),
        ];
        let differences: [(&str, Measure); 2] = [
            ("weighted score, evals/rubric.md, difference", weighted),
            (
                "correctness, evals/rubric-correctness.md, difference",
                correctness_only,
            ),
        ];
        for (i, first) in conditions.iter().enumerate() {
            for other in &conditions[i + 1..] {
                let (first, other) = (py::to_str(first), py::to_str(other));
                let _ = writeln!(out, " {first} against {other}:");
                let pair = (first.as_str(), other.as_str());
                for (label, value) in ratios {
                    let stat = |cases: &[&Value]| data.ratio(pair, value, cases);
                    show(&mut out, label, interval(&stat, &cases)?, false)?;
                }
                if run == "cold" {
                    for (label, value) in differences {
                        let stat = |cases: &[&Value]| data.difference(pair, value, cases);
                        show(&mut out, label, interval(&stat, &cases)?, true)?;
                    }
                }
            }
        }
    }
    Ok(out)
}

/// The rewriters asked through Claude Code, then patina, in the run's order.
const MODEL_REWRITERS: [&str; 5] = ["laconic", "humanizer", "avoid", "work", "plain"];

/// The sample id at the end of a rewrite case id, `aihb-2026-10-<id>`.
fn sample_id(row: &Value) -> Result<String, String> {
    let id = py::text(row, "case_id")?;
    id.rsplit_once('-')
        .map(|(_, sid)| sid.to_owned())
        .ok_or_else(|| format!("case id {} has no '-'", py::str_repr(id)))
}

/// JSON Lines text, one row a line, as `json.dumps(row) + "\n"` writes it.
fn jsonl(rows: &[Value]) -> String {
    rows.iter()
        .map(|r| py::dumps(r, None, true) + "\n")
        .collect()
}

/// Writes `pairs-<rewriter>.jsonl` ({id, source, rewrite}) for `factalign.py` from
/// `run-<rewriter>.jsonl` and, for patina, `patina-out/<id>.json`, whose `verification` goes to
/// `patina-verify.jsonl`; and `preserve/<rewriter>-<id>/original` and `edit` for `run.sh` to
/// check with `laconic-check preserve`.
pub fn rewrite_pairs(work: &Path) -> Result<(), String> {
    let mut sources = Map::new();
    for r in read_jsonl(&work.join("aihb-2026-10.jsonl"))? {
        sources.insert(
            py::text(&r, "sample_id")?.to_owned(),
            py::field(&r, "input")?.clone(),
        );
    }
    let source = |sid: &str| {
        sources
            .get(sid)
            .cloned()
            .ok_or_else(|| format!("no source text {}", py::str_repr(sid)))
    };
    let mut pairs: Vec<(&str, Vec<Value>)> = Vec::new();
    for s in MODEL_REWRITERS {
        let mut rows = Vec::new();
        for r in read_jsonl(&work.join(format!("run-{s}.jsonl")))? {
            let sid = sample_id(&r)?;
            rows.push(
                json!({"id": sid, "source": source(&sid)?, "rewrite": py::field(&r, "response")?}),
            );
        }
        pairs.push((s, rows));
    }
    let mut patina = Vec::new();
    let mut verify = Vec::new();
    for (sid, text) in &sources {
        let d = cases::parse(&read_text(&work.join(format!("patina-out/{sid}.json")))?)?;
        if !d.is_object() {
            return Err(format!("patina-out/{sid}.json is not a JSON object"));
        }
        let output = py::get(&d, "output").filter(|o| py::truthy(o));
        patina.push(
            json!({"id": sid, "source": text, "rewrite": output.cloned().unwrap_or(json!(""))}),
        );
        let mut row = Map::new();
        row.insert("id".into(), json!(sid));
        match py::get(&d, "verification").filter(|v| py::truthy(v)) {
            None => {}
            Some(Value::Object(fields)) => row.extend(fields.clone()),
            Some(other) => {
                return Err(format!(
                    "patina-out/{sid}.json: verification is {}",
                    py::repr(other)
                ));
            }
        }
        verify.push(Value::Object(row));
    }
    pairs.push(("patina", patina));
    write(&work.join("patina-verify.jsonl"), &jsonl(&verify))?;
    for (s, rows) in &pairs {
        write(&work.join(format!("pairs-{s}.jsonl")), &jsonl(rows))?;
        for row in rows {
            let id = py::text(row, "id")?;
            let folder = work.join(format!("preserve/{s}-{id}"));
            fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
            write(&folder.join("original"), py::text(row, "source")?)?;
            write(&folder.join("edit"), py::text(row, "rewrite")?)?;
        }
    }
    Ok(())
}

/// Rows of `path` by `id`, the later of 2 with one id winning in the earlier one's place.
fn by_id(
    rows: Vec<Value>,
    id: fn(&Value) -> Result<String, String>,
) -> Result<Map<String, Value>, String> {
    let mut map = Map::new();
    for row in rows {
        map.insert(id(&row)?, row);
    }
    Ok(map)
}

fn id_field(row: &Value) -> Result<String, String> {
    Ok(py::text(row, "id")?.to_owned())
}

/// One row per rewrite, without the texts: `factalign.py`'s units, `preserve`'s verdict,
/// lengths, cost and, for patina, its own `--verify` result.
pub fn rewrite_results(work: &Path) -> Result<String, String> {
    let rows = |name: String| rows_if_any(&work.join(name));
    let verify = by_id(rows("patina-verify.jsonl".into())?, id_field)?;
    let mut out = String::new();
    for s in MODEL_REWRITERS.iter().chain(&["patina"]) {
        let pairs = by_id(rows(format!("pairs-{s}.jsonl"))?, id_field)?;
        let runs = by_id(rows(format!("run-{s}.jsonl"))?, sample_id)?;
        for u in rows(format!("units-{s}.jsonl"))? {
            let sid = py::text(&u, "item_id")?;
            let units = match py::get(&u, "units") {
                None => &Vec::new(),
                Some(Value::Array(items)) => items,
                Some(other) => return Err(format!("units is {}", py::repr(other))),
            };
            let p = pairs
                .get(sid)
                .ok_or_else(|| format!("pairs-{s}.jsonl has no {}", py::str_repr(sid)))?;
            let mut altered = 0;
            let mut groups = Vec::new();
            for x in units {
                if !py::truthy(py::field(x, "kept")?) {
                    altered += 1;
                    groups.push(py::field(x, "group")?.clone());
                }
            }
            let words =
                |name| -> Result<usize, String> { Ok(py::words(py::text(p, name)?).count()) };
            let mut row = json!({
                "rewriter": s,
                "sample_id": sid,
                "units": units.len(),
                "units_altered": altered,
                "altered_groups": sorted_set(groups)?,
                "preserve_fails": flag(&work.join(format!("preserve/{s}-{sid}/fails")))?,
                "source_words": words("source")?,
                "rewrite_words": words("rewrite")?,
            });
            if let Some(run) = runs.get(sid) {
                let usage = py::get(run, "usage");
                row["output_tokens"] = usage_field(usage, "output_tokens");
                row["cost_usd"] = py::get(run, "cost_usd").cloned().unwrap_or(Value::Null);
            }
            if *s == "patina" {
                let v = verify
                    .get(sid)
                    .ok_or_else(|| format!("patina-verify.jsonl has no {}", py::str_repr(sid)))?;
                let mut fields = Map::new();
                for k in ["verified", "reason", "mps", "fidelity", "retried"] {
                    fields.insert(k.into(), py::get(v, k).cloned().unwrap_or(Value::Null));
                }
                row["patina_verify"] = Value::Object(fields);
            }
            out.push_str(&py::dumps(&row, None, true));
            out.push('\n');
        }
    }
    Ok(out)
}

/// A dict key as `json.dumps` writes one: text as it is, numbers and constants as JSON.
fn json_key(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Array(_) | Value::Object(_) => Err(format!("unhashable: {}", py::repr(value))),
        other => Ok(py::dumps(other, None, true)),
    }
}

/// `dict(Counter(values))`: each distinct value's count, in order of first sight.
fn counter(values: impl IntoIterator<Item = Value>) -> Result<Value, String> {
    let mut seen: Vec<(String, String, u64)> = Vec::new();
    for value in values {
        let key = py::key(&value);
        match seen.iter_mut().find(|(k, _, _)| *k == key) {
            Some(entry) => entry.2 += 1,
            None => seen.push((key, json_key(&value)?, 1)),
        }
    }
    Ok(Value::Object(
        seen.into_iter()
            .map(|(_, name, count)| (name, json!(count)))
            .collect(),
    ))
}

/// The rewrite run's table, recomputed from its rows: one line per rewriter.
pub fn rewrite_summary(rows: &[Value]) -> Result<String, String> {
    let mut rewriters: Vec<&Value> = Vec::new();
    for r in rows {
        let s = py::field(r, "rewriter")?;
        if !rewriters.contains(&s) {
            rewriters.push(s);
        }
    }
    let mut out = String::new();
    for s in rewriters {
        let rs: Vec<&Value> = rows.iter().filter(|r| &r["rewriter"] == s).collect();
        let (altered, kept): (Vec<&Value>, Vec<&Value>) = rs
            .iter()
            .copied()
            .partition(|r| py::truthy(&r["units_altered"]));
        let units = |r: &Value| num_at(r, &["units"]);
        let fails = |group: &[&Value]| -> Result<Value, String> {
            let failed = total_of(group, |r| num_at(r, &["preserve_fails"]))?.to_value();
            Ok(json!(format!("{failed} of {}", group.len())))
        };
        let mut ratios = Vec::new();
        for r in &rs {
            ratios.push(Num::Float(divide(
                num_at(r, &["rewrite_words"])?,
                num_at(r, &["source_words"])?,
            )?));
        }
        let mut groups = Vec::new();
        for r in &altered {
            groups.extend(items(py::field(r, "altered_groups")?)?.iter().cloned());
        }
        let mut line = Map::new();
        line.insert("texts".into(), json!(rs.len()));
        line.insert(
            "texts with a unit".into(),
            json!(rs.iter().filter(|r| py::truthy(&r["units"])).count()),
        );
        line.insert("units".into(), total_of(&rs, units)?.to_value());
        line.insert("documents with a unit altered".into(), json!(altered.len()));
        line.insert(
            "units altered".into(),
            total_of(&rs, |r| num_at(r, &["units_altered"]))?.to_value(),
        );
        line.insert("documents altered, by group".into(), counter(groups)?);
        line.insert("preserve fails, altered".into(), fails(&altered)?);
        line.insert("preserve fails, kept".into(), fails(&kept)?);
        line.insert(
            "mean length against source, in words".into(),
            json!(half_up(Num::Float(mean(&ratios)?), 2)?),
        );
        line.insert(
            "altered".into(),
            altered
                .iter()
                .map(|r| py::field(r, "sample_id").cloned())
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        );
        if rs.iter().any(|r| py::get(r, "cost_usd").is_some()) {
            let costs = rs
                .iter()
                .map(|r| match py::get(r, "cost_usd").filter(|c| py::truthy(c)) {
                    Some(c) => num(c),
                    None => Ok(Num::Int(0)),
                })
                .collect::<Result<Vec<_>, String>>()?;
            line.insert("cost US$".into(), json!(half_up(sum(costs), 2)?));
        }
        if s == "patina" {
            let v = rs
                .iter()
                .map(|r| py::field(r, "patina_verify"))
                .collect::<Result<Vec<_>, String>>()?;
            let column = |name| -> Result<Vec<Value>, String> {
                v.iter().map(|x| py::field(x, name).cloned()).collect()
            };
            let verified = column("verified")?;
            line.insert(
                "--verify passed".into(),
                json!(verified.iter().filter(|x| **x == json!(true)).count()),
            );
            let reasons = column("reason")?
                .into_iter()
                .zip(&verified)
                .filter(|(_, ok)| **ok != json!(true))
                .map(|(reason, _)| reason);
            line.insert("--verify reasons".into(), counter(reasons)?);
            line.insert("meaning score".into(), counter(column("mps")?)?);
            line.insert("fidelity".into(), counter(column("fidelity")?)?);
            line.insert(
                "retried".into(),
                json!(column("retried")?.iter().filter(|x| py::truthy(x)).count()),
            );
        }
        let _ = writeln!(
            out,
            "{}: {}",
            py::to_str(s),
            py::dumps(&Value::Object(line), None, true)
        );
    }
    Ok(out)
}

fn items(value: &Value) -> Result<&Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{} is not a list", py::repr(value)))
}

#[cfg(test)]
mod tests;
