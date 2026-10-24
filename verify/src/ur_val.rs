// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle representation (P-QVAL, spec 006 data-model §4.1, §3): validity, the
//! constructors and the corners, against the product description (`uv_spec`).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::r1::{self, Endpoint};
use s2rst::r2::{Point, Rect};

use crate::value_domain::{
    any_finite_uv_rect, any_number, any_real, any_uv_bounds, any_uv_point, any_uv_rect,
};
use crate::{real_spec, uv_spec};

fn bits(r: Rect) -> [u64; 4] {
    [r.x.lo, r.x.hi, r.y.lo, r.y.hi].map(f64::to_bits)
}

fn point_bits(p: Point) -> [u64; 2] {
    [p.x.to_bits(), p.y.to_bits()]
}

fn interval(lo: f64, hi: f64) -> r1::Interval {
    r1::Interval { lo, hi }
}

/// P-QVAL-01 (correct result): the validity check is upstream's emptiness rule for every bit
/// pattern, NaN included, so it agrees with the description wherever no bound is NaN; a NaN
/// bound passes it (K4, documented by FR-018; FR-019's deserialiser rejects it).
///
/// For every four `f64` bounds: `r.is_valid() == uv_spec::emptiness_rule(r)`, and if no bound
/// is NaN, `r.is_valid() == uv_spec::valid(r)`.
#[kani::proof]
fn p_qval_01() {
    let r = any_uv_bounds();
    let v = r.is_valid();
    assert_eq!(v, uv_spec::emptiness_rule(r));
    let nan = r.x.lo.is_nan() || r.x.hi.is_nan() || r.y.lo.is_nan() || r.y.hi.is_nan();
    if !nan {
        assert_eq!(v, uv_spec::valid(r));
    }
    kani::cover!(!v, "one axis empty, the other not");
    kani::cover!(v && nan, "a NaN bound passes the check");
}

/// P-QVAL-02 (validity), the constructors: each returns a valid rectangle on its documented
/// domain.
///
/// For intervals `x`, `y` in `Real` that are both empty or both not, corners in `UvPoint` whose
/// axes are both ordered or both reversed, points `p`, `q` in `UvPoint`, and a finite
/// non-negative size `s`: `new(x, y)`, `from_points(lo, hi)`, `empty()`, `from_point(p)`,
/// `from_point_pair(p, q)`, `from_center_size(p, s)` and `default()` are all `uv_spec::valid`.
#[kani::proof]
fn p_qval_02__constructors() {
    let (x, y) = (any_real(), any_real());
    kani::assume(real_spec::empty(x) == real_spec::empty(y));
    assert!(uv_spec::valid(Rect::new(x, y)));
    let (lo, hi) = (any_uv_point(), any_uv_point());
    kani::assume((lo.x > hi.x) == (lo.y > hi.y));
    assert!(uv_spec::valid(Rect::from_points(lo, hi)));
    assert!(uv_spec::valid(Rect::empty()));
    assert!(uv_spec::valid(Rect::default()));
    assert!(uv_spec::valid(Rect::from_point(lo)));
    assert!(uv_spec::valid(Rect::from_point_pair(lo, hi)));
    let (sx, sy) = (any_number(), any_number());
    kani::assume(sx.is_finite() && sy.is_finite() && sx >= 0.0 && sy >= 0.0);
    assert!(uv_spec::valid(Rect::from_center_size(
        lo,
        Point::new(sx, sy)
    )));
}

/// P-QVAL-02 (validity), the set operations: adding a point or a rectangle, the union and the
/// intersection of valid rectangles are valid.
///
/// For every `a`, `b` in `UvRect` and `p` in `UvPoint`: `a.add_point(p)`, `a.add_rect(b)`,
/// `a.union(b)` and `a.intersection(b)` are all `uv_spec::valid`.
#[kani::proof]
fn p_qval_02__set_operations() {
    let (a, b) = (any_uv_rect(), any_uv_rect());
    let p = any_uv_point();
    assert!(uv_spec::valid(a.add_point(p)));
    assert!(uv_spec::valid(a.add_rect(b)));
    assert!(uv_spec::valid(a.union(b)));
    let i = a.intersection(b);
    assert!(uv_spec::valid(i));
    kani::cover!(!a.is_empty() && !b.is_empty() && i.is_empty(), "disjoint");
}

/// P-QVAL-02 (validity), expansion: expanding a finite rectangle by finite margins gives a
/// valid one.
///
/// For every `r` in `FiniteUvRect` and finite `m`: `r.expanded(m)` and
/// `r.expanded_by_margin(m.x)` are `uv_spec::valid`.
#[kani::proof]
fn p_qval_02__expanded() {
    let r = any_finite_uv_rect();
    let (mx, my) = (any_number(), any_number());
    kani::assume(mx.is_finite() && my.is_finite());
    let e = r.expanded(Point::new(mx, my));
    assert!(uv_spec::valid(e));
    assert!(uv_spec::valid(r.expanded_by_margin(mx)));
    kani::cover!(!r.is_empty() && e.is_empty(), "shrunk to empty");
}

/// P-QVAL-03 (correct result), emptiness: a rectangle is empty exactly when its x-interval is,
/// and the empty and default rectangles are `[1, 0]²`.
///
/// For every `r` in `UvRect`: `r.is_empty() == uv_spec::empty(r)`; `empty()` and `default()`
/// are `[1, 0]²`, bit for bit.
#[kani::proof]
fn p_qval_03__empty() {
    let r = any_uv_rect();
    assert_eq!(r.is_empty(), uv_spec::empty(r));
    let empty = Rect {
        x: interval(1.0, 0.0),
        y: interval(1.0, 0.0),
    };
    assert_eq!(bits(Rect::empty()), bits(empty));
    assert_eq!(bits(Rect::default()), bits(empty));
    kani::cover!(
        r.is_empty() && r.x.lo != 1.0,
        "another representation of empty"
    );
}

/// P-QVAL-03 (correct result), from intervals: `new` keeps the two intervals as given.
///
/// For intervals `x`, `y` in `Real` both empty or both not: `new(x, y)` is `(x, y)`, bit for
/// bit.
#[kani::proof]
fn p_qval_03__new() {
    let (x, y) = (any_real(), any_real());
    kani::assume(real_spec::empty(x) == real_spec::empty(y));
    assert_eq!(bits(Rect::new(x, y)), bits(Rect { x, y }));
}

/// P-QVAL-03 (correct result), from points: `from_points` takes each axis' bounds from the two
/// corners, `from_point` is the single point, and `from_point_pair` orders each axis' pair.
///
/// For corners `lo`, `hi` in `UvPoint` whose axes are both ordered or both reversed, bit for
/// bit: `from_points(lo, hi)` is `[lo.x, hi.x] × [lo.y, hi.y]`; `from_point(lo)` is
/// `[lo.x, lo.x] × [lo.y, lo.y]`; `from_point_pair(lo, hi)` is the smaller then the larger
/// coordinate on each axis (`r2rect.h:175–212`).
#[kani::proof]
fn p_qval_03__points() {
    let (lo, hi) = (any_uv_point(), any_uv_point());
    kani::assume((lo.x > hi.x) == (lo.y > hi.y));
    let corners = Rect {
        x: interval(lo.x, hi.x),
        y: interval(lo.y, hi.y),
    };
    assert_eq!(bits(Rect::from_points(lo, hi)), bits(corners));
    let single = Rect {
        x: interval(lo.x, lo.x),
        y: interval(lo.y, lo.y),
    };
    assert_eq!(bits(Rect::from_point(lo)), bits(single));
    let order = |a: f64, b: f64| {
        if a <= b {
            interval(a, b)
        } else {
            interval(b, a)
        }
    };
    let pair = Rect {
        x: order(lo.x, hi.x),
        y: order(lo.y, hi.y),
    };
    assert_eq!(bits(Rect::from_point_pair(lo, hi)), bits(pair));
    kani::cover!(lo.x > hi.x, "reversed corners: the empty rectangle");
}

/// P-QVAL-03 (correct result), from a centre and a size: centre ± half the size on each axis
/// (`r2rect.cc:25`).
///
/// For a centre `c` in `UvPoint` and a finite size `s ≥ 0`: `from_center_size(c, s)` is
/// `[c.x − 0.5·s.x, c.x + 0.5·s.x] × [c.y − 0.5·s.y, c.y + 0.5·s.y]`, bit for bit.
///
/// Weekly: it timed out at 12 minutes per change (2026-10-14).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_qval_03__center_size() {
    let c = any_uv_point();
    let (sx, sy) = (any_number(), any_number());
    kani::assume(sx.is_finite() && sy.is_finite() && sx >= 0.0 && sy >= 0.0);
    let centred = Rect {
        x: interval(c.x - 0.5 * sx, c.x + 0.5 * sx),
        y: interval(c.y - 0.5 * sy, c.y + 0.5 * sy),
    };
    assert_eq!(
        bits(Rect::from_center_size(c, Point::new(sx, sy))),
        bits(centred)
    );
}

/// P-QVAL-04 (correct result): the corners and the vertices are the description's, for every
/// `i32` index.
///
/// For every `r` in `UvRect` and `k` in `i32`, bit for bit: `r.vertex(k)` is
/// `uv_spec::vertex(r, k)`; `r.vertices()[i]` is `uv_spec::vertex(r, i)`; `r.lo()` and `r.hi()`
/// are vertices 0 and 2; `r.vertex_ij(i, j)` is `(x.bound(i), y.bound(j))`, which is vertex 0,
/// 1, 2 or 3 for (Lo, Lo), (Hi, Lo), (Hi, Hi), (Lo, Hi).
#[kani::proof]
fn p_qval_04() {
    let r = any_uv_rect();
    let k: i32 = kani::any();
    assert_eq!(point_bits(r.vertex(k)), point_bits(uv_spec::vertex(r, k)));
    let all = r.vertices();
    for (i, v) in (0..4).zip(all) {
        assert_eq!(point_bits(v), point_bits(uv_spec::vertex(r, i)));
    }
    assert_eq!(point_bits(r.lo()), point_bits(uv_spec::vertex(r, 0)));
    assert_eq!(point_bits(r.hi()), point_bits(uv_spec::vertex(r, 2)));
    let (i, j): (bool, bool) = (kani::any(), kani::any());
    let end = |hi: bool| if hi { Endpoint::Hi } else { Endpoint::Lo };
    let want = match (i, j) {
        (false, false) => 0,
        (true, false) => 1,
        (true, true) => 2,
        (false, true) => 3,
    };
    assert_eq!(
        point_bits(r.vertex_ij(end(i), end(j))),
        point_bits(uv_spec::vertex(r, want))
    );
    kani::cover!(k < 0, "a negative index");
    kani::cover!(k > 3, "an index beyond 3");
}
