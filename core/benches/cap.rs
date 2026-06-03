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
use s2rst::s2::{Cap, LatLng, Rect};

// ─── Construction ───────────────────────────────────────────────────────

#[library_benchmark]
fn from_center_angle() -> Cap {
    let center = black_box(LatLng::from_degrees(47.6, -122.3).to_point());
    let angle = black_box(Angle::from_degrees(5.0));
    black_box(Cap::from_center_angle(center, angle))
}

#[library_benchmark]
fn from_center_chord_angle() -> Cap {
    let center = black_box(LatLng::from_degrees(47.6, -122.3).to_point());
    let radius = black_box(ChordAngle::from_degrees(5.0));
    black_box(Cap::from_center_chord_angle(center, radius))
}

#[library_benchmark]
fn from_center_area() -> Cap {
    let center = black_box(LatLng::from_degrees(47.6, -122.3).to_point());
    black_box(Cap::from_center_area(center, black_box(1.0)))
}

// ─── Queries ────────────────────────────────────────────────────────────

#[library_benchmark]
fn contains_point_inside() -> bool {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    let p = black_box(LatLng::from_degrees(5.0, 5.0).to_point());
    black_box(cap.contains_point(p))
}

#[library_benchmark]
fn contains_point_outside() -> bool {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    let p = black_box(LatLng::from_degrees(50.0, 50.0).to_point());
    black_box(cap.contains_point(p))
}

#[library_benchmark]
fn cap_contains_cap() -> bool {
    let big = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(20.0),
    );
    let small = Cap::from_center_angle(
        LatLng::from_degrees(5.0, 5.0).to_point(),
        Angle::from_degrees(2.0),
    );
    black_box(black_box(big).contains(black_box(small)))
}

#[library_benchmark]
fn cap_intersects_cap() -> bool {
    let a = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    let b = Cap::from_center_angle(
        LatLng::from_degrees(8.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    black_box(black_box(a).intersects(black_box(b)))
}

// ─── Properties ─────────────────────────────────────────────────────────

#[library_benchmark]
fn cap_area() -> f64 {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    black_box(black_box(cap).area())
}

#[library_benchmark]
fn cap_rect_bound() -> Rect {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(47.6, -122.3).to_point(),
        Angle::from_degrees(5.0),
    );
    black_box(black_box(cap).rect_bound())
}

#[library_benchmark]
fn cap_complement() -> Cap {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    black_box(black_box(cap).complement())
}

// ─── Expanded ───────────────────────────────────────────────────────────

#[library_benchmark]
fn cap_expanded() -> Cap {
    let cap = Cap::from_center_angle(
        LatLng::from_degrees(0.0, 0.0).to_point(),
        Angle::from_degrees(10.0),
    );
    black_box(black_box(cap).expanded(black_box(Angle::from_degrees(5.0))))
}

library_benchmark_group!(
    name = construction;
    benchmarks = from_center_angle, from_center_chord_angle, from_center_area
);

library_benchmark_group!(
    name = queries;
    benchmarks =
        contains_point_inside,
        contains_point_outside,
        cap_contains_cap,
        cap_intersects_cap
);

library_benchmark_group!(
    name = properties;
    benchmarks = cap_area, cap_rect_bound, cap_complement, cap_expanded
);

main!(library_benchmark_groups = construction, queries, properties);
