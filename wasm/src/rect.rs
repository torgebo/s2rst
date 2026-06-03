// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::cap::Cap;
use crate::cell_id::CellId;
use crate::latlng::LatLng;
use crate::point::Point;

/// A latitude/longitude rectangle (S2LatLngRect).
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Rect(pub(crate) s2rst::s2::Rect);

#[wasm_bindgen]
impl Rect {
    /// The empty rectangle.
    pub fn empty() -> Rect {
        Rect(s2rst::s2::Rect::empty())
    }

    /// The full rectangle (whole sphere).
    pub fn full() -> Rect {
        Rect(s2rst::s2::Rect::full())
    }

    /// Create from a single `LatLng`.
    #[wasm_bindgen(js_name = "fromLatLng")]
    pub fn from_lat_lng(ll: &LatLng) -> Rect {
        Rect(s2rst::s2::Rect::from_lat_lng(ll.0))
    }

    /// Create from two corner points.
    #[wasm_bindgen(js_name = "fromPointPair")]
    pub fn from_point_pair(a: &LatLng, b: &LatLng) -> Rect {
        Rect(s2rst::s2::Rect::from_point_pair(a.0, b.0))
    }

    /// Create from center and size (both as LatLng).
    #[wasm_bindgen(js_name = "fromCenterSize")]
    pub fn from_center_size(center: &LatLng, size: &LatLng) -> Rect {
        Rect(s2rst::s2::Rect::from_center_size(center.0, size.0))
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

    #[wasm_bindgen(js_name = "isPoint")]
    pub fn is_point(&self) -> bool {
        self.0.is_point()
    }

    /// Low corner.
    pub fn lo(&self) -> LatLng {
        LatLng(self.0.lo())
    }

    /// High corner.
    pub fn hi(&self) -> LatLng {
        LatLng(self.0.hi())
    }

    /// Center point.
    pub fn center(&self) -> LatLng {
        LatLng(self.0.center())
    }

    /// Size as lat/lng extents.
    pub fn size(&self) -> LatLng {
        LatLng(self.0.size())
    }

    /// Area in steradians.
    pub fn area(&self) -> f64 {
        self.0.area()
    }

    /// Whether this rect contains a lat/lng.
    #[wasm_bindgen(js_name = "containsLatLng")]
    pub fn contains_lat_lng(&self, ll: &LatLng) -> bool {
        self.0.contains_lat_lng(ll.0)
    }

    /// Whether this rect contains a point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this rect contains another rect.
    #[wasm_bindgen(js_name = "containsRect")]
    pub fn contains_rect(&self, other: &Rect) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this rect intersects another.
    #[wasm_bindgen(js_name = "intersectsRect")]
    pub fn intersects_rect(&self, other: &Rect) -> bool {
        self.0.intersects(other.0)
    }

    /// Add a point, expanding the rect.
    #[wasm_bindgen(js_name = "addPoint")]
    pub fn add_point(&self, ll: &LatLng) -> Rect {
        Rect(self.0.add_point(ll.0))
    }

    /// Expand by a margin.
    pub fn expanded(&self, margin: &LatLng) -> Rect {
        Rect(self.0.expanded(margin.0))
    }

    /// Expand by a distance.
    #[wasm_bindgen(js_name = "expandedByDistance")]
    pub fn expanded_by_distance(&self, distance: &Angle) -> Rect {
        Rect(self.0.expanded_by_distance(distance.0))
    }

    /// Union of two rects.
    #[wasm_bindgen(js_name = "union")]
    pub fn union_with(&self, other: &Rect) -> Rect {
        Rect(self.0.union(other.0))
    }

    /// Intersection of two rects.
    #[wasm_bindgen(js_name = "intersection")]
    pub fn intersection_with(&self, other: &Rect) -> Rect {
        Rect(self.0.intersection(other.0))
    }

    /// Bounding cap.
    #[wasm_bindgen(js_name = "capBound")]
    pub fn cap_bound(&self) -> Cap {
        Cap(self.0.cap_bound())
    }

    /// Bounding cell union.
    #[wasm_bindgen(js_name = "cellUnionBound")]
    pub fn cell_union_bound(&self) -> Vec<CellId> {
        self.0.cell_union_bound().into_iter().map(CellId).collect()
    }

    /// Centroid.
    pub fn centroid(&self) -> Point {
        Point(self.0.centroid())
    }

    /// Distance to a lat/lng point.
    #[wasm_bindgen(js_name = "getDistanceToLatLng")]
    pub fn get_distance_to_latlng(&self, p: &LatLng) -> Angle {
        Angle(self.0.get_distance_to_latlng(p.0))
    }

    /// Distance to another rect.
    #[wasm_bindgen(js_name = "getDistance")]
    pub fn get_distance(&self, other: &Rect) -> Angle {
        Angle(self.0.get_distance(other.0))
    }

    /// Hausdorff distance to another rect.
    #[wasm_bindgen(js_name = "getHausdorffDistance")]
    pub fn get_hausdorff_distance(&self, other: &Rect) -> Angle {
        Angle(self.0.get_hausdorff_distance(other.0))
    }

    /// Whether approximately equal.
    #[wasm_bindgen(js_name = "approxEq")]
    pub fn approx_eq(&self, other: &Rect) -> bool {
        self.0.approx_eq(other.0)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.0)
    }
}
