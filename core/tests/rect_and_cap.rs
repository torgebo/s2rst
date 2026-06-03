// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for `LatLngRect` and Cap, ported from C++ `s2latlng_rect_test.cc`
//! and `s2cap_test.cc`.

use s2rst::r1;
use s2rst::s1;
use s2rst::s1::Angle;
use s2rst::s2::{Cap, LatLng, Point, Rect};
use std::f64::consts::PI;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

// ---------------------------------------------------------------------------
// Rect tests
// ---------------------------------------------------------------------------

#[test]
fn test_rect_empty() {
    let r = Rect::empty();
    assert!(r.is_empty());
    assert!(!r.is_full());
    assert_eq!(r.area(), 0.0);
}

#[test]
fn test_rect_full() {
    let r = Rect::full();
    assert!(!r.is_empty());
    assert!(r.is_full());
    assert!(approx_eq(r.area(), 4.0 * PI, 1e-10));
}

#[test]
fn test_rect_from_lat_lng() {
    let ll = LatLng::from_degrees(45.0, 90.0);
    let r = Rect::from_lat_lng(ll);
    assert!(r.is_point());
    assert!(r.contains_lat_lng(ll));
}

#[test]
fn test_rect_from_center_size() {
    let center = LatLng::from_degrees(10.0, 20.0);
    let size = LatLng::from_degrees(6.0, 8.0);
    let r = Rect::from_center_size(center, size);

    assert!(!r.is_empty());
    assert!(r.contains_lat_lng(center));
    // lo should be (10-3, 20-4) = (7, 16), hi should be (13, 24)
    let lo = r.lo();
    let hi = r.hi();
    assert!(approx_eq(lo.lat.degrees(), 7.0, 1e-10));
    assert!(approx_eq(lo.lng.degrees(), 16.0, 1e-10));
    assert!(approx_eq(hi.lat.degrees(), 13.0, 1e-10));
    assert!(approx_eq(hi.lng.degrees(), 24.0, 1e-10));
}

#[test]
fn test_rect_contains() {
    let big = Rect::from_center_size(
        LatLng::from_degrees(0.0, 0.0),
        LatLng::from_degrees(40.0, 40.0),
    );
    let small = Rect::from_center_size(
        LatLng::from_degrees(0.0, 0.0),
        LatLng::from_degrees(10.0, 10.0),
    );
    assert!(big.contains(small));
    assert!(!small.contains(big));
}

#[test]
fn test_rect_intersects() {
    let r1 = Rect::from_center_size(
        LatLng::from_degrees(0.0, 0.0),
        LatLng::from_degrees(20.0, 20.0),
    );
    let r2 = Rect::from_center_size(
        LatLng::from_degrees(5.0, 5.0),
        LatLng::from_degrees(20.0, 20.0),
    );
    assert!(r1.intersects(r2));

    let far = Rect::from_center_size(
        LatLng::from_degrees(80.0, 80.0),
        LatLng::from_degrees(5.0, 5.0),
    );
    assert!(!r1.intersects(far));
}

#[test]
fn test_rect_union() {
    let r1 = Rect::from_lat_lng(LatLng::from_degrees(10.0, 20.0));
    let r2 = Rect::from_lat_lng(LatLng::from_degrees(30.0, 40.0));
    let u = r1.union(r2);
    assert!(u.contains_lat_lng(LatLng::from_degrees(10.0, 20.0)));
    assert!(u.contains_lat_lng(LatLng::from_degrees(30.0, 40.0)));
    assert!(u.contains_lat_lng(LatLng::from_degrees(20.0, 30.0)));
}

#[test]
fn test_rect_intersection() {
    let r1 = Rect::from_center_size(
        LatLng::from_degrees(0.0, 0.0),
        LatLng::from_degrees(20.0, 20.0),
    );
    let r2 = Rect::from_center_size(
        LatLng::from_degrees(5.0, 5.0),
        LatLng::from_degrees(20.0, 20.0),
    );
    let inter = r1.intersection(r2);
    assert!(!inter.is_empty());
    assert!(r1.contains(inter));
    assert!(r2.contains(inter));
}

#[test]
fn test_rect_expanded() {
    let r = Rect::from_center_size(
        LatLng::from_degrees(0.0, 0.0),
        LatLng::from_degrees(10.0, 10.0),
    );
    let expanded = r.expanded(LatLng::from_degrees(5.0, 5.0));
    assert!(expanded.contains(r));
    assert!(!r.contains(expanded));
}

#[test]
fn test_rect_add_point() {
    let ll1 = LatLng::from_degrees(10.0, 20.0);
    let ll2 = LatLng::from_degrees(30.0, 40.0);
    let r = Rect::from_lat_lng(ll1).add_point(ll2);
    assert!(r.contains_lat_lng(ll1));
    assert!(r.contains_lat_lng(ll2));
}

#[test]
fn test_rect_vertex() {
    let r = Rect::new(r1::Interval::new(0.1, 0.3), s1::Interval::new(0.2, 0.4));
    use s2rst::s2::RectVertex;
    // vertex(LowerLeft) = (lat.lo, lng.lo)
    let v0 = r.vertex(RectVertex::LowerLeft);
    assert!(approx_eq(v0.lat.radians(), 0.1, 1e-14));
    assert!(approx_eq(v0.lng.radians(), 0.2, 1e-14));
    // vertex(UpperRight) = (lat.hi, lng.hi)
    let v2 = r.vertex(RectVertex::UpperRight);
    assert!(approx_eq(v2.lat.radians(), 0.3, 1e-14));
    assert!(approx_eq(v2.lng.radians(), 0.4, 1e-14));
}

#[test]
fn test_rect_cap_bound() {
    let r = Rect::from_center_size(
        LatLng::from_degrees(10.0, 20.0),
        LatLng::from_degrees(10.0, 10.0),
    );
    let cap = r.cap_bound();
    assert!(!cap.is_empty(), "cap_bound should not be empty");
    // The cap should be large enough to contain the center.
    let center = r.center().to_point();
    assert!(
        cap.contains_point(center),
        "cap_bound should contain rect center"
    );
    // Slightly expanded cap should contain all vertices (accounting for boundary precision).
    let expanded = cap.expanded(Angle::from_degrees(0.01));
    for v in s2rst::s2::RectVertex::iter() {
        let p = r.vertex(v).to_point();
        assert!(
            expanded.contains_point(p),
            "expanded cap_bound should contain vertex {v:?}"
        );
    }
}

#[test]
fn test_rect_approx_equal() {
    let r = Rect::from_center_size(
        LatLng::from_degrees(10.0, 20.0),
        LatLng::from_degrees(10.0, 10.0),
    );
    assert!(r.approx_eq(r), "rect should be approx_equal to itself");
}

// ---------------------------------------------------------------------------
// Cap tests
// ---------------------------------------------------------------------------

#[test]
fn test_cap_empty() {
    let c = Cap::empty();
    assert!(c.is_empty());
    assert!(!c.is_full());
    assert_eq!(c.area(), 0.0);
}

#[test]
fn test_cap_full() {
    let c = Cap::full();
    assert!(!c.is_empty());
    assert!(c.is_full());
    assert!(approx_eq(c.area(), 4.0 * PI, 1e-10));
    // Full cap contains any point.
    assert!(c.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
    assert!(c.contains_point(Point::from_coords(0.0, 0.0, -1.0)));
}

#[test]
fn test_cap_from_point() {
    let p = Point::from_coords(0.0, 1.0, 0.0);
    let c = Cap::from_point(p);
    assert!(!c.is_empty());
    assert!(!c.is_full());
    assert!(c.contains_point(p));
    assert!(c.height() < 1e-15);
}

#[test]
fn test_cap_contains_point() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let c = Cap::from_center_angle(center, Angle::from_degrees(10.0));
    assert!(c.contains_point(center));

    // Antipodal point should not be contained.
    let antipode = Point::from_coords(-1.0, 0.0, 0.0);
    assert!(!c.contains_point(antipode));
}

#[test]
fn test_cap_containment() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let big = Cap::from_center_angle(center, Angle::from_degrees(20.0));
    let small = Cap::from_center_angle(center, Angle::from_degrees(5.0));
    assert!(big.contains(small));
    assert!(!small.contains(big));
}

#[test]
fn test_cap_intersection() {
    let c1 = Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(30.0));
    let c2 = Cap::from_center_angle(Point::from_coords(0.0, 1.0, 0.0), Angle::from_degrees(70.0));
    assert!(c1.intersects(c2), "overlapping caps should intersect");

    let c3 = Cap::from_center_angle(
        Point::from_coords(-1.0, 0.0, 0.0),
        Angle::from_degrees(10.0),
    );
    assert!(!c1.intersects(c3), "distant caps should not intersect");
}

#[test]
fn test_cap_complement() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let c = Cap::from_center_angle(center, Angle::from_degrees(30.0));
    let comp = c.complement();

    // Center of original should not be in complement.
    assert!(!comp.contains_point(center));
    // Antipodal point should be in complement.
    let antipode = Point::from_coords(-1.0, 0.0, 0.0);
    assert!(comp.contains_point(antipode));
}

#[test]
fn test_cap_union() {
    let c1 = Cap::from_center_angle(Point::from_coords(1.0, 0.0, 0.0), Angle::from_degrees(10.0));
    let c2 = Cap::from_center_angle(Point::from_coords(0.0, 1.0, 0.0), Angle::from_degrees(10.0));
    let u = c1.union(c2);
    assert!(u.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
    assert!(u.contains_point(Point::from_coords(0.0, 1.0, 0.0)));
}

#[test]
fn test_cap_add_point() {
    let c = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
    let p2 = Point::from_coords(0.0, 1.0, 0.0);
    let expanded = c.add_point(p2);
    assert!(expanded.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
    assert!(expanded.contains_point(p2));
}

#[test]
fn test_cap_rect_bound() {
    // Small cap near north pole: rect_bound should have lat.hi near π/2.
    let center = Point::from_coords(0.0, 0.0, 1.0);
    let c = Cap::from_center_angle(center, Angle::from_degrees(5.0));
    let rb = c.rect_bound();
    assert!(
        approx_eq(rb.lat.hi, PI / 2.0, 0.01),
        "lat.hi {} should be near π/2",
        rb.lat.hi
    );
}

#[test]
fn test_cap_approx_equal() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let c = Cap::from_center_angle(center, Angle::from_degrees(10.0));
    assert!(c.approx_eq(c), "cap should be approx_equal to itself");
}
