// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLDST: `LatLng::get_distance` (spec 010 data-model §4.1).
//!
//! P-LLDST-01, the formula bit for bit, has no harness of its own: a second copy of the
//! haversine's products times out (12 minutes, 2026-10-28). It is proven by composition: the
//! drift guard `latlng_spec_tests::conversions_are_the_restatements` checks that the library's
//! `get_distance` is `latlng_spec::upstream_distance`'s text but for the clamp, written
//! `x.min(1.0)` where upstream has `std::min(1.0, x)`, and L-LL-02 (`ll_lem::l_ll_02`) proves the
//! two the same for every `x`, bit for bit.
//!
//! P-LLDST-04, the symmetry within a tolerance, has no harness: the formula multiplies the two
//! cosines in argument order, so `get_distance(a, b)` and `get_distance(b, a)` differ bit for
//! bit on about 8% of valid pairs, upstream's as the library's, and a tolerance needs the
//! functions' values, which the contracts do not fix. `prop_distance_symmetric` and the
//! `LATLNG_OPS` rows check it.

use std::f64::consts::PI;

use crate::value_domain::any_valid_lat_lng;

/// P-LLDST-02 (correct result): for valid inputs the distance is a number in [0, π].
///
/// Under contract: `sin_contract`, `cos_contract`, `asin_contract`.
#[kani::proof]
#[kani::stub(f64::sin, crate::stubs::sin_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn p_lldst_02() {
    let (a, b) = (any_valid_lat_lng(), any_valid_lat_lng());
    let d = a.get_distance(b).radians();
    assert!((0.0..=PI).contains(&d));
    kani::cover!(d > 3.0, "nearly antipodal");
}

/// P-LLDST-03 (correct result): the distance from a valid `LatLng` to itself is +0.
///
/// Under contract: `sin_contract`, `cos_contract`, `asin_contract`.
#[kani::proof]
#[kani::stub(f64::sin, crate::stubs::sin_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn p_lldst_03() {
    let a = any_valid_lat_lng();
    assert!(a.get_distance(a).radians().to_bits() == 0);
}
