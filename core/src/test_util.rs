// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Test-only helpers shared by several modules' tests: the exact IEEE remainder by 2π, written
//! for spec 007's `Angle::normalized` (data-model §1.2) and reused by spec 008's
//! `Cap::rect_bound` (FR-020).

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::f64::consts::PI;

/// A finite `f64` as `m · 2^e`, `m` an integer of at most 53 bits.
fn decompose(x: f64) -> (BigInt, i64) {
    let bits = x.to_bits();
    let frac = i64::try_from(bits & ((1 << 52) - 1)).unwrap();
    let exp = i64::try_from((bits >> 52) & 0x7ff).unwrap();
    let (m, e) = if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1 << 52), exp - 1075)
    };
    let m = BigInt::from(m);
    (if x.is_sign_negative() { -m } else { m }, e)
}

/// `f · 2^e`, in steps that stay exact when the result is representable.
fn scale(mut f: f64, mut e: i64) -> f64 {
    while e > 0 {
        let k = e.min(500);
        f *= f64::from_bits(u64::try_from(1023 + k).unwrap() << 52);
        e -= k;
    }
    while e < 0 {
        let k = (-e).min(500);
        f *= f64::from_bits(u64::try_from(1023 - k).unwrap() << 52);
        e += k;
    }
    f
}

/// `num / den` rounded to the nearest integer, ties to even; `den > 0`.
fn div_round_half_even(num: &BigInt, den: &BigInt) -> BigInt {
    let (q, r) = (num.abs() / den, num.abs() % den);
    let twice = &r * 2;
    let q = if twice > *den || (twice == *den && (&q % 2) == BigInt::from(1)) {
        q + 1
    } else {
        q
    };
    if num.is_negative() { -q } else { q }
}

/// The IEEE-754 remainder of `x` by `y`, the `f64` nearest 2π: `x − n·y` with `n` the integer
/// nearest `x / y`, ties to even, computed exactly, as C's `remainder(x, 2 * M_PI)`. A zero
/// result has the sign of `x`, as IEEE's; ±∞ and NaN give NaN.
pub(crate) fn ieee_remainder_2pi(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    let y = 2.0 * PI;
    let (mx, ex) = decompose(x);
    let (my, ey) = decompose(y);
    let (num, den) = if ex >= ey {
        (mx.clone() << usize::try_from(ex - ey).unwrap(), my.clone())
    } else {
        (mx.clone(), my.clone() << usize::try_from(ey - ex).unwrap())
    };
    let n = div_round_half_even(&num, &den);
    let e = ex.min(ey);
    let r = (mx << usize::try_from(ex - e).unwrap()) - n * (my << usize::try_from(ey - e).unwrap());
    if r.is_zero() {
        return if x.is_sign_negative() { -0.0 } else { 0.0 };
    }
    scale(r.to_f64().unwrap(), e)
}
