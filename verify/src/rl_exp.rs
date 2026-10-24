// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle expansion and approximate equality (P-LEXP, spec 005 data-model §4.2; research
//! R8). Expansion's validity clause is `rl_val::p_lval_02__expanded`. Its containment clauses
//! are weekly (`#[cfg(feature = "slow")]`): the probe's whole-domain harness hit the 12-minute
//! limit (research R0), as the circular interval's did in spec 004. So is approximate
//! equality's symmetry, which timed out at 12 minutes per change (2026-10-13).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{LatLng, Rect};

use crate::rect_spec;
use crate::value_domain::{any_number, any_rect, any_valid_lat_lng};

/// P-LEXP-01 (correct result), construction from a centre and size: for every valid centre and
/// every size with non-negative, non-NaN components, the rectangle is valid and contains the
/// centre.
///
/// For every valid `c` and `s` with `s.lat ≥ 0`, `s.lng ≥ 0` (infinities included), with
/// `r = from_center_size(c, s)`: `rect_spec::valid(r)` and `rect_spec::member(r, c.lat, c.lng)`.
#[kani::proof]
fn p_lexp_01_center_size() {
    let c = any_valid_lat_lng();
    let (slat, slng) = (any_number(), any_number());
    kani::assume(slat >= 0.0 && slng >= 0.0);
    let r = Rect::from_center_size(c, LatLng::from_radians(slat, slng));
    assert!(rect_spec::valid(r));
    assert!(rect_spec::member(r, c.lat.radians(), c.lng.radians()));
    kani::cover!(r.is_full(), "full");
}

/// P-LEXP-01 (correct result), growth (weekly): expanding by finite non-negative margins
/// contains the original.
///
/// For every valid `r` and finite `m.lat ≥ 0`, `m.lng ≥ 0`: `rect_spec::contains(r.expanded(m), r)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_lexp_01__mpos() {
    let r = any_rect();
    let (mlat, mlng) = (any_number(), any_number());
    kani::assume(mlat.is_finite() && mlng.is_finite() && mlat >= 0.0 && mlng >= 0.0);
    assert!(rect_spec::contains(
        r.expanded(LatLng::from_radians(mlat, mlng)),
        r
    ));
}

/// P-LEXP-01 (correct result), shrinking (weekly): expanding by a finite non-positive latitude
/// margin and a finite negative longitude margin is contained in the original.
///
/// For every valid `r` and finite `m.lat ≤ 0`, `m.lng < 0`: `rect_spec::contains(r, r.expanded(m))`.
///
/// A zero longitude margin, of either sign, grows, as in spec 004's P-IEXP-01: upstream's
/// rounding allowance makes an arc within 2·ε of the full circle full. The first weekly run
/// (2026-10-14) found it with `m.lng = 0` (C++ and Go give the same).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_lexp_01__mneg() {
    let r = any_rect();
    let (mlat, mlng) = (any_number(), any_number());
    kani::assume(mlat.is_finite() && mlng.is_finite() && mlat <= 0.0 && mlng < 0.0);
    assert!(rect_spec::contains(
        r,
        r.expanded(LatLng::from_radians(mlat, mlng))
    ));
}

/// P-LEXP-02 (law), reflexivity: every valid rectangle is approximately equal to itself.
///
/// For every valid `r`: `r.approx_eq(r)`.
#[kani::proof]
fn p_lexp_02_reflexive() {
    let r = any_rect();
    assert!(r.approx_eq(r));
}

/// P-LEXP-02 (law), symmetry: approximate equality is symmetric.
///
/// For every valid `a`, `b`: `a.approx_eq(b) == b.approx_eq(a)`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-13). The longitudes' symmetry is
/// spec 004's P-IEXP-02 clause, whose members also time out.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_lexp_02_symmetric() {
    let (a, b) = (any_rect(), any_rect());
    let e = a.approx_eq(b);
    assert_eq!(e, b.approx_eq(a));
    kani::cover!(e && a != b, "approximately equal, not equal");
}
