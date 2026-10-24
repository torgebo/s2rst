// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval measures (P-IMEA, spec 004 data-model §4.2), with the FR-020 containment proof:
//! P-IMEA-01 shows that the seam fix changes the length of exactly one valid interval, and
//! P-IMEA-03 that it is the only one upstream's arithmetic gets wrong (research R5).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use std::f64::consts::PI;

use crate::arc_spec::{self, ULP_PI, member};
use crate::value_domain::any_interval;

/// P-IMEA-01 (exact output): the seam fix changes the length of exactly one valid interval: for
/// every other one, the length is bit for bit upstream's arithmetic, and at the corner it is one
/// ulp of π.
///
/// For every valid `x` (struct literals included): if `arc_spec::is_corner(x)`,
/// `x.length() == ULP_PI`; otherwise `x.length().to_bits() ==
/// arc_spec::upstream_length(x).to_bits()` (upstream `GetLength`, `s1interval.cc:41–47`;
/// spec FR-020, `UPSTREAM.md` U-007).
#[kani::proof]
fn p_imea_01() {
    let x = any_interval();
    if arc_spec::is_corner(x) {
        assert_eq!(x.length().to_bits(), ULP_PI.to_bits());
    } else {
        assert_eq!(x.length().to_bits(), arc_spec::upstream_length(x).to_bits());
    }
    kani::cover!(arc_spec::is_corner(x), "the corner");
    kani::cover!(x.is_inverted() && !x.is_empty(), "an inverted interval");
}

/// P-IMEA-02 (law): every non-empty interval has a non-negative length, positive unless it is a
/// single point; the empty interval's is −1 and the full one's is 2π.
///
/// For every valid `x`: `empty(x) ⇒ length == −1`; otherwise `length ≥ 0`, and `> 0` when
/// `x.lo != x.hi`; `full(x) ⇒ length == 2π`.
#[kani::proof]
fn p_imea_02() {
    let x = any_interval();
    let l = x.length();
    if arc_spec::empty(x) {
        assert_eq!(l, -1.0);
    } else {
        assert!(l >= 0.0);
        if x.lo != x.hi {
            assert!(l > 0.0);
        }
    }
    if arc_spec::full(x) {
        assert_eq!(l, 2.0 * PI);
    }
    kani::cover!(
        arc_spec::is_corner(x) && l > 0.0,
        "the corner has a positive length"
    );
}

/// P-IMEA-03 (model check): the corner is the only valid, non-empty interval for which
/// upstream's arithmetic gives a negative length.
///
/// For every valid `x`, calling only `arc_spec`: `¬empty(x) ∧ upstream_length(x) < 0 ⇒
/// is_corner(x)`.
#[kani::proof]
fn p_imea_03() {
    let x = any_interval();
    if !arc_spec::empty(x) && arc_spec::upstream_length(x) < 0.0 {
        assert!(arc_spec::is_corner(x));
    }
    kani::cover!(arc_spec::is_corner(x), "the corner is a valid interval");
}

/// P-IMEA-04 (correct result): the centre of an interval that is neither empty nor full lies in
/// it.
///
/// For every valid `x` neither empty nor full: `member(x, x.center())`.
#[kani::proof]
fn p_imea_04() {
    let x = any_interval();
    kani::assume(!arc_spec::empty(x) && !arc_spec::full(x));
    assert!(member(x, x.center()));
    kani::cover!(x.is_inverted(), "an inverted interval");
}

/// P-IMEA-05 (correct result): the complement's centre lies in the complement, for an interval
/// that is neither empty nor full.
///
/// For every valid `x` neither empty nor full: `member(x.complement(), x.complement_center())`
/// (P-ISET-01 proves the complement).
#[kani::proof]
fn p_imea_05() {
    let x = any_interval();
    kani::assume(!arc_spec::empty(x) && !arc_spec::full(x));
    assert!(member(x.complement(), x.complement_center()));
    kani::cover!(x.lo == x.hi, "a single point: the antipode");
}

/// P-IMEA-06 (correct result): the directed Hausdorff distance lies in [0, π]. It is 0 when the
/// second interval contains the first, and π when only the second is empty. The converse of
/// the first clause holds up to rounding only: when `y`'s gap is narrower than the floats
/// around it resolve (a one-ulp gap), every point of `x` lies within half an ulp of `y`, and
/// the distance rounds to 0 although `y` does not contain `x` (found by this harness's first
/// draft; upstream computes the same).
///
/// The 0 and π clauses, for every valid `x`, `y`: `arc_spec::contains(y, x) ⇒ d == 0`, and
/// `empty(y) ∧ ¬empty(x) ⇒ d == π`.
#[kani::proof]
fn p_imea_06() {
    let (x, y) = (any_interval(), any_interval());
    let d = x.directed_hausdorff_distance(y);
    if arc_spec::contains(y, x) {
        assert_eq!(d, 0.0);
    }
    if arc_spec::empty(y) && !arc_spec::empty(x) {
        assert_eq!(d, PI);
    }
    kani::cover!(d > 0.0 && d < PI, "a partial distance");
    kani::cover!(
        d == 0.0 && !arc_spec::contains(y, x),
        "a sub-ulp gap: the distance rounds to 0"
    );
}

/// The range clause of P-IMEA-06 over `y` inverted or not and `x` inverted or not (`None`: either).
fn dhd_range(y_inverted: bool, x_inverted: Option<bool>) {
    let (x, y) = (any_interval(), any_interval());
    kani::assume(arc_spec::inverted(y) == y_inverted);
    if let Some(inv) = x_inverted {
        kani::assume(arc_spec::inverted(x) == inv);
    }
    let d = x.directed_hausdorff_distance(y);
    assert!((0.0..=PI).contains(&d));
}

/// P-IMEA-06 (correct result): the range clause, `0 ≤ d ≤ π`, for an inverted `y`.
#[kani::proof]
fn p_imea_06_range__yinv() {
    dhd_range(true, None);
}

/// P-IMEA-06 (correct result): the range clause, `0 ≤ d ≤ π`, for a non-inverted `y` and an
/// inverted `x`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_imea_06_range__ynoninv_xinv() {
    dhd_range(false, Some(true));
}

/// P-IMEA-06 (correct result): the range clause, `0 ≤ d ≤ π`, for a non-inverted `y` and a
/// non-inverted `x`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_imea_06_range__ynoninv_xnoninv() {
    dhd_range(false, Some(false));
}
