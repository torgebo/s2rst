// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Earth modeled as a sphere.
//!
//! Convenience functions for converting between distances on the unit sphere
//! (expressed as angles) and distances on the Earth's surface. All functions
//! use a spherical Earth model with mean radius 6371.01 km (the radius of
//! the sphere with the same surface area as the WGS84 ellipsoid). This
//! introduces distance errors of up to 0.56% compared to the ellipsoid.
//!
//! # Examples
//!
//! ```
//! use s2rst::s2::earth;
//! use s2rst::s2::LatLng;
//!
//! // Distance between Paris and London.
//! let paris = LatLng::from_degrees(48.8566, 2.3522);
//! let london = LatLng::from_degrees(51.5074, -0.1278);
//! let km = earth::get_distance_km_latlng(paris, london);
//! assert!((km - 341.0).abs() < 5.0); // ~341 km
//!
//! // Convert a 10 km radius to an angular radius for use with Cap.
//! let angle = earth::km_to_angle(10.0);
//! assert!(angle.radians() > 0.0);
//!
//! // Convert steradians (unit-sphere area) to square kilometers.
//! let area_sr = 0.001; // about 40 000 km²
//! let area_km2 = earth::steradians_to_square_km(area_sr);
//! assert!(area_km2 > 40_000.0);
//! ```

use crate::s1::{Angle, ChordAngle};
use crate::s2::{LatLng, Point};

// ─── Distance newtypes ─────────────────────────────────────────────────

/// A distance in meters on the Earth's surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Meters(pub f64);

/// A distance in kilometers on the Earth's surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Kilometers(pub f64);

/// A solid angle in steradians.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Steradians(pub f64);

/// A solid angle in square meters on the Earth's surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct SquareMeters(pub f64);

/// A solid angle in square kilometers on the Earth's surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct SquareKilometers(pub f64);

// ─── Display ────────────────────────────────────────────────────────────

impl std::fmt::Display for Meters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} m", self.0)
    }
}

impl std::fmt::Display for Kilometers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} km", self.0)
    }
}

impl std::fmt::Display for Steradians {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} sr", self.0)
    }
}

impl std::fmt::Display for SquareMeters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} m²", self.0)
    }
}

impl std::fmt::Display for SquareKilometers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} km²", self.0)
    }
}

// ─── From<f64> / Into<f64> ─────────────────────────────────────────────

impl From<f64> for Meters {
    fn from(v: f64) -> Self {
        Meters(v)
    }
}

impl From<Meters> for f64 {
    fn from(m: Meters) -> Self {
        m.0
    }
}

impl From<f64> for Kilometers {
    fn from(v: f64) -> Self {
        Kilometers(v)
    }
}

impl From<Kilometers> for f64 {
    fn from(km: Kilometers) -> Self {
        km.0
    }
}

impl From<f64> for Steradians {
    fn from(v: f64) -> Self {
        Steradians(v)
    }
}

impl From<Steradians> for f64 {
    fn from(sr: Steradians) -> Self {
        sr.0
    }
}

impl From<f64> for SquareMeters {
    fn from(v: f64) -> Self {
        SquareMeters(v)
    }
}

impl From<SquareMeters> for f64 {
    fn from(m2: SquareMeters) -> Self {
        m2.0
    }
}

impl From<f64> for SquareKilometers {
    fn from(v: f64) -> Self {
        SquareKilometers(v)
    }
}

impl From<SquareKilometers> for f64 {
    fn from(km2: SquareKilometers) -> Self {
        km2.0
    }
}

// ─── Meters ↔ Kilometers cross-conversion ──────────────────────────────

impl From<Meters> for Kilometers {
    fn from(m: Meters) -> Kilometers {
        Kilometers(m.0 * 0.001)
    }
}

impl From<Kilometers> for Meters {
    fn from(km: Kilometers) -> Meters {
        Meters(km.0 * 1000.0)
    }
}

// ─── SquareMeters ↔ SquareKilometers cross-conversion ──────────────────

impl From<SquareMeters> for SquareKilometers {
    fn from(m2: SquareMeters) -> SquareKilometers {
        SquareKilometers(m2.0 * 1e-6)
    }
}

impl From<SquareKilometers> for SquareMeters {
    fn from(km2: SquareKilometers) -> SquareMeters {
        SquareMeters(km2.0 * 1e6)
    }
}

// ─── Angle / ChordAngle conversions ────────────────────────────────────

impl From<Meters> for Angle {
    fn from(m: Meters) -> Angle {
        meters_to_angle(m.0)
    }
}

impl From<Meters> for ChordAngle {
    fn from(m: Meters) -> ChordAngle {
        meters_to_chord_angle(m.0)
    }
}

impl From<Angle> for Meters {
    fn from(a: Angle) -> Meters {
        Meters(to_meters(a))
    }
}

impl From<ChordAngle> for Meters {
    fn from(ca: ChordAngle) -> Meters {
        Meters(chord_angle_to_meters(ca))
    }
}

impl From<Kilometers> for Angle {
    fn from(km: Kilometers) -> Angle {
        km_to_angle(km.0)
    }
}

impl From<Kilometers> for ChordAngle {
    fn from(km: Kilometers) -> ChordAngle {
        km_to_chord_angle(km.0)
    }
}

impl From<Angle> for Kilometers {
    fn from(a: Angle) -> Kilometers {
        Kilometers(to_km(a))
    }
}

impl From<ChordAngle> for Kilometers {
    fn from(ca: ChordAngle) -> Kilometers {
        Kilometers(chord_angle_to_km(ca))
    }
}

// ─── Area conversions ──────────────────────────────────────────────────

impl From<Steradians> for SquareMeters {
    fn from(sr: Steradians) -> SquareMeters {
        SquareMeters(steradians_to_square_meters(sr.0))
    }
}

impl From<Steradians> for SquareKilometers {
    fn from(sr: Steradians) -> SquareKilometers {
        SquareKilometers(steradians_to_square_km(sr.0))
    }
}

impl From<SquareMeters> for Steradians {
    fn from(m2: SquareMeters) -> Steradians {
        Steradians(square_meters_to_steradians(m2.0))
    }
}

impl From<SquareKilometers> for Steradians {
    fn from(km2: SquareKilometers) -> Steradians {
        Steradians(square_km_to_steradians(km2.0))
    }
}

// ─── Arithmetic ────────────────────────────────────────────────────────

macro_rules! impl_distance_arithmetic {
    ($T:ty, $unit:literal) => {
        impl std::ops::Add for $T {
            type Output = $T;
            fn add(self, rhs: $T) -> $T {
                <$T>::from(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $T {
            type Output = $T;
            fn sub(self, rhs: $T) -> $T {
                <$T>::from(self.0 - rhs.0)
            }
        }

        impl std::ops::Mul<f64> for $T {
            type Output = $T;
            fn mul(self, rhs: f64) -> $T {
                <$T>::from(self.0 * rhs)
            }
        }

        impl std::ops::Mul<$T> for f64 {
            type Output = $T;
            fn mul(self, rhs: $T) -> $T {
                <$T>::from(self * rhs.0)
            }
        }

        impl std::ops::Div<f64> for $T {
            type Output = $T;
            fn div(self, rhs: f64) -> $T {
                <$T>::from(self.0 / rhs)
            }
        }

        impl std::ops::Neg for $T {
            type Output = $T;
            fn neg(self) -> $T {
                <$T>::from(-self.0)
            }
        }
    };
}

impl_distance_arithmetic!(Meters, "m");
impl_distance_arithmetic!(Kilometers, "km");
impl_distance_arithmetic!(Steradians, "sr");
impl_distance_arithmetic!(SquareMeters, "m²");
impl_distance_arithmetic!(SquareKilometers, "km²");

/// Earth's mean radius in meters.
///
/// According to NASA, this value is 6371.01 ± 0.02 km. The equatorial
/// radius is 6378.136 km, and the polar radius is 6356.752 km.
pub const RADIUS_METERS: f64 = 6_371_010.0;

/// Earth's mean radius in kilometers.
pub const RADIUS_KM: f64 = 0.001 * RADIUS_METERS;

/// Altitude of the lowest known point (Challenger Deep), in meters.
pub const LOWEST_ALTITUDE_METERS: f64 = -10_898.0;

/// Altitude of the highest known point (Mount Everest), in meters.
pub const HIGHEST_ALTITUDE_METERS: f64 = 8_846.0;

// ─── Distance ↔ Angle conversions ──────────────────────────────────────

/// Converts meters to an [`Angle`].
pub fn meters_to_angle(meters: f64) -> Angle {
    Angle::from_radians(meters_to_radians(meters))
}

/// Converts meters to a [`ChordAngle`].
pub fn meters_to_chord_angle(meters: f64) -> ChordAngle {
    ChordAngle::from(meters_to_angle(meters))
}

/// Converts meters to radians.
pub fn meters_to_radians(meters: f64) -> f64 {
    meters / RADIUS_METERS
}

/// Converts an [`Angle`] to meters.
pub fn to_meters(angle: Angle) -> f64 {
    angle.radians() * RADIUS_METERS
}

/// Converts a [`ChordAngle`] to meters.
pub fn chord_angle_to_meters(cangle: ChordAngle) -> f64 {
    to_meters(Angle::from(cangle))
}

/// Converts radians to meters.
pub fn radians_to_meters(radians: f64) -> f64 {
    radians * RADIUS_METERS
}

/// Converts kilometers to an [`Angle`].
pub fn km_to_angle(km: f64) -> Angle {
    Angle::from_radians(km_to_radians(km))
}

/// Converts kilometers to a [`ChordAngle`].
pub fn km_to_chord_angle(km: f64) -> ChordAngle {
    ChordAngle::from(km_to_angle(km))
}

/// Converts kilometers to radians.
pub fn km_to_radians(km: f64) -> f64 {
    km / RADIUS_KM
}

/// Converts an [`Angle`] to kilometers.
pub fn to_km(angle: Angle) -> f64 {
    angle.radians() * RADIUS_KM
}

/// Converts a [`ChordAngle`] to kilometers.
pub fn chord_angle_to_km(cangle: ChordAngle) -> f64 {
    to_km(Angle::from(cangle))
}

/// Converts radians to kilometers.
pub fn radians_to_km(radians: f64) -> f64 {
    radians * RADIUS_KM
}

// ─── Area conversions ──────────────────────────────────────────────────

/// Converts square kilometers to steradians.
pub fn square_km_to_steradians(km2: f64) -> f64 {
    km2 / (RADIUS_KM * RADIUS_KM)
}

/// Converts square meters to steradians.
pub fn square_meters_to_steradians(m2: f64) -> f64 {
    m2 / (RADIUS_METERS * RADIUS_METERS)
}

/// Converts steradians to square kilometers.
pub fn steradians_to_square_km(steradians: f64) -> f64 {
    steradians * RADIUS_KM * RADIUS_KM
}

/// Converts steradians to square meters.
pub fn steradians_to_square_meters(steradians: f64) -> f64 {
    steradians * RADIUS_METERS * RADIUS_METERS
}

// ─── Longitude conversions ─────────────────────────────────────────────

/// Converts meters of east-west distance to radians of longitude at
/// the given latitude. Returns at most 2π.
pub fn meters_to_longitude_radians(meters: f64, latitude_radians: f64) -> f64 {
    let scalar = latitude_radians.cos();
    if scalar == 0.0 {
        return std::f64::consts::TAU;
    }
    (meters_to_radians(meters) / scalar).min(std::f64::consts::TAU)
}

/// Converts kilometers of east-west distance to radians of longitude.
pub fn km_to_longitude_radians(km: f64, latitude_radians: f64) -> f64 {
    meters_to_longitude_radians(1000.0 * km, latitude_radians)
}

// ─── Distance between points ───────────────────────────────────────────

/// Returns the distance in meters between two points on the sphere.
pub fn get_distance_meters_points(a: Point, b: Point) -> f64 {
    radians_to_meters(a.0.angle(b.0))
}

/// Returns the distance in meters between two lat/lng coordinates.
pub fn get_distance_meters_latlng(a: LatLng, b: LatLng) -> f64 {
    to_meters(a.get_distance(b))
}

/// Returns the distance in kilometers between two points on the sphere.
pub fn get_distance_km_points(a: Point, b: Point) -> f64 {
    radians_to_km(a.0.angle(b.0))
}

/// Returns the distance in kilometers between two lat/lng coordinates.
pub fn get_distance_km_latlng(a: LatLng, b: LatLng) -> f64 {
    to_km(a.get_distance(b))
}

// ─── Bearing ───────────────────────────────────────────────────────────

/// Returns the haversine of the angle in radians: `sin(x/2)^2`.
///
/// Numerically stable near zero (compared to `(1 - cos(x)) / 2`).
pub fn haversine(radians: f64) -> f64 {
    let sin_half = (radians / 2.0).sin();
    sin_half * sin_half
}

/// Returns the initial bearing from `a` to `b` (0° = north, clockwise).
///
/// If `a == b`, `a == -b`, or `a` is at a pole, the result is undefined.
pub fn get_initial_bearing(a: LatLng, b: LatLng) -> Angle {
    let lat1 = a.lat.radians();
    let cos_lat2 = b.lat.radians().cos();
    let lat_diff = b.lat.radians() - a.lat.radians();
    let lng_diff = b.lng.radians() - a.lng.radians();

    let x = lat_diff.sin() + lat1.sin() * cos_lat2 * 2.0 * haversine(lng_diff);
    let y = lng_diff.sin() * cos_lat2;
    Angle::from_radians(y.atan2(x))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_radius_constants() {
        assert!((RADIUS_KM - 6371.01).abs() < 0.01);
        assert!((RADIUS_METERS - 6_371_010.0).abs() < 1.0);
    }

    #[test]
    fn test_meters_roundtrip() {
        let meters = 1000.0;
        let angle = meters_to_angle(meters);
        let back = to_meters(angle);
        assert!((back - meters).abs() < EPSILON);
    }

    #[test]
    fn test_km_roundtrip() {
        let km = 42.0;
        let angle = km_to_angle(km);
        let back = to_km(angle);
        assert!((back - km).abs() < EPSILON);
    }

    #[test]
    fn test_radians_roundtrip() {
        let meters = 5000.0;
        let radians = meters_to_radians(meters);
        let back = radians_to_meters(radians);
        assert!((back - meters).abs() < EPSILON);
    }

    #[test]
    fn test_area_roundtrip() {
        let km2 = 100.0;
        let sr = square_km_to_steradians(km2);
        let back = steradians_to_square_km(sr);
        assert!((back - km2).abs() < EPSILON);
    }

    #[test]
    fn test_distance_between_points() {
        // Approximate distance from (0,0) to (0,1) in degrees
        let a = LatLng::from_degrees(0.0, 0.0);
        let b = LatLng::from_degrees(0.0, 1.0);
        let dist_km = get_distance_km_latlng(a, b);
        // At the equator, 1 degree of longitude ≈ 111.19 km
        assert!((dist_km - 111.19).abs() < 1.0);
    }

    #[test]
    fn test_distance_same_point() {
        let a = LatLng::from_degrees(45.0, 90.0);
        let dist = get_distance_meters_latlng(a, a);
        assert!(dist.abs() < 1e-6);
    }

    #[test]
    fn test_meters_to_longitude_radians_equator() {
        // At the equator, longitude meters = latitude meters
        let meters = RADIUS_METERS; // = 1 radian at equator
        let lng_rad = meters_to_longitude_radians(meters, 0.0);
        assert!((lng_rad - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_meters_to_longitude_radians_pole() {
        // At the pole, any distance covers all longitudes
        let meters = 1000.0;
        let lng_rad = meters_to_longitude_radians(meters, std::f64::consts::FRAC_PI_2);
        assert!((lng_rad - std::f64::consts::TAU).abs() < EPSILON);
    }

    #[test]
    fn test_chord_angle_conversion() {
        let meters = 10000.0;
        let ca = meters_to_chord_angle(meters);
        let back = chord_angle_to_meters(ca);
        assert!((back - meters).abs() < 0.01);
    }

    /// Table-driven port of C++ `TestGetInitialBearing` (all 8 cases, ≤ 0.01° tolerance).
    #[test]
    fn test_initial_bearing() {
        let cases: &[(&str, f64, f64, f64, f64, f64)] = &[
            ("eastward on equator", 0.0, 50.0, 0.0, 100.0, 90.0),
            ("westward on equator", 0.0, 50.0, 0.0, 0.0, -90.0),
            ("northward on meridian", 16.0, 28.0, 81.0, 28.0, 0.0),
            ("southward on meridian", 24.0, 64.0, -27.0, 64.0, 180.0),
            ("towards north pole", 12.0, 76.0, 90.0, 50.0, 0.0),
            ("towards south pole", -35.0, 105.0, -90.0, -120.0, 180.0),
            // Geodesic cross-checks (Spain ↔ Japan, C++ expected values)
            (
                "Spain to Japan",
                40.4379332,
                -3.749576,
                35.6733227,
                139.6403486,
                29.2,
            ),
            (
                "Japan to Spain",
                35.6733227,
                139.6403486,
                40.4379332,
                -3.749576,
                -27.2,
            ),
        ];
        for &(name, lat1, lng1, lat2, lng2, expected_deg) in cases {
            let a = LatLng::from_degrees(lat1, lng1);
            let b = LatLng::from_degrees(lat2, lng2);
            let bearing_deg = get_initial_bearing(a, b).degrees();
            let mut diff = (bearing_deg - expected_deg).abs();
            if diff > 180.0 {
                diff = 360.0 - diff;
            }
            assert!(
                diff <= 0.01,
                "get_initial_bearing({name}): expected {expected_deg}°, got {bearing_deg}°"
            );
        }
    }

    #[test]
    fn test_km_to_angle_and_back() {
        // 1 km → angle → back to km
        let km = 1.0;
        let angle = km_to_angle(km);
        let back = to_km(angle);
        assert!((back - km).abs() < 1e-10, "km roundtrip: {back} vs {km}");
    }

    #[test]
    fn test_km_to_chord_angle_and_back() {
        let km = 100.0;
        let ca = km_to_chord_angle(km);
        let back = chord_angle_to_km(ca);
        assert!(
            (back - km).abs() < 0.01,
            "km chord roundtrip: {back} vs {km}"
        );
    }

    #[test]
    fn test_area_sphere_surface() {
        // Full sphere area = 4π steradians = 4π × R² square meters
        let sphere_area_sr = 4.0 * std::f64::consts::PI;
        let sphere_km2 = steradians_to_square_km(sphere_area_sr);
        let expected_km2 = 4.0 * std::f64::consts::PI * (RADIUS_METERS / 1000.0).powi(2);
        assert!(
            (sphere_km2 - expected_km2).abs() / expected_km2 < 1e-10,
            "sphere area: {sphere_km2} vs {expected_km2}",
        );
    }

    #[test]
    fn test_distance_known_cities() {
        // NYC to London is approximately 5,570 km.
        let nyc = LatLng::from_degrees(40.7128, -74.0060);
        let london = LatLng::from_degrees(51.5074, -0.1278);
        let dist_km = get_distance_km_latlng(nyc, london);
        assert!(
            (dist_km - 5570.0).abs() < 50.0,
            "NYC-London distance: {dist_km} km, expected ~5570 km",
        );
    }

    /// Port of C++ `TestAngleConversion` — covers all double-precision overloads.
    #[test]
    fn test_angle_conversion_exact() {
        use std::f64::consts::PI;

        // Provably exact: a/a = 1.0 in IEEE 754.
        assert_eq!(meters_to_angle(RADIUS_METERS).radians(), 1.0);
        assert_eq!(km_to_angle(RADIUS_KM).radians(), 1.0);

        // Provably exact: identity multiplications.
        assert_eq!(to_km(Angle::from_radians(0.5)), 0.5 * RADIUS_KM);
        assert_eq!(radians_to_km(0.5), 0.5 * RADIUS_KM);

        // Chord angle roundtrips at 1 radian (sin/asin chain, within a few ULPs).
        assert!(
            (meters_to_chord_angle(RADIUS_METERS).radians() - 1.0).abs() < 1e-14,
            "meters_to_chord_angle(RADIUS_METERS).radians() != 1"
        );
        assert!(
            (km_to_chord_angle(RADIUS_KM).radians() - 1.0).abs() < 1e-14,
            "km_to_chord_angle(RADIUS_KM).radians() != 1"
        );

        // 0.5 radian chord angle → km (sin/asin roundtrip).
        let ca_km = chord_angle_to_km(ChordAngle::from_radians(0.5));
        assert!(
            (ca_km - 0.5 * RADIUS_KM).abs() < 1e-9,
            "chord_angle_to_km(0.5 rad) = {ca_km}, expected {}",
            0.5 * RADIUS_KM
        );

        // Antipodal chord angle: len2=4.0 → 2*asin(1) = PI.
        assert!(
            (chord_angle_to_meters(ChordAngle::from_degrees(180.0)) - RADIUS_METERS * PI).abs()
                < 1e-7,
            "chord_angle_to_meters(180°) should be PI * RADIUS_METERS"
        );

        // 180° angle → meters.
        assert!(
            (to_meters(Angle::from_degrees(180.0)) - RADIUS_METERS * PI).abs() < 1e-7,
            "to_meters(180°) should be PI * RADIUS_METERS"
        );

        // Cross-unit roundtrips.
        assert!(
            (radians_to_meters(km_to_radians(2.5)) - 2500.0).abs() < 1e-9,
            "radians_to_meters(km_to_radians(2.5)) should be 2500"
        );
        assert!(
            (meters_to_radians(radians_to_km(0.3) * 1000.0) - 0.3).abs() < 1e-14,
            "meters_to_radians(radians_to_km(0.3) * 1000) should be 0.3"
        );
        assert!(
            (km_to_radians(RADIUS_METERS / 1000.0) - 1.0).abs() < 1e-14,
            "km_to_radians(RADIUS_METERS / 1000) should be 1.0"
        );
    }

    /// Port of C++ `TestToLongitudeRadians` near-pole clamping case.
    #[test]
    fn test_longitude_radians_near_pole() {
        // Just inside the pole: cos is tiny but non-zero, so the exact-zero branch
        // is not taken. The result exceeds TAU and must be clamped.
        let lat_near_pole = std::f64::consts::FRAC_PI_2 - 1e-4;
        let result = meters_to_longitude_radians(RADIUS_METERS, lat_near_pole);
        assert_eq!(
            result,
            std::f64::consts::TAU,
            "near-pole latitude should clamp to TAU, got {result}"
        );
    }

    /// Helper: true when `a` and `b` are within 4 ULPs (mirrors gtest `EXPECT_DOUBLE_EQ`).
    fn double_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
    }

    /// Port of C++ `TestSolidAngleConversion` — exact double-equality checks.
    #[test]
    fn test_solid_angle_conversion() {
        // SquareKmToSteradians(RadiusKm^2) == 1
        assert!(double_eq(
            square_km_to_steradians((RADIUS_METERS / 1000.0).powi(2)),
            1.0
        ));
        // SteradiansToSquareKm(0.5^2) == (0.5 * RadiusKm)^2
        assert!(double_eq(
            steradians_to_square_km(0.5_f64.powi(2)),
            (0.5 * RADIUS_KM).powi(2)
        ));
        // SquareMetersToSteradians((RadiansToKm(0.3)*1000)^2) == 0.3^2
        assert!(double_eq(
            square_meters_to_steradians((radians_to_km(0.3) * 1000.0).powi(2)),
            0.3_f64.powi(2)
        ));
        // SteradiansToSquareMeters(KmToRadians(2.5)^2) == 2500^2
        assert!(double_eq(
            steradians_to_square_meters(km_to_radians(2.5).powi(2)),
            2500.0_f64.powi(2)
        ));
    }

    /// Port of C++ `TestToLongitudeRadians` — monotonicity and compatibility.
    #[test]
    fn test_longitude_radians_monotonicity_and_compat() {
        // Closer to poles ⇒ more radians for the same distance.
        assert!(
            meters_to_longitude_radians(RADIUS_METERS, 0.5)
                > meters_to_longitude_radians(RADIUS_METERS, 0.4)
        );

        // km and meters versions are compatible.
        assert_eq!(
            meters_to_longitude_radians(RADIUS_METERS, 0.5),
            km_to_longitude_radians(RADIUS_METERS / 1000.0, 0.5)
        );
    }

    /// Port of remaining C++ `TestGetDistance` `LatLng` cases.
    #[test]
    fn test_distance_latlng_exact() {
        use std::f64::consts::PI;

        // LatLng(0, 0.6) to LatLng(0, -0.4) spans 1 radian at equator = RadiusKm.
        assert!(double_eq(
            get_distance_km_latlng(
                LatLng::from_radians(0.0, 0.6),
                LatLng::from_radians(0.0, -0.4),
            ),
            RADIUS_KM
        ));

        // LatLng(80°,27°) to LatLng(55°,-153°) = PI/4 radians apart.
        assert!(double_eq(
            get_distance_meters_latlng(
                LatLng::from_degrees(80.0, 27.0),
                LatLng::from_degrees(55.0, -153.0),
            ),
            1000.0 * RADIUS_KM * PI / 4.0
        ));
    }

    /// Point-based distance tests from C++ `TestGetDistance`.
    #[test]
    fn test_distance_points() {
        use std::f64::consts::{FRAC_PI_2, PI};
        let north = Point::from_coords(0.0, 0.0, 1.0);
        let south = Point::from_coords(0.0, 0.0, -1.0);
        let west = Point::from_coords(0.0, -1.0, 0.0);

        // Same point: distance is exactly 0.
        assert_eq!(get_distance_km_points(west, west), 0.0);

        // Quarter-sphere: atan2(1, 0) = FRAC_PI_2 exactly.
        assert_eq!(
            get_distance_meters_points(north, west),
            FRAC_PI_2 * RADIUS_METERS
        );

        // Half-sphere (antipodal): atan2(0, -1) = PI exactly.
        assert_eq!(get_distance_meters_points(north, south), PI * RADIUS_METERS);
    }

    // ─── Newtype tests ─────────────────────────────────────────────

    #[test]
    fn test_meters_newtype_display() {
        assert_eq!(format!("{}", Meters(1234.5)), "1234.5 m");
    }

    #[test]
    fn test_kilometers_newtype_display() {
        assert_eq!(format!("{}", Kilometers(42.0)), "42 km");
    }

    #[test]
    fn test_steradians_newtype_display() {
        assert_eq!(format!("{}", Steradians(0.5)), "0.5 sr");
    }

    #[test]
    fn test_square_meters_newtype_display() {
        assert_eq!(format!("{}", SquareMeters(100.0)), "100 m²");
    }

    #[test]
    fn test_square_kilometers_newtype_display() {
        assert_eq!(format!("{}", SquareKilometers(50.0)), "50 km²");
    }

    #[test]
    fn test_meters_from_f64() {
        let m: Meters = 1000.0_f64.into();
        assert_eq!(m, Meters(1000.0));
        let v: f64 = m.into();
        assert_eq!(v, 1000.0);
    }

    #[test]
    fn test_km_from_f64() {
        let km: Kilometers = 42.0_f64.into();
        assert_eq!(km, Kilometers(42.0));
        let v: f64 = km.into();
        assert_eq!(v, 42.0);
    }

    #[test]
    fn test_meters_km_cross_conversion() {
        let m = Meters(5000.0);
        let km: Kilometers = m.into();
        assert_eq!(km, Kilometers(5.0));
        let back: Meters = km.into();
        assert_eq!(back, Meters(5000.0));
    }

    #[test]
    fn test_square_meters_km_cross_conversion() {
        let m2 = SquareMeters(1e6);
        let km2: SquareKilometers = m2.into();
        assert_eq!(km2, SquareKilometers(1.0));
        let back: SquareMeters = km2.into();
        assert_eq!(back, SquareMeters(1e6));
    }

    #[test]
    fn test_meters_to_angle_newtype() {
        let m = Meters(RADIUS_METERS);
        let a: Angle = m.into();
        assert_eq!(a.radians(), 1.0);
    }

    #[test]
    fn test_meters_from_chord_angle() {
        let ca = ChordAngle::from_radians(1.0);
        let m: Meters = ca.into();
        assert!((m.0 - RADIUS_METERS).abs() < 1.0);
    }

    #[test]
    fn test_km_from_chord_angle() {
        let ca = ChordAngle::from_radians(1.0);
        let km: Kilometers = ca.into();
        assert!((km.0 - RADIUS_KM).abs() < 0.001);
    }

    #[test]
    fn test_meters_arithmetic() {
        assert_eq!(Meters(100.0) + Meters(50.0), Meters(150.0));
        assert_eq!(Meters(100.0) - Meters(30.0), Meters(70.0));
        assert_eq!(Meters(100.0) * 2.0, Meters(200.0));
        assert_eq!(2.0 * Meters(100.0), Meters(200.0));
        assert_eq!(Meters(100.0) / 4.0, Meters(25.0));
        assert_eq!(-Meters(10.0), Meters(-10.0));
    }

    #[test]
    fn test_km_arithmetic() {
        assert_eq!(Kilometers(10.0) + Kilometers(5.0), Kilometers(15.0));
        assert_eq!(Kilometers(10.0) - Kilometers(3.0), Kilometers(7.0));
        assert_eq!(Kilometers(10.0) * 3.0, Kilometers(30.0));
        assert_eq!(3.0 * Kilometers(10.0), Kilometers(30.0));
        assert_eq!(Kilometers(10.0) / 2.0, Kilometers(5.0));
        assert_eq!(-Kilometers(5.0), Kilometers(-5.0));
    }

    #[test]
    fn test_steradians_to_square_km_newtype() {
        let sr = Steradians(1.0);
        let km2: SquareKilometers = sr.into();
        let expected = RADIUS_KM * RADIUS_KM;
        assert!((km2.0 - expected).abs() / expected < 1e-10);
    }

    #[test]
    fn test_square_km_to_steradians_newtype() {
        let km2 = SquareKilometers(RADIUS_KM * RADIUS_KM);
        let sr: Steradians = km2.into();
        assert!((sr.0 - 1.0).abs() < 1e-10);
    }
}
