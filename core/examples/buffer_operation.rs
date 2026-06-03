// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Buffering S2 geometries: expand or contract points, polylines, and polygons.
//!
//! The buffer operation dilates or erodes geometry by a given radius.
//! Internally it uses `S2BooleanOperation` + `S2WindingOperation`.
//!
//! Note: extracting the buffered result currently requires the internal
//! layer/builder pattern. This example shows how to set up and run the
//! operation; in production you would integrate with `S2Builder` layers.
//!
//! Run with: `cargo run --example buffer_operation`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s1;
use s2rst::s2::LatLng;
use s2rst::s2::buffer_operation::{BufferOptions, S2BufferOperation};
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::earth;

fn main() {
    // ── Buffer a point → disc ──────────────────────────────────────────
    println!("=== Buffer a point (Berlin) by 50 km ===\n");
    let berlin = LatLng::from_degrees(52.52, 13.405).to_point();

    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(earth::km_to_angle(50.0));
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_point(berlin);
    op.build().expect("buffer failed");
    println!("  Buffer of a point by 50 km: OK");
    println!(
        "  Expected area ≈ π × 50² ≈ {:.0} km²",
        std::f64::consts::PI * 50.0 * 50.0
    );

    // ── Buffer a polyline → corridor ───────────────────────────────────
    println!("\n=== Buffer a polyline (Berlin → Munich) by 20 km ===\n");
    let munich = LatLng::from_degrees(48.1351, 11.582).to_point();

    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(earth::km_to_angle(20.0));
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_polyline(&[berlin, munich]);
    op.build().expect("buffer failed");
    println!("  Buffer of Berlin→Munich by 20 km: OK");

    // ── Buffer a polygon → expanded polygon ────────────────────────────
    println!("\n=== Buffer a triangle (Bermuda Triangle) by 100 km ===\n");
    let miami = LatLng::from_degrees(25.7617, -80.1918).to_point();
    let bermuda_pt = LatLng::from_degrees(32.3214, -64.7574).to_point();
    let san_juan = LatLng::from_degrees(18.4655, -66.1057).to_point();

    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(earth::km_to_angle(100.0));
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_loop(&[miami, bermuda_pt, san_juan]);
    op.build().expect("buffer failed");
    println!("  Buffer of Bermuda Triangle by 100 km: OK");

    // ── Negative buffer (shrink) ───────────────────────────────────────
    println!("\n=== Negative buffer (shrink the Bermuda Triangle by 50 km) ===\n");
    let layer = S2PolygonLayer::new();
    let opts = BufferOptions::new(s1::Angle::from_degrees(-0.5)); // ~55 km
    let mut op = S2BufferOperation::new(Box::new(layer), opts);
    op.add_loop(&[miami, bermuda_pt, san_juan]);
    op.build().expect("buffer failed");
    println!("  Negative buffer of Bermuda Triangle: OK");

    println!("\n=== Buffer options ===\n");
    let mut opts = BufferOptions::new(earth::km_to_angle(10.0));
    println!(
        "  Default buffer radius: {:.4}°",
        opts.buffer_radius().degrees()
    );
    opts.set_error_fraction(0.01);
    opts.set_circle_segments(32.0);
    println!("  Error fraction: 1%");
    println!("  Circle segments: 32");
    println!("  Buffer operations support points, polylines, loops, and arbitrary shapes.");
}
