// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::ChordAngle;
use s2rst::s2::LatLng;
use s2rst::s2::closest_point_query::{ClosestPointQuery, Options, PointTarget, Result};
use s2rst::s2::point_index::S2PointIndex;

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_point_index(n: usize) -> S2PointIndex<usize> {
    let mut idx = S2PointIndex::new();
    for i in 0..n {
        let t = i as f64 / n as f64;
        let lat = -60.0 + 120.0 * t;
        let lng = -160.0 + 320.0 * ((i * 137) % n) as f64 / n as f64;
        idx.add(LatLng::from_degrees(lat, lng).to_point(), i);
    }
    idx
}

#[inline(never)]
fn make_clustered_index(n: usize, center_lat: f64, center_lng: f64) -> S2PointIndex<usize> {
    let mut idx = S2PointIndex::new();
    for i in 0..n {
        let t = i as f64 / n as f64;
        let angle = 2.0 * std::f64::consts::PI * t;
        let r = 0.1 * (i as f64 / n as f64);
        let lat = center_lat + r * angle.cos();
        let lng = center_lng + r * angle.sin();
        idx.add(LatLng::from_degrees(lat, lng).to_point(), i);
    }
    idx
}

// ─── Find closest ──────────────────────────────────────────────────────

#[library_benchmark]
fn find_closest_100_points() -> Result<usize> {
    let idx = make_point_index(100);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

#[library_benchmark]
fn find_closest_1000_points() -> Result<usize> {
    let idx = make_point_index(1000);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

#[library_benchmark]
fn find_closest_10000_points() -> Result<usize> {
    let idx = make_point_index(10000);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

#[library_benchmark]
fn find_closest_100000_points() -> Result<usize> {
    let idx = make_point_index(100000);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

// ─── Clustered points ──────────────────────────────────────────────────

#[library_benchmark]
fn find_closest_clustered_1000() -> Result<usize> {
    let idx = make_clustered_index(1000, 47.6, -122.3);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(47.6, -122.3).to_point()));
    black_box(query.find_closest_point(&mut target))
}

#[library_benchmark]
fn find_closest_clustered_10000() -> Result<usize> {
    let idx = make_clustered_index(10000, 47.6, -122.3);
    let query = ClosestPointQuery::new(&idx, Options::default());
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(47.6, -122.3).to_point()));
    black_box(query.find_closest_point(&mut target))
}

// ─── With max distance ─────────────────────────────────────────────────

#[library_benchmark]
fn find_closest_max_dist_10000() -> Result<usize> {
    let idx = make_point_index(10000);
    let mut opts = Options::default();
    opts.conservative_max_distance(ChordAngle::from_degrees(1.0));
    let query = ClosestPointQuery::new(&idx, opts);
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

#[library_benchmark]
fn find_closest_max_dist_tiny_10000() -> Result<usize> {
    let idx = make_point_index(10000);
    let mut opts = Options::default();
    opts.conservative_max_distance(ChordAngle::from_degrees(0.0001));
    let query = ClosestPointQuery::new(&idx, opts);
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_point(&mut target))
}

// ─── Find k closest ────────────────────────────────────────────────────

#[library_benchmark]
fn find_top5_10000_points() -> Vec<Result<usize>> {
    let idx = make_point_index(10000);
    let opts = Options {
        max_results: 5,
        ..Options::default()
    };
    let query = ClosestPointQuery::new(&idx, opts);
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_points(&mut target))
}

#[library_benchmark]
fn find_top100_10000_points() -> Vec<Result<usize>> {
    let idx = make_point_index(10000);
    let opts = Options {
        max_results: 100,
        ..Options::default()
    };
    let query = ClosestPointQuery::new(&idx, opts);
    let mut target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_points(&mut target))
}

library_benchmark_group!(
    name = find_closest_scaling;
    benchmarks =
        find_closest_100_points,
        find_closest_1000_points,
        find_closest_10000_points,
        find_closest_100000_points
);

library_benchmark_group!(
    name = find_closest_clustered;
    benchmarks =
        find_closest_clustered_1000,
        find_closest_clustered_10000
);

library_benchmark_group!(
    name = find_closest_max_dist;
    benchmarks =
        find_closest_max_dist_10000,
        find_closest_max_dist_tiny_10000
);

library_benchmark_group!(
    name = find_top_k;
    benchmarks =
        find_top5_10000_points,
        find_top100_10000_points
);

main!(
    library_benchmark_groups = find_closest_scaling,
    find_closest_clustered,
    find_closest_max_dist,
    find_top_k
);
