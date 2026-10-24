// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of the angle (spec 007 data-model §1.2): radians, and upstream's
//! conversions to degrees and to the integer E5, E6 and E7 forms.
//!
//! Written from upstream's `s1angle.{h,cc}` and `util/math/mathutil.h` (`v0.14.0-40`). An angle
//! is any `f64` of radians; the type has no validity rule. The conversions are upstream's
//! products, restated factor for factor (`s1angle.h:239`, `:363–378`), and the E-forms are
//! upstream's on x86-64: `MathUtil::Round<int32_t>` is `FastIntRound`, the `cvtsd2si`
//! instruction, which rounds ties to even (FR-022). Outside the `int32` range the library
//! saturates, a documented deviation from C++'s −2147483648 (FR-023, U-014); the restatement
//! states the library's documented value there.
//!
//! The reduction `normalized` is described by `remainder_2pi` in `core/src/s1/angle_tests.rs`,
//! not here: it needs exact big-integer arithmetic (`num-bigint`, a dependency of `core`, not of
//! this crate), and Kani cannot check it anyway, since its `%` on `f64` is wrong (research R0,
//! Finding 1).
//!
//! The rounding is built from `floor`, never from `round_ties_even` (the library's choice) or
//! `%` (research R0). Booleans are combined with `&&`, `||` and `==` only.

use std::f64::consts::PI;

/// `S1Angle::Degrees(d)`, `(M_PI / 180) * d` (`s1angle.h:363–365`). Written `d * (PI / 180)`:
/// IEEE-754 multiplication is commutative, bit for bit, so the value is upstream's; the order is
/// the library's, so the checker need not prove the commutation (research R0 addendum).
#[must_use]
pub fn upstream_from_degrees(d: f64) -> f64 {
    d * (PI / 180.0)
}

/// `S1Angle::degrees()`, `(180 / M_PI) * radians_` (`s1angle.h:239–241`), written
/// `radians * (180 / PI)` (commutative, as above).
#[must_use]
pub fn upstream_degrees(r: f64) -> f64 {
    r * (180.0 / PI)
}

/// `10⁻ᵏ` as upstream writes it: `1e-5`, `1e-6`, `1e-7`.
fn scale_down(k: u32) -> f64 {
    match k {
        5 => 1e-5,
        6 => 1e-6,
        _ => 1e-7,
    }
}

/// `10ᵏ` as upstream writes it: `1e5`, `1e6`, `1e7`.
fn scale_up(k: u32) -> f64 {
    match k {
        5 => 1e5,
        6 => 1e6,
        _ => 1e7,
    }
}

/// `S1Angle::E5(n)`, `E6(n)`, `E7(n)`: `Degrees(1e-k * n)` (`s1angle.h:367–378`), as radians
/// (the product written `n * 1e-k`, commutative, as above).
#[must_use]
pub fn upstream_from_e(k: u32, n: i32) -> f64 {
    upstream_from_degrees(f64::from(n) * scale_down(k))
}

/// `v` rounded to the nearest integer, ties to even, as an `f64`; `v` finite. Built from
/// `floor`: `f = floor(v)`, `d = v − f` (exact, `0 ≤ d < 1`), and `f` is even when
/// `floor(f / 2) * 2 == f`.
#[must_use]
pub fn round_half_even(v: f64) -> f64 {
    let f = v.floor();
    let d = v - f;
    let even = (f / 2.0).floor() * 2.0 == f;
    if d < 0.5 || (d == 0.5 && even) {
        f
    } else {
        f + 1.0
    }
}

/// `S1Angle::e5()`, `e6()`, `e7()` of an angle of `r` radians:
/// `MathUtil::Round<int32_t>(1ek * degrees())` (`s1angle.h:248–264`), ties to even. Outside the
/// `int32` range the library's documented value (FR-023): `i32::MAX` above (+∞ included),
/// `i32::MIN` below (−∞ included), and 0 for NaN.
#[must_use]
pub fn upstream_e(k: u32, r: f64) -> i32 {
    let v = upstream_degrees(r) * scale_up(k);
    if v.is_nan() {
        return 0;
    }
    if v == f64::INFINITY {
        return i32::MAX;
    }
    if v == f64::NEG_INFINITY {
        return i32::MIN;
    }
    let n = round_half_even(v);
    if n > f64::from(i32::MAX) {
        i32::MAX
    } else if n < f64::from(i32::MIN) {
        i32::MIN
    } else {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "n is an integer within the i32 range here"
        )]
        let i = n as i32;
        i
    }
}

#[cfg(test)]
#[path = "angle_spec_tests.rs"]
mod angle_spec_tests;
