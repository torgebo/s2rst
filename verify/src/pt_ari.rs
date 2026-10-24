// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The point's arithmetic (P-PTARI, spec 006 data-model §4.2): negation, addition and
//! subtraction, coordinate by coordinate, one member each (the whole statement timed out at 12
//! minutes in one harness, 2026-10-14).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::Point;

use crate::value_domain::any_finite_triple;

fn bits(p: Point) -> [u64; 3] {
    [p.0.x, p.0.y, p.0.z].map(f64::to_bits)
}

/// P-PTARI-01 (correct result), negation: coordinate-wise, bit for bit, and it undoes itself.
///
/// For every `a` in `FiniteTriple`, bit for bit: `−a` is `(−a.x, −a.y, −a.z)`, and `−(−a)` is `a`.
#[kani::proof]
fn p_ptari_01__neg() {
    let a = any_finite_triple();
    let n = -a;
    assert_eq!(bits(n), [-a.x(), -a.y(), -a.z()].map(f64::to_bits));
    assert_eq!(bits(-n), bits(a));
}

/// P-PTARI-01 (correct result), addition: coordinate-wise IEEE-754 addition, bit for bit.
///
/// For every `a`, `b` in `FiniteTriple`: `a + b` is `(a.x + b.x, a.y + b.y, a.z + b.z)`, bit
/// for bit.
#[kani::proof]
fn p_ptari_01__add() {
    let (a, b) = (any_finite_triple(), any_finite_triple());
    let s = a + b;
    assert_eq!(
        bits(s),
        [a.x() + b.x(), a.y() + b.y(), a.z() + b.z()].map(f64::to_bits)
    );
    kani::cover!(s.x().is_infinite(), "the sum overflows");
}

/// P-PTARI-01 (correct result), subtraction: coordinate-wise IEEE-754 subtraction, bit for bit,
/// which is adding the negation.
///
/// For every `a`, `b` in `FiniteTriple`, bit for bit: `a − b` is
/// `(a.x − b.x, a.y − b.y, a.z − b.z)`, and `a − b` is `a + (−b)`.
#[kani::proof]
fn p_ptari_01__sub() {
    let (a, b) = (any_finite_triple(), any_finite_triple());
    let d = a - b;
    assert_eq!(
        bits(d),
        [a.x() - b.x(), a.y() - b.y(), a.z() - b.z()].map(f64::to_bits)
    );
    assert_eq!(bits(d), bits(a + (-b)));
    kani::cover!(
        d.y() == 0.0 && d.y().is_sign_negative(),
        "a negative zero difference"
    );
}
