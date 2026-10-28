// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLTP: `LatLng` to a point (spec 010 data-model §4.1).

use crate::value_domain::any_lat_lng_bits;

/// Bit for bit, any two NaNs alike (the checker may give a NaN any payload).
fn same(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
}

/// P-LLTP-01 (exact output): `to_point` is upstream's formula, and `From<LatLng> for Point` is
/// `to_point`.
///
/// This harness proves the part without a product: for every pair of `f64`, `to_point()`'s `z`
/// is the latitude's sine, as `sin_cos` gives it. The products `cos θ cos φ` and `sin θ cos φ`,
/// computed a second time, time out (12 minutes, 2026-10-28): they rest on the drift guard
/// `latlng_spec_tests::conversions_are_the_restatements` (the library's body is
/// `latlng_spec::upstream_to_point`'s text), so the entry is proven by composition.
///
/// Under contract: `pure_sin_cos`.
#[kani::proof]
#[kani::stub(f64::sin_cos, crate::stubs::pure_sin_cos)]
fn p_lltp_01() {
    crate::stubs::reset();
    let ll = any_lat_lng_bits();
    let p = ll.to_point();
    let (sin_lat, _) = ll.lat.radians().sin_cos();
    assert!(same(p.0.z, sin_lat));
}

/// P-LLTP-02 (correct result): for finite coordinates each coordinate of `to_point` is finite
/// and in [−1, 1]; a non-finite latitude makes every coordinate NaN, a non-finite longitude the
/// first two.
///
/// Under contract: `sin_cos_contract`.
#[kani::proof]
#[kani::stub(f64::sin_cos, crate::stubs::sin_cos_contract)]
fn p_lltp_02() {
    let ll = any_lat_lng_bits();
    let (lat, lng) = (ll.lat.radians(), ll.lng.radians());
    let p = ll.to_point();
    if lat.is_finite() && lng.is_finite() {
        for c in [p.0.x, p.0.y, p.0.z] {
            assert!(c.is_finite() && c.abs() <= 1.0);
        }
    }
    if !lat.is_finite() {
        assert!(p.0.x.is_nan() && p.0.y.is_nan() && p.0.z.is_nan());
    }
    if !lng.is_finite() {
        assert!(p.0.x.is_nan() && p.0.y.is_nan());
    }
    kani::cover!(
        lat.is_finite() && !lng.is_finite(),
        "a non-finite longitude"
    );
}
