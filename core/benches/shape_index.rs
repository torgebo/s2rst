// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{CellId, LatLng, Loop, Point, Polygon};

// ─── Helpers ────────────────────────────────────────────────────────────

#[inline(never)]
fn make_zigzag_points(n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let lat = 45.0 + 10.0 * t;
            let lng = -120.0 + if i % 2 == 0 { 0.0 } else { 0.5 };
            LatLng::from_degrees(lat, lng).to_point()
        })
        .collect()
}

#[inline(never)]
fn make_regular_polygon(n: usize) -> Polygon {
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
        let lat = 10.0 * angle.cos();
        let lng = 10.0 * angle.sin();
        pts.push(LatLng::from_degrees(lat, lng).to_point());
    }
    Polygon::from_loops(vec![Loop::new(pts)])
}

#[inline(never)]
fn make_scattered_polylines(n: usize) -> Vec<Polyline> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            let lat = -60.0 + 120.0 * t;
            let lng = -160.0 + 320.0 * ((i * 137) % n) as f64 / n as f64;
            let p0 = LatLng::from_degrees(lat, lng).to_point();
            let p1 = LatLng::from_degrees(lat + 0.01, lng + 0.01).to_point();
            Polyline::new(vec![p0, p1])
        })
        .collect()
}

// ─── Build: single polyline ────────────────────────────────────────────

#[library_benchmark]
fn build_polyline_100_edges() {
    let pts = make_zigzag_points(101);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(pts)));
    idx.build();
}

#[library_benchmark]
fn build_polyline_1000_edges() {
    let pts = make_zigzag_points(1001);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(pts)));
    idx.build();
}

#[library_benchmark]
fn build_polyline_10000_edges() {
    let pts = make_zigzag_points(10001);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(pts)));
    idx.build();
}

// ─── Build: polygon ────────────────────────────────────────────────────

#[library_benchmark]
fn build_polygon_64_edges() {
    let polygon = make_regular_polygon(64);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
}

#[library_benchmark]
fn build_polygon_256_edges() {
    let polygon = make_regular_polygon(256);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
}

#[library_benchmark]
fn build_polygon_1024_edges() {
    let polygon = make_regular_polygon(1024);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
}

// ─── Build: many shapes ────────────────────────────────────────────────

#[library_benchmark]
fn build_100_polyline_shapes() {
    let polylines = make_scattered_polylines(100);
    let mut idx = ShapeIndex::new();
    for pl in polylines {
        idx.add(Box::new(pl));
    }
    idx.build();
}

#[library_benchmark]
fn build_1000_polyline_shapes() {
    let polylines = make_scattered_polylines(1000);
    let mut idx = ShapeIndex::new();
    for pl in polylines {
        idx.add(Box::new(pl));
    }
    idx.build();
}

#[library_benchmark]
fn build_10000_polyline_shapes() {
    let polylines = make_scattered_polylines(10000);
    let mut idx = ShapeIndex::new();
    for pl in polylines {
        idx.add(Box::new(pl));
    }
    idx.build();
}

// ─── Iterator / cell lookup ────────────────────────────────────────────

#[library_benchmark]
fn iterate_cells_1000_edges() -> usize {
    let pts = make_zigzag_points(1001);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(pts)));
    idx.build();
    let mut count = 0;
    let mut it = idx.iter();
    while !it.done() {
        count += 1;
        it.next();
    }
    black_box(count)
}

#[library_benchmark]
fn cell_lookup_1000_edges() -> bool {
    let pts = make_zigzag_points(1001);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(Polyline::new(pts)));
    idx.build();
    let target = CellId::from_lat_lng(&LatLng::from_degrees(50.0, -119.75));
    // Look up a specific cell.
    black_box(idx.cell(target).is_some())
}

// ─── Num edges (counting) ──────────────────────────────────────────────

#[library_benchmark]
fn num_edges_10000_shapes() -> usize {
    let polylines = make_scattered_polylines(10000);
    let mut idx = ShapeIndex::new();
    for pl in polylines {
        idx.add(Box::new(pl));
    }
    idx.build();
    black_box(idx.num_edges())
}

library_benchmark_group!(
    name = build_polyline;
    benchmarks =
        build_polyline_100_edges,
        build_polyline_1000_edges,
        build_polyline_10000_edges
);

library_benchmark_group!(
    name = build_polygon;
    benchmarks =
        build_polygon_64_edges,
        build_polygon_256_edges,
        build_polygon_1024_edges
);

library_benchmark_group!(
    name = build_many_shapes;
    benchmarks =
        build_100_polyline_shapes,
        build_1000_polyline_shapes,
        build_10000_polyline_shapes
);

// ─── Build: fractal polygon (C++: BM_BigFractalConstruction) ──────────

#[library_benchmark]
fn build_fractal_1024_edges() {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(1024);
    let loop_ = fractal.make_loop_at(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    let polygon = Polygon::from_loops(vec![loop_]);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
}

#[library_benchmark]
fn build_fractal_4096_edges() {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(4096);
    let loop_ = fractal.make_loop_at(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    let polygon = Polygon::from_loops(vec![loop_]);
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(polygon));
    idx.build();
}

// C++: BM_Construction (4 small loops)
#[library_benchmark]
fn build_4_small_loops() {
    let mut idx = ShapeIndex::new();
    for i in 0..4 {
        let center = LatLng::from_degrees(f64::from(i) * 5.0, 0.0).to_point();
        let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), 100);
        idx.add(Box::new(Polygon::from_loops(vec![loop_])));
    }
    idx.build();
}

// C++: BM_Construction (16 small loops)
#[library_benchmark]
fn build_16_small_loops() {
    let mut idx = ShapeIndex::new();
    for i in 0..16 {
        let lat = f64::from(i / 4) * 3.0;
        let lng = f64::from(i % 4) * 3.0;
        let center = LatLng::from_degrees(lat, lng).to_point();
        let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), 100);
        idx.add(Box::new(Polygon::from_loops(vec![loop_])));
    }
    idx.build();
}

// Go: BenchmarkShapeIndexIteratorLocatePoint
#[library_benchmark]
fn locate_point_100_polylines() -> bool {
    let mut idx = ShapeIndex::new();
    for i in 0..100 {
        let pts: Vec<Point> = (0..100)
            .map(|j| {
                let t = (i * 100 + j) as f64 / 10000.0;
                LatLng::from_degrees(-80.0 + 160.0 * t, -170.0 + 340.0 * (t * 7.3).fract())
                    .to_point()
            })
            .collect();
        idx.add(Box::new(Polyline::new(pts)));
    }
    idx.build();
    let target = LatLng::from_degrees(0.0, 0.0).to_point();
    let mut it = idx.iter();
    black_box(it.locate_point(target))
}

// C++: BM_IncrementalAddLoopGrid — incremental add (no release, just add + rebuild)
#[library_benchmark]
fn incremental_add_10_loops() {
    let mut idx = ShapeIndex::new();
    for i in 0..10 {
        let center = LatLng::from_degrees(f64::from(i) * 3.0, 0.0).to_point();
        let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), 100);
        idx.add(Box::new(Polygon::from_loops(vec![loop_])));
        idx.build();
    }
}

#[library_benchmark]
fn incremental_add_100_loops() {
    let mut idx = ShapeIndex::new();
    for i in 0..100 {
        let lat = f64::from(i / 10) * 3.0;
        let lng = f64::from(i % 10) * 3.0;
        let center = LatLng::from_degrees(lat, lng).to_point();
        let loop_ = Loop::make_regular(center, Angle::from_degrees(0.5), 50);
        idx.add(Box::new(Polygon::from_loops(vec![loop_])));
    }
    // Single build at the end (measuring add + build cost).
    idx.build();
}

library_benchmark_group!(
    name = iteration;
    benchmarks =
        iterate_cells_1000_edges,
        cell_lookup_1000_edges,
        num_edges_10000_shapes,
        locate_point_100_polylines,
        incremental_add_10_loops,
        incremental_add_100_loops
);

library_benchmark_group!(
    name = fractal_construction;
    benchmarks =
        build_fractal_1024_edges,
        build_fractal_4096_edges,
        build_4_small_loops,
        build_16_small_loops
);

main!(
    library_benchmark_groups = build_polyline,
    build_polygon,
    build_many_shapes,
    iteration,
    fractal_construction
);
