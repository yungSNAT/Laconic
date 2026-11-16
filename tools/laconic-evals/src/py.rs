// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! The parts of Python's behaviour the evaluation outputs depend on, so the port writes the
//! same bytes as the Python scripts did: `json.dumps`, `repr`, `str.split` and `splitlines`,
//! `sum` (Neumaier summation since Python 3.12), `statistics.mean` (exact), `round` and the
//! Mersenne Twister behind `random.Random(seed)`.

use num_bigint::{BigInt, BigUint, Sign};
use serde_json::{Map, Number, Value};
use std::cmp::Ordering;
use std::fmt::Write as _;

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
            c if c < ' ' || (ascii && !(' '..='~').contains(&c)) => {
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
                    x.partial_cmp(&y)
                        .ok_or_else(|| "cannot order NaN".to_owned())
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

/// A number from Python arithmetic: an int stays an int, a float a float.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    pub fn as_f64(self) -> f64 {
        match self {
            // Every count here is far below 2**53.
            #[expect(clippy::cast_precision_loss, reason = "counts are below 2**53")]
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }

    pub fn to_value(self) -> Value {
        match self {
            Num::Int(i) => Value::from(i),
            Num::Float(f) => float(f),
        }
    }
}

/// `sum()` of floats as CPython 3.12 and later computes it: the first term is added to the
/// int 0, then Neumaier's compensated summation, adding the compensation at the end.
pub fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut values = values.into_iter();
    let Some(first) = values.next() else {
        return 0.0;
    };
    let mut total = 0.0 + first;
    let mut compensation = 0.0;
    for x in values {
        let t = total + x;
        if total.abs() >= x.abs() {
            compensation += (total - t) + x;
        } else {
            compensation += (x - t) + total;
        }
        total = t;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

/// A finite float as an exact fraction over 2**1074.
fn scaled(x: f64) -> BigInt {
    let bits = x.to_bits();
    let exponent = i64::try_from((bits >> 52) & 0x7ff).expect("11 bits");
    let fraction = bits & ((1 << 52) - 1);
    let (mantissa, shift) = if exponent == 0 {
        (fraction, 0)
    } else {
        (fraction | (1 << 52), exponent - 1)
    };
    let magnitude = BigInt::from(mantissa) << u32::try_from(shift).expect("a small shift");
    if x.is_sign_negative() {
        -magnitude
    } else {
        magnitude
    }
}

/// `p / q` correctly rounded to the nearest float, ties to even, as `float(Fraction(p, q))`.
fn divide(p: &BigInt, q: &BigUint) -> f64 {
    let negative = p.sign() == Sign::Minus;
    let p = p.magnitude();
    if p.bits() == 0 {
        return 0.0;
    }
    let width = i64::try_from(p.bits()).expect("bits") - i64::try_from(q.bits()).expect("bits");
    // The quotient gets 55 or 56 bits: 53 to keep, a rounding bit and one more.
    let shift = 55 - width;
    let (numerator, denominator) = if shift >= 0 {
        (p << u64::try_from(shift).expect("non-negative"), q.clone())
    } else {
        (p.clone(), q << u64::try_from(-shift).expect("positive"))
    };
    let quotient = &numerator / &denominator;
    let sticky = &quotient * &denominator != numerator;
    let mut drop = i64::try_from(quotient.bits()).expect("bits") - 53;
    // Below the smallest normal float, the last kept bit is 2**-1074.
    let lowest = -shift + drop;
    if lowest < -1074 {
        drop += -1074 - lowest;
    }
    let drop_bits = u64::try_from(drop).expect("at least 2 bits dropped");
    let low = &quotient & ((BigUint::from(1u8) << drop_bits) - 1u8);
    let half = BigUint::from(1u8) << (drop_bits - 1);
    let mut kept = quotient >> drop_bits;
    if low > half || (low == half && (sticky || kept.bit(0))) {
        kept += 1u8;
    }
    let kept = u64::try_from(kept).unwrap_or(u64::MAX);
    #[expect(
        clippy::cast_precision_loss,
        reason = "kept holds at most 54 bits, exactly"
    )]
    let mut value = kept as f64;
    let mut exponent = -shift + drop;
    while exponent > 1000 {
        value *= 2f64.powi(1000);
        exponent -= 1000;
    }
    while exponent < -1000 {
        value *= 2f64.powi(-1000);
        exponent += 1000;
    }
    value *= 2f64.powi(i32::try_from(exponent).expect("a small exponent"));
    if negative { -value } else { value }
}

/// `statistics.mean`: the exact mean, as an int when every value is an int and the mean is
/// whole, otherwise the float nearest the exact mean. None for no values.
pub fn mean(values: impl IntoIterator<Item = Num>) -> Option<Num> {
    let mut total = BigInt::from(0);
    let mut count = 0u64;
    let mut any_float = false;
    for value in values {
        count += 1;
        total += match value {
            Num::Int(i) => BigInt::from(i) << 1074u32,
            Num::Float(f) => {
                any_float = true;
                scaled(f)
            }
        };
    }
    if count == 0 {
        return None;
    }
    let denominator = BigUint::from(count) << 1074u32;
    if !any_float {
        let whole = &total >> 1074u32;
        let count = BigInt::from(count);
        if &whole % &count == BigInt::from(0) {
            let quotient = whole / count;
            if let Ok(i) = i64::try_from(&quotient) {
                return Some(Num::Int(i));
            }
        }
    }
    Some(Num::Float(divide(&total, &denominator)))
}

/// The mean of floats, as `statistics.mean` gives it.
pub fn mean_f64(values: impl IntoIterator<Item = f64>) -> Option<f64> {
    mean(values.into_iter().map(Num::Float)).map(Num::as_f64)
}

/// `round(x, digits)` for a float: the nearest decimal with that many places, ties to even on
/// the float's exact value, read back as a float.
pub fn round(x: f64, digits: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    format!("{x:.digits$}")
        .parse()
        .expect("a formatted float parses")
}

/// `round(value, digits)`: an int is returned unchanged.
pub fn round_num(value: Num, digits: usize) -> Num {
    match value {
        Num::Int(_) => value,
        Num::Float(f) => Num::Float(round(f, digits)),
    }
}

/// Python's `random.Random(seed)` for a non-negative seed below 2**32: MT19937 seeded by
/// `init_by_array([seed])`, with `getrandbits`, `choice` and `shuffle` as CPython draws them.
pub struct Random {
    state: [u32; 624],
    index: usize,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = 19_650_218;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(u32::try_from(i).expect("below 624"));
        }
        let key = [seed];
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..624 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(u32::try_from(j).expect("below 1"));
            i += 1;
            j += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..623 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(u32::try_from(i).expect("below 624"));
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        Random { state, index: 624 }
    }

    fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            for k in 0..624 {
                let y = (self.state[k] & 0x8000_0000) | (self.state[(k + 1) % 624] & 0x7fff_ffff);
                let mut next = self.state[(k + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    next ^= 0x9908_b0df;
                }
                self.state[k] = next;
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `_randbelow(n)` for 0 < n < 2**32: rejection sampling on `getrandbits(n.bit_length())`.
    fn below(&mut self, n: usize) -> usize {
        let n = u32::try_from(n).expect("fewer than 2**32 items");
        let bits = 32 - n.leading_zeros();
        loop {
            let r = self.next_u32() >> (32 - bits);
            if r < n {
                return r as usize;
            }
        }
    }

    /// `random.choice(items)`.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// `random.shuffle(items)`.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn float_repr_matches_python() {
        for (x, text) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (0.1, "0.1"),
            (1e-4, "0.0001"),
            (1e-5, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1.2345e20, "1.2345e+20"),
            (123.456, "123.456"),
            (-2.5, "-2.5"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (f64::NAN, "nan"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(float_repr(x), text, "{x}");
        }
    }

    #[test]
    fn dumps_matches_python_spacing_and_escapes() {
        let value =
            json!({"a": [1, 2.0, "é\u{1f600}\n\u{7f}"], "b": {}, "c": [], "d": null, "e": true});
        assert_eq!(
            dumps_row(&value),
            "{\"a\": [1, 2.0, \"é\u{1f600}\\n\u{7f}\"], \"b\": {}, \"c\": [], \"d\": null, \"e\": true}"
        );
        assert_eq!(
            dumps_pretty(&value),
            "{\n  \"a\": [\n    1,\n    2.0,\n    \"\\u00e9\\ud83d\\ude00\\n\\u007f\"\n  ],\n  \"b\": {},\n  \"c\": [],\n  \"d\": null,\n  \"e\": true\n}"
        );
        assert_eq!(
            dumps(&json!("\u{8}\u{c}\t\r\"\\\u{1}"), None, true),
            "\"\\b\\f\\t\\r\\\"\\\\\\u0001\""
        );
    }

    #[test]
    fn repr_matches_python() {
        assert_eq!(
            repr(&json!([null, true, 1, 1.5, "it's", "a\"b'c", {"k": "v\n"}])),
            "[None, True, 1, 1.5, \"it's\", 'a\"b\\'c', {'k': 'v\\n'}]"
        );
        assert_eq!(str_repr("\u{1}\u{85}x\\\t\r"), "'\\x01\\x85x\\\\\\t\\r'");
        assert_eq!(to_str(&json!("plain")), "plain");
        assert_eq!(to_str(&json!(2)), "2");
    }

    #[test]
    fn truthiness_matches_python() {
        for (value, expected) in [
            (json!(null), false),
            (json!(false), false),
            (json!(0), false),
            (json!(0.0), false),
            (json!(""), false),
            (json!([]), false),
            (json!({}), false),
            (json!(true), true),
            (json!(2), true),
            (json!("x"), true),
            (json!([0]), true),
            (json!({"k": 0}), true),
        ] {
            assert_eq!(truthy(&value), expected, "{value}");
        }
        assert_eq!(get(&json!([1]), "k"), None);
    }

    #[test]
    fn compare_orders_as_python_sorted_does() {
        assert_eq!(compare(&json!("a"), &json!("b")), Ok(Ordering::Less));
        assert_eq!(compare(&json!(2), &json!(10)), Ok(Ordering::Less));
        assert_eq!(compare(&json!(1.5), &json!(1)), Ok(Ordering::Greater));
        assert_eq!(compare(&json!(true), &json!(1)), Ok(Ordering::Equal));
        assert_eq!(
            compare(&json!(["a", 2]), &json!(["a", 10])),
            Ok(Ordering::Less)
        );
        assert_eq!(compare(&json!(["a"]), &json!(["a", 1])), Ok(Ordering::Less));
        assert!(
            compare(&json!("a"), &json!(1))
                .unwrap_err()
                .contains("not supported")
        );
    }

    #[test]
    fn keys_equal_where_python_values_are_equal() {
        assert_eq!(key(&json!(1)), key(&json!(1.0)));
        assert_ne!(key(&json!(1)), key(&json!(1.5)));
        assert_eq!(key(&json!(1.5)), "1.5");
        assert_eq!(key(&json!(["a", 2.0])), key(&json!(["a", 2])));
        assert_eq!(key(&json!(1e300)), "1e+300");
        assert_eq!(key(&json!(true)), key(&json!(1)));
        assert_eq!(key(&json!(-0.0)), key(&json!(0)));
    }

    #[test]
    fn text_helpers_match_python() {
        assert_eq!(
            words(" a\u{1c}b\u{a0}c ").collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(strip("\u{2028} x \u{1f}"), "x");
        assert_eq!(
            split_lines("a\r\nb\rc\u{2028}d\u{85}e\n"),
            ["a", "b", "c", "d", "e"]
        );
        assert_eq!(universal_newlines("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn sum_uses_neumaier_compensation() {
        // Naive left-to-right addition gives 0.0 here; CPython 3.12 and later give 2.0.
        assert_eq!(sum([1.0, 1e100, 1.0, -1e100]), 2.0);
        assert_eq!(sum([0.1; 10]), 1.0);
        assert_eq!(sum([]), 0.0);
        assert!(sum([-0.0]).is_sign_positive(), "0 + -0.0 is 0.0");
        assert_eq!(sum([1e308, 1e308]), f64::INFINITY);
    }

    #[test]
    fn mean_is_exact_and_keeps_ints() {
        assert_eq!(mean([Num::Int(1), Num::Int(3)]), Some(Num::Int(2)));
        assert_eq!(mean([Num::Int(1), Num::Int(2)]), Some(Num::Float(1.5)));
        assert_eq!(mean([Num::Int(2), Num::Float(2.0)]), Some(Num::Float(2.0)));
        assert_eq!(mean([]), None);
        // The exact mean of 3 tenths is 0.1 as written, which naive summation misses.
        assert_eq!(mean_f64([0.1, 0.1, 0.1]), Some(0.1));
        assert_eq!(mean_f64([1e308, 1e308]), Some(1e308));
        assert_eq!(mean_f64([-0.5, -0.25]), Some(-0.375));
        // Half the smallest float is a tie, which rounds to the even 0.0, as Python gives.
        assert_eq!(mean_f64([5e-324, 0.0]), Some(0.0));
        assert_eq!(mean_f64([5e-324, 5e-324, 0.0]), Some(5e-324));
        assert_eq!(mean_f64([-0.0]), Some(0.0));
        assert_eq!(
            mean([Num::Int(i64::MAX), Num::Int(i64::MAX)]),
            Some(Num::Int(i64::MAX))
        );
        assert_eq!(Num::Float(2.5).as_f64(), 2.5);
        assert_eq!(Num::Int(2).to_value(), json!(2));
        assert_eq!(float(f64::NAN), Value::Null);
    }

    #[test]
    fn round_is_half_even_on_the_exact_value() {
        assert_eq!(round(0.125, 2), 0.12);
        assert_eq!(round(0.375, 2), 0.38);
        assert_eq!(round(2.675, 2), 2.67, "2.675 is stored below the half");
        assert_eq!(round(1.0 / 3.0, 3), 0.333);
        assert!(round(f64::NAN, 3).is_nan());
        assert_eq!(round_num(Num::Int(7), 3), Num::Int(7));
        assert_eq!(round_num(Num::Float(0.5555), 3), Num::Float(0.555));
    }

    #[test]
    fn random_matches_python_seeded_with_zero() {
        // Python: r = random.Random(0); [r.getrandbits(32) for _ in range(3)]
        let mut random = Random::new(0);
        let draws: Vec<u32> = (0..3).map(|_| random.next_u32()).collect();
        assert_eq!(draws, [3_626_764_237, 1_654_615_998, 3_255_389_356]);
        // Python: x = list(range(10)); random.Random(0).shuffle(x)
        let mut items: Vec<u32> = (0..10).collect();
        Random::new(0).shuffle(&mut items);
        assert_eq!(items, [7, 8, 1, 5, 3, 4, 2, 0, 9, 6]);
        // Python: r = random.Random(0); [r.choice('abcde') for _ in range(5)]
        let mut random = Random::new(0);
        let letters = ['a', 'b', 'c', 'd', 'e'];
        let picks: String = (0..5).map(|_| *random.choice(&letters)).collect();
        assert_eq!(picks, "ddace");
    }

    #[test]
    fn object_keeps_insertion_order() {
        let value = object([("b", json!(1)), ("a", json!(2))]);
        assert_eq!(dumps_row(&value), "{\"b\": 1, \"a\": 2}");
    }
}
