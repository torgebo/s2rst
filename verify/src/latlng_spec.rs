// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a latitude–longitude (spec 010 data-model §1; research R4).
//!
//! Written from upstream's `s2latlng.{h,cc}` (`v0.14.0-40`). A `LatLng` is a pair of angles in
//! radians, any pair of numbers; it is valid when the latitude lies in [−π/2, π/2] and the
//! longitude in [−π, π]. Normalising clamps the latitude and moves the longitude by a whole
//! number of turns into [−π, π], upstream's `remainder(lng, 2π)`.
//!
//! The description reads a `LatLng` through its public `lat` and `lng` only; it never calls the
//! operations it describes (FR-007). Booleans are combined with `&&`, `||` and `==` only; there
//! is no `%` on floats (Kani's model of it is wrong, `KANI_ISSUES.md`). The remainder's
//! exactness is described by `core/src/test_util.rs`'s `num-bigint` remainder, which the
//! library's tests use; here only its correction step is copied ([`reduce_fmod_copy`]).
//!
//! The last section restates upstream's arithmetic (`upstream_*`). It is not the meaning: it
//! says what upstream computes, so that a property can state "bit for bit upstream's", and it is
//! checked against the C++ vectors (`tests/values_spec_vs_cpp.rs`, the `LATLNG_*` sections).

use std::f64::consts::{FRAC_PI_2, PI};

/// Upstream's `is_valid`: `|lat| ≤ π/2` and `|lng| ≤ π`; false when either is NaN.
pub fn valid(lat: f64, lng: f64) -> bool {
    -FRAC_PI_2 <= lat && lat <= FRAC_PI_2 && -PI <= lng && lng <= PI
}

/// `Normalized()`'s latitude for a finite `lat`: `max(−π/2, min(π/2, lat))`, as upstream's two
/// comparisons (`std::min(a, b)` is `b < a ? b : a`), signed zeros kept.
pub fn clamp_lat(lat: f64) -> f64 {
    let m = if lat < FRAC_PI_2 { lat } else { FRAC_PI_2 };
    if -FRAC_PI_2 < m { m } else { -FRAC_PI_2 }
}

/// Upstream's `Invalid()`: `(π, 2π)`.
pub const INVALID: (f64, f64) = (PI, 2.0 * PI);

/// A copy of `s1::angle::reduce_fmod` (`core/src/s1/angle.rs`), the step from `r = fmod(x, 2π)`
/// to the remainder of `x`, with `r4 = fmod(x, 4π)` passed in where the library computes
/// `x % (2.0 * y)`: `latlng_spec_tests`' drift guard compares the two bodies after that
/// substitution. P-LLNRM-03 is proven on it.
pub fn reduce_fmod_copy(r: f64, r4: f64) -> f64 {
    let y = 2.0 * PI;
    if r > PI {
        r - y
    } else if r < -PI {
        r + y
    } else if r == PI || r == -PI {
        if r4 == r { r } else { -r }
    } else {
        r
    }
}

// ---- Restatements of upstream's arithmetic ------------------------------------------------

/// `S2LatLng::Latitude(p)`: `atan2(p[2] + 0.0, sqrt(p[0]² + p[1]²))` (`s2latlng.h:235`).
pub fn upstream_latitude(x: f64, y: f64, z: f64) -> f64 {
    (z + 0.0).atan2((x * x + y * y).sqrt())
}

/// `S2LatLng::Longitude(p)`: `atan2(p[1] + 0.0, p[0] + 0.0)` (`s2latlng.h:244`).
pub fn upstream_longitude(x: f64, y: f64) -> f64 {
    (y + 0.0).atan2(x + 0.0)
}

/// `S2LatLng::ToPoint()`: `(cos θ cos φ, sin θ cos φ, sin φ)` from `SinCos` of the latitude φ
/// and the longitude θ (`s2latlng.cc:68–76`). Written with the library's names, so that
/// `latlng_spec_tests`' drift guard can compare the two texts (P-LLTP-01).
pub fn upstream_to_point(lat: f64, lng: f64) -> (f64, f64, f64) {
    let (sin_lat, cos_lat) = lat.sin_cos();
    let (sin_lng, cos_lng) = lng.sin_cos();
    (cos_lng * cos_lat, sin_lng * cos_lat, sin_lat)
}

/// `S2LatLng::GetDistance(o)`: the haversine, `2 asin(sqrt(min(1, x)))` with
/// `x = dlat² + dlng² · cos lat1 · cos lat2` and `d· = sin(0.5 (·2 − ·1))`
/// (`s2latlng.cc:85–109`). `std::min(1.0, x)` is `x < 1.0 ? x : 1.0`, so a NaN `x` gives 1.
pub fn upstream_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let dlat = (0.5 * (lat2 - lat1)).sin();
    let dlng = (0.5 * (lng2 - lng1)).sin();
    let x = dlat * dlat + dlng * dlng * lat1.cos() * lat2.cos();
    let m = if x < 1.0 { x } else { 1.0 };
    2.0 * m.sqrt().asin()
}

/// `S2LatLng::ApproxEquals(o, m)`: `R2Point::aequal`, `|Δlat| ≤ m && |Δlng| ≤ m`
/// (`s2latlng.h:158`).
pub fn upstream_approx_eq(alat: f64, alng: f64, blat: f64, blng: f64, m: f64) -> bool {
    (alat - blat).abs() <= m && (alng - blng).abs() <= m
}

#[cfg(test)]
#[path = "latlng_spec_tests.rs"]
mod latlng_spec_tests;
