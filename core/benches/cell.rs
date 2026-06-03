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
use s2rst::s2::{Cap, Cell, CellId, LatLng, Point, Rect};

#[library_benchmark]
fn bench_cell_from_cell_id() -> Cell {
    let id = black_box(CellId::from_face_pos_level(3, 0x12345678, 15));
    black_box(Cell::from_cell_id(id))
}

#[library_benchmark]
fn bench_cell_from_point() -> Cell {
    let p = black_box(LatLng::from_degrees(47.6, -122.3).to_point());
    black_box(Cell::from_point(p))
}

#[library_benchmark]
fn bench_cell_vertex() -> Point {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let cell = black_box(cell);
    let _ = black_box(cell.vertex(0));
    let _ = black_box(cell.vertex(1));
    let _ = black_box(cell.vertex(2));
    black_box(cell.vertex(3))
}

#[library_benchmark]
fn bench_cell_children() -> Option<[Cell; 4]> {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let cell = black_box(cell);
    black_box(cell.children())
}

#[library_benchmark]
fn bench_cell_contains_point() -> bool {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let p = cell.center();
    black_box(black_box(cell).contains_point(black_box(p)))
}

#[library_benchmark]
fn bench_cell_cap_bound() -> Cap {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let cell = black_box(cell);
    black_box(cell.cap_bound())
}

#[library_benchmark]
fn bench_cell_rect_bound() -> Rect {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let cell = black_box(cell);
    black_box(cell.rect_bound())
}

// C++: BM_GetDistanceToPoint
#[library_benchmark]
fn bench_cell_distance_to_point() -> ChordAngle {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let p = LatLng::from_degrees(48.0, -121.0).to_point();
    black_box(black_box(cell).distance_to_point(black_box(p)))
}

// C++: BM_GetDistanceToEdge
#[library_benchmark]
fn bench_cell_distance_to_edge() -> ChordAngle {
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let a = LatLng::from_degrees(47.5, -122.5).to_point();
    let b = LatLng::from_degrees(48.5, -121.5).to_point();
    black_box(black_box(cell).distance_to_edge(black_box(a), black_box(b)))
}

// C++: BM_GetDistanceToCell
#[library_benchmark]
fn bench_cell_distance_to_cell() -> ChordAngle {
    let c1 = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345678, 15));
    let c2 = Cell::from_cell_id(CellId::from_face_pos_level(3, 0x12345679, 15));
    black_box(black_box(c1).distance_to_cell(black_box(c2)))
}

// C++: BM_Subdivide — recursively expand face 0 to level 3
#[library_benchmark]
fn bench_cell_subdivide() {
    fn expand(cell: Cell, level: u8) {
        if cell.level() < level
            && let Some(children) = cell.children()
        {
            for child in &children {
                expand(*child, level);
            }
        }
    }
    let root = black_box(Cell::from_cell_id(CellId::from_face(0)));
    expand(root, 3);
}

// C++: BM_DistanceCompare — is_distance_less with 50th percentile limit
#[library_benchmark]
fn bench_cell_is_distance_less() -> usize {
    let origin = Cell::from_cell_id(CellId::from_face_pos_level(4, 0xB181D000000000, 30));
    // Build 100 nearby cells and compute a median distance as limit.
    let cells: Vec<Cell> = (0..100_u64)
        .map(|i| Cell::from_cell_id(CellId::from_face_pos_level(4, 0xB181D000000000 + i, 30)))
        .collect();
    let mut distances: Vec<ChordAngle> =
        cells.iter().map(|c| origin.distance_to_cell(*c)).collect();
    distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let limit = distances[distances.len() / 2]; // median
    let mut count = 0usize;
    for c in &cells {
        if origin.is_distance_less(*c, limit) {
            count += 1;
        }
    }
    black_box(count)
}

// C++: BM_GetDistanceToCellSameFace — distance between cells on same face
#[library_benchmark]
fn bench_cell_distance_same_face() -> ChordAngle {
    let c1 = Cell::from_cell_id(CellId::from_face_pos_level(4, 0xB181D000000000, 20));
    let c2 = Cell::from_cell_id(CellId::from_face_pos_level(4, 0xB181D000100000, 20));
    black_box(c1.distance_to_cell(c2))
}

library_benchmark_group!(
    name = cell_benchmarks;
    benchmarks =
        bench_cell_from_cell_id,
        bench_cell_from_point,
        bench_cell_vertex,
        bench_cell_children,
        bench_cell_contains_point,
        bench_cell_cap_bound,
        bench_cell_rect_bound,
        bench_cell_distance_to_point,
        bench_cell_distance_to_edge,
        bench_cell_distance_to_cell,
        bench_cell_subdivide,
        bench_cell_is_distance_less,
        bench_cell_distance_same_face
);

main!(library_benchmark_groups = cell_benchmarks);
