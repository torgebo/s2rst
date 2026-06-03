// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::latlng::LatLng;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::rect::Rect;
use crate::s2loop::Loop;

// ---------------------------------------------------------------------------
// Parsing functions (string → geometry)
// ---------------------------------------------------------------------------

/// Parse a string of lat:lng pairs into `Point` objects.
/// Example: `"1:2, 3:4"` → two points.
#[wasm_bindgen(js_name = "parsePoints")]
pub fn parse_points(s: &str) -> Vec<Point> {
    s2rst::s2::text_format::parse_points(s)
        .into_iter()
        .map(Point)
        .collect()
}

/// Parse a string into a single `Point`.
#[wasm_bindgen(js_name = "parsePoint")]
pub fn parse_point(s: &str) -> Point {
    Point(s2rst::s2::text_format::parse_point(s))
}

/// Parse lat:lng pairs into `LatLng` values.
#[wasm_bindgen(js_name = "parseLatLngs")]
pub fn parse_latlngs(s: &str) -> Vec<LatLng> {
    s2rst::s2::text_format::parse_latlngs(s)
        .into_iter()
        .map(LatLng)
        .collect()
}

/// Parse a string into a `Rect`.
#[wasm_bindgen(js_name = "makeRect")]
pub fn make_rect(s: &str) -> Rect {
    Rect(s2rst::s2::text_format::make_rect(s))
}

/// Parse a string into a `Loop`.
#[wasm_bindgen(js_name = "makeLoop")]
pub fn make_loop(s: &str) -> Loop {
    Loop(s2rst::s2::text_format::make_loop(s))
}

/// Parse a string into a `Polygon`.
/// Loops are separated by `";"`, e.g. `"0:0, 0:1, 1:0; 0.1:0.1, 0.1:0.2, 0.2:0.1"`.
#[wasm_bindgen(js_name = "makePolygon")]
pub fn make_polygon(s: &str) -> Polygon {
    Polygon(s2rst::s2::text_format::make_polygon(s))
}

/// Parse a string into a `Polyline`.
#[wasm_bindgen(js_name = "makePolyline")]
pub fn make_polyline(s: &str) -> Polyline {
    Polyline(s2rst::s2::text_format::make_polyline(s))
}

// ---------------------------------------------------------------------------
// Formatting functions (geometry → string)
// ---------------------------------------------------------------------------

/// Format a `Point` as `"lat:lng"`.
#[wasm_bindgen(js_name = "pointToString")]
pub fn point_to_string(p: &Point) -> String {
    s2rst::s2::text_format::point_to_string(p.0)
}

/// Format an array of points.
#[wasm_bindgen(js_name = "pointsToString")]
pub fn points_to_string(points: Vec<Point>) -> String {
    let pts: Vec<s2rst::s2::Point> = points.iter().map(|p| p.0).collect();
    s2rst::s2::text_format::points_to_string(&pts)
}

/// Format a `Loop`.
#[wasm_bindgen(js_name = "loopToString")]
pub fn loop_to_string(loop_: &Loop) -> String {
    s2rst::s2::text_format::loop_to_string(&loop_.0)
}

/// Format a `Polygon`.
#[wasm_bindgen(js_name = "polygonToString")]
pub fn polygon_to_string(polygon: &Polygon) -> String {
    s2rst::s2::text_format::polygon_to_string(&polygon.0)
}

/// Format a `Polyline`.
#[wasm_bindgen(js_name = "polylineToString")]
pub fn polyline_to_string(polyline: &Polyline) -> String {
    s2rst::s2::text_format::polyline_to_string(&polyline.0)
}

/// Format a `LatLng`.
#[wasm_bindgen(js_name = "latlngToString")]
pub fn latlng_to_string(ll: &LatLng) -> String {
    s2rst::s2::text_format::latlng_to_string(ll.0)
}
