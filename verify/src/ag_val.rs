// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The angle's representation and laws (P-AGVAL, spec 007 data-model §4.2). An angle is any
//! `f64` of radians; the type has no validity rule.

use s2rst::s1::Angle;

use crate::laws;
use crate::value_domain::any_angle;

/// P-AGVAL-01 (correct result): `from_radians` and `radians` keep the bits; `ZERO`, `INFINITY`
/// and the default are upstream's; `abs`, negation and `is_infinite` are the IEEE-754
/// operations, and negation undoes itself.
///
/// For every `x` in `AnyF64`: `from_radians(x).radians()` is `x`, bit for bit; `abs` clears the
/// sign bit; `-a` flips it and `-(-a)` is `a`, bit for bit; `is_infinite` is `x.is_infinite()`.
/// `ZERO` and the default are `+0`, `INFINITY` is `+∞`.
#[kani::proof]
fn p_agval_01() {
    let x: f64 = kani::any();
    let a = Angle::from_radians(x);
    assert!(a.radians().to_bits() == x.to_bits());
    let sign = 1u64 << 63;
    assert!(a.abs().radians().to_bits() == x.to_bits() & !sign);
    assert!((-a).radians().to_bits() == x.to_bits() ^ sign);
    assert!((-(-a)).radians().to_bits() == x.to_bits());
    assert!(a.is_infinite() == x.is_infinite());
    assert!(Angle::ZERO.radians().to_bits() == 0);
    assert!(Angle::default().radians().to_bits() == 0);
    assert!(Angle::INFINITY.radians() == f64::INFINITY);
    kani::cover!(x.is_nan(), "NaN keeps its bits");
}

/// P-AGVAL-02 (law): without NaN, equality and the ordering are a total order (spec 006's order
/// laws, spec 004's equality laws).
///
/// For every `a`, `b`, `c` in `AnyAngle` without NaN: `laws::ord::all` with
/// `partial_cmp(..).unwrap()` (never `None`) and `laws::eq::all`.
#[kani::proof]
fn p_agval_02() {
    let (a, b, c) = (any_angle(), any_angle(), any_angle());
    kani::assume(!a.radians().is_nan() && !b.radians().is_nan() && !c.radians().is_nan());
    assert!(a.partial_cmp(&b).is_some());
    let cmp = |x: &Angle, y: &Angle| x.partial_cmp(y).unwrap();
    laws::ord::all(&cmp, &a, &b, &c);
    laws::eq::all(&a, &b, &c);
    kani::cover!(
        a == b && a.radians().to_bits() != b.radians().to_bits(),
        "±0 are equal"
    );
}
