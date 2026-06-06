// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::cap::Cap;
use crate::cell::Cell;
use crate::error::{js_err, validation_error_to_js};
use crate::point::Point;
use crate::rect::Rect;

/// A loop — a simple closed curve on the sphere.
#[wasm_bindgen(js_name = "S2Loop")]
pub struct Loop(pub(crate) s2rst::s2::Loop);

#[wasm_bindgen(js_class = "S2Loop")]
impl Loop {
    /// Create from an array of `Point` vertices. Throws on an empty vertex list
    /// (a 0-vertex loop traps in core; use `empty()`/`full()` for the sentinels).
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: Vec<Point>) -> Result<Loop, JsValue> {
        if vertices.is_empty() {
            return Err(js_err(
                "a loop requires at least one vertex; use S2Loop.empty() / S2Loop.full() for the sentinel loops",
            ));
        }
        let pts: Vec<s2rst::s2::Point> = vertices.iter().map(|p| p.0).collect();
        Ok(Loop(s2rst::s2::Loop::new(pts)))
    }

    /// The empty loop.
    pub fn empty() -> Loop {
        Loop(s2rst::s2::Loop::empty())
    }

    /// The full loop (whole sphere).
    pub fn full() -> Loop {
        Loop(s2rst::s2::Loop::full())
    }

    /// Create from a cell.
    #[wasm_bindgen(js_name = "fromCell")]
    pub fn from_cell(cell: &Cell) -> Loop {
        Loop(s2rst::s2::Loop::from_cell(&cell.0))
    }

    /// Create a regular loop.
    #[wasm_bindgen(js_name = "makeRegular")]
    pub fn make_regular(center: &Point, radius: &Angle, num_vertices: usize) -> Loop {
        Loop(s2rst::s2::Loop::make_regular(
            center.0,
            radius.0,
            num_vertices,
        ))
    }

    /// Number of vertices.
    #[wasm_bindgen(js_name = "numVertices")]
    pub fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// Get the i-th vertex. For a non-empty loop, indices wrap modulo the vertex
    /// count (so `vertex(n)` == `vertex(0)`, matching S2 edge conventions).
    /// Throws if the loop has no vertices (empty/full loops).
    pub fn vertex(&self, i: usize) -> Result<Point, JsValue> {
        if self.0.num_vertices() == 0 {
            return Err(js_err("loop has no vertices"));
        }
        Ok(Point(self.0.vertex(i)))
    }

    /// All vertices.
    pub fn vertices(&self) -> Vec<Point> {
        self.0.vertices().iter().map(|p| Point(*p)).collect()
    }

    /// Whether this is the empty loop.
    #[wasm_bindgen(js_name = "isEmptyLoop")]
    pub fn is_empty_loop(&self) -> bool {
        self.0.is_empty_loop()
    }

    /// Whether this is the full loop.
    #[wasm_bindgen(js_name = "isFullLoop")]
    pub fn is_full_loop(&self) -> bool {
        self.0.is_full_loop()
    }

    /// Whether this is the empty or the full loop (a sentinel loop).
    #[wasm_bindgen(js_name = "isEmptyOrFull")]
    pub fn is_empty_or_full(&self) -> bool {
        self.0.is_empty_or_full()
    }

    /// Whether this loop represents a hole.
    #[wasm_bindgen(js_name = "isHole")]
    pub fn is_hole(&self) -> bool {
        self.0.is_hole()
    }

    /// The sign (+1 or −1).
    pub fn sign(&self) -> i32 {
        self.0.sign()
    }

    /// Whether the loop is normalized (contains at most half the sphere).
    #[wasm_bindgen(js_name = "isNormalized")]
    pub fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    /// Normalize in place.
    pub fn normalize(&mut self) {
        self.0.normalize();
    }

    /// Invert in place.
    pub fn invert(&mut self) {
        self.0.invert();
    }

    /// Area in steradians.
    pub fn area(&self) -> f64 {
        self.0.area()
    }

    /// Centroid.
    pub fn centroid(&self) -> Point {
        Point(self.0.centroid())
    }

    /// Whether this loop contains the given point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        use s2rst::s2::Region;
        self.0.contains_point(&p.0)
    }

    /// Turning angle (curvature).
    #[wasm_bindgen(js_name = "turningAngle")]
    pub fn turning_angle(&self) -> f64 {
        self.0.turning_angle()
    }

    /// Validate the loop.
    pub fn validate(&self) -> Result<(), JsValue> {
        self.0.validate().map_err(validation_error_to_js)
    }

    /// Whether the loop equals another.
    pub fn equal(&self, other: &Loop) -> bool {
        self.0.equal(&other.0)
    }

    /// Bounding rectangle.
    pub fn bound(&self) -> Rect {
        Rect(self.0.bound())
    }

    /// Bounding cap.
    #[wasm_bindgen(js_name = "capBound")]
    pub fn cap_bound(&self) -> Cap {
        Cap(self.0.bound().cap_bound())
    }

    /// Distance to a point.
    #[wasm_bindgen(js_name = "getDistance")]
    pub fn get_distance(&self, x: &Point) -> Angle {
        Angle(self.0.get_distance(x.0))
    }

    /// Project a point onto the loop.
    #[wasm_bindgen(js_name = "projectPoint")]
    pub fn project_point(&self, x: &Point) -> Point {
        Point(self.0.project_point(x.0))
    }

    /// Whether this loop contains another loop.
    #[wasm_bindgen(js_name = "containsLoop")]
    pub fn contains_loop(&self, b: &Loop) -> bool {
        self.0.contains_loop(&b.0)
    }

    /// Whether this loop intersects another loop.
    #[wasm_bindgen(js_name = "intersectsLoop")]
    pub fn intersects_loop(&self, b: &Loop) -> bool {
        self.0.intersects_loop(&b.0)
    }

    /// Whether the boundary is approximately equal.
    #[wasm_bindgen(js_name = "boundaryApproxEq")]
    pub fn boundary_approx_eq(&self, b: &Loop, max_error: &Angle) -> bool {
        self.0.boundary_approx_eq(&b.0, max_error.0)
    }

    /// Whether the boundary is near.
    #[wasm_bindgen(js_name = "boundaryNear")]
    pub fn boundary_near(&self, b: &Loop, max_error: &Angle) -> bool {
        self.0.boundary_near(&b.0, max_error.0)
    }

    /// Whether this loop contains the origin.
    #[wasm_bindgen(js_name = "containsOrigin")]
    pub fn contains_origin(&self) -> bool {
        self.0.contains_origin()
    }

    /// The i-th vertex in canonical orientation. Throws on a loop with no vertices.
    #[wasm_bindgen(js_name = "orientedVertex")]
    pub fn oriented_vertex(&self, i: usize) -> Result<Point, JsValue> {
        if self.0.num_vertices() == 0 {
            return Err(js_err("loop has no vertices"));
        }
        Ok(Point(self.0.oriented_vertex(i)))
    }

    /// Nesting depth (0 = shell, 1 = hole inside a shell, ...).
    pub fn depth(&self) -> i32 {
        self.0.depth()
    }

    /// Total geodesic curvature (turning angle) of the loop boundary.
    #[wasm_bindgen(js_name = "getCurvature")]
    pub fn get_curvature(&self) -> f64 {
        self.0.get_curvature()
    }

    /// Maximum error of `getCurvature`.
    #[wasm_bindgen(js_name = "getCurvatureMaxError")]
    pub fn get_curvature_max_error(&self) -> f64 {
        self.0.get_curvature_max_error()
    }

    /// Distance from a point to the loop boundary (ignoring the interior).
    #[wasm_bindgen(js_name = "getDistanceToBoundary")]
    pub fn get_distance_to_boundary(&self, x: &Point) -> Angle {
        Angle(self.0.get_distance_to_boundary(x.0))
    }

    /// Project a point onto the loop boundary.
    #[wasm_bindgen(js_name = "projectToBoundary")]
    pub fn project_to_boundary(&self, x: &Point) -> Point {
        Point(self.0.project_to_boundary(x.0))
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
    pub fn decode(bytes: &[u8]) -> Result<Loop, JsValue> {
        use s2rst::s2::encoding::S2Decode;
        let mut cur = std::io::Cursor::new(bytes);
        s2rst::s2::Loop::decode(&mut cur).map(Loop).map_err(js_err)
    }
}
