// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a real interval (spec 005 data-model §1.1): the closed set of
//! numbers between its bounds.
//!
//! Written from upstream's `R1Interval` header (`r1interval.h`, `v0.14.0-40`): an interval is
//! the closed set `{p : lo ≤ p ≤ hi}`, empty exactly when `lo > hi`, so the empty set has many
//! representations, `[1, 0]` among them. Bounds are `f64` values other than NaN, the
//! infinities included, and `−0` and `0` are the same number.
//!
//! Every relation is that of the two sets of real numbers. For closed real intervals the sets'
//! relations are decided exactly by comparing the bounds, so, unlike for 004's arcs
//! (`arc_spec`), there is no gap between float points and real points to account for.
//!
//! The description reads an [`Interval`]'s two public fields and never calls its methods
//! (FR-007). Booleans are combined with `&&`, `||` and `==` only (spec 004 research R13).

use s2rst::r1::Interval;

/// Whether `x` denotes the empty set: `lo > hi` (`r1interval.h:93`).
#[must_use]
pub fn empty(x: Interval) -> bool {
    x.lo > x.hi
}

/// Whether both bounds are finite: the domain of the arithmetic properties (spec FR-004,
/// FR-017), where `∞ − ∞` cannot arise.
#[must_use]
pub fn finite(x: Interval) -> bool {
    x.lo.is_finite() && x.hi.is_finite()
}

/// Whether the number `p` is in the closed set `x` (`Contains(double)`, `r1interval.h:104`).
#[must_use]
pub fn member(x: Interval, p: f64) -> bool {
    x.lo <= p && p <= x.hi
}

/// Whether `p` is in the open set `(lo, hi)`, the interior of `x`
/// (`InteriorContains(double)`, `r1interval.h:109`).
#[must_use]
pub fn interior(x: Interval, p: f64) -> bool {
    x.lo < p && p < x.hi
}

/// Whether the set `x` contains the set `y`. The empty set is contained in every set; a
/// non-empty `y` is contained exactly when its bounds lie within `x`'s, which also rules out an
/// empty `x` (`Contains(const R1Interval&)`, `r1interval.h:114`).
#[must_use]
pub fn contains(x: Interval, y: Interval) -> bool {
    empty(y) || (x.lo <= y.lo && y.hi <= x.hi)
}

/// Whether the interior of `x` contains the set `y` (`InteriorContains(const R1Interval&)`,
/// `r1interval.h:121`).
#[must_use]
pub fn interior_contains(x: Interval, y: Interval) -> bool {
    empty(y) || (x.lo < y.lo && y.hi < x.hi)
}

/// Whether the sets `x` and `y` have a common number: both non-empty, and neither lies wholly
/// beyond the other (`Intersects`, `r1interval.h:128`).
#[must_use]
pub fn intersects(x: Interval, y: Interval) -> bool {
    !empty(x) && !empty(y) && x.lo <= y.hi && y.lo <= x.hi
}

/// Whether the interior of `x` has a number in common with the set `y`
/// (`InteriorIntersects`, `r1interval.h:138`): the open set `(x.lo, x.hi)` is non-empty, `y` is
/// non-empty, and they overlap.
#[must_use]
pub fn interior_intersects(x: Interval, y: Interval) -> bool {
    x.lo < x.hi && !empty(y) && x.lo < y.hi && y.lo < x.hi
}

#[cfg(test)]
#[path = "real_spec_tests.rs"]
mod real_spec_tests;
