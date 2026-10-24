// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the arc description (spec 004 T009), on the fixtures of upstream's
//! `s1interval_test.cc`.

use super::*;
use std::f64::consts::{FRAC_PI_2, PI};

/// An interval as a literal, the way the harnesses build them.
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

// Upstream's fixtures, after its constructor's −π normalisation.
fn empty_i() -> Interval {
    iv(PI, -PI)
}
fn full_i() -> Interval {
    iv(-PI, PI)
}
fn zero() -> Interval {
    iv(0.0, 0.0)
}
fn quad12() -> Interval {
    iv(0.0, PI)
}
fn quad23() -> Interval {
    iv(FRAC_PI_2, -FRAC_PI_2)
}
fn quad41() -> Interval {
    iv(-FRAC_PI_2, FRAC_PI_2)
}
fn quad123() -> Interval {
    iv(0.0, -FRAC_PI_2)
}

#[test]
fn validity_of_special_bit_patterns() {
    assert!(valid(0.0, -0.0) && valid(-0.0, 0.0));
    assert!(valid(PI, PI) && valid(PI, -PI) && valid(-PI, PI));
    assert!(!valid(-PI, -PI) && !valid(-PI, 0.0) && !valid(0.0, -PI));
    assert!(!valid(f64::NAN, 0.0) && !valid(0.0, f64::NAN));
    assert!(!valid(f64::INFINITY, 0.0) && !valid(0.0, f64::NEG_INFINITY));
    assert!(valid(PI, corner_hi()) && valid(corner_hi(), PI));
    assert!(!valid(4.0, 0.0));
}

#[test]
fn classification() {
    assert!(empty(empty_i()) && inverted(empty_i()) && !full(empty_i()));
    assert!(full(full_i()) && !inverted(full_i()) && !empty(full_i()));
    assert!(inverted(quad23()) && !inverted(quad12()));
}

#[test]
fn membership_of_the_fixtures() {
    for p in [0.0, FRAC_PI_2, PI, -PI, -FRAC_PI_2, 1e-300] {
        assert!(!member(empty_i(), p) && member(full_i(), p), "{p}");
        assert!(interior(full_i(), p) && !interior(empty_i(), p), "{p}");
    }
    // quad12 = [0, π] contains π, so it contains −π too: one point.
    assert!(member(quad12(), 0.0) && member(quad12(), PI) && member(quad12(), -PI));
    assert!(!member(quad12(), -FRAC_PI_2));
    assert!(interior(quad12(), FRAC_PI_2) && !interior(quad12(), 0.0));
    assert!(!interior(quad12(), PI) && !interior(quad12(), -PI));
    // quad23 = [π/2, −π/2] wraps across ±π.
    assert!(member(quad23(), PI) && member(quad23(), -PI) && !member(quad23(), 0.0));
    assert!(interior(quad23(), PI) && !interior(quad23(), FRAC_PI_2));
    // quad41 = [−π/2, π/2].
    assert!(member(quad41(), 0.0) && !member(quad41(), PI));
    // A single point: in the arc, not in its interior.
    assert!(member(zero(), 0.0) && member(zero(), -0.0) && !interior(zero(), 0.0));
}

#[test]
fn relations_of_the_fixtures() {
    // Upstream's s1interval_test.cc `IntervalOps` expectations for these pairs.
    assert!(contains(quad12(), zero()) && !contains(zero(), quad12()));
    assert!(contains(full_i(), quad23()) && !contains(quad23(), full_i()));
    assert!(contains(quad12(), empty_i()) && contains(empty_i(), empty_i()));
    assert!(!contains(quad12(), quad23()) && intersects(quad12(), quad23()));
    assert!(contains(quad123(), quad12()) && !contains(quad12(), quad123()));
    // Both of quad123's bounds lie in quad41, but quad123 runs out of it through π/2.
    assert!(member(quad41(), quad123().lo) && member(quad41(), quad123().hi));
    assert!(!contains(quad41(), quad123()));
    assert!(interior_contains(quad123(), iv(0.5, 1.0)) && !interior_contains(quad12(), quad12()));
    assert!(interior_intersects(quad12(), quad23()) && !interior_intersects(zero(), full_i()));
    assert!(interior_intersects(full_i(), zero()) && !intersects(empty_i(), full_i()));
}

#[test]
fn a_float_free_gap_is_still_a_gap() {
    // [next_up(−π), π] holds every float, π (= −π) included, but not the reals strictly
    // between −π and next_up(−π): it does not contain the full circle (C++ agrees).
    let almost = iv(corner_hi(), PI);
    for p in [-PI, PI, corner_hi(), 0.0] {
        assert!(member(almost, p));
    }
    assert!(!contains(almost, full_i()) && contains(full_i(), almost));
    // The corner is the complementary one-ulp arc: it meets `almost` only at ±π.
    let corner = iv(PI, corner_hi());
    assert!(intersects(almost, corner) && !interior_intersects(almost, corner));
}

#[test]
fn cyclic_order_counter_clockwise() {
    // From 1.0, counter-clockwise: 1.0, 2.0, π (= −π), then −3.0 and 0.5.
    let order = [1.0, 2.0, PI, -3.0, 0.5];
    for w in order.windows(2) {
        assert!(ccw_lt(1.0, w[0], w[1]) && ccw_le(1.0, w[0], w[1]), "{w:?}");
        assert!(!ccw_le(1.0, w[1], w[0]), "{w:?}");
    }
    assert!(ccw_le(1.0, -PI, PI) && ccw_le(1.0, PI, -PI) && !ccw_lt(1.0, PI, -PI));
}

#[test]
fn upstream_length_and_the_corner() {
    assert_eq!(upstream_length(full_i()), 2.0 * PI);
    assert_eq!(upstream_length(empty_i()), -1.0);
    assert_eq!(upstream_length(quad12()), PI);
    assert_eq!(upstream_length(zero()), 0.0);
    let corner = iv(PI, corner_hi());
    assert!(is_corner(corner) && valid(corner.lo, corner.hi) && !empty(corner));
    assert_eq!(upstream_length(corner), -1.0);
    // The true length: one ulp of π, computed in the stable form.
    assert_eq!((corner.hi + PI) - (corner.lo - PI), ULP_PI);
    assert_eq!(ULP_PI, PI.next_up() - PI);
}

#[test]
fn ulps_apart_counts_floats() {
    assert_eq!(ulps_apart(1.0, 1.0), 0);
    assert_eq!(ulps_apart(1.0, 1.0f64.next_up()), 1);
    assert_eq!(ulps_apart(-0.0, 0.0), 1);
    assert_eq!(ulps_apart(-f64::from_bits(1), f64::from_bits(1)), 3);
    assert_eq!(ulps_apart(PI, PI.next_down().next_down()), 2);
}
