// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval expansion, projection and approximate equality (P-REXP, spec 005 data-model
//! §4.1), against the real-set description (`real_spec`).

use s2rst::r1::Interval;

use crate::real_spec;
use crate::value_domain::{any_finite_real, any_number, any_real};

/// P-REXP-01 (correct result): expanding by a finite non-negative margin contains the
/// original; by a finite non-positive one, it is contained in it; the empty interval stays as
/// it is.
///
/// For every `x` in `FiniteReal` and finite `m`, with `e = x.expanded(m)`: `empty(x) ⇒ e` is
/// `x`, bit for bit; `m ≥ 0 ⇒ contains(e, x)`; `m ≤ 0 ⇒ contains(x, e)`.
#[kani::proof]
fn p_rexp_01() {
    let x = any_finite_real();
    let m = any_number();
    kani::assume(m.is_finite());
    let e = x.expanded(m);
    if real_spec::empty(x) {
        assert_eq!(
            (e.lo.to_bits(), e.hi.to_bits()),
            (x.lo.to_bits(), x.hi.to_bits())
        );
    }
    if m >= 0.0 {
        assert!(real_spec::contains(e, x));
    }
    if m <= 0.0 {
        assert!(real_spec::contains(x, e));
    }
    kani::cover!(
        m < 0.0 && real_spec::empty(e) && !x.is_empty(),
        "shrunk to empty"
    );
    kani::cover!(e.lo.is_infinite(), "grown to an infinite bound");
}

/// P-REXP-02 (correct result): projection gives the number itself if it is in the interval,
/// otherwise the nearer bound.
///
/// For every non-empty `x` in `Real` and non-NaN `p`, with `q = x.project(p)`:
/// `member(x, p) ⇒ q` is `p`; `p < x.lo ⇒ q` is `x.lo`; `p > x.hi ⇒ q` is `x.hi`; bit for bit.
#[kani::proof]
fn p_rexp_02() {
    let x = any_real();
    kani::assume(!real_spec::empty(x));
    let p = any_number();
    let q = x.project(p).to_bits();
    if real_spec::member(x, p) {
        assert_eq!(q, p.to_bits());
    } else if p < x.lo {
        assert_eq!(q, x.lo.to_bits());
    } else {
        assert!(p > x.hi);
        assert_eq!(q, x.hi.to_bits());
    }
}

/// The domain of P-REXP-03: two intervals in `FiniteReal` and a finite error bound `e ≥ 0`.
fn approx_eq_inputs() -> (Interval, Interval, f64) {
    let (x, y) = (any_finite_real(), any_finite_real());
    let e = any_number();
    kani::assume(e.is_finite() && e >= 0.0);
    (x, y, e)
}

/// P-REXP-03 (law): approximate equality is reflexive for every finite non-negative error
/// bound. The clauses of P-REXP-03 are separate harnesses: together they timed out at 12
/// minutes (2026-10-13).
///
/// For every `x` in `FiniteReal` and finite `e ≥ 0`: `x.approx_eq_with(x, e)`.
#[kani::proof]
fn p_rexp_03_reflexive() {
    let (x, _, e) = approx_eq_inputs();
    assert!(x.approx_eq_with(x, e));
    kani::cover!(x.is_empty(), "an empty interval");
}

/// P-REXP-03 (law): approximate equality is symmetric.
///
/// For every `x`, `y` in `FiniteReal` and finite `e ≥ 0`:
/// `x.approx_eq_with(y, e) == y.approx_eq_with(x, e)`.
///
/// Weekly: it took 474–565 s per change (2026-10-13) and timed out at 12 minutes in spec
/// 006's final run of all six parts (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_rexp_03_symmetric() {
    let (x, y, e) = approx_eq_inputs();
    let a = x.approx_eq_with(y, e);
    assert_eq!(a, y.approx_eq_with(x, e));
    kani::cover!(a && x.lo != y.lo, "approximately equal, not equal");
    kani::cover!(
        a && x.is_empty() != y.is_empty(),
        "a short interval matches an empty one"
    );
}

/// P-REXP-03 (law): the default form uses an error bound of 1e-15.
///
/// For every `x`, `y` in `FiniteReal`: `x.approx_eq(y) == x.approx_eq_with(y, 1e-15)`.
#[kani::proof]
fn p_rexp_03_default() {
    let (x, y, _) = approx_eq_inputs();
    assert_eq!(x.approx_eq(y), x.approx_eq_with(y, 1e-15));
}
