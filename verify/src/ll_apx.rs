// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLAPX: `LatLng`'s approximate equality (spec 010 data-model §4.1). Both harnesses are
//! weekly: the module is behind the `slow` feature (`lib.rs`), about eight minutes each
//! (2026-10-28).

use s2rst::s1::Angle;
use s2rst::s2::LatLng;

use crate::latlng_spec;
use crate::value_domain::any_lat_lng_bits;

/// Two pairs whose coordinate differences are not `∞ − ∞`.
fn any_pair() -> (LatLng, LatLng) {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    let eq_inf = |x: Angle, y: Angle| x.radians().is_infinite() && x.radians() == y.radians();
    kani::assume(!eq_inf(a.lat, b.lat) && !eq_inf(a.lng, b.lng));
    (a, b)
}

/// P-LLAPX-01 (correct result): `approx_eq_with` is upstream's `ApproxEquals` (`R2Point::aequal`,
/// both coordinates within the margin, inclusively), and `approx_eq` is it at 10⁻¹⁵.
///
/// Exactly: `a.approx_eq_with(b, m) == latlng_spec::upstream_approx_eq(…, m)` and
/// `a.approx_eq(b) == upstream_approx_eq(…, 1e-15)`, for every two pairs whose differences are
/// not `∞ − ∞` and every `f64` margin.
///
/// Weekly: 466.6 s (2026-10-28), over the per-change budget with P-LLAPX-02: both compute each
/// subtraction twice, the library's and the restatement's.
#[kani::proof]
fn p_llapx_01() {
    let (a, b) = any_pair();
    let m: f64 = kani::any();
    let up = |m: f64| {
        latlng_spec::upstream_approx_eq(
            a.lat.radians(),
            a.lng.radians(),
            b.lat.radians(),
            b.lng.radians(),
            m,
        )
    };
    assert!(a.approx_eq_with(b, Angle::from_radians(m)) == up(m));
    assert!(a.approx_eq(b) == up(1e-15));
    kani::cover!(a.approx_eq(b) && a != b, "close, not equal");
}

/// P-LLAPX-02 (law): `approx_eq_with` is symmetric for every input, and reflexive for finite
/// coordinates and a non-negative margin.
///
/// Exactly: `a.approx_eq_with(b, m) == b.approx_eq_with(a, m)` for every two pairs whose
/// differences are not `∞ − ∞` and every margin; `a.approx_eq_with(a, m)` for finite `a` and
/// `m ≥ 0`.
///
/// Weekly: 483.7 s (2026-10-28).
#[kani::proof]
fn p_llapx_02() {
    let (a, b) = any_pair();
    let m = Angle::from_radians(kani::any());
    assert!(a.approx_eq_with(b, m) == b.approx_eq_with(a, m));
    if a.lat.radians().is_finite() && a.lng.radians().is_finite() && m.radians() >= 0.0 {
        assert!(a.approx_eq_with(a, m));
    }
}
