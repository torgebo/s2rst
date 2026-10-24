// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for `s1::Interval` and `Cell` (spec 004 FR-021), `r1::Interval` and
//! `s2::Rect` (spec 005 FR-021), `r2::Rect` and `s2::Point` (spec 006 FR-023), and
//! `s1::ChordAngle` and `s1::Angle` (spec 007 FR-024), against the C++ reference implementation
//! (google/s2geometry), Principle I.
//!
//! The vectors in `data/cpp_values_vectors.csv` come from `tools/refgen-cpp/generate_values`,
//! with the C++ S2 library as an independent oracle. Every result is compared bit for bit.
//! Rows noted `deviation:seam-length` involve the one-ulp interval across the seam, where the
//! library documents its true length and upstream returns −1 (FR-020, `UPSTREAM.md` U-007): on
//! those rows the results computed from that length are checked against the documented values,
//! and every other result against C++. Rows noted `deviation:expanded-crossing` are expansions
//! whose bounds cross in upstream's arithmetic (FR-023, U-011), where the library returns the
//! empty or full interval.

use s2rst::r3::Vector;
use s2rst::s1::{Angle, ChordAngle, Interval};
use s2rst::s2::edge_crossings::{ROBUST_CROSS_PROD_ERROR, robust_cross_prod};
use s2rst::s2::{Cell, CellEdge, CellId, LatLng, Point, Rect, RectVertex, ortho, rotate};
use s2rst::{r1, r2};
use std::f64::consts::PI;

const CSV: &str = include_str!("data/cpp_values_vectors.csv");

/// One ulp of π, the corner's true length.
const ULP_PI: f64 = 4.440_892_098_500_626e-16;

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

fn is_corner(x: Interval) -> bool {
    x.lo == PI && x.hi.to_bits() == (-PI).to_bits() - 1
}

/// The library's 26 results, in the generator's order.
fn results(x: Interval, p: f64, margin: f64, max_error: f64, y: Interval) -> Vec<String> {
    let c = x.complement();
    let a = x.add_point(p);
    let e = x.expanded(margin);
    let u = x.union(y);
    let i = x.intersection(y);
    vec![
        b(x.is_valid()),
        b(x.is_empty()),
        b(x.is_full()),
        b(x.is_inverted()),
        d(x.center()),
        d(x.length()),
        d(c.lo),
        d(c.hi),
        d(x.complement_center()),
        b(x.contains(p)),
        b(x.interior_contains(p)),
        if x.is_empty() {
            "-".to_string()
        } else {
            d(x.project(p))
        },
        d(a.lo),
        d(a.hi),
        d(e.lo),
        d(e.hi),
        b(x.contains_interval(y)),
        b(x.interior_contains_interval(y)),
        b(x.intersects(y)),
        b(x.interior_intersects(y)),
        d(u.lo),
        d(u.hi),
        d(i.lo),
        d(i.hi),
        d(x.directed_hausdorff_distance(y)),
        b(x.approx_eq_with(y, max_error)),
    ]
}

const NAMES: [&str; 26] = [
    "is_valid",
    "is_empty",
    "is_full",
    "is_inverted",
    "center",
    "length",
    "complement.lo",
    "complement.hi",
    "complement_center",
    "contains(p)",
    "interior_contains(p)",
    "project(p)",
    "add_point.lo",
    "add_point.hi",
    "expanded.lo",
    "expanded.hi",
    "contains_interval(y)",
    "interior_contains_interval(y)",
    "intersects(y)",
    "interior_intersects(y)",
    "union.lo",
    "union.hi",
    "intersection.lo",
    "intersection.hi",
    "directed_hausdorff_distance(y)",
    "approx_eq_with(y, max_error)",
];

#[test]
fn s1_interval_matches_cpp() {
    let (mut rows, mut deviations, mut crossings) = (0, 0, 0);
    for name in ["S1INTERVAL_OPS", "S1INTERVAL_SPECIAL"] {
        for row in section(name) {
            let x = Interval {
                lo: f(row[0]),
                hi: f(row[1]),
            };
            let y = Interval {
                lo: f(row[5]),
                hi: f(row[6]),
            };
            let want: Vec<&str> = row[7].split(';').collect();
            let margin = f(row[3]);
            let got = results(x, f(row[2]), margin, f(row[4]), y);
            let notes: Vec<&str> = row[8].split('+').filter(|n| !n.is_empty()).collect();
            let deviation = notes.contains(&"deviation:seam-length");
            let crossing = notes.contains(&"deviation:expanded-crossing");
            assert_eq!(deviation, is_corner(x) || is_corner(y), "{row:?}");
            if crossing {
                // FR-023: upstream's bounds crossed; the library gives the empty interval when
                // shrinking and the full one when growing.
                let want = if margin < 0.0 {
                    Interval::empty()
                } else {
                    Interval::full()
                };
                assert_eq!(x.expanded(margin), want, "{row:?}");
            }
            for k in 0..26 {
                // FR-020: on the corner rows, the results computed from the corner's length
                // are the documented deviation: the length itself, the expansion of the
                // corner, and the intersection and approximate equality that compare lengths.
                // FR-023: on the crossing rows, the expansion (checked above).
                let documented = match k {
                    5 => is_corner(x),
                    14 | 15 => is_corner(x) || crossing,
                    22 | 23 | 25 => deviation,
                    _ => false,
                };
                if documented {
                    if k == 5 {
                        assert_eq!(x.length().to_bits(), ULP_PI.to_bits(), "{row:?}");
                    }
                    continue;
                }
                assert_eq!(got[k], want[k], "{name} {}: {row:?}", NAMES[k]);
            }
            deviations += usize::from(deviation);
            crossings += usize::from(crossing);
            rows += 1;
        }
    }
    assert!(
        rows > 5000 && deviations > 0 && crossings == 4,
        "{rows} rows, {deviations} seam-length deviations, {crossings} crossings"
    );
}

#[test]
fn cell_fields_match_cpp() {
    for row in section("CELL_FIELDS") {
        let c = Cell::from_cell_id(CellId(row[0].parse().unwrap()));
        let b = c.bound_uv();
        let got = [
            c.face().as_u8().to_string(),
            c.level().as_u8().to_string(),
            c.orientation().to_string(),
            d(b.x.lo),
            d(b.x.hi),
            d(b.y.lo),
            d(b.y.hi),
            d(c.uv_coord_of_edge(CellEdge::Bottom)),
            d(c.uv_coord_of_edge(CellEdge::Right)),
            d(c.uv_coord_of_edge(CellEdge::Top)),
            d(c.uv_coord_of_edge(CellEdge::Left)),
            c.size_ij().to_string(),
            if c.is_leaf() { "1" } else { "0" }.to_string(),
        ];
        assert_eq!(got.to_vec(), row[1..14].to_vec(), "{row:?}");
    }
}

#[test]
fn cell_children_match_cpp() {
    for row in section("CELL_CHILDREN") {
        let parent = Cell::from_cell_id(CellId(row[0].parse().unwrap()));
        let k: usize = row[1].parse().unwrap();
        let child = parent.children().expect("a non-leaf parent")[k];
        let b = child.bound_uv();
        let got = [
            child.id().0.to_string(),
            child.orientation().to_string(),
            d(b.x.lo),
            d(b.x.hi),
            d(b.y.lo),
            d(b.y.hi),
        ];
        assert_eq!(got.to_vec(), row[2..8].to_vec(), "{row:?}");
    }
}

// ---- spec 005 ------------------------------------------------------------------------------

/// Equal bit patterns, or both NaN (`∞ − ∞` may produce either sign of NaN).
fn same(got: &str, want: &str) -> bool {
    got == want || (want != "-" && got != "-" && f(got).is_nan() && f(want).is_nan())
}

/// The library's 23 `r1::Interval` results, in the generator's order.
fn r1_results(
    x: r1::Interval,
    p: f64,
    margin: f64,
    max_error: f64,
    y: r1::Interval,
) -> Vec<String> {
    let a = x.add_point(p);
    let e = x.expanded(margin);
    let ai = x.add_interval(y);
    let u = x.union(y);
    let i = x.intersection(y);
    vec![
        b(x.is_empty()),
        d(x.center()),
        d(x.length()),
        b(x.contains(p)),
        b(x.interior_contains(p)),
        if x.is_empty() {
            "-".to_string()
        } else {
            d(x.project(p))
        },
        d(a.lo),
        d(a.hi),
        d(e.lo),
        d(e.hi),
        b(x.contains_interval(y)),
        b(x.interior_contains_interval(y)),
        b(x.intersects(y)),
        b(x.interior_intersects(y)),
        d(ai.lo),
        d(ai.hi),
        d(u.lo),
        d(u.hi),
        d(i.lo),
        d(i.hi),
        d(x.directed_hausdorff_distance(y)),
        b(x.approx_eq_with(y, max_error)),
        b(x == y),
    ]
}

/// The directed Hausdorff distance of two non-empty real intervals, side by side: on each side
/// the excess of `x` beyond `y`, which is 0 when both bounds are the same infinity. This is
/// the true distance, which the library documents at infinite bounds (U-009).
fn true_hausdorff(x: r1::Interval, y: r1::Interval) -> f64 {
    let right = if x.hi == y.hi { 0.0 } else { x.hi - y.hi };
    let left = if x.lo == y.lo { 0.0 } else { y.lo - x.lo };
    right.max(left).max(0.0)
}

#[test]
fn r1_interval_matches_cpp() {
    let (mut rows, mut center_rows, mut infinite_rows) = (0, 0, 0);
    for name in ["R1INTERVAL_OPS", "R1INTERVAL_SPECIAL"] {
        for row in section(name) {
            let x = r1::Interval {
                lo: f(row[0]),
                hi: f(row[1]),
            };
            let y = r1::Interval {
                lo: f(row[5]),
                hi: f(row[6]),
            };
            let want: Vec<&str> = row[7].split(';').collect();
            let got = r1_results(x, f(row[2]), f(row[3]), f(row[4]), y);
            assert_eq!(want.len(), 23, "{row:?}");
            let notes: Vec<&str> = row[8].split('+').filter(|n| !n.is_empty()).collect();
            let center_overflow = notes.contains(&"deviation:center-overflow");
            let infinite_corner = notes.contains(&"deviation:infinite-corner");
            for k in 0..23 {
                // U-010: upstream's centre overflows for finite bounds beyond f64::MAX / 2; the
                // library documents the true centre, which lies between the bounds.
                if k == 1 && center_overflow {
                    let c = x.center();
                    assert!(
                        c.is_finite() && x.lo <= c && c <= x.hi,
                        "center {c}: {row:?}"
                    );
                    continue;
                }
                // U-009: at infinite bounds upstream keeps the NaN of ∞ − ∞ and answers 0; the
                // library documents the true distance.
                if k == 20 && infinite_corner {
                    let got = x.directed_hausdorff_distance(y);
                    assert_eq!(got, true_hausdorff(x, y), "{row:?}");
                    assert_ne!(f(want[20]), got, "{row:?}");
                    continue;
                }
                // Zero bounds chosen by min and max: `f64::min` and `max` do not fix the sign of a
                // zero result, where `std::min` and `max` return their first argument, so the
                // library may give 0 where upstream gives −0. Both are the same number (the
                // type's documentation, "Signed zeros").
                let zero_by_value = matches!(k, 14..=20)
                    && want[k] != "-"
                    && f(&got[k]) == 0.0
                    && f(want[k]) == 0.0;
                assert!(
                    same(&got[k], want[k]) || zero_by_value,
                    "{name} {}: {} vs C++ {}: {row:?}",
                    R1_NAMES[k],
                    got[k],
                    want[k]
                );
            }
            center_rows += usize::from(center_overflow);
            infinite_rows += usize::from(infinite_corner);
            rows += 1;
        }
    }
    assert!(
        rows > 5000 && center_rows > 0 && infinite_rows > 0,
        "{rows} rows, {center_rows} centre deviations, {infinite_rows} Hausdorff deviations"
    );
}

const R1_NAMES: [&str; 23] = [
    "is_empty",
    "center",
    "length",
    "contains(p)",
    "interior_contains(p)",
    "project(p)",
    "add_point.lo",
    "add_point.hi",
    "expanded.lo",
    "expanded.hi",
    "contains_interval(y)",
    "interior_contains_interval(y)",
    "intersects(y)",
    "interior_intersects(y)",
    "add_interval.lo",
    "add_interval.hi",
    "union.lo",
    "union.hi",
    "intersection.lo",
    "intersection.hi",
    "directed_hausdorff_distance(y)",
    "approx_eq_with(y, max_error)",
    "eq(y)",
];

fn rect_bounds(out: &mut Vec<String>, r: Rect) {
    out.extend([d(r.lat.lo), d(r.lat.hi), d(r.lng.lo), d(r.lng.hi)]);
}

/// The library's 49 `s2::Rect` results, in the generator's order. `-` where the generator
/// skips an invalid latitude–longitude.
fn rect_results(a: Rect, ll: LatLng, m: LatLng, other: Rect) -> Vec<String> {
    let mut out = vec![
        b(a.is_valid()),
        b(a.is_empty()),
        b(a.is_full()),
        b(a.is_point()),
        b(a.is_inverted()),
    ];
    for v in [
        RectVertex::LowerLeft,
        RectVertex::LowerRight,
        RectVertex::UpperRight,
        RectVertex::UpperLeft,
    ] {
        let p = a.vertex(v);
        out.extend([d(p.lat.radians()), d(p.lng.radians())]);
    }
    let (c, sz) = (a.center(), a.size());
    out.extend([
        d(c.lat.radians()),
        d(c.lng.radians()),
        d(sz.lat.radians()),
        d(sz.lng.radians()),
    ]);
    let dash = || "-".to_string();
    if ll.is_valid() {
        out.extend([
            b(a.contains_lat_lng(ll)),
            b(a.interior_contains_lat_lng(ll)),
        ]);
        rect_bounds(&mut out, a.add_point(ll));
    } else {
        out.extend(std::iter::repeat_with(dash).take(6));
    }
    rect_bounds(&mut out, a.expanded(m));
    rect_bounds(&mut out, a.polar_closure());
    out.extend([
        b(a.contains(other)),
        b(a.interior_contains(other)),
        b(a.intersects(other)),
        b(a.interior_intersects(other)),
    ]);
    rect_bounds(&mut out, a.union(other));
    rect_bounds(&mut out, a.intersection(other));
    out.extend([b(a.approx_eq(other)), b(a == other)]);
    if ll.is_valid() {
        let size = LatLng::from_radians(m.lat.radians().abs(), m.lng.radians().abs());
        rect_bounds(&mut out, Rect::from_center_size(ll, size));
    } else {
        out.extend(std::iter::repeat_with(dash).take(4));
    }
    out
}

fn rect_of(c: &[&str]) -> Rect {
    Rect {
        lat: r1::Interval {
            lo: f(c[0]),
            hi: f(c[1]),
        },
        lng: Interval {
            lo: f(c[2]),
            hi: f(c[3]),
        },
    }
}

#[test]
fn rect_matches_cpp() {
    let (mut rows, mut invalid_points, mut deviations) = (0, 0, 0);
    for name in ["RECT_OPS", "RECT_SPECIAL"] {
        for row in section(name) {
            let (a, other) = (rect_of(&row[0..4]), rect_of(&row[8..12]));
            let ll = LatLng::from_radians(f(row[4]), f(row[5]));
            let m = LatLng::from_radians(f(row[6]), f(row[7]));
            let want: Vec<&str> = row[12].split(';').collect();
            assert_eq!(want.len(), 49, "{row:?}");
            let got = rect_results(a, ll, m, other);
            let deviation = row[13] == "deviation:seam-length";
            assert_eq!(
                deviation,
                is_corner(a.lng) || is_corner(other.lng),
                "{row:?}"
            );
            for k in 0..49 {
                // Spec 004 FR-020: the results computed from the seam corner's length are the
                // documented deviation: its size, its expansion, and the intersection and
                // approximate equality that compare lengths.
                let from_length = match k {
                    16 | 23..=26 => is_corner(a.lng),
                    39..=43 => deviation,
                    _ => false,
                };
                if from_length {
                    if k == 16 {
                        assert_eq!(
                            a.size().lng.radians().to_bits(),
                            ULP_PI.to_bits(),
                            "{row:?}"
                        );
                    }
                    continue;
                }
                assert!(
                    same(&got[k], want[k]),
                    "{name} result {k}: {} vs C++ {}: {row:?}",
                    got[k],
                    want[k]
                );
            }
            // Spec 005 K2: an invalid point is never contained and never added (C++ has no
            // defined answer: it only logs the point).
            if !ll.is_valid() {
                assert!(!a.contains_lat_lng(ll), "{row:?}");
                assert_eq!(a.add_point(ll), a, "{row:?}");
                invalid_points += 1;
            }
            deviations += usize::from(deviation);
            rows += 1;
        }
    }
    assert!(
        rows > 2000 && invalid_points > 0 && deviations > 0,
        "{rows} rows, {invalid_points} invalid points, {deviations} seam-corner rows"
    );
}

// ---- spec 006: r2::Rect --------------------------------------------------------------------

fn uv_bounds(out: &mut Vec<String>, r: r2::Rect) {
    for x in [r.x.lo, r.x.hi, r.y.lo, r.y.hi] {
        out.push(d(x));
    }
}

/// The library's 53 `r2::Rect` results, in the generator's order.
fn r2_results(
    a: r2::Rect,
    p: r2::Point,
    m: r2::Point,
    max_error: f64,
    other: r2::Rect,
) -> Vec<String> {
    let dash = || "-".to_string();
    let mut out = vec![b(a.is_valid()), b(a.is_empty())];
    for k in 0..4 {
        out.push(d(a.vertex(k).x));
        out.push(d(a.vertex(k).y));
    }
    out.push(d(a.center().x));
    out.push(d(a.center().y));
    if a.is_empty() {
        out.extend([dash(), dash()]);
    } else {
        out.push(d(a.size().x));
        out.push(d(a.size().y));
    }
    out.push(b(a.contains_point(p)));
    out.push(b(a.interior_contains_point(p)));
    uv_bounds(&mut out, a.add_point(p));
    if a.is_empty() {
        out.extend([dash(), dash()]);
    } else {
        out.push(d(a.project(p).x));
        out.push(d(a.project(p).y));
    }
    uv_bounds(&mut out, a.expanded(m));
    uv_bounds(&mut out, a.expanded_by_margin(m.x));
    out.push(b(a.contains(other)));
    out.push(b(a.interior_contains(other)));
    out.push(b(a.intersects(other)));
    out.push(b(a.interior_intersects(other)));
    uv_bounds(&mut out, a.add_rect(other));
    uv_bounds(&mut out, a.union(other));
    uv_bounds(&mut out, a.intersection(other));
    out.push(b(a.approx_eq_with(other, max_error)));
    out.push(b(a.approx_eq(other)));
    out.push(b(a == other));
    let size = r2::Point::new(m.x.abs(), m.y.abs());
    uv_bounds(&mut out, r2::Rect::from_center_size(p, size));
    out
}

fn uv_rect_of(c: &[&str]) -> r2::Rect {
    r2::Rect {
        x: r1::Interval {
            lo: f(c[0]),
            hi: f(c[1]),
        },
        y: r1::Interval {
            lo: f(c[2]),
            hi: f(c[3]),
        },
    }
}

/// Every `R2RECT_*` row, bit for bit. Rows noted `deviation:center-overflow` have a bound beyond
/// `f64::MAX / 2`, where the library's centre is the true one (U-010, through
/// `r1::Interval::center`). Rows noted `deviation:infinite-corner` have a NaN result from
/// `∞ − ∞`: the library computes the same arithmetic and documents these corners (FR-018), so
/// they are compared as the others, NaN against NaN.
#[test]
fn r2_rect_matches_cpp() {
    let (mut rows, mut center_rows, mut infinite_rows) = (0, 0, 0);
    for name in ["R2RECT_OPS", "R2RECT_SPECIAL"] {
        for row in section(name) {
            let (a, other) = (uv_rect_of(&row[0..4]), uv_rect_of(&row[9..13]));
            let p = r2::Point::new(f(row[4]), f(row[5]));
            let m = r2::Point::new(f(row[6]), f(row[7]));
            let want: Vec<&str> = row[13].split(';').collect();
            assert_eq!(want.len(), 53, "{row:?}");
            let got = r2_results(a, p, m, f(row[8]), other);
            let notes: Vec<&str> = row[14].split('+').filter(|n| !n.is_empty()).collect();
            let center_overflow = notes.contains(&"deviation:center-overflow");
            for k in 0..53 {
                if (k == 10 || k == 11) && center_overflow {
                    let (c, x) = if k == 10 {
                        (a.center().x, a.x)
                    } else {
                        (a.center().y, a.y)
                    };
                    if !same(&got[k], want[k]) {
                        assert!(
                            c.is_finite() && x.lo <= c && c <= x.hi,
                            "center {c}: {row:?}"
                        );
                    }
                    continue;
                }
                // Bounds chosen by `f64::min` and `max` (adding a point or a rectangle, the union
                // and the intersection) may be 0 where upstream's `std::min` and `max` give −0, as
                // for `r1::Interval`: the same number.
                let zero_by_value =
                    matches!(k, 16..=19 | 34..=45) && f(&got[k]) == 0.0 && f(want[k]) == 0.0;
                assert!(
                    same(&got[k], want[k]) || zero_by_value,
                    "{name} result {k}: {} vs C++ {}: {row:?}",
                    got[k],
                    want[k]
                );
            }
            center_rows += usize::from(center_overflow);
            infinite_rows += usize::from(notes.contains(&"deviation:infinite-corner"));
            rows += 1;
        }
    }
    assert!(
        rows > 1200 && center_rows > 0 && infinite_rows > 0,
        "{rows} rows, {center_rows} centre deviations, {infinite_rows} infinite corners"
    );
}

// ---- spec 006: s2::Point -------------------------------------------------------------------

fn point_of(c: &[&str]) -> Point {
    Point(Vector {
        x: f(c[0]),
        y: f(c[1]),
        z: f(c[2]),
    })
}

fn push_point(out: &mut Vec<String>, p: Point) {
    out.extend([d(p.x()), d(p.y()), d(p.z())]);
}

fn finite(p: Point) -> bool {
    p.x().is_finite() && p.y().is_finite() && p.z().is_finite()
}

/// The library's 18 `s2::Point` results, in the generator's order, with `-` where the
/// generator computes none (a non-finite input, or a point off unit length for the operations
/// that require it).
fn point_results(a: Point, b_: Point, angle: f64) -> Vec<String> {
    let dash = || "-".to_string();
    let mut out = vec![b(a < b_), b(b_ < a), b(a == b_), b(a.is_unit())];
    push_point(&mut out, a.normalize());
    let both_finite = finite(a) && finite(b_);
    let unit = both_finite && a.is_unit() && b_.is_unit();
    if both_finite {
        push_point(&mut out, ortho(a));
    } else {
        out.extend([dash(), dash(), dash()]);
    }
    if unit {
        push_point(&mut out, robust_cross_prod(a, b_));
    } else {
        out.extend([dash(), dash(), dash()]);
    }
    out.push(if both_finite {
        d(a.distance(b_).radians())
    } else {
        dash()
    });
    out.push(if unit {
        d(a.chord_angle(b_).length2())
    } else {
        dash()
    });
    if unit {
        push_point(&mut out, rotate(a, b_, Angle::from_radians(angle)));
    } else {
        out.extend([dash(), dash(), dash()]);
    }
    out
}

/// Every `S2POINT_*` row, bit for bit, except the three documented deviations, each checked
/// against what the library documents instead:
/// - `deviation:normalize-zero-sign`: `normalize` (and `ortho`, whose cross product is then
///   zero too) returns positive zeros where C++ keeps the coordinates' signs (FR-022, K12): the
///   same values;
/// - `deviation:robust-cross-overflow`: C++'s robust cross product is not finite (U-012), the
///   library's is finite, non-zero and normalizable (FR-021, SC-010);
/// - `deviation:robust-cross-long-double`: C++ answered from its `long double` step, with twice
///   the cross product; the library's exact step returns it normalised. Each is within
///   `ROBUST_CROSS_PROD_ERROR` of the true cross product, so they are within twice that of
///   each other (FR-021).
///
/// `distance` is `atan2` of the cross product's norm and the dot product, as upstream's
/// `S1Angle(a, b)`, and Rust's `f64::atan2` is the platform's, so it is compared bit for bit in
/// the default build, whose products are unfused (K9).
#[test]
fn s2_point_matches_cpp() {
    let (mut rows, mut zero_sign, mut overflow, mut long_double) = (0, 0, 0, 0);
    for name in ["S2POINT_OPS", "S2POINT_SPECIAL"] {
        for row in section(name) {
            let (a, b_) = (point_of(&row[0..3]), point_of(&row[3..6]));
            let want: Vec<&str> = row[7].split(';').collect();
            assert_eq!(want.len(), 18, "{row:?}");
            let got = point_results(a, b_, f(row[6]));
            let notes: Vec<&str> = row[8].split('+').filter(|n| !n.is_empty()).collect();
            let is_zero_sign = notes.contains(&"deviation:normalize-zero-sign");
            let is_overflow = notes.contains(&"deviation:robust-cross-overflow");
            let is_long_double = notes.contains(&"deviation:robust-cross-long-double");
            assert_eq!(is_zero_sign, a.0.norm2() == 0.0, "zero-sign note: {row:?}");
            for k in 0..18 {
                // K12: `normalize` returns positive zeros where C++ keeps the coordinates' signs.
                if is_zero_sign && matches!(k, 4..=6) {
                    assert_eq!(got[k], d(0.0), "{row:?}");
                    assert_eq!(f(want[k]), 0.0, "{row:?}");
                    continue;
                }
                // `ortho` normalises its cross product with `a`, which may vanish with `a`'s.
                if is_zero_sign && matches!(k, 7..=9) && want[k] != "-" && f(&got[k]) == f(want[k])
                {
                    continue;
                }
                if (10..=12).contains(&k) && (is_overflow || is_long_double) {
                    continue;
                }
                // Where the cross and dot products both vanish (a zero vector, or coordinates so
                // small that every product underflows), `atan2(0, ±0)` is 0 or π by the sign of
                // a zero alone. Upstream's `DotProd` starts its sum at +0, so products that are
                // all −0 sum to +0 there and −0 here: (2⁻¹⁰⁷⁴, 0, 0) and its antipode are 0 apart
                // in C++ and π apart here.
                if k == 13 && a.0.cross(b_.0).norm() == 0.0 && a.0.dot(b_.0) == 0.0 {
                    continue;
                }
                assert!(
                    same(&got[k], want[k]),
                    "{name} {}: {} vs C++ {}: {row:?}",
                    POINT_NAMES[k],
                    got[k],
                    want[k]
                );
            }
            if is_overflow || is_long_double {
                let r = robust_cross_prod(a, b_);
                assert!(finite(r) && r.is_normalizable(), "{r:?}: {row:?}");
                let c = Vector {
                    x: f(want[10]),
                    y: f(want[11]),
                    z: f(want[12]),
                };
                if is_long_double {
                    let angle = r.0.angle(c);
                    assert!(
                        angle <= 2.0 * ROBUST_CROSS_PROD_ERROR.radians(),
                        "{angle} rad from C++: {row:?}"
                    );
                } else {
                    assert!(
                        !(c.x.is_finite() && c.y.is_finite() && c.z.is_finite()),
                        "{row:?}"
                    );
                }
            }
            zero_sign += usize::from(is_zero_sign);
            overflow += usize::from(is_overflow);
            long_double += usize::from(is_long_double);
            rows += 1;
        }
    }
    assert!(
        rows > 1600 && zero_sign > 0 && overflow == 4 && long_double > 0,
        "{rows} rows, {zero_sign} zero-sign, {overflow} overflow, {long_double} long double"
    );
}

const POINT_NAMES: [&str; 18] = [
    "a < b",
    "b < a",
    "a == b",
    "is_unit",
    "normalize.x",
    "normalize.y",
    "normalize.z",
    "ortho.x",
    "ortho.y",
    "ortho.z",
    "robust_cross_prod.x",
    "robust_cross_prod.y",
    "robust_cross_prod.z",
    "distance",
    "chord_angle",
    "rotate.x",
    "rotate.y",
    "rotate.z",
];

/// The value an E-form saturates to (spec 007 FR-023): `i32::MAX` above the range (+∞
/// included), `i32::MIN` below it, 0 for NaN. `v` is out of the `i32` range.
fn saturated(v: f64) -> String {
    if v.is_nan() {
        "0".to_string()
    } else if v > 0.0 {
        i32::MAX.to_string()
    } else {
        i32::MIN.to_string()
    }
}

/// The library's 24 `ChordAngle` results, in the generator's order (spec 007 R9).
fn chord_results(a: ChordAngle, l: f64, m: f64, e: f64) -> Vec<String> {
    let b_ = ChordAngle::from_length2(m);
    let special = a.is_special();
    let dash = || "-".to_string();
    let angle = a.to_angle();
    vec![
        d(a.length2()),
        b(a.is_zero()),
        b(a.is_negative()),
        b(a.is_infinity()),
        b(a.is_special()),
        b(a.is_valid()),
        d(a.successor().length2()),
        d(a.predecessor().length2()),
        d(a.plus_error(e).length2()),
        d(a.max_point_error()),
        d(a.max_angle_error()),
        if special { dash() } else { d(a.sin2()) },
        if special { dash() } else { d(a.cos()) },
        if special { dash() } else { d(a.sin()) },
        if special { dash() } else { d(a.tan()) },
        d(angle.radians()),
        a.e5().to_string(),
        a.e6().to_string(),
        a.e7().to_string(),
        if special || b_.is_special() {
            dash()
        } else {
            d((a + b_).length2())
        },
        if special || b_.is_special() {
            dash()
        } else {
            d((a - b_).length2())
        },
        b(a < b_),
        b(a == b_),
        d(ChordAngle::from_angle(Angle::from_radians(l)).length2()),
    ]
}

/// Spec 007 FR-024: every `CHORD_ANGLE` row, bit for bit, except the documented deviations:
/// `deviation:negative-canonical` (C++ keeps a negative length other than −1, the library
/// makes it `NEGATIVE`, FR-017) and `deviation:eform-saturate` (C++'s out-of-range E-forms are
/// −2147483648, the library saturates, FR-023, U-014).
#[test]
fn chord_angle_matches_cpp() {
    let (mut rows, mut negative, mut saturate) = (0, 0, 0);
    for row in section("CHORD_ANGLE") {
        let (l, m, e) = (f(row[1]), f(row[2]), f(row[3]));
        let a = if row[0] == "I" {
            ChordAngle::INFINITY
        } else {
            ChordAngle::from_length2(l)
        };
        let want: Vec<&str> = row[4].split(';').collect();
        assert_eq!(want.len(), 24, "{row:?}");
        let got = chord_results(a, l, m, e);
        let notes: Vec<&str> = row[5].split('+').filter(|n| !n.is_empty()).collect();
        let is_negative = notes.contains(&"deviation:negative-canonical");
        let is_saturate = notes.contains(&"deviation:eform-saturate");
        for k in 0..24 {
            if is_negative && k != 23 {
                // FR-017: the library's value is `NEGATIVE`; check it as −1.
                let neg = chord_results(ChordAngle::NEGATIVE, -1.0, m, e);
                assert_eq!(got[k], neg[k], "{row:?}");
                continue;
            }
            if is_saturate && (16..=18).contains(&k) && want[k] == i32::MIN.to_string() {
                let v = [1e5, 1e6, 1e7][k - 16] * a.to_angle().degrees();
                assert_eq!(got[k], saturated(v), "e{}: {row:?}", k - 11);
                continue;
            }
            assert!(
                same(&got[k], want[k]),
                "CHORD_ANGLE result {k}: {} vs C++ {}: {row:?}",
                got[k],
                want[k]
            );
        }
        if is_negative {
            assert_eq!(a.length2().to_bits(), (-1.0f64).to_bits(), "{row:?}");
            assert_eq!(f(want[0]).to_bits(), l.to_bits(), "C++ keeps {l}: {row:?}");
        }
        negative += usize::from(is_negative);
        saturate += usize::from(is_saturate);
        rows += 1;
    }
    assert!(
        rows > 950 && negative >= 18 && saturate >= 5,
        "{rows} rows, {negative} negative-canonical, {saturate} saturating"
    );
}

/// The library's 21 `Angle` results, in the generator's order (spec 007 R9). Result 6 is
/// upstream's `IsNormalized` of the input, stated here (the library has no such method).
fn angle_results(r: f64, s: f64, n: i32) -> Vec<String> {
    let a = Angle::from_radians(r);
    let x = Angle::from_radians(s);
    let u = u32::from_ne_bytes(n.to_ne_bytes());
    vec![
        d(Angle::from_degrees(r).radians()),
        d(a.degrees()),
        a.e5().to_string(),
        a.e6().to_string(),
        a.e7().to_string(),
        d(a.normalized().radians()),
        b(-PI < r && r <= PI),
        d(a.abs().radians()),
        d(a.sin()),
        d(a.cos()),
        d(a.tan()),
        d((a + x).radians()),
        d((a - x).radians()),
        d((a * s).radians()),
        d((a / s).radians()),
        d(a / x),
        d(Angle::from_e5(n).radians()),
        d(Angle::from_e6(n).radians()),
        d(Angle::from_e7(n).radians()),
        d(Angle::from_unsigned_e6(u).radians()),
        d(Angle::from_unsigned_e7(u).radians()),
    ]
}

/// Spec 007 FR-024: every `ANGLE` row, bit for bit, `normalized` included with no tolerance
/// (SC-008), except the E-forms out of the `int32` range (`deviation:eform-saturate`), where the
/// library saturates (FR-023, U-014).
#[test]
fn angle_matches_cpp() {
    let (mut rows, mut saturate) = (0, 0);
    for row in section("ANGLE") {
        let (r, s) = (f(row[0]), f(row[1]));
        let n: i32 = row[2].parse().unwrap();
        let want: Vec<&str> = row[3].split(';').collect();
        assert_eq!(want.len(), 21, "{row:?}");
        let got = angle_results(r, s, n);
        let is_saturate = row[4] == "deviation:eform-saturate";
        for k in 0..21 {
            if (2..=4).contains(&k) {
                let v = [1e5, 1e6, 1e7][k - 2] * Angle::from_radians(r).degrees();
                if !(-2_147_483_648.5..2_147_483_647.5).contains(&v) {
                    assert!(is_saturate, "{row:?}");
                    assert_eq!(want[k], i32::MIN.to_string(), "C++: {row:?}");
                    assert_eq!(got[k], saturated(v), "e{}: {row:?}", k + 3);
                    continue;
                }
            }
            assert!(
                same(&got[k], want[k]),
                "ANGLE result {k}: {} vs C++ {}: {row:?}",
                got[k],
                want[k]
            );
        }
        saturate += usize::from(is_saturate);
        rows += 1;
    }
    assert!(
        rows > 1000 && saturate > 100,
        "{rows} rows, {saturate} saturating"
    );
}
