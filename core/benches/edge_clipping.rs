// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2edge_clipping_test.cc`
//! `BM_EdgeClippingDisjoint`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::edge_clipping::clip_to_face;
use s2rst::s2::{CellId, LatLng};

// C++: BM_EdgeClippingDisjoint — clip edges against a cell face.
// Uses deterministic edges that mostly miss the target face.
#[library_benchmark]
fn edge_clipping_disjoint_100() -> usize {
    let cell = CellId::from_token("89c25c1");
    let face = cell.face();
    let mut count = 0usize;
    // Generate 100 deterministic edge pairs spread across the sphere.
    for i in 0..100_u32 {
        let t = f64::from(i) / 100.0;
        let lat1 = -90.0 + 180.0 * t;
        let lng1 = -180.0 + 360.0 * (t * 7.3).fract();
        let lat2 = lat1 + 5.0;
        let lng2 = lng1 + 5.0;
        let a = LatLng::from_degrees(black_box(lat1), black_box(lng1)).to_point();
        let b = LatLng::from_degrees(black_box(lat2), black_box(lng2)).to_point();
        if clip_to_face(a, b, face).is_some() {
            count += 1;
        }
    }
    black_box(count)
}

// Clip edges that intersect the face.
#[library_benchmark]
fn edge_clipping_intersecting_100() -> usize {
    let mut count = 0usize;
    // Edges centered on face 0 (near 0,0 in lat/lng).
    for i in 0..100_u32 {
        let t = f64::from(i) / 100.0;
        let lat = -10.0 + 20.0 * t;
        let lng = -10.0 + 20.0 * (t * 3.7).fract();
        let a = LatLng::from_degrees(black_box(lat), black_box(lng)).to_point();
        let b = LatLng::from_degrees(black_box(lat + 1.0), black_box(lng + 1.0)).to_point();
        if clip_to_face(a, b, s2rst::s2::Face::F0).is_some() {
            count += 1;
        }
    }
    black_box(count)
}

library_benchmark_group!(
    name = edge_clipping_benchmarks;
    benchmarks =
        edge_clipping_disjoint_100,
        edge_clipping_intersecting_100
);

main!(library_benchmark_groups = edge_clipping_benchmarks);
