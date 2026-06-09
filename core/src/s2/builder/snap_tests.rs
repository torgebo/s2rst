// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for the public snap-radius clamping edge cases of
//! [`super::IntLatLngSnapFunction`] and [`super::S2CellIdSnapFunction`] — the
//! branches not reached by the in-file tests (notably the `with_snap_radius`
//! constructors and the general `min_edge_vertex_separation` branch that only
//! runs for a custom radius above the per-level minimum). Written for this
//! crate, not ported from upstream S2.

use super::{IntLatLngSnapFunction, S2CellIdSnapFunction, SnapFunction};
use crate::s1::Angle;
use crate::s2::coords::Level;
use crate::s2::metric;

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

// ─── S2CellIdSnapFunction: with_snap_radius and the general
//     min_edge_vertex_separation branch ────────────────────────────────────

#[test]
fn s2cellid_with_snap_radius_clamps_up_to_the_minimum() {
    // A requested radius below the per-level minimum is raised to the min,
    // and the level is preserved.
    let level = 10u8;
    let min = S2CellIdSnapFunction::min_snap_radius_for_level(level);
    let f = S2CellIdSnapFunction::with_snap_radius(level, Angle::from_radians(0.0));
    assert_eq!(f.snap_radius().radians(), min.radians());
    assert_eq!(f.level(), Level::from(level));
}

#[test]
fn s2cellid_with_snap_radius_keeps_a_radius_above_the_minimum() {
    // A requested radius above the minimum is kept verbatim.
    let level = 10u8;
    let min = S2CellIdSnapFunction::min_snap_radius_for_level(level);
    let big = Angle::from_radians(5.0 * min.radians());
    let f = S2CellIdSnapFunction::with_snap_radius(level, big);
    assert_eq!(f.snap_radius().radians(), big.radians());
    assert_eq!(f.level(), Level::from(level));
}

#[test]
fn s2cellid_min_edge_vertex_separation_uses_general_branch_above_min_radius() {
    // Constructing with a radius comfortably above the per-level minimum skips
    // the min-radius special case (which returns 0.565 * min_diag) and runs the
    // general three-bound formula. This is the branch the in-file `new(level)`
    // tests never reach, because `new` always uses exactly the minimum radius.
    let level = 12u8;
    let min = S2CellIdSnapFunction::min_snap_radius_for_level(level);
    let snap_radius = Angle::from_radians(8.0 * min.radians());
    let f = S2CellIdSnapFunction::with_snap_radius(level, snap_radius);
    assert_eq!(f.snap_radius().radians(), snap_radius.radians());

    let min_diag = metric::MIN_DIAG.value(level);
    let sr = f.snap_radius().radians();
    let vs = f.min_vertex_separation().radians();
    // Mirror of the general branch: max(constant, proportional, asymptotic).
    let expected = (0.397 * min_diag).max(0.219 * sr).max(0.5 * (vs / sr) * vs);

    let evs = f.min_edge_vertex_separation().radians();
    assert!(
        (evs - expected).abs() < 1e-15,
        "general-branch evs={evs}, expected={expected}"
    );
    // It must differ from the min-radius special-case value, confirming the
    // general branch (not the `new`-radius shortcut) actually ran.
    assert!((evs - 0.565 * min_diag).abs() > 1e-12);

    // Separation invariants still hold.
    assert!(evs > 0.0);
    assert!(evs <= vs + 1e-15);
    assert!(vs <= 2.0 * sr + 1e-15);
}
