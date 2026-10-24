// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the real-set description (spec 005 T007), on the fixtures of
//! upstream's `r1interval_test.cc` and the corners the survey found.

use super::*;

/// An interval as a literal, the way the harnesses build them.
fn iv(lo: f64, hi: f64) -> Interval {
    Interval { lo, hi }
}

const INF: f64 = f64::INFINITY;

// Upstream's fixtures (`r1interval_test.cc`, `TestIntervalOps`).
fn unit() -> Interval {
    iv(0.0, 1.0)
}
fn neg_unit() -> Interval {
    iv(-1.0, 0.0)
}
fn half() -> Interval {
    iv(0.5, 0.5)
}
fn empty_i() -> Interval {
    iv(1.0, 0.0)
}

#[test]
fn emptiness_has_many_representations() {
    assert!(empty(empty_i()));
    assert!(empty(iv(5.0, -5.0)));
    assert!(empty(iv(INF, -INF)));
    assert!(empty(iv(f64::MIN_POSITIVE, 0.0)));
    assert!(!empty(half()));
    assert!(!empty(iv(-INF, INF)));
    assert!(!empty(iv(INF, INF)));
    // −0 and 0 are one number, so these are single points, not empty.
    assert!(!empty(iv(0.0, -0.0)));
    assert!(!empty(iv(-0.0, 0.0)));
}

#[test]
fn membership_and_interior() {
    assert!(member(unit(), 0.0) && member(unit(), 1.0) && member(unit(), 0.5));
    assert!(!member(unit(), -1e-300) && !member(unit(), 1.0 + f64::EPSILON));
    assert!(!interior(unit(), 0.0) && !interior(unit(), 1.0) && interior(unit(), 0.5));
    assert!(member(half(), 0.5) && !interior(half(), 0.5));
    assert!(!member(empty_i(), 0.5) && !interior(empty_i(), 0.5));
    assert!(member(iv(0.0, INF), INF) && !interior(iv(0.0, INF), INF));
    assert!(member(iv(-0.0, -0.0), 0.0));
}

#[test]
fn containment() {
    for x in [unit(), neg_unit(), half(), empty_i(), iv(-INF, INF)] {
        assert!(contains(x, empty_i()), "{x:?} contains the empty set");
        assert!(
            interior_contains(x, empty_i()),
            "{x:?} interior-contains the empty set"
        );
    }
    assert!(contains(unit(), half()) && interior_contains(unit(), half()));
    assert!(contains(unit(), unit()) && !interior_contains(unit(), unit()));
    assert!(!contains(empty_i(), half()) && !contains(half(), unit()));
    assert!(contains(iv(-INF, INF), iv(INF, INF)));
    assert!(!interior_contains(iv(-INF, INF), iv(INF, INF)));
    assert!(contains(iv(0.0, 0.0), iv(-0.0, -0.0)));
}

#[test]
fn intersection_relations() {
    assert!(intersects(unit(), neg_unit()) && intersects(neg_unit(), unit()));
    assert!(!interior_intersects(unit(), neg_unit()));
    assert!(intersects(unit(), half()) && interior_intersects(unit(), half()));
    assert!(!intersects(unit(), empty_i()) && !intersects(empty_i(), unit()));
    assert!(!interior_intersects(unit(), empty_i()));
    // A single point has an empty interior, so it interior-intersects nothing.
    assert!(intersects(half(), unit()) && !interior_intersects(half(), unit()));
    assert!(interior_intersects(
        unit(),
        iv(1.0 - f64::EPSILON / 2.0, 3.0)
    ));
    assert!(!interior_intersects(unit(), iv(1.0, 3.0)));
}

#[test]
fn finiteness() {
    assert!(finite(unit()) && finite(empty_i()));
    assert!(!finite(iv(0.0, INF)) && !finite(iv(-INF, 0.0)));
}
