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
use s2rst::s2::convex_hull_query::ConvexHullQuery;
use s2rst::s2::{LatLng, Loop, Point};

#[inline(never)]
fn make_regular_loop(n: usize) -> Vec<Point> {
    let center = LatLng::from_degrees(40.0, -75.0).to_point();
    let l = Loop::make_regular(center, Angle::from_degrees(5.0), n);
    (0..l.num_vertices()).map(|i| l.vertex(i)).collect()
}

#[library_benchmark]
fn convex_hull_10_points() -> Loop {
    let pts = make_regular_loop(10);
    let mut q = ConvexHullQuery::new();
    q.add_points(black_box(&pts));
    black_box(q.convex_hull())
}

#[library_benchmark]
fn convex_hull_100_points() -> Loop {
    let pts = make_regular_loop(100);
    let mut q = ConvexHullQuery::new();
    q.add_points(black_box(&pts));
    black_box(q.convex_hull())
}

#[library_benchmark]
fn convex_hull_1000_points() -> Loop {
    let pts = make_regular_loop(1000);
    let mut q = ConvexHullQuery::new();
    q.add_points(black_box(&pts));
    black_box(q.convex_hull())
}

#[library_benchmark]
fn convex_hull_single_loop() -> Loop {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), 64);
    let mut q = ConvexHullQuery::new();
    q.add_loop(&loop_);
    black_box(q.convex_hull())
}

library_benchmark_group!(
    name = convex_hull_benchmarks;
    benchmarks =
        convex_hull_10_points,
        convex_hull_100_points,
        convex_hull_1000_points,
        convex_hull_single_loop
);

main!(library_benchmark_groups = convex_hull_benchmarks);
