// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1;
use s2rst::s2::Point;

#[library_benchmark]
fn bench_point_distance() -> s1::Angle {
    let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let p2 = black_box(Point::from_coords(0.0, 1.0, 0.0));
    black_box(p1.distance(p2))
}

#[library_benchmark]
fn bench_point_cross() -> Point {
    let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let p2 = black_box(Point::from_coords(0.0, 1.0, 0.0));
    black_box(p1.point_cross(p2))
}

#[library_benchmark]
fn bench_point_normalize() -> Point {
    let p = black_box(Point::from_coords(3.0, 4.0, 5.0));
    black_box(p.normalize())
}

#[library_benchmark]
fn bench_angle_from_e6() -> f64 {
    let val = black_box(47_600_000_i32);
    black_box(s1::Angle::from_e6(val).radians())
}

#[library_benchmark]
fn bench_angle_to_e6() -> i32 {
    let angle = black_box(s1::Angle::from_radians(0.83));
    black_box(angle.e6())
}

#[library_benchmark]
fn bench_angle_from_degrees() -> s1::Angle {
    let deg = black_box(45.0_f64);
    black_box(s1::Angle::from_degrees(deg))
}

#[library_benchmark]
fn bench_angle_from_radians() -> s1::Angle {
    let rad = black_box(1.5_f64);
    black_box(s1::Angle::from_radians(rad))
}

#[library_benchmark]
fn bench_chord_angle_from_angle() -> s1::ChordAngle {
    let angle = black_box(s1::Angle::from_degrees(60.0));
    black_box(s1::ChordAngle::from_angle(angle))
}

library_benchmark_group!(
    name = point_angle_benchmarks;
    benchmarks =
        bench_point_distance,
        bench_point_cross,
        bench_point_normalize,
        bench_angle_from_e6,
        bench_angle_to_e6,
        bench_angle_from_degrees,
        bench_angle_from_radians,
        bench_chord_angle_from_angle
);

main!(library_benchmark_groups = point_angle_benchmarks);
