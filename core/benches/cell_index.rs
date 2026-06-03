// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2cell_index_test.cc`
//! `BM_FindIntersectingCapCoverings`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::cell_index::{CellIndex, CellIndexRangeIterator};
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, CellUnion, LatLng, Point};

#[inline(never)]
fn make_cap_covering(center: Point, radius_deg: f64, max_cells: usize) -> CellUnion {
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    RegionCoverer::new().max_cells(max_cells).covering(&cap)
}

#[inline(never)]
fn build_index(num_caps: usize, max_cells: usize) -> CellIndex {
    let mut index = CellIndex::new();
    for i in 0..num_caps {
        let t = i as f64 / num_caps as f64;
        let lat = -60.0 + 120.0 * t;
        let lng = -160.0 + 320.0 * (t * 7.3).fract();
        let center = LatLng::from_degrees(lat, lng).to_point();
        let covering = make_cap_covering(center, 0.5, max_cells);
        for id in covering.cell_ids() {
            index.add(*id, i as i32);
        }
    }
    index.build();
    index
}

fn count_intersecting(index: &CellIndex, target: &CellUnion) -> usize {
    let mut it = CellIndexRangeIterator::new_non_empty(index);
    let mut count = 0usize;
    for &cell_id in target.cell_ids() {
        it.begin();
        while !it.done() {
            let range_min = it.start_id();
            let range_max = it.limit_id();
            if range_min <= cell_id.range_max() && cell_id.range_min() < range_max {
                count += 1;
            }
            it.next();
        }
    }
    count
}

// C++: BM_FindIntersectingCapCoverings (100 caps, 16 cells each)
#[library_benchmark]
fn find_intersecting_100_caps_16cells() -> usize {
    let index = build_index(100, 16);
    let query_center = LatLng::from_degrees(0.0, 0.0).to_point();
    let target = make_cap_covering(query_center, 1.0, 16);
    black_box(count_intersecting(&index, &target))
}

// C++: BM_FindIntersectingCapCoverings (100 caps, 128 cells each)
#[library_benchmark]
fn find_intersecting_100_caps_128cells() -> usize {
    let index = build_index(100, 128);
    let query_center = LatLng::from_degrees(0.0, 0.0).to_point();
    let target = make_cap_covering(query_center, 1.0, 128);
    black_box(count_intersecting(&index, &target))
}

// 1000 caps, 16 cells each
#[library_benchmark]
fn find_intersecting_1000_caps_16cells() -> usize {
    let index = build_index(1000, 16);
    let query_center = LatLng::from_degrees(0.0, 0.0).to_point();
    let target = make_cap_covering(query_center, 1.0, 16);
    black_box(count_intersecting(&index, &target))
}

library_benchmark_group!(
    name = cell_index_benchmarks;
    benchmarks =
        find_intersecting_100_caps_16cells,
        find_intersecting_100_caps_128cells,
        find_intersecting_1000_caps_16cells
);

main!(library_benchmark_groups = cell_index_benchmarks);
