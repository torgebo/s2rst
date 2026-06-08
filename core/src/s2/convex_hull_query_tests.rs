// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::ConvexHullQuery`]: non-finite input
//! points (NaN / ∞) are filtered out instead of crossing into the exact
//! geometric predicates, where converting a non-finite coordinate to
//! `ExactFloat` panics. Written for this crate, not ported from upstream S2.

use super::*;
use crate::r3::Vector;

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

#[test]
fn nan_point_is_ignored_without_panicking() {
    let mut q = ConvexHullQuery::new();
    q.add_point(p(0.0, 0.0));
    q.add_point(p(0.0, 10.0));
    q.add_point(Point(Vector::new(f64::NAN, 0.0, 0.0)));
    // Must not panic; the NaN point contributes nothing to the hull.
    let hull = q.convex_hull();
    let _ = hull.area();
    assert_eq!(hull.num_vertices(), 3); // degenerate 2-point edge hull
}

#[test]
fn infinite_coordinate_point_is_ignored() {
    // `from_coords` with an infinite coordinate normalizes to NaN.
    let bad = Point::from_coords(f64::INFINITY, 0.0, 0.0);
    assert!(!bad.x().is_finite() || !bad.y().is_finite() || !bad.z().is_finite());

    let mut q = ConvexHullQuery::new();
    q.add_point(p(0.0, 0.0));
    q.add_point(p(10.0, 0.0));
    q.add_point(p(0.0, 10.0));
    q.add_point(bad);
    let with_bad = q.convex_hull();

    let mut clean_q = ConvexHullQuery::new();
    clean_q.add_point(p(0.0, 0.0));
    clean_q.add_point(p(10.0, 0.0));
    clean_q.add_point(p(0.0, 10.0));
    let clean = clean_q.convex_hull();

    assert_eq!(with_bad.num_vertices(), clean.num_vertices());
    assert!((with_bad.area() - clean.area()).abs() < 1e-12);
}

#[test]
fn only_non_finite_points_yields_empty_hull() {
    let mut q = ConvexHullQuery::new();
    q.add_point(Point(Vector::new(f64::NAN, 0.0, 0.0)));
    q.add_point(Point(Vector::new(f64::INFINITY, f64::NEG_INFINITY, 0.0)));
    let hull = q.convex_hull();
    assert!(hull.is_empty_loop());
}

#[test]
fn add_points_filters_non_finite() {
    let mut q = ConvexHullQuery::new();
    q.add_points(&[
        p(0.0, 0.0),
        Point(Vector::new(f64::NAN, 1.0, 2.0)),
        p(0.0, 10.0),
        p(10.0, 5.0),
    ]);
    let hull = q.convex_hull();
    assert!(hull.num_vertices() >= 3);
    assert!(hull.area() > 0.0);
}
