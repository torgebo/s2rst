// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The point's classifications (P-PTCLS, spec 006 data-model §4.2): normalizable and unit
//! length. The unit-length members are weekly (`#[cfg(feature = "slow")]`): R0's model of
//! `is_unit` took 644.5 s (research R12).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::point_spec;
use crate::value_domain::any_plain_triple;
#[cfg(feature = "slow")]
use crate::value_domain::{
    any_bounded_triple, any_finite_triple, any_normable_band_triple, any_normalizable_box_point,
};
#[cfg(feature = "slow")]
use s2rst::s2::{Point, ortho};

/// P-PTCLS-01 (correct result): a point is normalizable exactly when its largest absolute
/// coordinate is at least `2⁻²⁴²`.
///
/// For every `p` in `PlainTriple`: `p.is_normalizable() == point_spec::normalizable(p)`.
#[kani::proof]
fn p_ptcls_01() {
    let p = any_plain_triple();
    let n = p.is_normalizable();
    assert_eq!(n, point_spec::normalizable(p));
    kani::cover!(
        n && point_spec::max_abs(p) == point_spec::TWO_M242,
        "at the threshold"
    );
    kani::cover!(!n && !point_spec::zero(p), "a tiny non-zero point");
}

/// P-PTCLS-02 (exact output): `is_unit` is upstream's test, bit for bit (weekly).
///
/// For every `p` in `FiniteTriple`: `p.is_unit() == point_spec::upstream_is_unit(p)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptcls_02() {
    let p = any_finite_triple();
    let u = p.is_unit();
    assert_eq!(u, point_spec::upstream_is_unit(p));
    kani::cover!(
        u && p.x() != 1.0 && p.y() != 1.0 && p.z() != 1.0,
        "unit, off the axes"
    );
}

/// P-PTCLS-03 for one triple: where the squared length is a normal number, `normalize` and
/// `from_coords` give a unit point.
#[cfg(feature = "slow")]
fn unit_where_normal(p: Point) {
    kani::assume(point_spec::upstream_norm2(p) >= f64::MIN_POSITIVE);
    assert!(point_spec::upstream_is_unit(p.normalize()));
    assert!(point_spec::upstream_is_unit(Point::from_coords(
        p.x(),
        p.y(),
        p.z()
    )));
}

/// P-PTCLS-03 (correct result), normalising (weekly): `from_coords` and `normalize` give a unit
/// point where the squared length is a normal number (K7). This member: every coordinate at
/// most `2⁵¹¹`.
///
/// For every `p` in `BoundedTriple` whose squared length `upstream_norm2(p)` is at least
/// `f64::MIN_POSITIVE`: `upstream_is_unit(p.normalize())` and
/// `upstream_is_unit(Point::from_coords(p.x(), p.y(), p.z()))`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptcls_03__normalize() {
    unit_where_normal(any_bounded_triple());
}

/// P-PTCLS-03 (correct result), normalising (weekly), as `__normalize`, for the rest of
/// `NormableTriple`: a coordinate in `(2⁵¹¹, 2⁵¹²)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptcls_03__normalize_band() {
    unit_where_normal(any_normable_band_triple());
}

/// P-PTCLS-03 (correct result), the orthogonal directions (weekly): `ortho` and `reference_dir`
/// give a unit point for every normalizable point in the box.
///
/// For every `a` in `NormalizableBoxPoint`: `upstream_is_unit(ortho(a))` and
/// `upstream_is_unit(a.reference_dir())`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_ptcls_03__ortho() {
    let a = any_normalizable_box_point();
    assert!(point_spec::upstream_is_unit(ortho(a)));
    assert!(point_spec::upstream_is_unit(a.reference_dir()));
}
