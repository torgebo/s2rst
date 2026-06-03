// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::ChordAngle;
use crate::cap::Cap;
use crate::cell_id::CellId;
use crate::latlng::LatLng;
use crate::point::Point;
use crate::rect::Rect;

/// A cell in the S2 cell hierarchy, representing a quadrilateral on the sphere.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Cell(pub(crate) s2rst::s2::Cell);

#[wasm_bindgen]
impl Cell {
    /// Create from a `CellId`.
    #[wasm_bindgen(js_name = "fromCellId")]
    pub fn from_cell_id(id: &CellId) -> Cell {
        Cell(s2rst::s2::Cell::from_cell_id(id.0))
    }

    /// Create from a `Point`.
    #[wasm_bindgen(js_name = "fromPoint")]
    pub fn from_point(p: &Point) -> Cell {
        Cell(s2rst::s2::Cell::from_point(p.0))
    }

    /// Create from a `LatLng`.
    #[wasm_bindgen(js_name = "fromLatLng")]
    pub fn from_lat_lng(ll: &LatLng) -> Cell {
        Cell(s2rst::s2::Cell::from_lat_lng(ll.0))
    }

    /// Create from a face number (0–5).
    #[wasm_bindgen(js_name = "fromFace")]
    pub fn from_face(face: u8) -> Cell {
        Cell(s2rst::s2::Cell::from_face(face))
    }

    /// The face (0–5).
    #[wasm_bindgen(getter)]
    pub fn face(&self) -> u8 {
        self.0.face().into()
    }

    /// The level (0–30).
    #[wasm_bindgen(getter)]
    pub fn level(&self) -> u8 {
        self.0.level().into()
    }

    /// The cell's id.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> CellId {
        CellId(self.0.id())
    }

    /// Whether this is a leaf cell.
    #[wasm_bindgen(js_name = "isLeaf")]
    pub fn is_leaf(&self) -> bool {
        self.0.is_leaf()
    }

    /// Return the k-th vertex (0–3) as a `Point`.
    pub fn vertex(&self, k: usize) -> Point {
        Point(self.0.vertex(k))
    }

    /// Return the k-th edge normal (0–3) as a `Point`.
    pub fn edge(&self, k: u8) -> Point {
        let edge = s2rst::s2::CellEdge::ALL[k as usize];
        Point(self.0.edge(edge))
    }

    /// The center of this cell.
    pub fn center(&self) -> Point {
        Point(self.0.center())
    }

    /// Average area for the given level in steradians.
    #[wasm_bindgen(js_name = "averageAreaForLevel")]
    pub fn average_area_for_level(level: u8) -> f64 {
        s2rst::s2::Cell::average_area_for_level(level)
    }

    /// Average area for this cell's level.
    #[wasm_bindgen(js_name = "averageArea")]
    pub fn average_area(&self) -> f64 {
        self.0.average_area()
    }

    /// Approximate area in steradians.
    #[wasm_bindgen(js_name = "approxArea")]
    pub fn approx_area(&self) -> f64 {
        self.0.approx_area()
    }

    /// Exact area in steradians.
    #[wasm_bindgen(js_name = "exactArea")]
    pub fn exact_area(&self) -> f64 {
        self.0.exact_area()
    }

    /// Whether this cell contains the given point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Bounding cap.
    #[wasm_bindgen(js_name = "capBound")]
    pub fn cap_bound(&self) -> Cap {
        Cap(self.0.cap_bound())
    }

    /// Bounding lat/lng rectangle.
    #[wasm_bindgen(js_name = "rectBound")]
    pub fn rect_bound(&self) -> Rect {
        Rect(self.0.rect_bound())
    }

    /// Minimum distance to a point.
    #[wasm_bindgen(js_name = "distanceToPoint")]
    pub fn distance_to_point(&self, p: &Point) -> ChordAngle {
        ChordAngle(self.0.distance_to_point(p.0))
    }

    /// Maximum distance to a point.
    #[wasm_bindgen(js_name = "maxDistanceToPoint")]
    pub fn max_distance_to_point(&self, p: &Point) -> ChordAngle {
        ChordAngle(self.0.max_distance_to_point(p.0))
    }

    /// Minimum distance to another cell.
    #[wasm_bindgen(js_name = "distanceToCell")]
    pub fn distance_to_cell(&self, other: &Cell) -> ChordAngle {
        ChordAngle(self.0.distance_to_cell(other.0))
    }
}
