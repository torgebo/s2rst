// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s1::ChordAngle::<op>` for the 28 entries of the chord angle with
//! crash proofs (spec 007 US1, FR-004; data-model §5).
//!
//! The domains:
//! - every `f64` (`AnyF64`), every `i32` and every angle (`AnyAngle`) for the constructors;
//! - every canonical value (`Chord`) for the accessors, the classifications, the order, the
//!   neighbours and `plus_error` (with every error);
//! - the non-special values (`ChordLength`, [0, 4]) for the error bounds, the trigonometry
//!   and the arithmetic: their documented precondition (FR-019), under which their debug checks
//!   are proven never to fire.
//!
//! `to_angle`, `radians`, `degrees`, `e5`, `e6`, `e7` and the conversion to `Angle` reach
//! `f64::asin`, a C library call Kani replaces with a failing check (research R0): they have
//! no harness, and the inventory records them as tool limits.

use s2rst::s1::ChordAngle;
use std::cmp::Ordering;

use crate::value_domain::{any_angle, any_chord, any_chord_length};

/// C-s1::ChordAngle::from_length2, from_radians and from_degrees: complete without panic,
/// arithmetic overflow or undefined behaviour for every `f64`, NaN and the infinities included.
#[kani::proof]
fn c_from_numbers() {
    let x: f64 = kani::any();
    let a = ChordAngle::from_length2(x);
    let b = ChordAngle::from_radians(x);
    let c = ChordAngle::from_degrees(x);
    kani::cover!(x < 0.0 && a == ChordAngle::NEGATIVE, "a negative length");
    kani::cover!(x > 4.0 && a == ChordAngle::STRAIGHT, "a length above 4");
    kani::cover!(x.is_nan() && a == ChordAngle::STRAIGHT, "NaN");
    kani::cover!(
        (0.0..=4.0).contains(&x) && a.length2() == x,
        "a length kept"
    );
    kani::cover!(
        b == ChordAngle::INFINITY && c == ChordAngle::INFINITY,
        "+∞ radians, degrees"
    );
}

/// C-s1::ChordAngle::from_e5, from_e6 and from_e7: complete without panic, arithmetic
/// overflow or undefined behaviour for every `i32`.
#[kani::proof]
fn c_from_e_forms() {
    let n: i32 = kani::any();
    let (a, b, c) = (
        ChordAngle::from_e5(n),
        ChordAngle::from_e6(n),
        ChordAngle::from_e7(n),
    );
    kani::cover!(n < 0 && a == ChordAngle::NEGATIVE, "a negative angle");
    kani::cover!(
        n > 0 && b.length2() > 0.0 && c.length2() > 0.0,
        "a positive angle"
    );
}

/// C-s1::ChordAngle::from_angle and From<Angle>: complete without panic, arithmetic overflow
/// or undefined behaviour for every angle, NaN and the infinities included.
#[kani::proof]
fn c_from_angle() {
    let a = any_angle();
    let c = ChordAngle::from_angle(a);
    let d: ChordAngle = a.into();
    kani::cover!(a.radians() < 0.0 && c == ChordAngle::NEGATIVE, "negative");
    kani::cover!(
        a.radians() == f64::INFINITY && d == ChordAngle::INFINITY,
        "+∞"
    );
    kani::cover!(
        a.radians().is_nan() && c == ChordAngle::STRAIGHT,
        "NaN gives 4"
    );
    kani::cover!(a.radians() > 0.0 && a.radians() < 3.0, "a finite angle");
}

/// C-s1::ChordAngle::length2, is_zero, is_negative, is_infinity, is_special, is_valid,
/// PartialEq, PartialOrd and Default: complete without panic, arithmetic overflow or undefined
/// behaviour for every canonical value.
#[kani::proof]
fn c_accessors_classes_and_order() {
    let (a, b) = (any_chord(), any_chord());
    let l = a.length2();
    let classes = (
        a.is_zero(),
        a.is_negative(),
        a.is_infinity(),
        a.is_special(),
        a.is_valid(),
    );
    let (eq, ord) = (a == b, a.partial_cmp(&b));
    let d = ChordAngle::default();
    kani::cover!(classes.0 && d == a, "zero");
    kani::cover!(classes.1 && l == -1.0, "NEGATIVE");
    kani::cover!(classes.2, "INFINITY");
    kani::cover!(!classes.3 && classes.4, "a length");
    kani::cover!(eq, "equal");
    kani::cover!(ord == Some(Ordering::Less), "less");
}

/// C-s1::ChordAngle::successor and predecessor: complete without panic, arithmetic overflow
/// or undefined behaviour for every canonical value.
#[kani::proof]
fn c_neighbours() {
    let a = any_chord();
    let (s, p) = (a.successor(), a.predecessor());
    kani::cover!(
        a == ChordAngle::STRAIGHT && s == ChordAngle::INFINITY,
        "STRAIGHT's successor"
    );
    kani::cover!(
        a == ChordAngle::NEGATIVE && s == ChordAngle::ZERO,
        "NEGATIVE's successor"
    );
    kani::cover!(
        a == ChordAngle::ZERO && p == ChordAngle::NEGATIVE,
        "ZERO's predecessor"
    );
    kani::cover!(
        a == ChordAngle::INFINITY && p == ChordAngle::STRAIGHT,
        "INFINITY's predecessor"
    );
    kani::cover!(a.length2() > 0.0 && a.length2() < 4.0, "a length inside");
}

/// C-s1::ChordAngle::plus_error: completes without panic, arithmetic overflow or undefined
/// behaviour for every canonical value and every error, NaN and the infinities included.
#[kani::proof]
fn c_plus_error() {
    let a = any_chord();
    let e: f64 = kani::any();
    let r = a.plus_error(e);
    kani::cover!(a.is_special() && r == a, "a special value unchanged");
    kani::cover!(
        !a.is_special() && e.is_nan() && r == ChordAngle::STRAIGHT,
        "NaN sum"
    );
    kani::cover!(
        !a.is_special() && r == ChordAngle::ZERO && e < 0.0,
        "clamped at 0"
    );
    kani::cover!(
        !a.is_special() && r.length2() > 0.0 && r.length2() < 4.0,
        "inside"
    );
}

/// C-s1::ChordAngle::max_point_error and max_angle_error: complete without panic, arithmetic
/// overflow or undefined behaviour for every length in [0, 4].
#[kani::proof]
fn c_error_bounds() {
    let a = any_chord_length();
    let (p, q) = (a.max_point_error(), a.max_angle_error());
    kani::cover!(p > 0.0 && q > 0.0, "positive bounds");
    kani::cover!(q == 0.0, "zero angle error at zero");
}

/// C-s1::ChordAngle::sin, sin2, cos and tan: complete without panic (their debug checks never
/// fire), arithmetic overflow or undefined behaviour for every length in [0, 4], their
/// documented domain (FR-019).
#[kani::proof]
fn c_trigonometry() {
    let a = any_chord_length();
    let (s, s2, c, t) = (a.sin(), a.sin2(), a.cos(), a.tan());
    kani::cover!(s2 == 0.0 && c == -1.0, "STRAIGHT");
    kani::cover!(c == 0.0 && t.is_infinite(), "RIGHT: tan is infinite");
    kani::cover!(s > 0.0 && t > 0.0, "an acute angle");
}

/// C-s1::ChordAngle::Add and Sub: complete without panic (their debug checks never fire),
/// arithmetic overflow or undefined behaviour for every pair of lengths in [0, 4], their
/// documented domain (FR-019).
#[kani::proof]
fn c_add_sub() {
    let (a, b) = (any_chord_length(), any_chord_length());
    let (s, d) = (a + b, a - b);
    kani::cover!(
        s == ChordAngle::STRAIGHT && a != ChordAngle::STRAIGHT,
        "a sum reaching 4"
    );
    kani::cover!(d == ChordAngle::ZERO && a != b, "a difference clamped at 0");
    kani::cover!(
        s.length2() > a.length2() && d.length2() > 0.0,
        "an ordinary sum and difference"
    );
}
