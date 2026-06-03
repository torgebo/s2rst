// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Explore the S2 cell hierarchy.
//!
//! Demonstrates `CellId`, `Cell`, and how cells at different levels relate to
//! real-world sizes, plus hierarchy navigation (parents, children, neighbors).
//!
//! Run with: `cargo run --example cell_hierarchy`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s2::earth;
use s2rst::s2::{Cell, CellId, LatLng};

fn main() {
    // ── Cell sizes at every level ───────────────────────────────────────
    println!("=== S2 cell sizes by level ===\n");
    println!(
        "{:>5}  {:>14}  {:>14}",
        "Level", "Edge (approx)", "Area (approx)"
    );
    println!(
        "{:>5}  {:>14}  {:>14}",
        "-----", "--------------", "--------------"
    );
    for level in (0..=30).step_by(5) {
        let id = CellId::from_face_pos_level(0, 0, level);
        let cell = Cell::from_cell_id(id);
        let edge_km = edge_length_km(&cell);
        let area_km2 = earth::steradians_to_square_km(cell.approx_area());
        println!(
            "{:>5}  {:>14}  {:>14}",
            level,
            format_distance(edge_km),
            format_area(area_km2)
        );
    }

    // ── Leaf cell for a specific location ───────────────────────────────
    let paris = LatLng::from_degrees(48.8566, 2.3522);
    let leaf = CellId::from_lat_lng(&paris);
    println!(
        "\n=== Leaf cell for Paris ({:.4}°, {:.4}°) ===\n",
        48.8566, 2.3522
    );
    println!("  CellId:  0x{:016x}", leaf.0);
    println!("  Token:   {}", leaf.to_token());
    println!("  Face:    {}", leaf.face());
    println!("  Level:   {}", leaf.level());

    // ── Walk up the hierarchy ───────────────────────────────────────────
    println!("\n=== Ancestor cells ===\n");
    println!(
        "{:>5}  {:>18}  {:>14}  {:>14}",
        "Level", "Token", "Edge", "Area"
    );
    for level in [30, 20, 15, 10, 5, 1, 0] {
        let ancestor = leaf.parent_at_level(level);
        let cell = Cell::from_cell_id(ancestor);
        let edge_km = edge_length_km(&cell);
        let area_km2 = earth::steradians_to_square_km(cell.approx_area());
        println!(
            "{:>5}  {:>18}  {:>14}  {:>14}",
            level,
            ancestor.to_token(),
            format_distance(edge_km),
            format_area(area_km2)
        );
    }

    // ── Children of a level-4 cell ──────────────────────────────────────
    let parent = leaf.parent_at_level(4);
    let children = parent.children();
    println!(
        "\n=== 4 children of level-4 cell {} ===\n",
        parent.to_token()
    );
    for (i, child) in children.iter().enumerate() {
        let contains_paris = child.contains(leaf);
        println!(
            "  child[{}]: {}  level={}{}",
            i,
            child.to_token(),
            child.level(),
            if contains_paris {
                "  ← contains Paris"
            } else {
                ""
            }
        );
    }

    // ── Edge neighbors ──────────────────────────────────────────────────
    let level10 = leaf.parent_at_level(10);
    let neighbors = level10.edge_neighbors();
    println!(
        "\n=== 4 edge neighbors of level-10 cell {} ===\n",
        level10.to_token()
    );
    for (i, nb) in neighbors.iter().enumerate() {
        let ll = nb.to_lat_lng();
        println!(
            "  neighbor[{}]: {}  center=({:.4}°, {:.4}°)",
            i,
            nb.to_token(),
            ll.lat.degrees(),
            ll.lng.degrees()
        );
    }

    // ── Containment relationships ───────────────────────────────────────
    println!("\n=== Containment checks ===\n");
    let level5 = leaf.parent_at_level(5);
    let level15 = leaf.parent_at_level(15);
    println!("  level-5 contains level-15?  {}", level5.contains(level15));
    println!("  level-15 contains level-5?  {}", level15.contains(level5));
    println!("  level-5 contains leaf?      {}", level5.contains(leaf));

    // ── Token round-trip ────────────────────────────────────────────────
    println!("\n=== Token round-trip ===\n");
    let token = level10.to_token();
    let restored = CellId::from_token(&token);
    println!("  Original:  0x{:016x}", level10.0);
    println!("  Token:     {token}");
    println!("  Restored:  0x{:016x}", restored.0);
    println!("  Match:     {}", level10 == restored);
}

/// Approximate edge length in km (average of the four edges).
fn edge_length_km(cell: &Cell) -> f64 {
    let mut total = 0.0;
    for i in 0..4 {
        let v0 = cell.vertex(i);
        let v1 = cell.vertex((i + 1) % 4);
        total += earth::get_distance_km_points(v0, v1);
    }
    total / 4.0
}

/// Human-friendly distance string.
fn format_distance(km: f64) -> String {
    if km >= 1.0 {
        format!("{km:.1} km")
    } else if km >= 0.001 {
        format!("{:.1} m", km * 1000.0)
    } else {
        format!("{:.1} mm", km * 1_000_000.0)
    }
}

/// Human-friendly area string.
fn format_area(km2: f64) -> String {
    if km2 >= 1_000_000.0 {
        format!("{:.1} M km²", km2 / 1_000_000.0)
    } else if km2 >= 1.0 {
        format!("{km2:.1} km²")
    } else if km2 >= 0.000_001 {
        format!("{:.1} m²", km2 * 1_000_000.0)
    } else {
        format!("{:.2} mm²", km2 * 1e12)
    }
}
