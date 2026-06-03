// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2cell_iterator_join_test.cc`
//! `BM_IterateOverlappingFractal`, `BM_IterateDisjointFractal`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::cell_iterator_join::S2CellIteratorJoin;
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Point, Polygon};

#[inline(never)]
fn make_fractal_index(num_edges: usize, center: Point, radius_deg: f64, seed: u64) -> ShapeIndex {
    let mut fractal = S2Fractal::new(seed);
    fractal.level_for_approx_max_edges(num_edges as i32);
    let loop_ = fractal.make_loop_at(center, Angle::from_degrees(radius_deg));
    let polygon = Polygon::from_loops(vec![loop_]);
    let lax = LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();
    index
}

// C++: BM_IterateOverlappingFractal — two overlapping fractals
#[library_benchmark]
fn join_overlapping_fractal_4096() -> usize {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let index_a = make_fractal_index(4096, center, 10.0, 42);
    let index_b = make_fractal_index(4096, center, 8.0, 43);
    let mut count = 0usize;
    let mut join = S2CellIteratorJoin::new(&index_a, &index_b);
    join.join(|_a, _b| {
        count += 1;
        true
    });
    black_box(count)
}

// C++: BM_IterateDisjointFractal — two fractals on opposite sides of the sphere
#[library_benchmark]
fn join_disjoint_fractal_4096() -> usize {
    let center_a = LatLng::from_degrees(0.0, 0.0).to_point();
    let center_b = LatLng::from_degrees(0.0, 180.0).to_point();
    let index_a = make_fractal_index(4096, center_a, 10.0, 42);
    let index_b = make_fractal_index(4096, center_b, 10.0, 43);
    let mut count = 0usize;
    let mut join = S2CellIteratorJoin::new(&index_a, &index_b);
    join.join(|_a, _b| {
        count += 1;
        true
    });
    black_box(count)
}

// Smaller overlapping fractals
#[library_benchmark]
fn join_overlapping_fractal_256() -> usize {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let index_a = make_fractal_index(256, center, 10.0, 42);
    let index_b = make_fractal_index(256, center, 8.0, 43);
    let mut count = 0usize;
    let mut join = S2CellIteratorJoin::new(&index_a, &index_b);
    join.join(|_a, _b| {
        count += 1;
        true
    });
    black_box(count)
}

library_benchmark_group!(
    name = cell_iterator_join_benchmarks;
    benchmarks =
        join_overlapping_fractal_4096,
        join_disjoint_fractal_4096,
        join_overlapping_fractal_256
);

main!(library_benchmark_groups = cell_iterator_join_benchmarks);
