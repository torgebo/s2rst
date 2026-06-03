// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1;
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, LatLng, Loop};

#[library_benchmark]
fn bench_covering_cap() -> CellUnion {
    let center = LatLng::from_degrees(47.6, -122.3).to_point();
    let cap = Cap::from_center_angle(center, s1::Angle::from_degrees(5.0));
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let cap = black_box(cap);
    black_box(coverer.covering(&cap))
}

#[library_benchmark]
fn bench_covering_cell() -> CellUnion {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 10));
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let cell = black_box(cell);
    black_box(coverer.covering(&cell))
}

#[library_benchmark]
fn bench_covering_loop() -> CellUnion {
    // A small quadrilateral loop.
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 1.0).to_point(),
        LatLng::from_degrees(1.0, 1.0).to_point(),
        LatLng::from_degrees(1.0, 0.0).to_point(),
    ]);
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let loop_ = black_box(loop_);
    black_box(coverer.covering(&loop_))
}

#[library_benchmark]
fn bench_covering_cap_fine() -> CellUnion {
    // A tiny cap that requires deep covering.
    let center = LatLng::from_degrees(47.6, -122.3).to_point();
    let cap = Cap::from_center_angle(center, s1::Angle::from_degrees(0.001));
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let cap = black_box(cap);
    black_box(coverer.covering(&cap))
}

// ─── Go BenchmarkRegionCovererCoveringCellUnion ─────────────────────────
// Port of Go pattern: covering a CellUnion of varying sizes.

#[library_benchmark]
fn bench_covering_cell_union_4() -> CellUnion {
    // 4 scattered cells.
    let ids: Vec<CellId> = (0..4)
        .map(|i| {
            let lat = -60.0 + 30.0 * i as f64;
            let lng = -120.0 + 60.0 * i as f64;
            CellId::from_lat_lng(&LatLng::from_degrees(lat, lng)).parent_at_level(15)
        })
        .collect();
    let cu = CellUnion::from_cell_ids(ids);
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let cu = black_box(cu);
    black_box(coverer.covering(&cu))
}

#[library_benchmark]
fn bench_covering_cell_union_64() -> CellUnion {
    let ids: Vec<CellId> = (0..64)
        .map(|i| {
            let t = i as f64 / 64.0;
            let lat = -80.0 + 160.0 * t;
            let lng = -170.0 + 340.0 * ((i * 37) % 64) as f64 / 64.0;
            CellId::from_lat_lng(&LatLng::from_degrees(lat, lng)).parent_at_level(15)
        })
        .collect();
    let cu = CellUnion::from_cell_ids(ids);
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let cu = black_box(cu);
    black_box(coverer.covering(&cu))
}

// ─── Go BenchmarkRegionCovererCoveringLoop with larger loops ────────────

#[library_benchmark]
fn bench_covering_loop_32_vertices() -> CellUnion {
    let pts: Vec<_> = (0..32)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / 32.0;
            LatLng::from_degrees(5.0 * angle.cos(), 5.0 * angle.sin()).to_point()
        })
        .collect();
    let loop_ = Loop::new(pts);
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let loop_ = black_box(loop_);
    black_box(coverer.covering(&loop_))
}

#[library_benchmark]
fn bench_covering_loop_256_vertices() -> CellUnion {
    let pts: Vec<_> = (0..256)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / 256.0;
            LatLng::from_degrees(5.0 * angle.cos(), 5.0 * angle.sin()).to_point()
        })
        .collect();
    let loop_ = Loop::new(pts);
    let coverer = RegionCoverer::new().min_level(0).max_level(30).max_cells(8);
    let loop_ = black_box(loop_);
    black_box(coverer.covering(&loop_))
}

library_benchmark_group!(
    name = region_coverer_benchmarks;
    benchmarks =
        bench_covering_cap,
        bench_covering_cell,
        bench_covering_loop,
        bench_covering_cap_fine
);

library_benchmark_group!(
    name = covering_cell_union;
    benchmarks =
        bench_covering_cell_union_4,
        bench_covering_cell_union_64
);

library_benchmark_group!(
    name = covering_loop_scaling;
    benchmarks =
        bench_covering_loop_32_vertices,
        bench_covering_loop_256_vertices
);

main!(
    library_benchmark_groups = region_coverer_benchmarks,
    covering_cell_union,
    covering_loop_scaling
);
