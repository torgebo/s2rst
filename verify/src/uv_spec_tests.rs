// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the u-v rectangle description (spec 006 T007), on the fixtures of
//! upstream's `r2rect_test.cc` and the corners the survey found.

use super::*;
use s2rst::r1::Interval;

const INF: f64 = f64::INFINITY;

/// A rectangle as a literal, the way the harnesses build them.
fn rect(x_lo: f64, x_hi: f64, y_lo: f64, y_hi: f64) -> Rect {
    Rect {
        x: Interval { lo: x_lo, hi: x_hi },
        y: Interval { lo: y_lo, hi: y_hi },
    }
}

fn pt(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn bits(p: Point) -> [u64; 2] {
    [p.x.to_bits(), p.y.to_bits()]
}

#[test]
fn the_empty_rectangle_has_many_representations() {
    for r in [
        rect(1.0, 0.0, 1.0, 0.0),
        rect(5.0, -5.0, 0.0, -1.0),
        rect(INF, -INF, INF, -INF),
    ] {
        assert!(valid(r) && empty(r));
    }
    assert!(!empty(rect(0.0, 1.0, 0.0, 1.0)));
}

#[test]
fn one_axis_empty_is_invalid() {
    for r in [rect(1.0, 0.0, 0.0, 1.0), rect(0.0, 1.0, 1.0, 0.0)] {
        assert!(!emptiness_rule(r));
        assert!(!valid(r));
    }
}

#[test]
fn a_nan_bound_is_invalid_though_it_passes_the_emptiness_rule() {
    // Every comparison with NaN is false, so NaN never makes an axis empty (K4).
    for r in [
        rect(f64::NAN, 1.0, 0.0, 1.0),
        rect(0.0, 1.0, 0.0, f64::NAN),
        rect(f64::NAN, f64::NAN, f64::NAN, f64::NAN),
    ] {
        assert!(emptiness_rule(r));
        assert!(!valid(r));
    }
    // A NaN bound contains nothing, not even the point it was built from.
    assert!(!member(rect(f64::NAN, 1.0, 0.0, 1.0), pt(0.5, 0.5)));
}

#[test]
fn infinite_bounds_are_valid_but_not_finite() {
    let r = rect(-INF, INF, 0.0, INF);
    assert!(valid(r) && !empty(r) && !finite(r));
    assert!(member(r, pt(-INF, INF)));
    assert!(finite(rect(0.0, 1.0, 2.0, 3.0)));
}

#[test]
fn signed_zeros_are_one_number() {
    let r = rect(-0.0, 0.0, 0.0, -0.0);
    assert!(valid(r) && !empty(r));
    assert!(member(r, pt(0.0, -0.0)) && member(r, pt(-0.0, 0.0)));
}

#[test]
fn a_single_point_has_no_interior() {
    let r = rect(0.5, 0.5, 2.0, 2.0);
    assert!(member(r, pt(0.5, 2.0)));
    assert!(!interior(r, pt(0.5, 2.0)));
    assert!(contains(r, r));
    assert!(!interior_contains(r, r));
    assert!(intersects(r, r));
    assert!(!interior_intersects(r, r));
}

#[test]
fn interior_membership_is_strict() {
    let r = rect(0.0, 1.0, 0.0, 1.0);
    assert!(interior(r, pt(0.5, 0.5)));
    assert!(!interior(r, pt(0.0, 0.5)) && !interior(r, pt(0.5, 1.0)));
    assert!(member(r, pt(0.0, 0.5)));
}

#[test]
fn the_empty_rectangle_is_contained_in_everything() {
    let e = rect(1.0, 0.0, 1.0, 0.0);
    for r in [rect(0.0, 1.0, 0.0, 1.0), e, rect(0.5, 0.5, 2.0, 2.0)] {
        assert!(contains(r, e));
        assert!(interior_contains(r, e));
        assert!(!intersects(r, e) && !intersects(e, r));
        assert!(!interior_intersects(r, e) && !interior_intersects(e, r));
    }
    assert!(!contains(e, rect(0.0, 1.0, 0.0, 1.0)));
}

#[test]
fn the_relations_are_products() {
    // Upstream's fixtures (`r2rect_test.cc`, `TestIntervalOps`): `sw`, `ne` and their union.
    let sw = rect(0.0, 0.25, 0.0, 0.25);
    let ne = rect(0.25, 0.5, 0.25, 0.5);
    let both = rect(0.0, 0.5, 0.0, 0.5);
    assert!(contains(both, sw) && contains(both, ne));
    assert!(!contains(sw, ne) && !contains(ne, sw));
    assert!(intersects(sw, ne));
    assert!(!interior_intersects(sw, ne));
    assert!(interior_intersects(both, ne));
    assert!(!interior_contains(both, ne));
    assert!(interior_contains(rect(-1.0, 1.0, -1.0, 1.0), both));
    // Overlap on one axis only is not an intersection.
    assert!(!intersects(
        rect(0.0, 1.0, 0.0, 1.0),
        rect(0.5, 2.0, 2.0, 3.0)
    ));
}

#[test]
fn the_vertices_go_counter_clockwise_from_the_lower_left() {
    let r = rect(0.0, 1.0, 2.0, 3.0);
    assert_eq!(bits(vertex(r, 0)), bits(pt(0.0, 2.0)));
    assert_eq!(bits(vertex(r, 1)), bits(pt(1.0, 2.0)));
    assert_eq!(bits(vertex(r, 2)), bits(pt(1.0, 3.0)));
    assert_eq!(bits(vertex(r, 3)), bits(pt(0.0, 3.0)));
    assert_eq!(bits(vertex(r, 4)), bits(vertex(r, 0)));
    // The index is taken modulo 4, negative indices included.
    assert_eq!(bits(vertex(r, -1)), bits(pt(0.0, 3.0)));
    assert_eq!(bits(vertex(r, i32::MIN)), bits(vertex(r, 0)));
    assert_eq!(bits(vertex(r, i32::MAX)), bits(vertex(r, 3)));
}
