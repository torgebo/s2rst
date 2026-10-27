// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-r2::Rect::<op>` for the 32 in-scope operations of the u-v
//! rectangle (spec 006 US1, FR-004; data-model §5), grouped as spec 005's `ri_crash`.
//!
//! Each harness draws its inputs from the operation's domain in `values/INVENTORY.md` §5:
//! - every valid rectangle (`UvRect`: both axes empty or both not, no NaN bound, infinities,
//!   signed zeros and the empty rectangle's every representation included) and every non-NaN
//!   point (`UvPoint`), for the operations that compare and select bounds;
//! - four arbitrary bit patterns, NaN included, for `is_valid`, and every `i32` for `vertex`;
//! - finite bounds and finite margins for the arithmetic operations (`FiniteUvRect`), where
//!   `∞ − ∞` cannot arise (FR-004, FR-018);
//! - a non-empty rectangle for `project` (FR-017);
//! - for the constructors that check validity in debug builds, exactly the inputs their
//!   documentation admits, so that the proof shows the check never fires there.
//!
//! Kani builds with debug assertions on, so `new`'s, `from_points`'s and `from_center_size`'s
//! checks and `f64::clamp`'s assertion are obligations. The covers show the domains non-empty
//! and the outcomes FR-009 names reachable: empty, single-point and infinite sides.

use s2rst::r1::{self, Endpoint};
use s2rst::r2::{Point, Rect};

use crate::real_spec;
use crate::value_domain::{
    any_finite_uv_rect, any_number, any_real, any_uv_bounds, any_uv_point, any_uv_rect,
};

/// Whether an interval is a single point.
fn single(x: r1::Interval) -> bool {
    x.lo == x.hi
}

/// C-r2::Rect::new, from_points, empty, from_point, from_point_pair and Default: complete
/// without panic, arithmetic overflow or undefined behaviour, and the debug checks of `new`
/// and `from_points` never fire, for two intervals in `Real` that are both empty or both not,
/// for two corners whose axes are both ordered or both reversed, and for every pair of
/// `UvPoint`s.
#[kani::proof]
fn c_constructors() {
    let (x, y) = (any_real(), any_real());
    kani::assume(real_spec::empty(x) == real_spec::empty(y));
    let n = Rect::new(x, y);
    let (lo, hi) = (any_uv_point(), any_uv_point());
    kani::assume((lo.x > hi.x) == (lo.y > hi.y));
    let f = Rect::from_points(lo, hi);
    let (single_point, pair) = (Rect::from_point(lo), Rect::from_point_pair(lo, hi));
    let (e, d) = (Rect::empty(), Rect::default());
    kani::cover!(real_spec::empty(n.x), "an empty rectangle through new");
    kani::cover!(real_spec::empty(f.x), "reversed corners: empty");
    kani::cover!(
        lo.x > hi.x && pair.x.lo == hi.x,
        "from_point_pair orders the points"
    );
    kani::cover!(single_point.x.lo.is_infinite(), "an infinite single point");
    kani::cover!(e.x.lo == d.x.lo && e.y.hi == d.y.hi, "the empty default");
}

/// C-r2::Rect::from_center_size: completes without panic, arithmetic overflow or undefined
/// behaviour, and its debug check (FR-017) never fires, for every centre in `UvPoint` and every
/// finite non-negative size: a non-negative size never empties an axis.
#[kani::proof]
fn c_from_center_size() {
    let c = any_uv_point();
    let (sx, sy) = (any_number(), any_number());
    kani::assume(sx.is_finite() && sy.is_finite() && sx >= 0.0 && sy >= 0.0);
    let r = Rect::from_center_size(c, Point::new(sx, sy));
    kani::cover!(single(r.x) && single(r.y), "a zero size: a single point");
    kani::cover!(r.x.lo.is_infinite(), "an infinite centre");
    kani::cover!(
        r.y.lo < r.y.hi && r.y.hi.is_finite(),
        "a finite side of positive length"
    );
}

/// C-r2::Rect::is_valid: completes without panic, arithmetic overflow or undefined behaviour
/// for every four bounds, NaN included.
#[kani::proof]
fn c_is_valid() {
    let r = any_uv_bounds();
    let v = r.is_valid();
    kani::cover!(!v, "one axis empty, the other not");
    kani::cover!(v && r.x.lo.is_nan(), "a NaN bound passes (K4)");
}

/// C-r2::Rect::lo, hi, is_empty, vertex, vertex_ij and vertices: complete without panic,
/// arithmetic overflow or undefined behaviour for every rectangle in `UvRect`, every `i32` index
/// and both endpoints on each axis.
#[kani::proof]
fn c_accessors() {
    let r = any_uv_rect();
    let k: i32 = kani::any();
    let (i, j): (bool, bool) = (kani::any(), kani::any());
    let end = |b: bool| if b { Endpoint::Hi } else { Endpoint::Lo };
    let (lo, hi, empty) = (r.lo(), r.hi(), r.is_empty());
    let (v, vij, all) = (r.vertex(k), r.vertex_ij(end(i), end(j)), r.vertices());
    kani::cover!(empty, "empty");
    kani::cover!(k < 0 && v.x == all[3].x, "a negative index");
    kani::cover!(lo.x.is_infinite() && hi.y.is_infinite(), "infinite corners");
    kani::cover!(vij.x == hi.x && vij.y == lo.y, "the lower right corner");
}

/// C-r2::Rect::contains_point, interior_contains_point and add_point: complete without panic,
/// arithmetic overflow or undefined behaviour for every rectangle in `UvRect` and every point
/// in `UvPoint`.
#[kani::proof]
fn c_point_operations() {
    let r = any_uv_rect();
    let p = any_uv_point();
    let (c, ic) = (r.contains_point(p), r.interior_contains_point(p));
    let a = r.add_point(p);
    kani::cover!(c && !ic, "on the boundary");
    kani::cover!(
        r.is_empty() && single(a.x) && single(a.y),
        "the empty rectangle becomes a point"
    );
    kani::cover!(!c && !r.is_empty(), "outside");
}

/// C-r2::Rect::contains, interior_contains, intersects, interior_intersects, add_rect, union,
/// intersection and eq: complete without panic, arithmetic overflow or undefined behaviour for
/// every pair of rectangles in `UvRect`.
#[kani::proof]
fn c_relations_and_set_operations() {
    let (a, b) = (any_uv_rect(), any_uv_rect());
    let rel = (
        a.contains(b),
        a.interior_contains(b),
        a.intersects(b),
        a.interior_intersects(b),
    );
    let (ar, u, i) = (a.add_rect(b), a.union(b), a.intersection(b));
    let eq = a == b;
    kani::cover!(rel.0 && !rel.1, "contained, touching the boundary");
    kani::cover!(rel.2 && !rel.3, "intersecting only on the boundary");
    kani::cover!(!a.is_empty() && !b.is_empty() && i.is_empty(), "disjoint");
    kani::cover!(
        u.x.lo == b.x.lo && ar.x.hi == a.x.hi && u.x.lo != a.x.lo,
        "the union takes both sides"
    );
    kani::cover!(eq && a.x.lo != b.x.lo, "two empty rectangles are equal");
}

/// C-r2::Rect::center and size: complete without panic, arithmetic overflow or undefined
/// behaviour for every rectangle in `FiniteUvRect`.
#[kani::proof]
fn c_measures() {
    let r = any_finite_uv_rect();
    let (c, s) = (r.center(), r.size());
    kani::cover!(s.x < 0.0, "an empty rectangle's negative size");
    kani::cover!(s.x.is_infinite(), "a size that overflows");
    kani::cover!(
        r.x.lo > f64::MAX / 2.0 && c.x.is_finite(),
        "a centre beyond f64::MAX / 2 (U-010)"
    );
}

/// C-r2::Rect::expanded and expanded_by_margin: complete without panic, arithmetic overflow or
/// undefined behaviour for every rectangle in `FiniteUvRect` and every finite margin.
#[kani::proof]
fn c_expanded() {
    let r = any_finite_uv_rect();
    let (mx, my) = (any_number(), any_number());
    kani::assume(mx.is_finite() && my.is_finite());
    let (e, e1) = (r.expanded(Point::new(mx, my)), r.expanded_by_margin(mx));
    kani::cover!(
        !r.is_empty() && e.is_empty(),
        "shrinking empties the rectangle"
    );
    kani::cover!(e.x.hi.is_infinite(), "growing overflows a bound");
    kani::cover!(!e1.is_empty() && mx < 0.0, "shrinking keeps a rectangle");
}

/// C-r2::Rect::project: completes without panic, arithmetic overflow or undefined behaviour,
/// and neither its debug check nor `f64::clamp`'s assertion fires, for every non-empty
/// rectangle in `UvRect` and every point in `UvPoint` (FR-017: the empty rectangle panics).
#[kani::proof]
fn c_project() {
    let r = any_uv_rect();
    kani::assume(!real_spec::empty(r.x));
    let p = any_uv_point();
    let q = r.project(p);
    kani::cover!(q.x == p.x && q.y == p.y, "the point itself");
    kani::cover!(q.x == r.x.lo && p.x < r.x.lo, "the low bound");
    kani::cover!(q.y == r.y.hi && p.y > r.y.hi, "the high bound");
}

/// C-r2::Rect::approx_eq_with and approx_eq: complete without panic, arithmetic overflow or
/// undefined behaviour for every pair of rectangles in `FiniteUvRect` and every finite
/// non-negative error bound.
#[kani::proof]
fn c_approx_eq() {
    let (a, b) = (any_finite_uv_rect(), any_finite_uv_rect());
    let e = any_number();
    kani::assume(e.is_finite() && e >= 0.0);
    let (w, d) = (a.approx_eq_with(b, e), a.approx_eq(b));
    kani::cover!(w && a != b, "approximately but not exactly equal");
    kani::cover!(d && a.is_empty() && !b.is_empty(), "empty and tiny");
    kani::cover!(!w, "not approximately equal");
}

// ---------------------------------------------------------------------------------------------
// Outside the domain: the documented panics (gap 1; `crash.rs` explains the `n_*` and `r_*`
// conventions).
// ---------------------------------------------------------------------------------------------

/// Two arbitrary `f64` coordinates, NaN and the infinities included.
fn any_xy() -> Point {
    Point {
        x: kani::any(),
        y: kani::any(),
    }
}

/// Whether an interval is empty (`lo > hi`; false for a NaN bound, as `r1::Interval::is_empty`).
fn empty(x: r1::Interval) -> bool {
    x.lo > x.hi
}

/// N-r2::Rect::new, N-from_points: in debug builds, each panics for every pair of intervals
/// (corners) of which exactly one axis is empty, NaN bounds included.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_constructors() {
    let b = any_uv_bounds();
    kani::assume(empty(b.x) != empty(b.y));
    if kani::any::<bool>() {
        let _ = Rect::new(b.x, b.y);
    } else {
        let _ = Rect::from_points(b.lo(), b.hi());
    }
    kani::cover!(true, "returns");
}

/// R-r2::Rect::new, R-from_points: in a build without debug assertions, each completes without
/// panic or overflow on the same inputs, and returns the invalid rectangle.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_constructors() {
    let b = any_uv_bounds();
    kani::assume(empty(b.x) != empty(b.y));
    let (n, p) = (Rect::new(b.x, b.y), Rect::from_points(b.lo(), b.hi()));
    assert!(!n.is_valid() && !p.is_valid());
    kani::cover!(empty(b.x), "the x axis empty");
}

/// A finite centre and size of which exactly one axis comes out empty, `c − s/2 > c + s/2` on
/// it, the result `from_center_size`'s `# Panics` names. Finite, because an infinite centre and
/// size give `∞ − ∞`, which Kani reports as a failure (NaN) before the check. The doc's first
/// version said "one size component negative and the other not": over those inputs this
/// harness returned (2026-10-26), for a negative size that rounding absorbs (`-5e-324`).
fn outside_center_size() -> (Point, Point) {
    let (c, s) = (any_xy(), any_xy());
    kani::assume(c.x.is_finite() && c.y.is_finite() && s.x.is_finite() && s.y.is_finite());
    let axis_empty = |c: f64, s: f64| c - 0.5 * s > c + 0.5 * s;
    kani::assume(axis_empty(c.x, s.x) != axis_empty(c.y, s.y));
    (c, s)
}

/// N-r2::Rect::from_center_size: in debug builds, panics for every finite centre and size of
/// which exactly one axis comes out empty.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_from_center_size() {
    let (c, s) = outside_center_size();
    let _ = Rect::from_center_size(c, s);
    kani::cover!(true, "returns");
}

/// R-r2::Rect::from_center_size: in a build without debug assertions, completes without panic or
/// overflow on the same inputs, and returns the invalid rectangle.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_from_center_size() {
    let (c, s) = outside_center_size();
    let r = Rect::from_center_size(c, s);
    assert!(!r.is_valid());
    kani::cover!(s.x < 0.0, "a negative x size");
}

/// N-r2::Rect::project: panics, in every build, for every rectangle with an empty axis or a NaN
/// bound and every point (`r1::Interval::project` on each axis).
#[kani::proof]
#[kani::should_panic]
fn n_project() {
    let b = any_uv_bounds();
    let nan = |x: r1::Interval| x.lo.is_nan() || x.hi.is_nan();
    kani::assume(empty(b.x) || empty(b.y) || nan(b.x) || nan(b.y));
    let _ = b.project(any_xy());
    kani::cover!(true, "returns");
}
