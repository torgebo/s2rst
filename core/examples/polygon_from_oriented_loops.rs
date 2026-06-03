// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Building polygons from oriented loops.
//!
//! Ported from Go `ExamplePolygonFromOrientedLoops`. Demonstrates how
//! `Polygon::from_oriented_loops` automatically determines which loops
//! are shells and which are holes based on their vertex orientation.
//!
//! Run with: `cargo run --example polygon_from_oriented_loops`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::{LatLng, Loop, Polygon};

fn main() {
    // Three loops in WGS84 (GeoJSON-style: [lng, lat]).
    // Loop 1 and 2 are counter-clockwise (shells).
    // Loop 3 is clockwise (hole inside loop 2).

    // Remote island.
    let l1 = &[(102.0, 2.0), (103.0, 2.0), (103.0, 3.0), (102.0, 3.0)];
    // Larger region.
    let l2 = &[(100.0, 0.0), (101.0, 0.0), (101.0, 1.0), (100.0, 1.0)];
    // Hole within l2 (clockwise).
    let l3 = &[(100.2, 0.2), (100.2, 0.8), (100.8, 0.8), (100.8, 0.2)];

    let to_loop = |pts: &[(f64, f64)]| -> Loop {
        let points: Vec<_> = pts
            .iter()
            .map(|&(lng, lat)| LatLng::from_degrees(lat, lng).to_point())
            .collect();
        Loop::new(points)
    };

    // Combine all loops into a single polygon.
    let p = Polygon::from_oriented_loops(vec![to_loop(l1), to_loop(l2), to_loop(l3)]);

    for (i, loop_) in p.loops().iter().enumerate() {
        println!("loop {i} is hole: {}", loop_.is_hole());
    }
    println!("Combined area: {:.7}", p.area());

    // Verify: area = l1 + l2 - inv(l3).
    let p12 = Polygon::from_oriented_loops(vec![to_loop(l1), to_loop(l2)]);
    let mut p3 = Polygon::from_oriented_loops(vec![to_loop(l3)]);
    p3.invert();
    println!(
        "l1+l2 = {:.7}, inv(l3) = {:.7}; l1+l2 - inv(l3) = {:.7}",
        p12.area(),
        p3.area(),
        p12.area() - p3.area(),
    );

    // Expected output (matches Go):
    // loop 0 is hole: false
    // loop 1 is hole: false
    // loop 2 is hole: true
    // Combined area: 0.0004993
    // l1+l2 = 0.0006089, inv(l3) = 0.0001097; l1+l2 - inv(l3) = 0.0004993
}
