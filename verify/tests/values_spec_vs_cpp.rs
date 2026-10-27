// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The reference descriptions of spec 004 (`s2rst_verify::arc_spec`, `s2rst_verify::cell_spec`),
//! spec 005 (`s2rst_verify::real_spec`, `s2rst_verify::rect_spec`) and spec 006
//! (`s2rst_verify::uv_spec`, `s2rst_verify::point_spec`), spec 007
//! (`s2rst_verify::chord_spec`, `s2rst_verify::angle_spec`) and spec 008
//! (`s2rst_verify::cap_spec`, against `cpp_cap_vectors.csv`) against every row of the committed
//! C++ oracle vectors (FR-007, FR-021; spec 004 research R10, specs 005–008 research R9).
//!
//! The Kani proofs show the library agrees with the descriptions; this test shows the
//! descriptions agree with C++. Together: C++ ⇔ description ⇔ library. The description of an
//! interval is upstream's own meaning, so the rows marked `deviation:seam-length` are checked
//! against C++ too: the deviation is the library's, not the description's. So are the
//! restatements of upstream's arithmetic, `upstream_length` and `upstream_expanded` (FR-023),
//! which must give C++'s results bit for bit, crossed bounds included.

use s2rst::s1::Interval;
use s2rst::s2::{Point, Rect};
use s2rst::{r1, r2};
use s2rst_verify::{
    angle_spec, arc_spec, cap_spec, cell_spec, chord_spec, point_spec, real_spec, rect_spec, spec,
    uv_spec,
};

const CSV: &str = include_str!("../../core/tests/data/cpp_values_vectors.csv");
const CAP_CSV: &str = include_str!("../../core/tests/data/cpp_cap_vectors.csv");

/// The rows of one `# SECTION`, split at commas.
fn section(name: &str) -> Vec<Vec<&'static str>> {
    section_of(CSV, name)
}

/// The rows of one `# SECTION` of `csv`, split at commas.
fn section_of(csv: &'static str, name: &str) -> Vec<Vec<&'static str>> {
    let mut rows = Vec::new();
    let mut inside = false;
    for line in csv.lines() {
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
    f64::from_bits(u64::from_str_radix(hex, 16).unwrap_or_else(|e| panic!("{hex}: {e}")))
}

fn b(s: &str) -> bool {
    match s {
        "0" => false,
        "1" => true,
        _ => panic!("not a boolean: {s}"),
    }
}

fn iv(lo: &str, hi: &str) -> Interval {
    Interval {
        lo: f(lo),
        hi: f(hi),
    }
}

#[test]
fn arc_description_matches_cpp() {
    let (mut n, mut crossings) = (0, 0);
    for name in ["S1INTERVAL_OPS", "S1INTERVAL_SPECIAL"] {
        for row in section(name) {
            let (x, p, y) = (iv(row[0], row[1]), f(row[2]), iv(row[5], row[6]));
            let margin = f(row[3]);
            let r: Vec<&str> = row[7].split(';').collect();
            let notes: Vec<&str> = row[8].split('+').filter(|n| !n.is_empty()).collect();
            assert_eq!(r.len(), 26, "{row:?}");
            let ctx = || format!("{name}: {row:?}");
            assert!(arc_spec::valid(x.lo, x.hi) && b(r[0]), "{}", ctx());
            assert_eq!(arc_spec::empty(x), b(r[1]), "{}", ctx());
            assert_eq!(arc_spec::full(x), b(r[2]), "{}", ctx());
            assert_eq!(arc_spec::inverted(x), b(r[3]), "{}", ctx());
            assert_eq!(
                arc_spec::upstream_length(x).to_bits(),
                f(r[5]).to_bits(),
                "length: {}",
                ctx()
            );
            assert_eq!(arc_spec::member(x, p), b(r[9]), "contains(p): {}", ctx());
            assert_eq!(
                arc_spec::interior(x, p),
                b(r[10]),
                "interior_contains(p): {}",
                ctx()
            );
            assert_eq!(arc_spec::contains(x, y), b(r[16]), "contains(y): {}", ctx());
            assert_eq!(
                arc_spec::interior_contains(x, y),
                b(r[17]),
                "interior_contains(y): {}",
                ctx()
            );
            assert_eq!(
                arc_spec::intersects(x, y),
                b(r[18]),
                "intersects(y): {}",
                ctx()
            );
            assert_eq!(
                arc_spec::interior_intersects(x, y),
                b(r[19]),
                "interior_intersects(y): {}",
                ctx()
            );
            assert_eq!(
                notes.contains(&"deviation:seam-length"),
                arc_spec::is_corner(x) || arc_spec::is_corner(y),
                "{}",
                ctx()
            );
            // Off the corner, whose expansion follows the corrected length (FR-020), the
            // restatement is C++'s `Expanded`, and the crossing rows are exactly those where
            // its result breaks containment in the description's terms.
            if !arc_spec::is_corner(x) {
                let u = arc_spec::upstream_expanded(x, margin);
                assert_eq!(
                    [u.lo.to_bits(), u.hi.to_bits()],
                    [f(r[14]).to_bits(), f(r[15]).to_bits()],
                    "expanded: {}",
                    ctx()
                );
                let kept = if margin >= 0.0 {
                    arc_spec::contains(u, x)
                } else {
                    arc_spec::contains(x, u)
                };
                assert_eq!(
                    notes.contains(&"deviation:expanded-crossing"),
                    !kept,
                    "{}",
                    ctx()
                );
                crossings += usize::from(!kept);
            }
            n += 1;
        }
    }
    assert!(
        n > 5000 && crossings == 4,
        "{n} rows, {crossings} crossings"
    );
}

#[test]
fn cell_description_matches_cpp() {
    for row in section("CELL_FIELDS") {
        let id: u64 = row[0].parse().unwrap();
        let d = spec::decompose(id).unwrap_or_else(|| panic!("invalid id {id}"));
        let m = cell_spec::model(d);
        let num = |s: &str| s.parse::<u32>().unwrap();
        assert_eq!(
            (
                u32::from(m.face),
                u32::from(m.level),
                u32::from(m.orientation)
            ),
            (num(row[1]), num(row[2]), num(row[3])),
            "{row:?}"
        );
        let [[u_lo, u_hi], [v_lo, v_hi]] = cell_spec::uv_bounds(m);
        let want = [u_lo, u_hi, v_lo, v_hi].map(f64::to_bits);
        let got = [row[4], row[5], row[6], row[7]].map(|h| f(h).to_bits());
        assert_eq!(want, got, "bounds: {row:?}");
        // Edges 0..3: bottom (v lo), right (u hi), top (v hi), left (u lo).
        let edges = [v_lo, u_hi, v_hi, u_lo].map(f64::to_bits);
        let got = [row[8], row[9], row[10], row[11]].map(|h| f(h).to_bits());
        assert_eq!(edges, got, "edges: {row:?}");
        assert_eq!(m.size, num(row[12]), "{row:?}");
        assert_eq!(m.level == 30, b(row[13]), "{row:?}");
    }
}

#[test]
fn children_description_matches_cpp() {
    for row in section("CELL_CHILDREN") {
        let parent = spec::decompose(row[0].parse().unwrap()).unwrap();
        let k: u8 = row[1].parse().unwrap();
        let child = cell_spec::child_model(parent, k);
        assert_eq!(
            spec::compose(child),
            row[2].parse::<u64>().unwrap(),
            "{row:?}"
        );
        let m = cell_spec::model(child);
        let o: u8 = row[3].parse().unwrap();
        assert_eq!(m.orientation, o, "{row:?}");
        assert_eq!(
            cell_spec::child_orientation(cell_spec::model(parent).orientation, k),
            o,
            "{row:?}"
        );
        let [[u_lo, u_hi], [v_lo, v_hi]] = cell_spec::uv_bounds(m);
        let want = [u_lo, u_hi, v_lo, v_hi].map(f64::to_bits);
        let got = [row[4], row[5], row[6], row[7]].map(|h| f(h).to_bits());
        assert_eq!(want, got, "{row:?}");
    }
}

fn real(lo: &str, hi: &str) -> r1::Interval {
    r1::Interval {
        lo: f(lo),
        hi: f(hi),
    }
}

fn rect(c: &[&str]) -> Rect {
    Rect {
        lat: real(c[0], c[1]),
        lng: iv(c[2], c[3]),
    }
}

/// The real-set description against every `R1INTERVAL_*` row: emptiness (result 0), membership
/// and interior membership of `p` (3, 4), and the four relations with `y` (10–13). The
/// deviations noted on some rows are the library's (centre, Hausdorff distance), never the
/// description's.
#[test]
fn real_description_matches_cpp() {
    let mut n = 0;
    for name in ["R1INTERVAL_OPS", "R1INTERVAL_SPECIAL"] {
        for row in section(name) {
            let (x, p, y) = (real(row[0], row[1]), f(row[2]), real(row[5], row[6]));
            let r: Vec<&str> = row[7].split(';').collect();
            assert_eq!(r.len(), 23, "{row:?}");
            assert_eq!(real_spec::empty(x), b(r[0]), "empty: {row:?}");
            assert_eq!(real_spec::member(x, p), b(r[3]), "member: {row:?}");
            assert_eq!(real_spec::interior(x, p), b(r[4]), "interior: {row:?}");
            assert_eq!(real_spec::contains(x, y), b(r[10]), "contains: {row:?}");
            assert_eq!(
                real_spec::interior_contains(x, y),
                b(r[11]),
                "interior_contains: {row:?}"
            );
            assert_eq!(real_spec::intersects(x, y), b(r[12]), "intersects: {row:?}");
            assert_eq!(
                real_spec::interior_intersects(x, y),
                b(r[13]),
                "interior_intersects: {row:?}"
            );
            n += 1;
        }
    }
    assert!(n > 5000, "{n} rows");
}

/// The rectangle description against every `RECT_*` row: validity and emptiness (results 0,
/// 1), membership and interior membership of a valid latitude–longitude (17, 18), and the four
/// relations with `b` (31–34).
#[test]
fn rect_description_matches_cpp() {
    let mut n = 0;
    let mut members = 0;
    for name in ["RECT_OPS", "RECT_SPECIAL"] {
        for row in section(name) {
            let (a, b_) = (rect(&row[0..4]), rect(&row[8..12]));
            let (lat, lng) = (f(row[4]), f(row[5]));
            let r: Vec<&str> = row[12].split(';').collect();
            assert_eq!(r.len(), 49, "{row:?}");
            assert_eq!(rect_spec::valid(a), b(r[0]), "valid: {row:?}");
            assert_eq!(rect_spec::empty(a), b(r[1]), "empty: {row:?}");
            assert_eq!(
                r[17] == "-",
                !rect_spec::ll_valid(lat, lng),
                "ll_valid: {row:?}"
            );
            if r[17] != "-" {
                assert_eq!(rect_spec::member(a, lat, lng), b(r[17]), "member: {row:?}");
                assert_eq!(
                    rect_spec::interior_member(a, lat, lng),
                    b(r[18]),
                    "interior_member: {row:?}"
                );
                members += 1;
            }
            assert_eq!(rect_spec::contains(a, b_), b(r[31]), "contains: {row:?}");
            assert_eq!(
                rect_spec::interior_contains(a, b_),
                b(r[32]),
                "interior_contains: {row:?}"
            );
            assert_eq!(
                rect_spec::intersects(a, b_),
                b(r[33]),
                "intersects: {row:?}"
            );
            assert_eq!(
                rect_spec::interior_intersects(a, b_),
                b(r[34]),
                "interior_intersects: {row:?}"
            );
            n += 1;
        }
    }
    assert!(
        n > 2000 && members > 1500,
        "{n} rows, {members} with a valid point"
    );
}

// ---- spec 006 ------------------------------------------------------------------------------

fn uv_rect(c: &[&str]) -> r2::Rect {
    r2::Rect {
        x: real(c[0], c[1]),
        y: real(c[2], c[3]),
    }
}

/// The u-v rectangle description against every `R2RECT_*` row: upstream's validity check and
/// emptiness (results 0, 1), the four vertices (2–9), membership and interior membership of
/// `p` (14, 15), and the four relations with `b` (30–33). No row has a NaN input, so the
/// emptiness rule and `uv_spec::valid` agree on every one.
#[test]
fn uv_description_matches_cpp() {
    for (name, min) in [("R2RECT_OPS", 800), ("R2RECT_SPECIAL", 400)] {
        let mut n = 0;
        for row in section(name) {
            let (a, b_) = (uv_rect(&row[0..4]), uv_rect(&row[9..13]));
            let p = r2::Point {
                x: f(row[4]),
                y: f(row[5]),
            };
            let r: Vec<&str> = row[13].split(';').collect();
            assert_eq!(r.len(), 53, "{row:?}");
            assert_eq!(uv_spec::emptiness_rule(a), b(r[0]), "is_valid: {row:?}");
            assert_eq!(uv_spec::valid(a), b(r[0]), "valid: {row:?}");
            assert_eq!(uv_spec::empty(a), b(r[1]), "is_empty: {row:?}");
            for k in 0..4 {
                let v = uv_spec::vertex(a, k);
                let i = 2 + 2 * usize::try_from(k).expect("small");
                assert_eq!(v.x.to_bits(), f(r[i]).to_bits(), "vertex {k}.x: {row:?}");
                assert_eq!(
                    v.y.to_bits(),
                    f(r[i + 1]).to_bits(),
                    "vertex {k}.y: {row:?}"
                );
            }
            assert_eq!(uv_spec::member(a, p), b(r[14]), "member: {row:?}");
            assert_eq!(uv_spec::interior(a, p), b(r[15]), "interior: {row:?}");
            assert_eq!(uv_spec::contains(a, b_), b(r[30]), "contains: {row:?}");
            assert_eq!(
                uv_spec::interior_contains(a, b_),
                b(r[31]),
                "interior_contains: {row:?}"
            );
            assert_eq!(uv_spec::intersects(a, b_), b(r[32]), "intersects: {row:?}");
            assert_eq!(
                uv_spec::interior_intersects(a, b_),
                b(r[33]),
                "interior_intersects: {row:?}"
            );
            n += 1;
        }
        assert!(n >= min, "{name}: {n} rows");
        eprintln!("{name}: {n} rows");
    }
}

fn triple(c: &[&str]) -> Point {
    point_spec::point(f(c[0]), f(c[1]), f(c[2]))
}

/// Equal bit patterns, or both NaN (C++'s and Rust's NaN payloads need not agree).
fn same_bits(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
}

fn same_point_bits(p: Point, c: &[&str]) -> bool {
    same_bits(p.0.x, f(c[0])) && same_bits(p.0.y, f(c[1])) && same_bits(p.0.z, f(c[2]))
}

/// `SymbolicCrossProdSorted(a, b)` for `a < b` and `b`'s x or y or z non-zero
/// (`s2edge_crossings.cc:183–243`): the coefficient of the largest perturbation, before its
/// scaling. A test-only restatement, so that the antipodal rows check `upstream_scaled`.
fn symbolic_cross_prod_sorted(b_: Point) -> Point {
    if b_.0.y != 0.0 || b_.0.x != 0.0 {
        point_spec::point(-b_.0.y, b_.0.x, 0.0)
    } else {
        point_spec::point(b_.0.z, 0.0, 0.0)
    }
}

/// The point description against every `S2POINT_*` row: the order both ways (results 0, 1),
/// NaN rows included; `upstream_is_unit` (3); `upstream_normalize` (4–6) and, on the finite
/// rows, `upstream_ortho` (7–9), bit for bit; and, on
/// the exactly antipodal unit rows, where C++'s robust cross product falls back to its symbolic
/// perturbation, `upstream_scaled` against C++'s result (10–12): it is not finite exactly on
/// the rows noted `deviation:robust-cross-overflow` (spec 006 K6, U-012).
#[test]
fn point_description_matches_cpp() {
    let mut antipodal = 0;
    let mut overflow = 0;
    let mut orthos = 0;
    for (name, min) in [("S2POINT_OPS", 1500), ("S2POINT_SPECIAL", 150)] {
        let mut n = 0;
        for row in section(name) {
            let (a, b_) = (triple(&row[0..3]), triple(&row[3..6]));
            let r: Vec<&str> = row[7].split(';').collect();
            let notes: Vec<&str> = row[8].split('+').filter(|n| !n.is_empty()).collect();
            assert_eq!(r.len(), 18, "{row:?}");
            assert_eq!(point_spec::lex_less(a, b_), b(r[0]), "a < b: {row:?}");
            assert_eq!(point_spec::lex_less(b_, a), b(r[1]), "b < a: {row:?}");
            assert_eq!(point_spec::upstream_is_unit(a), b(r[3]), "is_unit: {row:?}");
            assert!(
                same_point_bits(point_spec::upstream_normalize(a), &r[4..7]),
                "normalize: {row:?}"
            );
            if r[7] != "-" {
                assert!(
                    same_point_bits(point_spec::upstream_ortho(a), &r[7..10]),
                    "ortho: {row:?}"
                );
                orthos += 1;
            }
            let is_overflow = notes.contains(&"deviation:robust-cross-overflow");
            let exactly_antipodal =
                point_spec::same(a, point_spec::point(-b_.0.x, -b_.0.y, -b_.0.z))
                    && !point_spec::zero(a);
            if r[10] != "-" && exactly_antipodal {
                // Upstream sorts the pair and negates the result when it swapped them.
                let (hi, sign) = if point_spec::lex_less(a, b_) {
                    (b_, 1.0)
                } else {
                    (a, -1.0)
                };
                let v = symbolic_cross_prod_sorted(hi);
                let scaled = if point_spec::normalizable(v) {
                    v
                } else {
                    point_spec::upstream_scaled(v)
                };
                let finite =
                    scaled.0.x.is_finite() && scaled.0.y.is_finite() && scaled.0.z.is_finite();
                assert_eq!(!finite, is_overflow, "overflow note: {row:?}");
                let expected =
                    point_spec::point(sign * scaled.0.x, sign * scaled.0.y, sign * scaled.0.z);
                assert!(
                    same_point_bits(expected, &r[10..13]),
                    "robust cross: {row:?}"
                );
                antipodal += 1;
            } else {
                assert!(!is_overflow, "an overflow off the antipodal rows: {row:?}");
            }
            overflow += usize::from(is_overflow);
            n += 1;
        }
        assert!(n >= min, "{name}: {n} rows");
        eprintln!("{name}: {n} rows");
    }
    eprintln!(
        "{antipodal} antipodal rows, {overflow} with the overflow (U-012), {orthos} with Ortho"
    );
    assert!(orthos > 1600, "{orthos} rows with Ortho");
    assert!(
        antipodal >= 40 && overflow == 4,
        "{antipodal} antipodal, {overflow} overflowing"
    );
}

/// The chord-angle description against `CHORD_ANGLE` (spec 007 T010). The rows marked
/// `deviation:negative-canonical` keep `l` in C++; the description states the library's −1
/// there (option B), so only the C++ side of that row is checked against `l` itself.
#[test]
fn chord_description_matches_cpp() {
    let (mut n, mut negative, mut infinity) = (0, 0, 0);
    for row in section("CHORD_ANGLE") {
        let ctx = || format!("{row:?}");
        let (kind, l, e) = (row[0], f(row[1]), f(row[3]));
        let r: Vec<&str> = row[4].split(';').collect();
        let notes: Vec<&str> = row[5].split('+').filter(|n| !n.is_empty()).collect();
        assert_eq!(r.len(), 24, "{}", ctx());
        let a = if kind == "I" {
            infinity += 1;
            chord_spec::INFINITY
        } else {
            chord_spec::upstream_from_length2(l)
        };
        let cpp = f(r[0]);
        if notes.contains(&"deviation:negative-canonical") {
            assert!(l < 0.0 && l != -1.0, "{}", ctx());
            assert_eq!(
                cpp.to_bits(),
                l.to_bits(),
                "C++ keeps the negative: {}",
                ctx()
            );
            assert_eq!(a.to_bits(), chord_spec::NEGATIVE.to_bits(), "{}", ctx());
            negative += 1;
            continue;
        }
        assert_eq!(a.to_bits(), cpp.to_bits(), "FromLength2: {}", ctx());
        assert!(chord_spec::canonical(a), "{}", ctx());
        assert_eq!(a == 0.0, b(r[1]), "is_zero: {}", ctx());
        assert_eq!(a < 0.0, b(r[2]), "is_negative: {}", ctx());
        assert_eq!(a == f64::INFINITY, b(r[3]), "is_infinity: {}", ctx());
        assert_eq!(chord_spec::special(a), b(r[4]), "is_special: {}", ctx());
        assert!(b(r[5]), "is_valid: {}", ctx());
        assert_eq!(chord_spec::next(a), f(r[6]), "Successor: {}", ctx());
        assert_eq!(chord_spec::prev(a), f(r[7]), "Predecessor: {}", ctx());
        assert_eq!(
            chord_spec::upstream_plus_error(a, e).to_bits(),
            f(r[8]).to_bits(),
            "PlusError: {}",
            ctx()
        );
        assert_eq!(
            chord_spec::upstream_max_point_error(a).to_bits(),
            f(r[9]).to_bits(),
            "point error: {}",
            ctx()
        );
        assert_eq!(
            chord_spec::upstream_max_angle_error(a).to_bits(),
            f(r[10]).to_bits(),
            "angle error: {}",
            ctx()
        );
        if r[11] != "-" {
            assert_eq!(
                chord_spec::upstream_sin2(a).to_bits(),
                f(r[11]).to_bits(),
                "{}",
                ctx()
            );
            assert_eq!(
                chord_spec::upstream_cos(a).to_bits(),
                f(r[12]).to_bits(),
                "{}",
                ctx()
            );
        }
        n += 1;
    }
    eprintln!("CHORD_ANGLE: {n} rows, {negative} negative-canonical, {infinity} Infinity()");
    assert!(
        n >= 950 && negative >= 18 && infinity == 5,
        "{n}, {negative}, {infinity}"
    );
}

/// The angle description against `ANGLE` (spec 007 T010): the conversions, and the E-forms
/// where C++'s value is in range (elsewhere C++ returns −2147483648, U-014).
#[test]
fn angle_description_matches_cpp() {
    let (mut n, mut saturate, mut ties) = (0, 0, 0);
    for row in section("ANGLE") {
        let ctx = || format!("{row:?}");
        let r_: f64 = f(row[0]);
        let n_: i32 = row[2].parse().unwrap_or_else(|e| panic!("{e}: {row:?}"));
        let r: Vec<&str> = row[3].split(';').collect();
        assert_eq!(r.len(), 21, "{}", ctx());
        assert_eq!(
            angle_spec::upstream_from_degrees(r_).to_bits(),
            f(r[0]).to_bits(),
            "Degrees: {}",
            ctx()
        );
        assert_eq!(
            angle_spec::upstream_degrees(r_).to_bits(),
            f(r[1]).to_bits(),
            "degrees: {}",
            ctx()
        );
        if row[4] == "deviation:eform-saturate" {
            saturate += 1;
        } else {
            for (k, i) in [(5, 2), (6, 3), (7, 4)] {
                let want: i32 = r[i].parse().unwrap_or_else(|e| panic!("{e}: {row:?}"));
                assert_eq!(angle_spec::upstream_e(k, r_), want, "e{k}: {}", ctx());
                let v = [1e5, 1e6, 1e7][(k - 5) as usize] * angle_spec::upstream_degrees(r_);
                ties += usize::from(v.is_finite() && v - v.floor() == 0.5);
            }
        }
        for (k, i) in [(5, 16), (6, 17), (7, 18)] {
            assert_eq!(
                angle_spec::upstream_from_e(k, n_).to_bits(),
                f(r[i]).to_bits(),
                "E{k}: {}",
                ctx()
            );
        }
        n += 1;
    }
    eprintln!("ANGLE: {n} rows, {saturate} out of the int32 range, {ties} exact ties");
    assert!(n >= 1000 && ties >= 9, "{n} rows, {ties} ties");
}

/// The cap a `CAP_*` row describes: centre `c`, radius `FromLength2(l)` (−1 for the empty cap).
fn cap_of(c: &[&str], l: &str) -> s2rst::s2::Cap {
    s2rst::s2::Cap::from_center_chord_angle(triple(c), s2rst::s1::ChordAngle::from_length2(f(l)))
}

/// The cap description against `CAP_OPS` and `CAP_SPECIAL` (spec 008 T008): membership,
/// interior membership, the classifications, and every restatement bit for bit (data-model §1).
/// The description reads only the row's centre and radius; `ChordAngle` sums in the `AddCap`
/// check are 007's verified ones.
#[test]
fn cap_description_matches_cpp() {
    use s2rst::s1::ChordAngle;
    let mut n = 0;
    for name in ["CAP_OPS", "CAP_SPECIAL"] {
        for row in section_of(CAP_CSV, name) {
            // cx,cy,cz,l,px,py,pz,bx,by,bz,bl,d,h,a,ang,results,note
            let a = cap_of(&row[0..3], row[3]);
            let p = triple(&row[4..7]);
            let b_cap = cap_of(&row[7..10], row[10]);
            let r: Vec<&str> = row[15].split(';').collect();
            let l = a.chord_radius().length2();
            let ctx = row.join(",");
            assert_eq!(cap_spec::valid(a), b(r[0]), "valid: {ctx}");
            assert_eq!(cap_spec::empty(a), b(r[1]), "empty: {ctx}");
            assert_eq!(cap_spec::full(a), b(r[2]), "full: {ctx}");
            assert!(
                same_bits(cap_spec::upstream_height(l), f(r[3])),
                "height: {ctx}"
            );
            assert!(
                same_bits(cap_spec::upstream_area(l), f(r[4])),
                "area: {ctx}"
            );
            assert!(
                same_point_bits(cap_spec::upstream_centroid(a.center(), l), &r[5..8]),
                "centroid: {ctx}"
            );
            assert_eq!(cap_spec::member(a, p), b(r[8]), "member: {ctx}");
            assert_eq!(
                cap_spec::interior_member(a, p),
                b(r[9]),
                "interior member: {ctx}"
            );
            if !cap_spec::empty(a) && !cap_spec::full(a) {
                assert!(
                    same_bits(cap_spec::upstream_complement_length2(l), f(r[16])),
                    "complement: {ctx}"
                );
            }
            if !cap_spec::empty(a) && !cap_spec::empty(b_cap) {
                let d = (ChordAngle::from_length2(cap_spec::chord2(a.center(), b_cap.center()))
                    + b_cap.chord_radius())
                .length2();
                let up = (d + cap_spec::upstream_add_cap_margin(d)).clamp(0.0, 4.0);
                assert!(same_bits(l.max(up), f(r[26])), "add_cap margin: {ctx}");
            }
            assert!(
                same_bits(cap_spec::upstream_from_height(f(row[12])), f(r[33])),
                "from_height: {ctx}"
            );
            assert!(
                same_bits(cap_spec::upstream_from_area(f(row[13])), f(r[34])),
                "from_area: {ctx}"
            );
            n += 1;
        }
    }
    eprintln!("CAP_OPS + CAP_SPECIAL: {n} rows");
}

/// The covering level against `CAP_BOUNDS` (spec 008 T008): `upstream_level` of the angle
/// radius (upstream's `GetRadius()`, `2 · asin(√l / 2)`) is the level of every id of
/// `GetCellUnionBound`, or the six faces when it is below 0.
#[test]
fn cap_level_matches_cpp() {
    let mut n = 0;
    for row in section_of(CAP_CSV, "CAP_BOUNDS") {
        // cx,cy,cz,l,results,note
        let l = f(row[3]);
        let r: Vec<&str> = row[4].split(';').collect();
        let ids: Vec<u64> = r[4]
            .split(' ')
            .filter(|s| !s.is_empty())
            .map(|h| u64::from_str_radix(h, 16).unwrap())
            .collect();
        // Upstream's `GetRadius()`: −1 radian for the empty cap, which upstream's covering, and
        // the library's since K8, does not treat apart (level 29, four cells).
        let radius = if l < 0.0 {
            -1.0
        } else {
            2.0 * (0.5 * l.sqrt()).asin()
        };
        let level = cap_spec::upstream_level(radius);
        if level < 0 {
            assert_eq!(ids.len(), 6, "six faces: {}", row.join(","));
        } else {
            for id in ids {
                let id_level = 30 - (id.trailing_zeros() / 2) as i32;
                assert_eq!(id_level, level, "level: {}", row.join(","));
            }
        }
        n += 1;
    }
    eprintln!("CAP_BOUNDS: {n} rows");
}
