// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
//! Wall-clock mirror of `benches/predicates.rs`.
//!
//! Input points and function calls MUST match the iai-callgrind benchmarks
//! 1:1 so numbers are directly comparable.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use s2rst::s2::Point;
use s2rst::s2::predicates;

fn bench_sign(c: &mut Criterion) {
    c.bench_function("sign", |b| {
        b.iter(|| {
            let p1 = black_box(Point::from_coords(-3.0, -1.0, 4.0).normalize());
            let p2 = black_box(Point::from_coords(2.0, -1.0, -3.0).normalize());
            let p3 = black_box(Point::from_coords(1.0, -2.0, 0.0).normalize());
            predicates::sign(p1, p2, p3)
        });
    });
}

fn bench_robust_sign(c: &mut Criterion) {
    c.bench_function("robust_sign", |b| {
        b.iter(|| {
            let p1 = black_box(Point::from_coords(-3.0, -1.0, 4.0).normalize());
            let p2 = black_box(Point::from_coords(2.0, -1.0, -3.0).normalize());
            let p3 = black_box(Point::from_coords(1.0, -2.0, 0.0).normalize());
            predicates::robust_sign(p1, p2, p3)
        });
    });
}

fn bench_sign_near_collinear(c: &mut Criterion) {
    c.bench_function("sign_near_collinear", |b| {
        b.iter(|| {
            let p1 = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let p2 = black_box(Point::from_coords(1.0, 1e-15, 0.0).normalize());
            let p3 = black_box(Point::from_coords(1.0, 0.0, 1e-15).normalize());
            predicates::sign(p1, p2, p3)
        });
    });
}

criterion_group!(
    benches,
    bench_sign,
    bench_robust_sign,
    bench_sign_near_collinear
);
criterion_main!(benches);
