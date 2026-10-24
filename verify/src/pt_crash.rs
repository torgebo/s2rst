// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s2::Point::<op>` for the 24 in-scope operations of the point and
//! `rotate` (spec 006 US1, FR-004; data-model §5).
//!
//! Each domain is maximal, as spec 005's: every triple, except exactly those on which an
//! operation produces a new NaN, which Kani reports as a failed check (research R0):
//! - every triple, NaN and the infinities included (`AnyTriple`), for the construction, the
//!   conversions, the accessors, the order, the comparisons, the classifications, negation and
//!   the scaling;
//! - for addition and subtraction, every pair without NaN except the coordinates where they
//!   meet `∞ − ∞`;
//! - for `from_coords` and `normalize`, the triples whose squared length is finite
//!   (`NormableTriple`): CBMC also reports `1 / ∞` as NaN, though IEEE-754 gives zero, so the
//!   other finite triples are a tool limit, which `point_tests.rs` and the C++ vectors test
//!   (research R0, finding 1). Over all of `NormableTriple` the checker times out at 12
//!   minutes, so the per-change member takes the triples with every coordinate at most `2⁵¹¹`
//!   (`BoundedTriple`, 2 s), and the weekly one the rest, a coordinate in `(2⁵¹¹, 2⁵¹²)`;
//! - for the geometric operations, the box `[−1, 1]³` (`BoxPoint`), which holds every unit point,
//!   and a finite angle for `rotate`.
//!
//! `distance`, `stable_angle`, `approx_eq`, `approx_eq_angle` and `approx_eq_with` reach
//! `f64::atan2`, which Kani does not support (research R0): they have no harness, and the
//! inventory records them as tool limits. `sin` and `cos` are modelled, so `rotate` has one.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::r3::Vector;
#[cfg(feature = "slow")]
use s2rst::s2::rotate;
use s2rst::s2::{Point, ortho};
use std::cmp::Ordering;

use crate::point_spec;
use crate::value_domain::{any_bounded_triple, any_box_point, any_plain_triple, any_triple};
#[cfg(feature = "slow")]
use crate::value_domain::{any_finite_angle, any_normable_band_triple};

/// Whether `a + b` is `∞ − ∞` (NaN): opposite infinities.
fn opposite_infinities(a: f64, b: f64) -> bool {
    a.is_infinite() && a == -b
}

/// Whether `a − b` is `∞ − ∞` (NaN): the same infinity.
fn same_infinities(a: f64, b: f64) -> bool {
    a.is_infinite() && a == b
}

/// C-s2::Point::new, origin, vector, x, y, z, From<Vector>, From<Point> for Vector and Default:
/// complete without panic, arithmetic overflow or undefined behaviour for every triple.
#[kani::proof]
fn c_construction_and_accessors() {
    let p = any_triple();
    let v: Vector = p.into();
    let q = Point::new(v);
    let r: Point = Point::from(p.vector());
    let (x, y, z) = (q.x(), r.y(), p.z());
    let (o, d) = (Point::origin(), Point::default());
    kani::cover!(x.is_nan(), "a NaN coordinate");
    kani::cover!(y.is_infinite(), "an infinite coordinate");
    kani::cover!(
        z == 0.0 && o.z() > 0.9 && d.x() == 0.0,
        "the origin and the default"
    );
}

/// C-s2::Point::cmp_point, PartialOrd and PartialEq: complete without panic, arithmetic
/// overflow or undefined behaviour for every pair of triples, NaN included.
#[kani::proof]
fn c_order() {
    let (a, b) = (any_triple(), any_triple());
    let (c, pc, eq) = (a.cmp_point(b), a.partial_cmp(&b), a == b);
    kani::cover!(c == Ordering::Less, "less");
    kani::cover!(
        c == Ordering::Equal && !eq,
        "equal in the order, unequal by == (NaN, K8)"
    );
    kani::cover!(pc == Some(Ordering::Greater), "greater");
}

/// C-s2::Point::is_normalizable, is_unit and Neg: complete without panic, arithmetic overflow
/// or undefined behaviour for every triple: squaring never meets `∞ · 0`, and a sum of squares
/// never meets `∞ − ∞`.
#[kani::proof]
fn c_classifications() {
    let p = any_triple();
    let (n, u, m) = (p.is_normalizable(), p.is_unit(), -p);
    kani::cover!(u, "unit length");
    kani::cover!(n && !u, "normalizable, not unit");
    kani::cover!(!n && m.x() == -p.x(), "not normalizable");
}

/// C-s2::Point::ensure_normalizable: completes without panic, arithmetic overflow or undefined
/// behaviour for every triple, and the scaling's debug check (both factors normal powers of
/// two) never fires (FR-020).
#[kani::proof]
fn c_ensure_normalizable() {
    let p = any_triple();
    let r = p.ensure_normalizable();
    kani::cover!(
        !point_spec::normalizable(p)
            && !point_spec::zero(p)
            && !p.x().is_nan()
            && !p.y().is_nan()
            && !p.z().is_nan()
            && r.x().abs() >= 1.0,
        "a scaled vector"
    );
    kani::cover!(point_spec::zero(p), "the zero vector");
    kani::cover!(p.x().is_nan(), "a NaN coordinate");
}

/// C-s2::Point::Add and Sub: complete without panic, arithmetic overflow or undefined
/// behaviour for every pair of triples without NaN, except the coordinates where addition meets
/// opposite infinities and subtraction the same one (`∞ − ∞`).
#[kani::proof]
fn c_add_sub() {
    let (a, b) = (any_plain_triple(), any_plain_triple());
    let pairs = [(a.x(), b.x()), (a.y(), b.y()), (a.z(), b.z())];
    let add_ok = pairs.iter().all(|&(u, v)| !opposite_infinities(u, v));
    let sub_ok = pairs.iter().all(|&(u, v)| !same_infinities(u, v));
    if add_ok {
        let s = a + b;
        kani::cover!(
            s.x().is_infinite() && a.x().is_finite(),
            "the sum overflows"
        );
    }
    if sub_ok {
        let d = a - b;
        kani::cover!(d.y().is_infinite(), "an infinite difference");
    }
    kani::cover!(add_ok && !sub_ok, "only the sum is in the domain");
}

/// `from_coords` and `normalize` on one triple, with the covers of both members.
fn from_coords_normalize(p: Point) {
    let (f, n) = (Point::from_coords(p.x(), p.y(), p.z()), p.normalize());
    kani::cover!(
        point_spec::zero(p) && f == Point::origin(),
        "zero gives origin"
    );
    kani::cover!(
        !point_spec::zero(p) && point_spec::zero(n),
        "a squared length that underflows: the zero vector"
    );
    kani::cover!(n.x() == 1.0, "an axis");
}

/// C-s2::Point::from_coords and normalize: complete without panic, arithmetic overflow or
/// undefined behaviour for every triple whose coordinates are at most `2⁵¹¹` in magnitude
/// (`BoundedTriple`), so whose squared length is finite. The rest of `NormableTriple` is the
/// weekly member `__band`. The finite triples whose squared length overflows are a tool limit:
/// the checker reports `1 / ∞` as NaN; `point_tests.rs` and the C++ vectors check their zero
/// result (FR-022, K7).
#[kani::proof]
fn c_from_coords_normalize__bounded() {
    from_coords_normalize(any_bounded_triple());
}

/// C-s2::Point::from_coords and normalize (weekly): the rest of `NormableTriple`, a finite
/// squared length with a coordinate in `(2⁵¹¹, 2⁵¹²)`. It timed out at 12 minutes per change
/// (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn c_from_coords_normalize__band() {
    let p = any_normable_band_triple();
    let (f, n) = (Point::from_coords(p.x(), p.y(), p.z()), p.normalize());
    kani::cover!(f.x() == n.x() && n.x() > 0.5, "a large coordinate");
}

/// C-s2::ortho and Point::reference_dir (weekly): complete without panic, arithmetic overflow
/// or undefined behaviour for every point in the box `[−1, 1]³`. With `point_cross` it took
/// 568 s per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn c_ortho() {
    let p = any_box_point();
    let (o, r) = (ortho(p), p.reference_dir());
    kani::cover!(
        point_spec::zero(o) && !point_spec::zero(p),
        "ortho of a tiny point is zero"
    );
    kani::cover!(
        r.x() != 0.0 && r.y() != 0.0 && r.z() != 0.0,
        "no zero coordinate"
    );
}

/// C-s2::Point::point_cross: completes without panic, arithmetic overflow or undefined
/// behaviour for every pair of points in the box `[−1, 1]³`.
#[kani::proof]
fn c_point_cross() {
    let (p, q) = (any_box_point(), any_box_point());
    let x = p.point_cross(q);
    kani::cover!(
        point_spec::same(x, ortho(p)) && !point_spec::same(p, q),
        "point_cross takes its ortho branch for distinct points (K11)"
    );
    kani::cover!(!point_spec::zero(x), "a non-zero result");
}

/// C-s2::Point::chord_angle: completes without panic, arithmetic overflow or undefined
/// behaviour for every pair of points in the box `[−1, 1]³`.
#[kani::proof]
fn c_chord_angle() {
    let (p, q) = (any_box_point(), any_box_point());
    let c = p.chord_angle(q).length2();
    kani::cover!(c == 4.0, "clamped to a straight angle");
    kani::cover!(c > 0.0 && c < 4.0, "between");
}

/// C-s2::rotate (weekly): completes without panic, arithmetic overflow or undefined behaviour
/// for every point and axis in the box `[−1, 1]³` and every finite angle. `sin` and `cos` are
/// CBMC's models, over-approximated in [−1, 1] (research R0). It took 482 s per change
/// (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn c_rotate() {
    let (p, axis) = (any_box_point(), any_box_point());
    let r = rotate(p, axis, any_finite_angle());
    kani::cover!(point_spec::zero(r), "a zero result");
    kani::cover!(r.x() > 0.5, "a non-zero result");
}
