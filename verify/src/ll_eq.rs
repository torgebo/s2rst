// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLEQ: `LatLng`'s equality (spec 010 data-model §4.1).

use crate::laws;
use crate::value_domain::any_lat_lng_bits;

/// P-LLEQ-01 (law): `==` is coordinate-wise `f64` equality, and an equivalence on pairs without
/// NaN (004's `laws::eq`).
///
/// Exactly: `(a == b) == (a.lat == b.lat && a.lng == b.lng)` (as `f64`) for every two pairs;
/// `laws::eq::all(a, b, c)` for every three pairs without NaN.
#[kani::proof]
fn p_lleq_01() {
    let (a, b, c) = (any_lat_lng_bits(), any_lat_lng_bits(), any_lat_lng_bits());
    assert!((a == b) == (a.lat.radians() == b.lat.radians() && a.lng.radians() == b.lng.radians()));
    for ll in [a, b, c] {
        kani::assume(!ll.lat.radians().is_nan() && !ll.lng.radians().is_nan());
    }
    laws::eq::all(&a, &b, &c);
    kani::cover!(a == b && b == c, "three equal pairs");
}
