// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for the `Earth` helpers — converting between angles on the
//! unit sphere and physical distances/areas on the Earth.

use pyo3::prelude::*;

use s2rst::s2::earth;

use crate::angle::{PyAngle, PyChordAngle};
use crate::s2point::{PyLatLng, PyS2Point};

/// Conversions between spherical angles and physical distances/areas on Earth,
/// plus great-circle distances between points. All methods are static.
#[pyclass(name = "Earth")]
pub struct PyEarth;

#[pymethods]
impl PyEarth {
    /// Earth's mean radius in meters.
    #[classattr]
    const RADIUS_METERS: f64 = earth::RADIUS_METERS;
    /// Earth's mean radius in kilometers.
    #[classattr]
    const RADIUS_KM: f64 = earth::RADIUS_KM;

    #[staticmethod]
    fn meters_to_angle(meters: f64) -> PyAngle {
        PyAngle(earth::meters_to_angle(meters))
    }
    #[staticmethod]
    fn meters_to_chord_angle(meters: f64) -> PyChordAngle {
        PyChordAngle(earth::meters_to_chord_angle(meters))
    }
    #[staticmethod]
    fn meters_to_radians(meters: f64) -> f64 {
        earth::meters_to_radians(meters)
    }
    #[staticmethod]
    fn to_meters(angle: &PyAngle) -> f64 {
        earth::to_meters(angle.0)
    }
    #[staticmethod]
    fn radians_to_meters(radians: f64) -> f64 {
        earth::radians_to_meters(radians)
    }

    #[staticmethod]
    fn km_to_angle(km: f64) -> PyAngle {
        PyAngle(earth::km_to_angle(km))
    }
    #[staticmethod]
    fn km_to_radians(km: f64) -> f64 {
        earth::km_to_radians(km)
    }
    #[staticmethod]
    fn to_km(angle: &PyAngle) -> f64 {
        earth::to_km(angle.0)
    }
    #[staticmethod]
    fn radians_to_km(radians: f64) -> f64 {
        earth::radians_to_km(radians)
    }

    #[staticmethod]
    fn square_km_to_steradians(km2: f64) -> f64 {
        earth::square_km_to_steradians(km2)
    }
    #[staticmethod]
    fn square_meters_to_steradians(m2: f64) -> f64 {
        earth::square_meters_to_steradians(m2)
    }
    #[staticmethod]
    fn steradians_to_square_km(steradians: f64) -> f64 {
        earth::steradians_to_square_km(steradians)
    }
    #[staticmethod]
    fn steradians_to_square_meters(steradians: f64) -> f64 {
        earth::steradians_to_square_meters(steradians)
    }

    /// Great-circle distance between two points, in meters.
    #[staticmethod]
    fn distance_meters(a: &PyS2Point, b: &PyS2Point) -> f64 {
        earth::get_distance_meters_points(a.0, b.0)
    }
    /// Great-circle distance between two points, in kilometers.
    #[staticmethod]
    fn distance_km(a: &PyS2Point, b: &PyS2Point) -> f64 {
        earth::get_distance_km_points(a.0, b.0)
    }
    /// Great-circle distance between two `LatLng`s, in meters.
    #[staticmethod]
    fn distance_meters_latlng(a: &PyLatLng, b: &PyLatLng) -> f64 {
        earth::get_distance_meters_latlng(a.0, b.0)
    }
    /// Great-circle distance between two `LatLng`s, in kilometers.
    #[staticmethod]
    fn distance_km_latlng(a: &PyLatLng, b: &PyLatLng) -> f64 {
        earth::get_distance_km_latlng(a.0, b.0)
    }

    /// Convert a chord angle to a surface distance in meters.
    #[staticmethod]
    fn chord_angle_to_meters(cangle: &PyChordAngle) -> f64 {
        earth::chord_angle_to_meters(cangle.0)
    }
    /// Convert a chord angle to a surface distance in kilometers.
    #[staticmethod]
    fn chord_angle_to_km(cangle: &PyChordAngle) -> f64 {
        earth::chord_angle_to_km(cangle.0)
    }
    /// Convert a distance in kilometers to a chord angle.
    #[staticmethod]
    fn km_to_chord_angle(km: f64) -> PyChordAngle {
        PyChordAngle(earth::km_to_chord_angle(km))
    }
    /// East-west distance in meters expressed as a longitude span (radians) at
    /// the given latitude.
    #[staticmethod]
    fn meters_to_longitude_radians(meters: f64, latitude_radians: f64) -> f64 {
        earth::meters_to_longitude_radians(meters, latitude_radians)
    }
    /// East-west distance in kilometers expressed as a longitude span (radians)
    /// at the given latitude.
    #[staticmethod]
    fn km_to_longitude_radians(km: f64, latitude_radians: f64) -> f64 {
        earth::km_to_longitude_radians(km, latitude_radians)
    }
    /// The haversine of an angle (radians): `sin^2(x / 2)`.
    #[staticmethod]
    fn haversine(radians: f64) -> f64 {
        earth::haversine(radians)
    }
    /// The initial great-circle bearing (heading) from `a` to `b`.
    #[staticmethod]
    fn get_initial_bearing(a: &PyLatLng, b: &PyLatLng) -> PyAngle {
        PyAngle(earth::get_initial_bearing(a.0, b.0))
    }
}
