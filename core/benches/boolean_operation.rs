// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};

use s2rst::s2::boolean_operation::{OpType, Options, S2BooleanOperation};
use s2rst::s2::builder::lax_polygon_layer::LaxPolygonLayer;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::text_format;
use s2rst::s2::{LatLng, Loop, Polygon};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_polygon_index(polygon_str: &str) -> ShapeIndex {
    text_format::make_index(&format!("# # {polygon_str}"))
}

#[inline(never)]
fn make_square_polygon_index(size_deg: f64) -> ShapeIndex {
    let s = size_deg;
    let polygon_str = format!("0:0, 0:{s}, {s}:{s}, {s}:0");
    make_polygon_index(&polygon_str)
}

#[inline(never)]
fn make_regular_polygon_index(n: usize) -> ShapeIndex {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let lat = 10.0 * angle.cos();
        let lng = 10.0 * angle.sin();
        pts.push(LatLng::from_degrees(lat, lng).to_point());
    }
    let loop_ = Loop::new(pts);
    let polygon = Polygon::from_loops(vec![loop_]);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
    idx
}

fn run_boolean_op(op_type: OpType, a_str: &str, b_str: &str) {
    let mut a = text_format::make_index(a_str);
    let mut b = text_format::make_index(b_str);
    let layer = LaxPolygonLayer::new();
    let mut op = S2BooleanOperation::new(op_type, Box::new(layer), Options::default());
    let _result = op.build(&mut a, &mut b);
}

// ─── Polygon union ─────────────────────────────────────────────────────

#[library_benchmark]
fn union_two_squares() {
    run_boolean_op(
        OpType::Union,
        "# # 0:0, 0:5, 5:5, 5:0",
        "# # 3:3, 3:8, 8:8, 8:3",
    );
}

#[library_benchmark]
fn union_non_overlapping() {
    run_boolean_op(
        OpType::Union,
        "# # 0:0, 0:5, 5:5, 5:0",
        "# # 10:10, 10:15, 15:15, 15:10",
    );
}

#[library_benchmark]
fn intersection_two_squares() {
    run_boolean_op(
        OpType::Intersection,
        "# # 0:0, 0:5, 5:5, 5:0",
        "# # 3:3, 3:8, 8:8, 8:3",
    );
}

#[library_benchmark]
fn difference_two_squares() {
    run_boolean_op(
        OpType::Difference,
        "# # 0:0, 0:5, 5:5, 5:0",
        "# # 3:3, 3:8, 8:8, 8:3",
    );
}

#[library_benchmark]
fn symmetric_difference_two_squares() {
    run_boolean_op(
        OpType::SymmetricDifference,
        "# # 0:0, 0:5, 5:5, 5:0",
        "# # 3:3, 3:8, 8:8, 8:3",
    );
}

// ─── Polygon complexity scaling ────────────────────────────────────────

#[library_benchmark]
fn union_regular_polygon_16_vertices() {
    let mut a = make_regular_polygon_index(16);
    let mut b = make_square_polygon_index(5.0);
    let layer = LaxPolygonLayer::new();
    let mut op = S2BooleanOperation::new(OpType::Union, Box::new(layer), Options::default());
    let _result = op.build(&mut a, &mut b);
}

#[library_benchmark]
fn union_regular_polygon_64_vertices() {
    let mut a = make_regular_polygon_index(64);
    let mut b = make_square_polygon_index(5.0);
    let layer = LaxPolygonLayer::new();
    let mut op = S2BooleanOperation::new(OpType::Union, Box::new(layer), Options::default());
    let _result = op.build(&mut a, &mut b);
}

#[library_benchmark]
fn union_regular_polygon_256_vertices() {
    let mut a = make_regular_polygon_index(256);
    let mut b = make_square_polygon_index(5.0);
    let layer = LaxPolygonLayer::new();
    let mut op = S2BooleanOperation::new(OpType::Union, Box::new(layer), Options::default());
    let _result = op.build(&mut a, &mut b);
}

// ─── Polyline × polygon ───────────────────────────────────────────────

#[library_benchmark]
fn polyline_polygon_intersection() {
    run_boolean_op(
        OpType::Intersection,
        "# 0:2, 10:2 # ",
        "# # 0:0, 0:5, 5:5, 5:0",
    );
}

// ─── Mixed geometry ────────────────────────────────────────────────────

#[library_benchmark]
fn three_overlapping_bars() {
    run_boolean_op(
        OpType::Union,
        "# # 0:0, 0:3, 1:3, 1:0; 0:1, 0:4, 1:4, 1:1",
        "# # 0:2, 0:5, 1:5, 1:2",
    );
}

library_benchmark_group!(
    name = basic_ops;
    benchmarks =
        union_two_squares,
        union_non_overlapping,
        intersection_two_squares,
        difference_two_squares,
        symmetric_difference_two_squares
);

library_benchmark_group!(
    name = polygon_scaling;
    benchmarks =
        union_regular_polygon_16_vertices,
        union_regular_polygon_64_vertices,
        union_regular_polygon_256_vertices
);

library_benchmark_group!(
    name = mixed_geometry;
    benchmarks =
        polyline_polygon_intersection,
        three_overlapping_bars
);

main!(
    library_benchmark_groups = basic_ops,
    polygon_scaling,
    mixed_geometry
);
