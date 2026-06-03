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
use s2rst::s2::edge_crosser::EdgeCrosser;
use s2rst::s2::edge_crossings;
use s2rst::s2::edge_crossings::Crossing;

#[library_benchmark]
fn bench_crossing_sign() -> Crossing {
    let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
    let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
    let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
    black_box(edge_crossings::crossing_sign(a, b, c, d))
}

#[library_benchmark]
fn bench_crossing_sign_crossing() -> Crossing {
    // Two edges that actually cross.
    let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let b = black_box(Point::from_coords(-1.0, 0.0, 0.0));
    let c = black_box(Point::from_coords(0.0, 1.0, 0.0));
    let d = black_box(Point::from_coords(0.0, -1.0, 0.0));
    black_box(edge_crossings::crossing_sign(a, b, c, d))
}

#[library_benchmark]
fn bench_edge_crosser_chain() -> Crossing {
    let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
    let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
    let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
    let mut crosser = EdgeCrosser::new(a, b);
    crosser.restart_at(c);
    black_box(crosser.chain_crossing_sign(d))
}

#[library_benchmark]
fn bench_edge_or_vertex_crossing() -> bool {
    let a = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let b = black_box(Point::from_coords(0.0, 1.0, 0.0));
    let c = black_box(Point::from_coords(0.0, 0.0, 1.0));
    let d = black_box(Point::from_coords(1.0, 1.0, 0.0).normalize());
    black_box(edge_crossings::edge_or_vertex_crossing(a, b, c, d))
}

#[library_benchmark]
fn bench_intersection() -> Point {
    // Two crossing edges for intersection computation.
    let a0 = black_box(Point::from_coords(1.0, 0.0, 0.0));
    let a1 = black_box(Point::from_coords(-1.0, 0.0, 0.0));
    let b0 = black_box(Point::from_coords(0.0, 1.0, 0.0));
    let b1 = black_box(Point::from_coords(0.0, -1.0, 0.0));
    black_box(edge_crossings::intersection(a0, a1, b0, b1))
}

library_benchmark_group!(
    name = edge_crossing_benchmarks;
    benchmarks =
        bench_crossing_sign,
        bench_crossing_sign_crossing,
        bench_edge_crosser_chain,
        bench_edge_or_vertex_crossing,
        bench_intersection
);

main!(library_benchmark_groups = edge_crossing_benchmarks);
