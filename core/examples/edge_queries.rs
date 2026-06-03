// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Closest-edge and furthest-edge queries on a Koch snowflake polyline.
//!
//! Ported from Go `ExampleEdgeQuery_FindEdges_findClosestEdges` and
//! `ExampleEdgeQuery_FindEdges_findFurthestEdges`.
//!
//! Run with: `cargo run --example edge_queries`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::LatLng;
use s2rst::s2::closest_edge_query::{
    ClosestEdgeQuery, Options as ClosestOptions, PointTarget as ClosestPointTarget,
};
use s2rst::s2::furthest_edge_query::{
    FurthestEdgeQuery, Options as FurthestOptions, PointTarget as FurthestPointTarget,
};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;

/// Koch snowflake (iteration ≈ 3) centered on the continental US.
fn koch_snowflake() -> Polyline {
    let coords: &[(f64, f64)] = &[
        (47.5467, -103.6035),
        (45.9214, -103.7320),
        (45.1527, -105.8000),
        (44.2866, -103.8538),
        (42.6450, -103.9695),
        (41.8743, -105.9314),
        (42.7141, -107.8226),
        (41.0743, -107.8377),
        (40.2486, -109.6869),
        (39.4333, -107.8521),
        (37.7936, -107.8658),
        (38.5849, -106.0503),
        (37.7058, -104.2841),
        (36.0638, -104.3793),
        (35.3062, -106.1585),
        (34.4284, -104.4703),
        (32.8024, -104.5573),
        (33.5273, -102.8163),
        (32.6053, -101.1982),
        (34.2313, -101.0361),
        (34.9120, -99.2189),
        (33.9382, -97.6134),
        (32.3185, -97.8489),
        (32.9481, -96.0510),
        (31.9449, -94.5321),
        (33.5521, -94.2263),
        (34.1285, -92.3780),
        (35.1678, -93.9070),
        (36.7893, -93.5734),
        (37.3529, -91.6381),
        (36.2777, -90.1050),
        (37.8824, -89.6824),
        (38.3764, -87.7108),
        (39.4869, -89.2407),
        (41.0883, -88.7784),
        (40.5829, -90.8289),
        (41.6608, -92.4765),
        (43.2777, -92.0749),
        (43.7961, -89.9408),
        (44.8865, -91.6533),
        (46.4844, -91.2100),
        (45.9512, -93.4327),
        (46.9863, -95.2792),
        (45.3722, -95.6237),
        (44.7496, -97.7776),
        (45.7189, -99.6629),
        (47.3422, -99.4244),
        (46.6523, -101.6056),
    ];
    let pts: Vec<_> = coords
        .iter()
        .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
        .collect();
    Polyline::new(pts)
}

fn main() {
    let polyline = koch_snowflake();
    let point = LatLng::from_degrees(37.7, -122.5).to_point();

    // Load the polyline into a ShapeIndex.
    let mut index = ShapeIndex::new();
    index.add(Box::new(polyline));
    index.build();

    // ── Closest edges ──────────────────────────────────────────────────
    println!("=== 7 closest edges ===\n");
    let opts = ClosestOptions {
        max_results: 7,
        ..ClosestOptions::default()
    };
    let query = ClosestEdgeQuery::new(&index);
    let target = ClosestPointTarget::new(point);
    let results = query.find_closest_edges(&target, &opts);

    for r in &results {
        println!(
            "Polyline {}, Edge {} is {:.4} degrees from Point ({:.6}, {:.6}, {:.6})",
            r.shape_id,
            r.edge_id,
            r.distance.to_angle().degrees(),
            point.0.x,
            point.0.y,
            point.0.z,
        );
    }

    // ── Furthest edges ─────────────────────────────────────────────────
    println!("\n=== 3 furthest edges ===\n");
    let opts = FurthestOptions {
        max_results: 3,
        ..FurthestOptions::default()
    };
    let query = FurthestEdgeQuery::new(&index);
    let target = FurthestPointTarget::new(point);
    let results = query.find_furthest_edges(&target, &opts);

    for r in &results {
        println!(
            "Polyline {}, Edge {} is {:.3} degrees from Point ({:.6}, {:.6}, {:.6})",
            r.shape_id,
            r.edge_id,
            r.distance.to_angle().degrees(),
            point.0.x,
            point.0.y,
            point.0.z,
        );
    }
}
