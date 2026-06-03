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
use s2rst::s2::polyline::Polyline;
use s2rst::s2::{LatLng, Point};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_straight_polyline(n: usize) -> Polyline {
    let pts: Vec<Point> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            LatLng::from_degrees(t * 30.0, t * 30.0).to_point()
        })
        .collect();
    Polyline::new(pts)
}

#[inline(never)]
fn make_zigzag_polyline(n: usize) -> Polyline {
    let pts: Vec<Point> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let lat = 30.0 * t;
            let lng = if i % 2 == 0 { 0.0 } else { 1.0 };
            LatLng::from_degrees(lat, lng).to_point()
        })
        .collect();
    Polyline::new(pts)
}

// ─── Construction ───────────────────────────────────────────────────────

#[library_benchmark]
fn construct_100() -> Polyline {
    black_box(make_straight_polyline(100))
}

#[library_benchmark]
fn construct_1000() -> Polyline {
    black_box(make_straight_polyline(1000))
}

// ─── Length ─────────────────────────────────────────────────────────────

#[library_benchmark]
fn length_100() -> Angle {
    let pl = make_straight_polyline(100);
    black_box(black_box(pl).length())
}

#[library_benchmark]
fn length_1000() -> Angle {
    let pl = make_straight_polyline(1000);
    black_box(black_box(pl).length())
}

// ─── Centroid ───────────────────────────────────────────────────────────

#[library_benchmark]
fn centroid_100() -> Point {
    let pl = make_straight_polyline(100);
    black_box(black_box(pl).centroid())
}

#[library_benchmark]
fn centroid_1000() -> Point {
    let pl = make_straight_polyline(1000);
    black_box(black_box(pl).centroid())
}

// ─── Project ────────────────────────────────────────────────────────────

#[library_benchmark]
fn project_100() -> (Point, usize) {
    let pl = make_zigzag_polyline(100);
    let p = black_box(LatLng::from_degrees(15.0, 0.5).to_point());
    black_box(black_box(pl).project(p))
}

#[library_benchmark]
fn project_1000() -> (Point, usize) {
    let pl = make_zigzag_polyline(1000);
    let p = black_box(LatLng::from_degrees(15.0, 0.5).to_point());
    black_box(black_box(pl).project(p))
}

// ─── Interpolate ────────────────────────────────────────────────────────

#[library_benchmark]
fn interpolate_100() -> (Point, usize) {
    let pl = make_straight_polyline(100);
    black_box(black_box(pl).interpolate(black_box(0.5)))
}

#[library_benchmark]
fn interpolate_1000() -> (Point, usize) {
    let pl = make_straight_polyline(1000);
    black_box(black_box(pl).interpolate(black_box(0.5)))
}

// ─── Validate ───────────────────────────────────────────────────────────

#[library_benchmark]
fn validate_100() -> Result<(), String> {
    let pl = make_straight_polyline(100);
    black_box(black_box(pl).validate())
}

#[library_benchmark]
fn validate_1000() -> Result<(), String> {
    let pl = make_straight_polyline(1000);
    black_box(black_box(pl).validate())
}

// ─── ApproxEq ───────────────────────────────────────────────────────────

#[library_benchmark]
fn approx_eq_100() -> bool {
    let a = make_straight_polyline(100);
    let b = make_straight_polyline(100);
    black_box(black_box(a).approx_eq_with(black_box(&b), Angle::from_degrees(1e-10)))
}

library_benchmark_group!(
    name = construction;
    benchmarks = construct_100, construct_1000
);

library_benchmark_group!(
    name = length_centroid;
    benchmarks = length_100, length_1000, centroid_100, centroid_1000
);

library_benchmark_group!(
    name = project_interpolate;
    benchmarks = project_100, project_1000, interpolate_100, interpolate_1000
);

library_benchmark_group!(
    name = validation;
    benchmarks = validate_100, validate_1000, approx_eq_100
);

main!(
    library_benchmark_groups = construction,
    length_centroid,
    project_interpolate,
    validation
);
