// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use s2rst::s1::Angle;
use s2rst::s2::builder::lax_polygon_layer::LaxPolygonLayer;
use s2rst::s2::builder::polyline_vector_layer::S2PolylineVectorLayer;
use s2rst::s2::builder::snap::IdentitySnapFunction;
use s2rst::s2::builder::{Options, S2Builder};
use s2rst::s2::{LatLng, Point};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_square_loop(size_deg: f64) -> Vec<Point> {
    let s = size_deg;
    vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, s).to_point(),
        LatLng::from_degrees(s, s).to_point(),
        LatLng::from_degrees(s, 0.0).to_point(),
    ]
}

#[inline(never)]
fn make_regular_points(n: usize, radius_deg: f64) -> Vec<Point> {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let lat = radius_deg * angle.cos();
        let lng = radius_deg * angle.sin();
        pts.push(LatLng::from_degrees(lat, lng).to_point());
    }
    pts
}

// ─── Builder with polygon layer ────────────────────────────────────────

#[library_benchmark]
fn build_polygon_4_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    builder.add_loop_from_points(&make_square_loop(5.0));
    let _result = builder.build();
}

#[library_benchmark]
fn build_polygon_64_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    builder.add_loop_from_points(&make_regular_points(64, 10.0));
    let _result = builder.build();
}

#[library_benchmark]
fn build_polygon_256_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    builder.add_loop_from_points(&make_regular_points(256, 10.0));
    let _result = builder.build();
}

#[library_benchmark]
fn build_polygon_1024_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    builder.add_loop_from_points(&make_regular_points(1024, 10.0));
    let _result = builder.build();
}

// ─── Builder with polyline layer ───────────────────────────────────────

#[library_benchmark]
fn build_polyline_100_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = S2PolylineVectorLayer::new();
    builder.start_layer(Box::new(layer));
    let pts = make_regular_points(100, 10.0);
    builder.add_polyline_from_points(&pts);
    let _result = builder.build();
}

#[library_benchmark]
fn build_polyline_1000_vertices() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = S2PolylineVectorLayer::new();
    builder.start_layer(Box::new(layer));
    let pts = make_regular_points(1000, 10.0);
    builder.add_polyline_from_points(&pts);
    let _result = builder.build();
}

// ─── Builder add_edge throughput ───────────────────────────────────────

#[library_benchmark]
fn add_100_edges() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    let pts = make_regular_points(100, 10.0);
    for i in 0..pts.len() {
        builder.add_edge(pts[i], pts[(i + 1) % pts.len()]);
    }
    let _result = builder.build();
}

#[library_benchmark]
fn add_1000_edges() {
    let opts = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_radians(
        0.0,
    ))));
    let mut builder = S2Builder::new(opts);
    let layer = LaxPolygonLayer::new();
    builder.start_layer(Box::new(layer));
    let pts = make_regular_points(1000, 10.0);
    for i in 0..pts.len() {
        builder.add_edge(pts[i], pts[(i + 1) % pts.len()]);
    }
    let _result = builder.build();
}

library_benchmark_group!(
    name = build_polygon;
    benchmarks =
        build_polygon_4_vertices,
        build_polygon_64_vertices,
        build_polygon_256_vertices,
        build_polygon_1024_vertices
);

library_benchmark_group!(
    name = build_polyline;
    benchmarks =
        build_polyline_100_vertices,
        build_polyline_1000_vertices
);

library_benchmark_group!(
    name = add_edges;
    benchmarks =
        add_100_edges,
        add_1000_edges
);

main!(
    library_benchmark_groups = build_polygon,
    build_polyline,
    add_edges
);
