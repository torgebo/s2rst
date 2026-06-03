// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for the public snap-radius clamping edge cases of
//! [`super::IntLatLngSnapFunction`] — the branches not reached by the in-file
//! tests. Written for this crate, not ported from upstream S2.

use super::{IntLatLngSnapFunction, SnapFunction};
use crate::s1::Angle;

#[test]
fn with_snap_radius_clamps_up_to_the_minimum() {
    // A requested radius below the per-exponent minimum is raised to the min.
    let f = IntLatLngSnapFunction::with_snap_radius(7, Angle::from_radians(0.0));
    let min = IntLatLngSnapFunction::min_snap_radius_for_exponent(7);
    assert_eq!(f.snap_radius().radians(), min.radians());
    assert_eq!(f.exponent(), 7);
}

#[test]
fn with_snap_radius_keeps_a_radius_above_the_minimum() {
    // A requested radius above the minimum is kept verbatim.
    let big = Angle::from_degrees(1.0);
    let f = IntLatLngSnapFunction::with_snap_radius(7, big);
    assert_eq!(f.snap_radius().radians(), big.radians());
}

#[test]
fn exponent_for_max_snap_radius_clamps_to_min_exponent() {
    // A very large radius needs only the coarsest exponent.
    let e = IntLatLngSnapFunction::exponent_for_max_snap_radius(Angle::from_degrees(170.0));
    assert_eq!(e, IntLatLngSnapFunction::MIN_EXPONENT);
}

#[test]
fn exponent_for_max_snap_radius_clamps_to_max_exponent() {
    // A vanishingly small radius would need an exponent beyond the maximum,
    // so the result is clamped.
    let e = IntLatLngSnapFunction::exponent_for_max_snap_radius(Angle::from_radians(1e-20));
    assert_eq!(e, IntLatLngSnapFunction::MAX_EXPONENT);
}
