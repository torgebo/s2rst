// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Regression tests for `Angle`'s reduction and E-forms (spec 007 FR-021 to FR-023), and the
//! exact-remainder description `remainder_2pi` that `normalized` is checked against (spec 007
//! data-model §1.2; Kani cannot check it, research R0 Finding 1).

use super::*;
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use quickcheck_macros::quickcheck;

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
/// nearest `x / y`, ties to even, computed exactly; then −π becomes π, as upstream's
/// `S1Angle::Normalize` (`s1angle.cc`). A zero result has the sign of `x`, as IEEE's; ±∞ and
/// NaN give NaN.
fn remainder_2pi(x: f64) -> f64 {
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
    let rem = scale(r.to_f64().unwrap(), e);
    if rem == -PI { PI } else { rem }
}

/// The C++ results of the survey probe (`specs/007-formal-verify-chord-angle-angle/probe/cpp/
/// survey_cpp.txt`), and R0's input, where the old formula returned 4.0.
#[test]
fn normalized_matches_cpp_for_large_angles() {
    for (x, want) in [
        (1e6, 0xbfd6_e254_d0eb_fc80_u64),
        (1e16, 0x4005_1912_8f76_be50),
        (1e20, 0x3ffe_5456_15f7_0020),
        (1e300, 0xbfe7_264f_c07a_22c0),
        (-1e20, 0xbffe_5456_15f7_0020),
        (3.0 * PI, PI.to_bits()),
        (-3.0 * PI, PI.to_bits()),
        (5.0 * PI, PI.to_bits()),
        (-PI, PI.to_bits()),
        (7.0, 0x3fe6_f025_5dde_9740),
    ] {
        assert_eq!(
            Angle::from_radians(x).normalized().radians().to_bits(),
            want,
            "normalized({x:e})"
        );
    }
    let r0 = -2.462_748_394_469_102e16;
    let n = Angle::from_radians(r0).normalized().radians();
    assert!(-PI < n && n <= PI, "normalized({r0:e}) = {n}");
    assert_eq!(n.to_bits(), remainder_2pi(r0).to_bits());
}

#[test]
fn normalized_of_non_finite_is_nan() {
    for x in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(
            Angle::from_radians(x).normalized().radians().is_nan(),
            "{x}"
        );
    }
}

#[test]
fn remainder_2pi_is_ieee_remainder() {
    // Spot values of the description itself, independent of the library.
    assert_eq!(remainder_2pi(0.5), 0.5);
    assert_eq!(remainder_2pi(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(remainder_2pi(2.0 * PI).to_bits(), 0.0f64.to_bits());
    assert_eq!(remainder_2pi(-2.0 * PI).to_bits(), (-0.0f64).to_bits());
    assert_eq!(remainder_2pi(PI), PI);
    assert_eq!(remainder_2pi(-PI), PI);
    assert_eq!(remainder_2pi(1e20).to_bits(), 0x3ffe_5456_15f7_0020);
    assert_eq!(remainder_2pi(f64::from_bits(1)).to_bits(), 1);
}

/// FR-021: `normalized` is the exact remainder, bit for bit, at every magnitude (the angle is
/// drawn as a bit pattern, so every binade is as likely as any other), lies in (−π, π], and is
/// idempotent.
#[quickcheck]
fn prop_normalized_is_the_exact_remainder(bits: u64) -> bool {
    let x = f64::from_bits(bits);
    if !x.is_finite() {
        return true;
    }
    let n = Angle::from_radians(x).normalized().radians();
    n.to_bits() == remainder_2pi(x).to_bits()
        && -PI < n
        && n <= PI
        && Angle::from_radians(n).normalized().radians().to_bits() == n.to_bits()
}

/// FR-022: exact ties round to even, as C++ on x86-64 (`MathUtil::Round<int32_t>` is
/// `cvtsd2si`). The radians are the survey probe's, whose degrees × 10⁶ (or × 10⁷) is exactly
/// the half integer named; the results are C++'s
/// (`specs/007-formal-verify-chord-angle-angle/probe/cpp/survey_cpp.txt`).
#[test]
fn e_forms_round_ties_to_even() {
    for (bits, half, e6) in [
        (0x3e42_bd86_46c1_beed_u64, 0.5, 0),
        (0x3e5c_1c49_6a22_9e64, 1.5, 2),
        (0x3e67_6ce7_d872_2ea9, 2.5, 2),
        (0x3e75_1537_0f99_f6cb, 4.5, 4),
        (0x3e79_c498_a14a_6686, 5.5, 6),
        (0x3e7e_73fa_32fa_d641, 6.5, 6),
    ] {
        let a = Angle::from_radians(f64::from_bits(bits));
        assert_eq!(a.degrees() * 1e6, half, "the input is a tie");
        assert_eq!(a.e6(), e6, "e6 at {half}");
    }
    for (bits, e7) in [
        (0xbe0d_fc0a_0acf_97e2_u64, 0),
        (0xbe26_7d07_881b_b1e9, -2),
        (0xbe32_bd86_46c1_beed, -2),
    ] {
        assert_eq!(
            Angle::from_radians(f64::from_bits(bits)).e7(),
            e7,
            "{bits:#x}"
        );
    }
}

/// FR-023: outside the `i32` range the E-forms saturate, and NaN gives 0 (C++ returns
/// −2147483648 for all of these, U-014).
#[test]
fn e_forms_saturate_outside_the_range() {
    assert_eq!(Angle::from_radians(1e10).e7(), i32::MAX);
    assert_eq!(Angle::from_radians(-1e10).e7(), i32::MIN);
    assert_eq!(Angle::INFINITY.e5(), i32::MAX);
    assert_eq!((-Angle::INFINITY).e6(), i32::MIN);
    assert_eq!(Angle::from_radians(f64::NAN).e5(), 0);
}
