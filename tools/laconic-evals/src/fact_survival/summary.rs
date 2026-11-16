// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use super::*;

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
