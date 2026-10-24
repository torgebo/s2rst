// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a latitude–longitude rectangle (spec 005 data-model §1.2): the
//! product of a set of latitudes and an arc of longitudes, restricted to valid latitudes.
//!
//! Written from upstream's `S2LatLngRect` header (`s2latlng_rect.h`, `v0.14.0-40`). A rectangle
//! is valid when both latitude bounds lie in [−π/2, π/2], its longitudes form a valid arc, and
//! its latitudes are empty exactly when its longitudes are (`s2latlng_rect.h:417`). Its points
//! are the valid latitude–longitudes whose latitude is in the latitude set ([`real_spec`]) and
//! whose longitude is on the arc ([`arc_spec`], spec 004).
//!
//! The relations between rectangles are those of the product sets. For valid rectangles they
//! are the conjunctions of the two factors' relations, as upstream defines them
//! (`s2latlng_rect.cc:176–192`), because a valid rectangle is empty exactly when both factors
//! are. The description is built from the two factor descriptions and never calls the
//! library's methods (FR-007). Booleans are combined with `&&`, `||` and `==` only (spec 004
//! research R13).

use s2rst::s2::Rect;
use std::f64::consts::{FRAC_PI_2, PI};

use crate::{arc_spec, real_spec};

/// Whether `r` is a valid rectangle (spec FR-005; `s2latlng_rect.h:417`).
#[must_use]
pub fn valid(r: Rect) -> bool {
    r.lat.lo.abs() <= FRAC_PI_2
        && r.lat.hi.abs() <= FRAC_PI_2
        && arc_spec::valid(r.lng.lo, r.lng.hi)
        && real_spec::empty(r.lat) == arc_spec::empty(r.lng)
}

/// Whether a latitude–longitude is valid: `|lat| ≤ π/2` and `|lng| ≤ π`, false for NaN
/// (`S2LatLng::is_valid`, `s2latlng.h`).
#[must_use]
pub fn ll_valid(lat: f64, lng: f64) -> bool {
    lat.abs() <= FRAC_PI_2 && lng.abs() <= PI
}

/// Whether `r` is empty: no latitude (for a valid rectangle, no longitude either).
#[must_use]
pub fn empty(r: Rect) -> bool {
    real_spec::empty(r.lat)
}

/// Whether the latitude–longitude is a point of `r`: valid, with its latitude in the
/// latitude set and its longitude on the arc.
#[must_use]
pub fn member(r: Rect, lat: f64, lng: f64) -> bool {
    ll_valid(lat, lng) && real_spec::member(r.lat, lat) && arc_spec::member(r.lng, lng)
}

/// Whether a valid latitude–longitude lies in the interior of `r`: the product of the open
/// latitude set and the open arc (`InteriorContains(const S2LatLng&)`,
/// `s2latlng_rect.cc:168`).
#[must_use]
pub fn interior_member(r: Rect, lat: f64, lng: f64) -> bool {
    ll_valid(lat, lng) && real_spec::interior(r.lat, lat) && arc_spec::interior(r.lng, lng)
}

/// Whether `a` contains `b`: `b` is empty, or each factor of `a` contains `b`'s.
#[must_use]
pub fn contains(a: Rect, b: Rect) -> bool {
    empty(b) || (real_spec::contains(a.lat, b.lat) && arc_spec::contains(a.lng, b.lng))
}

/// Whether the interior of `a` contains `b`.
#[must_use]
pub fn interior_contains(a: Rect, b: Rect) -> bool {
    empty(b)
        || (real_spec::interior_contains(a.lat, b.lat) && arc_spec::interior_contains(a.lng, b.lng))
}

/// Whether `a` and `b` have a common point: both factors intersect.
#[must_use]
pub fn intersects(a: Rect, b: Rect) -> bool {
    real_spec::intersects(a.lat, b.lat) && arc_spec::intersects(a.lng, b.lng)
}

/// Whether the interior of `a` has a point in common with `b`.
#[must_use]
pub fn interior_intersects(a: Rect, b: Rect) -> bool {
    real_spec::interior_intersects(a.lat, b.lat) && arc_spec::interior_intersects(a.lng, b.lng)
}

/// Whether `r` reaches a pole: its latitudes include −π/2 or π/2 as a bound. At a pole every
/// longitude is the same point, which is what the polar closure makes explicit
/// (`PolarClosure`, `s2latlng_rect.h:247`).
#[must_use]
pub fn touches_pole(r: Rect) -> bool {
    r.lat.lo == -FRAC_PI_2 || r.lat.hi == FRAC_PI_2
}

#[cfg(test)]
#[path = "rect_spec_tests.rs"]
mod rect_spec_tests;
