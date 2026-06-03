// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2latlng_rect_bounder_test.cc`
//! `BM_AddPoints`, `BM_AddLatLngAsPoints`, `BM_AddLatLngAsLatLng`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::latlng_rect_bounder::LatLngRectBounder;
use s2rst::s2::{LatLng, Rect};

// Pre-computed deterministic lat/lng pairs along a spiral.
#[inline(never)]
fn make_latlngs(n: usize) -> Vec<LatLng> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            LatLng::from_degrees(-90.0 + 180.0 * t, -180.0 + 360.0 * t * 3.7)
        })
        .collect()
}

// C++: BM_AddPoints (8 points)
#[library_benchmark]
fn add_points_8() -> Rect {
    let latlngs = make_latlngs(8);
    let points: Vec<_> = latlngs.iter().map(|ll| ll.to_point()).collect();
    let mut bounder = LatLngRectBounder::new();
    for p in &points {
        bounder.add_point(*p);
    }
    black_box(bounder.get_bound())
}

// C++: BM_AddPoints (1000 points)
#[library_benchmark]
fn add_points_1000() -> Rect {
    let latlngs = make_latlngs(1000);
    let points: Vec<_> = latlngs.iter().map(|ll| ll.to_point()).collect();
    let mut bounder = LatLngRectBounder::new();
    for p in &points {
        bounder.add_point(*p);
    }
    black_box(bounder.get_bound())
}

// C++: BM_AddLatLngAsPoints (convert LatLng → Point then add)
#[library_benchmark]
fn add_latlng_as_points_1000() -> Rect {
    let latlngs = make_latlngs(1000);
    let mut bounder = LatLngRectBounder::new();
    for ll in &latlngs {
        bounder.add_point(ll.to_point());
    }
    black_box(bounder.get_bound())
}

// C++: BM_AddLatLngAsLatLng (use add_latlng directly)
#[library_benchmark]
fn add_latlng_as_latlng_1000() -> Rect {
    let latlngs = make_latlngs(1000);
    let mut bounder = LatLngRectBounder::new();
    for ll in &latlngs {
        bounder.add_latlng(*ll);
    }
    black_box(bounder.get_bound())
}

library_benchmark_group!(
    name = latlng_rect_bounder_benchmarks;
    benchmarks =
        add_points_8,
        add_points_1000,
        add_latlng_as_points_1000,
        add_latlng_as_latlng_1000
);

main!(library_benchmark_groups = latlng_rect_bounder_benchmarks);
