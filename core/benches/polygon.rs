// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Polygon benchmarks ported from C++: `s2polygon_test.cc`
//! `BM_ConstructorSingleLoop`, `BM_IsValidConcentricLoops`, `BM_ContainsPoint`*,
//! `BM_Contains`*, `BM_Intersects`*, `BM_Union/Intersect/Subtract`*
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::Region;
use s2rst::s2::builder::S2Error;
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::shape::Shape;
use s2rst::s2::{CellUnion, LatLng, Loop, Point, Polygon};

fn center() -> Point {
    LatLng::from_degrees(0.0, 0.0).to_point()
}

#[inline(never)]
fn make_regular_polygon(n: usize) -> Polygon {
    Polygon::from_loops(vec![Loop::make_regular(
        center(),
        Angle::from_degrees(1.0),
        n,
    )])
}

// Helper: make concentric loop polygon with `n_loops` loops, each `verts_per_loop` vertices.
#[inline(never)]
fn make_concentric_polygon(n_loops: usize, verts_per_loop: usize) -> Polygon {
    let loops: Vec<Loop> = (0..n_loops)
        .map(|i| {
            let radius = 0.1 + 0.8 * f64::from(i as i32) / f64::from(n_loops as i32).max(1.0);
            Loop::make_regular(center(), Angle::from_degrees(radius), verts_per_loop)
        })
        .collect();
    Polygon::from_loops(loops)
}

#[inline(never)]
fn make_fractal_polygon(num_edges: usize) -> Polygon {
    let mut fractal = S2Fractal::new(42);
    fractal.level_for_approx_max_edges(num_edges as i32);
    let loop_ = fractal.make_loop_at(center(), Angle::from_degrees(5.0));
    Polygon::from_loops(vec![loop_])
}

// ─── Constructor (C++: BM_ConstructorSingleLoop) ──────────────────────

#[library_benchmark]
fn polygon_constructor_8v() -> Polygon {
    black_box(make_regular_polygon(8))
}

#[library_benchmark]
fn polygon_constructor_1024v() -> Polygon {
    black_box(make_regular_polygon(1024))
}

#[library_benchmark]
fn polygon_constructor_fractal_1024() -> Polygon {
    black_box(make_fractal_polygon(1024))
}

// ─── Validate (C++: BM_IsValidConcentricLoops) ───────────────────────

#[library_benchmark]
fn polygon_validate_8v() -> Option<S2Error> {
    let p = make_regular_polygon(8);
    black_box(p.find_validation_error())
}

#[library_benchmark]
fn polygon_validate_1024v() -> Option<S2Error> {
    let p = make_regular_polygon(1024);
    black_box(p.find_validation_error())
}

#[library_benchmark]
fn polygon_validate_concentric_4x256() -> Option<S2Error> {
    let p = make_concentric_polygon(4, 256);
    black_box(p.find_validation_error())
}

// ─── ContainsPoint (C++: BM_ContainsPointLoopGrid, BM_ContainsPointNestedFractals) ──

#[library_benchmark]
fn polygon_contains_point_64v() -> bool {
    let p = make_regular_polygon(64);
    let pt = LatLng::from_degrees(0.5, 0.0).to_point();
    black_box(p.contains_point(&pt))
}

#[library_benchmark]
fn polygon_contains_point_1024v() -> bool {
    let p = make_regular_polygon(1024);
    let pt = LatLng::from_degrees(0.5, 0.0).to_point();
    black_box(p.contains_point(&pt))
}

#[library_benchmark]
fn polygon_contains_point_fractal_1024() -> bool {
    let p = make_fractal_polygon(1024);
    let pt = center();
    black_box(p.contains_point(&pt))
}

// ─── Contains/Intersects polygon (C++: BM_ContainsContains, BM_IntersectsContains, etc.) ──

#[library_benchmark]
fn polygon_contains_contained_64v() -> bool {
    let outer = make_regular_polygon(64);
    let inner = Polygon::from_loops(vec![Loop::make_regular(
        center(),
        Angle::from_degrees(0.5),
        64,
    )]);
    black_box(outer.approx_contains(&inner, Angle::from_degrees(1e-10)))
}

#[library_benchmark]
fn polygon_intersects_crossing_64v() -> bool {
    let a = make_regular_polygon(64);
    let b = Polygon::from_loops(vec![Loop::make_regular(
        LatLng::from_degrees(0.5, 0.0).to_point(),
        Angle::from_degrees(1.0),
        64,
    )]);
    black_box(a.intersects_polygon(&b))
}

#[library_benchmark]
fn polygon_intersects_disjoint_64v() -> bool {
    let a = make_regular_polygon(64);
    let b = Polygon::from_loops(vec![Loop::make_regular(
        LatLng::from_degrees(10.0, 10.0).to_point(),
        Angle::from_degrees(1.0),
        64,
    )]);
    black_box(a.intersects_polygon(&b))
}

// ─── Covering (C++: BM_FractalCovering, BM_AnnulusCovering) ──────────

#[library_benchmark]
fn polygon_covering_64v_8cells() -> CellUnion {
    let p = make_regular_polygon(64);
    let coverer = RegionCoverer::new().max_cells(8);
    black_box(coverer.covering(&p))
}

#[library_benchmark]
fn polygon_covering_fractal_1024_8cells() -> CellUnion {
    let p = make_fractal_polygon(1024);
    let coverer = RegionCoverer::new().max_cells(8);
    black_box(coverer.covering(&p))
}

// ─── Union/Intersection/Difference (C++: BM_Union*, BM_Intersect*, BM_Subtract*) ──

#[library_benchmark]
fn polygon_union_64v() -> Polygon {
    let mut a = make_regular_polygon(64);
    let mut b = Polygon::from_loops(vec![Loop::make_regular(
        LatLng::from_degrees(0.5, 0.0).to_point(),
        Angle::from_degrees(1.0),
        64,
    )]);
    black_box(Polygon::union(&mut a, &mut b))
}

#[library_benchmark]
fn polygon_intersection_64v() -> Polygon {
    let mut a = make_regular_polygon(64);
    let mut b = Polygon::from_loops(vec![Loop::make_regular(
        LatLng::from_degrees(0.5, 0.0).to_point(),
        Angle::from_degrees(1.0),
        64,
    )]);
    black_box(Polygon::intersection(&mut a, &mut b))
}

#[library_benchmark]
fn polygon_difference_64v() -> Polygon {
    let mut a = make_regular_polygon(64);
    let mut b = Polygon::from_loops(vec![Loop::make_regular(
        LatLng::from_degrees(0.5, 0.0).to_point(),
        Angle::from_degrees(1.0),
        64,
    )]);
    black_box(Polygon::difference(&mut a, &mut b))
}

#[library_benchmark]
fn polygon_union_fractal_self() -> Polygon {
    let mut a = make_fractal_polygon(1024);
    let mut b = a.clone();
    black_box(Polygon::union(&mut a, &mut b))
}

// ─── Shape::get_edge (C++: BM_ShapeGetEdge) ──────────────────────────

#[library_benchmark]
fn polygon_shape_get_edge_64v() {
    let p = make_regular_polygon(64);
    for i in 0..p.num_edges() {
        let _ = black_box(p.edge(i));
    }
}

#[library_benchmark]
fn polygon_shape_get_edge_1024v() {
    let p = make_regular_polygon(1024);
    for i in 0..p.num_edges() {
        let _ = black_box(p.edge(i));
    }
}

// ─── Area (C++: BM_GetArea) ──────────────────────────────────────────

#[library_benchmark]
fn polygon_area_64v() -> f64 {
    let p = make_regular_polygon(64);
    black_box(p.area())
}

#[library_benchmark]
fn polygon_area_1024v() -> f64 {
    let p = make_regular_polygon(1024);
    black_box(p.area())
}

library_benchmark_group!(
    name = polygon_benchmarks;
    benchmarks =
        polygon_constructor_8v,
        polygon_constructor_1024v,
        polygon_constructor_fractal_1024,
        polygon_validate_8v,
        polygon_validate_1024v,
        polygon_validate_concentric_4x256,
        polygon_contains_point_64v,
        polygon_contains_point_1024v,
        polygon_contains_point_fractal_1024,
        polygon_contains_contained_64v,
        polygon_intersects_crossing_64v,
        polygon_intersects_disjoint_64v,
        polygon_covering_64v_8cells,
        polygon_covering_fractal_1024_8cells,
        polygon_union_64v,
        polygon_intersection_64v,
        polygon_difference_64v,
        polygon_union_fractal_self,
        polygon_shape_get_edge_64v,
        polygon_shape_get_edge_1024v,
        polygon_area_64v,
        polygon_area_1024v
);

main!(library_benchmark_groups = polygon_benchmarks);
