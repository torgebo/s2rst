// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A point on the unit sphere as a pair of latitude-longitude coordinates.
//!
//! Corresponds to C++ `S2LatLng`, Go `s2.LatLng`, Java `S2LatLng`.
//!
//! This is a mathematical abstraction; functions specific to Earth's geometry
//! (e.g. easting/northing conversions) belong elsewhere.

use crate::r3::Vector;
use crate::s1::Angle;
use crate::s2::Point;
use std::f64::consts::{FRAC_PI_2, PI};
use std::fmt;
use std::ops::{Add, Mul, Sub};

/// A point on the unit sphere as a (latitude, longitude) pair.
///
/// Latitude is measured from the equator towards the poles (range [-π/2, π/2]
/// for valid values). Longitude is measured from the prime meridian (range
/// [-π, π] for valid values).
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s2::LatLng;
///
/// // Create from degrees
/// let paris = LatLng::from_degrees(48.8566, 2.3522);
/// assert!(paris.is_valid());
///
/// // Convert to Point and back
/// let point = paris.to_point();
/// let back = LatLng::from_point(point);
/// assert!((back.lat.degrees() - 48.8566).abs() < 1e-13);
///
/// // Distance between two locations
/// let london = LatLng::from_degrees(51.5074, -0.1278);
/// let dist = paris.get_distance(london);
/// assert!(dist.degrees() > 3.0 && dist.degrees() < 4.0);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LatLng {
    /// Latitude angle.
    pub lat: Angle,
    /// Longitude angle.
    pub lng: Angle,
}

impl LatLng {
    /// Creates a `LatLng` from two angles. The values are allowed to be outside
    /// the `is_valid()` range, but most methods expect normalized values.
    #[inline]
    pub fn new(lat: Angle, lng: Angle) -> Self {
        LatLng { lat, lng }
    }

    /// Creates a `LatLng` from values in radians.
    #[inline]
    pub fn from_radians(lat_radians: f64, lng_radians: f64) -> Self {
        LatLng {
            lat: Angle::from_radians(lat_radians),
            lng: Angle::from_radians(lng_radians),
        }
    }

    /// Creates a `LatLng` from values in degrees.
    #[inline]
    pub fn from_degrees(lat_degrees: f64, lng_degrees: f64) -> Self {
        LatLng {
            lat: Angle::from_degrees(lat_degrees),
            lng: Angle::from_degrees(lng_degrees),
        }
    }

    /// Creates a `LatLng` from E5 representation (degrees × 10⁵).
    #[inline]
    pub fn from_e5(lat_e5: i32, lng_e5: i32) -> Self {
        LatLng {
            lat: Angle::from_e5(lat_e5),
            lng: Angle::from_e5(lng_e5),
        }
    }

    /// Creates a `LatLng` from E6 representation (degrees × 10⁶).
    #[inline]
    pub fn from_e6(lat_e6: i32, lng_e6: i32) -> Self {
        LatLng {
            lat: Angle::from_e6(lat_e6),
            lng: Angle::from_e6(lng_e6),
        }
    }

    /// Creates a `LatLng` from E7 representation (degrees × 10⁷).
    #[inline]
    pub fn from_e7(lat_e7: i32, lng_e7: i32) -> Self {
        LatLng {
            lat: Angle::from_e7(lat_e7),
            lng: Angle::from_e7(lng_e7),
        }
    }

    /// Creates a `LatLng` from unsigned E6 representation.
    #[inline]
    pub fn from_unsigned_e6(lat_e6: u32, lng_e6: u32) -> Self {
        LatLng {
            lat: Angle::from_unsigned_e6(lat_e6),
            lng: Angle::from_unsigned_e6(lng_e6),
        }
    }

    /// Creates a `LatLng` from unsigned E7 representation.
    #[inline]
    pub fn from_unsigned_e7(lat_e7: u32, lng_e7: u32) -> Self {
        LatLng {
            lat: Angle::from_unsigned_e7(lat_e7),
            lng: Angle::from_unsigned_e7(lng_e7),
        }
    }

    /// Returns a `LatLng` for which `is_valid()` will return false.
    #[inline]
    pub fn invalid() -> Self {
        LatLng::from_radians(PI, 2.0 * PI)
    }

    /// Computes the latitude of a direction vector (not necessarily unit length).
    ///
    /// Uses `atan2` for accuracy near the poles. The `+ 0.0` ensures that
    /// negative zeros are converted to positive zeros for consistent results.
    #[inline]
    pub fn latitude(p: Point) -> Angle {
        Angle::from_radians((p.z() + 0.0).atan2((p.x() * p.x() + p.y() * p.y()).sqrt()))
    }

    /// Computes the longitude of a direction vector (not necessarily unit length).
    ///
    /// The `+ 0.0` ensures that negative zeros are converted to positive zeros.
    #[inline]
    pub fn longitude(p: Point) -> Angle {
        Angle::from_radians((p.y() + 0.0).atan2(p.x() + 0.0))
    }

    /// Creates a `LatLng` from a direction vector (not necessarily unit length).
    pub fn from_point(p: Point) -> Self {
        LatLng {
            lat: Self::latitude(p),
            lng: Self::longitude(p),
        }
    }

    /// Returns true if the latitude is in [-π/2, π/2] and the longitude is
    /// in [-π, π].
    #[inline]
    pub fn is_valid(self) -> bool {
        self.lat.radians().abs() <= FRAC_PI_2 && self.lng.radians().abs() <= PI
    }

    /// Clamps the latitude to [-π/2, π/2] and reduces the longitude to
    /// [-π, π] using IEEE remainder. Returns `invalid()` if not finite.
    pub fn normalized(self) -> Self {
        let lat_rad = self.lat.radians();
        let lng_rad = self.lng.radians();
        if !lat_rad.is_finite() || !lng_rad.is_finite() {
            return Self::invalid();
        }
        // C++ uses std::remainder(lng, 2*PI) which returns the IEEE remainder
        // in [-PI, PI]. Rust equivalent: x - round(x/y)*y.
        let lng_normalized = lng_rad - (lng_rad / (2.0 * PI)).round() * (2.0 * PI);
        LatLng::from_radians(lat_rad.clamp(-FRAC_PI_2, FRAC_PI_2), lng_normalized)
    }

    /// Converts this `LatLng` to the equivalent unit-length `S2Point`.
    ///
    /// The maximum error in the result is 1.5 * `f64::EPSILON`.
    /// Requires latitude and longitude to be finite.
    pub fn to_point(self) -> Point {
        let (sin_lat, cos_lat) = self.lat.sin_cos();
        let (sin_lng, cos_lng) = self.lng.sin_cos();
        Point(Vector {
            x: cos_lng * cos_lat,
            y: sin_lng * cos_lat,
            z: sin_lat,
        })
    }

    /// Returns the surface distance to the given `LatLng`, using the Haversine
    /// formula.
    ///
    /// This is equivalent to `self.to_point().distance(other.to_point())` but
    /// slightly faster. It is less accurate for distances approaching 180°
    /// (about 8 digits of precision vs 15 for the Point-based approach).
    ///
    /// Both `LatLngs` must be normalized.
    pub fn get_distance(self, other: LatLng) -> Angle {
        let lat1 = self.lat.radians();
        let lat2 = other.lat.radians();
        let lng1 = self.lng.radians();
        let lng2 = other.lng.radians();
        let dlat = (0.5 * (lat2 - lat1)).sin();
        let dlng = (0.5 * (lng2 - lng1)).sin();
        let x = dlat * dlat + dlng * dlng * lat1.cos() * lat2.cos();
        Angle::from_radians(2.0 * x.sqrt().min(1.0).asin())
    }

    /// Reports whether the coordinates of two `LatLngs` are close, within the
    /// given tolerance applied independently to latitude and longitude.
    ///
    /// Note: this operates in rectangular lat/lng space and does not reflect
    /// closeness on the sphere near the poles. Use `get_distance()` for that.
    #[inline]
    pub fn approx_eq(self, other: LatLng) -> bool {
        self.approx_eq_with(other, Angle::from_radians(1e-15))
    }

    /// Like [`approx_eq`](LatLng::approx_eq) but with a configurable tolerance.
    #[inline]
    pub fn approx_eq_with(self, other: LatLng, max_error: Angle) -> bool {
        (self.lat - other.lat).abs().radians() <= max_error.radians()
            && (self.lng - other.lng).abs().radians() <= max_error.radians()
    }

    /// Exports the latitude and longitude in degrees, separated by a comma.
    /// Values are normalized before formatting.
    pub fn to_string_in_degrees(self) -> String {
        let n = self.normalized();
        format!("{:.6},{:.6}", n.lat.degrees(), n.lng.degrees())
    }
}

// --- Arithmetic operator impls ---

impl Add for LatLng {
    type Output = LatLng;
    #[inline]
    fn add(self, rhs: LatLng) -> LatLng {
        LatLng {
            lat: self.lat + rhs.lat,
            lng: self.lng + rhs.lng,
        }
    }
}

impl Sub for LatLng {
    type Output = LatLng;
    #[inline]
    fn sub(self, rhs: LatLng) -> LatLng {
        LatLng {
            lat: self.lat - rhs.lat,
            lng: self.lng - rhs.lng,
        }
    }
}

impl Mul<LatLng> for f64 {
    type Output = LatLng;
    #[inline]
    fn mul(self, rhs: LatLng) -> LatLng {
        LatLng {
            lat: rhs.lat * self,
            lng: rhs.lng * self,
        }
    }
}

impl Mul<f64> for LatLng {
    type Output = LatLng;
    #[inline]
    fn mul(self, rhs: f64) -> LatLng {
        LatLng {
            lat: self.lat * rhs,
            lng: self.lng * rhs,
        }
    }
}

impl Default for LatLng {
    /// The default `LatLng` has latitude and longitude both equal to zero.
    fn default() -> Self {
        LatLng::from_radians(0.0, 0.0)
    }
}

impl fmt::Display for LatLng {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lat, self.lng)
    }
}

impl From<Point> for LatLng {
    fn from(p: Point) -> Self {
        LatLng::from_point(p)
    }
}

impl From<LatLng> for Point {
    fn from(ll: LatLng) -> Self {
        ll.to_point()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn latlng_is_send_sync() {
        is_send_sync::<LatLng>();
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    // --- TestBasic (from C++) ---

    #[test]
    fn test_basic() {
        let ll_rad = LatLng::from_radians(FRAC_PI_4, FRAC_PI_2);
        assert_eq!(ll_rad.lat.radians(), FRAC_PI_4);
        assert_eq!(ll_rad.lng.radians(), FRAC_PI_2);
        assert!(ll_rad.is_valid());

        let ll_deg = LatLng::from_degrees(45.0, 90.0);
        assert_eq!(ll_rad, ll_deg);
        assert!(ll_deg.is_valid());

        assert!(!LatLng::from_degrees(-91.0, 0.0).is_valid());
        assert!(!LatLng::from_degrees(0.0, 181.0).is_valid());

        // Normalization
        let bad = LatLng::from_degrees(120.0, 200.0);
        assert!(!bad.is_valid());
        let better = bad.normalized();
        assert!(better.is_valid());
        assert_eq!(better.lat, Angle::from_degrees(90.0));
        assert!(
            float64_near(
                better.lng.radians(),
                Angle::from_degrees(-160.0).radians(),
                1e-15,
            ),
            "lng = {}, want {}",
            better.lng.degrees(),
            -160.0,
        );

        let bad = LatLng::from_degrees(-100.0, -360.0);
        assert!(!bad.is_valid());
        let better = bad.normalized();
        assert!(better.is_valid());
        assert_eq!(better.lat, Angle::from_degrees(-90.0));
        assert!(
            float64_near(better.lng.radians(), 0.0, 1e-15),
            "lng = {}, want 0",
            better.lng.degrees(),
        );

        // Arithmetic
        assert!(
            (LatLng::from_degrees(10.0, 20.0) + LatLng::from_degrees(20.0, 30.0))
                .approx_eq(LatLng::from_degrees(30.0, 50.0))
        );
        assert!(
            (LatLng::from_degrees(10.0, 20.0) - LatLng::from_degrees(20.0, 30.0))
                .approx_eq(LatLng::from_degrees(-10.0, -10.0))
        );
        assert!(
            (0.5 * LatLng::from_degrees(10.0, 20.0)).approx_eq(LatLng::from_degrees(5.0, 10.0))
        );

        // Invalid() returns an invalid point.
        assert!(!LatLng::invalid().is_valid());

        // Default constructor sets lat and lng to 0.
        let default_ll = LatLng::default();
        assert!(default_ll.is_valid());
        assert_eq!(default_ll.lat.radians(), 0.0);
        assert_eq!(default_ll.lng.radians(), 0.0);
    }

    // --- TestConversion (from C++) ---

    #[test]
    fn test_conversion() {
        // Test special cases: poles, "date line"
        assert!(float64_near(
            LatLng::from_point(LatLng::from_degrees(90.0, 65.0).to_point())
                .lat
                .degrees(),
            90.0,
            1e-14,
        ));

        assert_eq!(
            LatLng::from_point(LatLng::from_radians(-FRAC_PI_2, 1.0).to_point())
                .lat
                .radians(),
            -FRAC_PI_2,
        );

        assert!(float64_near(
            LatLng::from_point(LatLng::from_degrees(12.2, 180.0).to_point())
                .lng
                .degrees()
                .abs(),
            180.0,
            1e-14,
        ));

        assert_eq!(
            LatLng::from_point(LatLng::from_radians(0.1, -PI).to_point())
                .lng
                .radians()
                .abs(),
            PI,
        );
    }

    // --- TestConversion roundtrip (from Go) ---

    #[test]
    fn test_point_conversion_roundtrip() {
        let cases = [
            (0.0, 0.0),
            (90.0, 0.0),
            (-90.0, 0.0),
            (0.0, 180.0),
            (0.0, -180.0),
            (90.0, 180.0),
            (-90.0, -180.0),
            (-81.82750430354997, 151.19796752929685),
        ];

        for (lat, lng) in &cases {
            let ll = LatLng::from_degrees(*lat, *lng);
            let p = ll.to_point();
            let ll2 = LatLng::from_point(p);
            let is_polar = *lat == 90.0 || *lat == -90.0;
            assert!(
                float64_near(ll2.lat.degrees(), *lat, 1e-13),
                "lat roundtrip ({lat}, {lng}): got {}, want {lat}",
                ll2.lat.degrees(),
            );
            if !is_polar {
                assert!(
                    float64_near(ll2.lng.degrees(), *lng, 1e-13),
                    "lng roundtrip ({lat}, {lng}): got {}, want {lng}",
                    ll2.lng.degrees(),
                );
            }
        }
    }

    // --- NegativeZeros (from C++) ---

    #[test]
    fn test_negative_zeros() {
        fn is_identical(x: f64, y: f64) -> bool {
            x == y && x.is_sign_positive() == y.is_sign_positive()
        }

        // atan2(0.0, sqrt(1+0)) should give +0.0
        assert!(
            is_identical(
                LatLng::latitude(Point(Vector {
                    x: 1.0,
                    y: 0.0,
                    z: -0.0
                }))
                .radians(),
                0.0,
            ),
            "Latitude(1, 0, -0) should be +0.0",
        );

        // atan2(+0.0, 1.0) should give +0.0
        assert!(
            is_identical(
                LatLng::longitude(Point(Vector {
                    x: 1.0,
                    y: -0.0,
                    z: 0.0
                }))
                .radians(),
                0.0,
            ),
            "Longitude(1, -0, 0) should be +0.0",
        );

        // atan2(+0.0, -1.0) should give π
        assert!(
            is_identical(
                LatLng::longitude(Point(Vector {
                    x: -1.0,
                    y: -0.0,
                    z: 0.0
                }))
                .radians(),
                PI,
            ),
            "Longitude(-1, -0, 0) should be π",
        );

        // atan2(0.0, +0.0) should give +0.0
        assert!(
            is_identical(
                LatLng::longitude(Point(Vector {
                    x: -0.0,
                    y: 0.0,
                    z: 1.0
                }))
                .radians(),
                0.0,
            ),
            "Longitude(-0, 0, 1) should be +0.0",
        );

        assert!(
            is_identical(
                LatLng::longitude(Point(Vector {
                    x: -0.0,
                    y: -0.0,
                    z: 1.0
                }))
                .radians(),
                0.0,
            ),
            "Longitude(-0, -0, 1) should be +0.0",
        );
    }

    // --- InfIsInvalid (from C++) ---

    #[test]
    fn test_inf_is_invalid() {
        assert!(!LatLng::from_degrees(f64::INFINITY, -122.0).is_valid());
        assert!(!LatLng::from_degrees(37.0, f64::INFINITY).is_valid());

        // Also check .normalized()
        assert!(
            !LatLng::from_degrees(f64::INFINITY, -122.0)
                .normalized()
                .is_valid()
        );
        assert!(
            !LatLng::from_degrees(37.0, f64::INFINITY)
                .normalized()
                .is_valid()
        );
    }

    // --- NanIsInvalid (from C++) ---

    #[test]
    fn test_nan_is_invalid() {
        assert!(!LatLng::from_degrees(f64::NAN, -122.0).is_valid());
        assert!(!LatLng::from_degrees(37.0, f64::NAN).is_valid());

        // Also check .normalized()
        assert!(!LatLng::from_degrees(37.0, f64::NAN).normalized().is_valid());
        assert!(
            !LatLng::from_degrees(f64::NAN, -122.0)
                .normalized()
                .is_valid()
        );
    }

    // --- TestDistance (from C++) ---

    #[test]
    fn test_distance() {
        assert_eq!(
            LatLng::from_degrees(90.0, 0.0)
                .get_distance(LatLng::from_degrees(90.0, 0.0))
                .radians(),
            0.0,
        );
        assert!(float64_near(
            LatLng::from_degrees(-37.0, 25.0)
                .get_distance(LatLng::from_degrees(-66.0, -155.0))
                .degrees(),
            77.0,
            1e-13,
        ));
        assert!(float64_near(
            LatLng::from_degrees(0.0, 165.0)
                .get_distance(LatLng::from_degrees(0.0, -80.0))
                .degrees(),
            115.0,
            1e-13,
        ));
        assert!(float64_near(
            LatLng::from_degrees(47.0, -127.0)
                .get_distance(LatLng::from_degrees(-47.0, 53.0))
                .degrees(),
            180.0,
            2e-6,
        ));
    }

    // --- TestNormalized (from Go) ---

    #[test]
    fn test_normalized() {
        let cases = [
            (21.8275043, 151.1979675, 21.8275043, 151.1979675),
            (21.8275043, -151.1979675, 21.8275043, -151.1979675),
            (95.0, 151.1979675, 90.0, 151.1979675),
            (-95.0, 151.1979675, -90.0, 151.1979675),
            (21.8275043, 180.0, 21.8275043, 180.0),
            (21.8275043, -180.0, 21.8275043, -180.0),
            (21.8275043, 181.0012, 21.8275043, -178.9988),
            (21.8275043, -181.0012, 21.8275043, 178.9988),
            (256.0, 256.0, 90.0, -104.0),
        ];

        for (lat, lng, want_lat, want_lng) in &cases {
            let got = LatLng::from_degrees(*lat, *lng).normalized();
            assert!(
                got.is_valid(),
                "LatLng({lat}, {lng}).normalized() should be valid, got {got}",
            );
            let want = LatLng::from_degrees(*want_lat, *want_lng);
            assert!(
                got.get_distance(want).degrees() < 1e-13,
                "LatLng({lat}, {lng}).normalized() = ({}, {}), want ({want_lat}, {want_lng})",
                got.lat.degrees(),
                got.lng.degrees(),
            );
        }
    }

    // --- TestToString (from C++) ---

    #[test]
    fn test_to_string_in_degrees() {
        let cases: [(f64, f64, f64, f64); 6] = [
            (0.0, 0.0, 0.0, 0.0),
            (1.5, 91.7, 1.5, 91.7),
            (9.9, -0.31, 9.9, -0.31),
            (
                2.0_f64.sqrt(),
                -(5.0_f64.sqrt()),
                std::f64::consts::SQRT_2,
                -2.236_068,
            ),
            (91.3, 190.4, 90.0, -169.6),
            (-100.0, -710.0, -90.0, 10.0),
        ];

        for (lat, lng, expected_lat, expected_lng) in &cases {
            let p = LatLng::from_degrees(*lat, *lng);
            let output = p.to_string_in_degrees();
            let parts: Vec<&str> = output.split(',').collect();
            assert_eq!(parts.len(), 2, "output = {output}");
            let got_lat: f64 = parts[0].parse().unwrap();
            let got_lng: f64 = parts[1].parse().unwrap();
            assert!(
                float64_near(got_lat, *expected_lat, 1e-6),
                "lat: got {got_lat}, want {expected_lat} (input: {lat}, {lng})",
            );
            assert!(
                float64_near(got_lng, *expected_lng, 1e-6),
                "lng: got {got_lng}, want {expected_lng} (input: {lat}, {lng})",
            );
        }
    }

    // --- TestApproxEqual (from Go) ---

    #[test]
    fn test_approx_equal() {
        let eps = 1e-16; // smaller than default tolerance of 1e-15
        assert!(
            LatLng::from_degrees(30.0, 50.0)
                .approx_eq(LatLng::from_degrees(30.0, 50.0 + eps * (180.0 / PI)))
        );
        assert!(
            LatLng::from_degrees(30.0 - eps * (180.0 / PI), 50.0)
                .approx_eq(LatLng::from_degrees(30.0, 50.0))
        );
        assert!(!LatLng::from_degrees(1.0, 5.0).approx_eq(LatLng::from_degrees(2.0, 3.0)));
    }

    // --- Display ---

    #[test]
    fn test_display() {
        let ll = LatLng::from_degrees(45.0, 90.0);
        let s = format!("{ll}");
        assert!(s.contains("45.0"), "Display should contain lat: {s}");
        assert!(s.contains("90.0"), "Display should contain lng: {s}");
    }

    // --- From<Point> / From<LatLng> ---

    #[test]
    fn test_from_point_from_latlng() {
        let ll = LatLng::from_degrees(45.0, 90.0);
        let p: Point = ll.into();
        let ll2: LatLng = p.into();
        assert!(
            float64_near(ll2.lat.degrees(), 45.0, 1e-13),
            "lat roundtrip: {}",
            ll2.lat.degrees(),
        );
        assert!(
            float64_near(ll2.lng.degrees(), 90.0, 1e-13),
            "lng roundtrip: {}",
            ll2.lng.degrees(),
        );
    }

    // --- E5/E6/E7 constructors ---

    #[test]
    fn test_e5_e6_e7() {
        let ll = LatLng::from_e5(-4500000, 15000000);
        assert!(float64_near(ll.lat.degrees(), -45.0, 1e-10));
        assert!(float64_near(ll.lng.degrees(), 150.0, 1e-10));

        let ll = LatLng::from_e6(-60000000, 150000000);
        assert_eq!(ll.lat.radians(), Angle::from_degrees(-60.0).radians());
        assert_eq!(ll.lng.radians(), Angle::from_degrees(150.0).radians());

        let ll = LatLng::from_e7(750000000, -1200000000);
        assert_eq!(ll.lat.radians(), Angle::from_degrees(75.0).radians());
        assert_eq!(ll.lng.radians(), Angle::from_degrees(-120.0).radians());
    }

    #[test]
    fn test_from_unsigned_e6() {
        let ll = LatLng::from_unsigned_e6(48856600, 2352200);
        assert!((ll.lat.degrees() - 48.8566).abs() < 1e-6);
        assert!((ll.lng.degrees() - 2.3522).abs() < 1e-6);
    }

    #[test]
    fn test_from_unsigned_e7() {
        let ll = LatLng::from_unsigned_e7(488566000, 23522000);
        assert!((ll.lat.degrees() - 48.8566).abs() < 1e-7);
        assert!((ll.lng.degrees() - 2.3522).abs() < 1e-7);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    fn make_valid_ll(lat: f64, lng: f64) -> LatLng {
        let lat = clamp_finite(lat).clamp(-90.0, 90.0);
        let lng = clamp_finite(lng).clamp(-180.0, 180.0);
        LatLng::from_degrees(lat, lng)
    }

    #[quickcheck]
    fn prop_from_point_roundtrip(x: f64, y: f64, z: f64) -> bool {
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return true;
        }
        let p = Point::from_coords(x, y, z);
        let ll = LatLng::from_point(p);
        let p2 = ll.to_point();
        p.approx_eq(p2)
    }

    #[quickcheck]
    fn prop_latlng_from_point_valid(x: f64, y: f64, z: f64) -> bool {
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return true;
        }
        let p = Point::from_coords(x, y, z);
        LatLng::from_point(p).is_valid()
    }

    #[quickcheck]
    fn prop_normalized_is_valid(lat: f64, lng: f64) -> bool {
        let lat = clamp_finite(lat);
        let lng = clamp_finite(lng);
        LatLng::from_degrees(lat, lng).normalized().is_valid()
    }

    #[quickcheck]
    fn prop_distance_non_negative(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> bool {
        let a = make_valid_ll(lat1, lng1);
        let b = make_valid_ll(lat2, lng2);
        a.get_distance(b).radians() >= 0.0
    }

    #[quickcheck]
    fn prop_distance_symmetric(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> bool {
        let a = make_valid_ll(lat1, lng1);
        let b = make_valid_ll(lat2, lng2);
        (a.get_distance(b).radians() - b.get_distance(a).radians()).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_distance_self_zero(lat: f64, lng: f64) -> bool {
        let ll = make_valid_ll(lat, lng);
        ll.get_distance(ll).radians().abs() < 1e-14
    }

    #[quickcheck]
    fn prop_to_point_unit_length(lat: f64, lng: f64) -> bool {
        let ll = make_valid_ll(lat, lng);
        let p = ll.to_point();
        (p.0.norm() - 1.0).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_add_sub_inverse(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> bool {
        let a = make_valid_ll(lat1, lng1);
        let b = make_valid_ll(lat2, lng2);
        let result = (a + b) - b;
        (result.lat.radians() - a.lat.radians()).abs() < 1e-10
            && (result.lng.radians() - a.lng.radians()).abs() < 1e-10
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(lat: i32, lng: i32) -> bool {
        let ll = LatLng::from_degrees(f64::from(lat % 90) + 0.5, f64::from(lng % 180) + 0.5);
        let json1 = serde_json::to_string(&ll).unwrap();
        let back: LatLng = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: LatLng = serde_json::from_str(&json2).unwrap();
        back == back2
    }
}
