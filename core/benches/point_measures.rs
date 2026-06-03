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
use s2rst::s2::Point;
use s2rst::s2::point_measures;

fn p(x: f64, y: f64, z: f64) -> Point {
    Point::from_coords(x, y, z)
}

// Go: BenchmarkPointArea
#[library_benchmark]
fn point_area_right_angle() -> f64 {
    black_box(point_measures::point_area(
        black_box(p(1.0, 0.0, 0.0)),
        black_box(p(0.0, 1.0, 0.0)),
        black_box(p(0.0, 0.0, 1.0)),
    ))
}

// Go: BenchmarkPointAreaGirardCase
#[library_benchmark]
fn point_area_girard_case() -> f64 {
    let g1 = Point::from_coords(1.0, 1.0, 0.0).normalize();
    let g2 = Point::from_coords(1.0, -1.0, 1e-30).normalize();
    let g3 = Point::from_coords(-1.0, 0.0, 1e-30).normalize();
    black_box(point_measures::point_area(
        black_box(g1),
        black_box(g2),
        black_box(g3),
    ))
}

#[library_benchmark]
fn point_area_small_triangle() -> f64 {
    let eps = 1e-10;
    let a = Point::from_coords(eps, 0.0, 1.0).normalize();
    let b = Point::from_coords(0.0, eps, 1.0).normalize();
    let c = p(0.0, 0.0, 1.0);
    black_box(point_measures::point_area(
        black_box(a),
        black_box(b),
        black_box(c),
    ))
}

#[library_benchmark]
fn girard_area_right_angle() -> f64 {
    black_box(point_measures::girard_area(
        black_box(p(1.0, 0.0, 0.0)),
        black_box(p(0.0, 1.0, 0.0)),
        black_box(p(0.0, 0.0, 1.0)),
    ))
}

#[library_benchmark]
fn angle_right_angle() -> Angle {
    black_box(point_measures::angle(
        black_box(p(1.0, 0.0, 0.0)),
        black_box(p(0.0, 0.0, 1.0)),
        black_box(Point::from_coords(1.0, 1.0, 0.0).normalize()),
    ))
}

#[library_benchmark]
fn turn_angle() -> Angle {
    black_box(point_measures::turn_angle(
        black_box(p(1.0, 0.0, 0.0)),
        black_box(p(0.0, 0.0, 1.0)),
        black_box(Point::from_coords(1.0, 1.0, 0.0).normalize()),
    ))
}

library_benchmark_group!(
    name = point_measures_benchmarks;
    benchmarks =
        point_area_right_angle,
        point_area_girard_case,
        point_area_small_triangle,
        girard_area_right_angle,
        angle_right_angle,
        turn_angle
);

main!(library_benchmark_groups = point_measures_benchmarks);
