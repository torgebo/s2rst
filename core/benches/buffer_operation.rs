// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2buffer_operation_test.cc`
//! `BM_BufferPoints`, `BM_BufferConvexLoop`, `BM_BufferConcaveLoop`,
//! `BM_BufferHiDimFractal`, `BM_BufferLoDimFractal`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::buffer_operation::{BufferOptions, S2BufferOperation};
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::earth;
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::{LatLng, Loop, Point};

fn buffer_point(p: Point, radius: Angle) {
    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(radius);
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_point(p);
    op.build().unwrap();
}

fn buffer_loop(vertices: &[Point], radius: Angle) {
    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(radius);
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_loop(vertices);
    op.build().unwrap();
}

// C++: BM_BufferPoints (1 point)
#[library_benchmark]
fn buffer_point_1() {
    let p = LatLng::from_degrees(0.0, 0.0).to_point();
    let radius = earth::meters_to_angle(100.0);
    buffer_point(p, radius);
    black_box(());
}

// C++: BM_BufferPoints (10 points)
#[library_benchmark]
fn buffer_points_10() {
    let radius = earth::meters_to_angle(100.0);
    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(radius);
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    for i in 0..10 {
        let lat = f64::from(i) * 0.1;
        op.add_point(LatLng::from_degrees(lat, lat * 1.3).to_point());
    }
    op.build().unwrap();
}

// C++: BM_BufferConvexLoop (10 vertices, 10m buffer)
#[library_benchmark]
fn buffer_convex_loop_10v() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let loop_ = Loop::make_regular(center, earth::meters_to_angle(1000.0), 10);
    let verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    buffer_loop(&verts, earth::meters_to_angle(10.0));
    black_box(());
}

// C++: BM_BufferConvexLoop (100 vertices, 10m buffer)
#[library_benchmark]
fn buffer_convex_loop_100v() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let loop_ = Loop::make_regular(center, earth::meters_to_angle(1000.0), 100);
    let verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    buffer_loop(&verts, earth::meters_to_angle(10.0));
    black_box(());
}

// C++: BM_BufferConcaveLoop (10 vertices, 10m buffer — reversed winding)
#[library_benchmark]
fn buffer_concave_loop_10v() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let loop_ = Loop::make_regular(center, earth::meters_to_angle(1000.0), 10);
    let mut verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    verts.reverse();
    buffer_loop(&verts, earth::meters_to_angle(10.0));
    black_box(());
}

// C++: BM_BufferConcaveLoop (100 vertices, 100m buffer)
#[library_benchmark]
fn buffer_concave_loop_100v() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let loop_ = Loop::make_regular(center, earth::meters_to_angle(1000.0), 100);
    let mut verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    verts.reverse();
    buffer_loop(&verts, earth::meters_to_angle(100.0));
    black_box(());
}

// C++: BM_BufferHiDimFractal (dimension ~1.5, 256 edges, 10m buffer)
#[library_benchmark]
fn buffer_hi_dim_fractal_256() {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(256);
    fractal.set_fractal_dimension(1.5);
    let loop_ = fractal.make_loop_at(
        Point::from_coords(1.0, 0.0, 0.0),
        earth::meters_to_angle(1000.0),
    );
    let verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    buffer_loop(&verts, earth::meters_to_angle(10.0));
    black_box(());
}

// C++: BM_BufferLoDimFractal (dimension ~1.1, 256 edges, 10m buffer)
#[library_benchmark]
fn buffer_lo_dim_fractal_256() {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(256);
    fractal.set_fractal_dimension(1.1);
    let loop_ = fractal.make_loop_at(
        Point::from_coords(1.0, 0.0, 0.0),
        earth::meters_to_angle(1000.0),
    );
    let verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    buffer_loop(&verts, earth::meters_to_angle(10.0));
    black_box(());
}

library_benchmark_group!(
    name = buffer_operation_benchmarks;
    benchmarks =
        buffer_point_1,
        buffer_points_10,
        buffer_convex_loop_10v,
        buffer_convex_loop_100v,
        buffer_concave_loop_10v,
        buffer_concave_loop_100v,
        buffer_hi_dim_fractal_256,
        buffer_lo_dim_fractal_256
);

main!(library_benchmark_groups = buffer_operation_benchmarks);
