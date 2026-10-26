// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The lemmas the lemma stubs rest on (L-CP-NN, spec 008 data-model §3; research R6). Each is
//! proven on the library's real code, with no stub: a stub in `stubs.rs` may return only values
//! these lemmas show the real function can return.

use s2rst::s1::ChordAngle;
use s2rst::s2::Cap;

use crate::value_domain::{any_box_point, any_chord_length};

/// L-CP-01 (law): the chord from a point to itself is zero.
///
/// For every `p` in `BoxPoint`: `p.chord_angle(p).length2() == 0`. `pure_chord` returns `ZERO`
/// for a pair of equal bit patterns.
#[kani::proof]
fn l_cp_01() {
    let p = any_box_point();
    assert!(p.chord_angle(p).length2() == 0.0);
}

/// L-CP-02 (law): the chord between two box points is a length in [0, 4].
///
/// For every `a`, `b` in `BoxPoint`: `0 ≤ a.chord_angle(b).length2() ≤ 4`. `pure_chord` returns
/// only such lengths. (006's P-PTDST-01 `__range` states the same; this is the lemma's own
/// harness, named for the stub.)
#[kani::proof]
fn l_cp_02() {
    let (a, b) = (any_box_point(), any_box_point());
    let l = a.chord_angle(b).length2();
    assert!((0.0..=4.0).contains(&l));
    kani::cover!(l > 0.0 && l < 4.0, "an ordinary chord");
}

/// L-CP-03 (law): negating a difference is exact: `b − a` is `−(a − b)`, bit for bit, for finite
/// operands, except that a zero difference is `+0` both ways.
///
/// With L-CP-04, the chord's symmetry follows by composition: `(b − a)` negates each coordinate
/// of `(a − b)`, the squares are then equal bit for bit, and the sum, the clamp at 4 and
/// `from_length2` take equal operands. That is `pure_chord_sym`'s fact, and 006's P-PTDST-01
/// symmetry clause (research R12).
#[kani::proof]
fn l_cp_03() {
    let (a, b): (f64, f64) = (kani::any(), kani::any());
    kani::assume(a.is_finite() && b.is_finite());
    let (d, e) = (a - b, b - a);
    assert!(e.to_bits() == (-d).to_bits() || d == 0.0);
    if d == 0.0 {
        assert!(e == 0.0);
    }
}

/// L-CP-04 (law): squaring a negated number is exact: `(−u) · (−u)` is `u · u`, bit for bit.
///
/// So negating any coordinate keeps the squared length bit for bit, the fact `pure_is_unit`
/// relies on to ignore the coordinates' signs (`is_unit(−p) == is_unit(p)`), and with L-CP-03
/// the chord's symmetry. (A direct harness on the squared length timed out at 12 minutes in
/// R0b; this is its one-variable core, and the rest is equal operands.)
#[kani::proof]
fn l_cp_04() {
    let u: f64 = kani::any();
    kani::assume(u.is_finite());
    assert!(((-u) * (-u)).to_bits() == (u * u).to_bits());
}

/// L-CP-05 (law): the sum of two lengths is in [0, 4] and at least each operand.
///
/// For every `a`, `b` in [0, 4]: `a.length2() ≤ (a + b).length2() ≤ 4` and
/// `b.length2() ≤ (a + b).length2()`. `pure_add` returns only such sums. Weekly: 626 s (R0b).
#[cfg(feature = "slow")]
#[kani::proof]
fn l_cp_05() {
    let (a, b) = (any_chord_length(), any_chord_length());
    let s = (a + b).length2();
    assert!((0.0..=4.0).contains(&s));
    assert!(s >= a.length2() && s >= b.length2());
}

/// L-CP-06 (law): the sum of two non-zero lengths is commutative: `a + b` is `b + a`, bit for
/// bit, for lengths in (0, 4]. Proven by composition from L-CP-06a and L-CP-06b.
///
/// The argument. With `a2`, `b2` the operands' lengths, `a + b` computes
/// `x = a2 · (1 − 0.25 · b2)`, `y = b2 · (1 − 0.25 · a2)` and `min(4, x + y + 2 · sqrt(x · y))`,
/// after the test `a2 + b2 >= 4`. Swapping the operands evaluates the same expressions with `x`
/// and `y` swapped, term for term. So the swapped sum differs only in `a2 + b2` against
/// `b2 + a2`, `x · y` against `y · x` and `x + y` against `y + x`: equal bit for bit by L-CP-06b
/// and L-CP-06a. The rest takes equal operands, and `sqrt` is correctly rounded (IEEE-754), so a
/// function. With a zero operand the sum returns its left operand when the right is zero, so
/// `+0 + −0` is `+0` but `−0 + +0` is `−0`; `pure_add_sym` applies those zero rules first.
///
/// A direct harness does not finish: on the real code the checker returned two different square
/// roots of one subnormal argument (3.5·10⁻³¹⁰) and failed the statement, where the native sums
/// agree bit for bit (5.826840342854593e-157 and 5.966677735361369e-154, replayed); with `sqrt`
/// stubbed purely it timed out at 12 minutes (2026-10-24).
///
/// L-CP-06a (law): IEEE multiplication commutes: `u · v` is `v · u`, bit for bit, for finite `u`,
/// `v`. IEEE-754 defines the product as the correctly rounded exact product, which is symmetric;
/// the checker does not finish it in 12 minutes (two symbolic multipliers, 2026-10-24), so it is
/// weekly, and `TRUSTED_BASE.md` records it as an IEEE fact.
#[cfg(feature = "slow")]
#[kani::proof]
fn l_cp_06a() {
    let (u, v): (f64, f64) = (kani::any(), kani::any());
    kani::assume(u.is_finite() && v.is_finite());
    assert!((u * v).to_bits() == (v * u).to_bits());
}

/// L-CP-06b (law): IEEE addition commutes: `u + v` is `v + u`, bit for bit, for finite `u`, `v`.
/// (The signs of zero agree too: `+0 + −0` and `−0 + +0` are both `+0` under round to nearest.)
#[kani::proof]
fn l_cp_06b() {
    let (u, v): (f64, f64) = (kani::any(), kani::any());
    kani::assume(u.is_finite() && v.is_finite());
    assert!((u + v).to_bits() == (v + u).to_bits());
}

/// L-CP-07 (law): the empty and full caps are valid (their centre (1, 0, 0) is unit length).
///
/// `pure_is_unit` returns true for (±1, 0, 0).
#[kani::proof]
fn l_cp_07() {
    assert!(Cap::empty().is_valid());
    assert!(Cap::full().is_valid());
    assert!(Cap::empty().center().is_unit() && (-Cap::empty().center()).is_unit());
}

/// L-CP-08 (law): adding a zero length returns the other operand, bit for bit, with either zero,
/// `ZERO` or the length −0, on either side: `a + z` is `a` for every length `a` in [0, 4], and
/// `z + b` is `b` for every length `b` in (0, 4].
///
/// These are `pure_add`'s two zero rules, which test `== 0.0` and so also fire for −0 (a length
/// the domain and `from_length2` allow); 007's P-CAARI-01 proves only `a + ZERO`. With `a2 = ±0`
/// the formula's `x` is ±0 and `y` is `b2`, so the result is `b2 + sqrt(±0)`. For `b = ±0` the
/// sum returns its left operand (`ZERO + −0` is `+0`; this harness found it, 2026-10-24), so
/// `pure_add` tests the right operand first, as the library does.
#[kani::proof]
fn l_cp_08() {
    let a = any_chord_length();
    let z = if kani::any() {
        ChordAngle::ZERO
    } else {
        ChordAngle::from_length2(-0.0)
    };
    assert!((a + z).length2().to_bits() == a.length2().to_bits());
    if a.length2() != 0.0 {
        assert!((z + a).length2().to_bits() == a.length2().to_bits());
    }
    kani::cover!(
        z.length2().is_sign_negative() && a.length2() != 0.0,
        "−0 + b"
    );
    kani::cover!(
        z.length2().is_sign_negative() && a.length2() == 0.0,
        "a zero + −0"
    );
}
