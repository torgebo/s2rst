// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLARI: `LatLng`'s arithmetic (spec 010 data-model §4.1).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::LatLng;

use crate::value_domain::any_lat_lng_bits;

/// Bit for bit, any two NaNs alike (the checker may give a NaN any payload).
fn same(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
}

fn is(ll: LatLng, lat: f64, lng: f64) -> bool {
    same(ll.lat.radians(), lat) && same(ll.lng.radians(), lng)
}

/// P-LLARI-01 (exact output): `+`, `−` and both scalings are upstream's `R2Point` arithmetic on
/// the coordinates, bit for bit, and `k * ll` is `ll * k`.
///
/// Two members prove the sum and the difference: for every two pairs whose sums (differences)
/// are not `∞ − ∞`, `a + b` is `(a.lat + b.lat, a.lng + b.lng)` (the differences), as `f64`, any
/// two NaNs alike. The scalings, stated the same way, compute each product twice and timed out
/// at 12 minutes (2026-10-28): they rest on the drift guard
/// `latlng_spec_tests::scalings_are_angle_products` (both are `Angle * f64` on each coordinate,
/// which is `Angle(radians * k)`, spec 007's P-AGARI-01), so the entry is proven by composition.
#[kani::proof]
fn p_llari_01__add() {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    let (alat, alng, blat, blng) = (
        a.lat.radians(),
        a.lng.radians(),
        b.lat.radians(),
        b.lng.radians(),
    );
    kani::assume(!(alat.is_infinite() && alat == -blat));
    kani::assume(!(alng.is_infinite() && alng == -blng));
    assert!(is(a + b, alat + blat, alng + blng));
}

/// P-LLARI-01 (exact output), the difference: see [`p_llari_01__add`].
#[kani::proof]
fn p_llari_01__sub() {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    let (alat, alng, blat, blng) = (
        a.lat.radians(),
        a.lng.radians(),
        b.lat.radians(),
        b.lng.radians(),
    );
    kani::assume(!(alat.is_infinite() && alat == blat));
    kani::assume(!(alng.is_infinite() && alng == blng));
    assert!(is(a - b, alat - blat, alng - blng));
}
