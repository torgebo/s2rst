// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::Point;
use s2rst::s2::predicates;

#[library_benchmark]
fn bench_sign() -> bool {
    // Same test points as Go BenchmarkSign.
    let p1 = black_box(Point::from_coords(-3.0, -1.0, 4.0).normalize());
    let p2 = black_box(Point::from_coords(2.0, -1.0, -3.0).normalize());
    let p3 = black_box(Point::from_coords(1.0, -2.0, 0.0).normalize());
    black_box(predicates::sign(p1, p2, p3))
}

#[library_benchmark]
fn bench_robust_sign() -> predicates::Direction {
    let p1 = black_box(Point::from_coords(-3.0, -1.0, 4.0).normalize());
    let p2 = black_box(Point::from_coords(2.0, -1.0, -3.0).normalize());
    let p3 = black_box(Point::from_coords(1.0, -2.0, 0.0).normalize());
    black_box(predicates::robust_sign(p1, p2, p3))
}

#[library_benchmark]
fn bench_sign_near_collinear() -> bool {
    // Near-collinear points that force the expensive exact arithmetic path.
    let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let p2 = black_box(Point::from_coords(1.0, 1e-15, 0.0).normalize());
    let p3 = black_box(Point::from_coords(1.0, 0.0, 1e-15).normalize());
    black_box(predicates::sign(p1, p2, p3))
}

library_benchmark_group!(
    name = predicates_benchmarks;
    benchmarks =
        bench_sign,
        bench_robust_sign,
        bench_sign_near_collinear
);

main!(library_benchmark_groups = predicates_benchmarks);
