// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::latlng::LatLng;

/// A point on the unit sphere, represented as a unit-length 3D vector.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Point(pub(crate) s2rst::s2::Point);

#[wasm_bindgen]
impl Point {
    /// Create a point from (x, y, z) coordinates. The vector is **normalized**
    /// to unit length (S2 points are unit vectors), so any non-zero input
    /// vector works; the zero vector yields the origin.
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64, z: f64) -> Point {
        Point(s2rst::s2::Point::from_coords(x, y, z))
    }

    /// The origin point.
    pub fn origin() -> Point {
        Point(s2rst::s2::Point::origin())
    }

    /// Create from a `LatLng`.
    #[wasm_bindgen(js_name = "fromLatLng")]
    pub fn from_lat_lng(ll: &LatLng) -> Point {
        Point(ll.0.to_point())
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> f64 {
        self.0.x()
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> f64 {
        self.0.y()
    }

    #[wasm_bindgen(getter)]
    pub fn z(&self) -> f64 {
        self.0.z()
    }

    /// Whether this point has unit length (within tolerance).
    #[wasm_bindgen(js_name = "isUnit")]
    pub fn is_unit(&self) -> bool {
        self.0.is_unit()
    }

    /// Return a unit-length version of this point.
    pub fn normalize(&self) -> Point {
        Point(self.0.normalize())
    }

    /// Angular distance to another point.
    pub fn distance(&self, other: &Point) -> Angle {
        Angle(self.0.distance(other.0))
    }

    /// Chord angle distance to another point.
    #[wasm_bindgen(js_name = "chordAngle")]
    pub fn chord_angle(&self, other: &Point) -> crate::angle::ChordAngle {
        crate::angle::ChordAngle(self.0.chord_angle(other.0))
    }

    /// Whether approximately equal to another point.
    #[wasm_bindgen(js_name = "approxEq")]
    pub fn approx_eq(&self, other: &Point) -> bool {
        self.0.approx_eq(other.0)
    }

    /// Approximate equality with a given tolerance.
    #[wasm_bindgen(js_name = "approxEqWithAngle")]
    pub fn approx_eq_with_angle(&self, other: &Point, max_error: &Angle) -> bool {
        self.0.approx_eq_with(other.0, max_error.0)
    }

    /// Cross product that avoids cancellation when the points are nearly
    /// identical or antipodal.
    #[wasm_bindgen(js_name = "pointCross")]
    pub fn point_cross(&self, other: &Point) -> Point {
        Point(self.0.point_cross(other.0))
    }

    /// Vector sum (the result is generally not unit-length).
    pub fn add(&self, other: &Point) -> Point {
        Point(self.0 + other.0)
    }

    /// Vector difference (the result is generally not unit-length).
    pub fn sub(&self, other: &Point) -> Point {
        Point(self.0 - other.0)
    }

    /// Vector negation (antipodal point).
    pub fn neg(&self) -> Point {
        Point(-self.0)
    }

    /// Convert to `LatLng`.
    #[wasm_bindgen(js_name = "toLatLng")]
    pub fn to_lat_lng(&self) -> LatLng {
        LatLng(s2rst::s2::LatLng::from_point(self.0))
    }

    /// Get the coordinates as `[x, y, z]`.
    #[wasm_bindgen(js_name = "toArray")]
    pub fn to_array(&self) -> Vec<f64> {
        vec![self.0.x(), self.0.y(), self.0.z()]
    }

    /// Matches the core `Display` format: `(x, y, z)` at 15-digit precision.
    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.0.to_string()
    }
}

/// Rotate point `p` around `axis` by the given angle.
#[wasm_bindgen(js_name = "rotatePoint")]
pub fn rotate_point(p: &Point, axis: &Point, angle: &Angle) -> Point {
    Point(s2rst::s2::rotate(p.0, axis.0, angle.0))
}

/// Return a unit-length vector orthogonal to `p`.
#[wasm_bindgen(js_name = "orthoPoint")]
pub fn ortho_point(p: &Point) -> Point {
    Point(s2rst::s2::ortho(p.0))
}

// ---------------------------------------------------------------------------
// Batch API for high-throughput use
// ---------------------------------------------------------------------------

/// Parse an array of `[lat, lng, lat, lng, ...]` (degrees) into an array of
/// `Point` objects. Operates entirely in WASM, avoiding per-point boundary
/// crossings.
#[wasm_bindgen(js_name = "pointsFromLatLngDegrees")]
pub fn points_from_lat_lng_degrees(coords: &[f64]) -> Result<Vec<Point>, JsValue> {
    if !coords.len().is_multiple_of(2) {
        return Err(JsValue::from_str(
            "coords array length must be even (lat/lng pairs)",
        ));
    }
    Ok(coords
        .chunks_exact(2)
        .map(|c| {
            let ll = s2rst::s2::LatLng::from_degrees(c[0], c[1]);
            Point(ll.to_point())
        })
        .collect())
}
