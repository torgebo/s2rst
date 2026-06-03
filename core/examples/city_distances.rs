// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Compute distances and bearings between world cities.
//!
//! Demonstrates `LatLng`, `Point`, the `earth` module for converting angular
//! distances to real-world units, and `Cap` for proximity searches.
//!
//! Run with: `cargo run --example city_distances`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::earth;
use s2rst::s2::{Cap, LatLng, Point};

/// A named location on the globe.
struct City {
    name: &'static str,
    ll: LatLng,
}

impl City {
    fn new(name: &'static str, lat: f64, lng: f64) -> Self {
        City {
            name,
            ll: LatLng::from_degrees(lat, lng),
        }
    }

    fn point(&self) -> Point {
        self.ll.to_point()
    }
}

fn main() {
    let cities = [
        City::new("New York", 40.7128, -74.0060),
        City::new("London", 51.5074, -0.1278),
        City::new("Tokyo", 35.6762, 139.6503),
        City::new("Sydney", -33.8688, 151.2093),
        City::new("São Paulo", -23.5505, -46.6333),
        City::new("Cairo", 30.0444, 31.2357),
    ];

    // ── Pairwise distances ──────────────────────────────────────────────
    println!("=== Pairwise distances (km) ===\n");
    print!("{:>12}", "");
    for city in &cities {
        print!("{:>12}", city.name);
    }
    println!();

    for a in &cities {
        print!("{:>12}", a.name);
        for b in &cities {
            let km = earth::get_distance_km_latlng(a.ll, b.ll);
            print!("{km:>12.0}");
        }
        println!();
    }

    // ── Initial bearings ────────────────────────────────────────────────
    println!("\n=== Initial bearings from New York ===\n");
    let nyc = &cities[0];
    for dest in &cities[1..] {
        let bearing = earth::get_initial_bearing(nyc.ll, dest.ll);
        let degrees = bearing.degrees();
        let compass = compass_direction(degrees);
        println!(
            "  {:<12} → {:<12}  bearing {:.1}° ({})",
            nyc.name, dest.name, degrees, compass
        );
    }

    // ── Proximity search: cities within 10 000 km of Cairo ──────────────
    let radius_km = 10_000.0;
    let cairo = &cities[5];
    let search_cap = Cap::from_center_angle(cairo.point(), earth::km_to_angle(radius_km));
    println!(
        "\n=== Cities within {:.0} km of {} ===\n",
        radius_km, cairo.name
    );
    for city in &cities {
        if search_cap.contains_point(city.point()) {
            let d = earth::get_distance_km_latlng(cairo.ll, city.ll);
            println!("  {} ({:.0} km)", city.name, d);
        }
    }

    // ── Midpoint between two cities ─────────────────────────────────────
    println!("\n=== Midpoint between New York and London ===\n");
    let london = &cities[1];
    // Spherical midpoint: average the unit vectors and re-normalize.
    let mid_vec = nyc.point().0 + london.point().0;
    let midpoint = Point::from_coords(mid_vec.x, mid_vec.y, mid_vec.z);
    let mid_ll = LatLng::from_point(midpoint);
    println!(
        "  ({:.4}°, {:.4}°)",
        mid_ll.lat.degrees(),
        mid_ll.lng.degrees()
    );
    let d1 = earth::get_distance_km_points(nyc.point(), midpoint);
    let d2 = earth::get_distance_km_points(london.point(), midpoint);
    println!("  Distance from NYC: {d1:.0} km");
    println!("  Distance from London: {d2:.0} km");

    // ── Area of a spherical cap ─────────────────────────────────────────
    println!("\n=== Area of a 500 km cap centered on Tokyo ===\n");
    let tokyo = &cities[2];
    let cap_500 = Cap::from_center_angle(tokyo.point(), earth::km_to_angle(500.0));
    let area_km2 = earth::steradians_to_square_km(cap_500.area());
    println!("  {area_km2:.0} km²");
}

/// Rough compass direction from a bearing in degrees.
fn compass_direction(deg: f64) -> &'static str {
    let d = ((deg % 360.0) + 360.0) % 360.0;
    match d {
        d if d < 22.5 => "N",
        d if d < 67.5 => "NE",
        d if d < 112.5 => "E",
        d if d < 157.5 => "SE",
        d if d < 202.5 => "S",
        d if d < 247.5 => "SW",
        d if d < 292.5 => "W",
        d if d < 337.5 => "NW",
        _ => "N",
    }
}
