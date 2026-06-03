// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::s2loop::Loop;

/// Computes the convex hull of a set of points, polylines, loops, and polygons.
#[wasm_bindgen]
pub struct ConvexHullQuery(s2rst::s2::convex_hull_query::ConvexHullQuery);

impl Default for ConvexHullQuery {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl ConvexHullQuery {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ConvexHullQuery {
        ConvexHullQuery(s2rst::s2::convex_hull_query::ConvexHullQuery::new())
    }

    /// Add a single point.
    #[wasm_bindgen(js_name = "addPoint")]
    pub fn add_point(&mut self, p: &Point) {
        self.0.add_point(p.0);
    }

    /// Add an array of points.
    #[wasm_bindgen(js_name = "addPoints")]
    pub fn add_points(&mut self, points: Vec<Point>) {
        let pts: Vec<s2rst::s2::Point> = points.iter().map(|p| p.0).collect();
        self.0.add_points(&pts);
    }

    /// Add a polyline.
    #[wasm_bindgen(js_name = "addPolyline")]
    pub fn add_polyline(&mut self, polyline: &Polyline) {
        self.0.add_polyline(&polyline.0);
    }

    /// Add a loop.
    #[wasm_bindgen(js_name = "addLoop")]
    pub fn add_loop(&mut self, loop_: &Loop) {
        self.0.add_loop(&loop_.0);
    }

    /// Add a polygon.
    #[wasm_bindgen(js_name = "addPolygon")]
    pub fn add_polygon(&mut self, polygon: &Polygon) {
        self.0.add_polygon(&polygon.0);
    }

    /// Compute the convex hull as a loop.
    #[wasm_bindgen(js_name = "convexHull")]
    pub fn convex_hull(&mut self) -> Loop {
        Loop(self.0.convex_hull())
    }
}
