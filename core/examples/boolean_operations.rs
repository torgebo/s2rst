// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Boolean operations on S2 polygons: union, intersection, difference.
//!
//! Run with: `cargo run --example boolean_operations`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::earth;
use s2rst::s2::{LatLng, Loop, Polygon};

fn main() {
    // Two overlapping square-ish polygons.
    let a = make_square("A", 0.0, 0.0, 2.0);
    let b = make_square("B", 1.0, 1.0, 2.0);

    // ── Union ──────────────────────────────────────────────────────────
    let union = Polygon::union(&mut a.clone(), &mut b.clone());
    print_polygon("Union (A ∪ B)", &union);

    // ── Intersection ───────────────────────────────────────────────────
    let inter = Polygon::intersection(&mut a.clone(), &mut b.clone());
    print_polygon("Intersection (A ∩ B)", &inter);

    // ── Difference ─────────────────────────────────────────────────────
    let diff = Polygon::difference(&mut a.clone(), &mut b.clone());
    print_polygon("Difference (A \\ B)", &diff);

    // ── Symmetric difference ───────────────────────────────────────────
    let sym_diff = Polygon::symmetric_difference(&mut a.clone(), &mut b.clone());
    print_polygon("Symmetric difference (A △ B)", &sym_diff);

    // ── Verify: area(union) = area(A) + area(B) - area(intersection) ──
    println!("=== Area identity check ===");
    let area_a = earth::steradians_to_square_km(a.area());
    let area_b = earth::steradians_to_square_km(b.area());
    let area_union = earth::steradians_to_square_km(union.area());
    let area_inter = earth::steradians_to_square_km(inter.area());
    println!("  area(A)           = {area_a:.2} km²");
    println!("  area(B)           = {area_b:.2} km²");
    println!("  area(A ∪ B)       = {area_union:.2} km²");
    println!("  area(A ∩ B)       = {area_inter:.2} km²");
    println!(
        "  A + B - (A ∩ B)   = {:.2} km²  (should ≈ union)",
        area_a + area_b - area_inter,
    );
}

/// Create a square polygon centered at (lat, lng) with half-side `half` in degrees.
fn make_square(name: &str, lat: f64, lng: f64, half: f64) -> Polygon {
    let loop_ = Loop::new(vec![
        LatLng::from_degrees(lat - half, lng - half).to_point(),
        LatLng::from_degrees(lat - half, lng + half).to_point(),
        LatLng::from_degrees(lat + half, lng + half).to_point(),
        LatLng::from_degrees(lat + half, lng - half).to_point(),
    ]);
    let p = Polygon::from_loops(vec![loop_]);
    println!(
        "Polygon {name}: {:.2} km², {} loops, {} vertices",
        earth::steradians_to_square_km(p.area()),
        p.num_loops(),
        p.loops().iter().map(Loop::num_vertices).sum::<usize>(),
    );
    p
}

fn print_polygon(label: &str, p: &Polygon) {
    println!(
        "\n{label}: {:.2} km², {} loop(s), {} vertices",
        earth::steradians_to_square_km(p.area()),
        p.num_loops(),
        p.loops().iter().map(Loop::num_vertices).sum::<usize>(),
    );
}
