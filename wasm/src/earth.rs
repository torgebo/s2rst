// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
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
