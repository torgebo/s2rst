// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle set operations (P-QSET, spec 006 data-model §4.1): the union, the
//! intersection and adding a point, against the product sets (`uv_spec`).

use s2rst::r2::Rect;

use crate::uv_spec;
use crate::value_domain::{any_uv_point, any_uv_rect};

fn bits(r: Rect) -> [u64; 4] {
    [r.x.lo, r.x.hi, r.y.lo, r.y.hi].map(f64::to_bits)
}

/// P-QSET-01 (correct result): the union is the least rectangle containing both, and adding a
/// rectangle is the union.
///
/// For every `a`, `b`, `c` in `UvRect`, with `u = a.union(b)`: `uv_spec::contains(u, a)` and
/// `uv_spec::contains(u, b)`; if `c` contains both, it contains `u`; and `a.add_rect(b)` is
/// `u`, bit for bit, unless both are empty, when both are empty. (Upstream's `Union` returns
/// the other operand when this one is empty, and `AddRect` keeps this one when the other is, so
/// two empty rectangles give their two representations of the empty set.)
#[kani::proof]
fn p_qset_01() {
    let (a, b, c) = (any_uv_rect(), any_uv_rect(), any_uv_rect());
    let u = a.union(b);
    assert!(uv_spec::contains(u, a) && uv_spec::contains(u, b));
    if uv_spec::contains(c, a) && uv_spec::contains(c, b) {
        assert!(uv_spec::contains(c, u));
    }
    let ar = a.add_rect(b);
    if uv_spec::empty(a) && uv_spec::empty(b) {
        assert!(uv_spec::empty(ar) && uv_spec::empty(u));
    } else {
        assert_eq!(bits(ar), bits(u));
    }
    kani::cover!(
        !uv_spec::contains(a, b) && !uv_spec::contains(b, a),
        "neither contains the other"
    );
}

/// P-QSET-02 (correct result): the intersection holds exactly the common points, and is empty
/// exactly when the two do not intersect.
///
/// For every `a`, `b` in `UvRect` and `p` in `UvPoint`, with `i = a.intersection(b)`:
/// `uv_spec::member(i, p) == (uv_spec::member(a, p) && uv_spec::member(b, p))` and
/// `uv_spec::empty(i) == !uv_spec::intersects(a, b)`.
#[kani::proof]
fn p_qset_02() {
    let (a, b, p) = (any_uv_rect(), any_uv_rect(), any_uv_point());
    let i = a.intersection(b);
    assert_eq!(
        uv_spec::member(i, p),
        uv_spec::member(a, p) && uv_spec::member(b, p)
    );
    assert_eq!(uv_spec::empty(i), !uv_spec::intersects(a, b));
    kani::cover!(uv_spec::member(i, p), "a common point");
    kani::cover!(
        !a.is_empty() && !b.is_empty() && i.is_empty(),
        "disjoint on one axis only"
    );
}

/// P-QSET-03 (correct result): adding a point gives the least rectangle containing the original
/// and the point.
///
/// For every `r`, `c` in `UvRect` and `p` in `UvPoint`, with `s = r.add_point(p)`:
/// `uv_spec::member(s, p)` and `uv_spec::contains(s, r)`; if `c` contains `r` and `p`, it
/// contains `s`.
#[kani::proof]
fn p_qset_03() {
    let (r, c, p) = (any_uv_rect(), any_uv_rect(), any_uv_point());
    let s = r.add_point(p);
    assert!(uv_spec::member(s, p) && uv_spec::contains(s, r));
    if uv_spec::contains(c, r) && uv_spec::member(c, p) {
        assert!(uv_spec::contains(c, s));
    }
    kani::cover!(
        !uv_spec::member(r, p) && !r.is_empty(),
        "the rectangle grows"
    );
}
