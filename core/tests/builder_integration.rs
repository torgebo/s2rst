// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for `S2Builder`, ported from C++ `s2builder_test.cc`.

use s2rst::s1::Angle;
use s2rst::s2::builder::point_vector_layer::S2PointVectorLayer;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::builder::polyline_vector_layer::S2PolylineVectorLayer;
use s2rst::s2::builder::snap::{IdentitySnapFunction, IntLatLngSnapFunction, S2CellIdSnapFunction};
use s2rst::s2::builder::{Options, S2Builder};
use s2rst::s2::text_format;
use s2rst::s2::{LatLng, Point, Polygon};

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

/// Build with a single `S2PolygonLayer` and extract the polygon.
fn build_polygon(builder: &mut S2Builder) -> Polygon {
    let mut layers = builder.build().expect("build failed");
    layers
        .remove(0)
        .into_any()
        .downcast::<S2PolygonLayer>()
        .expect("wrong layer type")
        .into_output()
}

// ---------------------------------------------------------------------------
// 1. Polygon passthrough with identity snap
// ---------------------------------------------------------------------------
#[test]
fn test_builder_polygon_passthrough() {
    let input = text_format::make_polygon("0:0, 0:10, 10:10, 10:0");
    let input_area = input.area();

    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&input);

    let result = build_polygon(&mut builder);
    assert_eq!(result.num_loops(), 1);
    assert!(
        approx_eq(result.area(), input_area, 1e-12),
        "output area should match input area"
    );
}

// ---------------------------------------------------------------------------
// 2. Loop passthrough
// ---------------------------------------------------------------------------
#[test]
fn test_builder_loop_passthrough() {
    let input = text_format::make_loop("0:0, 0:10, 10:0");

    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_loop(&input);

    let result = build_polygon(&mut builder);
    assert_eq!(result.num_loops(), 1);
    assert_eq!(result.loop_at(0).num_vertices(), 3);
}

// ---------------------------------------------------------------------------
// 3. Polyline output
// ---------------------------------------------------------------------------
#[test]
fn test_builder_polyline() {
    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PolylineVectorLayer::new()));
    builder.add_edge(p(0.0, 0.0), p(10.0, 0.0));
    builder.add_edge(p(10.0, 0.0), p(20.0, 0.0));

    let mut layers = builder.build().expect("build failed");
    let layer = layers
        .remove(0)
        .into_any()
        .downcast::<S2PolylineVectorLayer>()
        .expect("wrong type");
    let result = layer.into_output();
    assert_eq!(result.len(), 1, "should produce one polyline");
    assert_eq!(
        result[0].num_vertices(),
        3,
        "polyline should have 3 vertices"
    );
}

// ---------------------------------------------------------------------------
// 4. Point output
// ---------------------------------------------------------------------------
#[test]
fn test_builder_points() {
    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PointVectorLayer::new()));
    builder.add_point(p(10.0, 20.0));
    builder.add_point(p(30.0, 40.0));
    builder.add_point(p(50.0, 60.0));

    let mut layers = builder.build().expect("build failed");
    let layer = layers
        .remove(0)
        .into_any()
        .downcast::<S2PointVectorLayer>()
        .expect("wrong type");
    let result = layer.into_output();
    assert_eq!(result.len(), 3, "should produce three points");
}

// ---------------------------------------------------------------------------
// 5. Identity snap preserves vertices
// ---------------------------------------------------------------------------
#[test]
fn test_builder_identity_snap_preserves_vertices() {
    let input = text_format::make_polygon("1.5:2.5, 3.5:4.5, 5.5:6.5");

    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&input);

    let result = build_polygon(&mut builder);
    assert_eq!(result.loop_at(0).num_vertices(), 3);
    // With identity snap and zero radius, vertices should be preserved exactly.
    for i in 0..3 {
        let dist = (result.loop_at(0).vertex(i).0 - input.loop_at(0).vertex(i).0).norm();
        assert!(
            dist < 1e-14,
            "vertex {i} should be preserved, dist = {dist}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. CellId snap modifies vertices
// ---------------------------------------------------------------------------
#[test]
fn test_builder_cellid_snap() {
    let input = text_format::make_polygon("0:0, 5:10, 10:0");

    let mut builder = S2Builder::new(Options::new(Box::new(S2CellIdSnapFunction::new(10))));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&input);

    let result = build_polygon(&mut builder);
    assert!(result.num_loops() >= 1, "should have at least 1 loop");
    let mut any_moved = false;
    for i in 0..result.loop_at(0).num_vertices() {
        let dist = (result.loop_at(0).vertex(i).0 - input.loop_at(0).vertex(i).0).norm();
        if dist > 1e-10 {
            any_moved = true;
        }
    }
    assert!(any_moved, "CellId snap should move at least one vertex");
}

// ---------------------------------------------------------------------------
// 7. IntLatLng snap
// ---------------------------------------------------------------------------
#[test]
fn test_builder_intlatlng_snap() {
    let input = text_format::make_polygon("0.5:0.5, 5.5:10.5, 10.5:0.5");

    let mut builder = S2Builder::new(Options::new(Box::new(
        IntLatLngSnapFunction::new(0), // exponent 0 = 1-degree grid
    )));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&input);

    let result = build_polygon(&mut builder);
    assert!(result.num_loops() >= 1, "should have at least 1 loop");
    for i in 0..result.loop_at(0).num_vertices() {
        let ll = LatLng::from_point(result.loop_at(0).vertex(i));
        let lat_deg = ll.lat.degrees();
        let lng_deg = ll.lng.degrees();
        assert!(
            (lat_deg - lat_deg.round()).abs() < 0.01,
            "lat {lat_deg} should be near an integer"
        );
        assert!(
            (lng_deg - lng_deg.round()).abs() < 0.01,
            "lng {lng_deg} should be near an integer"
        );
    }
}

// ---------------------------------------------------------------------------
// 8. Force vertex
// ---------------------------------------------------------------------------
#[test]
fn test_builder_force_vertex() {
    let forced = p(1.0, 1.0);

    let mut options = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_degrees(
        1.0,
    ))));
    options.idempotent = false;
    let mut builder = S2Builder::new(options);
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.force_vertex(forced);
    builder.add_edge(p(1.001, 1.001), p(0.0, 10.0));
    builder.add_edge(p(0.0, 10.0), p(10.0, 0.0));
    builder.add_edge(p(10.0, 0.0), p(1.001, 1.001));

    let result = build_polygon(&mut builder);
    assert!(result.num_loops() >= 1, "should have at least 1 loop");
    let mut found = false;
    for i in 0..result.loop_at(0).num_vertices() {
        let dist = (result.loop_at(0).vertex(i).0 - forced.0).norm();
        if dist < 1e-10 {
            found = true;
            break;
        }
    }
    assert!(found, "forced vertex should appear in output");
}

// ---------------------------------------------------------------------------
// 9. Idempotent: build→rebuild produces same output
// ---------------------------------------------------------------------------
#[test]
fn test_builder_idempotent() {
    let input = text_format::make_polygon("0:0, 0:10, 10:10, 10:0");

    // First build.
    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&input);
    let r1 = build_polygon(&mut builder);

    // Second build from first output.
    let mut builder2 = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));
    builder2.start_layer(Box::new(S2PolygonLayer::new()));
    builder2.add_polygon(&r1);
    let r2 = build_polygon(&mut builder2);

    assert_eq!(r1.num_loops(), r2.num_loops());
    assert!(
        approx_eq(r1.area(), r2.area(), 1e-14),
        "idempotent builds should produce same area"
    );
}

// ---------------------------------------------------------------------------
// 10. Builder reset
// ---------------------------------------------------------------------------
#[test]
fn test_builder_reset() {
    let mut builder = S2Builder::new(Options::new(Box::new(IdentitySnapFunction::new(
        Angle::from_radians(0.0),
    ))));

    // First build: triangle.
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&text_format::make_polygon("0:0, 0:10, 10:0"));
    let r1 = build_polygon(&mut builder);

    // Reset and build a different polygon.
    builder.reset();
    builder.start_layer(Box::new(S2PolygonLayer::new()));
    builder.add_polygon(&text_format::make_polygon("20:20, 20:30, 30:30, 30:20"));
    let r2 = build_polygon(&mut builder);

    assert_eq!(r1.num_loops(), 1);
    assert_eq!(r2.num_loops(), 1);
    assert_eq!(r1.loop_at(0).num_vertices(), 3);
    assert_eq!(r2.loop_at(0).num_vertices(), 4);
}
