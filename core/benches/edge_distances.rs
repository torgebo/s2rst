// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::edge_distances;
use s2rst::s2::{LatLng, Point};

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

#[library_benchmark]
fn interpolate_midpoint() -> Point {
    let a = p(0.0, 0.0);
    let b = p(0.0, 10.0);
    black_box(edge_distances::interpolate(black_box(0.5), a, b))
}

#[library_benchmark]
fn interpolate_at_distance() -> Point {
    let a = p(0.0, 0.0);
    let b = p(0.0, 10.0);
    black_box(edge_distances::interpolate_at_distance(
        black_box(Angle::from_degrees(5.0)),
        a,
        b,
    ))
}

#[library_benchmark]
fn project_onto_edge() -> Point {
    let x = p(0.5, 5.0);
    let a = p(0.0, 0.0);
    let b = p(0.0, 10.0);
    black_box(edge_distances::project(black_box(x), a, b))
}

#[library_benchmark]
fn update_min_distance() -> (ChordAngle, bool) {
    let x = p(0.5, 5.0);
    let a = p(0.0, 0.0);
    let b = p(0.0, 10.0);
    black_box(edge_distances::update_min_distance(
        black_box(x),
        a,
        b,
        ChordAngle::INFINITY,
    ))
}

#[library_benchmark]
fn edge_pair_closest_points() -> (Point, Point) {
    let a0 = p(0.0, 0.0);
    let a1 = p(1.0, 0.0);
    let b0 = p(0.5, -0.5);
    let b1 = p(0.5, 0.5);
    black_box(edge_distances::edge_pair_closest_points(
        black_box(a0),
        a1,
        b0,
        b1,
    ))
}

#[library_benchmark]
fn update_min_interior_distance() -> (ChordAngle, bool) {
    let x = p(0.5, 5.0);
    let a = p(0.0, 0.0);
    let b = p(0.0, 10.0);
    black_box(edge_distances::update_min_interior_distance(
        black_box(x),
        a,
        b,
        ChordAngle::INFINITY,
    ))
}

library_benchmark_group!(
    name = edge_distances_benchmarks;
    benchmarks =
        interpolate_midpoint,
        interpolate_at_distance,
        project_onto_edge,
        update_min_distance,
        edge_pair_closest_points,
        update_min_interior_distance
);

main!(library_benchmark_groups = edge_distances_benchmarks);
