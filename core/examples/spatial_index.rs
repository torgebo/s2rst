// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Spatial indexing with `ShapeIndex` and queries.
//!
//! Demonstrates how to build a `ShapeIndex` containing multiple shapes and
//! query it with `ContainsPointQuery` and `ClosestEdgeQuery`.
//!
//! Run with: `cargo run --example spatial_index`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::LatLng;
use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::earth;
use s2rst::s2::lax_loop::LaxLoop;
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::shape_index::ShapeIndex;

fn main() {
    // ── Build an index with several neighborhoods in Manhattan ───────────
    println!("=== Building spatial index ===\n");

    // Vertices must be counter-clockwise (interior on left) so the
    // polygon covers a small region rather than most of the sphere.
    let neighborhoods: &[(&str, &[(f64, f64)])] = &[
        (
            "Central Park",
            &[
                (40.764, -73.973), // SW corner
                (40.764, -73.949), // SE corner
                (40.800, -73.949), // NE corner
                (40.800, -73.973), // NW corner
            ],
        ),
        (
            "Midtown",
            &[
                (40.748, -73.986), // SW
                (40.748, -73.968), // SE
                (40.763, -73.968), // NE
                (40.763, -73.986), // NW
            ],
        ),
        (
            "SoHo",
            &[
                (40.719, -74.001), // SW
                (40.719, -73.992), // SE
                (40.727, -73.992), // NE
                (40.727, -74.001), // NW
            ],
        ),
    ];

    let mut index = ShapeIndex::new();
    for &(name, coords) in neighborhoods {
        let vertices: Vec<_> = coords
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();
        index.add(Box::new(LaxLoop::new(vertices)));
        println!("  Added polygon: {name}");
    }

    // Add a polyline for Broadway.
    let broadway = [
        (40.7061, -74.0133), // Battery Park
        (40.7128, -74.0060), // Wall St
        (40.7268, -73.9917), // Houston
        (40.7484, -73.9857), // Times Square
        (40.7831, -73.9712), // Upper West Side
    ];
    let broadway_pts: Vec<_> = broadway
        .iter()
        .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
        .collect();
    index.add(Box::new(LaxPolyline::new(broadway_pts)));
    println!("  Added polyline: Broadway");

    index.build();
    println!(
        "\n  Index contains {} shapes, {} total edges\n",
        index.len(),
        index.num_edges()
    );

    // ── Point containment: which neighborhood is a point in? ────────────
    println!("=== Point containment queries ===\n");
    let test_locations = [
        ("Empire State", 40.7484, -73.9857),
        ("Times Square", 40.7580, -73.9855),
        ("Central Park Zoo", 40.7678, -73.9718),
        ("Brooklyn Bridge", 40.7061, -73.9969),
    ];

    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    for &(name, lat, lng) in &test_locations {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let ids = query.containing_shape_ids(p);
        if ids.is_empty() {
            println!("  {name} ({lat:.4}°, {lng:.4}°): not in any neighborhood");
        } else {
            for id in &ids {
                let neighborhood = neighborhoods
                    .get((*id).as_usize())
                    .map_or("Broadway", |n| n.0);
                println!("  {name} ({lat:.4}°, {lng:.4}°): inside {neighborhood}");
            }
        }
    }

    // ── Closest edge: find nearest shape feature to a point ─────────────
    println!("\n=== Closest edge queries ===\n");
    let query = ClosestEdgeQuery::new(&index);
    for &(name, lat, lng) in &test_locations {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let target = PointTarget::new(p);
        let result = query.find_closest_edge(&target);
        if !result.is_empty() {
            let dist_m = earth::chord_angle_to_meters(result.distance);
            let shape_name = if (result.shape_id.as_usize()) < neighborhoods.len() {
                neighborhoods[result.shape_id.as_usize()].0
            } else {
                "Broadway"
            };
            println!(
                "  {}: nearest edge in {} (shape {}, edge {}), {:.0} m away",
                name, shape_name, result.shape_id, result.edge_id, dist_m
            );

            // Project the query point onto the nearest edge.
            let projected = query.project(p, &result);
            let proj_ll = LatLng::from_point(projected);
            println!(
                "    → projected to ({:.6}°, {:.6}°)",
                proj_ll.lat.degrees(),
                proj_ll.lng.degrees()
            );
        }
    }

    // ── Distance threshold: which shapes are within 500 m? ──────────────
    println!("\n=== Distance threshold: shapes within 500 m of Brooklyn Bridge ===\n");
    let bridge = LatLng::from_degrees(40.7061, -73.9969).to_point();
    let target = PointTarget::new(bridge);
    let limit = earth::meters_to_chord_angle(500.0);

    let opts = s2rst::s2::closest_edge_query::Options {
        max_distance: limit,
        max_results: 10,
        ..s2rst::s2::closest_edge_query::Options::default()
    };
    let results = query.find_closest_edges(&target, &opts);
    if results.is_empty() {
        println!("  No shapes within 500 m.");
    }
    for r in &results {
        let dist_m = earth::chord_angle_to_meters(r.distance);
        let shape_name = if (r.shape_id.as_usize()) < neighborhoods.len() {
            neighborhoods[r.shape_id.as_usize()].0
        } else {
            "Broadway"
        };
        println!(
            "  {} (shape {}, edge {}): {:.0} m",
            shape_name, r.shape_id, r.edge_id, dist_m
        );
    }
}
