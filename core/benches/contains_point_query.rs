// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Loop, Point, Polygon};

#[inline(never)]
fn make_polygon_index(n: usize) -> ShapeIndex {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    let polygon = Polygon::from_loops(vec![loop_]);
    let lax = LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();
    index
}

fn inside_point() -> Point {
    LatLng::from_degrees(0.5, 0.5).to_point()
}

fn outside_point() -> Point {
    LatLng::from_degrees(20.0, 20.0).to_point()
}

// C++: BM_ContainsPointLoopGrid / Java: containsPoint
#[library_benchmark]
fn contains_point_inside_64v() -> bool {
    let index = make_polygon_index(64);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.contains(black_box(inside_point())))
}

#[library_benchmark]
fn contains_point_outside_64v() -> bool {
    let index = make_polygon_index(64);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.contains(black_box(outside_point())))
}

#[library_benchmark]
fn contains_point_inside_256v() -> bool {
    let index = make_polygon_index(256);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.contains(black_box(inside_point())))
}

#[library_benchmark]
fn contains_point_outside_256v() -> bool {
    let index = make_polygon_index(256);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.contains(black_box(outside_point())))
}

#[library_benchmark]
fn contains_point_inside_1024v() -> bool {
    let index = make_polygon_index(1024);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.contains(black_box(inside_point())))
}

#[library_benchmark]
fn containing_shapes_256v() {
    let index = make_polygon_index(256);
    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    black_box(query.containing_shape_ids(black_box(inside_point())));
}

library_benchmark_group!(
    name = contains_point_benchmarks;
    benchmarks =
        contains_point_inside_64v,
        contains_point_outside_64v,
        contains_point_inside_256v,
        contains_point_outside_256v,
        contains_point_inside_1024v,
        containing_shapes_256v
);

main!(library_benchmark_groups = contains_point_benchmarks);
