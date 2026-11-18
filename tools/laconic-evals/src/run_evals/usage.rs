// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::*;

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
pub(super) fn total(values: &[Option<i128>]) -> Option<i128> {
    values.iter().copied().sum()
}

pub(super) fn int_value(n: i128) -> Result<Value, String> {
    i64::try_from(n)
        .map(Value::from)
        .or_else(|_| u64::try_from(n).map(Value::from))
        .map_err(|_| format!("a token count of {n} is too large"))
}

/// A float the summary prints; JSON has no NaN or infinity.
pub(super) fn finite(x: f64) -> Result<Value, String> {
    if x.is_finite() {
        Ok(py::float(x))
    } else {
        Err("Out of range float values are not JSON compliant".into())
    }
}

/// A count or amount from the usage summary: an int, a float or None.
#[derive(Clone, Copy)]
pub(super) enum Amount {
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

pub(super) fn difference(value: Amount, baseline: Amount) -> Amount {
    match (value, baseline) {
        (Amount::Int(a), Amount::Int(b)) => Amount::Int(a - b),
        (Amount::None, _) | (_, Amount::None) => Amount::None,
        (a, b) => Amount::Float(a.as_f64().expect("a value") - b.as_f64().expect("a value")),
    }
}

pub(super) fn percent_change(delta: Amount, baseline: Amount) -> Result<Value, String> {
    match (delta.as_f64(), baseline.as_f64()) {
        (Some(d), Some(b)) if b != 0.0 => finite(d / b * 100.0),
        _ => Ok(Value::Null),
    }
}

pub(super) fn validate_usage_row(row: &Value, index: usize) -> Result<(), String> {
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
