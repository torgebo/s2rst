// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The angle's conversions to and from degrees and from the E-forms (P-AGCNV, spec 007
//! data-model §4.2), against the restatements of upstream's products (`angle_spec`). The
//! clauses over one integer are proven by enumeration (`tests/angle_enumeration.rs`): P-AGCNV-01's
//! E-form and unsigned constructors and P-AGCNV-02's integer identities; Kani timed out on them
//! (two float products compared, 12 minutes). This module proves the clauses over an `f64`:
//! P-AGCNV-01's two products weekly (they time out at 12 minutes too, 2026-10-17), and
//! P-AGCNV-02's degree–radian identities per change.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::Angle;
use std::f64::consts::PI;

#[cfg(feature = "slow")]
use crate::angle_spec;

/// P-AGCNV-01 (exact output), `from_degrees`: upstream's product, bit for bit.
///
/// For every `x` in `AnyF64`: `from_degrees(x).radians()` is `upstream_from_degrees(x)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agcnv_01__from_degrees() {
    let x: f64 = kani::any();
    let got = Angle::from_degrees(x).radians();
    assert!(got.to_bits() == angle_spec::upstream_from_degrees(x).to_bits());
}

/// P-AGCNV-01 (exact output), `degrees`: upstream's product, bit for bit.
///
/// For every `x` in `AnyF64`: `from_radians(x).degrees()` is `upstream_degrees(x)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agcnv_01__degrees() {
    let x: f64 = kani::any();
    let got = Angle::from_radians(x).degrees();
    assert!(got.to_bits() == angle_spec::upstream_degrees(x).to_bits());
}

/// `2^j` for `−1022 ≤ j ≤ 1023`, from its exponent bits.
fn pow2(j: i32) -> f64 {
    f64::from_bits(u64::try_from(1023 + j).unwrap() << 52)
}

/// P-AGCNV-02 (correct result), the degree–radian identities upstream documents
/// (`s1angle.h:58–72`): `Degrees(45 k) == Radians(k π / 4)` for `k` in 0..=8, scaled by any
/// power of two that keeps both sides normal and finite, and `Degrees(45 k).degrees() == 45 k`.
///
/// For every `k` in 0..=8 and `j` in −1000..=960: `from_degrees(45 k · 2^j).radians() ==
/// k · π / 4 · 2^j`; and `from_degrees(45 k).degrees() == 45 k`.
#[kani::proof]
fn p_agcnv_02__degrees() {
    let k: u8 = kani::any();
    kani::assume(k <= 8);
    let j: i32 = kani::any();
    kani::assume((-1000..=960).contains(&j));
    let (kf, s) = (f64::from(k), pow2(j));
    assert!(Angle::from_degrees(45.0 * kf * s).radians() == kf * PI / 4.0 * s);
    assert!(Angle::from_degrees(45.0 * kf).degrees() == 45.0 * kf);
    kani::cover!(k == 8 && j == 960, "the top");
}
