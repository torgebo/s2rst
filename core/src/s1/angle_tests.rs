// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Regression tests for `Angle`'s reduction and E-forms (spec 007 FR-021 to FR-023), and the
//! exact-remainder description `remainder_2pi` that `normalized` is checked against (spec 007
//! data-model §1.2; Kani cannot check it, research R0 Finding 1).

use super::*;
use quickcheck_macros::quickcheck;

/// The IEEE-754 remainder of `x` by the `f64` nearest 2π, exactly, then −π becomes π, as
/// upstream's `S1Angle::Normalize` (`s1angle.cc`). The exact remainder lives in
/// `crate::test_util`, shared with spec 008's `Cap::rect_bound` tests.
fn remainder_2pi(x: f64) -> f64 {
    let rem = crate::test_util::ieee_remainder_2pi(x);
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

/// Spec 010 FR-017: the shared `remainder_2pi` (without `normalized`'s −π to π) is the exact
/// IEEE remainder, bit for bit, at every magnitude, the sign of a zero result included.
#[quickcheck]
fn prop_shared_remainder_is_exact(bits: u64) -> bool {
    let x = f64::from_bits(bits);
    let r = super::remainder_2pi(x);
    if !x.is_finite() {
        return r.is_nan();
    }
    r.to_bits() == crate::test_util::ieee_remainder_2pi(x).to_bits() && r.abs() <= PI
}

/// Spec 010 FR-017: the ties `(2k + 1)·π`, where the remainder's quotient rounds to the even
/// neighbour: ±π and ±5π keep their sign, ±3π and ±7π change it.
#[test]
fn shared_remainder_ties_round_to_even() {
    for (k, keeps) in [(1.0, true), (3.0, false), (5.0, true), (7.0, false)] {
        for x in [k * PI, -k * PI] {
            let r = super::remainder_2pi(x);
            assert_eq!(
                r.to_bits(),
                crate::test_util::ieee_remainder_2pi(x).to_bits(),
                "{x}"
            );
            let want = if keeps {
                PI.copysign(x)
            } else {
                -PI.copysign(x)
            };
            assert_eq!(r, want, "{x}");
        }
    }
    assert_eq!(
        super::remainder_2pi(-2.0 * PI).to_bits(),
        (-0.0f64).to_bits()
    );
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
