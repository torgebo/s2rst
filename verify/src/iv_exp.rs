// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval expansion and approximate equality (P-IEXP, spec 004 data-model §4.2), against
//! `arc_spec`. The containment clause of P-IEXP-01 is split by the margin's sign and by
//! inversion, and the symmetry clause of P-IEXP-02 by inversion (research R9). Every member of
//! both splits timed out at 12 minutes (2026-10-12), so they are in the weekly tier (feature
//! `slow`, 60 minutes each).
//!
//! The weekly member `p_iexp_01_shrink__xinv` found U-011 (2026-10-13): upstream's bounds can
//! cross when shrinking to almost nothing. P-IEXP-03 states the fix (FR-023) against
//! `arc_spec::upstream_expanded` and `arc_spec::contains`, so it also gives P-IEXP-01's
//! containment clause: a result is either upstream's, where that keeps containment, or empty or
//! full. (The library's guard uses its own `contains_interval`, which P-IREL-02 proves is
//! `arc_spec::contains`; the statement does not rely on it.) Its two members
//! timed out at 12 minutes per change (2026-10-13) and at 60 minutes in the weekly tier
//! (2026-10-14), so P-IEXP-03 is not proven (tool limit); P-IEXP-01's members prove the
//! containment directly.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::Interval;

use crate::arc_spec;
use crate::value_domain::{any_f64, any_interval};

fn bits(x: Interval) -> [u64; 2] {
    [x.lo.to_bits(), x.hi.to_bits()]
}

/// P-IEXP-01 (correct result): expanding by any margin except NaN gives a valid interval, and
/// the empty and full intervals stay so.
///
/// For every valid `x` and margin `m` not NaN (K5): `x.expanded(m)` is valid; `empty(x) ⇒`
/// empty; `full(x) ⇒` full.
#[kani::proof]
fn p_iexp_01() {
    let x = any_interval();
    let m = any_f64();
    kani::assume(!m.is_nan());
    let e = x.expanded(m);
    assert!(arc_spec::valid(e.lo, e.hi));
    if arc_spec::empty(x) {
        assert!(arc_spec::empty(e));
    }
    if arc_spec::full(x) {
        assert!(arc_spec::full(e));
    }
    kani::cover!(
        arc_spec::is_corner(x) && arc_spec::full(e),
        "the corner grows to full"
    );
}

/// The containment clause of P-IEXP-01 for a finite margin of one sign, over `x` inverted or not:
/// growing contains the original, shrinking is contained in it.
#[cfg_attr(
    not(feature = "slow"),
    allow(dead_code, reason = "only the weekly members use it (research R9)")
)]
fn containment(grow: bool, x_inverted: bool) {
    let x = any_interval();
    kani::assume(arc_spec::inverted(x) == x_inverted);
    let m = any_f64();
    kani::assume(m.is_finite() && (m >= 0.0) == grow);
    let e = x.expanded(m);
    if grow {
        assert!(arc_spec::contains(e, x));
    } else {
        assert!(arc_spec::contains(x, e));
    }
}

/// P-IEXP-01 (correct result): expanding by a finite non-negative margin contains the original
/// (an inverted `x`).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_01_grow__xinv() {
    containment(true, true);
}

/// P-IEXP-01 (correct result): expanding by a finite non-negative margin contains the original
/// (a non-inverted `x`).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_01_grow__xnoninv() {
    containment(true, false);
}

/// P-IEXP-01 (correct result): shrinking by a finite negative margin gives an interval the
/// original contains (an inverted `x`).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_01_shrink__xinv() {
    containment(false, true);
}

/// P-IEXP-01 (correct result): shrinking by a finite negative margin gives an interval the
/// original contains (a non-inverted `x`).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_01_shrink__xnoninv() {
    containment(false, false);
}

/// P-IEXP-02 (law): approximate equality is reflexive for every finite non-negative error
/// bound.
///
/// For every valid `x` and finite `e ≥ 0`: `x.approx_eq_with(x, e)`.
#[kani::proof]
fn p_iexp_02_reflexive() {
    let x = any_interval();
    let e = any_f64();
    kani::assume(e.is_finite() && e >= 0.0);
    assert!(x.approx_eq_with(x, e));
    kani::cover!(arc_spec::is_corner(x), "the corner");
}

/// The symmetry clause of P-IEXP-02 over `x` and `y` inverted or not.
#[cfg_attr(
    not(feature = "slow"),
    allow(dead_code, reason = "only the weekly members use it (research R9)")
)]
fn symmetric(x_inverted: bool, y_inverted: bool) {
    let (x, y) = (any_interval(), any_interval());
    kani::assume(arc_spec::inverted(x) == x_inverted && arc_spec::inverted(y) == y_inverted);
    let e = any_f64();
    assert_eq!(x.approx_eq_with(y, e), y.approx_eq_with(x, e));
}

/// P-IEXP-02 (law): approximate equality is symmetric, for every error bound (`x`, `y` both
/// inverted).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_02_symmetric__xinv_yinv() {
    symmetric(true, true);
}

/// P-IEXP-02 (law): approximate equality is symmetric, for every error bound (`x` inverted, `y`
/// not).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_02_symmetric__xinv_ynoninv() {
    symmetric(true, false);
}

/// P-IEXP-02 (law): approximate equality is symmetric, for every error bound (`x` not inverted,
/// `y` inverted).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_02_symmetric__xnoninv_yinv() {
    symmetric(false, true);
}

/// P-IEXP-02 (law): approximate equality is symmetric, for every error bound (neither
/// inverted).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_02_symmetric__xnoninv_ynoninv() {
    symmetric(false, false);
}

/// P-IEXP-02 (law): the default form uses an error bound of 1e-15.
///
/// For every valid `x`, `y`: `x.approx_eq(y) == x.approx_eq_with(y, 1e-15)`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_02_default() {
    let (x, y) = (any_interval(), any_interval());
    assert_eq!(x.approx_eq(y), x.approx_eq_with(y, 1e-15));
}

/// P-IEXP-03 for one sign of the margin: the expansion is upstream's arithmetic, kept when it
/// keeps containment, and otherwise the empty interval (shrinking) or the full one (growing).
/// Containment is the description's, `arc_spec::contains`, not the library's `contains_interval`
/// (code review of v0.4.0..74b9bc1, finding 9).
#[cfg_attr(
    not(feature = "slow"),
    allow(dead_code, reason = "only the weekly members use it")
)]
fn confined(grow: bool) {
    let x = any_interval();
    let m = any_f64();
    kani::assume(!m.is_nan() && (m >= 0.0) == grow);
    let (e, u) = (x.expanded(m), arc_spec::upstream_expanded(x, m));
    let kept = if grow {
        arc_spec::contains(u, x)
    } else {
        arc_spec::contains(x, u)
    };
    if kept {
        assert_eq!(bits(e), bits(u));
    } else if grow {
        assert!(arc_spec::full(e));
    } else {
        assert!(arc_spec::empty(e));
    }
    kani::cover!(
        kept && !arc_spec::empty(u) && !arc_spec::full(u),
        "upstream's result, neither empty nor full"
    );
}

/// P-IEXP-03 (exact output): the fix for U-011 is narrow (FR-023). The expansion is
/// upstream's, bit for bit, unless upstream's bounds have crossed, and then it is the empty
/// interval when shrinking and the full one when growing.
///
/// For every valid `x` and margin `m < 0`, with `u = arc_spec::upstream_expanded(x, m)`:
/// `x.expanded(m)` is `u` bit for bit if `arc_spec::contains(x, u)`, and empty otherwise.
/// Kani's counterexample to P-IEXP-01, where the second case arises, is the regression test
/// `test_expanded_crossing_bounds_give_empty` in `core/src/s1/interval.rs`.
///
/// Weekly: it timed out at 12 minutes per change and at 60 minutes weekly (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_03__mneg() {
    confined(false);
}

/// P-IEXP-03 (exact output): as `p_iexp_03__mneg`, for growing.
///
/// For every valid `x` and margin `m ≥ 0`, with `u = arc_spec::upstream_expanded(x, m)`:
/// `x.expanded(m)` is `u` bit for bit if `arc_spec::contains(u, x)`, and full otherwise. No
/// input is known where the second case arises.
///
/// Weekly: it timed out at 12 minutes per change and at 60 minutes weekly (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_iexp_03__mpos() {
    confined(true);
}
