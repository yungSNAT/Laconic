// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: Apache-2.0

use super::float;
use num_bigint::{BigInt, BigUint, Sign};
use serde_json::Value;

/// A number from Python arithmetic: an int stays an int, a float a float.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    pub fn as_f64(self) -> f64 {
        match self {
            // Past 2**53 this rounds to nearest, ties to even, as CPython's float(int) does.
            #[expect(clippy::cast_precision_loss, reason = "rounds as Python's float(int)")]
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
    sum_of(&values.into_iter().collect::<Vec<_>>())
}

fn sum_of(values: &[f64]) -> f64 {
    let Some((&first, rest)) = values.split_first() else {
        return 0.0;
    };
    let mut total = 0.0 + first;
    let mut compensation = 0.0;
    for &x in rest {
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
    mean_of(&values.into_iter().collect::<Vec<_>>())
}

fn mean_of(values: &[Num]) -> Option<Num> {
    let mut total = BigInt::from(0);
    let mut count = 0u64;
    let mut any_float = false;
    for &value in values {
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
            let mean = i64::try_from(&quotient).expect("a mean lies between its values");
            return Some(Num::Int(mean));
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
