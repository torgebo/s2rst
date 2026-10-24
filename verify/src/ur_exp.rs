// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle expansion, projection and approximate equality (P-QEXP, spec 006 data-model
//! §4.1). Expansion is stated through the factors' `r1::Interval::expanded`, whose own meaning
//! is spec 005's P-REXP-01: the rectangle's statement is the product of proven intervals.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::r2::{Point, Rect};

use crate::value_domain::{any_finite_uv_rect, any_number, any_uv_point, any_uv_rect};
use crate::{real_spec, uv_spec};

fn bits(r: Rect) -> [u64; 4] {
    [r.x.lo, r.x.hi, r.y.lo, r.y.hi].map(f64::to_bits)
}

/// The finite rectangle and finite margins of P-QEXP-01, and the expansion.
fn expansion() -> (Rect, f64, f64, Rect) {
    let r = any_finite_uv_rect();
    let (mx, my) = (any_number(), any_number());
    kani::assume(mx.is_finite() && my.is_finite());
    (r, mx, my, r.expanded(Point::new(mx, my)))
}

/// P-QEXP-01 (correct result), the result: expanding by finite margins keeps a valid rectangle;
/// if either axis empties, the result is the empty rectangle, otherwise the two expanded axes;
/// expanding by one margin is expanding by it on both axes. The axes are expanded by
/// `r1::Interval::expanded`, whose meaning is spec 005's P-REXP-01.
///
/// For every `r` in `FiniteUvRect` and finite `m`, with `e = r.expanded(m)`,
/// `x = r.x.expanded(m.x)` and `y = r.y.expanded(m.y)`: `uv_spec::valid(e)`; if `x` or `y` is
/// empty, `e` is `Rect::empty()`, else `(x, y)`, bit for bit; and `r.expanded_by_margin(m.x)` is
/// `r.expanded((m.x, m.x))`, bit for bit.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_qexp_01__result() {
    let (r, mx, my, e) = expansion();
    assert!(uv_spec::valid(e));
    let (x, y) = (r.x.expanded(mx), r.y.expanded(my));
    if real_spec::empty(x) || real_spec::empty(y) {
        assert_eq!(bits(e), bits(Rect::empty()));
    } else {
        assert_eq!(bits(e), bits(Rect { x, y }));
    }
    assert_eq!(
        bits(r.expanded_by_margin(mx)),
        bits(r.expanded(Point::new(mx, mx)))
    );
    kani::cover!(!r.is_empty() && e.is_empty(), "shrunk to empty");
    kani::cover!(mx > 0.0 && my < 0.0 && !e.is_empty(), "mixed margins");
}

/// P-QEXP-01 (correct result), growing: by non-negative margins the result contains the
/// original.
///
/// For every `r` in `FiniteUvRect` and finite `m ≥ 0`: `uv_spec::contains(r.expanded(m), r)`.
#[kani::proof]
fn p_qexp_01__mpos() {
    let (r, mx, my, e) = expansion();
    kani::assume(mx >= 0.0 && my >= 0.0);
    assert!(uv_spec::contains(e, r));
}

/// P-QEXP-01 (correct result), shrinking: by non-positive margins the result is contained in
/// the original. A zero margin leaves an axis unchanged: `r1::Interval::expanded` has no
/// rounding allowance, unlike `s1::Interval::expanded`.
///
/// For every `r` in `FiniteUvRect` and finite `m ≤ 0`: `uv_spec::contains(r, r.expanded(m))`.
#[kani::proof]
fn p_qexp_01__mneg() {
    let (r, mx, my, e) = expansion();
    kani::assume(mx <= 0.0 && my <= 0.0);
    assert!(uv_spec::contains(r, e));
}

/// P-QEXP-02 (correct result): projection gives a point of a non-empty rectangle: the point
/// itself if inside, otherwise the nearest bound on each axis outside.
///
/// For every non-empty `r` in `UvRect` and `p` in `UvPoint`, with `q = r.project(p)`:
/// `uv_spec::member(r, q)`; and on each axis, `q` is `p`'s coordinate if it is within the
/// interval, the low bound if it is below and the high bound if it is above, bit for bit.
#[kani::proof]
fn p_qexp_02() {
    let r = any_uv_rect();
    kani::assume(!uv_spec::empty(r));
    let p = any_uv_point();
    let q = r.project(p);
    assert!(uv_spec::member(r, q));
    let nearest = |lo: f64, hi: f64, v: f64| {
        if v < lo {
            lo
        } else if v > hi {
            hi
        } else {
            v
        }
    };
    assert_eq!(q.x.to_bits(), nearest(r.x.lo, r.x.hi, p.x).to_bits());
    assert_eq!(q.y.to_bits(), nearest(r.y.lo, r.y.hi, p.y).to_bits());
    kani::cover!(uv_spec::member(r, p), "inside: the point itself");
    kani::cover!(p.x < r.x.lo && p.y > r.y.hi, "outside on both axes");
}

/// The finite rectangles and finite non-negative error bound of P-QEXP-03.
fn approx_inputs() -> (Rect, Rect, f64) {
    let (a, b) = (any_finite_uv_rect(), any_finite_uv_rect());
    let e = any_number();
    kani::assume(e.is_finite() && e >= 0.0);
    (a, b, e)
}

/// P-QEXP-03 (law), reflexivity: every finite rectangle is approximately equal to itself.
///
/// For every `a` in `FiniteUvRect` and finite `e ≥ 0`: `a.approx_eq_with(a, e)`.
#[kani::proof]
fn p_qexp_03__reflexive() {
    let (a, _, e) = approx_inputs();
    assert!(a.approx_eq_with(a, e));
}

/// P-QEXP-03 (law), symmetry: approximate equality is symmetric.
///
/// For every `a`, `b` in `FiniteUvRect` and finite `e ≥ 0`:
/// `a.approx_eq_with(b, e) == b.approx_eq_with(a, e)`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_qexp_03__symmetric() {
    let (a, b, e) = approx_inputs();
    let w = a.approx_eq_with(b, e);
    assert_eq!(w, b.approx_eq_with(a, e));
    kani::cover!(w && a != b, "approximately but not exactly equal");
}

/// P-QEXP-03 (law), the product: approximate equality is the two intervals' (spec 005's
/// P-REXP-03), and the default uses 1e-15.
///
/// For every `a`, `b` in `FiniteUvRect` and finite `e ≥ 0`:
/// `a.approx_eq_with(b, e) == (a.x.approx_eq_with(b.x, e) && a.y.approx_eq_with(b.y, e))` and
/// `a.approx_eq(b) == a.approx_eq_with(b, 1e-15)`.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_qexp_03__product() {
    let (a, b, e) = approx_inputs();
    let w = a.approx_eq_with(b, e);
    assert_eq!(w, a.x.approx_eq_with(b.x, e) && a.y.approx_eq_with(b.y, e));
    assert_eq!(a.approx_eq(b), a.approx_eq_with(b, 1e-15));
    kani::cover!(w && a.is_empty() != b.is_empty(), "empty and tiny");
}
