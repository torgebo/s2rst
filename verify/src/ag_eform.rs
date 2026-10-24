// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The angle's E5, E6 and E7 forms (P-AGEF, spec 007 data-model §4.2), against
//! `angle_spec::upstream_e`: rounding ties to even (FR-022) and saturating outside the `i32`
//! range (FR-023). The round trips (P-AGEF-02) are proven by enumeration
//! (`tests/angle_enumeration.rs`). The three members are weekly: `e5` and `e7` took 570.8 s and
//! 620.3 s per change, and `e6` timed out at 12 minutes (2026-10-17).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

#[cfg(feature = "slow")]
use crate::angle_spec;
#[cfg(feature = "slow")]
use crate::value_domain::any_angle;

/// P-AGEF-01 (exact output), `e5`: upstream's rounding of degrees × 10⁵, ties to even,
/// saturated outside the `i32` range, 0 for NaN.
///
/// For every angle `a` in `AnyAngle`, NaN and the infinities included: `a.e5()` is
/// `angle_spec::upstream_e(5, a.radians())`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agef_01__e5() {
    let a = any_angle();
    assert!(a.e5() == angle_spec::upstream_e(5, a.radians()));
    kani::cover!(a.e5() == i32::MAX, "saturated");
}

/// P-AGEF-01 (exact output), `e6`: as `e5`, with 10⁶.
///
/// For every angle `a` in `AnyAngle`: `a.e6()` is `angle_spec::upstream_e(6, a.radians())`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agef_01__e6() {
    let a = any_angle();
    assert!(a.e6() == angle_spec::upstream_e(6, a.radians()));
    let v = 1e6 * angle_spec::upstream_degrees(a.radians());
    kani::cover!(v.is_finite() && v - v.floor() == 0.5, "an exact tie");
}

/// P-AGEF-01 (exact output), `e7`: as `e5`, with 10⁷.
///
/// For every angle `a` in `AnyAngle`: `a.e7()` is `angle_spec::upstream_e(7, a.radians())`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agef_01__e7() {
    let a = any_angle();
    assert!(a.e7() == angle_spec::upstream_e(7, a.radians()));
    kani::cover!(a.e7() == i32::MIN, "saturated below");
}
