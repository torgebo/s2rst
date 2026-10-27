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

use s2rst::s1::Angle;
use s2rst::s2::{LatLng, Rect, RectVertex, Region};
use std::f64::consts::{FRAC_PI_2, PI};

use crate::rect_spec;
use crate::value_domain::{any_box_point, any_f64, any_lat_lng, any_rect, any_valid_lat_lng};

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

// ---------------------------------------------------------------------------------------------
// Outside the domain: the documented panics (gap 1; `crash.rs` explains the `n_*` and `r_*`
// conventions). The longitude checks are `s1::Interval`'s `debug_assert!`s.
// ---------------------------------------------------------------------------------------------

/// Every latitude–longitude whose longitude is outside [−π, π] or NaN, with any latitude.
fn any_bad_lng() -> LatLng {
    let (lat, lng): (f64, f64) = (kani::any(), kani::any());
    kani::assume(!crate::arc_spec::in_range(lng));
    LatLng::from_radians(lat, lng)
}

/// N-s2::Rect::from_lat_lng, N-from_point_pair: in debug builds, each panics for every point
/// whose longitude is outside [−π, π] or NaN (for `from_point_pair`, either point), whatever
/// the latitude.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_from_points() {
    let bad = any_bad_lng();
    let other = any_lat_lng();
    let op: u8 = kani::any();
    match op {
        0 => {
            let _ = Rect::from_lat_lng(bad);
        }
        1 => {
            let _ = Rect::from_point_pair(bad, other);
        }
        _ => {
            let _ = Rect::from_point_pair(other, bad);
        }
    }
    kani::cover!(true, "returns");
}

/// R-s2::Rect::from_lat_lng, R-from_point_pair: in a build without debug assertions, each
/// completes without panic or overflow on the same points.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_from_points() {
    let (bad, other) = (any_bad_lng(), any_lat_lng());
    let (a, b, c) = (
        Rect::from_lat_lng(bad),
        Rect::from_point_pair(bad, other),
        Rect::from_point_pair(other, bad),
    );
    kani::cover!(
        !a.is_valid() && !b.is_valid() && !c.is_valid(),
        "invalid rectangles"
    );
}

/// The complement of `from_center_size`'s domain: a centre whose longitude is outside [−π, π]
/// or NaN with any size, or a centre with a valid longitude and a NaN longitude size.
///
/// The latitudes are finite: an infinite centre latitude with an infinite size latitude gives
/// `∞ − ∞` in the latitude interval, which comes first and which Kani reports as a failure
/// (NaN), although Rust gives NaN without a panic.
fn outside_center_size() -> (LatLng, LatLng) {
    let (lat, lng, slat, slng): (f64, f64, f64, f64) =
        (kani::any(), kani::any(), kani::any(), kani::any());
    kani::assume(lat.is_finite() && slat.is_finite());
    kani::assume(!crate::arc_spec::in_range(lng) || slng.is_nan());
    (
        LatLng::from_radians(lat, lng),
        LatLng::from_radians(slat, slng),
    )
}

/// N-s2::Rect::from_center_size: in debug builds, panics for every centre whose longitude is
/// outside [−π, π] or NaN, and every size whose longitude is NaN.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_from_center_size() {
    let (c, s) = outside_center_size();
    let _ = Rect::from_center_size(c, s);
    kani::cover!(true, "returns");
}

/// R-s2::Rect::from_center_size: in a build without debug assertions, completes without panic
/// or overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_from_center_size() {
    let (c, s) = outside_center_size();
    let r = Rect::from_center_size(c, s);
    kani::cover!(!r.is_valid(), "an invalid rectangle");
}

/// N-s2::Rect::interior_contains_lat_lng: in debug builds, panics for every valid rectangle and
/// every point whose latitude is strictly inside the rectangle's latitudes and whose longitude
/// is outside [−π, π] or NaN. The latitude is tested first: over every point the first version
/// of this harness returned (2026-10-26), for a latitude outside, which answers `false` before
/// the longitude is checked, as upstream.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_interior_contains_lat_lng() {
    let (r, ll) = (any_rect(), any_bad_lng());
    kani::assume(crate::real_spec::interior(r.lat, ll.lat.radians()));
    let _ = r.interior_contains_lat_lng(ll);
    kani::cover!(true, "returns");
}

/// R-s2::Rect::interior_contains_lat_lng: in a build without debug assertions, completes
/// without panic or overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_interior_contains_lat_lng() {
    let (r, ll) = (any_rect(), any_bad_lng());
    kani::assume(crate::real_spec::interior(r.lat, ll.lat.radians()));
    let c = r.interior_contains_lat_lng(ll);
    kani::cover!(!c, "not contained");
}

/// N-s2::Rect::expanded: in debug builds, panics for every valid rectangle whose longitudes
/// are not full and every margin whose longitude is NaN. Over every valid rectangle the first
/// version of this harness returned (2026-10-26): full longitudes, as `o_expanded_nan_full_lng`
/// proves.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_expanded() {
    let r = any_rect();
    kani::assume(!crate::arc_spec::full(r.lng));
    let m = LatLng::from_radians(kani::any(), f64::NAN);
    let _ = r.expanded(m);
    kani::cover!(true, "returns");
}

/// O-s2::Rect::expanded (outside its domain, returns): completes without panic or overflow, in
/// every build, for every valid rectangle with full longitudes and every margin whose longitude is
/// NaN (its `# Panics`: the longitudes stay full).
#[kani::proof]
fn o_expanded_nan_full_lng() {
    let r = any_rect();
    kani::assume(crate::arc_spec::full(r.lng));
    let e = r.expanded(LatLng::from_radians(kani::any(), f64::NAN));
    assert!(e.is_empty() || e.lng.is_full());
    kani::cover!(!e.is_empty(), "full longitudes kept");
}

/// R-s2::Rect::expanded: in a build without debug assertions, completes without panic or
/// overflow on the same inputs as `n_expanded`.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_expanded() {
    let r = any_rect();
    kani::assume(!crate::arc_spec::full(r.lng));
    let e = r.expanded(LatLng::from_radians(kani::any(), f64::NAN));
    kani::cover!(!e.is_valid(), "an invalid rectangle");
}

// ---------------------------------------------------------------------------------------------
// Floating-point geometry (gap 2 of the 2026-10-26 coverage review): the entries spec 005
// excluded as floating-point geometry, proven crash-free under the function contracts of
// `stubs` where they reach a C library function the checker does not model (spec 008 FR-003).
// Each harness names its contracts; `sin` and `cos` are the checker's models.
// ---------------------------------------------------------------------------------------------

/// C-s2::Rect::area: completes without panic, overflow or undefined behaviour for every valid
/// rectangle.
#[kani::proof]
fn c_fg_area() {
    let r = any_rect();
    let a = r.area();
    kani::cover!(r.is_empty() && a == 0.0, "the empty rectangle");
    kani::cover!(!r.is_empty(), "a rectangle with an area");
}

/// C-s2::Rect::contains_point, interior_contains_point and Region::contains_point: complete for
/// every valid rectangle and every box point, under `atan2`'s contract (the point's latitude and
/// longitude, which the contract keeps in [−π, π], so `interior_contains_lat_lng`'s debug check
/// holds).
///
/// Under contract: `atan2_contract`.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn c_fg_contains_point() {
    let (r, p) = (any_rect(), any_box_point());
    let c = (
        r.contains_point(p),
        r.interior_contains_point(p),
        Region::contains_point(&r, &p),
    );
    kani::cover!(c.0, "a contained point");
    kani::cover!(!c.0, "a point outside");
}

/// C-s2::Rect::expanded_by_distance, a negative distance: completes for every valid rectangle
/// and every finite negative angle, under the contracts of `asin`, `sin` and `cos`. With the
/// checker's own `sin` and `cos` it failed (2026-10-26): they let `sin(−d)` or `cos(max |lat|)`
/// take the wrong sign, so `asin`'s argument left [−1, 1] and its NaN reached `s1::Interval`'s
/// debug check. The real values: the latitudes are non-empty only for `|d| ≤ π/2`, where both are
/// non-negative.
///
/// Under contract: `asin_contract`, `sin_contract`, `cos_contract`.
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::sin, crate::stubs::sin_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
fn c_fg_expanded_by_distance__neg() {
    let r = any_rect();
    let d: f64 = kani::any();
    kani::assume(d.is_finite() && d < 0.0);
    let e = r.expanded_by_distance(Angle::from_radians(d));
    kani::cover!(e.is_empty() && !r.is_empty(), "shrunk to empty");
}

/// C-s2::Rect::expanded_by_distance, a distance of at least 0: completes for every valid
/// rectangle and every finite non-negative angle, under the contracts of `asin`, `atan2` and
/// `cos` (the vertices' caps' `rect_bound`, spec 008).
///
/// Under contract: `asin_contract`, `atan2_contract`, `cos_contract`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
fn c_fg_expanded_by_distance__pos() {
    let r = any_rect();
    let d = crate::value_domain::any_finite_nonneg_angle();
    let e = r.expanded_by_distance(d);
    kani::cover!(!r.is_empty() && e.is_full(), "grown to the full rectangle");
}

/// C-s2::Rect::expand_for_subregions: completes for every valid rectangle.
#[kani::proof]
fn c_fg_expand_for_subregions() {
    let r = any_rect();
    let e = r.expand_for_subregions();
    kani::cover!(!r.is_full() && e.is_full(), "nearly antipodal points: full");
}

/// C-s2::Rect::cap_bound and Region::cap_bound: complete for every valid rectangle.
#[cfg(feature = "slow")]
#[kani::proof]
fn c_fg_cap_bound() {
    let r = any_rect();
    let (c, _) = (r.cap_bound(), Region::cap_bound(&r));
    kani::cover!(r.is_empty() && c.is_empty(), "the empty rectangle");
    kani::cover!(!r.is_empty() && !c.is_full(), "a proper cap");
}

/// C-s2::Rect::centroid: completes for every valid rectangle.
#[kani::proof]
fn c_fg_centroid() {
    let r = any_rect();
    let _ = r.centroid();
    kani::cover!(!r.is_empty(), "a rectangle with a centroid");
}

/// C-s2::Rect::boundary_intersects: completes for every valid rectangle and every pair of box
/// points, under `atan2`'s contract.
///
/// Under contract: `atan2_contract`.
///
/// **Not proven:** times out at 60 minutes weekly (2026-10-27; the robust crossing test, `EdgeCrosser`), a tool limit listed in
/// `known-timeouts.txt`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn c_fg_boundary_intersects() {
    let (r, a, b) = (any_rect(), any_box_point(), any_box_point());
    let x = r.boundary_intersects(a, b);
    kani::cover!(x, "crossing the boundary");
}

/// C-s2::Rect::get_distance: completes for every pair of non-empty valid rectangles (its debug
/// checks), under the contracts of `asin`, `atan2` and `sin_cos` (as `get_distance_to_latlng`).
///
/// Under contract: `asin_contract`, `atan2_contract`, `sin_cos_contract`.
///
/// **Not proven:** times out at 60 minutes weekly (2026-10-27; four `distance_from_segment`s), a tool limit listed in
/// `known-timeouts.txt`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::sin_cos, crate::stubs::sin_cos_contract)]
fn c_fg_get_distance() {
    let (a, b) = (any_rect(), any_rect());
    kani::assume(!rect_spec::empty(a) && !rect_spec::empty(b));
    let d = a.get_distance(b);
    kani::cover!(d.radians() > 0.0, "apart");
}

/// C-s2::Rect::get_hausdorff_distance and get_directed_hausdorff_distance: complete for every
/// pair of valid rectangles, under the contracts of `asin`, `atan2` and `sin_cos` (as
/// `get_distance_to_latlng`).
///
/// Under contract: `asin_contract`, `atan2_contract`, `sin_cos_contract`.
///
/// **Not proven:** times out at 60 minutes weekly (2026-10-27; the directed distances in both directions), a tool limit listed in
/// `known-timeouts.txt`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::sin_cos, crate::stubs::sin_cos_contract)]
fn c_fg_hausdorff() {
    let (a, b) = (any_rect(), any_rect());
    let d = (
        a.get_hausdorff_distance(b),
        a.get_directed_hausdorff_distance(b),
    );
    kani::cover!(d.1.radians() > 0.0, "apart");
}

/// C-Region for s2::Rect::contains_cell and intersects_cell: complete for every valid rectangle
/// and every valid cell, under `atan2`'s contract (the cell's `rect_bound`).
///
/// Under contract: `atan2_contract`. Stubs: `any_uv_bound`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_region_cells() {
    let r = any_rect();
    let (c, _) = crate::value_domain::any_cell();
    let x = (
        Region::contains_cell(&r, &c),
        Region::intersects_cell(&r, &c),
    );
    kani::cover!(x.1 && !x.0, "intersecting, not containing");
}

/// N-s2::Rect::get_distance_to_latlng: in debug builds, panics for every empty valid rectangle and
/// every point, and for every valid rectangle and every invalid point, NaN included (its two
/// `debug_assert!`s, first).
///
/// Stubs: `any_segment_distance`, for the code after the checks, which no input reaches (an
/// any-value stub cannot hide an input that returns).
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::stub(
    s2rst::s2::edge_distances::distance_from_segment,
    crate::stubs::any_segment_distance
)]
fn n_get_distance_to_latlng() {
    let r = any_rect();
    let (lat, lng): (f64, f64) = (kani::any(), kani::any());
    kani::assume(rect_spec::empty(r) || !rect_spec::ll_valid(lat, lng));
    let _ = r.get_distance_to_latlng(LatLng::from_radians(lat, lng));
    kani::cover!(true, "returns");
}

/// N-s2::Rect::get_distance: in debug builds, panics for every pair of valid rectangles of which
/// one is empty.
///
/// Stubs: `any_segment_distance`, as `n_get_distance_to_latlng`.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::stub(
    s2rst::s2::edge_distances::distance_from_segment,
    crate::stubs::any_segment_distance
)]
fn n_get_distance() {
    let (a, b) = (any_rect(), any_rect());
    kani::assume(rect_spec::empty(a) || rect_spec::empty(b));
    let _ = a.get_distance(b);
    kani::cover!(true, "returns");
}
