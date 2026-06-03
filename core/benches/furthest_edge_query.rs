// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::furthest_edge_query::{FurthestEdgeQuery, PointTarget, Result};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Loop, Polygon};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_zigzag_index(n: usize) -> ShapeIndex {
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let lat = 45.0 + 10.0 * t;
        let lng = -120.0 + if i % 2 == 0 { 0.0 } else { 0.5 };
        points.push(LatLng::from_degrees(lat, lng).to_point());
    }
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(points)));
    idx.build();
    idx
}

#[inline(never)]
fn make_polygon_index(n: usize) -> ShapeIndex {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let lat = 10.0 * angle.cos();
        let lng = 10.0 * angle.sin();
        pts.push(LatLng::from_degrees(lat, lng).to_point());
    }
    let loop_ = Loop::new(pts);
    let polygon = Polygon::from_loops(vec![loop_]);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
    idx
}

// ─── Furthest edge from point, polyline index ──────────────────────────

#[library_benchmark]
fn furthest_edge_polyline_100() -> Result {
    let idx = black_box(make_zigzag_index(100));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(-45.0, 60.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

#[library_benchmark]
fn furthest_edge_polyline_1000() -> Result {
    let idx = black_box(make_zigzag_index(1000));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(-45.0, 60.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

#[library_benchmark]
fn furthest_edge_polyline_10000() -> Result {
    let idx = black_box(make_zigzag_index(10000));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(-45.0, 60.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

// ─── Furthest edge from point, polygon index ───────────────────────────

#[library_benchmark]
fn furthest_edge_polygon_16() -> Result {
    let idx = black_box(make_polygon_index(16));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

#[library_benchmark]
fn furthest_edge_polygon_256() -> Result {
    let idx = black_box(make_polygon_index(256));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

#[library_benchmark]
fn furthest_edge_polygon_1024() -> Result {
    let idx = black_box(make_polygon_index(1024));
    let query = FurthestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

// ─── Antipodal target ──────────────────────────────────────────────────

#[library_benchmark]
fn furthest_edge_antipodal_1000() -> Result {
    let idx = black_box(make_zigzag_index(1000));
    let query = FurthestEdgeQuery::new(&idx);
    // Antipodal to center of the zigzag.
    let target = PointTarget::new(black_box(LatLng::from_degrees(-50.0, 60.0).to_point()));
    black_box(query.find_furthest_edge(&target))
}

library_benchmark_group!(
    name = furthest_polyline;
    benchmarks =
        furthest_edge_polyline_100,
        furthest_edge_polyline_1000,
        furthest_edge_polyline_10000
);

library_benchmark_group!(
    name = furthest_polygon;
    benchmarks =
        furthest_edge_polygon_16,
        furthest_edge_polygon_256,
        furthest_edge_polygon_1024
);

library_benchmark_group!(
    name = furthest_antipodal;
    benchmarks = furthest_edge_antipodal_1000
);

main!(
    library_benchmark_groups = furthest_polyline,
    furthest_polygon,
    furthest_antipodal
);
