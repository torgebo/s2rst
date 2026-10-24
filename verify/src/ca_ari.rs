// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's arithmetic and trigonometry (P-CAARI, spec 007 data-model §4.1), over the
//! non-special values, their documented domain (FR-019). The sum, the difference, `sin` and
//! `tan` go through `sqrt`, which the checker constrains without fixing (spec 006 research R0):
//! each property states facts of one result, never compares two.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::ChordAngle;

use crate::chord_spec;
use crate::value_domain::any_chord_length;

/// P-CAARI-01 (correct result): the sum of two lengths is in [0, 4]; adding `ZERO` returns the
/// other operand; lengths adding up to at least 4 give `STRAIGHT`.
///
/// For every `a`, `b` in [0, 4]: `(a + b).length2()` is in [0, 4]; `a + ZERO` is `a`, bit for
/// bit; and if `a.length2() + b.length2() >= 4`, `a + b` is `STRAIGHT`.
#[kani::proof]
fn p_caari_01() {
    let (a, b) = (any_chord_length(), any_chord_length());
    let s = (a + b).length2();
    assert!((0.0..=4.0).contains(&s));
    assert!((a + ChordAngle::ZERO).length2().to_bits() == a.length2().to_bits());
    if a.length2() + b.length2() >= 4.0 {
        assert!(s == 4.0);
    }
    kani::cover!(s > 0.0 && s < 4.0, "an ordinary sum");
}

/// P-CAARI-02 (correct result): the difference of two lengths is in [0, 4]; subtracting a zero
/// returns the first operand; otherwise subtracting a length at least as large gives `ZERO`.
///
/// For every `a`, `b` in [0, 4]: `(a - b).length2()` is in [0, 4]; if `b` is a zero (either
/// sign), `a - b` is `a`, bit for bit (upstream's `if (b2 == 0) return a`, so `−0 − −0` is
/// `−0`); otherwise, if `a.length2() <= b.length2()`, `a - b` is `ZERO`, bit for bit.
#[kani::proof]
fn p_caari_02() {
    let (a, b) = (any_chord_length(), any_chord_length());
    let d = (a - b).length2();
    assert!((0.0..=4.0).contains(&d));
    if b.length2() == 0.0 {
        assert!(d.to_bits() == a.length2().to_bits());
    } else if a.length2() <= b.length2() {
        assert!(d.to_bits() == 0.0f64.to_bits());
    }
    kani::cover!(d > 0.0, "a positive difference");
}

/// P-CAARI-03 (exact output), the cosine: `cos` is upstream's formula, bit for bit.
///
/// For every length `l` in [0, 4]: `cos` is `chord_spec::upstream_cos(l)`.
#[kani::proof]
fn p_caari_03__cos() {
    let a = any_chord_length();
    assert!(a.cos().to_bits() == chord_spec::upstream_cos(a.length2()).to_bits());
}

/// P-CAARI-03 (exact output), the squared sine: `sin2` is upstream's formula, bit for bit
/// (weekly: it asserts on a product of symbolic values, over 12 minutes per change).
///
/// For every length `l` in [0, 4]: `sin2` is `chord_spec::upstream_sin2(l)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_caari_03__sin2() {
    let a = any_chord_length();
    assert!(a.sin2().to_bits() == chord_spec::upstream_sin2(a.length2()).to_bits());
}

/// P-CAARI-03 (exact output), the sine's range: `sin` lies in [0, 1] (weekly: the bound needs
/// the product inside `sin2`, over 12 minutes per change).
///
/// For every length `l` in [0, 4]: `0 ≤ sin ≤ 1`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_caari_03__sin() {
    let a = any_chord_length();
    let s = a.sin();
    assert!((0.0..=1.0).contains(&s));
    kani::cover!(s > 0.0 && s < 1.0, "an ordinary sine");
}
