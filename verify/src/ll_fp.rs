// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLFP: `LatLng` from a point (spec 010 data-model §4.1).

use s2rst::s2::LatLng;

use crate::latlng_spec;
use crate::value_domain::any_triple;

/// Bit for bit, any two NaNs alike (the checker may give a NaN any payload).
fn same(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
}

/// P-LLFP-01 (exact output): `latitude` and `longitude` are upstream's formulas, `+ 0.0`
/// included, and `from_point` and `From<Point>` are the pair.
///
/// This harness proves the longitude's half: for every triple of `f64`, `longitude(p)` and
/// `from_point(p).lng` are `latlng_spec::upstream_longitude`, bit for bit (any two NaNs alike).
/// The latitude's half recomputes `x² + y²`, and a second copy of the products times out (12
/// minutes, 2026-10-28; spec 008's identical restatements did too): it rests on the drift guard
/// `latlng_spec_tests::conversions_are_the_restatements`, which checks that the library's
/// `latitude`, `from_point` and `From<Point>` are the restatement's text. Rust evaluates
/// identical expressions identically (no contraction, `TRUSTED_BASE.md`, "The build"), so the
/// entry is proven by composition.
///
/// Under contract: `pure_atan2` (one value per pair, so the restatement's call gets the
/// library's).
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::pure_atan2)]
fn p_llfp_01() {
    crate::stubs::reset();
    let p = any_triple();
    let lng = latlng_spec::upstream_longitude(p.0.x, p.0.y);
    assert!(same(LatLng::longitude(p).radians(), lng));
    assert!(same(LatLng::from_point(p).lng.radians(), lng));
}

/// P-LLFP-02 (validity): `from_point(p)` is valid exactly when no coordinate of `p` is NaN, the
/// zero vector and the infinities included (spec 010 K2, extended to every non-NaN triple).
///
/// Exactly: `from_point(p).is_valid() == !(x, y or z is NaN)` for every triple of `f64`.
///
/// Under contract: `atan2_contract`.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn p_llfp_02() {
    let p = any_triple();
    let nan = p.0.x.is_nan() || p.0.y.is_nan() || p.0.z.is_nan();
    let ll = LatLng::from_point(p);
    assert!(latlng_spec::valid(ll.lat.radians(), ll.lng.radians()) == !nan);
    kani::cover!(p.0.z.is_infinite() && !nan, "an infinite coordinate");
    kani::cover!(nan, "a NaN coordinate");
}
