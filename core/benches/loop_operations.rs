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
use s2rst::s2::Region;
use s2rst::s2::{LatLng, Loop, Point};

#[inline(never)]
fn make_loop(center: Point, radius_deg: f64, n: usize) -> Loop {
    Loop::make_regular(center, Angle::from_degrees(radius_deg), n)
}

fn center1() -> Point {
    LatLng::from_degrees(0.0, 0.0).to_point()
}
fn center2() -> Point {
    LatLng::from_degrees(0.5, 0.0).to_point()
}

// ─── Loop vs Loop containment (C++: BM_ContainsContains) ───────────────

#[library_benchmark]
fn loop_contains_nested_64v() -> bool {
    let outer = make_loop(center1(), 2.0, 64);
    let inner = make_loop(center1(), 1.0, 64);
    black_box(outer.contains_loop(&inner))
}

#[library_benchmark]
fn loop_contains_nested_256v() -> bool {
    let outer = make_loop(center1(), 2.0, 256);
    let inner = make_loop(center1(), 1.0, 256);
    black_box(outer.contains_loop(&inner))
}

// ─── Loop vs Loop disjoint (C++: BM_ContainsDisjoint) ──────────────────

#[library_benchmark]
fn loop_contains_disjoint_64v() -> bool {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(LatLng::from_degrees(10.0, 10.0).to_point(), 1.0, 64);
    black_box(a.contains_loop(&b))
}

// ─── Loop vs Loop intersects (C++: BM_IntersectsContains) ──────────────

#[library_benchmark]
fn loop_intersects_nested_64v() -> bool {
    let outer = make_loop(center1(), 2.0, 64);
    let inner = make_loop(center1(), 1.0, 64);
    black_box(outer.intersects_loop(&inner))
}

#[library_benchmark]
fn loop_intersects_crossing_64v() -> bool {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(center2(), 1.0, 64);
    black_box(a.intersects_loop(&b))
}

#[library_benchmark]
fn loop_intersects_disjoint_64v() -> bool {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(LatLng::from_degrees(10.0, 10.0).to_point(), 1.0, 64);
    black_box(a.intersects_loop(&b))
}

// ─── Loop area/centroid (C++: BM_GetArea) ──────────────────────────────

#[library_benchmark]
fn loop_get_area_64v() -> f64 {
    let loop_ = make_loop(center1(), 1.0, 64);
    black_box(loop_.area())
}

#[library_benchmark]
fn loop_get_area_256v() -> f64 {
    let loop_ = make_loop(center1(), 1.0, 256);
    black_box(loop_.area())
}

#[library_benchmark]
fn loop_get_area_1024v() -> f64 {
    let loop_ = make_loop(center1(), 1.0, 1024);
    black_box(loop_.area())
}

// ─── Loop contains point (C++: BM_ContainsPoint) ──────────────────────

#[library_benchmark]
fn loop_contains_point_64v() -> bool {
    let loop_ = make_loop(center1(), 1.0, 64);
    let pt = LatLng::from_degrees(0.5, 0.0).to_point();
    black_box(loop_.contains_point(&pt))
}

#[library_benchmark]
fn loop_contains_point_256v() -> bool {
    let loop_ = make_loop(center1(), 1.0, 256);
    let pt = LatLng::from_degrees(0.5, 0.0).to_point();
    black_box(loop_.contains_point(&pt))
}

// ─── Loop construction (C++: BM_Constructor) ───────────────────────────

#[library_benchmark]
fn loop_constructor_64v() -> Loop {
    black_box(make_loop(center1(), 1.0, 64))
}

#[library_benchmark]
fn loop_constructor_256v() -> Loop {
    black_box(make_loop(center1(), 1.0, 256))
}

#[library_benchmark]
fn loop_constructor_1024v() -> Loop {
    black_box(make_loop(center1(), 1.0, 1024))
}

// ─── Loop validate (C++: BM_IsValid) ──────────────────────────────────

#[library_benchmark]
fn loop_validate_64v() -> Result<(), String> {
    let loop_ = make_loop(center1(), 1.0, 64);
    black_box(loop_.validate())
}

#[library_benchmark]
fn loop_validate_256v() -> Result<(), String> {
    let loop_ = make_loop(center1(), 1.0, 256);
    black_box(loop_.validate())
}

// ─── Loop compare_boundary (C++: BM_CompareBoundary*) ─────────────────

#[library_benchmark]
fn loop_compare_boundary_contains_64v() -> i32 {
    let outer = make_loop(center1(), 2.0, 64);
    let inner = make_loop(center1(), 1.0, 64);
    black_box(outer.compare_boundary(&inner))
}

#[library_benchmark]
fn loop_compare_boundary_crosses_64v() -> i32 {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(center2(), 1.0, 64);
    black_box(a.compare_boundary(&b))
}

#[library_benchmark]
fn loop_compare_boundary_disjoint_64v() -> i32 {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(LatLng::from_degrees(10.0, 10.0).to_point(), 1.0, 64);
    black_box(a.compare_boundary(&b))
}

// ─── Loop contains_nested (C++: BM_ContainsNested) ────────────────────

#[library_benchmark]
fn loop_contains_nested_boundary_64v() -> bool {
    let outer = make_loop(center1(), 2.0, 64);
    let inner = make_loop(center1(), 1.0, 64);
    black_box(outer.contains_nested(&inner))
}

// ─── Loop intersects crosses (C++: BM_IntersectsCrosses) ──────────────

#[library_benchmark]
fn loop_intersects_crosses_64v() -> bool {
    let a = make_loop(center1(), 1.0, 64);
    let b = make_loop(center2(), 1.0, 64);
    black_box(a.intersects_loop(&b))
}

library_benchmark_group!(
    name = loop_operations_benchmarks;
    benchmarks =
        loop_contains_nested_64v,
        loop_contains_nested_256v,
        loop_contains_disjoint_64v,
        loop_intersects_nested_64v,
        loop_intersects_crossing_64v,
        loop_intersects_disjoint_64v,
        loop_get_area_64v,
        loop_get_area_256v,
        loop_get_area_1024v,
        loop_contains_point_64v,
        loop_contains_point_256v,
        loop_constructor_64v,
        loop_constructor_256v,
        loop_constructor_1024v,
        loop_validate_64v,
        loop_validate_256v,
        loop_compare_boundary_contains_64v,
        loop_compare_boundary_crosses_64v,
        loop_compare_boundary_disjoint_64v,
        loop_contains_nested_boundary_64v,
        loop_intersects_crosses_64v
);

main!(library_benchmark_groups = loop_operations_benchmarks);
