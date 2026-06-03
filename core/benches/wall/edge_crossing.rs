// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
//! Wall-clock mirror of `benches/edge_crossing.rs`.
//!
//! Input points and function calls MUST match the iai-callgrind benchmarks
//! 1:1 so numbers are directly comparable.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

use s2rst::s2::Point;
use s2rst::s2::edge_crosser::EdgeCrosser;
use s2rst::s2::edge_crossings;

fn bench_crossing_sign(c: &mut Criterion) {
    c.bench_function("crossing_sign", |bencher| {
        bencher.iter(|| {
            let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
            let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
            let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
            edge_crossings::crossing_sign(a, b, c, d)
        });
    });
}

fn bench_crossing_sign_crossing(c: &mut Criterion) {
    c.bench_function("crossing_sign_crossing", |bencher| {
        bencher.iter(|| {
            let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let b = black_box(Point::from_coords(-1.0, 0.0, 0.0));
            let c = black_box(Point::from_coords(0.0, 1.0, 0.0));
            let d = black_box(Point::from_coords(0.0, -1.0, 0.0));
            edge_crossings::crossing_sign(a, b, c, d)
        });
    });
}

fn bench_edge_crosser_chain(c: &mut Criterion) {
    c.bench_function("edge_crosser_chain", |bencher| {
        bencher.iter(|| {
            let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
            let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
            let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
            let mut crosser = EdgeCrosser::new(a, b);
            crosser.restart_at(c);
            crosser.chain_crossing_sign(d)
        });
    });
}

fn bench_edge_or_vertex_crossing(c: &mut Criterion) {
    c.bench_function("edge_or_vertex_crossing", |bencher| {
        bencher.iter(|| {
            let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
            let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
            let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
            edge_crossings::edge_or_vertex_crossing(a, b, c, d)
        });
    });
}

fn bench_intersection(c: &mut Criterion) {
    c.bench_function("intersection", |bencher| {
        bencher.iter(|| {
            let a0 = black_box(Point::from_coords(1.0, 0.0, 0.0));
            let a1 = black_box(Point::from_coords(-1.0, 0.0, 0.0));
            let b0 = black_box(Point::from_coords(0.0, 1.0, 0.0));
            let b1 = black_box(Point::from_coords(0.0, -1.0, 0.0));
            edge_crossings::intersection(a0, a1, b0, b1)
        });
    });
}

criterion_group!(
    benches,
    bench_crossing_sign,
    bench_crossing_sign_crossing,
    bench_edge_crosser_chain,
    bench_edge_or_vertex_crossing,
    bench_intersection
);
criterion_main!(benches);
