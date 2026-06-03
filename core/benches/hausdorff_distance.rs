// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::hausdorff_distance_query::{
    DirectedResult, HausdorffResult, S2HausdorffDistanceQuery,
};
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Loop, Polygon};

#[inline(never)]
fn make_polyline_index(n: usize, center_lat: f64) -> ShapeIndex {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f64 / n as f64;
        pts.push(LatLng::from_degrees(center_lat + t * 2.0, t * 2.0).to_point());
    }
    let mut index = ShapeIndex::new();
    index.add(Box::new(LaxPolyline::new(pts)));
    index.build();
    index
}

#[inline(never)]
fn make_polygon_index(n: usize) -> ShapeIndex {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let polygon = Polygon::from_loops(vec![Loop::make_regular(
        center,
        Angle::from_degrees(1.0),
        n,
    )]);
    let lax = s2rst::s2::lax_polygon::LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();
    index
}

#[library_benchmark]
fn hausdorff_polyline_100() -> Option<HausdorffResult> {
    let a = make_polyline_index(100, 0.0);
    let b = make_polyline_index(100, 0.1);
    let query = S2HausdorffDistanceQuery::new();
    black_box(query.get_result(&a, &b))
}

#[library_benchmark]
fn hausdorff_polyline_1000() -> Option<HausdorffResult> {
    let a = make_polyline_index(1000, 0.0);
    let b = make_polyline_index(1000, 0.1);
    let query = S2HausdorffDistanceQuery::new();
    black_box(query.get_result(&a, &b))
}

#[library_benchmark]
fn hausdorff_polygon_64() -> Option<HausdorffResult> {
    let a = make_polygon_index(64);
    let b = make_polygon_index(64);
    let query = S2HausdorffDistanceQuery::new();
    black_box(query.get_result(&a, &b))
}

#[library_benchmark]
fn hausdorff_directed_polyline_100() -> Option<DirectedResult> {
    let a = make_polyline_index(100, 0.0);
    let b = make_polyline_index(100, 0.1);
    let query = S2HausdorffDistanceQuery::new();
    black_box(query.get_directed_result(&a, &b))
}

library_benchmark_group!(
    name = hausdorff_benchmarks;
    benchmarks =
        hausdorff_polyline_100,
        hausdorff_polyline_1000,
        hausdorff_polygon_64,
        hausdorff_directed_polyline_100
);

main!(library_benchmark_groups = hausdorff_benchmarks);
