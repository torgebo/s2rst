// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::{Angle, ChordAngle};
use crate::cell_id::CellId;
use crate::point::Point;
use crate::rect::Rect;

/// A spherical cap — the set of points within a given angular distance
/// of a center point.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Cap(pub(crate) s2rst::s2::Cap);

#[wasm_bindgen]
impl Cap {
    /// Create from center and angular radius.
    #[wasm_bindgen(js_name = "fromCenterAngle")]
    pub fn from_center_angle(center: &Point, angle: &Angle) -> Cap {
        Cap(s2rst::s2::Cap::from_center_angle(center.0, angle.0))
    }

    /// Create from center and chord angle radius.
    #[wasm_bindgen(js_name = "fromCenterChordAngle")]
    pub fn from_center_chord_angle(center: &Point, radius: &ChordAngle) -> Cap {
        Cap(s2rst::s2::Cap::from_center_chord_angle(center.0, radius.0))
    }

    /// Create from a single point.
    #[wasm_bindgen(js_name = "fromPoint")]
    pub fn from_point(p: &Point) -> Cap {
        Cap(s2rst::s2::Cap::from_point(p.0))
    }

    /// Create from center and area in steradians.
    #[wasm_bindgen(js_name = "fromCenterArea")]
    pub fn from_center_area(center: &Point, area: f64) -> Cap {
        Cap(s2rst::s2::Cap::from_center_area(center.0, area))
    }

    /// Cap from a center and height (1 − cos(radius)).
    #[wasm_bindgen(js_name = "fromCenterHeight")]
    pub fn from_center_height(center: &Point, height: f64) -> Cap {
        Cap(s2rst::s2::Cap::from_center_height(center.0, height))
    }

    /// The empty cap.
    pub fn empty() -> Cap {
        Cap(s2rst::s2::Cap::empty())
    }

    /// The full cap (whole sphere).
    pub fn full() -> Cap {
        Cap(s2rst::s2::Cap::full())
    }

    /// The center point.
    #[wasm_bindgen(getter)]
    pub fn center(&self) -> Point {
        Point(self.0.center())
    }

    /// The chord-angle radius.
    #[wasm_bindgen(getter, js_name = "chordRadius")]
    pub fn chord_radius(&self) -> ChordAngle {
        ChordAngle(self.0.chord_radius())
    }

    /// The angular radius.
    #[wasm_bindgen(getter, js_name = "angleRadius")]
    pub fn angle_radius(&self) -> Angle {
        Angle(self.0.angle_radius())
    }

    /// The height of the cap.
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> f64 {
        self.0.height()
    }

    /// Area in steradians.
    pub fn area(&self) -> f64 {
        self.0.area()
    }

    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[wasm_bindgen(js_name = "isFull")]
    pub fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Whether this cap contains a point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this cap contains another cap.
    #[wasm_bindgen(js_name = "containsCap")]
    pub fn contains_cap(&self, other: &Cap) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this cap intersects another cap.
    #[wasm_bindgen(js_name = "intersectsCap")]
    pub fn intersects_cap(&self, other: &Cap) -> bool {
        self.0.intersects(other.0)
    }

    /// The complement of this cap.
    pub fn complement(&self) -> Cap {
        Cap(self.0.complement())
    }

    /// Expand by the given angle.
    pub fn expanded(&self, distance: &Angle) -> Cap {
        Cap(self.0.expanded(distance.0))
    }

    /// Union of this cap and another.
    #[wasm_bindgen(js_name = "union")]
    pub fn union_with(&self, other: &Cap) -> Cap {
        Cap(self.0.union(other.0))
    }

    /// Add a point to the cap.
    #[wasm_bindgen(js_name = "addPoint")]
    pub fn add_point(&self, p: &Point) -> Cap {
        Cap(self.0.add_point(p.0))
    }

    /// Whether the cap's interior (excluding boundary) contains the point.
    #[wasm_bindgen(js_name = "interiorContainsPoint")]
    pub fn interior_contains_point(&self, p: &Point) -> bool {
        self.0.interior_contains_point(p.0)
    }

    /// Whether the cap's interior intersects another cap.
    #[wasm_bindgen(js_name = "interiorIntersects")]
    pub fn interior_intersects(&self, other: &Cap) -> bool {
        self.0.interior_intersects(other.0)
    }

    /// The smallest cap containing both this cap and another.
    #[wasm_bindgen(js_name = "addCap")]
    pub fn add_cap(&self, other: &Cap) -> Cap {
        Cap(self.0.add_cap(other.0))
    }

    /// Bounding cap (returns self).
    #[wasm_bindgen(js_name = "capBound")]
    pub fn cap_bound(&self) -> Cap {
        Cap(self.0.cap_bound())
    }

    /// Bounding lat/lng rectangle.
    #[wasm_bindgen(js_name = "rectBound")]
    pub fn rect_bound(&self) -> Rect {
        Rect(self.0.rect_bound())
    }

    /// Bounding cell union.
    #[wasm_bindgen(js_name = "cellUnionBound")]
    pub fn cell_union_bound(&self) -> Vec<CellId> {
        self.0.cell_union_bound().into_iter().map(CellId).collect()
    }

    /// Centroid of the cap.
    pub fn centroid(&self) -> Point {
        Point(self.0.centroid())
    }

    /// Whether approximately equal to another cap.
    #[wasm_bindgen(js_name = "approxEq")]
    pub fn approx_eq(&self, other: &Cap) -> bool {
        self.0.approx_eq(other.0)
    }
}
