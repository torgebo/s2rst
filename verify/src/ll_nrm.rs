// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLNRM: `LatLng::normalized` (spec 010 data-model §4.1; research R3).
//!
//! The longitude is `s1::angle::remainder_2pi`: `r = x % 2π` (IEEE `fmod`), then one correction
//! step. Kani's `%` is wrong (`KANI_ISSUES.md`), so the entries are proven in parts:
//!
//! - **P-LLNRM-01** (here): the latitude is the clamp, and a non-finite input gives `invalid()`.
//! - **P-LLNRM-03** (here): the correction step, on `latlng_spec::reduce_fmod_copy` (drift-guarded
//!   against `angle.rs`), lands in [−π, π], moves `r` by exactly 0 or ±2π, keeps a zero's sign,
//!   and at a tie keeps `r` exactly when `fmod(x, 4π) = r`, the even quotient.
//! - **P-LLNRM-02**, by composition: IEEE `fmod(x, 2π)` is exact, with the sign of `x` and
//!   `|r| < 2π` (trusted, `TRUSTED_BASE.md`), so `x − r` is a whole number of turns. P-LLNRM-03
//!   moves `r` by a whole number of turns into [−π, π]; the only values there congruent to `x` are
//!   the remainder, and at a tie both ±π, of which the step keeps the one whose quotient is even,
//!   as `fmod(x, 4π)` (exact) tells. So the longitude is `remainder(x, 2π)`, exactly.
//!   `latlng_tests::prop_normalized_is_the_exact_remainder` checks it against the exact
//!   `num-bigint` remainder at every binade, and the `LATLNG_NORM` rows against C++.
//! - **P-LLNRM-04**, by composition: for finite input the latitude is in [−π/2, π/2]
//!   (P-LLNRM-01) and the longitude in [−π, π] (P-LLNRM-02), so the result is valid; `invalid()`
//!   is not (P-LLVAL-01, P-LLCON-01).
//! - **P-LLNRM-05**, by composition: a valid `LatLng` has `|lng| ≤ π < 2π`, so `fmod` returns
//!   `lng` itself (trusted), P-LLNRM-03 keeps it (`|r| < π`, or a tie with `fmod(x, 4π) = x = r`),
//!   and the latitude's clamp keeps it (P-LLNRM-01): the identity, bit for bit.

use std::f64::consts::PI;

use crate::latlng_spec;
use crate::value_domain::any_lat_lng_bits;

/// P-LLNRM-01 (correct result): `normalized`'s latitude is upstream's clamp, bit for bit, and a
/// non-finite coordinate gives `invalid()`.
///
/// Exactly: for every pair of `f64`, if both are finite the result's latitude is
/// `latlng_spec::clamp_lat(lat)`, bit for bit; otherwise the result is `latlng_spec::INVALID`.
#[kani::proof]
fn p_llnrm_01() {
    let ll = any_lat_lng_bits();
    let (lat, lng) = (ll.lat.radians(), ll.lng.radians());
    let n = ll.normalized();
    if lat.is_finite() && lng.is_finite() {
        assert!(n.lat.radians().to_bits() == latlng_spec::clamp_lat(lat).to_bits());
    } else {
        let inv = latlng_spec::INVALID;
        assert!(n.lat.radians().to_bits() == inv.0.to_bits());
        assert!(n.lng.radians().to_bits() == inv.1.to_bits());
    }
    kani::cover!(lat > 2.0 && lng.is_finite(), "a clamped latitude");
}

/// P-LLNRM-03 (exact output): the correction step of the exact reduction.
///
/// Exactly: for every `r` with `|r| < 2π` and `r4` either `r` or `r + copysign(2π, r)` (the
/// two values `fmod(x, 4π)` can take, as the truncated quotient is even or odd), the result `n`
/// of `reduce_fmod_copy(r, r4)` has `|n| ≤ π`; `k = r − n` is 0, `2π` or `−2π` with
/// `n + k == r`; a zero `r` is returned bit for bit; and at `|r| = π`, `n == r` exactly when
/// `r4 == r`, else `n == −r`.
#[kani::proof]
fn p_llnrm_03() {
    let y = 2.0 * PI;
    let r: f64 = kani::any();
    kani::assume(r.abs() < y);
    let r4 = if kani::any() { r } else { r + y.copysign(r) };
    let n = latlng_spec::reduce_fmod_copy(r, r4);
    assert!(n.abs() <= PI);
    let k = r - n;
    assert!(k == 0.0 || k == y || k == -y);
    assert!(n + k == r);
    if r == 0.0 {
        assert!(n.to_bits() == r.to_bits());
    }
    if r.abs() == PI {
        assert!((n == r) == (r4 == r));
        assert!(n == r || n == -r);
    }
    kani::cover!(r.abs() == PI && n == -r, "an odd tie");
}
