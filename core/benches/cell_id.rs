// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::{CellId, Face, LatLng, Point, from_face_ij};

#[library_benchmark]
fn bench_cellid_from_point() -> CellId {
    let p = black_box(Point::from_coords(1.0, 2.0, 3.0).normalize());
    black_box(CellId::from_point(&p))
}

#[library_benchmark]
fn bench_cellid_to_point() -> Point {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 20));
    black_box(id.to_point())
}

// C++: BM_ToPointRaw
#[library_benchmark]
fn bench_cellid_to_point_raw() -> Point {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 20));
    black_box(id.to_point_raw())
}

#[library_benchmark]
fn bench_cellid_from_lat_lng() -> CellId {
    let ll = black_box(LatLng::from_degrees(47.6, -122.3));
    black_box(CellId::from_lat_lng(&ll))
}

// C++: BM_FromFaceIJ
#[library_benchmark]
fn bench_cellid_from_face_ij() -> CellId {
    black_box(from_face_ij(
        black_box(Face::F3),
        black_box(12345),
        black_box(67890),
    ))
}

#[library_benchmark]
fn bench_cellid_level() -> u8 {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.level().as_u8())
}

#[library_benchmark]
fn bench_cellid_parent() -> CellId {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 20));
    black_box(id.parent())
}

#[library_benchmark]
fn bench_cellid_parent_at_level() -> CellId {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 20));
    black_box(id.parent_at_level(10))
}

// C++: BM_child_position (level 10)
#[library_benchmark]
fn bench_cellid_child_position() -> u8 {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.child_position(10))
}

#[library_benchmark]
fn bench_cellid_children() -> [CellId; 4] {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.children())
}

#[library_benchmark]
fn bench_cellid_all_neighbors() -> Option<Vec<CellId>> {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.all_neighbors(15))
}

#[library_benchmark]
fn bench_cellid_advance() -> CellId {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 20));
    black_box(id.advance(123))
}

#[library_benchmark]
fn bench_cellid_contains() -> bool {
    let parent = CellId::from_face_pos_level(3, 0x12345678, 10);
    let child = parent.children()[0].children()[1];
    black_box(black_box(parent).contains(black_box(child)))
}

#[library_benchmark]
fn bench_cellid_edge_neighbors() -> [CellId; 4] {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.edge_neighbors())
}

// C++: BM_ToToken (level 15)
#[library_benchmark]
fn bench_cellid_to_token() -> String {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(id.to_token())
}

// C++: BM_FromToken
#[library_benchmark]
fn bench_cellid_from_token() -> CellId {
    let token = "89c25c1";
    black_box(CellId::from_token(black_box(token)))
}

library_benchmark_group!(
    name = cellid_benchmarks;
    benchmarks =
        bench_cellid_from_point,
        bench_cellid_to_point,
        bench_cellid_to_point_raw,
        bench_cellid_from_lat_lng,
        bench_cellid_from_face_ij,
        bench_cellid_level,
        bench_cellid_parent,
        bench_cellid_parent_at_level,
        bench_cellid_child_position,
        bench_cellid_children,
        bench_cellid_all_neighbors,
        bench_cellid_advance,
        bench_cellid_contains,
        bench_cellid_edge_neighbors,
        bench_cellid_to_token,
        bench_cellid_from_token
);

main!(library_benchmark_groups = cellid_benchmarks);
