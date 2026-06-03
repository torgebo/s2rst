// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::uv_edge_clipper::UVEdgeClipper;
use s2rst::s2::{Cell, CellId, LatLng};

fn face0_cell() -> Cell {
    Cell::from_cell_id(CellId::from_face(0))
}

// Clip an edge fully inside a face cell
#[library_benchmark]
fn clip_inside_face() -> bool {
    let cell = face0_cell();
    let mut c = UVEdgeClipper::from_cell(cell);
    let v0 = black_box(LatLng::from_degrees(10.0, 10.0).to_point());
    let v1 = black_box(LatLng::from_degrees(20.0, 20.0).to_point());
    black_box(c.clip_edge(v0, v1, false))
}

// Clip an edge that misses the face entirely
#[library_benchmark]
fn clip_miss_face() -> bool {
    let cell = face0_cell();
    let mut c = UVEdgeClipper::from_cell(cell);
    let v0 = black_box(LatLng::from_degrees(10.0, -170.0).to_point());
    let v1 = black_box(LatLng::from_degrees(20.0, -170.0).to_point());
    black_box(c.clip_edge(v0, v1, false))
}

// Clip an edge crossing face boundary
#[library_benchmark]
fn clip_cross_face() -> bool {
    let cell = face0_cell();
    let mut c = UVEdgeClipper::from_cell(cell);
    let v0 = black_box(LatLng::from_degrees(10.0, 10.0).to_point());
    let v1 = black_box(LatLng::from_degrees(10.0, 80.0).to_point());
    black_box(c.clip_edge(v0, v1, false))
}

// Clip to a small cell
#[library_benchmark]
fn clip_small_cell() -> bool {
    let id = CellId::from_face(0).children()[0].children()[0].children()[0];
    let cell = Cell::from_cell_id(id);
    let mut c = UVEdgeClipper::from_cell(cell);
    let center = cell.center();
    let v1 = black_box(LatLng::from_degrees(5.0, 5.0).to_point());
    black_box(c.clip_edge(center, v1, false))
}

// Connected edge chain
#[library_benchmark]
fn clip_connected_chain() {
    let cell = face0_cell();
    let mut c = UVEdgeClipper::from_cell(cell);
    let a = LatLng::from_degrees(10.0, 10.0).to_point();
    let b = LatLng::from_degrees(20.0, 20.0).to_point();
    let d = LatLng::from_degrees(30.0, 15.0).to_point();
    black_box(c.clip_edge(a, b, false));
    black_box(c.clip_edge(b, d, true));
}

library_benchmark_group!(
    name = uv_clipper;
    benchmarks =
        clip_inside_face,
        clip_miss_face,
        clip_cross_face,
        clip_small_cell,
        clip_connected_chain,
);

main!(library_benchmark_groups = uv_clipper);
