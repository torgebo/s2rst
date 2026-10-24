// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-r1::Interval::<op>` for the 26 in-scope operations of the real
//! interval (spec 005 US1, FR-004; data-model §5), grouped as spec 004's `iv_crash`.
//!
//! Each harness draws its inputs from the operation's domain in `values/INVENTORY.md` §3:
//! - every pair of non-NaN bounds, infinities, signed zeros and empty intervals included
//!   (`Real`), and every non-NaN number (`Number`), for the operations that compare and select
//!   bounds;
//! - for the arithmetic operations, the same, except the inputs on which `∞ − ∞` arises: the
//!   infinite corners their documentation describes (FR-017). Kani reports a NaN produced from
//!   non-NaN operands as a failed check (research R0), so those inputs are assumed away, and
//!   no others: the domains are exactly the documented ones, not `FiniteReal`;
//! - a non-empty interval and every `f64`, NaN included, for `project` (FR-019);
//! - every `f64`, NaN included, for `approx_eq_with`'s error bound.
//!
//! Kani builds with debug assertions on, so `project`'s `debug_assert!` and `f64::clamp`'s
//! assertion are obligations. The covers show the domains non-empty and the outcomes FR-009
//! names reachable: empty, single-point and infinite intervals.

use s2rst::r1::{Endpoint, Interval};

use crate::real_spec;
use crate::value_domain::{any_f64, any_number, any_real};

/// Whether `a − b` is `∞ − ∞` (NaN): both the same infinity.
fn inf_minus_inf(a: f64, b: f64) -> bool {
    a.is_infinite() && a == b
}

/// Whether `a + b` is `∞ − ∞` (NaN): opposite infinities.
fn inf_plus_minus_inf(a: f64, b: f64) -> bool {
    a.is_infinite() && a == -b
}

/// C-r1::Interval::new, empty, from_point, from_point_pair, From<(f64, f64)> and Default:
/// complete without panic, arithmetic overflow or undefined behaviour for every pair of
/// non-NaN numbers.
#[kani::proof]
fn c_constructors() {
    let (p, q) = (any_number(), any_number());
    let n = Interval::new(p, q);
    let pair = Interval::from_point_pair(p, q);
    let single = Interval::from_point(p);
    let from = Interval::from((p, q));
    let (e, d) = (Interval::empty(), Interval::default());
    kani::cover!(real_spec::empty(n), "an empty interval through new");
    kani::cover!(p > q && pair.lo == q, "from_point_pair orders the points");
    kani::cover!(single.lo.is_infinite(), "an infinite single point");
    kani::cover!(
        from.lo == p && e.lo == d.lo,
        "the conversion and the empty default"
    );
}

/// C-r1::Interval::is_empty, bound and eq: complete without panic, arithmetic overflow or
/// undefined behaviour for every pair of intervals in `Real`, and both endpoints.
#[kani::proof]
fn c_classifiers() {
    let (x, y) = (any_real(), any_real());
    let lo: bool = kani::any();
    let i = if lo { Endpoint::Lo } else { Endpoint::Hi };
    let (empty, b, eq) = (x.is_empty(), x.bound(i), x == y);
    kani::cover!(empty, "empty");
    kani::cover!(!empty && b.is_infinite(), "an infinite bound");
    kani::cover!(
        eq && x.lo != y.lo,
        "two empty intervals with different bounds are equal"
    );
}

/// C-r1::Interval::contains, interior_contains and add_point: complete without panic,
/// arithmetic overflow or undefined behaviour for every interval in `Real` and every non-NaN
/// number.
#[kani::proof]
fn c_point_operations() {
    let x = any_real();
    let p = any_number();
    let (c, ic) = (x.contains(p), x.interior_contains(p));
    let a = x.add_point(p);
    kani::cover!(c && !ic, "on the boundary");
    kani::cover!(!c && a.lo == p && !x.is_empty(), "extended at the low end");
    kani::cover!(
        x.is_empty() && a.lo == p && a.hi == p,
        "the empty interval becomes a point"
    );
}

/// C-r1::Interval::contains_interval, interior_contains_interval, intersects,
/// interior_intersects, add_interval, union and intersection: complete without panic,
/// arithmetic overflow or undefined behaviour for every pair of intervals in `Real`.
#[kani::proof]
fn c_relations_and_set_operations() {
    let (x, y) = (any_real(), any_real());
    let r = (
        x.contains_interval(y),
        x.interior_contains_interval(y),
        x.intersects(y),
        x.interior_intersects(y),
    );
    let (a, u, i) = (x.add_interval(y), x.union(y), x.intersection(y));
    kani::cover!(r.0 && !r.1, "contained, touching the boundary");
    kani::cover!(r.2 && !r.3, "intersecting only at a bound");
    kani::cover!(!x.is_empty() && !y.is_empty() && i.is_empty(), "disjoint");
    kani::cover!(
        u.lo == y.lo && a.hi == x.hi && u.lo != x.lo,
        "the union takes both sides"
    );
}

/// C-r1::Interval::length and center: complete without panic, arithmetic overflow or undefined
/// behaviour for every interval in `Real` except their documented infinite corners (FR-017):
/// `[∞, ∞]` and `[−∞, −∞]` for the length, `[−∞, ∞]` and `[∞, −∞]` for the center.
#[kani::proof]
fn c_measures() {
    let x = any_real();
    kani::assume(!inf_minus_inf(x.hi, x.lo));
    kani::assume(!inf_plus_minus_inf(x.lo, x.hi));
    let (len, c) = (x.length(), x.center());
    kani::cover!(len < 0.0, "an empty interval's negative length");
    kani::cover!(len.is_infinite() && !x.is_empty(), "an infinite length");
    kani::cover!(
        x.lo > f64::MAX / 2.0 && c.is_finite(),
        "a center beyond f64::MAX / 2 (U-010)"
    );
}

/// C-r1::Interval::directed_hausdorff_distance: completes without panic, arithmetic overflow
/// or undefined behaviour for every pair of intervals in `Real` on which `∞ − ∞` does not
/// arise: when both are non-empty, not both high bounds, nor both low bounds, the same
/// infinity (FR-017).
#[kani::proof]
fn c_directed_hausdorff_distance() {
    let (x, y) = (any_real(), any_real());
    kani::assume(
        x.is_empty() || y.is_empty() || (!inf_minus_inf(x.hi, y.hi) && !inf_minus_inf(y.lo, x.lo)),
    );
    let d = x.directed_hausdorff_distance(y);
    kani::cover!(d == 0.0 && !x.is_empty(), "contained");
    kani::cover!(d.is_infinite() && !y.is_empty(), "infinitely far");
    kani::cover!(d > 0.0 && d.is_finite(), "a finite distance");
}

/// C-r1::Interval::expanded: completes without panic, arithmetic overflow or undefined
/// behaviour for every interval in `Real` and every non-NaN margin, except the documented
/// infinite corner: an infinite margin against an infinite bound of the opposite sign
/// (FR-017).
#[kani::proof]
fn c_expanded() {
    let x = any_real();
    let m = any_number();
    kani::assume(x.is_empty() || (!inf_minus_inf(x.lo, m) && !inf_plus_minus_inf(x.hi, m)));
    let e = x.expanded(m);
    kani::cover!(!x.is_empty() && e.is_empty(), "shrunk to empty");
    kani::cover!(m.is_infinite() && e.lo.is_infinite(), "an infinite margin");
}

/// C-r1::Interval::project: completes without panic, arithmetic overflow or undefined
/// behaviour for every non-empty interval in `Real` and every `f64`, NaN included (FR-019:
/// it panics on an empty interval).
#[kani::proof]
fn c_project() {
    let x = any_real();
    kani::assume(!real_spec::empty(x));
    let p = any_f64();
    let q = x.project(p);
    kani::cover!(q == p, "inside");
    kani::cover!(q != p && q == x.lo, "the low bound is nearer");
    kani::cover!(q != p && q == x.hi, "the high bound is nearer");
    kani::cover!(q.is_nan(), "a NaN point");
}

/// C-r1::Interval::approx_eq_with and approx_eq: complete without panic, arithmetic overflow
/// or undefined behaviour for every pair of intervals in `Real` on which `∞ − ∞` does not
/// arise, and every error bound, NaN included (FR-017).
#[kani::proof]
fn c_approx_eq() {
    let (x, y) = (any_real(), any_real());
    let e = any_f64();
    // The subtractions on the path: an empty side compares the other side's length.
    if x.is_empty() {
        kani::assume(!inf_minus_inf(y.hi, y.lo));
    } else if y.is_empty() {
        kani::assume(!inf_minus_inf(x.hi, x.lo));
    } else {
        kani::assume(!inf_minus_inf(y.lo, x.lo) && !inf_minus_inf(y.hi, x.hi));
    }
    let (a, d) = (x.approx_eq_with(y, e), x.approx_eq(y));
    kani::cover!(a && x.lo != y.lo, "approximately equal, not equal");
    kani::cover!(
        d && x.is_empty() && !y.is_empty(),
        "a short interval matches the empty one"
    );
}
