// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle validity and classification (P-LVAL, spec 005 data-model §4.2, §3), against the
//! rectangle description (`rect_spec`), the product of `real_spec` and spec 004's `arc_spec`.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::r1;
use s2rst::s1;
use s2rst::s2::{LatLng, Rect, Region};
use std::f64::consts::FRAC_PI_2;

use crate::arc_spec;
use crate::real_spec;
use crate::rect_spec;
use crate::value_domain::{any_f64, any_lat_lng, any_rect, any_valid_lat_lng};

/// Every latitude–longitude, NaN components included.
fn any_ll() -> LatLng {
    LatLng::from_radians(any_f64(), any_f64())
}

/// P-LVAL-01 (correct result): the validity check agrees with the description for every four
/// bounds, NaN and the infinities included.
///
/// For every four `f64` bounds: `Rect { lat, lng }.is_valid() == rect_spec::valid(..)`.
#[kani::proof]
fn p_lval_01() {
    let r = Rect {
        lat: r1::Interval {
            lo: any_f64(),
            hi: any_f64(),
        },
        lng: s1::Interval {
            lo: any_f64(),
            hi: any_f64(),
        },
    };
    let v = r.is_valid();
    assert_eq!(v, rect_spec::valid(r));
    kani::cover!(v && r.lng.lo > r.lng.hi, "valid, across the antimeridian");
    kani::cover!(
        !v && real_spec::empty(r.lat) && !arc_spec::empty(r.lng),
        "emptiness disagrees"
    );
}

/// P-LVAL-02 (validity): every in-scope operation that returns a rectangle returns a valid
/// one, for every input in its domain. One harness per operation (`p_lval_02__<op>`); this one
/// covers the constants: `empty`, `full` and `default`.
#[kani::proof]
fn p_lval_02__constants() {
    assert!(rect_spec::valid(Rect::empty()));
    assert!(rect_spec::valid(Rect::full()));
    assert!(rect_spec::valid(Rect::default()));
}

/// P-LVAL-02 (validity): `from_lat_lng` returns a valid rectangle for every valid
/// latitude–longitude.
#[kani::proof]
fn p_lval_02__from_lat_lng() {
    assert!(rect_spec::valid(Rect::from_lat_lng(any_valid_lat_lng())));
}

/// P-LVAL-02 (validity): `from_point_pair` returns a valid rectangle for every pair of valid
/// latitude–longitudes.
#[kani::proof]
fn p_lval_02__from_point_pair() {
    let (a, b) = (any_valid_lat_lng(), any_valid_lat_lng());
    assert!(rect_spec::valid(Rect::from_point_pair(a, b)));
}

/// P-LVAL-02 (validity): `from_center_size` returns a valid rectangle for every valid centre
/// and every size with non-NaN components.
#[kani::proof]
fn p_lval_02__from_center_size() {
    let r = Rect::from_center_size(any_valid_lat_lng(), any_lat_lng());
    assert!(rect_spec::valid(r));
    kani::cover!(r.is_full(), "full");
    kani::cover!(rect_spec::empty(r), "empty");
}

/// P-LVAL-02 (validity): `add_point` returns a valid rectangle for every valid rectangle and
/// every latitude–longitude, NaN and out-of-range values included (K2).
#[kani::proof]
fn p_lval_02__add_point() {
    let (r, ll) = (any_rect(), any_ll());
    assert!(rect_spec::valid(r.add_point(ll)));
}

/// P-LVAL-02 (validity): `polar_closure` returns a valid rectangle for every valid rectangle.
#[kani::proof]
fn p_lval_02__polar_closure() {
    assert!(rect_spec::valid(any_rect().polar_closure()));
}

/// P-LVAL-02 (validity): `expanded` returns a valid rectangle for every valid rectangle and
/// every margin with non-NaN components. This is also P-LEXP-01's validity clause.
#[kani::proof]
fn p_lval_02__expanded() {
    let (r, m) = (any_rect(), any_lat_lng());
    let e = r.expanded(m);
    assert!(rect_spec::valid(e));
    kani::cover!(
        rect_spec::empty(e) && !rect_spec::empty(r),
        "shrunk to empty"
    );
    kani::cover!(
        e.lat.hi == FRAC_PI_2 && r.lat.hi < FRAC_PI_2,
        "clipped at the north pole"
    );
}

/// P-LVAL-02 (validity): `union` returns a valid rectangle for every pair of valid
/// rectangles.
#[kani::proof]
fn p_lval_02__union() {
    let (a, b) = (any_rect(), any_rect());
    assert!(rect_spec::valid(a.union(b)));
}

/// P-LVAL-02 (validity): `intersection` returns a valid rectangle for every pair of valid
/// rectangles.
#[kani::proof]
fn p_lval_02__intersection() {
    let (a, b) = (any_rect(), any_rect());
    assert!(rect_spec::valid(a.intersection(b)));
}

/// P-LVAL-02 (validity): `rect_bound`, inherent and through the region interface, returns a
/// valid rectangle for every valid rectangle.
#[kani::proof]
fn p_lval_02__rect_bound() {
    let r = any_rect();
    assert!(rect_spec::valid(r.rect_bound()));
    assert!(rect_spec::valid(Region::rect_bound(&r)));
}

/// P-LVAL-03 (correct result): empty, full, a single point and inverted longitudes agree with
/// the description, and `empty`, `full`, `default`, `full_lat` and `full_lng` are the
/// documented values.
///
/// For every valid `r`: `is_empty() == rect_spec::empty(r)`; `is_full() ⇔ lat == [−π/2, π/2]
/// ∧ arc_spec::full(lng)`; `is_point() ⇔ lat.lo == lat.hi ∧ lng.lo == lng.hi`;
/// `is_inverted() == arc_spec::inverted(lng)`. `empty()` and `default()` are empty; `full()`
/// is full; `full_lat()` is `[−π/2, π/2]` and `full_lng()` is full.
#[kani::proof]
fn p_lval_03() {
    let r = any_rect();
    assert_eq!(r.is_empty(), rect_spec::empty(r));
    let full_lat = r.lat.lo == -FRAC_PI_2 && r.lat.hi == FRAC_PI_2;
    assert_eq!(r.is_full(), full_lat && arc_spec::full(r.lng));
    assert_eq!(r.is_point(), r.lat.lo == r.lat.hi && r.lng.lo == r.lng.hi);
    assert_eq!(r.is_inverted(), arc_spec::inverted(r.lng));
    assert!(rect_spec::empty(Rect::empty()) && rect_spec::empty(Rect::default()));
    let f = Rect::full();
    assert!(f.lat.lo == -FRAC_PI_2 && f.lat.hi == FRAC_PI_2 && arc_spec::full(f.lng));
    let (fl, fg) = (Rect::full_lat(), Rect::full_lng());
    assert!(fl.lo == -FRAC_PI_2 && fl.hi == FRAC_PI_2 && arc_spec::full(fg));
    kani::cover!(r.is_point(), "a point");
    kani::cover!(r.is_inverted() && !r.is_empty(), "inverted, not empty");
}
