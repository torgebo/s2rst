// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 008 planning probe (research R0): Kani harnesses that measure whether `asin` and
//! `atan2` can be replaced by range contracts (FR-003, option A), what `Cap`'s laws cost, and
//! drafts of the fixes FR-017 (covering level), FR-018 (upstream's cell tests), FR-019
//! (complement) and FR-020 (exact longitude reduction), before the library changes.
//!
//! They run from a scratch crate that depends on `s2rst`, through a copy of `run-verify.sh`
//! (4 GiB and 12 minutes per harness), with `-Z stubbing`. Results go to research.md R0. Not
//! part of `verify/`.

use s2rst::r1;
use s2rst::r3::Vector;
use s2rst::s1::{self, Angle, ChordAngle};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::metric::MIN_WIDTH;
use s2rst::s2::{Cap, Cell, CellEdge, CellId, LatLng, Point, Rect};
use std::f64::consts::{FRAC_PI_2, PI};

// ---- the function contracts (FR-003, option A) -----------------------------------------------

/// `asin`'s contract: NaN outside [−1, 1] and for NaN; otherwise any value in [−π/2, π/2]
/// with the argument's sign, and ±0 for ±0.
fn asin_contract(x: f64) -> f64 {
    if x.is_nan() || x.abs() > 1.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return x;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= FRAC_PI_2);
    kani::assume(if x > 0.0 { r >= 0.0 } else { r <= 0.0 });
    r
}

/// `atan2`'s contract (C11 F.10.1.4): NaN if either argument is NaN; otherwise any value in
/// [−π, π], non-negative for a non-negative `y` (sign bit clear), non-positive for a
/// sign-bit-set `y`, and within [−π/2, π/2] when `x` is +0 or positive.
fn atan2_contract(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    let r: f64 = kani::any();
    kani::assume(r.abs() <= PI);
    kani::assume(if y.is_sign_positive() { r >= 0.0 } else { r <= 0.0 });
    if x.is_sign_positive() {
        kani::assume(r.abs() <= FRAC_PI_2);
    }
    r
}

// ---- domains ----------------------------------------------------------------------------------

/// A centre in the box [−1, 1]³ (spec 006 research R4): no products in the assumption.
fn any_box_point() -> Point {
    let (x, y, z): (f64, f64, f64) = (kani::any(), kani::any(), kani::any());
    kani::assume(x.abs() <= 1.0 && y.abs() <= 1.0 && z.abs() <= 1.0);
    Point(Vector { x, y, z })
}

/// A unit centre, as `Point::is_unit` defines it (one squared length in the assumption).
fn any_unit_point() -> Point {
    let p = any_box_point();
    kani::assume(p.is_unit());
    p
}

/// A valid cap's radius: `NEGATIVE` or a length in [0, 4].
fn any_radius() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume(x == -1.0 || (0.0..=4.0).contains(&x));
    ChordAngle::from_length2(x)
}

/// A non-empty radius: a length in [0, 4].
fn any_length() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume((0.0..=4.0).contains(&x));
    ChordAngle::from_length2(x)
}

fn any_box_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_radius())
}

fn any_nonempty_box_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_length())
}

fn any_valid_cap() -> Cap {
    Cap::from_center_chord_angle(any_unit_point(), any_radius())
}

// ---- drafts of the fixes ----------------------------------------------------------------------

/// FR-017: upstream's level, `kMinWidth.GetLevelForMinValue(r) − 1`.
fn level_draft(radius: f64) -> i32 {
    i32::from(MIN_WIDTH.max_level(radius).as_u8()) - 1
}

/// FR-019: upstream's complement radius, `FromLength2(4 − r²)`.
fn complement_draft(c: Cap) -> Cap {
    if c.is_full() {
        return Cap::empty();
    }
    if c.is_empty() {
        return Cap::full();
    }
    Cap::from_center_chord_angle(-c.center(), ChordAngle::from_length2(4.0 - c.chord_radius().length2()))
}

/// FR-020: the exact IEEE remainder by the `f64` nearest 2π for |x| ≤ 3π/2 (the range of
/// `lng ± asin(…)`), without `%`: one exact correction (Sterbenz, π ≤ |x| ≤ 4π).
fn reduce_draft(x: f64) -> f64 {
    let y = 2.0 * PI;
    if x > PI {
        x - y
    } else if x < -PI {
        x + y
    } else {
        x
    }
}

/// `rect_bound` with FR-020's reduction (otherwise the library's code).
fn rect_bound_draft(c: Cap) -> Rect {
    if c.is_empty() {
        return Rect::empty();
    }
    let cap_angle = c.angle_radius().radians();
    let center_lat = LatLng::latitude(c.center()).radians();
    let center_lng = LatLng::longitude(c.center()).radians();
    let mut all_longitudes = false;
    let mut lat = r1::Interval::new(center_lat - cap_angle, center_lat + cap_angle);
    let mut lng = s1::Interval::full();
    if lat.lo <= -FRAC_PI_2 {
        lat.lo = -FRAC_PI_2;
        all_longitudes = true;
    }
    if lat.hi >= FRAC_PI_2 {
        lat.hi = FRAC_PI_2;
        all_longitudes = true;
    }
    if !all_longitudes {
        let sin_a = c.chord_radius().sin();
        let sin_c = center_lat.cos();
        if sin_a <= sin_c {
            let angle_a = (sin_a / sin_c).asin();
            lng = s1::Interval::new(reduce_draft(center_lng - angle_a), reduce_draft(center_lng + angle_a));
        }
    }
    Rect::new(lat, lng)
}

/// Upstream's `S2Cap::Intersects(cell, vertices)` (s2cap.cc:225), over given vertices.
fn intersects_draft(c: Cap, cell: Cell, v: &[Point; 4]) -> bool {
    if c.chord_radius() >= ChordAngle::RIGHT || c.is_empty() {
        return false;
    }
    if cell.contains_point(c.center()) {
        return true;
    }
    let sin2 = c.chord_radius().sin2();
    let cv = c.center().vector();
    for k in 0..4 {
        let e = cell.edge_raw(CellEdge::ALL[k]).vector();
        let dot = cv.dot(e);
        if dot > 0.0 {
            continue;
        }
        if dot * dot > sin2 * e.norm2() {
            return false;
        }
        let dir = e.cross(cv);
        if dir.dot(v[k].vector()) < 0.0 && dir.dot(v[(k + 1) & 3].vector()) > 0.0 {
            return true;
        }
    }
    false
}

/// Upstream's `Contains(const S2Cell&)` over given vertices.
fn contains_cell_draft(c: Cap, cell: Cell, v: &[Point; 4]) -> bool {
    if !v.iter().all(|&p| c.contains_point(p)) {
        return false;
    }
    !intersects_draft(complement_draft(c), cell, v)
}

/// Upstream's `MayIntersect(const S2Cell&)` over given vertices.
fn may_intersect_draft(c: Cap, cell: Cell, v: &[Point; 4]) -> bool {
    if v.iter().any(|&p| c.contains_point(p)) {
        return true;
    }
    intersects_draft(c, cell, v)
}

fn any_cell() -> Cell {
    let bits: u64 = kani::any();
    let id = CellId::from(bits);
    kani::assume(id.is_valid());
    Cell::from_cell_id(id)
}

// ---- 1. the stubs ---------------------------------------------------------------------------

/// Does `#[kani::stub]` replace `f64::asin`? `to_angle` of every length is in [0, π].
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
fn r0_stub_asin_to_angle() {
    let r = any_length();
    let a = r.to_angle().radians();
    assert!((0.0..=PI).contains(&a));
}

/// Does `#[kani::stub]` replace `f64::atan2`? The latitude of a box point is in [−π/2, π/2].
#[kani::proof]
#[kani::stub(f64::atan2, atan2_contract)]
fn r0_stub_atan2_latitude() {
    let p = any_box_point();
    let lat = LatLng::latitude(p).radians();
    assert!(lat.abs() <= FRAC_PI_2);
}

/// Control: without the stub, the same harness meets the unsupported foreign call.
#[kani::proof]
fn r0_nostub_asin_to_angle() {
    let r = any_length();
    let a = r.to_angle().radians();
    assert!((0.0..=PI).contains(&a));
}

// ---- 2. point and cap relations ---------------------------------------------------------------

/// P-CAPGRO: after `add_point(p)` the cap contains `p` (upstream's own claim).
#[kani::proof]
fn r0_add_point_contains() {
    let c = any_box_cap();
    let p = any_box_point();
    assert!(c.add_point(p).contains_point(p));
}

/// `contains` is reflexive on non-empty caps.
#[kani::proof]
fn r0_contains_reflexive() {
    let c = any_nonempty_box_cap();
    assert!(c.contains(c));
}

/// `contains` a non-empty cap implies `intersects` it (two chord-angle sums through `sqrt`).
#[kani::proof]
fn r0_contains_implies_intersects() {
    let a = any_box_cap();
    let b = any_nonempty_box_cap();
    if a.contains(b) {
        assert!(a.intersects(b));
    }
}

/// `intersects` is symmetric.
#[kani::proof]
fn r0_intersects_symmetric() {
    let a = any_box_cap();
    let b = any_box_cap();
    assert!(a.intersects(b) == b.intersects(a));
}

/// After `add_cap(b)` the cap contains `b` (upstream's error margin).
#[kani::proof]
fn r0_add_cap_contains() {
    let a = any_box_cap();
    let b = any_box_cap();
    assert!(a.add_cap(b).contains(b));
}

/// `expanded` by a finite non-negative distance contains the original cap.
#[kani::proof]
fn r0_expanded_contains() {
    let c = any_box_cap();
    let d: f64 = kani::any();
    kani::assume(d.is_finite() && d >= 0.0);
    assert!(c.expanded(Angle::from_radians(d)).contains(c));
}

/// The equality laws (004's `laws::eq`), restated: reflexive, symmetric, transitive.
#[kani::proof]
fn r0_eq_laws() {
    let (a, b, c) = (any_box_cap(), any_box_cap(), any_box_cap());
    assert!(a == a);
    assert!((a == b) == (b == a));
    if a == b && b == c {
        assert!(a == c);
    }
}

/// The codec round trip under `Cap`'s own equality (the `CORE_TRAITS.md` law).
#[kani::proof]
fn r0_codec_round_trip_eq() {
    let c = any_valid_cap();
    let mut buf = Vec::with_capacity(32);
    c.encode(&mut buf).unwrap();
    let d = Cap::decode(&mut buf.as_slice()).unwrap();
    assert!(d == c);
}

// ---- 3. complement (K4) -----------------------------------------------------------------------

/// K4 on the library: `complement`'s radius against upstream's `4 − r²` (expected to fail).
#[kani::proof]
fn r0_complement_library_vs_upstream() {
    let c = any_nonempty_box_cap();
    kani::assume(!c.is_full());
    let lib = c.complement().chord_radius().length2();
    assert!(lib == 4.0 - c.chord_radius().length2());
}

/// FR-019's draft: the complement is valid when the cap is, and full and empty swap.
#[kani::proof]
fn r0_complement_draft_valid() {
    let c = any_valid_cap();
    let k = complement_draft(c);
    assert!(k.is_valid());
    assert!(c.is_empty() == k.is_full());
}

// ---- 4. the covering level (K1) --------------------------------------------------------------

/// FR-017's draft: the level keeps the cells at least twice as wide as the radius.
#[kani::proof]
fn r0_level_draft_margin() {
    let r: f64 = kani::any();
    kani::assume(r.is_finite() && r > 0.0);
    let level = level_draft(r);
    if level >= 0 {
        assert!(MIN_WIDTH.value(level as u8) >= 2.0 * r);
    }
}

/// K1 on the library's formula, `floor(log2(d / r)) − 1` (expected to fail, or to show that
/// `log2` is not modelled).
#[kani::proof]
fn r0_level_library_margin() {
    let r: f64 = kani::any();
    kani::assume(r.is_finite() && r > 0.0);
    let d = 2.0 * std::f64::consts::SQRT_2 / 3.0;
    let raw = (d / r).log2().floor() as i32;
    let level = raw.clamp(0, 30) - 1;
    if level >= 0 {
        assert!(MIN_WIDTH.value(level as u8) >= 2.0 * r);
    }
}

/// `cell_union_bound` under the `asin` contract: at most six ids (library code, K1 aside).
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
fn r0_cell_union_bound_len() {
    let c = any_valid_cap();
    assert!(c.cell_union_bound().len() <= 6);
}

// ---- 5. rect_bound (K5) under both contracts --------------------------------------------------

/// FR-020's draft: the reduction is exact: `x − r` is 0 or ±2π exactly, and the result is in
/// [−π, π], for every |x| ≤ 3π/2.
#[kani::proof]
fn r0_reduce_draft_exact() {
    let x: f64 = kani::any();
    kani::assume(x.abs() <= 1.5 * PI);
    let r = reduce_draft(x);
    assert!(r.abs() <= PI);
    let k = x - r;
    assert!(k == 0.0 || k == 2.0 * PI || k == -2.0 * PI);
    assert!(r + k == x);
}

/// `rect_bound` with FR-020's draft returns a valid rectangle for every valid cap.
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
fn r0_rect_bound_draft_valid() {
    let c = any_valid_cap();
    let r = rect_bound_draft(c);
    assert!(r.is_valid());
    assert!(r.is_empty() == c.is_empty());
}

/// The library's `rect_bound`: crash freedom and validity under both contracts.
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
fn r0_rect_bound_library_valid() {
    let c = any_valid_cap();
    assert!(c.rect_bound().is_valid());
}

// ---- 6. union and the unit centre ----------------------------------------------------------------

/// `from_center_angle` over a unit centre: its debug-build validity check never fires.
#[kani::proof]
fn r0_from_center_angle_crash() {
    let c = any_unit_point();
    let a: f64 = kani::any();
    kani::assume(!a.is_nan());
    let _ = Cap::from_center_angle(c, Angle::from_radians(a));
}

/// `union` under both contracts: crash freedom over valid caps (its validity check included).
#[kani::proof]
#[kani::stub(f64::asin, asin_contract)]
#[kani::stub(f64::atan2, atan2_contract)]
fn r0_union_crash() {
    let a = any_valid_cap();
    let b = any_valid_cap();
    let _ = a.union(b);
}

// ---- 7. the Region laws over upstream's cell tests (K2, K3) -------------------------------------

/// FR-010's law over given vertices: upstream's `Contains(cell)` implies `MayIntersect(cell)`,
/// for any four points as the vertices.
#[kani::proof]
fn r0_region_contains_implies_may_intersect() {
    let c = any_box_cap();
    let cell = any_cell();
    let v = [any_box_point(), any_box_point(), any_box_point(), any_box_point()];
    if contains_cell_draft(c, cell, &v) {
        assert!(may_intersect_draft(c, cell, &v));
    }
}

/// The new law: a cell the cap does not intersect has no vertex the cap contains.
#[kani::proof]
fn r0_region_no_intersect_no_vertex() {
    let c = any_box_cap();
    let cell = any_cell();
    let v = [any_box_point(), any_box_point(), any_box_point(), any_box_point()];
    if !may_intersect_draft(c, cell, &v) {
        for p in v {
            assert!(!c.contains_point(p));
        }
    }
}

/// Crash freedom of upstream's cell tests with the real vertices of every valid cell.
#[kani::proof]
fn r0_region_cell_tests_crash() {
    let c = any_valid_cap();
    let cell = any_cell();
    let v = [cell.vertex(0), cell.vertex(1), cell.vertex(2), cell.vertex(3)];
    let _ = contains_cell_draft(c, cell, &v);
    let _ = may_intersect_draft(c, cell, &v);
}
