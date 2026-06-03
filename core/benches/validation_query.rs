// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2validation_query_test.cc`
//! `BM_LegacyValidFractal`, `BM_LegacyPolygonValidFractal`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::builder::S2Error;
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::validation_query::{S2LegacyValidQuery, S2ValidQuery};
use s2rst::s2::{LatLng, Polygon};

#[inline(never)]
fn make_fractal_index(num_edges: usize) -> ShapeIndex {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(num_edges as i32);
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = fractal.make_loop_at(center, Angle::from_degrees(10.0));
    let polygon = Polygon::from_loops(vec![loop_]);
    let lax = LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();
    index
}

// C++: BM_LegacyValidFractal (256 edges)
#[library_benchmark]
fn legacy_valid_fractal_256() -> Result<(), S2Error> {
    let index = make_fractal_index(256);
    let query = S2LegacyValidQuery::new();
    black_box(query.validate(&index))
}

// C++: BM_LegacyValidFractal (4096 edges)
#[library_benchmark]
fn legacy_valid_fractal_4096() -> Result<(), S2Error> {
    let index = make_fractal_index(4096);
    let query = S2LegacyValidQuery::new();
    black_box(query.validate(&index))
}

// S2ValidQuery (256 edges)
#[library_benchmark]
fn valid_query_fractal_256() -> Result<(), S2Error> {
    let index = make_fractal_index(256);
    let query = S2ValidQuery::new();
    black_box(query.validate(&index))
}

// S2ValidQuery (4096 edges)
#[library_benchmark]
fn valid_query_fractal_4096() -> Result<(), S2Error> {
    let index = make_fractal_index(4096);
    let query = S2ValidQuery::new();
    black_box(query.validate(&index))
}

library_benchmark_group!(
    name = validation_query_benchmarks;
    benchmarks =
        legacy_valid_fractal_256,
        legacy_valid_fractal_4096,
        valid_query_fractal_256,
        valid_query_fractal_4096
);

main!(library_benchmark_groups = validation_query_benchmarks);
