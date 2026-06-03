// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Compute convex hulls of point sets on the sphere.
//!
//! Demonstrates `ConvexHullQuery` for finding the smallest convex region
//! enclosing a set of points, and shows how it relates to `Cap` bounds.
//!
//! Run with: `cargo run --example convex_hull`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::convex_hull_query::ConvexHullQuery;
use s2rst::s2::earth;
use s2rst::s2::{LatLng, Region};

fn main() {
    // ── Convex hull of European capitals ────────────────────────────────
    println!("=== Convex hull of European capitals ===\n");
    let capitals = [
        ("Lisbon", 38.7223, -9.1393),
        ("London", 51.5074, -0.1278),
        ("Paris", 48.8566, 2.3522),
        ("Berlin", 52.5200, 13.4050),
        ("Rome", 41.9028, 12.4964),
        ("Madrid", 40.4168, -3.7038),
        ("Athens", 37.9838, 23.7275),
        ("Helsinki", 60.1699, 24.9384),
        ("Reykjavik", 64.1466, -21.9426),
    ];

    let mut query = ConvexHullQuery::new();
    for &(name, lat, lng) in &capitals {
        let p = LatLng::from_degrees(lat, lng).to_point();
        query.add_point(p);
        println!("  Added: {name} ({lat:.2}°, {lng:.2}°)");
    }

    let hull = query.convex_hull();
    let hull_area_km2 = earth::steradians_to_square_km(hull.area());
    println!("\n  Hull vertices: {}", hull.num_vertices());
    println!("  Hull area: {hull_area_km2:.0} km²");

    // Show which capitals are vertices of the hull.
    println!("\n  Hull boundary vertices:");
    for i in 0..hull.num_vertices() {
        let v = hull.vertex(i);
        let ll = LatLng::from_point(v);
        // Find the nearest capital.
        let nearest = capitals
            .iter()
            .min_by(|a, b| {
                let da = LatLng::from_degrees(a.1, a.2).get_distance(ll).radians();
                let db = LatLng::from_degrees(b.1, b.2).get_distance(ll).radians();
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
        println!(
            "    vertex {}: ({:.4}°, {:.4}°) ≈ {}",
            i,
            ll.lat.degrees(),
            ll.lng.degrees(),
            nearest.0
        );
    }

    // ── Check which capitals lie on the hull boundary vs interior ────────
    println!("\n  Containment check:");
    for &(name, lat, lng) in &capitals {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let inside = hull.contains_point(&p);
        println!(
            "    {:<12} {}",
            name,
            if inside { "inside hull" } else { "on boundary" }
        );
    }

    // ── Compare hull area vs bounding cap ───────────────────────────────
    let cap = hull.cap_bound();
    let cap_area_km2 = earth::steradians_to_square_km(cap.area());
    println!("\n  Bounding cap area:  {cap_area_km2:.0} km²");
    println!("  Convex hull area:   {hull_area_km2:.0} km²");
    println!(
        "  Hull / cap ratio:   {:.1}%",
        100.0 * hull_area_km2 / cap_area_km2
    );

    // ── Convex hull of a small cluster ──────────────────────────────────
    println!("\n=== Convex hull of NYC boroughs ===\n");
    let boroughs = [
        ("Manhattan", 40.7831, -73.9712),
        ("Brooklyn", 40.6782, -73.9442),
        ("Queens", 40.7282, -73.7949),
        ("Bronx", 40.8448, -73.8648),
        ("Staten Island", 40.5795, -74.1502),
    ];

    let mut query2 = ConvexHullQuery::new();
    for &(_, lat, lng) in &boroughs {
        query2.add_point(LatLng::from_degrees(lat, lng).to_point());
    }
    let hull2 = query2.convex_hull();
    let area_km2 = earth::steradians_to_square_km(hull2.area());
    println!("  Hull vertices: {}", hull2.num_vertices());
    println!("  Hull area: {area_km2:.1} km²");

    // Check which boroughs are interior vs boundary.
    for &(name, lat, lng) in &boroughs {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let inside = hull2.contains_point(&p);
        println!(
            "    {:<16} {}",
            name,
            if inside {
                "interior"
            } else {
                "boundary vertex"
            }
        );
    }
}
