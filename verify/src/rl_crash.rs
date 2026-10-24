// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s2::Rect::<op>` for the 34 in-scope operations of the
//! latitude–longitude rectangle (spec 005 US1, FR-004; data-model §5), grouped as spec 004's
//! `iv_crash`.
//!
//! Each harness draws its inputs from the operation's domain in `values/INVENTORY.md` §4:
//! - every valid rectangle, built as a struct literal from four arbitrary bounds and kept when
//!   `rect_spec::valid` accepts it (`ValidRect`);
//! - every latitude–longitude, NaN included, for the operations that screen out an invalid one
//!   (`add_point`, `contains_lat_lng`; K2, FR-018);
//! - a valid latitude–longitude for the operations that check the longitude only in debug
//!   builds (`from_lat_lng`, `from_point_pair`, `interior_contains_lat_lng`; K3, FR-019);
//! - every non-NaN margin for `expanded`, and every non-NaN size for `from_center_size`
//!   (FR-019).
//!
//! Every rectangle operation calls the real and circular intervals' code, which runs here as
//! it is, not stubbed, debug assertions included (spec 005 research R5). The covers show the
//! domains non-empty and the outcomes FR-009 names reachable: rectangles at a pole, across the
//! antimeridian, empty and full, and an out-of-range point that `add_point` ignores.

use s2rst::s2::{LatLng, Rect, RectVertex, Region};
use std::f64::consts::{FRAC_PI_2, PI};

use crate::rect_spec;
use crate::value_domain::{any_f64, any_lat_lng, any_rect, any_valid_lat_lng};

/// Every latitude–longitude, NaN components included.
fn any_ll() -> LatLng {
    LatLng::from_radians(any_f64(), any_f64())
}

/// C-s2::Rect::new, empty, full, Default, full_lat and full_lng: complete without panic,
/// arithmetic overflow or undefined behaviour, `new` for the two intervals of every valid
/// rectangle.
#[kani::proof]
fn c_constructors() {
    let r = any_rect();
    let n = Rect::new(r.lat, r.lng);
    let (e, f, d) = (Rect::empty(), Rect::full(), Rect::default());
    let (lat, lng) = (Rect::full_lat(), Rect::full_lng());
    kani::cover!(
        rect_spec::empty(n) && !rect_spec::empty(f),
        "empty and full"
    );
    kani::cover!(
        e == d && lat.hi == FRAC_PI_2 && lng.hi == PI,
        "the special values"
    );
}

/// C-s2::Rect::from_lat_lng and from_point_pair: complete without panic, arithmetic overflow
/// or undefined behaviour for every pair of valid latitude–longitudes.
#[kani::proof]
fn c_from_points() {
    let (a, b) = (any_valid_lat_lng(), any_valid_lat_lng());
    let p = Rect::from_lat_lng(a);
    let pair = Rect::from_point_pair(a, b);
    kani::cover!(p.lat.lo == FRAC_PI_2, "a point at the north pole");
    kani::cover!(pair.lng.lo > pair.lng.hi, "a pair across the antimeridian");
}

/// C-s2::Rect::from_center_size: completes without panic, arithmetic overflow or undefined
/// behaviour for every valid center and every size with non-NaN components (FR-019).
#[kani::proof]
fn c_from_center_size() {
    let (c, size) = (any_valid_lat_lng(), any_lat_lng());
    let r = Rect::from_center_size(c, size);
    kani::cover!(r.is_full(), "full");
    kani::cover!(rect_spec::empty(r), "empty, from a negative size");
    kani::cover!(
        r.lat.hi == FRAC_PI_2 && !r.is_full(),
        "clipped at the north pole"
    );
}

/// C-s2::Rect::is_valid: completes without panic, arithmetic overflow or undefined behaviour
/// for every four `f64` bounds, NaN and the infinities included.
#[kani::proof]
fn c_is_valid() {
    let r = Rect {
        lat: s2rst::r1::Interval {
            lo: any_f64(),
            hi: any_f64(),
        },
        lng: s2rst::s1::Interval {
            lo: any_f64(),
            hi: any_f64(),
        },
    };
    let v = r.is_valid();
    kani::cover!(v, "valid");
    kani::cover!(!v && r.lat.lo.is_nan(), "NaN");
}

/// C-s2::Rect::is_empty, is_full, is_point, is_inverted, lo, hi, vertex, center and size:
/// complete without panic, arithmetic overflow or undefined behaviour for every valid
/// rectangle and every vertex.
#[kani::proof]
fn c_classifiers_and_bounds() {
    let r = any_rect();
    let k: u8 = kani::any();
    let v = match k % 4 {
        0 => RectVertex::LowerLeft,
        1 => RectVertex::LowerRight,
        2 => RectVertex::UpperRight,
        _ => RectVertex::UpperLeft,
    };
    let c = (r.is_empty(), r.is_full(), r.is_point(), r.is_inverted());
    let _ = (r.lo(), r.hi(), r.vertex(v), r.center(), r.size());
    kani::cover!(c.0, "empty");
    kani::cover!(c.1, "full");
    kani::cover!(c.2, "a point");
    kani::cover!(c.3 && !c.0, "inverted longitudes");
}

/// C-s2::Rect::contains_lat_lng and add_point: complete without panic, arithmetic overflow or
/// undefined behaviour for every valid rectangle and every latitude–longitude, NaN and
/// out-of-range values included (K2: such a point is not contained and not added).
#[kani::proof]
fn c_screened_point_operations() {
    let r = any_rect();
    let ll = any_ll();
    let c = r.contains_lat_lng(ll);
    let a = r.add_point(ll);
    kani::cover!(c, "contained");
    kani::cover!(!ll.is_valid() && a == r, "an invalid point is ignored");
    kani::cover!(
        rect_spec::empty(r) && a.is_point(),
        "the empty rectangle becomes a point"
    );
}

/// C-s2::Rect::interior_contains_lat_lng: completes without panic, arithmetic overflow or
/// undefined behaviour for every valid rectangle and every valid latitude–longitude (K3,
/// FR-019).
#[kani::proof]
fn c_interior_contains_lat_lng() {
    let r = any_rect();
    let ll = any_valid_lat_lng();
    let i = r.interior_contains_lat_lng(ll);
    kani::cover!(i, "in the interior");
    kani::cover!(!i && r.contains_lat_lng(ll), "on the boundary");
}

/// C-s2::Rect::contains, interior_contains, intersects, interior_intersects, union and
/// intersection: complete without panic, arithmetic overflow or undefined behaviour for every
/// pair of valid rectangles.
#[kani::proof]
fn c_relations_and_set_operations() {
    let (a, b) = (any_rect(), any_rect());
    let r = (
        a.contains(b),
        a.interior_contains(b),
        a.intersects(b),
        a.interior_intersects(b),
    );
    let (u, i) = (a.union(b), a.intersection(b));
    kani::cover!(
        r.0 && !r.1 && !rect_spec::empty(b),
        "contained, touching the boundary"
    );
    kani::cover!(r.2 && !r.3, "intersecting only on the boundary");
    kani::cover!(u.is_full() && !a.is_full() && !b.is_full(), "a full union");
    kani::cover!(
        rect_spec::empty(i) && !rect_spec::empty(a) && !rect_spec::empty(b),
        "disjoint"
    );
}

/// C-s2::Rect::polar_closure: completes without panic, arithmetic overflow or undefined
/// behaviour for every valid rectangle.
#[kani::proof]
fn c_polar_closure() {
    let r = any_rect();
    let p = r.polar_closure();
    kani::cover!(p.lng.is_full() && !r.lng.is_full(), "widened at a pole");
    kani::cover!(p == r && !rect_spec::touches_pole(r), "unchanged");
}

/// C-s2::Rect::expanded: completes without panic, arithmetic overflow or undefined behaviour
/// for every valid rectangle and every margin with non-NaN components (FR-019).
#[kani::proof]
fn c_expanded() {
    let r = any_rect();
    let m = any_lat_lng();
    let e = r.expanded(m);
    kani::cover!(
        rect_spec::empty(e) && !rect_spec::empty(r),
        "shrunk to empty"
    );
    kani::cover!(
        e.lat.hi == FRAC_PI_2 && r.lat.hi < FRAC_PI_2,
        "clipped at the north pole"
    );
    kani::cover!(
        e.lng.is_full() && !r.lng.is_full(),
        "grown to full longitudes"
    );
}

/// C-s2::Rect::approx_eq: completes without panic, arithmetic overflow or undefined behaviour
/// for every pair of valid rectangles.
#[kani::proof]
fn c_approx_eq() {
    let (a, b) = (any_rect(), any_rect());
    let e = a.approx_eq(b);
    kani::cover!(e && a != b, "approximately equal, not equal");
    kani::cover!(!e, "not approximately equal");
}

/// C-s2::Rect::eq, rect_bound and `impl Region for s2::Rect::rect_bound`: complete without
/// panic, arithmetic overflow or undefined behaviour for every pair of valid rectangles.
#[kani::proof]
fn c_eq_and_bound() {
    let (a, b) = (any_rect(), any_rect());
    let eq = a == b;
    let (inherent, region) = (a.rect_bound(), Region::rect_bound(&a));
    kani::cover!(eq && rect_spec::empty(a), "equal empty rectangles");
    kani::cover!(inherent == region, "the two bounds");
}
