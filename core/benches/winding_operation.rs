// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2winding_operation_test.cc`
//! `BM_LoopWithPointCloud`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::winding_operation::{S2WindingOperation, WindingOptions, WindingRule};
use s2rst::s2::{LatLng, Loop, Point};

#[inline(never)]
fn make_cloud(n: usize) -> Vec<Point> {
    // Deterministic points spread around the sphere.
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            LatLng::from_degrees(-80.0 + 160.0 * t, -170.0 + 340.0 * (t * 7.3).fract()).to_point()
        })
        .collect()
}

fn bench_winding(loop_vertices: usize, cloud_size: usize) {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(10.0), loop_vertices);
    let loop_verts: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    let cloud = make_cloud(cloud_size);

    let mut options = WindingOptions::new();
    options.set_include_degeneracies(true);
    let layer = S2PolygonLayer::new();
    let mut op = S2WindingOperation::new(Box::new(layer), options);
    op.add_loop(&loop_verts);
    for p in &cloud {
        op.add_loop(std::slice::from_ref(p));
    }
    drop(op.build(center, 1, WindingRule::Positive));
}

// C++: BM_LoopWithPointCloud (10000 vertices, 0 cloud points)
#[library_benchmark]
fn winding_loop_10k_cloud_0() {
    bench_winding(1000, 0);
    black_box(());
}

// C++: BM_LoopWithPointCloud (10000 vertices, 1 cloud point)
#[library_benchmark]
fn winding_loop_1k_cloud_1() {
    bench_winding(1000, 1);
    black_box(());
}

// C++: BM_LoopWithPointCloud (10000 vertices, 10 cloud points)
#[library_benchmark]
fn winding_loop_1k_cloud_10() {
    bench_winding(1000, 10);
    black_box(());
}

// C++: BM_LoopWithPointCloud (10000 vertices, 100 cloud points)
#[library_benchmark]
fn winding_loop_1k_cloud_100() {
    bench_winding(1000, 100);
    black_box(());
}

library_benchmark_group!(
    name = winding_operation_benchmarks;
    benchmarks =
        winding_loop_10k_cloud_0,
        winding_loop_1k_cloud_1,
        winding_loop_1k_cloud_10,
        winding_loop_1k_cloud_100
);

main!(library_benchmark_groups = winding_operation_benchmarks);
