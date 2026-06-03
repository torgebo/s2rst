// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
//! Wall-clock mirror of `benches/polyline.rs` length and centroid groups.
//!
//! Phase 2 (ILP in reductions) needs cycle-level numbers on the long-tail
//! sums; iai-callgrind sees the per-instruction count but cannot show the
//! benefit of breaking the accumulator dependency chain. Inputs match the
//! iai bench 1:1 so a `--bench polyline` and `--bench wall_polyline`
//! describe the same workload.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::polyline_measures;
use s2rst::s2::{LatLng, Point};

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
fn make_straight_points(n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            LatLng::from_degrees(t * 30.0, t * 30.0).to_point()
        })
        .collect()
}

fn bench_length_100(c: &mut Criterion) {
    let pl = make_straight_polyline(100);
    c.bench_function("length_100", |b| b.iter(|| black_box(&pl).length()));
}

fn bench_length_1000(c: &mut Criterion) {
    let pl = make_straight_polyline(1000);
    c.bench_function("length_1000", |b| b.iter(|| black_box(&pl).length()));
}

fn bench_centroid_100(c: &mut Criterion) {
    let pl = make_straight_polyline(100);
    c.bench_function("centroid_100", |b| b.iter(|| black_box(&pl).centroid()));
}

fn bench_centroid_1000(c: &mut Criterion) {
    let pl = make_straight_polyline(1000);
    c.bench_function("centroid_1000", |b| b.iter(|| black_box(&pl).centroid()));
}

fn bench_get_length_1000(c: &mut Criterion) {
    let pts = make_straight_points(1000);
    c.bench_function("get_length_1000", |b| {
        b.iter(|| polyline_measures::get_length(black_box(&pts)));
    });
}

fn bench_get_centroid_1000(c: &mut Criterion) {
    let pts = make_straight_points(1000);
    c.bench_function("get_centroid_1000", |b| {
        b.iter(|| polyline_measures::get_centroid(black_box(&pts)));
    });
}

criterion_group!(
    benches,
    bench_length_100,
    bench_length_1000,
    bench_centroid_100,
    bench_centroid_1000,
    bench_get_length_1000,
    bench_get_centroid_1000,
);
criterion_main!(benches);
