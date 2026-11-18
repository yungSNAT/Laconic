// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The Concise and token-cost runs' summary: each condition's figures and each pair's intervals.

use super::*;
use crate::py::{Num, Random};
use std::fmt::Write as _;

/// evals/rubric.md's weights.
const WEIGHTS: [(&str, f64); 5] = [
    ("correctness", 0.35),
    ("autonomy", 0.25),
    ("actionability", 0.2),
    ("safety", 0.1),
    ("concision", 0.1),
];

/// A row's weighted score under evals/rubric.md.
pub(super) fn weighted(row: &Value) -> Result<Num, String> {
    let terms = WEIGHTS
        .iter()
        .map(|(d, w)| Ok(Num::Float(num_at(row, &["house", d])?.as_f64() * w)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(sum(terms))
}

fn output_tokens(row: &Value) -> Result<Num, String> {
    num_at(row, &["output_tokens"])
}

pub(super) fn cost(row: &Value) -> Result<Num, String> {
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

pub(super) fn lint_clean(row: &Value) -> Result<Num, String> {
    num_at(row, &["lint_clean"])
}

fn house_blocker(row: &Value) -> Result<Num, String> {
    num_at(row, &["house", "blocker"])
}

fn correctness_blocker(row: &Value) -> Result<Num, String> {
    num_at(row, &["correctness_only", "blocker"])
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
        // Python reads every row's run, so a row without one raises rather than drops out.
        py::field(r, "run")?;
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
