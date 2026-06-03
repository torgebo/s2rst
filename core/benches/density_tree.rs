// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::LatLng;
use s2rst::s2::density_tree::{FeatureMap, S2DensityTree, VisitAction};
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::point_vector::PointVector;
use s2rst::s2::shape_index::ShapeIndex;

// ─── Helpers ────────────────────────────────────────────────────────────

/// Builds a density tree from `n` deterministically-placed point shapes.
#[inline(never)]
fn make_density_tree(n: usize, tree_size: i64, max_level: u8) -> S2DensityTree {
    let mut index = ShapeIndex::new();
    for i in 0..n {
        let t = i as f64 / n as f64;
        let lat = -80.0 + 160.0 * t;
        let lng = -170.0 + 340.0 * ((i * 137) % n) as f64 / n as f64;
        let p = LatLng::from_degrees(lat, lng).to_point();
        index.add(Box::new(PointVector::new(vec![p])));
    }
    index.build();
    let mut tree = S2DensityTree::new();
    tree.init_to_vertex_density(&index, tree_size, max_level)
        .unwrap();
    tree
}

// ─── BM_DecodeCellsByVisitor ────────────────────────────────────────────
// Corresponds to C++ BM_DecodeCellsByVisitor(Arg).

#[library_benchmark]
fn visit_cells_10_shapes() {
    let tree = make_density_tree(10, 100_000, 30);
    drop(black_box(tree.visit_cells(|_, _| VisitAction::EnterCell)));
}

#[library_benchmark]
fn visit_cells_100_shapes() {
    let tree = make_density_tree(100, 100_000, 30);
    drop(black_box(tree.visit_cells(|_, _| VisitAction::EnterCell)));
}

#[library_benchmark]
fn visit_cells_1000_shapes() {
    let tree = make_density_tree(1000, 100_000, 30);
    drop(black_box(tree.visit_cells(|_, _| VisitAction::EnterCell)));
}

#[library_benchmark]
fn visit_cells_10000_shapes() {
    let tree = make_density_tree(10000, 100_000, 30);
    drop(black_box(tree.visit_cells(|_, _| VisitAction::EnterCell)));
}

// ─── BM_InitToVertexDensity ─────────────────────────────────────────────
// Rust equivalent of BM_InitToFeatureDensity (vertex density variant).

#[library_benchmark]
fn init_vertex_density_100_shapes() {
    black_box(make_density_tree(100, 10_000, 15));
}

#[library_benchmark]
fn init_vertex_density_1000_shapes() {
    black_box(make_density_tree(1000, 10_000, 15));
}

#[library_benchmark]
fn init_vertex_density_1000_shapes_100k() {
    black_box(make_density_tree(1000, 100_000, 15));
}

// ─── Groups ─────────────────────────────────────────────────────────────

library_benchmark_group!(
    name = decode_cells_by_visitor;
    benchmarks =
        visit_cells_10_shapes,
        visit_cells_100_shapes,
        visit_cells_1000_shapes,
        visit_cells_10000_shapes
);

// ─── BM_InitToFeatureDensity ─────────────────────────────────────────────
// Corresponds to C++ BM_InitToFeatureDensity(num_shapes, tree_size_bytes).

#[inline(never)]
fn make_polygon_index(n: usize) -> ShapeIndex {
    let mut index = ShapeIndex::new();
    for i in 0..n {
        let t = i as f64 / n as f64;
        let lat = -80.0 + 160.0 * t;
        let lng = -170.0 + 340.0 * ((i * 137) % n) as f64 / n as f64;
        // Small 5-vertex polygon (approximating make_regular_points).
        let pts: Vec<_> = (0..5)
            .map(|k| {
                let angle = k as f64 * std::f64::consts::TAU / 5.0;
                let dlat = 0.01 * angle.cos();
                let dlng = 0.01 * angle.sin();
                LatLng::from_degrees(lat + dlat, lng + dlng).to_point()
            })
            .collect();
        index.add(Box::new(LaxPolygon::from_loops_owned(vec![pts])));
    }
    index.build();
    index
}

#[library_benchmark]
fn init_feature_density_100_shapes() {
    let index = make_polygon_index(100);
    let feature_map = FeatureMap::from_shapes(
        index.num_shape_ids(),
        (0..index.num_shape_ids() as i32).map(|id| (id, id, 1_i64)),
    );
    let mut tree = S2DensityTree::new();
    drop(black_box(tree.init_to_feature_density(
        &index,
        &feature_map,
        10_000,
        15,
    )));
}

#[library_benchmark]
fn init_feature_density_1000_shapes() {
    let index = make_polygon_index(1000);
    let feature_map = FeatureMap::from_shapes(
        index.num_shape_ids(),
        (0..index.num_shape_ids() as i32).map(|id| (id, id, 1_i64)),
    );
    let mut tree = S2DensityTree::new();
    drop(black_box(tree.init_to_feature_density(
        &index,
        &feature_map,
        100_000,
        15,
    )));
}

library_benchmark_group!(
    name = init_density;
    benchmarks =
        init_vertex_density_100_shapes,
        init_vertex_density_1000_shapes,
        init_vertex_density_1000_shapes_100k
);

library_benchmark_group!(
    name = init_feature_density;
    benchmarks =
        init_feature_density_100_shapes,
        init_feature_density_1000_shapes
);

main!(
    library_benchmark_groups = decode_cells_by_visitor,
    init_density,
    init_feature_density
);
