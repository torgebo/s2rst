// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Lax (relaxed-validity) shape types. Unlike `S2Loop`/`Polygon`/`Polyline`,
//! these tolerate degeneracies (duplicate vertices, zero-length edges, etc.),
//! which makes them convenient inputs to a `ShapeIndex`.

use wasm_bindgen::prelude::*;

use crate::error::js_err;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::s2loop::Loop;

/// A polyline that tolerates degeneracies.
#[wasm_bindgen]
pub struct LaxPolyline(pub(crate) s2rst::s2::lax_polyline::LaxPolyline);

#[wasm_bindgen]
impl LaxPolyline {
    /// Create from an array of vertices.
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: Vec<Point>) -> LaxPolyline {
        let pts: Vec<s2rst::s2::Point> = vertices.iter().map(|p| p.0).collect();
        LaxPolyline(s2rst::s2::lax_polyline::LaxPolyline::new(pts))
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
            return Err(js_err(format!("vertex index {i} out of range (0..{n})")));
        }
        Ok(Point(self.0.vertex(i)))
    }

    /// All vertices.
    pub fn vertices(&self) -> Vec<Point> {
        self.0.vertices().iter().map(|p| Point(*p)).collect()
    }
}

/// A single loop that tolerates degeneracies.
#[wasm_bindgen]
pub struct LaxLoop(pub(crate) s2rst::s2::lax_loop::LaxLoop);

#[wasm_bindgen]
impl LaxLoop {
    /// Create from an array of vertices.
    #[wasm_bindgen(constructor)]
    pub fn new(vertices: Vec<Point>) -> LaxLoop {
        let pts: Vec<s2rst::s2::Point> = vertices.iter().map(|p| p.0).collect();
        LaxLoop(s2rst::s2::lax_loop::LaxLoop::new(pts))
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
            return Err(js_err(format!("vertex index {i} out of range (0..{n})")));
        }
        Ok(Point(self.0.vertex(i)))
    }
}

/// A polygon (possibly with holes) that tolerates degeneracies.
#[wasm_bindgen]
pub struct LaxPolygon(pub(crate) s2rst::s2::lax_polygon::LaxPolygon);

#[wasm_bindgen]
impl LaxPolygon {
    /// Build from an array of loops (each given by its vertices).
    #[wasm_bindgen(js_name = "fromLoops")]
    pub fn from_loops(loops: Vec<Loop>) -> LaxPolygon {
        let owned: Vec<Vec<s2rst::s2::Point>> =
            loops.iter().map(|l| l.0.vertices().to_vec()).collect();
        LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::from_loops_owned(owned))
    }

    /// Build from a (strict) `Polygon`.
    #[wasm_bindgen(js_name = "fromPolygon")]
    pub fn from_polygon(polygon: &Polygon) -> LaxPolygon {
        LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::from_polygon_ref(
            &polygon.0,
        ))
    }

    /// The empty lax polygon.
    pub fn empty() -> LaxPolygon {
        LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::empty())
    }

    /// The full lax polygon (whole sphere).
    pub fn full() -> LaxPolygon {
        LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::full())
    }

    /// Number of loops.
    #[wasm_bindgen(js_name = "numLoops")]
    pub fn num_loops(&self) -> usize {
        self.0.num_loops()
    }

    /// Total number of vertices across all loops.
    #[wasm_bindgen(js_name = "numVertices")]
    pub fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// Number of vertices in the i-th loop. Throws if `i` is out of range.
    #[wasm_bindgen(js_name = "numLoopVertices")]
    pub fn num_loop_vertices(&self, i: usize) -> Result<usize, JsValue> {
        let n = self.0.num_loops();
        if i >= n {
            return Err(js_err(format!("loop index {i} out of range (0..{n})")));
        }
        Ok(self.0.num_loop_vertices(i))
    }

    /// The j-th vertex of the i-th loop. Throws if either index is out of range.
    #[wasm_bindgen(js_name = "loopVertex")]
    pub fn loop_vertex(&self, i: usize, j: usize) -> Result<Point, JsValue> {
        let n = self.0.num_loops();
        if i >= n {
            return Err(js_err(format!("loop index {i} out of range (0..{n})")));
        }
        let m = self.0.num_loop_vertices(i);
        if j >= m {
            return Err(js_err(format!("vertex index {j} out of range (0..{m})")));
        }
        Ok(Point(self.0.loop_vertex(i, j)))
    }

    /// All vertices across all loops.
    pub fn vertices(&self) -> Vec<Point> {
        self.0.all_vertices().iter().map(|p| Point(*p)).collect()
    }
}

// ---------------------------------------------------------------------------
// text_format helpers (strict, mirroring the non-lax parsers)
// ---------------------------------------------------------------------------

/// Parse a lax polyline from `"lat:lng, ..."`. Throws on malformed input.
#[wasm_bindgen(js_name = "makeLaxPolyline")]
pub fn make_lax_polyline(s: &str) -> Result<LaxPolyline, JsValue> {
    let pts: Vec<s2rst::s2::Point> = crate::text_format::parse_latlngs_strict(s)?
        .into_iter()
        .map(|ll| ll.to_point())
        .collect();
    Ok(LaxPolyline(s2rst::s2::lax_polyline::LaxPolyline::new(pts)))
}

/// Parse a lax polygon from text (loops separated by `";"`, plus `"empty"`/
/// `"full"`). Throws on malformed input.
#[wasm_bindgen(js_name = "makeLaxPolygon")]
pub fn make_lax_polygon(s: &str) -> Result<LaxPolygon, JsValue> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("empty") {
        return Ok(LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::empty()));
    }
    if trimmed.eq_ignore_ascii_case("full") {
        return Ok(LaxPolygon(s2rst::s2::lax_polygon::LaxPolygon::full()));
    }
    let mut loops: Vec<Vec<s2rst::s2::Point>> = Vec::new();
    for part in trimmed.split(';') {
        if part.trim().is_empty() {
            continue;
        }
        let pts: Vec<s2rst::s2::Point> = crate::text_format::parse_latlngs_strict(part)?
            .into_iter()
            .map(|ll| ll.to_point())
            .collect();
        loops.push(pts);
    }
    Ok(LaxPolygon(
        s2rst::s2::lax_polygon::LaxPolygon::from_loops_owned(loops),
    ))
}

/// Format a lax polyline as text.
#[wasm_bindgen(js_name = "laxPolylineToString")]
pub fn lax_polyline_to_string(polyline: &LaxPolyline) -> String {
    s2rst::s2::text_format::lax_polyline_to_string(&polyline.0)
}

/// Format a lax polygon as text.
#[wasm_bindgen(js_name = "laxPolygonToString")]
pub fn lax_polygon_to_string(polygon: &LaxPolygon) -> String {
    s2rst::s2::text_format::lax_polygon_to_string(&polygon.0)
}
