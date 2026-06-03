// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::{LatLng, Loop, Polygon, Region};

#[library_benchmark]
fn bench_loop_contains_point() -> bool {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_contains_point_outside() -> bool {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    let p = LatLng::from_degrees(50.0, 50.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_area() -> f64 {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    black_box(black_box(loop_).area())
}

#[library_benchmark]
fn bench_loop_construction() -> Loop {
    let v0 = black_box(LatLng::from_degrees(0.0, 0.0).to_point());
    let v1 = black_box(LatLng::from_degrees(0.0, 10.0).to_point());
    let v2 = black_box(LatLng::from_degrees(10.0, 10.0).to_point());
    let v3 = black_box(LatLng::from_degrees(10.0, 0.0).to_point());
    black_box(Loop::new(vec![v0, v1, v2, v3]))
}

#[library_benchmark]
fn bench_polygon_area() -> f64 {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    let polygon = Polygon::from_loops(vec![loop_]);
    black_box(black_box(polygon).area())
}

#[library_benchmark]
fn bench_polygon_from_loops() -> Polygon {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    black_box(Polygon::from_loops(vec![black_box(loop_)]))
}

// ─── Go BenchmarkLoopContainsPoint: scaling with vertex count ───────────
// Port of Go pattern: test ContainsPoint on regular loops with varying
// vertex counts (4, 16, 64, 256, 1024).

#[inline(never)]
fn make_regular_loop(n: usize, radius_deg: f64) -> Loop {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let lat = radius_deg * angle.cos();
        let lng = radius_deg * angle.sin();
        pts.push(LatLng::from_degrees(lat, lng).to_point());
    }
    Loop::new(pts)
}

#[library_benchmark]
fn bench_loop_contains_point_4v() -> bool {
    let loop_ = make_regular_loop(4, 5.0);
    let p = LatLng::from_degrees(2.0, 2.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_contains_point_16v() -> bool {
    let loop_ = make_regular_loop(16, 5.0);
    let p = LatLng::from_degrees(2.0, 2.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_contains_point_64v() -> bool {
    let loop_ = make_regular_loop(64, 5.0);
    let p = LatLng::from_degrees(2.0, 2.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_contains_point_256v() -> bool {
    let loop_ = make_regular_loop(256, 5.0);
    let p = LatLng::from_degrees(2.0, 2.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_loop_contains_point_1024v() -> bool {
    let loop_ = make_regular_loop(1024, 5.0);
    let p = LatLng::from_degrees(2.0, 2.0).to_point();
    black_box(black_box(loop_).brute_force_contains_point(black_box(p)))
}

// ─── Go polygon benchmarks: area and containment with varying complexity ─

#[library_benchmark]
fn bench_polygon_contains_point() -> bool {
    let polygon = Polygon::from_loops(vec![make_regular_loop(64, 10.0)]);
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    black_box(black_box(polygon).contains_point(&black_box(p)))
}

#[library_benchmark]
fn bench_polygon_contains_point_256v() -> bool {
    let polygon = Polygon::from_loops(vec![make_regular_loop(256, 10.0)]);
    let p = LatLng::from_degrees(5.0, 5.0).to_point();
    black_box(black_box(polygon).contains_point(&black_box(p)))
}

#[library_benchmark]
fn bench_polygon_area_64v() -> f64 {
    let polygon = Polygon::from_loops(vec![make_regular_loop(64, 10.0)]);
    black_box(black_box(polygon).area())
}

#[library_benchmark]
fn bench_polygon_area_256v() -> f64 {
    let polygon = Polygon::from_loops(vec![make_regular_loop(256, 10.0)]);
    black_box(black_box(polygon).area())
}

#[library_benchmark]
fn bench_polygon_from_loops_64v() -> Polygon {
    let loop_ = make_regular_loop(64, 10.0);
    black_box(Polygon::from_loops(vec![black_box(loop_)]))
}

library_benchmark_group!(
    name = loop_polygon_benchmarks;
    benchmarks =
        bench_loop_contains_point,
        bench_loop_contains_point_outside,
        bench_loop_area,
        bench_loop_construction,
        bench_polygon_area,
        bench_polygon_from_loops
);

library_benchmark_group!(
    name = loop_contains_point_scaling;
    benchmarks =
        bench_loop_contains_point_4v,
        bench_loop_contains_point_16v,
        bench_loop_contains_point_64v,
        bench_loop_contains_point_256v,
        bench_loop_contains_point_1024v
);

library_benchmark_group!(
    name = polygon_benchmarks;
    benchmarks =
        bench_polygon_contains_point,
        bench_polygon_contains_point_256v,
        bench_polygon_area_64v,
        bench_polygon_area_256v,
        bench_polygon_from_loops_64v
);

main!(
    library_benchmark_groups = loop_polygon_benchmarks,
    loop_contains_point_scaling,
    polygon_benchmarks
);
