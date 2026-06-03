// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::r3::Vector;
use s2rst::s2::robust_cell_clipper::{Options, RobustCellClipper, RobustClipResult};
use s2rst::s2::{Cell, CellId, LatLng, Point};

fn cell_05() -> Cell {
    Cell::from_cell_id(CellId::from_token("05"))
}

// Clip an edge fully inside the cell (fast path)
#[library_benchmark]
fn clip_edge_inside() -> RobustClipResult {
    let cell = cell_05();
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let v0 = black_box(cell.center());
    let tiny = Point((v0.0 + Vector::new(1e-10, 1e-10, 0.0)).normalize());
    black_box(c.clip_edge(v0, tiny, false))
}

// Clip an edge that completely misses (different face)
#[library_benchmark]
fn clip_edge_miss_face() -> RobustClipResult {
    let cell = cell_05();
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let v0 = black_box(LatLng::from_degrees(40.0, -170.0).to_point());
    let v1 = black_box(LatLng::from_degrees(41.0, -170.0).to_point());
    black_box(c.clip_edge(v0, v1, false))
}

// Clip an edge crossing a cell boundary (normal path with crossings)
#[library_benchmark]
fn clip_edge_crossing() -> RobustClipResult {
    let cell = cell_05();
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let center = cell.center();
    let outside = LatLng::from_degrees(80.0, 0.0).to_point();
    black_box(c.clip_edge(center, outside, false))
}

// Clip edge near corner (triggers exact path)
#[library_benchmark]
fn clip_edge_near_corner() -> RobustClipResult {
    let cell = Cell::from_cell_id(CellId::from_token("14"));
    let corner = cell.vertex(0);
    let k_tiny = 2e-15;
    let pnt0 = Point((corner.0 + Vector::new(-k_tiny, -k_tiny, k_tiny)).normalize());
    let pnt1 = Point((corner.0 + Vector::new(-k_tiny, k_tiny, -k_tiny)).normalize());
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    black_box(c.clip_edge(pnt0, pnt1, false))
}

// Clip with crossings disabled
#[library_benchmark]
fn clip_edge_no_crossings() -> RobustClipResult {
    let cell = cell_05();
    let mut c = RobustCellClipper::with_options(Options {
        enable_crossings: false,
    });
    c.start_cell(cell);
    let center = cell.center();
    let outside = LatLng::from_degrees(80.0, 0.0).to_point();
    black_box(c.clip_edge(center, outside, false))
}

// start_cell setup cost
#[library_benchmark]
fn start_cell_setup() {
    let cell = cell_05();
    let mut c = RobustCellClipper::new();
    c.start_cell(black_box(cell));
}

// Clip multiple edges and get sorted crossings
#[library_benchmark]
fn clip_multiple_and_sort() -> usize {
    let cell = Cell::from_cell_id(CellId::from_token("1b"));
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let center = cell.center();
    for k in 0..4 {
        let v = cell.vertex(k % 4);
        let outer = Point((center.0 * 2.0 - v.0).normalize());
        c.clip_edge(center, outer, false);
    }
    black_box(c.get_crossings().len())
}

// is_boundary_contained with contained edges
#[library_benchmark]
fn boundary_contained() -> bool {
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let v0 = LatLng::from_degrees(-10.0, 0.0).to_point();
    let v1 = LatLng::from_degrees(0.0, 10.0).to_point();
    let v2 = LatLng::from_degrees(10.0, 0.0).to_point();
    let v3 = LatLng::from_degrees(0.0, -10.0).to_point();
    c.clip_edge(v0, v1, false);
    c.clip_edge(v1, v2, false);
    c.clip_edge(v2, v3, false);
    c.clip_edge(v3, v0, false);
    black_box(c.is_boundary_contained(false))
}

library_benchmark_group!(
    name = robust_clipper;
    benchmarks =
        clip_edge_inside,
        clip_edge_miss_face,
        clip_edge_crossing,
        clip_edge_near_corner,
        clip_edge_no_crossings,
        start_cell_setup,
        clip_multiple_and_sort,
        boundary_contained,
);

main!(library_benchmark_groups = robust_clipper);
