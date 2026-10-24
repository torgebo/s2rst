// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's error bounds (P-CAERR, spec 007 data-model §4.1), against the
//! restatements of upstream's formulas (`chord_spec::upstream_*`).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::chord_spec;
use crate::value_domain::{any_chord, any_chord_length};

/// P-CAERR-01 (correct result): `plus_error` leaves a special value unchanged and is upstream's
/// clamp otherwise, bit for bit; a NaN sum gives `STRAIGHT`.
///
/// For every canonical `c` and every `e` in `AnyF64`: `c.plus_error(e).length2()` is
/// `chord_spec::upstream_plus_error(c.length2(), e)`, bit for bit.
#[kani::proof]
fn p_caerr_01() {
    let c = any_chord();
    let e: f64 = kani::any();
    let got = c.plus_error(e).length2();
    assert!(got.to_bits() == chord_spec::upstream_plus_error(c.length2(), e).to_bits());
    kani::cover!(!c.is_special() && e.is_nan(), "a NaN error");
    kani::cover!(!c.is_special() && got == 0.0 && e < 0.0, "clamped at zero");
}

/// P-CAERR-02 (exact output), the point bound: `max_point_error` is upstream's
/// `GetS2PointConstructorMaxError`, bit for bit.
///
/// For every length `l` in [0, 4]: `max_point_error` is `chord_spec::upstream_max_point_error(l)`.
#[kani::proof]
fn p_caerr_02__point() {
    let c = any_chord_length();
    assert!(
        c.max_point_error().to_bits()
            == chord_spec::upstream_max_point_error(c.length2()).to_bits()
    );
}

/// P-CAERR-02 (exact output), the angle bound: `max_angle_error` is upstream's
/// `GetS1AngleConstructorMaxError`, `1.5 · ε · length²` at `v0.14.0-40`, bit for bit.
///
/// For every length `l` in [0, 4]: `max_angle_error` is `chord_spec::upstream_max_angle_error(l)`.
/// It found D-008 (the library computed `ε · length²`, older upstream's), fixed on 2026-10-18.
#[kani::proof]
fn p_caerr_02__angle() {
    let c = any_chord_length();
    assert!(
        c.max_angle_error().to_bits()
            == chord_spec::upstream_max_angle_error(c.length2()).to_bits()
    );
}
