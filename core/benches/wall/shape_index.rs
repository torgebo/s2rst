// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
//! Wall-clock mirror of `benches/shape_index.rs` build benches.
//!
//! Phase 7 (rayon per-face parallelism) needs cycle-level numbers; iai-
//! callgrind sees instruction count but cannot show wall-clock parallel
//! speedup. Inputs match `benches/shape_index.rs` 1:1 — `build_polyline_*`
//! and `build_10000_polyline_shapes` are the parallel-eligible polyline-only
//! workloads.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Point};

// Mirrors `make_zigzag_points` from `benches/shape_index.rs` so the wall
// numbers compare 1:1 with the iai bench.
#[inline(never)]
fn make_zigzag_points(n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let lat = 45.0 + 10.0 * t;
            let lng = -120.0 + if i % 2 == 0 { 0.0 } else { 0.5 };
            LatLng::from_degrees(lat, lng).to_point()
        })
        .collect()
}

fn bench_build_polyline_1000(c: &mut Criterion) {
    c.bench_function("build_polyline_1000", |b| {
        let pts = make_zigzag_points(1001);
        b.iter(|| {
            let mut idx = ShapeIndex::new();
            idx.add(Box::new(Polyline::new(pts.clone())));
            black_box(&mut idx).build();
            idx
        });
    });
}

fn bench_build_polyline_10000(c: &mut Criterion) {
    c.bench_function("build_polyline_10000", |b| {
        let pts = make_zigzag_points(10001);
        b.iter(|| {
            let mut idx = ShapeIndex::new();
            idx.add(Box::new(Polyline::new(pts.clone())));
            black_box(&mut idx).build();
            idx
        });
    });
}

fn bench_build_10000_polyline_shapes(c: &mut Criterion) {
    c.bench_function("build_10000_polyline_shapes", |b| {
        let polylines: Vec<Polyline> = (0..10000)
            .map(|i| {
                let t = i as f64 / 10000.0;
                let lat = -60.0 + 120.0 * t;
                let lng = -160.0 + 320.0 * ((i * 137) % 10000) as f64 / 10000.0;
                let p0 = LatLng::from_degrees(lat, lng).to_point();
                let p1 = LatLng::from_degrees(lat + 0.01, lng + 0.01).to_point();
                Polyline::new(vec![p0, p1])
            })
            .collect();
        b.iter(|| {
            let mut idx = ShapeIndex::new();
            for pl in &polylines {
                idx.add(Box::new(pl.clone()));
            }
            black_box(&mut idx).build();
            idx
        });
    });
}

criterion_group!(
    benches,
    bench_build_polyline_1000,
    bench_build_polyline_10000,
    bench_build_10000_polyline_shapes,
);
criterion_main!(benches);
