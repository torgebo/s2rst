// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Geofencing with polygons and loops.
//!
//! Demonstrates creating geographic boundaries and testing whether points
//! fall inside them. Uses `Loop`, `Polygon`, `Cap`, and `Rect` to build
//! various geofence shapes.
//!
//! Run with: `cargo run --example geofencing`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::earth;
use s2rst::s2::{Cap, LatLng, Loop, Polygon, Rect, Region};

fn main() {
    // ── Define a delivery zone as a polygon ─────────────────────────────
    println!("=== Delivery zone (downtown polygon) ===\n");
    let zone = Loop::new(vec![
        LatLng::from_degrees(40.700, -74.020).to_point(),
        LatLng::from_degrees(40.700, -73.970).to_point(),
        LatLng::from_degrees(40.730, -73.970).to_point(),
        LatLng::from_degrees(40.730, -74.020).to_point(),
    ]);
    let zone_polygon = Polygon::from_loops(vec![zone]);
    let area_km2 = earth::steradians_to_square_km(zone_polygon.area());
    println!("  Zone area: {area_km2:.2} km²");

    let orders = [
        ("Wall Street", 40.7068, -74.0090),
        ("Chinatown", 40.7158, -73.9970),
        ("Brooklyn Heights", 40.6960, -73.9936),
        ("Greenwich Village", 40.7336, -74.0027),
    ];

    for &(name, lat, lng) in &orders {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let inside = zone_polygon.contains_point(&p);
        println!(
            "  {:<22} ({:.4}°, {:.4}°)  {}",
            name,
            lat,
            lng,
            if inside { "✓ in zone" } else { "✗ outside" }
        );
    }

    // ── Circular geofence: 2 km around a warehouse ──────────────────────
    println!("\n=== Circular geofence (2 km from warehouse) ===\n");
    let warehouse = LatLng::from_degrees(40.7128, -74.0060);
    let fence = Cap::from_center_angle(warehouse.to_point(), earth::km_to_angle(2.0));
    println!(
        "  Warehouse at ({:.4}°, {:.4}°)",
        warehouse.lat.degrees(),
        warehouse.lng.degrees()
    );
    println!("  Fence radius: 2 km");
    println!(
        "  Fence area: {:.2} km²",
        earth::steradians_to_square_km(fence.area())
    );

    for &(name, lat, lng) in &orders {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let inside = fence.contains_point(p);
        let dist = earth::get_distance_km_latlng(warehouse, LatLng::from_degrees(lat, lng));
        println!(
            "  {:<22} {:.2} km  {}",
            name,
            dist,
            if inside {
                "✓ in range"
            } else {
                "✗ too far"
            }
        );
    }

    // ── Rectangular geofence (bounding box) ─────────────────────────────
    println!("\n=== Rectangular geofence (lat-lng box) ===\n");
    let sw = LatLng::from_degrees(40.700, -74.020);
    let ne = LatLng::from_degrees(40.720, -73.990);
    let rect = Rect::from_lat_lng(sw).add_point(ne);
    println!(
        "  SW corner: ({:.3}°, {:.3}°)",
        sw.lat.degrees(),
        sw.lng.degrees()
    );
    println!(
        "  NE corner: ({:.3}°, {:.3}°)",
        ne.lat.degrees(),
        ne.lng.degrees()
    );
    println!(
        "  Box area: {:.2} km²",
        earth::steradians_to_square_km(rect.area())
    );

    for &(name, lat, lng) in &orders {
        let ll = LatLng::from_degrees(lat, lng);
        let inside = rect.contains_lat_lng(ll);
        println!(
            "  {:<22} {}",
            name,
            if inside { "✓ in box" } else { "✗ outside" }
        );
    }

    // ── Polygon with a hole (exclusion zone) ────────────────────────────
    println!("\n=== Polygon with hole (park excluded from delivery) ===\n");
    let outer = Loop::new(vec![
        LatLng::from_degrees(40.760, -73.990).to_point(),
        LatLng::from_degrees(40.760, -73.960).to_point(),
        LatLng::from_degrees(40.780, -73.960).to_point(),
        LatLng::from_degrees(40.780, -73.990).to_point(),
    ]);
    // The hole must wind in the opposite direction.
    let mut hole = Loop::new(vec![
        LatLng::from_degrees(40.765, -73.980).to_point(),
        LatLng::from_degrees(40.775, -73.980).to_point(),
        LatLng::from_degrees(40.775, -73.970).to_point(),
        LatLng::from_degrees(40.765, -73.970).to_point(),
    ]);
    hole.invert();
    let polygon_with_hole = Polygon::from_loops(vec![outer, hole]);
    println!(
        "  Polygon area (with hole): {:.2} km²",
        earth::steradians_to_square_km(polygon_with_hole.area())
    );
    println!("  Has holes: {}", polygon_with_hole.has_holes());

    let test_points = [
        ("In outer, outside hole", 40.762, -73.985),
        ("Inside the hole", 40.770, -73.975),
        ("Outside entirely", 40.750, -73.990),
    ];
    for &(name, lat, lng) in &test_points {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let inside = polygon_with_hole.contains_point(&p);
        println!(
            "  {:<28} {}",
            name,
            if inside {
                "✓ deliverable"
            } else {
                "✗ excluded"
            }
        );
    }

    // ── Multiple zones: find which zone a point is in ───────────────────
    println!("\n=== Multi-zone lookup ===\n");
    let zones: Vec<(&str, Polygon)> = vec![
        (
            "Zone A",
            Polygon::from_loops(vec![Loop::new(vec![
                LatLng::from_degrees(40.70, -74.02).to_point(),
                LatLng::from_degrees(40.70, -73.99).to_point(),
                LatLng::from_degrees(40.72, -73.99).to_point(),
                LatLng::from_degrees(40.72, -74.02).to_point(),
            ])]),
        ),
        (
            "Zone B",
            Polygon::from_loops(vec![Loop::new(vec![
                LatLng::from_degrees(40.72, -74.02).to_point(),
                LatLng::from_degrees(40.72, -73.99).to_point(),
                LatLng::from_degrees(40.74, -73.99).to_point(),
                LatLng::from_degrees(40.74, -74.02).to_point(),
            ])]),
        ),
    ];

    let lookups = [
        ("Point 1", 40.710, -74.005),
        ("Point 2", 40.730, -74.005),
        ("Point 3", 40.750, -74.005),
    ];

    for &(name, lat, lng) in &lookups {
        let p = LatLng::from_degrees(lat, lng).to_point();
        let zone_name = zones
            .iter()
            .find(|(_, poly)| poly.contains_point(&p))
            .map_or("none", |(name, _)| *name);
        println!("  {name}: zone = {zone_name}");
    }
}
