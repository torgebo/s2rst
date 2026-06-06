// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::{Angle, ChordAngle};
use crate::latlng::LatLng;
use crate::point::Point;

/// Convert meters to an `Angle`.
#[wasm_bindgen(js_name = "metersToAngle")]
pub fn meters_to_angle(meters: f64) -> Angle {
    Angle(s2rst::s2::earth::meters_to_angle(meters))
}

/// Convert an `Angle` to meters.
#[wasm_bindgen(js_name = "angleToMeters")]
pub fn angle_to_meters(angle: &Angle) -> f64 {
    s2rst::s2::earth::to_meters(angle.0)
}

/// Convert kilometers to an `Angle`.
#[wasm_bindgen(js_name = "kmToAngle")]
pub fn km_to_angle(km: f64) -> Angle {
    Angle(s2rst::s2::earth::km_to_angle(km))
}

/// Convert an `Angle` to kilometers.
#[wasm_bindgen(js_name = "angleToKm")]
pub fn angle_to_km(angle: &Angle) -> f64 {
    s2rst::s2::earth::to_km(angle.0)
}

/// Distance between two points in meters.
#[wasm_bindgen(js_name = "getDistanceMetersPoints")]
pub fn get_distance_meters_points(a: &Point, b: &Point) -> f64 {
    s2rst::s2::earth::get_distance_meters_points(a.0, b.0)
}

/// Distance between two `LatLng` values in meters.
#[wasm_bindgen(js_name = "getDistanceMetersLatLng")]
pub fn get_distance_meters_latlng(a: &LatLng, b: &LatLng) -> f64 {
    s2rst::s2::earth::get_distance_meters_latlng(a.0, b.0)
}

/// Distance between two points in kilometers.
#[wasm_bindgen(js_name = "getDistanceKmPoints")]
pub fn get_distance_km_points(a: &Point, b: &Point) -> f64 {
    s2rst::s2::earth::get_distance_km_points(a.0, b.0)
}

/// Distance between two `LatLng` values in kilometers.
#[wasm_bindgen(js_name = "getDistanceKmLatLng")]
pub fn get_distance_km_latlng(a: &LatLng, b: &LatLng) -> f64 {
    s2rst::s2::earth::get_distance_km_latlng(a.0, b.0)
}

/// Initial bearing from a to b.
#[wasm_bindgen(js_name = "getInitialBearing")]
pub fn get_initial_bearing(a: &LatLng, b: &LatLng) -> Angle {
    Angle(s2rst::s2::earth::get_initial_bearing(a.0, b.0))
}

/// Square kilometers to steradians.
#[wasm_bindgen(js_name = "squareKmToSteradians")]
pub fn square_km_to_steradians(km2: f64) -> f64 {
    s2rst::s2::earth::square_km_to_steradians(km2)
}

/// Steradians to square kilometers.
#[wasm_bindgen(js_name = "steradiansToSquareKm")]
pub fn steradians_to_square_km(steradians: f64) -> f64 {
    s2rst::s2::earth::steradians_to_square_km(steradians)
}

/// Square meters to steradians.
#[wasm_bindgen(js_name = "squareMetersToSteradians")]
pub fn square_meters_to_steradians(m2: f64) -> f64 {
    s2rst::s2::earth::square_meters_to_steradians(m2)
}

/// Steradians to square meters.
#[wasm_bindgen(js_name = "steradiansToSquareMeters")]
pub fn steradians_to_square_meters(steradians: f64) -> f64 {
    s2rst::s2::earth::steradians_to_square_meters(steradians)
}

// -- Radians / ChordAngle / longitude conversions + constants (Tier 3.4) -----

/// Convert meters to radians on Earth's surface.
#[wasm_bindgen(js_name = "metersToRadians")]
pub fn meters_to_radians(meters: f64) -> f64 {
    s2rst::s2::earth::meters_to_radians(meters)
}

/// Convert radians to meters on Earth's surface.
#[wasm_bindgen(js_name = "radiansToMeters")]
pub fn radians_to_meters(radians: f64) -> f64 {
    s2rst::s2::earth::radians_to_meters(radians)
}

/// Convert km to radians on Earth's surface.
#[wasm_bindgen(js_name = "kmToRadians")]
pub fn km_to_radians(km: f64) -> f64 {
    s2rst::s2::earth::km_to_radians(km)
}

/// Convert radians to km on Earth's surface.
#[wasm_bindgen(js_name = "radiansToKm")]
pub fn radians_to_km(radians: f64) -> f64 {
    s2rst::s2::earth::radians_to_km(radians)
}

/// Convert meters to a `ChordAngle`.
#[wasm_bindgen(js_name = "metersToChordAngle")]
pub fn meters_to_chord_angle(meters: f64) -> ChordAngle {
    ChordAngle(s2rst::s2::earth::meters_to_chord_angle(meters))
}

/// Convert a `ChordAngle` to meters.
#[wasm_bindgen(js_name = "chordAngleToMeters")]
pub fn chord_angle_to_meters(cangle: &ChordAngle) -> f64 {
    s2rst::s2::earth::chord_angle_to_meters(cangle.0)
}

/// Convert km to a `ChordAngle`.
#[wasm_bindgen(js_name = "kmToChordAngle")]
pub fn km_to_chord_angle(km: f64) -> ChordAngle {
    ChordAngle(s2rst::s2::earth::km_to_chord_angle(km))
}

/// Convert a `ChordAngle` to km.
#[wasm_bindgen(js_name = "chordAngleToKm")]
pub fn chord_angle_to_km(cangle: &ChordAngle) -> f64 {
    s2rst::s2::earth::chord_angle_to_km(cangle.0)
}

/// Longitude span (radians) of a given east-west distance in meters at a latitude.
#[wasm_bindgen(js_name = "metersToLongitudeRadians")]
pub fn meters_to_longitude_radians(meters: f64, latitude_radians: f64) -> f64 {
    s2rst::s2::earth::meters_to_longitude_radians(meters, latitude_radians)
}

/// Longitude span (radians) of a given east-west distance in km at a latitude.
#[wasm_bindgen(js_name = "kmToLongitudeRadians")]
pub fn km_to_longitude_radians(km: f64, latitude_radians: f64) -> f64 {
    s2rst::s2::earth::km_to_longitude_radians(km, latitude_radians)
}

/// The haversine of an angle (radians): `(1 − cos θ) / 2`.
#[wasm_bindgen]
pub fn haversine(radians: f64) -> f64 {
    s2rst::s2::earth::haversine(radians)
}

/// Earth's mean radius in meters.
#[wasm_bindgen(js_name = "radiusMeters")]
pub fn radius_meters() -> f64 {
    s2rst::s2::earth::RADIUS_METERS
}

/// Earth's mean radius in km.
#[wasm_bindgen(js_name = "radiusKm")]
pub fn radius_km() -> f64 {
    s2rst::s2::earth::RADIUS_KM
}

/// Lowest land altitude on Earth, in meters.
#[wasm_bindgen(js_name = "lowestAltitudeMeters")]
pub fn lowest_altitude_meters() -> f64 {
    s2rst::s2::earth::LOWEST_ALTITUDE_METERS
}

/// Highest land altitude on Earth, in meters.
#[wasm_bindgen(js_name = "highestAltitudeMeters")]
pub fn highest_altitude_meters() -> f64 {
    s2rst::s2::earth::HIGHEST_ALTITUDE_METERS
}
