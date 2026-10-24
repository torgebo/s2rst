// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle set operations (P-LSET, spec 005 data-model §4.2): union, intersection, adding a
//! point and the polar closure, against the rectangle description (`rect_spec`).

use s2rst::s2::{LatLng, Rect};

use crate::arc_spec;
use crate::rect_spec;
use crate::value_domain::{any_f64, any_rect, any_valid_lat_lng};

fn bits(r: Rect) -> [u64; 4] {
    [r.lat.lo, r.lat.hi, r.lng.lo, r.lng.hi].map(f64::to_bits)
}

/// P-LSET-01 (correct result): the union contains both rectangles.
///
/// For every valid `a`, `b`: `rect_spec::contains(a.union(b), a)` and
/// `rect_spec::contains(a.union(b), b)`.
#[kani::proof]
fn p_lset_01() {
    let (a, b) = (any_rect(), any_rect());
    let u = a.union(b);
    assert!(rect_spec::contains(u, a) && rect_spec::contains(u, b));
    kani::cover!(
        !a.intersects(b) && !rect_spec::empty(a) && !rect_spec::empty(b),
        "disjoint"
    );
}

/// P-LSET-02 (correct result): the intersection holds every point the two have in common, and
/// is empty exactly when they do not intersect.
///
/// For every valid `a`, `b` and every two `f64` values `lat`, `lng`, with
/// `i = a.intersection(b)`: `member(a, lat, lng) ∧ member(b, lat, lng) ⇒ member(i, lat, lng)`;
/// `rect_spec::empty(i) == ¬rect_spec::intersects(a, b)`.
#[kani::proof]
fn p_lset_02() {
    let (a, b) = (any_rect(), any_rect());
    let (lat, lng) = (any_f64(), any_f64());
    let i = a.intersection(b);
    if rect_spec::member(a, lat, lng) && rect_spec::member(b, lat, lng) {
        assert!(rect_spec::member(i, lat, lng));
    }
    assert_eq!(rect_spec::empty(i), !rect_spec::intersects(a, b));
    kani::cover!(rect_spec::member(i, lat, lng), "a common point");
}

/// P-LSET-03 (correct result): adding a valid point gives a rectangle that contains the point
/// and the original; an invalid point (out of range, or NaN) changes nothing (K2). A
/// rectangle built from one or two valid points contains them.
///
/// For every valid `r` and every two `f64` values `lat`, `lng`, with
/// `a = r.add_point(from_radians(lat, lng))`: if `ll_valid(lat, lng)`,
/// `member(a, lat, lng) ∧ contains(a, r)`; otherwise `a` is `r`, bit for bit. For valid `p`,
/// `q`: `member(from_lat_lng(p), p)`, and `from_point_pair(p, q)` holds `p` and `q`.
#[kani::proof]
fn p_lset_03() {
    let r = any_rect();
    let (lat, lng) = (any_f64(), any_f64());
    let a = r.add_point(LatLng::from_radians(lat, lng));
    if rect_spec::ll_valid(lat, lng) {
        assert!(rect_spec::member(a, lat, lng) && rect_spec::contains(a, r));
    } else {
        assert_eq!(bits(a), bits(r));
    }
    let (p, q) = (any_valid_lat_lng(), any_valid_lat_lng());
    let member = |x: Rect, ll: LatLng| rect_spec::member(x, ll.lat.radians(), ll.lng.radians());
    assert!(member(Rect::from_lat_lng(p), p));
    let pair = Rect::from_point_pair(p, q);
    assert!(member(pair, p) && member(pair, q));
    kani::cover!(
        rect_spec::empty(r) && a.is_point(),
        "the empty rectangle becomes a point"
    );
    kani::cover!(
        !rect_spec::ll_valid(lat, lng) && lat.abs() < 1.0,
        "an out-of-range longitude"
    );
}

/// P-LSET-04 (correct result): the polar closure contains the original; a rectangle that
/// reaches a pole gets every longitude, with its latitudes unchanged, and any other rectangle
/// is unchanged.
///
/// For every valid `r`, with `p = r.polar_closure()`: `rect_spec::contains(p, r)`; if
/// `touches_pole(r)`, `arc_spec::full(p.lng)` and `p.lat` is `r.lat`, bit for bit; otherwise
/// `p` is `r`, bit for bit.
#[kani::proof]
fn p_lset_04() {
    let r = any_rect();
    let p = r.polar_closure();
    assert!(rect_spec::contains(p, r));
    if rect_spec::touches_pole(r) {
        assert!(arc_spec::full(p.lng));
        assert_eq!(bits(p)[..2], bits(r)[..2]);
    } else {
        assert_eq!(bits(p), bits(r));
    }
    kani::cover!(
        rect_spec::touches_pole(r) && !arc_spec::full(r.lng),
        "widened at a pole"
    );
}
