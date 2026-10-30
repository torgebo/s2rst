// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for `Cell`'s bounds against the points a cell contains, and for
//! `RegionUnion` and `RegionIntersection` (spec 012 FR-018), against the C++ reference
//! implementation (google/s2geometry), Principle I.
//!
//! The vectors in `data/cpp_cell_bounds_vectors.csv` come from
//! `tools/refgen-cpp/generate_cell_bounds`. Every result is compared bit for bit, except the one
//! documented deviation: the adaptors' cap bound is their rectangle's `cap_bound`, which the
//! library widens (spec 010 FR-019, `UPSTREAM.md` U-016), so its centre is checked bit for bit
//! and its radius as upstream's widened by the documented rule. The `CELL_BOUND_POINTS` rows pin
//! U-017: upstream's bounds miss points of `contains_point`'s margin, and so do the library's.

use s2rst::r1;
use s2rst::r3::Vector;
use s2rst::s1::{self, Angle, ChordAngle};
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;
use s2rst::s2::{Cap, Cell, CellId, Point, Rect, Region};

const CSV: &str = include_str!("data/cpp_cell_bounds_vectors.csv");

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

fn cell(hex: &str) -> Cell {
    Cell::from_cell_id(CellId(u64::from_str_radix(hex, 16).unwrap()))
}

fn point(c: &[&str]) -> Point {
    Point(Vector {
        x: f(c[0]),
        y: f(c[1]),
        z: f(c[2]),
    })
}

fn cap_s(c: Cap) -> [String; 4] {
    let p = c.center();
    [d(p.x()), d(p.y()), d(p.z()), d(c.chord_radius().length2())]
}

fn rect_s(r: Rect) -> [String; 4] {
    [d(r.lat.lo), d(r.lat.hi), d(r.lng.lo), d(r.lng.hi)]
}

/// Spec 010's widening of `Rect::cap_bound` (FR-019), from upstream's cap.
fn widened(up: Cap) -> Cap {
    if up.is_empty() {
        return up;
    }
    let wide = up.expanded(Angle::from_radians(1e-15));
    let radius = wide.chord_radius();
    Cap::from_center_chord_angle(
        wide.center(),
        radius.plus_error(8.0 * f64::EPSILON * radius.length2()),
    )
}

#[test]
fn cell_bounds_match_cpp() {
    for r in section("CELL_BOUNDS") {
        let c = cell(r[0]);
        assert_eq!(cap_s(c.cap_bound()), r[1..5], "cap_bound {}", r[0]);
        assert_eq!(rect_s(c.rect_bound()), r[5..9], "rect_bound {}", r[0]);
    }
}

/// U-017: every answer is upstream's, and the margin's misses are pinned (both sides miss them).
#[test]
fn cell_bound_points_match_cpp() {
    let (mut cap_misses, mut rect_misses, mut contained) = (0, 0, 0);
    for r in section("CELL_BOUND_POINTS") {
        let c = cell(r[0]);
        let p = point(&r[1..4]);
        let inside = c.contains_point(p);
        assert_eq!(b(inside), r[4], "contains_point {r:?}");
        assert_eq!(b(c.cap_bound().contains_point(p)), r[5], "cap {r:?}");
        assert_eq!(b(c.rect_bound().contains_point(p)), r[6], "rect {r:?}");
        if inside {
            contained += 1;
            cap_misses += usize::from(r[5] == "0");
            rect_misses += usize::from(r[6] == "0");
        }
    }
    assert_eq!((contained, cap_misses, rect_misses), (25_469, 2_371, 840));
}

/// A member from its encoding (the generator's `c:`, `r:`, `e:`, `p:`).
fn member(enc: &str) -> Box<dyn Region> {
    let v: Vec<&str> = enc.split(':').collect();
    match v[0] {
        "c" => Box::new(Cap::from_center_chord_angle(
            point(&v[1..4]),
            ChordAngle::from_angle(Angle::from_radians(f(v[4]))),
        )),
        "r" => Box::new(Rect::new(
            r1::Interval::new(f(v[1]), f(v[2])),
            s1::Interval::new(f(v[3]), f(v[4])),
        )),
        "e" => Box::new(cell(v[1])),
        "p" => Box::new(PointRegion::new(point(&v[1..4]))),
        other => panic!("unknown member {other}"),
    }
}

fn check_adaptor(name: &str, build: impl Fn(Vec<Box<dyn Region>>) -> Box<dyn Region>) {
    let rows = section(name);
    for r in &rows {
        let members = if r[0].is_empty() {
            Vec::new()
        } else {
            r[0].split(';').map(member).collect()
        };
        let a = build(members);
        let c = cell(r[1]);
        let p = point(&r[2..5]);
        let up = Cap::from_center_chord_angle(point(&r[5..8]), ChordAngle::from_length2(f(r[8])));
        let cap = a.cap_bound();
        assert_eq!(cap_s(cap)[..3], cap_s(up)[..3], "{name} cap centre {r:?}");
        assert_eq!(
            d(cap.chord_radius().length2()),
            d(widened(up).chord_radius().length2()),
            "{name} cap radius, the documented widening {r:?}"
        );
        assert_eq!(rect_s(a.rect_bound()), r[9..13], "{name} rect_bound {r:?}");
        assert_eq!(
            a.cell_union_bound().len().to_string(),
            r[13],
            "{name} cover {r:?}"
        );
        assert_eq!(b(a.contains_cell(&c)), r[14], "{name} contains_cell {r:?}");
        assert_eq!(
            b(a.intersects_cell(&c)),
            r[15],
            "{name} intersects_cell {r:?}"
        );
        assert_eq!(
            b(a.contains_point(&p)),
            r[16],
            "{name} contains_point {r:?}"
        );
    }
    assert_eq!(rows.len(), 2000);
}

#[test]
fn region_union_matches_cpp() {
    check_adaptor("REGION_UNION", |m| Box::new(RegionUnion::from_regions(m)));
}

#[test]
fn region_intersection_matches_cpp() {
    check_adaptor("REGION_INTERSECTION", |m| {
        Box::new(RegionIntersection::from_regions(m))
    });
}
