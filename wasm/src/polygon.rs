// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::cap::Cap;
use crate::cell::Cell;
use crate::point::Point;
use crate::polyline::Polyline;
use crate::rect::Rect;
use crate::s2loop::Loop;

/// A polygon — a region bounded by zero or more loops.
#[wasm_bindgen]
pub struct Polygon(pub(crate) s2rst::s2::Polygon);

#[wasm_bindgen]
impl Polygon {
    /// Create from an array of `S2Loop` objects.
    /// The first loop is the outer boundary; subsequent loops are holes.
    #[wasm_bindgen(constructor)]
    pub fn new(loops: Vec<Loop>) -> Polygon {
        let inner: Vec<s2rst::s2::Loop> = loops.into_iter().map(|l| l.0).collect();
        Polygon(s2rst::s2::Polygon::from_loops(inner))
    }

    /// The empty polygon.
    pub fn empty() -> Polygon {
        Polygon(s2rst::s2::Polygon::empty())
    }

    /// The full polygon (whole sphere).
    pub fn full() -> Polygon {
        Polygon(s2rst::s2::Polygon::full())
    }

    /// Create from a cell.
    #[wasm_bindgen(js_name = "fromCell")]
    pub fn from_cell(cell: &Cell) -> Polygon {
        Polygon(s2rst::s2::Polygon::from_cell(&cell.0))
    }

    /// A copy of this polygon with vertices snapped to the given S2 cell level
    /// (0–30). Builds the snapping pipeline internally.
    pub fn snapped(&self, snap_level: u8) -> Polygon {
        Polygon(s2rst::s2::Polygon::snapped(&self.0, snap_level))
    }

    /// A simplified copy of this polygon, merging vertices and edges according
    /// to the given snap function (see `SnapFunction`).
    pub fn simplified(&self, snap: &crate::snap::SnapFunction) -> Polygon {
        Polygon(s2rst::s2::Polygon::simplified(&self.0, snap.build()))
    }

    /// Whether this is the empty polygon.
    #[wasm_bindgen(js_name = "isEmptyPolygon")]
    pub fn is_empty_polygon(&self) -> bool {
        self.0.is_empty_polygon()
    }

    /// Whether this is the full polygon (whole sphere).
    #[wasm_bindgen(js_name = "isFullPolygon")]
    pub fn is_full_polygon(&self) -> bool {
        self.0.is_full_polygon()
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

    /// Whether the polygon has holes.
    #[wasm_bindgen(js_name = "hasHoles")]
    pub fn has_holes(&self) -> bool {
        self.0.has_holes()
    }

    /// Get the k-th loop. Throws if `k` is out of range.
    #[wasm_bindgen(js_name = "loopAt")]
    pub fn loop_at(&self, k: usize) -> Result<Loop, JsValue> {
        let n = self.0.num_loops();
        if k >= n {
            return Err(crate::error::js_err(format!(
                "loop index {k} out of range (0..{n})"
            )));
        }
        // We must clone — the core loop is borrowed from the polygon.
        Ok(Loop(self.0.loop_at(k).clone()))
    }

    /// Area in steradians.
    pub fn area(&self) -> f64 {
        self.0.area()
    }

    /// Centroid.
    pub fn centroid(&self) -> Point {
        Point(self.0.centroid())
    }

    /// Whether this polygon contains the given point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        use s2rst::s2::Region;
        self.0.contains_point(&p.0)
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

    /// Project a point onto the polygon boundary.
    #[wasm_bindgen(js_name = "projectPoint")]
    pub fn project_point(&self, x: &Point) -> Point {
        Point(self.0.project_point(x.0))
    }

    /// Project a point onto the nearest boundary edge.
    #[wasm_bindgen(js_name = "projectToBoundary")]
    pub fn project_to_boundary(&self, x: &Point) -> Point {
        Point(self.0.project_to_boundary(x.0))
    }

    /// Validate the polygon (loop-level checks). Throws on error.
    pub fn validate(&self) -> Result<(), JsValue> {
        self.0
            .validate()
            .map_err(crate::error::validation_error_to_js)
    }

    /// Full topological validation (self-intersection, nesting, ...). Throws
    /// with a descriptive message if the polygon is invalid.
    #[wasm_bindgen(js_name = "findValidationError")]
    pub fn find_validation_error(&self) -> Result<(), JsValue> {
        match self.0.find_validation_error() {
            Some(e) => Err(crate::error::s2_error_to_js(e)),
            None => Ok(()),
        }
    }

    /// Construct from oriented loops (CCW shells, CW holes), inferring nesting.
    #[wasm_bindgen(js_name = "fromOrientedLoops")]
    pub fn from_oriented_loops(loops: Vec<Loop>) -> Polygon {
        let inner: Vec<s2rst::s2::Loop> = loops.into_iter().map(|l| l.0).collect();
        Polygon(s2rst::s2::Polygon::from_oriented_loops(inner))
    }

    /// Union of many polygons into one (destructive on the inputs).
    #[wasm_bindgen(js_name = "unionAll")]
    pub fn union_all(polygons: Vec<Polygon>) -> Polygon {
        let inner: Vec<s2rst::s2::Polygon> = polygons.into_iter().map(|p| p.0).collect();
        Polygon(s2rst::s2::Polygon::union_all(inner))
    }

    /// Whether this polygon is normalized (no loop contains more than half the sphere).
    #[wasm_bindgen(js_name = "isNormalized")]
    pub fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    /// Whether the boundary is within `maxError` of another polygon's boundary.
    #[wasm_bindgen(js_name = "boundaryNear")]
    pub fn boundary_near(&self, other: &Polygon, max_error: &Angle) -> bool {
        self.0.boundary_near(&other.0, max_error.0)
    }

    /// Approximate containment within a tolerance.
    #[wasm_bindgen(js_name = "approxContains")]
    pub fn approx_contains(&self, other: &Polygon, tolerance: &Angle) -> bool {
        self.0.approx_contains(&other.0, tolerance.0)
    }

    /// Approximate disjointness within a tolerance.
    #[wasm_bindgen(js_name = "approxDisjoint")]
    pub fn approx_disjoint(&self, other: &Polygon, tolerance: &Angle) -> bool {
        self.0.approx_disjoint(&other.0, tolerance.0)
    }

    /// Overlap fractions `[fractionOfA, fractionOfB]` of two polygons' areas.
    #[wasm_bindgen(js_name = "getOverlapFractions")]
    pub fn get_overlap_fractions(a: &mut Polygon, b: &mut Polygon) -> Vec<f64> {
        let (fa, fb) = s2rst::s2::Polygon::get_overlap_fractions(&mut a.0, &mut b.0);
        vec![fa, fb]
    }

    /// Invert in place.
    pub fn invert(&mut self) {
        self.0.invert();
    }

    /// Complement of a polygon.
    pub fn complement(a: &Polygon) -> Polygon {
        Polygon(s2rst::s2::Polygon::complement(&a.0))
    }

    /// Union of two polygons.
    #[wasm_bindgen(js_name = "union")]
    pub fn union_op(a: &mut Polygon, b: &mut Polygon) -> Polygon {
        Polygon(s2rst::s2::Polygon::union(&mut a.0, &mut b.0))
    }

    /// Intersection of two polygons.
    #[wasm_bindgen(js_name = "intersection")]
    pub fn intersection_op(a: &mut Polygon, b: &mut Polygon) -> Polygon {
        Polygon(s2rst::s2::Polygon::intersection(&mut a.0, &mut b.0))
    }

    /// Difference of two polygons (A − B).
    #[wasm_bindgen(js_name = "difference")]
    pub fn difference_op(a: &mut Polygon, b: &mut Polygon) -> Polygon {
        Polygon(s2rst::s2::Polygon::difference(&mut a.0, &mut b.0))
    }

    /// Symmetric difference.
    #[wasm_bindgen(js_name = "symmetricDifference")]
    pub fn symmetric_difference_op(a: &mut Polygon, b: &mut Polygon) -> Polygon {
        Polygon(s2rst::s2::Polygon::symmetric_difference(&mut a.0, &mut b.0))
    }

    /// Whether this polygon equals another.
    pub fn equal(&self, b: &Polygon) -> bool {
        self.0.equal(&b.0)
    }

    /// Whether this polygon contains another.
    #[wasm_bindgen(js_name = "containsPolygon")]
    pub fn contains_polygon(&self, b: &Polygon) -> bool {
        self.0.contains_polygon(&b.0)
    }

    /// Whether this polygon intersects another.
    #[wasm_bindgen(js_name = "intersectsPolygon")]
    pub fn intersects_polygon(&self, b: &Polygon) -> bool {
        self.0.intersects_polygon(&b.0)
    }

    /// Whether the boundary is approximately equal.
    #[wasm_bindgen(js_name = "boundaryApproxEq")]
    pub fn boundary_approx_eq(&self, other: &Polygon, max_error: &Angle) -> bool {
        self.0.boundary_approx_eq(&other.0, max_error.0)
    }

    /// Intersect with a polyline, returning the portions inside the polygon.
    #[wasm_bindgen(js_name = "intersectWithPolyline")]
    pub fn intersect_with_polyline(&mut self, polyline: &Polyline) -> Vec<Polyline> {
        self.0
            .intersect_with_polyline(&polyline.0)
            .into_iter()
            .map(Polyline)
            .collect()
    }

    /// Subtract from a polyline, returning portions outside the polygon.
    #[wasm_bindgen(js_name = "subtractFromPolyline")]
    pub fn subtract_from_polyline(&mut self, polyline: &Polyline) -> Vec<Polyline> {
        self.0
            .subtract_from_polyline(&polyline.0)
            .into_iter()
            .map(Polyline)
            .collect()
    }

    /// Whether this polygon contains the given polyline.
    #[wasm_bindgen(js_name = "containsPolyline")]
    pub fn contains_polyline(&mut self, polyline: &Polyline) -> bool {
        self.0.contains_polyline(&polyline.0)
    }

    /// Whether this polygon intersects the given polyline.
    #[wasm_bindgen(js_name = "intersectsPolyline")]
    pub fn intersects_polyline(&mut self, polyline: &Polyline) -> bool {
        self.0.intersects_polyline(&polyline.0)
    }

    /// Snap level, or -1 if none.
    #[wasm_bindgen(js_name = "getSnapLevel")]
    pub fn get_snap_level(&self) -> i8 {
        self.0
            .get_snap_level()
            .map(|l| -> i8 { u8::from(l) as i8 })
            .unwrap_or(-1)
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
    pub fn decode(bytes: &[u8]) -> Result<Polygon, JsValue> {
        use s2rst::s2::encoding::S2Decode;
        let mut cur = std::io::Cursor::new(bytes);
        s2rst::s2::Polygon::decode(&mut cur)
            .map(Polygon)
            .map_err(crate::error::js_err)
    }
}
