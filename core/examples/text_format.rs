// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Parsing and formatting S2 geometries with the text format module.
//!
//! The text format provides a human-readable representation of S2 geometries.
//! This is useful for debugging, test data, and quick prototyping.
//!
//! Run with: `cargo run --example text_format`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::shape::Shape;
use s2rst::s2::text_format;

fn main() {
    // ── Parse and display a polyline ───────────────────────────────────
    println!("=== Polyline ===\n");
    let polyline = text_format::make_polyline("0:0, 1:1, 2:0, 3:1");
    println!("  Vertices: {}", polyline.num_vertices());
    println!("  Text:     {}", text_format::polyline_to_string(&polyline));

    // ── Parse and display a loop ───────────────────────────────────────
    println!("\n=== Loop ===\n");
    let loop_ = text_format::make_loop("0:0, 0:10, 10:10, 10:0");
    println!("  Vertices: {}", loop_.num_vertices());
    println!("  Area:     {:.6} sr", loop_.area());
    println!("  Text:     {}", text_format::loop_to_string(&loop_));

    // ── Parse and display a polygon ────────────────────────────────────
    println!("\n=== Polygon (with hole) ===\n");
    let polygon = text_format::make_polygon("0:0, 0:10, 10:10, 10:0; 2:2, 2:8, 8:8, 8:2");
    println!("  Loops:    {}", polygon.num_loops());
    println!("  Area:     {:.6} sr", polygon.area());
    println!("  Text:     {}", text_format::polygon_to_string(&polygon));

    // ── Parse and display a lax polygon ────────────────────────────────
    println!("\n=== Lax polygon ===\n");
    let lax = text_format::make_lax_polygon("0:0, 0:5, 5:5, 5:0");
    println!("  Chains:   {}", lax.num_chains());
    println!("  Edges:    {}", lax.num_edges());
    println!("  Text:     {}", text_format::lax_polygon_to_string(&lax));

    // ── Parse and display a ShapeIndex (mixed geometry) ─────────────────
    println!("\n=== ShapeIndex (points | polylines | polygons) ===\n");
    let index =
        text_format::make_index("0:0 | 1:1 | 2:2 # 3:0, 3:5, 3:10 # 5:5, 5:10, 10:10, 10:5");
    println!("  Shapes:   {}", index.num_shape_ids());
    println!("  Edges:    {}", index.num_edges());
    println!("  Text:     {}", text_format::index_to_string(&index));

    // ── Parse a rect ───────────────────────────────────────────────────
    println!("\n=== Rect ===\n");
    let rect = text_format::make_rect("-10:-20, 10:20");
    println!(
        "  Lat:  [{:.1}°, {:.1}°]",
        rect.lat.lo.to_degrees(),
        rect.lat.hi.to_degrees(),
    );
    println!(
        "  Lng:  [{:.1}°, {:.1}°]",
        rect.lng.lo.to_degrees(),
        rect.lng.hi.to_degrees(),
    );

    // ── Round-trip: parse → format → parse ─────────────────────────────
    println!("\n=== Round-trip ===\n");
    let original = "0:0, 0:5, 5:5, 5:0; 1:1, 1:4, 4:4, 4:1";
    let poly1 = text_format::make_polygon(original);
    let formatted = text_format::polygon_to_string(&poly1);
    let poly2 = text_format::make_polygon(&formatted);
    println!("  Original:    {original}");
    println!("  Formatted:   {formatted}");
    println!(
        "  Areas match: {}",
        (poly1.area() - poly2.area()).abs() < 1e-15
    );
}
