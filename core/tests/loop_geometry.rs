// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for Loop geometry operations, ported from C++ `s2loop_test.cc`.

use s2rst::s2::text_format;
use s2rst::s2::{Cell, CellId, LatLng, Loop, Region};
use std::f64::consts::PI;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

// ---------------------------------------------------------------------------
// 1. Empty loop
// ---------------------------------------------------------------------------
#[test]
fn test_empty_loop() {
    let l = Loop::empty();
    assert!(l.is_empty_loop());
    assert!(!l.is_full_loop());
    assert!(approx_eq(l.area(), 0.0, 1e-10));
    assert!(approx_eq(l.turning_angle(), -2.0 * PI, 1e-10));
}

// ---------------------------------------------------------------------------
// 2. Full loop
// ---------------------------------------------------------------------------
#[test]
fn test_full_loop() {
    let l = Loop::full();
    assert!(!l.is_empty_loop());
    assert!(l.is_full_loop());
    assert!(approx_eq(l.area(), 4.0 * PI, 1e-10));
    assert!(approx_eq(l.turning_angle(), 2.0 * PI, 1e-10));
}

// ---------------------------------------------------------------------------
// 3. Loop from Cell
// ---------------------------------------------------------------------------
#[test]
fn test_loop_from_cell() {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 5));
    let l = Loop::from_cell(&cell);

    assert_eq!(l.num_vertices(), 4);
    assert!(l.area() > 0.0);

    // All 4 vertices should be distinct.
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert!(
                (l.vertex(i).0 - l.vertex(j).0).norm() > 1e-15,
                "vertices {i} and {j} should be distinct"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Triangle loop containment
// ---------------------------------------------------------------------------
#[test]
fn test_triangle_containment() {
    let l = text_format::make_loop("0:0, 0:10, 10:0");

    // An interior point should be contained.
    let inside = LatLng::from_degrees(2.0, 2.0).to_point();
    assert!(l.contains_point(&inside), "should contain interior point");
    assert!(
        l.brute_force_contains_point(inside),
        "brute force should agree"
    );

    // A far-away exterior point should not be contained.
    let outside = LatLng::from_degrees(50.0, 50.0).to_point();
    assert!(
        !l.contains_point(&outside),
        "should not contain exterior point"
    );
    assert!(
        !l.brute_force_contains_point(outside),
        "brute force should agree"
    );
}

// ---------------------------------------------------------------------------
// 5. Quadrilateral loop
// ---------------------------------------------------------------------------
#[test]
fn test_quadrilateral_loop() {
    let l = text_format::make_loop("-10:-10, -10:10, 10:10, 10:-10");

    // Interior point
    let inside = LatLng::from_degrees(0.0, 0.0).to_point();
    assert!(l.contains_point(&inside), "center should be inside quad");

    // Exterior point
    let outside = LatLng::from_degrees(50.0, 0.0).to_point();
    assert!(
        !l.contains_point(&outside),
        "far point should be outside quad"
    );

    // Area should be roughly (20 deg)^2 in steradians
    let area = l.area();
    assert!(area > 0.1, "quad area should be substantial, got {area}");
    assert!(area < 0.5, "quad area should be bounded, got {area}");
}

// ---------------------------------------------------------------------------
// 6. Small loop area
// ---------------------------------------------------------------------------
#[test]
fn test_small_loop_area() {
    let l = text_format::make_loop("89:0, 89:120, 89:240");
    let area = l.area();
    assert!(area > 0.0, "area should be positive");
    assert!(area < 0.01, "area should be small, got {area}");
}

// ---------------------------------------------------------------------------
// 7. Loop rect bound
// ---------------------------------------------------------------------------
#[test]
fn test_loop_rect_bound() {
    let full = Loop::full();
    let rb = full.rect_bound();
    assert!(rb.is_full(), "full loop rect bound should be full");

    // Empty loop rect bound should be empty.
    let empty = Loop::empty();
    let rb = empty.rect_bound();
    assert!(rb.is_empty(), "empty loop rect bound should be empty");

    // A normal loop's rect bound should contain all vertices.
    let l = text_format::make_loop("10:20, 10:40, 30:40, 30:20");
    let rb = l.rect_bound();
    for i in 0..l.num_vertices() {
        let ll = LatLng::from_point(l.vertex(i));
        assert!(rb.contains_lat_lng(ll), "bound should contain vertex {i}");
    }
}

// ---------------------------------------------------------------------------
// 8. Turning angle of a CCW triangle
// ---------------------------------------------------------------------------
#[test]
fn test_loop_turning_angle_ccw_triangle() {
    // For a convex CCW loop, turning_angle = 2π - area (Gauss-Bonnet).
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    let area = l.area();
    let ta = l.turning_angle();
    let expected = 2.0 * PI - area;
    assert!(
        approx_eq(ta, expected, 1e-6),
        "turning angle {ta} not close to 2pi - area = {expected}"
    );
}

// ---------------------------------------------------------------------------
// 9. Gauss-Bonnet: area + turning_angle = 2π
// ---------------------------------------------------------------------------
#[test]
fn test_area_consistent_with_turning_angle() {
    let l = text_format::make_loop("10:20, 10:40, 30:40, 30:20");
    let area = l.area();
    let ta = l.turning_angle();
    assert!(
        approx_eq(area + ta, 2.0 * PI, 1e-6),
        "area({area}) + turning_angle({ta}) should be 2pi"
    );
}

// ---------------------------------------------------------------------------
// 10. Loop sign
// ---------------------------------------------------------------------------
#[test]
fn test_loop_sign_positive() {
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    assert_eq!(l.sign(), 1, "CCW loop should have sign +1");

    let mut hole = text_format::make_loop("0:0, 0:10, 10:0");
    hole.set_depth(1);
    assert_eq!(hole.sign(), -1, "hole loop should have sign -1");
}

// ---------------------------------------------------------------------------
// 11. Loop validate
// ---------------------------------------------------------------------------
#[test]
fn test_loop_validate_ok() {
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    assert!(l.validate().is_ok(), "valid loop should pass validate()");
    assert!(Loop::empty().validate().is_ok());
    assert!(Loop::full().validate().is_ok());
}

// ---------------------------------------------------------------------------
// 12. Loop contains cell
// ---------------------------------------------------------------------------
#[test]
fn test_loop_contains_cell() {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 10));

    let full = Loop::full();
    assert!(
        full.contains_cell(&cell),
        "full loop should contain any cell"
    );

    let empty = Loop::empty();
    assert!(
        !empty.contains_cell(&cell),
        "empty loop should not contain any cell"
    );
}

// ---------------------------------------------------------------------------
// 13. Loop intersects cell
// ---------------------------------------------------------------------------
#[test]
fn test_loop_intersects_cell() {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 10));

    let full = Loop::full();
    assert!(
        full.intersects_cell(&cell),
        "full loop should intersect any cell"
    );

    let empty = Loop::empty();
    assert!(
        !empty.intersects_cell(&cell),
        "empty loop should not intersect any cell"
    );
}

// ---------------------------------------------------------------------------
// 14. Loop equal
// ---------------------------------------------------------------------------
#[test]
fn test_loop_equal() {
    let l1 = text_format::make_loop("0:0, 0:10, 10:0");
    let l2 = text_format::make_loop("0:0, 0:10, 10:0");
    assert!(l1.equal(&l2), "identical loops should be equal");

    let l3 = text_format::make_loop("0:0, 0:20, 20:0");
    assert!(!l1.equal(&l3), "different loops should not be equal");
}

// ---------------------------------------------------------------------------
// 15. Inverted loop area complementary
// ---------------------------------------------------------------------------
#[test]
fn test_inverted_loop_area() {
    let original = text_format::make_loop("0:0, 0:10, 10:0");
    let original_area = original.area();

    let mut inverted = text_format::make_loop("0:0, 0:10, 10:0");
    inverted.invert();
    let inverted_area = inverted.area();

    assert!(
        approx_eq(original_area + inverted_area, 4.0 * PI, 1e-6),
        "original({original_area}) + inverted({inverted_area}) should equal 4π"
    );
}

// ---------------------------------------------------------------------------
// 16. Loop normalize
// ---------------------------------------------------------------------------
#[test]
fn test_loop_normalize() {
    let mut l = text_format::make_loop("0:0, 10:0, 0:10");
    l.normalize();
    // After normalization, the area should be less than 2π (the smaller half).
    assert!(
        l.area() <= 2.0 * PI + 1e-10,
        "normalized area should be ≤ 2π"
    );
}
