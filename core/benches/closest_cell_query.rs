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
use s2rst::s2::cell_index::CellIndex;
use s2rst::s2::closest_cell_query::{ClosestCellQuery, Options, PointTarget};
use s2rst::s2::{CellId, LatLng};

#[inline(never)]
fn make_cell_index(n: usize) -> CellIndex {
    let center = LatLng::from_degrees(37.7749, -122.4194).to_point();
    let center_id = CellId::from_point(&center);
    let mut index = CellIndex::new();
    for i in 0..n {
        let id = center_id.parent_at_level(15).advance(i as i64);
        index.add(id, i as i32);
    }
    index.build();
    index
}

#[library_benchmark]
fn find_closest_cell_100() {
    let index = make_cell_index(100);
    let query = ClosestCellQuery::new(&index, Options::default());
    for i in 0..10 {
        let lat = 37.77 + (i as f64) * 0.001;
        let pt = LatLng::from_degrees(lat, -122.42).to_point();
        let mut target = PointTarget::new(pt);
        black_box(query.find_closest_cell(&mut target));
    }
}

#[library_benchmark]
fn find_closest_cell_1000() {
    let index = make_cell_index(1000);
    let query = ClosestCellQuery::new(&index, Options::default());
    for i in 0..10 {
        let lat = 37.77 + (i as f64) * 0.001;
        let pt = LatLng::from_degrees(lat, -122.42).to_point();
        let mut target = PointTarget::new(pt);
        black_box(query.find_closest_cell(&mut target));
    }
}

#[library_benchmark]
fn is_distance_less_100() {
    let index = make_cell_index(100);
    let query = ClosestCellQuery::new(&index, Options::default());
    let limit = ChordAngle::from_angle(Angle::from_degrees(0.01));
    for i in 0..10 {
        let lat = 37.77 + (i as f64) * 0.001;
        let pt = LatLng::from_degrees(lat, -122.42).to_point();
        let mut target = PointTarget::new(pt);
        black_box(query.is_distance_less(&mut target, limit));
    }
}

#[library_benchmark]
fn get_distance_1000() {
    let index = make_cell_index(1000);
    let query = ClosestCellQuery::new(&index, Options::default());
    for i in 0..10 {
        let lat = 37.77 + (i as f64) * 0.001;
        let pt = LatLng::from_degrees(lat, -122.42).to_point();
        let mut target = PointTarget::new(pt);
        let _dist = black_box(query.get_distance(&mut target));
    }
}

library_benchmark_group!(
    name = closest_cell_benchmarks;
    benchmarks =
        find_closest_cell_100,
        find_closest_cell_1000,
        is_distance_less_100,
        get_distance_1000
);

main!(library_benchmark_groups = closest_cell_benchmarks);
