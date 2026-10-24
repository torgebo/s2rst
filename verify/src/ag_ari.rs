// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The angle's arithmetic (P-AGARI, spec 007 data-model §4.2): the IEEE-754 operations on
//! radians, over the domains of `ag_crash` (no new NaN; no infinite divisor, a tool limit).
//! Addition and subtraction are per change (28.2 s, 31.1 s); the products (512.0 s) and the
//! quotients (over 12 minutes) are weekly (2026-10-17).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::value_domain::any_angle;

/// P-AGARI-01 (exact output), addition: `(a + b).radians()` is `a.radians() + b.radians()`,
/// bit for bit, for every `a`, `b` without NaN except opposite infinities.
#[kani::proof]
fn p_agari_01__add() {
    let (a, b) = (any_angle(), any_angle());
    let (x, y) = (a.radians(), b.radians());
    kani::assume(!x.is_nan() && !y.is_nan() && !(x.is_infinite() && x == -y));
    assert!((a + b).radians().to_bits() == (x + y).to_bits());
}

/// P-AGARI-01 (exact output), subtraction: `(a - b).radians()` is `a.radians() - b.radians()`,
/// bit for bit, for every `a`, `b` without NaN except equal infinities.
#[kani::proof]
fn p_agari_01__sub() {
    let (a, b) = (any_angle(), any_angle());
    let (x, y) = (a.radians(), b.radians());
    kani::assume(!x.is_nan() && !y.is_nan() && !(x.is_infinite() && x == y));
    assert!((a - b).radians().to_bits() == (x - y).to_bits());
}

/// P-AGARI-01 (exact output), the scalar products: `(a * m).radians()` is `a.radians() * m` and
/// `(m * a).radians()` is `m * a.radians()`, bit for bit, for every `a`, `m` without NaN except
/// zero times an infinity.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agari_01__mul() {
    let a = any_angle();
    let m: f64 = kani::any();
    let x = a.radians();
    kani::assume(!x.is_nan() && !m.is_nan());
    kani::assume(!((x == 0.0 && m.is_infinite()) || (x.is_infinite() && m == 0.0)));
    assert!((a * m).radians().to_bits() == (x * m).to_bits());
    assert!((m * a).radians().to_bits() == (m * x).to_bits());
}

/// P-AGARI-01 (exact output), the division by a scalar: `(a / m).radians()` is
/// `a.radians() / m`, bit for bit, for every `a` without NaN and finite `m` except `0 / 0` (an infinite divisor is a
/// tool limit: the checker reports `x / ±∞` as NaN).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agari_01__div() {
    let a = any_angle();
    let m: f64 = kani::any();
    let x = a.radians();
    kani::assume(!x.is_nan() && m.is_finite() && !(x == 0.0 && m == 0.0));
    assert!((a / m).radians().to_bits() == (x / m).to_bits());
}

/// P-AGARI-01 (exact output), the ratio of two angles: `a / b` is `a.radians() / b.radians()`,
/// bit for bit, for every `a` without NaN and finite `b` except `0 / 0`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_agari_01__ratio() {
    let (a, b) = (any_angle(), any_angle());
    let (x, y) = (a.radians(), b.radians());
    kani::assume(!x.is_nan() && y.is_finite() && !(x == 0.0 && y == 0.0));
    assert!((a / b).to_bits() == (x / y).to_bits());
}
