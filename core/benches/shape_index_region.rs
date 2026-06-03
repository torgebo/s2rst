// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2shape_index_region_test.cc`
//! `BM_VisitIntersectingPolygons{ParentCells,IndexCells,DescendantCells`}
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;
use std::ops::ControlFlow;

use s2rst::s1::Angle;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::shape_index_region::ShapeIndexRegion;
use s2rst::s2::{Cell, CellId, LatLng, Loop, Polygon};

// Build an index with `n` small polygons scattered across face 0.
#[inline(never)]
fn build_polygon_index(n: usize) -> ShapeIndex {
    let mut index = ShapeIndex::new();
    for i in 0..n {
        let t = i as f64 / n as f64;
        let lat = -10.0 + 20.0 * t;
        let lng = -10.0 + 20.0 * (t * 3.7).fract();
        let center = LatLng::from_degrees(lat, lng).to_point();
        let loop_ = Loop::make_regular(center, Angle::from_degrees(0.5), 8);
        let polygon = Polygon::from_loops(vec![loop_]);
        let lax = LaxPolygon::from_polygon_ref(&polygon);
        index.add(Box::new(lax));
    }
    index.build();
    index
}

// Visit intersecting shapes for a given cell and count them.
fn visit_and_count(index: &ShapeIndex, cell: &Cell) -> usize {
    let region = ShapeIndexRegion::new(index);
    let mut count = 0usize;
    let _ = region.visit_intersecting_shape_ids(cell, |_shape_id, _contains| {
        count += 1;
        ControlFlow::Continue(())
    });
    count
}

// C++: BM_VisitIntersectingPolygonsParentCells — query with a parent cell
#[library_benchmark]
fn visit_intersecting_parent_cell() -> usize {
    let index = build_polygon_index(50);
    // Use a large cell (face 0 at level 1) as query — parent of index cells.
    let cell = Cell::from_cell_id(CellId::from_face(0).children()[0]);
    black_box(visit_and_count(&index, &cell))
}

// C++: BM_VisitIntersectingPolygonsIndexCells — query with cells at index level
#[library_benchmark]
fn visit_intersecting_index_cells() -> usize {
    let index = build_polygon_index(50);
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cell = Cell::from_cell_id(CellId::from_point(&center).parent_at_level(10));
    black_box(visit_and_count(&index, &cell))
}

// C++: BM_VisitIntersectingPolygonsDescendantCells — query with a leaf cell
#[library_benchmark]
fn visit_intersecting_descendant_cells() -> usize {
    let index = build_polygon_index(50);
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let cell = Cell::from_cell_id(CellId::from_point(&center));
    black_box(visit_and_count(&index, &cell))
}

// Larger index: 200 polygons, parent cell query
#[library_benchmark]
fn visit_intersecting_200_polygons() -> usize {
    let index = build_polygon_index(200);
    let cell = Cell::from_cell_id(CellId::from_face(0).children()[0]);
    black_box(visit_and_count(&index, &cell))
}

library_benchmark_group!(
    name = shape_index_region_benchmarks;
    benchmarks =
        visit_intersecting_parent_cell,
        visit_intersecting_index_cells,
        visit_intersecting_descendant_cells,
        visit_intersecting_200_polygons
);

main!(library_benchmark_groups = shape_index_region_benchmarks);
