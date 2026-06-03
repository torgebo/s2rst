// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
//! Wall-clock mirror of `benches/point_angle.rs`.
//!
//! Input points and function calls MUST match the iai-callgrind benchmarks
//! 1:1 so numbers are directly comparable.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use s2rst::s1;
use s2rst::s2::Point;

fn bench_point_distance(c: &mut Criterion) {
    c.bench_function("point_distance", |b| {
        b.iter(|| {
            let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let p2 = black_box(Point::from_coords(0.0, 1.0, 0.0));
            p1.distance(p2)
        });
    });
}

fn bench_point_cross(c: &mut Criterion) {
    c.bench_function("point_cross", |b| {
        b.iter(|| {
            let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let p2 = black_box(Point::from_coords(0.0, 1.0, 0.0));
            p1.point_cross(p2)
        });
    });
}

fn bench_point_normalize(c: &mut Criterion) {
    c.bench_function("point_normalize", |b| {
        b.iter(|| {
            let p = black_box(Point::from_coords(3.0, 4.0, 5.0));
            p.normalize()
        });
    });
}

fn bench_angle_from_e6(c: &mut Criterion) {
    c.bench_function("angle_from_e6", |b| {
        b.iter(|| {
            let val = black_box(47_600_000_i32);
            s1::Angle::from_e6(val).radians()
        });
    });
}

fn bench_angle_to_e6(c: &mut Criterion) {
    c.bench_function("angle_to_e6", |b| {
        b.iter(|| {
            let angle = black_box(s1::Angle::from_radians(0.83));
            angle.e6()
        });
    });
}

fn bench_angle_from_degrees(c: &mut Criterion) {
    c.bench_function("angle_from_degrees", |b| {
        b.iter(|| {
            let deg = black_box(45.0_f64);
            s1::Angle::from_degrees(deg)
        });
    });
}

fn bench_angle_from_radians(c: &mut Criterion) {
    c.bench_function("angle_from_radians", |b| {
        b.iter(|| {
            let rad = black_box(1.5_f64);
            s1::Angle::from_radians(rad)
        });
    });
}

fn bench_chord_angle_from_angle(c: &mut Criterion) {
    c.bench_function("chord_angle_from_angle", |b| {
        b.iter(|| {
            let angle = black_box(s1::Angle::from_degrees(60.0));
            s1::ChordAngle::from_angle(angle)
        });
    });
}

criterion_group!(
    benches,
    bench_point_distance,
    bench_point_cross,
    bench_point_normalize,
    bench_angle_from_e6,
    bench_angle_to_e6,
    bench_angle_from_degrees,
    bench_angle_from_radians,
    bench_chord_angle_from_angle
);
criterion_main!(benches);
