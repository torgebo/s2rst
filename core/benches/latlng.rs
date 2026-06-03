// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1;
use s2rst::s2::{LatLng, Point};

#[library_benchmark]
fn bench_latlng_to_point() -> Point {
    // C++ uses E7(0x150bc888), E7(0x5099d63f) which are ~35.4deg and ~135.3deg.
    let ll = black_box(LatLng::from_degrees(35.4, 135.3));
    black_box(ll.to_point())
}

#[library_benchmark]
fn bench_latlng_from_point() -> LatLng {
    let p = black_box(Point::from_coords(1.0, 2.0, 3.0).normalize());
    black_box(LatLng::from_point(p))
}

#[library_benchmark]
fn bench_latlng_distance() -> s1::Angle {
    // C++ uses (25deg, -78deg) to (35deg, 56deg).
    let a = black_box(LatLng::from_degrees(25.0, -78.0));
    let b = black_box(LatLng::from_degrees(35.0, 56.0));
    black_box(a.get_distance(b))
}

library_benchmark_group!(
    name = latlng_benchmarks;
    benchmarks =
        bench_latlng_to_point,
        bench_latlng_from_point,
        bench_latlng_distance
);

main!(library_benchmark_groups = latlng_benchmarks);
