// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 004 planning probe (research R0): Kani harnesses that measure the operations FR-002
//! and FR-003 leave to planning, and the FR-020 containment proof against the proposed
//! narrow fix. Round 1 is `r0_*`, and round 2 (`r1_*`) narrows round 1's timeouts and
//! failures. They run from a scratch crate that depends on `s2rst`, through a copy of
//! `verify/run-verify.sh`; the results are in research.md R0. Not part of `verify/`.

use s2rst::s1::Interval;
use s2rst::s2::{Cell, CellEdge, CellId};
use std::f64::consts::PI;

// ---- domains ---------------------------------------------------------------------------

fn any_valid_interval() -> Interval {
    let x = Interval { lo: kani::any(), hi: kani::any() };
    kani::assume(x.is_valid());
    x
}

/// The K4 corner: π to the next value above −π.
fn is_corner(x: Interval) -> bool {
    x.lo == PI && x.hi.to_bits() == (-PI).to_bits() - 1
}

/// Upstream's `S1Interval::GetLength` (s1interval.cc:41–47), restated.
fn upstream_length(x: Interval) -> f64 {
    let length = x.hi - x.lo;
    if length >= 0.0 {
        return length;
    }
    let length = length + 2.0 * PI;
    if length > 0.0 { length } else { -1.0 }
}

/// The proposed FR-020 fix: upstream's arithmetic, with the stable form only where it
/// gives no positive length for a non-empty interval.
fn fixed_length(x: Interval) -> f64 {
    let length = x.hi - x.lo;
    if length >= 0.0 {
        return length;
    }
    let length = length + 2.0 * PI;
    if length > 0.0 {
        length
    } else if x.is_empty() {
        -1.0
    } else {
        (x.hi + PI) - (x.lo - PI)
    }
}

/// A valid cell id at a fixed level, any face and position.
fn any_cell_id_at(level: u32) -> CellId {
    let face: u64 = kani::any();
    kani::assume(face < 6);
    let pos: u64 = kani::any();
    kani::assume(pos < (1u64 << (2 * level)));
    let shift = 2 * (30 - level);
    let raw = (face << 61) | (pos << (shift + 1)) | (1u64 << shift);
    let id = CellId::from(raw);
    kani::assume(id.is_valid());
    id
}

fn any_valid_cell_id() -> CellId {
    let id = CellId::from(kani::any::<u64>());
    kani::assume(id.is_valid());
    id
}

// ---- FR-020: uniqueness and containment ---------------------------------------------------

/// The corner is the only valid, non-empty interval for which upstream's length is negative.
#[kani::proof]
fn r0_len_unique() {
    let x = any_valid_interval();
    if !x.is_empty() && upstream_length(x) < 0.0 {
        assert!(is_corner(x));
    }
    kani::cover!(is_corner(x), "the corner is a valid interval");
}

/// The proposed fix changes the length of the corner only, to one ulp of π, and leaves
/// every other valid interval's length bit for bit as the library (upstream's arithmetic)
/// gives it.
#[kani::proof]
fn r0_len_contained() {
    let x = any_valid_interval();
    if is_corner(x) {
        assert!(fixed_length(x) == f64::from_bits(0x3CC0_0000_0000_0000)); // 2^-51
        assert!(x.length() == -1.0);
    } else {
        assert!(fixed_length(x).to_bits() == x.length().to_bits());
    }
    assert!(x.length().to_bits() == upstream_length(x).to_bits());
}

// ---- FR-003: the operations the pilot left out ---------------------------------------------

#[kani::proof]
fn r0_center() {
    let x = any_valid_interval();
    let c = x.center();
    if !x.is_empty() && !x.is_full() {
        assert!(x.contains(c));
    }
}

#[kani::proof]
fn r0_complement_center() {
    let x = any_valid_interval();
    let c = x.complement_center();
    if !x.is_empty() && !x.is_full() {
        assert!(x.complement().contains(c));
    }
}

/// Crash freedom over every margin, NaN and the infinities included.
#[kani::proof]
fn r0_expanded_any_margin() {
    let x = any_valid_interval();
    let m: f64 = kani::any();
    let _ = x.expanded(m);
}

#[kani::proof]
fn r0_expanded_contains() {
    let x = any_valid_interval();
    let m: f64 = kani::any();
    kani::assume(m.is_finite() && m >= 0.0);
    let y = x.expanded(m);
    assert!(y.is_valid());
    assert!(y.contains_interval(x));
}

#[kani::proof]
fn r0_dhd() {
    let x = any_valid_interval();
    let y = any_valid_interval();
    let d = x.directed_hausdorff_distance(y);
    assert!(d >= 0.0 && d <= PI);
}

#[kani::proof]
fn r0_approx_eq_any_error() {
    let x = any_valid_interval();
    let y = any_valid_interval();
    let e: f64 = kani::any();
    let _ = x.approx_eq_with(y, e);
}

#[kani::proof]
fn r0_approx_eq_reflexive() {
    let x = any_valid_interval();
    let e: f64 = kani::any();
    kani::assume(e.is_finite() && e >= 0.0);
    assert!(x.approx_eq_with(x, e));
}

// ---- FR-002: the stored-bound readers of Cell -----------------------------------------------

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_uv_edge() {
    let c = Cell::from_cell_id(any_valid_cell_id());
    let b = c.bound_uv();
    assert!(c.uv_coord_of_edge(CellEdge::Bottom).to_bits() == b.y.lo.to_bits());
    assert!(c.uv_coord_of_edge(CellEdge::Right).to_bits() == b.x.hi.to_bits());
    assert!(c.uv_coord_of_edge(CellEdge::Top).to_bits() == b.y.hi.to_bits());
    assert!(c.uv_coord_of_edge(CellEdge::Left).to_bits() == b.x.lo.to_bits());
}

fn uv_ordered_at(level: u32) {
    let c = Cell::from_cell_id(any_cell_id_at(level));
    let b = c.bound_uv();
    assert!(b.x.lo < b.x.hi && b.y.lo < b.y.hi);
    assert!(-1.0 <= b.x.lo && b.x.hi <= 1.0 && -1.0 <= b.y.lo && b.y.hi <= 1.0);
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_uv_ordered__l03() {
    uv_ordered_at(3);
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_uv_ordered__l10() {
    uv_ordered_at(10);
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_uv_ordered__l30() {
    uv_ordered_at(30);
}

/// Upstream's s2cell_test: a child's bounds, as `children()` computes them from the parent's
/// centre, are bit for bit those of the child built from its own id.
fn children_uv_at(level: u32) {
    let c = Cell::from_cell_id(any_cell_id_at(level));
    let kids = c.children().unwrap();
    let k: usize = kani::any();
    kani::assume(k < 4);
    let direct = Cell::from_cell_id(kids[k].id()).bound_uv();
    let got = kids[k].bound_uv();
    assert!(got.x.lo.to_bits() == direct.x.lo.to_bits());
    assert!(got.x.hi.to_bits() == direct.x.hi.to_bits());
    assert!(got.y.lo.to_bits() == direct.y.lo.to_bits());
    assert!(got.y.hi.to_bits() == direct.y.hi.to_bits());
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_children_uv__l03() {
    children_uv_at(3);
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_children_uv__l10() {
    children_uv_at(10);
}

#[kani::proof]
#[kani::unwind(9)]
fn r0_cell_children_uv__l29() {
    children_uv_at(29);
}

// ---- round 2: narrowing the round-1 timeouts --------------------------------------------------

/// Crash freedom alone, with the internal debug check.
#[kani::proof]
fn r1_dhd_crash() {
    let x = any_valid_interval();
    let y = any_valid_interval();
    let _ = x.directed_hausdorff_distance(y);
}

/// The cheap clauses of the property.
#[kani::proof]
fn r1_dhd_zero_and_pi() {
    let x = any_valid_interval();
    let y = any_valid_interval();
    let d = x.directed_hausdorff_distance(y);
    assert!((d == 0.0) == y.contains_interval(x));
    if y.is_empty() && !x.is_empty() {
        assert!(d == PI);
    }
}

fn dhd_range(y_inverted: bool) {
    let x = any_valid_interval();
    let y = any_valid_interval();
    kani::assume(y.is_inverted() == y_inverted);
    let d = x.directed_hausdorff_distance(y);
    assert!(d >= 0.0 && d <= PI);
}

#[kani::proof]
fn r1_dhd_range__yinv() {
    dhd_range(true);
}

#[kani::proof]
fn r1_dhd_range__ynoninv() {
    dhd_range(false);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_children_uv__l05() {
    children_uv_at(5);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_children_uv__l07() {
    children_uv_at(7);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_uv_ordered__l15() {
    uv_ordered_at(15);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_uv_ordered__l20() {
    uv_ordered_at(20);
}

/// Crash freedom for every margin except NaN (round 1: a NaN margin trips `new`'s check).
#[kani::proof]
fn r1_expanded_non_nan() {
    let x = any_valid_interval();
    let m: f64 = kani::any();
    kani::assume(!m.is_nan());
    let y = x.expanded(m);
    assert!(y.is_valid());
}

/// P-IEXP-01 off the corner (round 1's counterexample is the corner, the K4 knock-on).
#[kani::proof]
fn r1_expanded_contains_off_corner() {
    let x = any_valid_interval();
    kani::assume(!is_corner(x));
    let m: f64 = kani::any();
    kani::assume(m.is_finite() && m >= 0.0);
    let y = x.expanded(m);
    assert!(y.is_valid());
    assert!(y.contains_interval(x));
}

/// P-IMEA-01's real shape: the fixed length against upstream's, two pipelines.
fn len_2way(inverted: Option<bool>) {
    let x = any_valid_interval();
    if let Some(inv) = inverted {
        kani::assume(x.is_inverted() == inv);
    }
    if is_corner(x) {
        assert!(fixed_length(x) == f64::from_bits(0x3CC0_0000_0000_0000));
    } else {
        assert!(fixed_length(x).to_bits() == upstream_length(x).to_bits());
    }
}

#[kani::proof]
fn r1_len_2way() {
    len_2way(None);
}

#[kani::proof]
fn r1_len_2way__inv() {
    len_2way(Some(true));
}

#[kani::proof]
fn r1_len_2way__noninv() {
    len_2way(Some(false));
}

/// Upstream's quadratic `STtoUV` (s2coords.h:324–327), restated.
fn model_st_to_uv(s: f64) -> f64 {
    if s >= 0.5 { (1.0 / 3.0) * (4.0 * s * s - 1.0) } else { (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s)) }
}

/// P-CBND-01's shape: the stored u bounds against the restated projection of the (i) range.
fn bound_model_at(level: u32) {
    let id = any_cell_id_at(level);
    let c = Cell::from_cell_id(id);
    let (_, i, _, _) = id.to_face_ij_orientation();
    let size = 1i64 << (30 - level);
    let lo = (i64::from(i) & !(size - 1)) as f64 * (1.0 / f64::from(1u32 << 30));
    let hi = ((i64::from(i) & !(size - 1)) + size) as f64 * (1.0 / f64::from(1u32 << 30));
    assert!(c.bound_uv().x.lo.to_bits() == model_st_to_uv(lo).to_bits());
    assert!(c.bound_uv().x.hi.to_bits() == model_st_to_uv(hi).to_bits());
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_bound_model__l03() {
    bound_model_at(3);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_bound_model__l10() {
    bound_model_at(10);
}

#[kani::proof]
#[kani::unwind(9)]
fn r1_cell_bound_model__l20() {
    bound_model_at(20);
}
