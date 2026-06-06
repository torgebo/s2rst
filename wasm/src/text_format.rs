// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::error::js_err;
use crate::latlng::LatLng;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::rect::Rect;
use crate::s2loop::Loop;

// ---------------------------------------------------------------------------
// Parsing functions (string → geometry)
//
// Unlike the lenient core parsers (which silently skip malformed tokens and
// default to the origin), these bindings are STRICT: any malformed `lat:lng`
// token throws a JS exception. This is a deliberate divergence — a typed JS API
// should never return silently-wrong geometry. See the crate-level error model.
// ---------------------------------------------------------------------------

/// Strictly parse a comma-separated list of `lat:lng` pairs (degrees).
/// Empty/whitespace input yields an empty list; any malformed token throws.
pub(crate) fn parse_latlngs_strict(s: &str) -> Result<Vec<s2rst::s2::LatLng>, JsValue> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let (lat_str, lng_str) = part
            .split_once(':')
            .ok_or_else(|| js_err(format!("expected `lat:lng`, got {part:?}")))?;
        let lat: f64 = lat_str
            .trim()
            .parse()
            .map_err(|_| js_err(format!("invalid latitude {:?}", lat_str.trim())))?;
        let lng: f64 = lng_str
            .trim()
            .parse()
            .map_err(|_| js_err(format!("invalid longitude {:?}", lng_str.trim())))?;
        out.push(s2rst::s2::LatLng::from_degrees(lat, lng));
    }
    Ok(out)
}

/// Parse a string of lat:lng pairs into `Point` objects.
/// Example: `"1:2, 3:4"` → two points. Throws on malformed input.
#[wasm_bindgen(js_name = "parsePoints")]
pub fn parse_points(s: &str) -> Result<Vec<Point>, JsValue> {
    Ok(parse_latlngs_strict(s)?
        .into_iter()
        .map(|ll| Point(ll.to_point()))
        .collect())
}

/// Parse a string into a single `Point`. Throws on malformed or empty input.
#[wasm_bindgen(js_name = "parsePoint")]
pub fn parse_point(s: &str) -> Result<Point, JsValue> {
    let lls = parse_latlngs_strict(s)?;
    let first = lls
        .first()
        .ok_or_else(|| js_err("expected a single `lat:lng` pair, got nothing"))?;
    Ok(Point(first.to_point()))
}

/// Parse lat:lng pairs into `LatLng` values. Throws on malformed input.
#[wasm_bindgen(js_name = "parseLatLngs")]
pub fn parse_latlngs(s: &str) -> Result<Vec<LatLng>, JsValue> {
    Ok(parse_latlngs_strict(s)?.into_iter().map(LatLng).collect())
}

/// Parse a string into a `Rect` (the bounding rectangle of the points).
/// Throws on malformed input.
#[wasm_bindgen(js_name = "makeRect")]
pub fn make_rect(s: &str) -> Result<Rect, JsValue> {
    let mut rect = s2rst::s2::Rect::empty();
    for ll in parse_latlngs_strict(s)? {
        rect = rect.add_point(ll);
    }
    Ok(Rect(rect))
}

/// Parse a string into a `Loop`. Supports `"empty"`/`"full"`.
/// Throws on malformed input.
#[wasm_bindgen(js_name = "makeLoop")]
pub fn make_loop(s: &str) -> Result<Loop, JsValue> {
    match s.trim().to_lowercase().as_str() {
        "empty" => Ok(Loop(s2rst::s2::Loop::empty())),
        "full" => Ok(Loop(s2rst::s2::Loop::full())),
        _ => {
            let pts: Vec<s2rst::s2::Point> = parse_latlngs_strict(s)?
                .into_iter()
                .map(|ll| ll.to_point())
                .collect();
            if pts.is_empty() {
                return Err(js_err(
                    "makeLoop: no vertices parsed; use \"empty\"/\"full\" for sentinel loops",
                ));
            }
            Ok(Loop(s2rst::s2::Loop::new(pts)))
        }
    }
}

/// Parse a string into a `Polygon`.
/// Loops are separated by `";"`, e.g. `"0:0, 0:1, 1:0; 0.1:0.1, 0.1:0.2, 0.2:0.1"`.
/// Supports `"empty"`/`"full"`. Throws on malformed input.
#[wasm_bindgen(js_name = "makePolygon")]
pub fn make_polygon(s: &str) -> Result<Polygon, JsValue> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("empty") {
        return Ok(Polygon(s2rst::s2::Polygon::empty()));
    }
    if trimmed.eq_ignore_ascii_case("full") {
        return Ok(Polygon(s2rst::s2::Polygon::full()));
    }
    let mut loops = Vec::new();
    for part in trimmed.split(';') {
        if part.trim().is_empty() {
            continue;
        }
        let pts = parse_latlngs_strict(part)?
            .into_iter()
            .map(|ll| ll.to_point())
            .collect();
        loops.push(s2rst::s2::Loop::new(pts));
    }
    Ok(Polygon(s2rst::s2::Polygon::from_loops(loops)))
}

/// Parse a string into a `Polyline`. Throws on malformed input.
#[wasm_bindgen(js_name = "makePolyline")]
pub fn make_polyline(s: &str) -> Result<Polyline, JsValue> {
    let pts = parse_latlngs_strict(s)?
        .into_iter()
        .map(|ll| ll.to_point())
        .collect();
    Ok(Polyline(s2rst::s2::polyline::Polyline::new(pts)))
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
