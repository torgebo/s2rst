// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! `S2LatLngRect`: distance from a rectangle to a point.
//!
//! Ported from Go `ExampleRect_DistanceToLatLng`.
//!
//! Run with: `cargo run --example rect_distance`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::{LatLng, Rect};

fn main() {
    // Build a rectangle from (-1,-1) to (1,1) in degrees.
    let r = Rect::from_lat_lng(LatLng::from_degrees(-1.0, -1.0))
        .add_point(LatLng::from_degrees(1.0, 1.0));

    let print_dist = |lat: f64, lng: f64| {
        let d = r.get_distance_to_latlng(LatLng::from_degrees(lat, lng));
        println!("{:.6}", d.degrees());
    };

    println!("Distances next to the rectangle.");
    print_dist(-2.0, 0.0);
    print_dist(0.0, -2.0);
    print_dist(2.0, 0.0);
    print_dist(0.0, 2.0);

    println!("Distances beyond the corners of the rectangle.");
    print_dist(-2.0, -2.0);
    print_dist(-2.0, 2.0);
    print_dist(2.0, 2.0);
    print_dist(2.0, -2.0);

    println!("Distance within the rectangle.");
    print_dist(0.0, 0.0);
    print_dist(0.5, 0.0);
    print_dist(0.0, 0.5);
    print_dist(-0.5, 0.0);
    print_dist(0.0, -0.5);

    // Expected output (matches Go):
    // Distances next to the rectangle.
    // 1.000000
    // 1.000000
    // 1.000000
    // 1.000000
    // Distances beyond the corners of the rectangle.
    // 1.413962
    // 1.413962
    // 1.413962
    // 1.413962
    // Distance within the rectangle.
    // 0.000000
    // 0.000000
    // 0.000000
    // 0.000000
    // 0.000000
}
