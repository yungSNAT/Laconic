// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Python's arithmetic, rounding and set ordering for the record steps' figures.

use super::*;
use crate::py::Num;

/// A JSON number or bool as Python adds it: bools are the ints 0 and 1.
pub(super) fn num(value: &Value) -> Result<Num, String> {
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

pub(super) fn num_at(row: &Value, path: &[&str]) -> Result<Num, String> {
    let mut value = row;
    for key in path {
        value = py::field(value, key)?;
    }
    num(value)
}

/// `a / b` as Python's true division.
pub(super) fn divide(a: Num, b: Num) -> Result<f64, String> {
    if b.as_f64() == 0.0 {
        return Err("division by zero".into());
    }
    Ok(a.as_f64() / b.as_f64())
}

/// `Decimal(repr(x)).quantize(Decimal(1).scaleb(-places), ROUND_HALF_UP)`, as text. Up to 6
/// places, where Python writes no exponent.
pub(super) fn half_up(x: Num, places: usize) -> Result<String, String> {
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
        Some((m, e)) => (m, e.parse::<i64>().expect("an integer exponent")),
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
pub(super) fn add(a: Num, b: Num) -> Num {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => Num::Int(x + y),
        (x, y) => Num::Float(x.as_f64() + y.as_f64()),
    }
}

/// `sum()` as CPython adds ints and floats: ints exactly until the first float, which is added
/// to their total, then floats by Neumaier's summation with any later int added plainly.
pub(super) fn sum(values: impl IntoIterator<Item = Num>) -> Num {
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
pub(super) fn mean(values: &[Num]) -> Result<f64, String> {
    let count = i64::try_from(values.len()).expect("few rows");
    divide(sum(values.iter().copied()), Num::Int(count))
}

/// A measure of one result row.
pub(super) type Measure = fn(&Value) -> Result<Num, String>;

/// The sum of `value` over `rows`: a count for bools.
pub(super) fn total_of(rows: &[&Value], value: Measure) -> Result<Num, String> {
    Ok(sum(rows
        .iter()
        .map(|r| value(r))
        .collect::<Result<Vec<_>, String>>()?))
}

/// The mean of `value` over `rows`, rounded half up to `places`.
pub(super) fn mean_of(rows: &[&Value], value: Measure, places: usize) -> Result<Value, String> {
    let values = rows
        .iter()
        .map(|r| value(r))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(json!(half_up(Num::Float(mean(&values)?), places)?))
}

/// `sorted(set(values))`, as Python orders and compares them.
pub(super) fn sorted_set(mut values: Vec<Value>) -> Result<Vec<Value>, String> {
    if let Some(v) = values.iter().find(|v| v.is_array() || v.is_object()) {
        return Err(format!("unhashable: {}", py::repr(v)));
    }
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
