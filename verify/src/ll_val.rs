// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLVAL: `LatLng`'s validity (spec 010 data-model §4.1).

use s2rst::s2::LatLng;

use crate::latlng_spec;
use crate::value_domain::any_lat_lng_bits;

/// P-LLVAL-01 (correct result): `is_valid` is upstream's predicate, the latitude in
/// [−π/2, π/2] and the longitude in [−π, π], for every pair; the default is valid.
///
/// Exactly: `ll.is_valid() == latlng_spec::valid(lat, lng)` for every pair of `f64`, NaN and
/// the infinities included; `LatLng::default().is_valid()`.
#[kani::proof]
fn p_llval_01() {
    let ll = any_lat_lng_bits();
    assert!(ll.is_valid() == latlng_spec::valid(ll.lat.radians(), ll.lng.radians()));
    assert!(LatLng::default().is_valid());
    kani::cover!(ll.is_valid(), "valid");
    kani::cover!(
        !ll.is_valid() && !ll.lat.radians().is_nan(),
        "invalid, no NaN"
    );
}
