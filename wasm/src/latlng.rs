// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::point::Point;

/// A point on the unit sphere expressed as latitude/longitude.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct LatLng(pub(crate) s2rst::s2::LatLng);

#[wasm_bindgen]
impl LatLng {
    /// Create from `Angle` lat and lng.
    #[wasm_bindgen(constructor)]
    pub fn new(lat: &Angle, lng: &Angle) -> LatLng {
        LatLng(s2rst::s2::LatLng::new(lat.0, lng.0))
    }

    /// Create from degrees.
    #[wasm_bindgen(js_name = "fromDegrees")]
    pub fn from_degrees(lat: f64, lng: f64) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_degrees(lat, lng))
    }

    /// Create from radians.
    #[wasm_bindgen(js_name = "fromRadians")]
    pub fn from_radians(lat: f64, lng: f64) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_radians(lat, lng))
    }

    /// Create from a `Point`.
    #[wasm_bindgen(js_name = "fromPoint")]
    pub fn from_point(p: &Point) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_point(p.0))
    }

    /// Create from E5 representation.
    #[wasm_bindgen(js_name = "fromE5")]
    pub fn from_e5(lat_e5: i32, lng_e5: i32) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_e5(lat_e5, lng_e5))
    }

    /// Create from E6 representation.
    #[wasm_bindgen(js_name = "fromE6")]
    pub fn from_e6(lat_e6: i32, lng_e6: i32) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_e6(lat_e6, lng_e6))
    }

    /// Create from E7 representation.
    #[wasm_bindgen(js_name = "fromE7")]
    pub fn from_e7(lat_e7: i32, lng_e7: i32) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_e7(lat_e7, lng_e7))
    }

    #[wasm_bindgen(js_name = "fromUnsignedE6")]
    pub fn from_unsigned_e6(lat_e6: u32, lng_e6: u32) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_unsigned_e6(lat_e6, lng_e6))
    }

    #[wasm_bindgen(js_name = "fromUnsignedE7")]
    pub fn from_unsigned_e7(lat_e7: u32, lng_e7: u32) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_unsigned_e7(lat_e7, lng_e7))
    }

    /// Component-wise sum (treats lat/lng as a 2-vector of angles).
    pub fn add(&self, other: &LatLng) -> LatLng {
        LatLng(self.0 + other.0)
    }

    /// Component-wise difference.
    pub fn sub(&self, other: &LatLng) -> LatLng {
        LatLng(self.0 - other.0)
    }

    /// Invalid sentinel value.
    pub fn invalid() -> LatLng {
        LatLng(s2rst::s2::LatLng::invalid())
    }

    /// Latitude as an Angle.
    #[wasm_bindgen(getter, js_name = "lat")]
    pub fn lat(&self) -> Angle {
        Angle(self.0.lat)
    }

    /// Longitude as an Angle.
    #[wasm_bindgen(getter, js_name = "lng")]
    pub fn lng(&self) -> Angle {
        Angle(self.0.lng)
    }

    /// Latitude in degrees.
    #[wasm_bindgen(getter, js_name = "latDegrees")]
    pub fn lat_degrees(&self) -> f64 {
        self.0.lat.degrees()
    }

    /// Longitude in degrees.
    #[wasm_bindgen(getter, js_name = "lngDegrees")]
    pub fn lng_degrees(&self) -> f64 {
        self.0.lng.degrees()
    }

    /// Whether this is a valid lat/lng pair.
    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Normalized version (longitude wrapped to [−180, 180]).
    pub fn normalized(&self) -> LatLng {
        LatLng(self.0.normalized())
    }

    /// Convert to a `Point`.
    #[wasm_bindgen(js_name = "toPoint")]
    pub fn to_point(&self) -> Point {
        Point(self.0.to_point())
    }

    /// Great-circle distance to another `LatLng`.
    #[wasm_bindgen(js_name = "getDistance")]
    pub fn get_distance(&self, other: &LatLng) -> Angle {
        Angle(self.0.get_distance(other.0))
    }

    /// Whether approximately equal.
    #[wasm_bindgen(js_name = "approxEq")]
    pub fn approx_eq(&self, other: &LatLng) -> bool {
        self.0.approx_eq(other.0)
    }

    /// String in degrees.
    #[wasm_bindgen(js_name = "toStringInDegrees")]
    pub fn to_string_in_degrees(&self) -> String {
        self.0.to_string_in_degrees()
    }

    /// Matches the core `Display` format: `[lat, lng]`. For the degrees-only
    /// rendering use `toStringInDegrees()`.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.0.to_string()
    }
}
