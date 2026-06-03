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
use s2rst::s2::closest_edge_query::{
    CellTarget, ClosestEdgeQuery, EdgeTarget, Options, PointTarget, Result, ShapeIndexTarget,
};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{Cell, CellId, LatLng};

// ─── Geometry helpers ───────────────────────────────────────────────────

/// Build a "fractal-like" index with `n` polyline edges arranged in a zigzag
/// pattern within a cap.  Deterministic for a given `n`.
#[inline(never)]
fn make_zigzag_index(n: usize) -> ShapeIndex {
    let mut points = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let lat = 45.0 + 10.0 * t;
        let lng = -120.0 + if i % 2 == 0 { 0.0 } else { 0.5 };
        points.push(LatLng::from_degrees(lat, lng).to_point());
    }
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(points)));
    idx.build();
    idx
}

/// Build an index with `n_shapes` separate 2-vertex polylines spread out
/// across a region, simulating a multi-shape point-cloud-like index.
#[inline(never)]
fn make_scattered_index(n_shapes: usize) -> ShapeIndex {
    let mut idx = ShapeIndex::new();
    for i in 0..n_shapes {
        let t = i as f64 / n_shapes as f64;
        let lat = -60.0 + 120.0 * t;
        let lng = -160.0 + 320.0 * ((i * 137) % n_shapes) as f64 / n_shapes as f64;
        let p0 = LatLng::from_degrees(lat, lng).to_point();
        let p1 = LatLng::from_degrees(lat + 0.01, lng + 0.01).to_point();
        idx.add(Box::new(Polyline::new(vec![p0, p1])));
    }
    idx.build();
    idx
}

// ─── BM_FindClosest: Point target, varying edge counts ─────────────────
// Corresponds to C++ BM_FindClosest<Fractal> with different edge counts.

#[library_benchmark]
fn find_closest_point_12_edges() -> Result {
    let idx = make_zigzag_index(12);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_point_48_edges() -> Result {
    let idx = make_zigzag_index(48);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_point_768_edges() -> Result {
    let idx = make_zigzag_index(768);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_point_12288_edges() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_point_49152_edges() -> Result {
    let idx = make_zigzag_index(49152);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_FindClosest with brute force (for comparison) ──────────────────

#[library_benchmark]
fn find_closest_point_768_brute() -> Result {
    let idx = make_zigzag_index(768);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 1,
        use_brute_force: true,
        ..Options::default()
    };
    black_box(query.find_closest_edge_with_options(&target, &opts))
}

#[library_benchmark]
fn find_closest_point_12288_brute() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 1,
        use_brute_force: true,
        ..Options::default()
    };
    black_box(query.find_closest_edge_with_options(&target, &opts))
}

// ─── BM_FindClosestInterior: include polygon interiors ─────────────────

#[library_benchmark]
fn find_closest_interior_12288_edges() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 1,
        include_interiors: true,
        ..Options::default()
    };
    black_box(query.find_closest_edge_with_options(&target, &opts))
}

// ─── BM_FindClosestMaxDist: with small distance limit ──────────────────
// Corresponds to C++ BM_FindClosestMaxDistPow10.

#[library_benchmark]
fn find_closest_max_dist_small() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 1,
        max_distance: ChordAngle::from_angle(Angle::from_degrees(0.01)),
        ..Options::default()
    };
    black_box(query.find_closest_edge_with_options(&target, &opts))
}

#[library_benchmark]
fn find_closest_max_dist_tiny() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 1,
        max_distance: ChordAngle::from_angle(Angle::from_degrees(0.000001)),
        ..Options::default()
    };
    black_box(query.find_closest_edge_with_options(&target, &opts))
}

// ─── BM_FindClosestNearVertex: query on a known vertex ─────────────────

#[library_benchmark]
fn find_closest_near_vertex_12288() -> Result {
    let idx = make_zigzag_index(12288);
    // Pick a point that is exactly a vertex of the polyline.
    let vertex = LatLng::from_degrees(50.0, -120.0).to_point();
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(vertex));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_FindClosestToEdge: Edge target ─────────────────────────────────

#[library_benchmark]
fn find_closest_to_edge_768() -> Result {
    let idx = make_zigzag_index(768);
    let query = ClosestEdgeQuery::new(&idx);
    let a = LatLng::from_degrees(50.0, -121.0).to_point();
    let b = LatLng::from_degrees(50.5, -119.0).to_point();
    let target = EdgeTarget::new(black_box(a), black_box(b));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_to_edge_12288() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let a = LatLng::from_degrees(50.0, -121.0).to_point();
    let b = LatLng::from_degrees(50.5, -119.0).to_point();
    let target = EdgeTarget::new(black_box(a), black_box(b));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_FindClosestToCell: Cell target ─────────────────────────────────

#[library_benchmark]
fn find_closest_to_cell_12288() -> Result {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let cell = Cell::from_cell_id(CellId::from_lat_lng(&LatLng::from_degrees(50.0, -119.5)));
    let target = CellTarget::new(black_box(cell));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_FindClosestToIndex: ShapeIndexTarget ───────────────────────────
// Corresponds to C++ BM_FindClosestToSameSizeAbuttingIndex.

#[library_benchmark]
fn find_closest_to_index_768() -> Result {
    let idx_a = make_zigzag_index(768);
    let idx_b = make_scattered_index(100);
    let query = ClosestEdgeQuery::new(&idx_a);
    let target = ShapeIndexTarget::new(black_box(&idx_b));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_to_index_12288() -> Result {
    let idx_a = make_zigzag_index(12288);
    let idx_b = make_scattered_index(100);
    let query = ClosestEdgeQuery::new(&idx_a);
    let target = ShapeIndexTarget::new(black_box(&idx_b));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_FindClosestEdges: top-k results with bounded k ─────────────────

#[library_benchmark]
fn find_closest_top5_12288() -> Vec<Result> {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 5,
        ..Options::default()
    };
    black_box(query.find_closest_edges(&target, &opts))
}

#[library_benchmark]
fn find_closest_top100_12288() -> Vec<Result> {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    let opts = Options {
        max_results: 100,
        ..Options::default()
    };
    black_box(query.find_closest_edges(&target, &opts))
}

// ─── BM with scattered multi-shape index ────────────────────────────────

#[library_benchmark]
fn find_closest_scattered_1000_shapes() -> Result {
    let idx = make_scattered_index(1000);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_edge(&target))
}

#[library_benchmark]
fn find_closest_scattered_10000_shapes() -> Result {
    let idx = make_scattered_index(10000);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    black_box(query.find_closest_edge(&target))
}

// ─── BM_IsDistanceLess: threshold queries ──────────────────────────────

#[library_benchmark]
fn is_distance_less_12288() -> bool {
    let idx = make_zigzag_index(12288);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(50.0, -119.5).to_point()));
    black_box(query.is_distance_less(&target, ChordAngle::from_degrees(1.0)))
}

// ─── BM with shape filter ──────────────────────────────────────────────

#[library_benchmark]
fn find_closest_filtered_1000_shapes() -> Vec<Result> {
    let idx = make_scattered_index(1000);
    let query = ClosestEdgeQuery::new(&idx);
    let target = PointTarget::new(black_box(LatLng::from_degrees(0.0, 0.0).to_point()));
    let opts = Options {
        max_results: 1,
        ..Options::default()
    };
    // Filter: only even-numbered shapes.
    black_box(query.find_closest_edges_filtered(&target, &opts, Some(&|id| id.0 % 2 == 0)))
}

library_benchmark_group!(
    name = find_closest_point;
    benchmarks =
        find_closest_point_12_edges,
        find_closest_point_48_edges,
        find_closest_point_768_edges,
        find_closest_point_12288_edges,
        find_closest_point_49152_edges
);

library_benchmark_group!(
    name = find_closest_brute_force;
    benchmarks =
        find_closest_point_768_brute,
        find_closest_point_12288_brute
);

library_benchmark_group!(
    name = find_closest_variants;
    benchmarks =
        find_closest_interior_12288_edges,
        find_closest_max_dist_small,
        find_closest_max_dist_tiny,
        find_closest_near_vertex_12288,
        is_distance_less_12288
);

library_benchmark_group!(
    name = find_closest_target_types;
    benchmarks =
        find_closest_to_edge_768,
        find_closest_to_edge_12288,
        find_closest_to_cell_12288,
        find_closest_to_index_768,
        find_closest_to_index_12288
);

library_benchmark_group!(
    name = find_closest_topk;
    benchmarks =
        find_closest_top5_12288,
        find_closest_top100_12288
);

library_benchmark_group!(
    name = find_closest_multi_shape;
    benchmarks =
        find_closest_scattered_1000_shapes,
        find_closest_scattered_10000_shapes,
        find_closest_filtered_1000_shapes
);

main!(
    library_benchmark_groups = find_closest_point,
    find_closest_brute_force,
    find_closest_variants,
    find_closest_target_types,
    find_closest_topk,
    find_closest_multi_shape
);
