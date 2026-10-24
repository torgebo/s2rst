// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 006 planning probe (research R0): Kani harnesses that measure the u-v rectangle against
//! the product of two of 005's real-interval descriptions, and the point's value-level surface,
//! its scaling (the K5 defect and a draft of the FR-020 fix), its orthogonal directions over the
//! box `[−1, 1]³`, and whether the checker models `atan2`, `sin` and `cos`.
//!
//! They run from a scratch crate that depends on `s2rst` and `s2rst-verify`, through a copy of
//! `run-verify.sh` (4 jobs, 4 GiB and 12 minutes per harness), after the 004/005 queue has
//! finished, one job at a time. Results go to research.md R0. Not part of `verify/`.
//!
//! Run on 2026-10-14: `r0_results.tsv` and `r0_counterexamples.txt` beside this file. The
//! harnesses are kept as they ran; three of them failed and reopened decisions (R0 findings
//! 1–3), so `r0_pt_from_coords_finite`, `r0_pt_ortho_odd` and
//! `r0_pt_point_cross_antisymmetric` do not state what the feature proves.

use s2rst::r1;
use s2rst::r2;
use s2rst::r3::Vector;
use s2rst::s1::Angle;
use s2rst::s2::{Point, ortho, rotate};
use s2rst_verify::real_spec;
use std::cmp::Ordering;

// ---- the u-v rectangle as a product of two real intervals --------------------------------------

fn any_real() -> r1::Interval {
    let x = r1::Interval { lo: kani::any(), hi: kani::any() };
    kani::assume(!x.lo.is_nan() && !x.hi.is_nan());
    x
}
fn any_finite_real() -> r1::Interval {
    let x = any_real();
    kani::assume(x.lo.is_finite() && x.hi.is_finite());
    x
}
fn uv_valid(r: r2::Rect) -> bool {
    real_spec::empty(r.x) == real_spec::empty(r.y)
}
fn any_uv() -> r2::Rect {
    let r = r2::Rect { x: any_real(), y: any_real() };
    kani::assume(uv_valid(r));
    r
}
fn any_finite_uv() -> r2::Rect {
    let r = r2::Rect { x: any_finite_real(), y: any_finite_real() };
    kani::assume(uv_valid(r));
    r
}
fn any_number() -> f64 {
    let p: f64 = kani::any();
    kani::assume(!p.is_nan());
    p
}
fn any_uv_point() -> r2::Point {
    r2::Point { x: any_number(), y: any_number() }
}
fn uv_member(r: r2::Rect, p: r2::Point) -> bool {
    real_spec::member(r.x, p.x) && real_spec::member(r.y, p.y)
}
fn uv_contains(a: r2::Rect, b: r2::Rect) -> bool {
    real_spec::empty(b.x) || (real_spec::contains(a.x, b.x) && real_spec::contains(a.y, b.y))
}

#[kani::proof]
fn r0_uv_valid_model() {
    let r = r2::Rect { x: any_real(), y: any_real() };
    assert!(r.is_valid() == uv_valid(r));
}

#[kani::proof]
fn r0_uv_contains_model() {
    let (a, b) = (any_uv(), any_uv());
    assert!(a.contains(b) == uv_contains(a, b));
}

#[kani::proof]
fn r0_uv_contains_point_model() {
    let (r, p) = (any_uv(), any_uv_point());
    assert!(r.contains_point(p) == uv_member(r, p));
}

#[kani::proof]
fn r0_uv_union_least() {
    let (a, b) = (any_uv(), any_uv());
    let u = a.union(b);
    assert!(uv_valid(u));
    assert!(uv_contains(u, a) && uv_contains(u, b));
}

#[kani::proof]
fn r0_uv_intersection_common_points() {
    let (a, b, p) = (any_uv(), any_uv(), any_uv_point());
    let i = a.intersection(b);
    assert!(uv_valid(i));
    assert!(uv_member(i, p) == (uv_member(a, p) && uv_member(b, p)));
}

#[kani::proof]
fn r0_uv_add_point() {
    let (r, p) = (any_uv(), any_uv_point());
    let s = r.add_point(p);
    assert!(uv_valid(s));
    assert!(uv_member(s, p) && uv_contains(s, r));
}

#[kani::proof]
fn r0_uv_expanded_crash_finite() {
    let r = any_finite_uv();
    let m = r2::Point { x: any_number(), y: any_number() };
    kani::assume(m.x.is_finite() && m.y.is_finite());
    assert!(uv_valid(r.expanded(m)));
}

#[kani::proof]
fn r0_uv_expanded_contains() {
    let r = any_finite_uv();
    let m = r2::Point { x: any_number(), y: any_number() };
    kani::assume(m.x.is_finite() && m.y.is_finite() && m.x >= 0.0 && m.y >= 0.0);
    assert!(uv_contains(r.expanded(m), r));
}

#[kani::proof]
fn r0_uv_from_center_size() {
    let c = r2::Point { x: any_number(), y: any_number() };
    let s = r2::Point { x: any_number(), y: any_number() };
    kani::assume(c.x.is_finite() && c.y.is_finite() && s.x.is_finite() && s.y.is_finite());
    kani::assume(s.x >= 0.0 && s.y >= 0.0);
    assert!(uv_valid(r2::Rect::from_center_size(c, s)));
}

#[kani::proof]
fn r0_uv_project_inside() {
    let (r, p) = (any_uv(), any_uv_point());
    kani::assume(!real_spec::empty(r.x));
    assert!(uv_member(r, r.project(p)));
}

#[kani::proof]
fn r0_uv_approx_eq_reflexive() {
    let r = any_finite_uv();
    assert!(r.approx_eq(r));
}

#[kani::proof]
fn r0_uv_vertex() {
    let r = any_uv();
    let k: i32 = kani::any();
    let v = r.vertex(k);
    let w = r.vertices()[k.rem_euclid(4) as usize];
    assert!(v.x.to_bits() == w.x.to_bits() && v.y.to_bits() == w.y.to_bits());
}

// ---- the point -----------------------------------------------------------------------------------

fn pt(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}
fn any_triple() -> Point {
    pt(kani::any(), kani::any(), kani::any())
}
fn any_plain() -> Point {
    let p = any_triple();
    kani::assume(!p.x().is_nan() && !p.y().is_nan() && !p.z().is_nan());
    p
}
fn any_finite() -> Point {
    let p = any_triple();
    kani::assume(p.x().is_finite() && p.y().is_finite() && p.z().is_finite());
    p
}
/// The box `[−1, 1]³`, which holds every unit point.
fn any_box() -> Point {
    let p = any_triple();
    kani::assume(p.x().abs() <= 1.0 && p.y().abs() <= 1.0 && p.z().abs() <= 1.0);
    p
}
fn bits(p: Point) -> [u64; 3] {
    [p.x().to_bits(), p.y().to_bits(), p.z().to_bits()]
}

/// Upstream's order: `std::lexicographical_compare` over the coordinates with `<`.
fn lex_less(a: Point, b: Point) -> bool {
    let (a, b) = ([a.x(), a.y(), a.z()], [b.x(), b.y(), b.z()]);
    for i in 0..3 {
        if a[i] < b[i] {
            return true;
        }
        if b[i] < a[i] {
            return false;
        }
    }
    false
}

#[kani::proof]
fn r0_pt_cmp_is_upstreams_order() {
    let (a, b) = (any_triple(), any_triple());
    let expected = if lex_less(a, b) {
        Ordering::Less
    } else if lex_less(b, a) {
        Ordering::Greater
    } else {
        Ordering::Equal
    };
    assert!(a.cmp_point(b) == expected);
}

#[kani::proof]
fn r0_pt_cmp_transitive() {
    let (a, b, c) = (any_plain(), any_plain(), any_plain());
    kani::assume(a.cmp_point(b) == Ordering::Less && b.cmp_point(c) == Ordering::Less);
    assert!(a.cmp_point(c) == Ordering::Less);
}

#[kani::proof]
fn r0_pt_ties_are_equality() {
    let (a, b) = (any_plain(), any_plain());
    assert!((a.cmp_point(b) == Ordering::Equal) == (a == b));
    assert!((a.partial_cmp(&b) == Some(Ordering::Equal)) == (a == b));
}

const TWO_M242: f64 = f64::from_bits((1023 - 242) << 52);

#[kani::proof]
fn r0_pt_is_normalizable_model() {
    let p = any_plain();
    let m = p.x().abs().max(p.y().abs()).max(p.z().abs());
    assert!(p.is_normalizable() == (m >= TWO_M242));
}

#[kani::proof]
fn r0_pt_is_unit_model() {
    let p = any_finite();
    let n2 = p.x() * p.x() + p.y() * p.y() + p.z() * p.z();
    assert!(p.is_unit() == ((n2 - 1.0).abs() <= 5.0 * f64::EPSILON));
}

/// Expected to fail (K5): the library's scaling for a largest component below 2⁻¹⁰²³.
#[kani::proof]
fn r0_pt_ensure_normalizable_library() {
    let p = any_finite();
    kani::assume(p.0 != Vector::default());
    assert!(p.ensure_normalizable().is_normalizable());
}

/// `ilogb` for a non-zero finite `x`, from IEEE-754's encoding.
fn exponent(x: f64) -> i32 {
    let b = x.abs().to_bits();
    let e = ((b >> 52) & 0x7FF) as i32;
    if e == 0 { -1011 - (b.leading_zeros() as i32) } else { e - 1023 }
}
fn pow2(k: i32) -> f64 {
    f64::from_bits(((1023 + k) as u64) << 52)
}

/// A draft of FR-020's fix: multiply by `2^e`, `e = −ilogb(p_max)`, in two exact steps.
fn scale_draft(p: Vector) -> Vector {
    let m = p.x.abs().max(p.y.abs()).max(p.z.abs());
    if m == 0.0 || m >= TWO_M242 {
        return p;
    }
    let e = -exponent(m);
    let (e1, e2) = (e / 2, e - e / 2);
    p * pow2(e1) * pow2(e2)
}

#[kani::proof]
fn r0_pt_scale_draft() {
    let p = any_finite();
    kani::assume(p.0 != Vector::default());
    let r = scale_draft(p.0);
    let rm = r.x.abs().max(r.y.abs()).max(r.z.abs());
    assert!(rm >= TWO_M242);
    let m = p.x().abs().max(p.y().abs()).max(p.z().abs());
    if m < TWO_M242 {
        assert!((1.0..2.0).contains(&rm));
        // Exact: scaling back down gives the input, bit for bit.
        let e = -exponent(m);
        let back = r * pow2(-(e - e / 2)) * pow2(-(e / 2));
        assert!(bits(Point(back)) == bits(p));
    }
}

#[kani::proof]
fn r0_pt_from_coords_finite() {
    let p = any_finite();
    let q = Point::from_coords(p.x(), p.y(), p.z());
    let _ = q.is_unit();
}

/// Expected to fail on the NaN check: `normalize` computes `∞ · 0` (FR-004).
#[kani::proof]
fn r0_pt_from_coords_infinite() {
    let p = any_plain();
    let _ = Point::from_coords(p.x(), p.y(), p.z());
}

#[kani::proof]
fn r0_pt_add_sub_neg() {
    let (a, b) = (any_finite(), any_finite());
    let s = a + b;
    let d = a - b;
    assert!(bits(d) == bits(a + (-b)));
    assert!(bits(-(-a)) == bits(a));
    assert!(s.x().to_bits() == (a.x() + b.x()).to_bits());
}

#[kani::proof]
fn r0_pt_ortho_odd() {
    let a = any_box();
    assert!(bits(ortho(-a)) == bits(-ortho(a)));
}

#[kani::proof]
fn r0_pt_ortho_nonzero() {
    let a = any_box();
    let m = a.x().abs().max(a.y().abs()).max(a.z().abs());
    kani::assume(m >= TWO_M242);
    assert!(ortho(a).0 != Vector::default());
}

#[kani::proof]
fn r0_pt_point_cross_antisymmetric() {
    let (p, q) = (any_box(), any_box());
    kani::assume(p != q && p != -q);
    assert!(bits(q.point_cross(p)) == bits(-p.point_cross(q)));
}

#[kani::proof]
fn r0_pt_chord_angle_range() {
    let (p, q) = (any_box(), any_box());
    let c = p.chord_angle(q).length2();
    assert!((0.0..=4.0).contains(&c));
    assert!(p.chord_angle(p).length2() == 0.0);
}

#[kani::proof]
fn r0_pt_distance_self() {
    let p = any_box();
    assert!(p.distance(p).radians() == 0.0);
}

/// Reaches `atan2`: does the checker model it?
#[kani::proof]
fn r0_pt_distance_symmetric() {
    let (p, q) = (any_box(), any_box());
    assert!(p.distance(q).radians().to_bits() == q.distance(p).radians().to_bits());
}

/// Reaches `sin` and `cos`: does the checker model them?
#[kani::proof]
fn r0_pt_rotate_crash() {
    let (p, axis) = (any_box(), any_box());
    let a: f64 = kani::any();
    kani::assume(a.is_finite());
    let _ = rotate(p, axis, Angle::from_radians(a));
}
