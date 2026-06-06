// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::latlng::LatLng;
use crate::point::Point;

/// A polyline — a sequence of connected edges on the sphere.
#[wasm_bindgen]
pub struct Polyline(pub(crate) s2rst::s2::polyline::Polyline);

#[wasm_bindgen]
impl Polyline {
    /// Create from an array of `Point` vertices.
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: Vec<Point>) -> Polyline {
        let pts: Vec<s2rst::s2::Point> = vertices.iter().map(|p| p.0).collect();
        Polyline(s2rst::s2::polyline::Polyline::new(pts))
    }

    /// Create from an array of `LatLng` values.
    #[wasm_bindgen(js_name = "fromLatLngs")]
    pub fn from_lat_lngs(latlngs: Vec<LatLng>) -> Polyline {
        let inner: Vec<s2rst::s2::LatLng> = latlngs.iter().map(|l| l.0).collect();
        Polyline(s2rst::s2::polyline::Polyline::from_lat_lngs(&inner))
    }

    /// Number of vertices.
    #[wasm_bindgen(js_name = "numVertices")]
    pub fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// Get the i-th vertex. Throws if `i` is out of range.
    pub fn vertex(&self, i: usize) -> Result<Point, JsValue> {
        let n = self.0.num_vertices();
        if i >= n {
            return Err(crate::error::js_err(format!(
                "vertex index {i} out of range (0..{n})"
            )));
        }
        Ok(Point(self.0.vertex(i)))
    }

    /// All vertices.
    pub fn vertices(&self) -> Vec<Point> {
        self.0.vertices_vec().iter().map(|p| Point(*p)).collect()
    }

    /// Reverse in place.
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Length of the polyline.
    pub fn length(&self) -> Angle {
        Angle(self.0.length())
    }

    /// Centroid.
    pub fn centroid(&self) -> Point {
        Point(self.0.centroid())
    }

    /// Validate. Throws on error.
    pub fn validate(&self) -> Result<(), JsValue> {
        self.0
            .validate()
            .map_err(crate::error::validation_error_to_js)
    }

    /// Project a point onto the polyline, returning `[projected_point, next_vertex_index]`.
    pub fn project(&self, point: &Point) -> Vec<JsValue> {
        let (proj, idx) = self.0.project(point.0);
        vec![Point(proj).into(), JsValue::from_f64(idx as f64)]
    }

    /// Interpolate along the polyline. `fraction` in [0, 1].
    /// Returns `[point, next_vertex_index]`.
    pub fn interpolate(&self, fraction: f64) -> Vec<JsValue> {
        let (pt, idx) = self.0.interpolate(fraction);
        vec![Point(pt).into(), JsValue::from_f64(idx as f64)]
    }

    /// Whether this polyline equals another.
    pub fn equal(&self, other: &Polyline) -> bool {
        self.0.equal(&other.0)
    }

    /// Whether approximately equal with tolerance.
    #[wasm_bindgen(js_name = "approxEqWith")]
    pub fn approx_eq_with(&self, other: &Polyline, max_error: &Angle) -> bool {
        self.0.approx_eq_with(&other.0, max_error.0)
    }

    /// Whether the given point is on the right side of the polyline.
    #[wasm_bindgen(js_name = "isOnRight")]
    pub fn is_on_right(&self, point: &Point) -> bool {
        self.0.is_on_right(point.0)
    }

    /// Whether this polyline intersects another.
    pub fn intersects(&self, other: &Polyline) -> bool {
        self.0.intersects(&other.0)
    }

    /// Subsample vertices within a tolerance. Returns indices.
    #[wasm_bindgen(js_name = "subsampleVertices")]
    pub fn subsample_vertices(&self, tolerance: &Angle) -> Vec<usize> {
        self.0.subsample_vertices(tolerance.0)
    }

    /// Whether this polyline nearly covers another.
    #[wasm_bindgen(js_name = "nearlyCovers")]
    pub fn nearly_covers(&self, covered: &Polyline, max_error: &Angle) -> bool {
        self.0.nearly_covers(&covered.0, max_error.0)
    }

    /// Inverse of `interpolate`: the fraction in `[0, 1]` along the polyline of
    /// `point`, given the `nextVertex` index returned by `interpolate`.
    pub fn uninterpolate(&self, point: &Point, next_vertex: usize) -> f64 {
        self.0.uninterpolate(point.0, next_vertex)
    }

    /// Encode to the S2 binary format (`Uint8Array`).
    pub fn encode(&self) -> Vec<u8> {
        use s2rst::s2::encoding::S2Encode;
        let mut buf = Vec::new();
        self.0
            .encode(&mut buf)
            .expect("encoding to a Vec is infallible");
        buf
    }

    /// Decode from the S2 binary format. Throws on malformed data.
    pub fn decode(bytes: &[u8]) -> Result<Polyline, JsValue> {
        use s2rst::s2::encoding::S2Decode;
        let mut cur = std::io::Cursor::new(bytes);
        s2rst::s2::polyline::Polyline::decode(&mut cur)
            .map(Polyline)
            .map_err(crate::error::js_err)
    }
}
