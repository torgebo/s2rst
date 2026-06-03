// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Approximate regions with S2 cell coverings.
//!
//! Shows how `RegionCoverer` converts arbitrary regions into compact sets of
//! `CellId` tokens suitable for spatial database indexing.
//!
//! Run with: `cargo run --example region_covering`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::earth;
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, CellUnion, LatLng, Loop, Rect};

fn main() {
    // ── Cover a circular cap (50 km around Berlin) ──────────────────────
    println!("=== Cap covering: 50 km around Berlin ===\n");
    let berlin = LatLng::from_degrees(52.5200, 13.4050);
    let cap = Cap::from_center_angle(berlin.to_point(), earth::km_to_angle(50.0));

    let coverer = RegionCoverer::new().max_level(12).max_cells(8);
    let covering = coverer.covering(&cap);
    print_covering("Cap", &covering);

    // Verify: every point in the cap should be covered.
    let test_points = [
        LatLng::from_degrees(52.5200, 13.4050), // center
        LatLng::from_degrees(52.8, 13.4),       // ~30 km north
        LatLng::from_degrees(52.2, 13.4),       // ~35 km south
    ];
    println!("  Containment checks:");
    for ll in &test_points {
        println!(
            "    ({:.2}°, {:.2}°) in cap: {}  in covering: {}",
            ll.lat.degrees(),
            ll.lng.degrees(),
            cap.contains_point(ll.to_point()),
            covering.contains_point(ll.to_point()),
        );
    }

    // ── Cover a lat-lng rectangle (bounding box for Switzerland) ────────
    println!("\n=== Rect covering: Switzerland bounding box ===\n");
    let sw = LatLng::from_degrees(45.818, 5.956);
    let ne = LatLng::from_degrees(47.808, 10.492);
    let rect = Rect::from_lat_lng(sw).add_point(ne);

    let coverer = RegionCoverer::new().max_level(10).max_cells(20);
    let covering = coverer.covering(&rect);
    print_covering("Rect", &covering);

    // ── Cover a triangular polygon (Bermuda Triangle) ───────────────────
    println!("\n=== Loop covering: Bermuda Triangle ===\n");
    let bermuda = Loop::new(vec![
        LatLng::from_degrees(25.7617, -80.1918).to_point(), // Miami
        LatLng::from_degrees(32.3214, -64.7574).to_point(), // Bermuda
        LatLng::from_degrees(18.4655, -66.1057).to_point(), // San Juan
    ]);
    let area_km2 = earth::steradians_to_square_km(bermuda.area());
    println!("  Triangle area: {area_km2:.0} km²");

    let coverer = RegionCoverer::new().max_level(8).max_cells(30);
    let covering = coverer.covering(&bermuda);
    print_covering("Loop", &covering);

    // ── Interior covering vs outer covering ─────────────────────────────
    println!("\n=== Interior vs outer covering (100 km cap around Tokyo) ===\n");
    let tokyo = LatLng::from_degrees(35.6762, 139.6503);
    let tokyo_cap = Cap::from_center_angle(tokyo.to_point(), earth::km_to_angle(100.0));

    let coverer = RegionCoverer::new().max_level(10).max_cells(12);
    let outer = coverer.covering(&tokyo_cap);
    let inner = coverer.interior_covering(&tokyo_cap);

    println!("  Outer covering: {} cells", outer.num_cells());
    println!("  Inner covering: {} cells", inner.num_cells());
    println!(
        "  Outer area: {:.0} km²",
        earth::steradians_to_square_km(area_of_union(&outer))
    );
    println!(
        "  Inner area: {:.0} km²",
        earth::steradians_to_square_km(area_of_union(&inner))
    );
    println!(
        "  Cap area:   {:.0} km²",
        earth::steradians_to_square_km(tokyo_cap.area())
    );

    // ── Covering with level constraints ─────────────────────────────────
    println!("\n=== Level-constrained covering (levels 5-8 only) ===\n");
    let coverer = RegionCoverer::new().min_level(5).max_level(8).max_cells(12);
    let constrained = coverer.covering(&tokyo_cap);
    print_covering("Constrained", &constrained);

    // Show level distribution.
    let mut level_counts = [0u32; 31];
    for id in constrained.cell_ids() {
        level_counts[id.level().as_usize()] += 1;
    }
    println!("  Level distribution:");
    for (level, &count) in level_counts.iter().enumerate() {
        if count > 0 {
            println!("    level {level}: {count} cells");
        }
    }
}

/// Print summary of a covering.
fn print_covering(_label: &str, covering: &CellUnion) {
    println!("  {} cells in covering:", covering.num_cells());
    for id in covering.cell_ids() {
        let ll = id.to_lat_lng();
        println!(
            "    {}  level={:>2}  center=({:.3}°, {:.3}°)",
            id.to_token(),
            id.level(),
            ll.lat.degrees(),
            ll.lng.degrees(),
        );
    }
}

/// Sum the exact areas of cells in a `CellUnion`.
fn area_of_union(cu: &CellUnion) -> f64 {
    use s2rst::s2::Cell;
    cu.cell_ids()
        .iter()
        .map(|id| Cell::from_cell_id(*id).exact_area())
        .sum()
}
