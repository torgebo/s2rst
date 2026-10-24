// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 005 planning probe (research R0): Kani harnesses that measure the real interval
//! against a real-set description, the rectangle against the product of the two interval
//! descriptions (`real` below and 004's `arc_spec`), the component-wise alternative, and the
//! operations that go through the circular interval's expansion and approximate equality.
//! They run from a scratch crate that depends on `s2rst` and `s2rst-verify`; results in
//! research.md R0. Not part of `verify/`.
#![allow(non_snake_case)]

use s2rst::r1;
use s2rst::s1;
use s2rst::s2::{LatLng, Rect};
use s2rst_verify::arc_spec;
use std::f64::consts::{FRAC_PI_2, PI};

// ---- a real-set description of r1::Interval (draft of real_spec) ------------------------------

fn r_empty(x: r1::Interval) -> bool {
    x.lo > x.hi
}
fn r_member(x: r1::Interval, p: f64) -> bool {
    x.lo <= p && p <= x.hi
}
fn r_contains(x: r1::Interval, y: r1::Interval) -> bool {
    r_empty(y) || (x.lo <= y.lo && y.hi <= x.hi)
}

fn any_r1() -> r1::Interval {
    let x = r1::Interval { lo: kani::any(), hi: kani::any() };
    kani::assume(!x.lo.is_nan() && !x.hi.is_nan());
    x
}
fn any_finite_r1() -> r1::Interval {
    let x = any_r1();
    kani::assume(x.lo.is_finite() && x.hi.is_finite());
    x
}

// ---- the rectangle as a product ---------------------------------------------------------------

fn rect_valid(r: Rect) -> bool {
    r.lat.lo.abs() <= FRAC_PI_2
        && r.lat.hi.abs() <= FRAC_PI_2
        && arc_spec::valid(r.lng.lo, r.lng.hi)
        && r_empty(r.lat) == arc_spec::empty(r.lng)
}
fn ll_valid(lat: f64, lng: f64) -> bool {
    lat.abs() <= FRAC_PI_2 && lng.abs() <= PI
}
fn rect_member(r: Rect, lat: f64, lng: f64) -> bool {
    ll_valid(lat, lng) && r_member(r.lat, lat) && arc_spec::member(r.lng, lng)
}
fn rect_contains(a: Rect, b: Rect) -> bool {
    r_empty(b.lat) || (r_contains(a.lat, b.lat) && arc_spec::contains(a.lng, b.lng))
}

fn any_rect() -> Rect {
    let r = Rect {
        lat: r1::Interval { lo: kani::any(), hi: kani::any() },
        lng: s1::Interval { lo: kani::any(), hi: kani::any() },
    };
    kani::assume(rect_valid(r));
    r
}
fn any_ll() -> (f64, f64) {
    let (lat, lng): (f64, f64) = (kani::any(), kani::any());
    (lat, lng)
}

// ---- r1::Interval ------------------------------------------------------------------------------

#[kani::proof]
fn r0_r1_contains_model() {
    let (x, y) = (any_r1(), any_r1());
    assert_eq!(x.contains_interval(y), r_contains(x, y));
}

#[kani::proof]
fn r0_r1_union_least() {
    let (x, y, z) = (any_r1(), any_r1(), any_r1());
    let u = x.union(y);
    assert!(r_contains(u, x) && r_contains(u, y));
    if r_contains(z, x) && r_contains(z, y) {
        assert!(r_contains(z, u));
    }
}

#[kani::proof]
fn r0_r1_expanded_crash() {
    let x = any_r1();
    let m: f64 = kani::any();
    let _ = x.expanded(m);
}

#[kani::proof]
fn r0_r1_expanded_finite_contains() {
    let x = any_finite_r1();
    let m: f64 = kani::any();
    kani::assume(m.is_finite() && m >= 0.0);
    assert!(r_contains(x.expanded(m), x));
}

#[kani::proof]
fn r0_r1_approx_eq_reflexive() {
    let x = any_r1();
    let e: f64 = kani::any();
    kani::assume(e.is_finite() && e >= 0.0);
    assert!(x.approx_eq_with(x, e));
}

#[kani::proof]
fn r0_r1_dhd_nonneg() {
    let (x, y) = (any_r1(), any_r1());
    let d = x.directed_hausdorff_distance(y);
    assert!(d >= 0.0);
}

#[kani::proof]
fn r0_r1_center_inside() {
    let x = any_finite_r1();
    kani::assume(!r_empty(x));
    assert!(r_member(x, x.center()));
}

#[kani::proof]
fn r0_r1_project_crash() {
    let x = any_r1();
    kani::assume(!r_empty(x));
    let p: f64 = kani::any();
    kani::assume(!p.is_nan());
    let q = x.project(p);
    assert!(r_member(x, q));
}

// ---- s2::Rect ---------------------------------------------------------------------------------

#[kani::proof]
fn r0_rect_valid_model() {
    let r = Rect {
        lat: r1::Interval { lo: kani::any(), hi: kani::any() },
        lng: s1::Interval { lo: kani::any(), hi: kani::any() },
    };
    assert_eq!(r.is_valid(), rect_valid(r));
}

#[kani::proof]
fn r0_rect_contains_latlng() {
    let r = any_rect();
    let (lat, lng) = any_ll();
    kani::assume(!lat.is_nan() && !lng.is_nan());
    assert_eq!(r.contains_lat_lng(LatLng::from_radians(lat, lng)), rect_member(r, lat, lng));
}

#[kani::proof]
fn r0_rect_contains_model() {
    let (a, b) = (any_rect(), any_rect());
    assert_eq!(a.contains(b), rect_contains(a, b));
}

#[kani::proof]
fn r0_rect_union_direct() {
    let (a, b) = (any_rect(), any_rect());
    let u = a.union(b);
    assert!(rect_valid(u));
    assert!(rect_contains(u, a) && rect_contains(u, b));
}

#[kani::proof]
fn r0_rect_union_componentwise() {
    let (a, b) = (any_rect(), any_rect());
    let u = a.union(b);
    let (lat, lng) = (a.lat.union(b.lat), a.lng.union(b.lng));
    assert_eq!(u.lat.lo.to_bits(), lat.lo.to_bits());
    assert_eq!(u.lat.hi.to_bits(), lat.hi.to_bits());
    assert_eq!(u.lng.lo.to_bits(), lng.lo.to_bits());
    assert_eq!(u.lng.hi.to_bits(), lng.hi.to_bits());
}

#[kani::proof]
fn r0_rect_intersection_direct() {
    let (a, b) = (any_rect(), any_rect());
    let (lat, lng) = any_ll();
    let i = a.intersection(b);
    assert!(rect_valid(i));
    if rect_member(a, lat, lng) && rect_member(b, lat, lng) {
        assert!(rect_member(i, lat, lng));
    }
}

#[kani::proof]
fn r0_rect_add_point() {
    let r = any_rect();
    let (lat, lng) = any_ll();
    kani::assume(!lat.is_nan() && !lng.is_nan());
    let a = r.add_point(LatLng::from_radians(lat, lng));
    assert!(rect_valid(a));
    assert!(rect_contains(a, r));
    if ll_valid(lat, lng) {
        assert!(rect_member(a, lat, lng));
    }
}

#[kani::proof]
fn r0_rect_polar_closure() {
    let r = any_rect();
    let c = r.polar_closure();
    assert!(rect_valid(c) && rect_contains(c, r));
}

#[kani::proof]
fn r0_rect_expanded_crash() {
    let r = any_rect();
    let (mlat, mlng): (f64, f64) = (kani::any(), kani::any());
    kani::assume(!mlat.is_nan() && !mlng.is_nan());
    let e = r.expanded(LatLng::from_radians(mlat, mlng));
    assert!(rect_valid(e));
}

#[kani::proof]
fn r0_rect_expanded_contains() {
    let r = any_rect();
    let (mlat, mlng): (f64, f64) = (kani::any(), kani::any());
    kani::assume(mlat.is_finite() && mlng.is_finite() && mlat >= 0.0 && mlng >= 0.0);
    let e = r.expanded(LatLng::from_radians(mlat, mlng));
    assert!(rect_contains(e, r));
}

#[kani::proof]
fn r0_rect_from_center_size() {
    let (clat, clng) = any_ll();
    kani::assume(ll_valid(clat, clng));
    let (slat, slng) = any_ll();
    kani::assume(!slat.is_nan() && !slng.is_nan());
    let r = Rect::from_center_size(LatLng::from_radians(clat, clng), LatLng::from_radians(slat, slng));
    assert!(rect_valid(r));
}

#[kani::proof]
fn r0_rect_approx_eq_reflexive() {
    let r = any_rect();
    assert!(r.approx_eq(r));
}
