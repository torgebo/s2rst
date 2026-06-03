// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for Polygon operations, ported from C++ `s2polygon_test.cc`.

use s2rst::s2::shape::{Dimension, Shape};
use s2rst::s2::text_format;
use s2rst::s2::{Cell, CellId, LatLng, Loop, Point, Polygon, Region};
use std::f64::consts::PI;

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

// ---------------------------------------------------------------------------
// 1. Empty polygon
// ---------------------------------------------------------------------------
#[test]
fn test_empty_polygon() {
    let poly = Polygon::empty();
    assert!(poly.is_empty_polygon());
    assert!(!poly.is_full_polygon());
    assert_eq!(poly.num_loops(), 0);
    assert_eq!(poly.num_vertices(), 0);
    assert_eq!(poly.area(), 0.0);
}

// ---------------------------------------------------------------------------
// 2. Full polygon
// ---------------------------------------------------------------------------
#[test]
fn test_full_polygon() {
    let poly = Polygon::full();
    assert!(!poly.is_empty_polygon());
    assert!(poly.is_full_polygon());
    assert_eq!(poly.num_loops(), 1);

    let area = poly.area();
    assert!(
        approx_eq(area, 4.0 * PI, 1e-10),
        "full polygon area should be ~4π, got {area}"
    );
}

// ---------------------------------------------------------------------------
// 3. Polygon from single loop
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_from_single_loop() {
    let vertices = vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)];
    let loop_ = Loop::new(vertices.clone());
    let poly = Polygon::from_loops(vec![loop_]);

    assert_eq!(poly.num_loops(), 1);
    assert_eq!(poly.loop_at(0).num_vertices(), 3);

    for (i, v) in vertices.iter().enumerate() {
        let pv = poly.loop_at(0).vertex(i);
        let dist = (pv.0 - v.0).norm();
        assert!(dist < 1e-14, "vertex {i} mismatch: dist = {dist}");
    }

    assert!(
        poly.area() > 0.0,
        "single-loop polygon should have positive area"
    );
}

// ---------------------------------------------------------------------------
// 4. Polygon from cell
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_from_cell() {
    let cell_id = CellId::from_face_pos_level(3, 0, 10);
    let cell = Cell::from_cell_id(cell_id);
    let poly = Polygon::from_cell(&cell);

    assert_eq!(poly.num_loops(), 1);
    assert_eq!(poly.loop_at(0).num_vertices(), 4);
    assert!(poly.area() > 0.0, "cell polygon should have positive area");
}

// ---------------------------------------------------------------------------
// 5. Polygon with hole
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_with_hole() {
    let outer = Loop::new(vec![
        p(-20.0, -20.0),
        p(-20.0, 20.0),
        p(20.0, 20.0),
        p(20.0, -20.0),
    ]);
    let outer_area = outer.area();

    let inner = Loop::new(vec![p(-5.0, -5.0), p(-5.0, 5.0), p(5.0, 5.0), p(5.0, -5.0)]);

    let poly = Polygon::from_loops(vec![outer, inner]);

    assert!(
        poly.has_holes(),
        "polygon with hole should report has_holes"
    );
    assert_eq!(poly.num_loops(), 2);
    assert!(
        poly.area() < outer_area,
        "polygon with hole should have less area than shell"
    );
    assert!(
        poly.area() > 0.0,
        "polygon with hole should have positive area"
    );

    // A point at the center (inside the hole) should NOT be contained.
    assert!(
        !poly.contains_point(&p(0.0, 0.0)),
        "center should be in the hole"
    );

    // A point between the inner and outer loops should be contained.
    assert!(
        poly.contains_point(&p(15.0, 0.0)),
        "point between shell and hole should be contained"
    );
}

// ---------------------------------------------------------------------------
// 6. Polygon contains point
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_contains_point() {
    let poly = text_format::make_polygon("0:0, 0:10, 10:10, 10:0");

    let inside = p(5.0, 5.0);
    assert!(
        poly.contains_point(&inside),
        "polygon should contain interior point"
    );

    let outside = p(50.0, 50.0);
    assert!(
        !poly.contains_point(&outside),
        "polygon should not contain exterior point"
    );
}

// ---------------------------------------------------------------------------
// 7. Polygon nesting
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_nesting() {
    let poly = text_format::make_polygon("-20:-20, -20:20, 20:20, 20:-20; -5:-5, -5:5, 5:5, 5:-5");
    assert_eq!(poly.num_loops(), 2);

    assert_eq!(poly.loop_at(0).depth(), 0, "outer loop should have depth 0");
    assert_eq!(poly.loop_at(1).depth(), 1, "inner loop should have depth 1");
    assert_eq!(poly.parent(0), None, "outer shell should have no parent");
    assert_eq!(
        poly.parent(1),
        Some(0),
        "hole's parent should be the outer shell"
    );
    assert_eq!(
        poly.last_descendant(0),
        1,
        "outer shell's last descendant should be 1"
    );
    assert_eq!(
        poly.last_descendant(1),
        1,
        "hole's last descendant should be itself"
    );
}

// ---------------------------------------------------------------------------
// 8. Polygon shape trait
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_shape_trait() {
    let poly = text_format::make_polygon("0:0, 0:20, 20:20, 20:0; 5:5, 5:15, 15:15, 15:5");

    assert_eq!(poly.dimension(), Dimension::Polygon);

    let expected_edges: usize = (0..poly.num_loops())
        .map(|k| poly.loop_at(k).num_vertices())
        .sum();
    assert_eq!(poly.num_edges(), expected_edges);
    assert_eq!(poly.num_chains(), poly.num_loops());

    for k in 0..poly.num_loops() {
        let chain = poly.chain(k);
        assert_eq!(chain.length, poly.loop_at(k).num_vertices());
    }

    assert!(poly.has_interior(), "polygon should have interior");
}

// ---------------------------------------------------------------------------
// 9. Polygon rect bound
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_rect_bound() {
    // Empty polygon's rect_bound should be empty.
    let empty = Polygon::empty();
    assert!(
        empty.rect_bound().is_empty(),
        "empty polygon's rect_bound should be empty"
    );

    // A normal polygon's rect_bound should contain all vertices.
    let poly = text_format::make_polygon("10:20, 30:40, 50:60");
    let rb = poly.rect_bound();
    assert!(!rb.is_empty());
    for i in 0..poly.loop_at(0).num_vertices() {
        let ll = LatLng::from_point(poly.loop_at(0).vertex(i));
        assert!(rb.contains_lat_lng(ll), "bound should contain vertex {i}");
    }
}

// ---------------------------------------------------------------------------
// 10. Polygon validate
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_validate() {
    let poly = text_format::make_polygon("0:0, 0:10, 10:0");
    assert!(poly.validate().is_ok());

    let poly2 = text_format::make_polygon("-20:-20, -20:20, 20:20, 20:-20; -5:-5, -5:5, 5:5, 5:-5");
    assert!(poly2.validate().is_ok());

    assert!(Polygon::empty().validate().is_ok());
    assert!(Polygon::full().validate().is_ok());
}

// ---------------------------------------------------------------------------
// 11. Polygon contains small cell inside
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_contains_small_cell() {
    let poly = text_format::make_polygon("-30:-30, -30:30, 30:30, 30:-30");
    let inside_id = CellId::from_point(&p(0.0, 0.0)).parent_at_level(20);
    let inside_cell = Cell::from_cell_id(inside_id);
    assert!(
        poly.contains_cell(&inside_cell),
        "polygon should contain small cell deep inside"
    );
}

// ---------------------------------------------------------------------------
// 12. Polygon does not contain cell outside
// ---------------------------------------------------------------------------
#[test]
fn test_polygon_does_not_contain_outside_cell() {
    let poly = text_format::make_polygon("-10:-10, -10:10, 10:10, 10:-10");
    let outside_id = CellId::from_point(&p(80.0, 80.0)).parent_at_level(16);
    let outside_cell = Cell::from_cell_id(outside_id);
    assert!(
        !poly.contains_cell(&outside_cell),
        "polygon should not contain cell far outside"
    );
}
