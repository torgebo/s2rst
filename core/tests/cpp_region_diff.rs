// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for `PointRegion` and for `CellUnion`'s floating-point methods (spec 011
//! FR-021) against the C++ reference implementation (google/s2geometry), Principle I.
//!
//! The vectors in `data/cpp_region_vectors.csv` come from `tools/refgen-cpp/generate_region`.
//! Every result is compared bit for bit; there is no deviation (after FR-017's polar latitude
//! and FR-018's +0 sums).

use s2rst::r3::Vector;
use s2rst::s1::Angle;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, Face, Point, Rect, Region};

const CSV: &str = include_str!("data/cpp_region_vectors.csv");

fn section(name: &str) -> Vec<Vec<&'static str>> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in CSV.lines() {
        if let Some(h) = line.strip_prefix("# ") {
            inside = h == name;
            continue;
        }
        if inside && !line.is_empty() {
            rows.push(line.split(',').collect());
        }
    }
    assert!(!rows.is_empty(), "section {name} is empty or missing");
    rows
}

fn f(hex: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap())
}

fn d(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn b(x: bool) -> &'static str {
    if x { "1" } else { "0" }
}

fn point(c: &[&str]) -> Point {
    Point(Vector {
        x: f(c[0]),
        y: f(c[1]),
        z: f(c[2]),
    })
}

fn ids(s: &str) -> Vec<CellId> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(';')
        .map(|h| CellId(u64::from_str_radix(h, 16).unwrap()))
        .collect()
}

fn ids_s(ids: &[CellId]) -> String {
    ids.iter()
        .map(|id| format!("{:016x}", id.id()))
        .collect::<Vec<_>>()
        .join(";")
}

fn cap_s(c: Cap) -> [String; 4] {
    let p = c.center();
    [
        d(p.x()),
        d(p.y()),
        d(p.z()),
        d(c.chord_radius().length2()),
    ]
}

fn rect_s(r: Rect) -> [String; 4] {
    [d(r.lat.lo), d(r.lat.hi), d(r.lng.lo), d(r.lng.hi)]
}

/// The rows' unions are upstream's normalised ids: the library normalises them to the same.
fn union(s: &str) -> CellUnion {
    let v = ids(s);
    let u = CellUnion::from_cell_ids(v.clone());
    assert_eq!(u.cell_ids(), &v[..], "normalisation differs for {s}");
    u
}

#[test]
fn point_region_bounds_match_cpp() {
    let rows = section("POINTREGION");
    for r in &rows {
        let pr = PointRegion::new(point(&r[0..3]));
        assert_eq!(cap_s(pr.cap_bound()), r[3..7], "cap_bound {r:?}");
        assert_eq!(rect_s(pr.rect_bound()), r[7..11], "rect_bound {r:?}");
        assert_eq!(ids_s(&pr.cell_union_bound()), r[11], "cell_union_bound {r:?}");
    }
    assert!(rows.len() > 2000);
}

#[test]
fn point_region_cell_tests_match_cpp() {
    let rows = section("POINTREGION_CELL");
    for r in &rows {
        let pr = PointRegion::new(point(&r[0..3]));
        let cell = Cell::from_cell_id(CellId(u64::from_str_radix(r[3], 16).unwrap()));
        assert_eq!(b(pr.contains_cell(&cell)), r[4], "contains_cell {r:?}");
        assert_eq!(b(pr.intersects_cell(&cell)), r[5], "intersects_cell {r:?}");
    }
}

#[test]
fn cell_union_bounds_match_cpp() {
    for r in section("CELLUNION_BOUND") {
        let u = union(r[0]);
        assert_eq!(cap_s(u.cap_bound()), r[1..5], "cap_bound {}", r[0]);
        assert_eq!(rect_s(u.rect_bound()), r[5..9], "rect_bound {}", r[0]);
        assert_eq!(ids_s(&u.cell_union_bound()), r[9], "cell_union_bound {}", r[0]);
    }
}

#[test]
fn cell_union_areas_match_cpp() {
    for r in section("CELLUNION_AREA") {
        let u = union(r[0]);
        assert_eq!(d(u.average_based_area()), r[1], "average_based_area {}", r[0]);
        assert_eq!(d(u.approx_area()), r[2], "approx_area {}", r[0]);
        assert_eq!(d(u.exact_area()), r[3], "exact_area {}", r[0]);
    }
}

#[test]
fn cell_union_contains_point_matches_cpp() {
    for r in section("CELLUNION_POINT") {
        let u = union(r[0]);
        let p = point(&r[1..4]);
        assert_eq!(b(u.contains_point(p)), r[4], "contains_point {r:?}");
        assert_eq!(b(Region::contains_point(&u, &p)), r[4], "Region::contains_point {r:?}");
    }
}

#[test]
fn cell_union_expand_by_radius_matches_cpp() {
    for r in section("CELLUNION_EXPAND") {
        let mut u = union(r[0]);
        let diff: u8 = r[2].parse().unwrap();
        u.expand_by_radius(Angle::from_radians(f(r[1])), diff);
        assert_eq!(ids_s(u.cell_ids()), r[3], "expand_by_radius {}, {}, {diff}", r[0], r[1]);
    }
}

#[test]
fn face_cell_rect_bounds_match_cpp() {
    let rows = section("CELL_FACE_RECT");
    assert_eq!(rows.len(), 6);
    for r in rows {
        let face = Face::ALL[r[0].parse::<usize>().unwrap()];
        let cell = Cell::from_cell_id(CellId::from_face(face));
        assert_eq!(rect_s(cell.rect_bound()), r[1..5], "face {face}");
    }
}
