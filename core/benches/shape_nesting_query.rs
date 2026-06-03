// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2shape_nesting_query_test.cc`
//! `BM_ShapeNestingQueryButtonLoop`, `BM_ShapeNestingQueryChainScaling`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::shape_nesting_query::{ChainRelation, ShapeNestingQuery};
use s2rst::s2::{LatLng, Loop, Polygon};

#[inline(never)]
fn make_concentric_lax_polygon(n_rings: usize, verts_per_ring: usize) -> LaxPolygon {
    let center = LatLng::from_degrees(0.5, 0.5).to_point();
    let loops: Vec<Loop> = (0..n_rings)
        .map(|i| {
            let radius = 2.0 / f64::from((i + 1) as i32);
            Loop::make_regular(center, Angle::from_degrees(radius), verts_per_ring)
        })
        .collect();
    let polygon = Polygon::from_loops(loops);
    LaxPolygon::from_polygon_ref(&polygon)
}

// C++: BM_ShapeNestingQueryButtonLoop (64 edges)
#[library_benchmark]
fn nesting_query_button_64() -> Vec<ChainRelation> {
    let lax = make_concentric_lax_polygon(5, 64);
    let mut index = ShapeIndex::new();
    let id = index.add(Box::new(lax));
    index.build();
    let query = ShapeNestingQuery::new(&index);
    black_box(query.compute_shape_nesting(id))
}

// C++: BM_ShapeNestingQueryButtonLoop (1024 edges)
#[library_benchmark]
fn nesting_query_button_1024() -> Vec<ChainRelation> {
    let lax = make_concentric_lax_polygon(5, 1024);
    let mut index = ShapeIndex::new();
    let id = index.add(Box::new(lax));
    index.build();
    let query = ShapeNestingQuery::new(&index);
    black_box(query.compute_shape_nesting(id))
}

// C++: BM_ShapeNestingQueryChainScaling (depth=5, 16 edges each)
#[library_benchmark]
fn nesting_query_chain_depth_5() -> Vec<ChainRelation> {
    let lax = make_concentric_lax_polygon(5, 16);
    let mut index = ShapeIndex::new();
    let id = index.add(Box::new(lax));
    index.build();
    let query = ShapeNestingQuery::new(&index);
    black_box(query.compute_shape_nesting(id))
}

// C++: BM_ShapeNestingQueryChainScaling (depth=10, 16 edges each)
#[library_benchmark]
fn nesting_query_chain_depth_10() -> Vec<ChainRelation> {
    let lax = make_concentric_lax_polygon(10, 16);
    let mut index = ShapeIndex::new();
    let id = index.add(Box::new(lax));
    index.build();
    let query = ShapeNestingQuery::new(&index);
    black_box(query.compute_shape_nesting(id))
}

// C++: BM_ShapeNestingQueryChainScaling (depth=20, 16 edges each)
#[library_benchmark]
fn nesting_query_chain_depth_20() -> Vec<ChainRelation> {
    let lax = make_concentric_lax_polygon(20, 16);
    let mut index = ShapeIndex::new();
    let id = index.add(Box::new(lax));
    index.build();
    let query = ShapeNestingQuery::new(&index);
    black_box(query.compute_shape_nesting(id))
}

library_benchmark_group!(
    name = shape_nesting_query_benchmarks;
    benchmarks =
        nesting_query_button_64,
        nesting_query_button_1024,
        nesting_query_chain_depth_5,
        nesting_query_chain_depth_10,
        nesting_query_chain_depth_20
);

main!(library_benchmark_groups = shape_nesting_query_benchmarks);
