// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s1::Angle::<op>` for the 29 entries of the angle with crash
//! proofs (spec 007 US1, FR-004; data-model §5).
//!
//! An angle has no validity rule, so every domain is every `f64` of radians (`AnyAngle`), every
//! `i32` and every `u32`, narrowed only where an operation would produce a new NaN, which Kani
//! reports as a failed check:
//! - addition and subtraction without `∞ − ∞`, the products without `0 · ∞`, the quotients
//!   without `0 / 0` and `∞ / ∞`;
//! - the quotients by an infinite divisor, which CBMC reports as NaN although IEEE-754 gives
//!   ±0 (spec 006 research R0): a tool limit, checked by the C++ vectors (`ANGLE` rows with
//!   divisor +∞);
//! - `normalized` over every angle but ±∞, whose IEEE remainder is NaN (documented). Its crash
//!   proof does not depend on the value of `%`, which Kani models wrongly (research R0,
//!   Finding 1): every operation after it is a comparison, an addition or a subtraction of a
//!   finite value and the constant 2π, none of which can panic.
//!
//! `tan` calls the C library's `tan`, which Kani replaces with a failing check (research R0):
//! it has no harness, and the inventory records it as a tool limit.

use s2rst::s1::Angle;
use std::cmp::Ordering;

use crate::value_domain::any_angle;

/// Whether `a + b` is `∞ − ∞` (NaN): opposite infinities.
fn opposite_infinities(a: f64, b: f64) -> bool {
    a.is_infinite() && a == -b
}

/// C-s1::Angle::from_radians, radians, from_degrees, degrees, ZERO, INFINITY, Default, abs,
/// Neg and is_infinite: complete without panic, arithmetic overflow or undefined behaviour for
/// every `f64`, NaN and the infinities included.
#[kani::proof]
fn c_construction_and_accessors() {
    let x: f64 = kani::any();
    let a = Angle::from_radians(x);
    let d = Angle::from_degrees(x);
    let (r, g) = (a.radians(), d.degrees());
    let (z, i, def) = (Angle::ZERO, Angle::INFINITY, Angle::default());
    let (abs, neg, inf) = (a.abs(), -a, a.is_infinite());
    kani::cover!(r.is_nan() && g.is_nan(), "NaN");
    kani::cover!(inf && neg.radians() == f64::NEG_INFINITY, "+∞");
    kani::cover!(
        abs.radians() > 0.0 && neg.radians() > 0.0 && z == def && i.is_infinite(),
        "a negative angle"
    );
}

/// C-s1::Angle::e5, e6 and e7: complete without panic, arithmetic overflow or undefined
/// behaviour for every angle, NaN and the infinities included (they saturate, FR-023).
#[kani::proof]
fn c_e_forms() {
    let a = any_angle();
    let (e5, e6, e7) = (a.e5(), a.e6(), a.e7());
    kani::cover!(e5 == i32::MAX, "saturated above");
    kani::cover!(e7 == i32::MIN, "saturated below");
    kani::cover!(a.radians().is_nan() && e6 == 0, "NaN gives 0");
    kani::cover!(e7 > 0 && e7 < i32::MAX, "in range");
}

/// C-s1::Angle::from_e5, from_e6, from_e7, from_unsigned_e6 and from_unsigned_e7: complete
/// without panic, arithmetic overflow or undefined behaviour for every `i32` and `u32`.
#[kani::proof]
fn c_from_e_forms() {
    let (n, u): (i32, u32) = (kani::any(), kani::any());
    let (a, b, c) = (Angle::from_e5(n), Angle::from_e6(n), Angle::from_e7(n));
    let (d, e) = (Angle::from_unsigned_e6(u), Angle::from_unsigned_e7(u));
    kani::cover!(n == i32::MIN && a.radians() < 0.0, "the most negative E5");
    kani::cover!(
        u > 0x7fff_ffff && d.radians() < 0.0 && e.radians() < 0.0,
        "u32 wraps"
    );
    kani::cover!(b.radians() > 0.0 && c.radians() > 0.0, "positive");
}

/// C-s1::Angle::normalized: completes without panic, arithmetic overflow or undefined
/// behaviour for every angle but ±∞ (whose remainder is NaN, documented), NaN included.
#[kani::proof]
fn c_normalized() {
    let a = any_angle();
    kani::assume(!a.is_infinite());
    let n = a.normalized();
    kani::cover!(n.radians().is_nan(), "NaN");
    kani::cover!(!n.radians().is_nan(), "a number");
}

/// C-s1::Angle::sin, cos and sin_cos: complete without panic, arithmetic overflow or undefined
/// behaviour for every angle, NaN and the infinities included.
#[kani::proof]
fn c_trigonometry() {
    let a = any_angle();
    let (s, c, (s2, c2)) = (a.sin(), a.cos(), a.sin_cos());
    kani::cover!(
        s >= -1.0 && c <= 1.0 && s2 >= -1.0 && c2 <= 1.0,
        "a finite angle"
    );
}

/// C-s1::Angle::Add and Sub: complete without panic, arithmetic overflow or undefined behaviour
/// for every pair of angles except opposite infinities (`Add`) and equal infinities (`Sub`),
/// where IEEE-754 gives NaN.
#[kani::proof]
fn c_add_sub() {
    let (a, b) = (any_angle(), any_angle());
    let (x, y) = (a.radians(), b.radians());
    if !opposite_infinities(x, y) {
        let s = a + b;
        kani::cover!(s.is_infinite(), "an infinite sum");
    }
    if !opposite_infinities(x, -y) {
        let d = a - b;
        kani::cover!(d.radians() == 0.0, "a zero difference");
    }
}

/// C-s1::Angle::Mul<f64> and Mul<Angle> for f64: complete without panic, arithmetic overflow
/// or undefined behaviour for every angle and factor except zero times an infinity.
#[kani::proof]
fn c_mul() {
    let (a, m): (Angle, f64) = (any_angle(), kani::any());
    let x = a.radians();
    kani::assume(!((x == 0.0 && m.is_infinite()) || (x.is_infinite() && m == 0.0)));
    let (p, q) = (a * m, m * a);
    kani::cover!(p.is_infinite() && q.is_infinite(), "an infinite product");
}

/// C-s1::Angle::Div<f64> and Div<Angle>: complete without panic, arithmetic overflow or
/// undefined behaviour for every angle and finite divisor except `0 / 0`. An infinite divisor is
/// a tool limit (CBMC reports `x / ±∞` as NaN; IEEE-754 gives ±0), checked by the C++ vectors.
#[kani::proof]
fn c_div() {
    let (a, b) = (any_angle(), any_angle());
    let m: f64 = kani::any();
    let (x, y) = (a.radians(), b.radians());
    kani::assume(m.is_finite() && y.is_finite());
    kani::assume(!(x == 0.0 && m == 0.0) && !(x == 0.0 && y == 0.0));
    let (p, q) = (a / m, a / b);
    kani::cover!(m == 0.0 && p.is_infinite(), "division by zero");
    kani::cover!(q.is_finite() && q != 0.0, "an ordinary ratio");
}

/// C-s1::Angle::PartialEq and PartialOrd: complete without panic, arithmetic overflow or
/// undefined behaviour for every pair of angles, NaN included.
#[kani::proof]
fn c_order() {
    let (a, b) = (any_angle(), any_angle());
    let (eq, ord) = (a == b, a.partial_cmp(&b));
    kani::cover!(eq, "equal");
    kani::cover!(ord.is_none(), "NaN is unordered");
    kani::cover!(ord == Some(Ordering::Greater), "greater");
}

// ---------------------------------------------------------------------------------------------
// The tool limit of spec 007 (gap 2 of the 2026-10-26 coverage review): `tan`, proven
// crash-free under its function contract (`stubs::tan_contract`).
// ---------------------------------------------------------------------------------------------

/// C-s1::Angle::tan: completes without panic, overflow or undefined behaviour for every angle,
/// NaN and the infinities included, under `tan`'s contract.
///
/// Under contract: `tan_contract`.
#[kani::proof]
#[kani::stub(f64::tan, crate::stubs::tan_contract)]
fn c_fg_tan() {
    let a = any_angle();
    let t = a.tan();
    kani::cover!(t.is_nan(), "a non-finite angle");
    kani::cover!(t.is_finite(), "a finite tangent");
}
