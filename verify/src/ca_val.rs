// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's construction and its canonical-value invariant (P-CAVAL, spec 007
//! data-model §3, §4.1): every way to build a chord angle gives one of −1, a length in [0, 4]
//! or +∞ (`chord_spec::canonical`), the type property option B makes possible (FR-017).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::{Angle, ChordAngle};

use crate::chord_spec;
use crate::value_domain::{any_angle, any_chord, any_chord_length};

fn canonical(c: ChordAngle) -> bool {
    chord_spec::canonical(c.length2())
}

/// P-CAVAL-01 (correct result): the constants are upstream's, the default is `ZERO`, and
/// `length2` returns the stored length.
///
/// `NEGATIVE`, `ZERO`, `RIGHT`, `STRAIGHT`, `INFINITY` have the bits of −1, +0, 2, 4, +∞
/// (`chord_spec`); `ChordAngle::default()` is `ZERO`; for every `x` in [0, 4],
/// `from_length2(x).length2()` is `x`, bit for bit.
#[kani::proof]
fn p_caval_01() {
    let pairs = [
        (ChordAngle::NEGATIVE, chord_spec::NEGATIVE),
        (ChordAngle::ZERO, chord_spec::ZERO),
        (ChordAngle::RIGHT, chord_spec::RIGHT),
        (ChordAngle::STRAIGHT, chord_spec::STRAIGHT),
        (ChordAngle::INFINITY, chord_spec::INFINITY),
    ];
    for (c, l) in pairs {
        assert!(c.length2().to_bits() == l.to_bits());
    }
    assert!(ChordAngle::default().length2().to_bits() == 0.0f64.to_bits());
    let x: f64 = kani::any();
    kani::assume((0.0..=4.0).contains(&x));
    assert!(ChordAngle::from_length2(x).length2().to_bits() == x.to_bits());
    kani::cover!(x.is_sign_negative(), "−0 kept");
}

/// P-CAVAL-02 (validity), the constructors from numbers: `from_length2`, `from_radians` and
/// `from_degrees` give a canonical value for every `f64`, and the E-form constructors for every
/// `i32`.
#[kani::proof]
fn p_caval_02__ctor() {
    let x: f64 = kani::any();
    assert!(canonical(ChordAngle::from_length2(x)));
    assert!(canonical(ChordAngle::from_radians(x)));
    assert!(canonical(ChordAngle::from_degrees(x)));
    let n: i32 = kani::any();
    assert!(canonical(ChordAngle::from_e5(n)));
    assert!(canonical(ChordAngle::from_e6(n)));
    assert!(canonical(ChordAngle::from_e7(n)));
    kani::cover!(x < -1.0, "a negative length other than −1");
}

/// P-CAVAL-02 (validity), from an angle: `from_angle` and `From<Angle>` give a canonical value
/// for every angle, NaN and the infinities included.
#[kani::proof]
fn p_caval_02__angle() {
    let a = any_angle();
    assert!(canonical(ChordAngle::from_angle(a)));
    let c: ChordAngle = a.into();
    assert!(canonical(c));
}

/// P-CAVAL-02 (validity), the neighbours: `successor` and `predecessor` of every canonical
/// value are canonical.
#[kani::proof]
fn p_caval_02__nbr() {
    let a = any_chord();
    assert!(canonical(a.successor()));
    assert!(canonical(a.predecessor()));
}

/// P-CAVAL-02 (validity), the error bound: `plus_error` of every canonical value with every
/// error, NaN and the infinities included, is canonical.
#[kani::proof]
fn p_caval_02__err() {
    let a = any_chord();
    let e: f64 = kani::any();
    assert!(canonical(a.plus_error(e)));
}

/// P-CAVAL-02 (validity), the arithmetic: the sum and difference of two lengths in [0, 4] are
/// canonical (lengths in [0, 4]).
#[kani::proof]
fn p_caval_02__ari() {
    let (a, b) = (any_chord_length(), any_chord_length());
    assert!(canonical(a + b));
    assert!(canonical(a - b));
}

/// P-CAVAL-03 (correct result): `from_length2` is upstream's `FromLength2` with option B: a
/// negative input (−∞ included) gives `NEGATIVE`, a value in [0, 4] is kept, and above 4, +∞
/// and NaN give `STRAIGHT`.
///
/// For every `x` in `AnyF64`: `from_length2(x).length2()` is
/// `chord_spec::upstream_from_length2(x)`, bit for bit.
#[kani::proof]
fn p_caval_03() {
    let x: f64 = kani::any();
    let got = ChordAngle::from_length2(x).length2();
    assert!(got.to_bits() == chord_spec::upstream_from_length2(x).to_bits());
    kani::cover!(x == f64::NEG_INFINITY, "−∞");
    kani::cover!(x.is_nan(), "NaN");
}

/// P-CAVAL-04 (correct result): `from_angle` gives `NEGATIVE` for a negative angle, `INFINITY`
/// at +∞, and a length in [0, 4] otherwise; `from_radians`, `from_degrees` and the E-form
/// constructors follow it.
///
/// For every angle `a`: `from_angle(a)` is `NEGATIVE` if `a < 0`, `INFINITY` if `a == +∞`,
/// and in [0, 4] otherwise (NaN included); the same for `From<Angle>`, `from_radians(r)` and
/// `from_degrees(d)` (for every `f64`) and `from_eK(n)` (for every `i32`), read through the angle
/// each builds. The checker's `sin` is over-approximated within [−1, 1], so the exact length of
/// a NaN angle (4, through `sin(π / 2)`, as C++) is out of its reach: the C++ vectors check it
/// (`CHORD_ANGLE` result 23). Two `sin` results are never compared.
#[kani::proof]
fn p_caval_04() {
    let check = |c: ChordAngle, r: f64| {
        let l = c.length2();
        if r < 0.0 {
            assert!(l == -1.0);
        } else if r == f64::INFINITY {
            assert!(l == f64::INFINITY);
        } else {
            assert!((0.0..=4.0).contains(&l));
        }
    };
    let a = any_angle();
    check(ChordAngle::from_angle(a), a.radians());
    check(a.into(), a.radians());
    let x: f64 = kani::any();
    check(ChordAngle::from_radians(x), x);
    check(
        ChordAngle::from_degrees(x),
        Angle::from_degrees(x).radians(),
    );
    let n: i32 = kani::any();
    check(ChordAngle::from_e7(n), Angle::from_e7(n).radians());
    kani::cover!(a.radians() > 0.0 && a.radians() < 1.0, "a small angle");
}
