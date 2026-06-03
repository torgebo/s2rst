// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::r2;
use s2rst::s2::r2_edge_clipper::{R2Edge, R2EdgeClipper};

fn unit_rect() -> r2::Rect {
    r2::Rect::from_points(r2::Point::new(0.0, 0.0), r2::Point::new(1.0, 1.0))
}

// Clip an edge fully inside the rectangle
#[library_benchmark]
fn clip_fully_inside() -> bool {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let edge = R2Edge::new(r2::Point::new(0.2, 0.3), r2::Point::new(0.7, 0.8));
    black_box(c.clip_edge(&edge, false))
}

// Clip an edge crossing left boundary
#[library_benchmark]
fn clip_crossing_left() -> bool {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let edge = R2Edge::new(r2::Point::new(-1.0, 0.5), r2::Point::new(0.5, 0.5));
    black_box(c.clip_edge(&edge, false))
}

// Clip an edge crossing both sides (left to right)
#[library_benchmark]
fn clip_crossing_both() -> bool {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let edge = R2Edge::new(r2::Point::new(-1.0, 0.5), r2::Point::new(2.0, 0.5));
    black_box(c.clip_edge(&edge, false))
}

// Clip an edge that completely misses
#[library_benchmark]
fn clip_miss() -> bool {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let edge = R2Edge::new(r2::Point::new(-2.0, 0.5), r2::Point::new(-1.0, 0.5));
    black_box(c.clip_edge(&edge, false))
}

// Clip a diagonal edge through the corner
#[library_benchmark]
fn clip_diagonal() -> bool {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let edge = R2Edge::new(r2::Point::new(-0.5, -0.5), r2::Point::new(1.5, 1.5));
    black_box(c.clip_edge(&edge, false))
}

// Connected edge reuse
#[library_benchmark]
fn clip_connected_chain() {
    let mut c = R2EdgeClipper::from_rect(&unit_rect());
    let e1 = R2Edge::new(r2::Point::new(-1.0, 0.5), r2::Point::new(0.5, 0.5));
    let e2 = R2Edge::new(r2::Point::new(0.5, 0.5), r2::Point::new(0.8, 0.8));
    let e3 = R2Edge::new(r2::Point::new(0.8, 0.8), r2::Point::new(2.0, 0.3));
    black_box(c.clip_edge(&e1, false));
    black_box(c.clip_edge(&e2, true));
    black_box(c.clip_edge(&e3, true));
}

library_benchmark_group!(
    name = r2_clipper;
    benchmarks =
        clip_fully_inside,
        clip_crossing_left,
        clip_crossing_both,
        clip_miss,
        clip_diagonal,
        clip_connected_chain,
);

main!(library_benchmark_groups = r2_clipper);
