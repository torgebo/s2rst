// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the rectangle description (spec 005 T009).

use super::*;
use s2rst::{r1, s1};

/// A rectangle as a literal, the way the harnesses build them.
fn rect(lat_lo: f64, lat_hi: f64, lng_lo: f64, lng_hi: f64) -> Rect {
    Rect {
        lat: r1::Interval {
            lo: lat_lo,
            hi: lat_hi,
        },
        lng: s1::Interval {
            lo: lng_lo,
            hi: lng_hi,
        },
    }
}

const H: f64 = FRAC_PI_2;

fn empty_r() -> Rect {
    rect(1.0, 0.0, PI, -PI)
}
fn full_r() -> Rect {
    rect(-H, H, -PI, PI)
}

#[test]
fn validity_rules_one_at_a_time() {
    assert!(valid(empty_r()) && valid(full_r()));
    assert!(valid(rect(0.0, 0.5, 1.0, 2.0)));
    // Latitudes out of range.
    assert!(!valid(rect(0.0, H + 1e-15, 1.0, 2.0)));
    assert!(!valid(rect(-H - 1e-15, 0.0, 1.0, 2.0)));
    assert!(!valid(rect(f64::NAN, 0.0, 1.0, 2.0)));
    // An invalid arc: −π only starts the full interval or ends the empty one.
    assert!(!valid(rect(0.0, 0.5, -PI, 0.0)));
    assert!(!valid(rect(0.0, 0.5, 4.0, 0.0)));
    // Emptiness that disagrees.
    assert!(!valid(rect(1.0, 0.0, 1.0, 2.0)));
    assert!(!valid(rect(-H, H, PI, -PI)));
    // A single point and a rectangle across the antimeridian.
    assert!(valid(rect(0.3, 0.3, PI, PI)));
    assert!(valid(rect(-0.1, 0.1, 3.0, -3.0)));
}

#[test]
fn membership_screens_invalid_points() {
    let r = rect(-0.1, 0.1, 3.0, -3.0); // across the antimeridian
    assert!(member(r, 0.0, PI) && member(r, 0.0, -PI) && member(r, 0.1, 3.0));
    assert!(!member(r, 0.0, 0.0) && !member(r, 0.2, PI));
    assert!(!member(full_r(), H + 1e-15, 0.0));
    assert!(!member(full_r(), 0.0, PI + 1e-15));
    assert!(!member(full_r(), f64::NAN, 0.0));
    assert!(member(full_r(), H, 0.0) && member(full_r(), -H, PI));
    assert!(!member(empty_r(), 0.0, 0.0));
    assert!(interior_member(full_r(), 0.0, 1.0) && !interior_member(full_r(), H, 1.0));
    assert!(!interior_member(r, 0.0, 3.0) && interior_member(r, 0.0, PI));
}

#[test]
fn relations() {
    let a = rect(-0.5, 0.5, 3.0, -3.0);
    let b = rect(-0.1, 0.1, 3.1, -3.1);
    assert!(contains(a, b) && interior_contains(a, b) && !contains(b, a));
    assert!(intersects(a, b) && intersects(b, a) && interior_intersects(a, b));
    for x in [a, b, full_r(), empty_r()] {
        assert!(contains(x, empty_r()) && !intersects(x, empty_r()));
        assert!(contains(full_r(), x));
    }
    // The same latitudes, disjoint longitudes.
    let c = rect(-0.5, 0.5, 0.0, 1.0);
    assert!(!intersects(a, c) && !contains(a, c));
    // Touching at a latitude edge: they intersect, but not in the interior.
    let d = rect(0.5, 1.0, 3.0, -3.0);
    assert!(intersects(a, d) && !interior_intersects(a, d));
}

#[test]
fn poles() {
    assert!(touches_pole(full_r()) && touches_pole(rect(1.0, H, 0.0, 1.0)));
    assert!(touches_pole(rect(-H, -H, 0.0, 0.0)));
    assert!(!touches_pole(rect(-1.0, 1.0, 0.0, 1.0)) && !touches_pole(empty_r()));
}

#[test]
fn ll_validity() {
    assert!(ll_valid(H, PI) && ll_valid(-H, -PI) && ll_valid(0.0, -0.0));
    assert!(!ll_valid(H + 1e-15, 0.0) && !ll_valid(0.0, -PI - 1e-15));
    assert!(!ll_valid(f64::NAN, 0.0) && !ll_valid(0.0, f64::NAN));
    assert!(!ll_valid(0.0, f64::INFINITY));
}
