// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::{CellId, CellUnion, LatLng};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_cell_ids(n: usize, level: u8) -> Vec<CellId> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let lat = -60.0 + 120.0 * t;
            let lng = -160.0 + 320.0 * ((i * 137) % n) as f64 / n as f64;
            CellId::from_lat_lng(&LatLng::from_degrees(lat, lng)).parent_at_level(level)
        })
        .collect()
}

// ─── Construction ───────────────────────────────────────────────────────

#[library_benchmark]
fn from_cell_ids_100() -> CellUnion {
    let ids = make_cell_ids(100, 15);
    black_box(CellUnion::from_cell_ids(black_box(ids)))
}

#[library_benchmark]
fn from_cell_ids_1000() -> CellUnion {
    let ids = make_cell_ids(1000, 15);
    black_box(CellUnion::from_cell_ids(black_box(ids)))
}

#[library_benchmark]
fn from_cell_ids_10000() -> CellUnion {
    let ids = make_cell_ids(10000, 15);
    black_box(CellUnion::from_cell_ids(black_box(ids)))
}

// ─── Normalize ──────────────────────────────────────────────────────────

#[library_benchmark]
fn normalize_100() {
    let mut cu = CellUnion::from_verbatim(make_cell_ids(100, 15));
    cu.normalize();
}

#[library_benchmark]
fn normalize_1000() {
    let mut cu = CellUnion::from_verbatim(make_cell_ids(1000, 15));
    cu.normalize();
}

// ─── Contains ───────────────────────────────────────────────────────────

#[library_benchmark]
fn contains_cell_id_100() -> bool {
    let cu = CellUnion::from_cell_ids(make_cell_ids(100, 15));
    let target = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(20);
    black_box(black_box(cu).contains_cell_id(black_box(target)))
}

#[library_benchmark]
fn contains_cell_id_1000() -> bool {
    let cu = CellUnion::from_cell_ids(make_cell_ids(1000, 15));
    let target = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(20);
    black_box(black_box(cu).contains_cell_id(black_box(target)))
}

#[library_benchmark]
fn contains_point_1000() -> bool {
    let cu = CellUnion::from_cell_ids(make_cell_ids(1000, 15));
    let p = black_box(LatLng::from_degrees(0.0, 0.0).to_point());
    black_box(black_box(cu).contains_point(p))
}

// ─── Intersects ─────────────────────────────────────────────────────────

#[library_benchmark]
fn intersects_cell_id_1000() -> bool {
    let cu = CellUnion::from_cell_ids(make_cell_ids(1000, 15));
    let target = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0)).parent_at_level(10);
    black_box(black_box(cu).intersects_cell_id(black_box(target)))
}

#[library_benchmark]
fn intersects_union_100() -> bool {
    let a = CellUnion::from_cell_ids(make_cell_ids(100, 15));
    let b = CellUnion::from_cell_ids(make_cell_ids(100, 15));
    black_box(black_box(a).intersects_union(black_box(&b)))
}

#[library_benchmark]
fn contains_union_100() -> bool {
    let a = CellUnion::from_cell_ids(make_cell_ids(100, 10));
    let b = CellUnion::from_cell_ids(make_cell_ids(50, 15));
    black_box(black_box(a).contains_union(black_box(&b)))
}

// ─── Leaf cells covered ────────────────────────────────────────────────

#[library_benchmark]
fn leaf_cells_covered_1000() -> i64 {
    let cu = CellUnion::from_cell_ids(make_cell_ids(1000, 15));
    black_box(black_box(cu).leaf_cells_covered())
}

// ─── From range (Go: BenchmarkCellUnionFromRange, C++: BM_InitFromBeginEnd) ─

#[library_benchmark]
fn from_range_full_sphere() -> CellUnion {
    let begin = CellId::from_face(0).child_begin_at_level(30);
    let end = CellId::from_face(5).child_end_at_level(30);
    black_box(CellUnion::from_range(begin, end))
}

#[library_benchmark]
fn from_range_single_face() -> CellUnion {
    let begin = CellId::from_face(2).child_begin_at_level(30);
    let end = CellId::from_face(2).child_end_at_level(30);
    black_box(CellUnion::from_range(begin, end))
}

library_benchmark_group!(
    name = construction;
    benchmarks = from_cell_ids_100, from_cell_ids_1000, from_cell_ids_10000,
        from_range_full_sphere, from_range_single_face
);

library_benchmark_group!(
    name = normalize;
    benchmarks = normalize_100, normalize_1000
);

library_benchmark_group!(
    name = contains;
    benchmarks = contains_cell_id_100, contains_cell_id_1000, contains_point_1000
);

library_benchmark_group!(
    name = intersects;
    benchmarks = intersects_cell_id_1000, intersects_union_100, contains_union_100
);

library_benchmark_group!(
    name = properties;
    benchmarks = leaf_cells_covered_1000
);

main!(
    library_benchmark_groups = construction,
    normalize,
    contains,
    intersects,
    properties
);
