// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

//! The parts of Python's behaviour the evaluation outputs depend on, so the port writes the
//! same bytes as the Python scripts did: `json.dumps`, `repr`, `str.split` and `splitlines`,
//! `sum` (Neumaier summation since Python 3.12), `statistics.mean` (exact), `round` and the
//! Mersenne Twister behind `random.Random(seed)`.

use serde_json::{Map, Number, Value};
use std::cmp::Ordering;
use std::fmt::Write as _;

mod num;
mod random;

pub use num::{Num, mean, mean_f64, round, round_num, sum};
pub use random::Random;

/// Python's `str.isspace`, which `str.split()` and `str.strip()` use.
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `str.split()`: the words between runs of whitespace.
pub fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(is_space).filter(|w| !w.is_empty())
}

/// `str.strip()`.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// `str.splitlines()`: Python also ends a line at `\v`, `\f`, `\x1c` to `\x1e`, `\x85`,
/// ` ` and ` `.
pub fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        let end = match c {
            '\r' if chars.peek().is_some_and(|&(_, n)| n == '\n') => {
                chars.next();
                i + 2
            }
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}'
            | '\u{2028}' | '\u{2029}' => i + c.len_utf8(),
            _ => continue,
        };
        lines.push(&text[start..i]);
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Text as Python's text mode reads it: CRLF and lone CR become LF.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `repr(float)`: the shortest digits that round-trip, in exponent form below 1e-4 or from
/// 1e16, always with a decimal point or exponent.
pub fn float_repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.into();
    }
    // Rust's `{:e}` gives the same shortest round-trip digits, as `d.ddde[-]x`.
    let sci = format!("{x:e}");
    let (mantissa, exponent) = sci.split_once('e').expect("{:e} has an exponent");
    let exponent: i32 = exponent.parse().expect("an integer exponent");
    let (sign, mantissa) = mantissa
        .strip_prefix('-')
        .map_or(("", mantissa), |m| ("-", m));
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let point = exponent + 1;
    let count = i32::try_from(digits.len()).expect("few digits");
    let body = if point <= -4 || point > 16 {
        let (first, rest) = digits.split_at(1);
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let mark = if exponent < 0 { '-' } else { '+' };
        format!("{first}{fraction}e{mark}{:02}", exponent.abs())
    } else if point <= 0 {
        format!("0.{}{digits}", "0".repeat(point.unsigned_abs() as usize))
    } else if point >= count {
        format!("{digits}{}.0", "0".repeat((point - count) as usize))
    } else {
        let (whole, fraction) = digits.split_at(point as usize);
        format!("{whole}.{fraction}")
    };
    format!("{sign}{body}")
}

/// A JSON number as Python prints it: an int as digits, a float by `float_repr`.
fn number_text(n: &Number) -> String {
    if let Some(f) = n.as_f64().filter(|_| n.is_f64()) {
        float_repr(f)
    } else {
        n.to_string()
    }
}

fn escape_into(out: &mut String, text: &str, ascii: bool) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            // Python's json writes U+0085, U+2028 and U+2029 raw; escaping them keeps a row on
            // one line for `split_lines`, which ends lines there.
            c if c < ' '
                || matches!(c, '\u{85}' | '\u{2028}' | '\u{2029}')
                || (ascii && !(' '..='~').contains(&c)) =>
            {
                let mut units = [0; 2];
                for unit in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The type name Python's error messages give a JSON value.
pub fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "NoneType",
        Value::Bool(_) => "bool",
        Value::Number(n) if n.is_f64() => "float",
        Value::Number(_) => "int",
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

/// `json.dumps(value, ensure_ascii=ascii, indent=indent)`.
pub fn dumps(value: &Value, indent: Option<usize>, ascii: bool) -> String {
    let mut out = String::new();
    write_value(&mut out, value, indent, ascii, 0);
    out
}

/// `json.dumps(value, indent=2)`, as the commands print their summaries.
pub fn dumps_pretty(value: &Value) -> String {
    dumps(value, Some(2), true)
}

/// `print(line)` to `out`.
pub fn print(out: &mut dyn std::io::Write, line: &str) -> Result<(), String> {
    writeln!(out, "{line}").map_err(|e| e.to_string())
}

/// `json.dumps(value, ensure_ascii=False)`, as the commands write JSON Lines rows.
pub fn dumps_row(value: &Value) -> String {
    dumps(value, None, false)
}

fn write_value(out: &mut String, value: &Value, indent: Option<usize>, ascii: bool, depth: usize) {
    let newline = |out: &mut String, depth: usize| {
        if let Some(width) = indent {
            out.push('\n');
            out.push_str(&" ".repeat(width * depth));
        }
    };
    let separator = if indent.is_some() { "," } else { ", " };
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number_text(n)),
        Value::String(s) => escape_into(out, s, ascii),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(separator);
                }
                newline(out, depth + 1);
                write_value(out, item, indent, ascii, depth + 1);
            }
            newline(out, depth);
            out.push(']');
        }
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Object(map) => {
            out.push('{');
            for (i, (key, item)) in map.iter().enumerate() {
                if i > 0 {
                    out.push_str(separator);
                }
                newline(out, depth + 1);
                escape_into(out, key, ascii);
                out.push_str(": ");
                write_value(out, item, indent, ascii, depth + 1);
            }
            newline(out, depth);
            out.push('}');
        }
    }
}

/// `repr(str)`: single quotes unless the text holds one and no double quote.
pub fn str_repr(text: &str) -> String {
    let quote = if text.contains('\'') && !text.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::from(quote);
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            // shortcut: Python escapes every non-printable character (str.isprintable); this
            // escapes the C0 and C1 controls and DEL, which covers what reaches error messages.
            c if c < ' ' || ('\u{7f}'..='\u{a0}').contains(&c) => {
                let _ = write!(out, "\\x{:02x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `repr()` of a value parsed from JSON.
pub fn repr(value: &Value) -> String {
    match value {
        Value::Null => "None".into(),
        Value::Bool(b) => if *b { "True" } else { "False" }.into(),
        Value::Number(n) => number_text(n),
        Value::String(s) => str_repr(s),
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(repr).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", str_repr(k), repr(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
    }
}

/// `str()` of a value parsed from JSON: a string as itself, anything else as `repr`.
pub fn to_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => repr(other),
    }
}

/// Python truthiness of a JSON value.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `value.get(key)` on a dict, with a non-dict as an error, as Python's AttributeError.
pub fn get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object().and_then(|map| map.get(key))
}

/// A dict's field, with a missing one as an error, as Python's KeyError.
pub fn field<'a>(row: &'a Value, name: &str) -> Result<&'a Value, String> {
    get(row, name).ok_or_else(|| format!("a row has no {}", str_repr(name)))
}

/// A dict's text field; a missing or non-text one is an error.
pub fn text<'a>(row: &'a Value, name: &str) -> Result<&'a str, String> {
    field(row, name)?
        .as_str()
        .ok_or_else(|| format!("{} is not text in {}", str_repr(name), repr(row)))
}

/// Python's ordering of 2 values from JSON, as `sorted` uses: numbers by value (a bool is
/// 0 or 1), strings by code point, lists element by element. Anything else cannot be ordered.
pub fn compare(a: &Value, b: &Value) -> Result<Ordering, String> {
    let number = |v: &Value| match v {
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        Value::Number(n) => n.as_f64(),
        _ => None,
    };
    match (a, b) {
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        (Value::Array(x), Value::Array(y)) => {
            for (p, q) in x.iter().zip(y) {
                if p != q {
                    return compare(p, q);
                }
            }
            Ok(x.len().cmp(&y.len()))
        }
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => {
                if let (Some(p), Some(q)) = (a.as_i64(), b.as_i64()) {
                    Ok(p.cmp(&q))
                } else {
                    Ok(x.partial_cmp(&y).expect("a JSON number is never NaN"))
                }
            }
            _ => Err(format!(
                "'<' not supported between {} and {}",
                repr(a),
                repr(b)
            )),
        },
    }
}

/// A hashable key for a JSON value, equal where Python's `==` is: an integral float and the
/// same int give one key.
pub fn key(value: &Value) -> String {
    match value {
        Value::Bool(b) => u8::from(*b).to_string(),
        Value::Number(n) if n.is_f64() => {
            let f = n.as_f64().expect("an f64");
            if f.fract() == 0.0 && f.abs() < 9.0e15 {
                format!("{:.0}", f + 0.0)
            } else {
                float_repr(f)
            }
        }
        Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(key).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}

/// A JSON float.
pub fn float(x: f64) -> Value {
    Number::from_f64(x).map_or(Value::Null, Value::Number)
}

/// An object from key and value pairs, in order.
pub fn object(pairs: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect::<Map<String, Value>>(),
    )
}

#[cfg(test)]
mod tests;
