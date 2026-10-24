// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle membership of a latitude–longitude (P-LMEM, spec 005 data-model §4.2), against
//! the rectangle description (`rect_spec`).

use s2rst::s2::LatLng;

use crate::rect_spec;
use crate::value_domain::{any_f64, any_rect, any_valid_lat_lng};

/// P-LMEM-01 (correct result): a latitude–longitude is in a rectangle exactly when it is
/// valid and in both of its intervals; an invalid one (out of range, or NaN) is in none (K2).
///
/// For every valid `r` and every two `f64` values `lat`, `lng`:
/// `r.contains_lat_lng(from_radians(lat, lng)) == rect_spec::member(r, lat, lng)`.
#[kani::proof]
fn p_lmem_01() {
    let r = any_rect();
    let (lat, lng) = (any_f64(), any_f64());
    let c = r.contains_lat_lng(LatLng::from_radians(lat, lng));
    assert_eq!(c, rect_spec::member(r, lat, lng));
    kani::cover!(
        c && lng.abs() == std::f64::consts::PI,
        "on the antimeridian"
    );
    kani::cover!(
        !c && r.is_full() && lng.is_finite(),
        "an out-of-range point is not in the full rectangle"
    );
}

/// P-LMEM-02 (correct result): a valid latitude–longitude is in the interior exactly when its
/// latitude is in the latitudes' interior and its longitude in the longitudes'.
///
/// For every valid `r` and valid `ll`:
/// `r.interior_contains_lat_lng(ll) == rect_spec::interior_member(r, ll.lat, ll.lng)`.
#[kani::proof]
fn p_lmem_02() {
    let r = any_rect();
    let ll = any_valid_lat_lng();
    let i = r.interior_contains_lat_lng(ll);
    assert_eq!(
        i,
        rect_spec::interior_member(r, ll.lat.radians(), ll.lng.radians())
    );
    kani::cover!(i, "in the interior");
    kani::cover!(!i && r.contains_lat_lng(ll), "on the boundary");
}
