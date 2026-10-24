// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a u-v rectangle (spec 006 data-model §1.1): the product of two
//! real sets, one per axis.
//!
//! Written from upstream's `R2Rect` header (`r2rect.h`, `v0.14.0-40`). A rectangle is the
//! product `x × y` of two closed real intervals ([`real_spec`], spec 005). It is valid when its
//! two intervals are both empty or both non-empty (`r2rect.h:201`), and empty exactly when its
//! x-interval is (`r2rect.h:206`). Every relation between rectangles is that of the product
//! sets; for valid rectangles it is the conjunction of the two axes' relations, as upstream
//! defines it (`r2rect.cc:32–46`), because a valid rectangle is empty exactly when both axes
//! are.
//!
//! The description reads a [`Rect`]'s two public fields and never calls its methods or
//! `r1::Interval`'s (FR-007). Booleans are combined with `&&`, `||` and `==` only (spec 004
//! research R13).

use s2rst::r2::{Point, Rect};

use crate::real_spec;

/// Upstream's emptiness rule alone: the two intervals are both empty or both non-empty
/// (`is_valid`, `r2rect.h:201`). A NaN bound passes it, because every comparison with NaN is
/// false, so NaN never makes an interval empty (spec K4).
#[must_use]
pub fn emptiness_rule(r: Rect) -> bool {
    real_spec::empty(r.x) == real_spec::empty(r.y)
}

/// Whether `r` is a valid rectangle (spec 006 FR-005): upstream's emptiness rule, and no bound
/// NaN, since a bound is a number (`r1interval.h`; spec 006 K4).
#[must_use]
pub fn valid(r: Rect) -> bool {
    emptiness_rule(r)
        && !r.x.lo.is_nan()
        && !r.x.hi.is_nan()
        && !r.y.lo.is_nan()
        && !r.y.hi.is_nan()
}

/// Whether all four bounds are finite: the domain of the arithmetic properties (spec 006
/// FR-004, FR-018), where `∞ − ∞` cannot arise.
#[must_use]
pub fn finite(r: Rect) -> bool {
    real_spec::finite(r.x) && real_spec::finite(r.y)
}

/// Whether `r` is empty: no x, so (for a valid rectangle) no y either (`is_empty`,
/// `r2rect.h:206`).
#[must_use]
pub fn empty(r: Rect) -> bool {
    real_spec::empty(r.x)
}

/// Whether the point `p` is in the closed set `r` (`Contains(const R2Point&)`, `r2rect.h:233`).
#[must_use]
pub fn member(r: Rect, p: Point) -> bool {
    real_spec::member(r.x, p.x) && real_spec::member(r.y, p.y)
}

/// Whether `p` is in the interior of `r`, the product of the two open intervals
/// (`InteriorContains(const R2Point&)`, `r2rect.h:237`).
#[must_use]
pub fn interior(r: Rect, p: Point) -> bool {
    real_spec::interior(r.x, p.x) && real_spec::interior(r.y, p.y)
}

/// Whether the set `a` contains the set `b`: `b` is empty, or each axis of `a` contains `b`'s
/// (`Contains(const R2Rect&)`, `r2rect.cc:32`).
#[must_use]
pub fn contains(a: Rect, b: Rect) -> bool {
    empty(b) || (real_spec::contains(a.x, b.x) && real_spec::contains(a.y, b.y))
}

/// Whether the interior of `a` contains the set `b` (`InteriorContains(const R2Rect&)`,
/// `r2rect.cc:36`).
#[must_use]
pub fn interior_contains(a: Rect, b: Rect) -> bool {
    empty(b) || (real_spec::interior_contains(a.x, b.x) && real_spec::interior_contains(a.y, b.y))
}

/// Whether `a` and `b` have a common point: both axes intersect (`Intersects`,
/// `r2rect.cc:40`).
#[must_use]
pub fn intersects(a: Rect, b: Rect) -> bool {
    real_spec::intersects(a.x, b.x) && real_spec::intersects(a.y, b.y)
}

/// Whether the interior of `a` has a point in common with `b`: both axes' interiors intersect
/// (`InteriorIntersects`, `r2rect.cc:44`).
#[must_use]
pub fn interior_intersects(a: Rect, b: Rect) -> bool {
    real_spec::interior_intersects(a.x, b.x) && real_spec::interior_intersects(a.y, b.y)
}

/// The `k`-th vertex, `k` taken modulo 4, in upstream's counter-clockwise order: lower left
/// `(x.lo, y.lo)`, lower right `(x.hi, y.lo)`, upper right `(x.hi, y.hi)`, upper left
/// `(x.lo, y.hi)` (`GetVertex(int)`, `r2rect.h:90–93`).
pub fn vertex(r: Rect, k: i32) -> Point {
    match k.rem_euclid(4) {
        0 => Point {
            x: r.x.lo,
            y: r.y.lo,
        },
        1 => Point {
            x: r.x.hi,
            y: r.y.lo,
        },
        2 => Point {
            x: r.x.hi,
            y: r.y.hi,
        },
        _ => Point {
            x: r.x.lo,
            y: r.y.hi,
        },
    }
}

#[cfg(test)]
#[path = "uv_spec_tests.rs"]
mod uv_spec_tests;
