// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The point's distance and chord angle (P-PTDST, spec 006 data-model §4.2), over the box
//! `[−1, 1]³`.
//!
//! `distance`, `stable_angle` and the approximate equalities reach `f64::atan2`, which Kani
//! replaces with a failing check (research R0). `r3::Vector::angle` returns 0 early for equal
//! vectors, so P-PTDST-01 states their values for equal points only. P-PTDST-02, their
//! symmetry, has no harness: it is not proven (tool limit), and the C++ vectors and a property
//! test check it.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::Angle;

use crate::point_spec;
use crate::value_domain::{any_box_point, any_finite_angle};

/// P-PTDST-01 (correct result), to itself: the distance and the chord angle from a point to
/// itself are zero, and the approximate equalities are reflexive for every non-negative
/// tolerance (each reaches only `angle`'s early return).
///
/// For every `p` in `BoxPoint` and finite `e ≥ 0`: `p.distance(p)` and
/// `p.chord_angle(p).length2()` are 0; `p.approx_eq(p)`, `p.approx_eq_angle(p, e)` and
/// `p.approx_eq_with(p, e)` hold.
#[kani::proof]
fn p_ptdst_01__self() {
    let p = any_box_point();
    assert!(p.distance(p).radians() == 0.0);
    assert!(p.chord_angle(p).length2() == 0.0);
    let e: Angle = any_finite_angle();
    kani::assume(e.radians() >= 0.0);
    assert!(p.approx_eq(p));
    assert!(p.approx_eq_angle(p, e));
    assert!(p.approx_eq_with(p, e));
}

/// P-PTDST-01 (correct result), the range: the chord angle's squared length is in [0, 4].
///
/// For every `p`, `q` in `BoxPoint`: `0 ≤ p.chord_angle(q).length2() ≤ 4`.
#[kani::proof]
fn p_ptdst_01__range() {
    let (p, q) = (any_box_point(), any_box_point());
    let c = p.chord_angle(q).length2();
    assert!((0.0..=4.0).contains(&c));
    kani::cover!(c == 4.0, "clamped to a straight angle");
    kani::cover!(!point_spec::same(p, q) && c > 0.0, "distinct points");
}

// P-PTDST-01's symmetry clause (`p.chord_angle(q)` is `q.chord_angle(p)`, bit for bit) has no
// harness here: a direct one timed out at 60 minutes (2026-10-14). Spec 008 proves it by
// composition from `cp_lem::l_cp_03` (`b − a` is `−(a − b)`, bit for bit) and `cp_lem::l_cp_04`
// (`(−u)·(−u)` is `u·u`): the three differences negate exactly, their squares are equal, and the
// sum, the clamp at 4 and `from_length2` take equal operands.
