// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for `s2::LatLng` and for `s2::Rect` as a region (spec 010 FR-021) against
//! the C++ reference implementation (google/s2geometry), Principle I.
//!
//! The vectors in `data/cpp_latlng_vectors.csv` come from `tools/refgen-cpp/generate_latlng`.
//! Every result is compared bit for bit, except the one documented deviation:
//! - `RECT_CAPBOUND`: `Rect::cap_bound` widens upstream's cap (spec 010 FR-019, `UPSTREAM.md`
//!   U-016). Its centre is upstream's bit for bit, and its radius is upstream's widened by the
//!   documented rule, recomputed here from upstream's value; the library's cap contains every
//!   point the rectangle contains, where upstream's misses 993 of the 5,552 contained points
//!   (pinned).

use s2rst::r1;
use s2rst::r3::Vector;
use s2rst::s1::{self, Angle, ChordAngle};
use s2rst::s2::{Cap, Cell, CellId, LatLng, Point, Rect, Region};

const CSV: &str = include_str!("data/cpp_latlng_vectors.csv");

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

fn b(x: bool) -> String {
    if x { "1" } else { "0" }.to_string()
}

fn ll_bits(ll: LatLng) -> [String; 2] {
    [d(ll.lat.radians()), d(ll.lng.radians())]
}

fn rect(c: &[&str]) -> Rect {
    Rect::new(
        r1::Interval::new(f(c[0]), f(c[1])),
        s1::Interval::new(f(c[2]), f(c[3])),
    )
}

#[test]
fn constructors_match_cpp() {
    let rows = section("LATLNG_CTOR");
    for row in &rows {
        let got = match row[0] {
            "rad" => LatLng::from_radians(f(row[1]), f(row[2])),
            "deg" => LatLng::from_degrees(f(row[1]), f(row[2])),
            "e5" => LatLng::from_e5(row[1].parse().unwrap(), row[2].parse().unwrap()),
            "e6" => LatLng::from_e6(row[1].parse().unwrap(), row[2].parse().unwrap()),
            "e7" => LatLng::from_e7(row[1].parse().unwrap(), row[2].parse().unwrap()),
            "ue6" => LatLng::from_unsigned_e6(row[1].parse().unwrap(), row[2].parse().unwrap()),
            "ue7" => LatLng::from_unsigned_e7(row[1].parse().unwrap(), row[2].parse().unwrap()),
            "invalid" => LatLng::invalid(),
            "default" => LatLng::default(),
            k => panic!("unknown kind {k}"),
        };
        assert_eq!(ll_bits(got), [row[3], row[4]], "{row:?}");
    }
    assert_eq!(rows.len(), 1884);
}

#[test]
fn point_conversions_match_cpp() {
    let rows = section("LATLNG_POINT");
    for row in &rows {
        let p = Point(Vector {
            x: f(row[0]),
            y: f(row[1]),
            z: f(row[2]),
        });
        let want = [row[3], row[4]];
        assert_eq!(
            [
                d(LatLng::latitude(p).radians()),
                d(LatLng::longitude(p).radians())
            ],
            want,
            "{row:?}"
        );
        assert_eq!(ll_bits(LatLng::from_point(p)), want, "{row:?}");
        assert_eq!(ll_bits(LatLng::from(p)), want, "{row:?}");
    }
    assert_eq!(rows.len(), 6014);
}

#[test]
fn normalized_matches_cpp() {
    let rows = section("LATLNG_NORM");
    for row in &rows {
        let n = LatLng::from_radians(f(row[0]), f(row[1])).normalized();
        assert_eq!(ll_bits(n), [row[2], row[3]], "{row:?}");
    }
    assert_eq!(rows.len(), 3032);
}

#[test]
fn operations_match_cpp() {
    let rows = section("LATLNG_OPS");
    for row in &rows {
        let a = LatLng::from_radians(f(row[0]), f(row[1]));
        let bb = LatLng::from_radians(f(row[2]), f(row[3]));
        let (k, m) = (f(row[4]), f(row[5]));
        let want: Vec<&str> = row[6].split(';').collect();
        let p = a.to_point();
        let mut got = vec![
            d(p.0.x),
            d(p.0.y),
            d(p.0.z),
            d(a.get_distance(bb).radians()),
        ];
        got.push(b(a.approx_eq(bb)));
        got.push(b(a.approx_eq_with(bb, Angle::from_radians(m))));
        for r in [a + bb, a - bb, k * a, a * k] {
            got.extend(ll_bits(r));
        }
        got.push(b(a.is_valid()));
        got.push(b(a == bb));
        assert_eq!(got, want, "{row:?}");
        // The `From` conversion is `to_point`.
        assert_eq!(Point::from(a), p);
    }
    assert_eq!(rows.len(), 2759);
}

#[test]
fn to_string_in_degrees_matches_cpp() {
    let rows = section("LATLNG_STR");
    for row in &rows {
        let s = LatLng::from_radians(f(row[0]), f(row[1])).to_string_in_degrees();
        assert_eq!(s, format!("{},{}", row[2], row[3]), "{row:?}");
    }
    assert_eq!(rows.len(), 309);
}

#[test]
fn rect_cell_tests_match_cpp() {
    let rows = section("RECT_REGION");
    for row in &rows {
        let r = rect(row);
        let cell = Cell::from(CellId::from(u64::from_str_radix(row[4], 16).unwrap()));
        assert_eq!(
            [b(r.contains_cell(&cell)), b(r.intersects_cell(&cell))],
            [row[5], row[6]],
            "{row:?}"
        );
    }
    assert_eq!(rows.len(), 10_000);
}

/// Upstream's cap, widened as `Rect::cap_bound` documents (FR-019).
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
fn rect_cap_bound_matches_cpp_but_for_the_widening() {
    let rows = section("RECT_CAPBOUND");
    let (mut contained, mut upstream_misses) = (0, 0);
    for row in &rows {
        let r = rect(row);
        let c = Point(Vector {
            x: f(row[4]),
            y: f(row[5]),
            z: f(row[6]),
        });
        let l2 = f(row[7]);
        let up = if l2 < 0.0 {
            Cap::empty()
        } else {
            Cap::from_center_chord_angle(c, ChordAngle::from_length2(l2))
        };
        let lib = r.cap_bound();
        if !up.is_empty() {
            assert_eq!(lib.center(), c, "the centre is upstream's: {row:?}");
        }
        assert_eq!(
            d(lib.chord_radius().length2()),
            d(widened(up).chord_radius().length2()),
            "the documented widening: {row:?}"
        );
        let p = LatLng::from_radians(f(row[8]), f(row[9])).to_point();
        let inside = r.contains_point(p);
        assert_eq!(b(inside), row[10], "Contains(p): {row:?}");
        assert_eq!(b(up.contains_point(p)), row[11], "upstream's cap: {row:?}");
        if inside {
            contained += 1;
            assert!(lib.contains_point(p), "the bound misses a point: {row:?}");
            if row[11] == "0" {
                upstream_misses += 1;
            }
        }
    }
    assert_eq!(rows.len(), 6004);
    // Upstream's bound misses 993 of the 5,552 contained points (U-016); the library's none.
    assert_eq!((contained, upstream_misses), (5552, 993));
}
