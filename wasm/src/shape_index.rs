// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::angle::ChordAngle;
use crate::cell_id::CellId;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::s2loop::Loop;

/// A spatial index for shapes on the sphere.
///
/// Add shapes (polygons, polylines, etc.) and then run spatial queries.
/// Query objects (`ClosestEdgeQuery`, `ContainsPointQuery`) borrow this
/// index via shared ownership, so the `ShapeIndex` must stay alive while
/// queries are in use.
#[wasm_bindgen]
pub struct ShapeIndex {
    pub(crate) inner: Rc<RefCell<s2rst::s2::shape_index::ShapeIndex>>,
}

impl Default for ShapeIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl ShapeIndex {
    /// Create a new, empty shape index.
    #[wasm_bindgen(constructor)]
    pub fn new() -> ShapeIndex {
        ShapeIndex {
            inner: Rc::new(RefCell::new(s2rst::s2::shape_index::ShapeIndex::new())),
        }
    }

    /// Add a polygon (as an owned shape) and return its shape id.
    #[wasm_bindgen(js_name = "addPolygon")]
    pub fn add_polygon(&mut self, polygon: Polygon) -> i32 {
        self.inner.borrow_mut().add(Box::new(polygon.0)).0
    }

    /// Add a polyline (as an owned shape) and return its shape id.
    #[wasm_bindgen(js_name = "addPolyline")]
    pub fn add_polyline(&mut self, polyline: Polyline) -> i32 {
        self.inner.borrow_mut().add(Box::new(polyline.0)).0
    }

    /// Add a loop (as an owned shape) and return its shape id.
    #[wasm_bindgen(js_name = "addLoop")]
    pub fn add_loop(&mut self, loop_: Loop) -> i32 {
        self.inner.borrow_mut().add(Box::new(loop_.0)).0
    }

    /// Add a lax polygon and return its shape id.
    #[wasm_bindgen(js_name = "addLaxPolygon")]
    pub fn add_lax_polygon(&mut self, polygon: crate::lax::LaxPolygon) -> i32 {
        self.inner.borrow_mut().add(Box::new(polygon.0)).0
    }

    /// Add a lax polyline and return its shape id.
    #[wasm_bindgen(js_name = "addLaxPolyline")]
    pub fn add_lax_polyline(&mut self, polyline: crate::lax::LaxPolyline) -> i32 {
        self.inner.borrow_mut().add(Box::new(polyline.0)).0
    }

    /// Add a lax loop and return its shape id.
    #[wasm_bindgen(js_name = "addLaxLoop")]
    pub fn add_lax_loop(&mut self, loop_: crate::lax::LaxLoop) -> i32 {
        self.inner.borrow_mut().add(Box::new(loop_.0)).0
    }

    /// Build the index (forces lazy construction).
    pub fn build(&mut self) {
        self.inner.borrow_mut().build();
    }

    /// Number of shapes.
    #[wasm_bindgen(js_name = "numShapeIds")]
    pub fn num_shape_ids(&self) -> usize {
        self.inner.borrow().num_shape_ids()
    }

    /// Number of shapes (excluding removed ones).
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    /// Whether empty.
    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    /// Total number of edges across all shapes.
    #[wasm_bindgen(js_name = "numEdges")]
    pub fn num_edges(&self) -> usize {
        self.inner.borrow().num_edges()
    }

    // -- Inline query convenience methods --
    // These avoid the need to create separate query objects for simple cases.

    /// Find the distance from the index to a target point.
    #[wasm_bindgen(js_name = "getDistanceToPoint")]
    pub fn get_distance_to_point(&self, point: &Point) -> ChordAngle {
        let idx = self.inner.borrow();
        let query = s2rst::s2::closest_edge_query::ClosestEdgeQuery::new(&idx);
        let target = s2rst::s2::closest_edge_query::PointTarget::new(point.0);
        ChordAngle(query.get_distance(&target))
    }

    /// Whether any edge is within the given distance of a point.
    #[wasm_bindgen(js_name = "isDistanceLessToPoint")]
    pub fn is_distance_less_to_point(&self, point: &Point, limit: &ChordAngle) -> bool {
        let idx = self.inner.borrow();
        let query = s2rst::s2::closest_edge_query::ClosestEdgeQuery::new(&idx);
        let target = s2rst::s2::closest_edge_query::PointTarget::new(point.0);
        query.is_distance_less(&target, limit.0)
    }

    /// Whether the index contains the point (SEMI_OPEN vertex model).
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        let idx = self.inner.borrow();
        let mut q = s2rst::s2::contains_point_query::ContainsPointQuery::new(
            &idx,
            s2rst::s2::contains_point_query::VertexModel::SemiOpen,
        );
        q.contains(p.0)
    }

    /// IDs of shapes containing the point.
    #[wasm_bindgen(js_name = "containingShapeIds")]
    pub fn containing_shape_ids(&self, p: &Point) -> Vec<i32> {
        let idx = self.inner.borrow();
        let mut q = s2rst::s2::contains_point_query::ContainsPointQuery::new(
            &idx,
            s2rst::s2::contains_point_query::VertexModel::SemiOpen,
        );
        q.containing_shape_ids(p.0).iter().map(|id| id.0).collect()
    }

    /// Locate a cell in the index. Returns "DISJOINT", "SUBDIVIDED", or "INDEXED".
    #[wasm_bindgen(js_name = "locateCell")]
    pub fn locate_cell(&self, cell_id: &CellId) -> String {
        let mut idx = self.inner.borrow_mut();
        idx.build();
        let mut iter = idx.iter();
        let rel = iter.locate_cell_id(cell_id.0);
        match rel {
            s2rst::s2::shape_index::CellRelation::Disjoint => "DISJOINT".to_string(),
            s2rst::s2::shape_index::CellRelation::Subdivided => "SUBDIVIDED".to_string(),
            s2rst::s2::shape_index::CellRelation::Indexed => "INDEXED".to_string(),
        }
    }

    /// Locate a point in the index. Returns true if contained.
    #[wasm_bindgen(js_name = "locatePoint")]
    pub fn locate_point(&self, point: &Point) -> bool {
        let mut idx = self.inner.borrow_mut();
        idx.build();
        let mut iter = idx.iter();
        iter.locate_point(point.0)
    }

    // -- Distance & crossing queries (Tier 1.5) -------------------------------

    /// Up to `maxResults` closest edges to a point (`maxResults <= 0` = no
    /// limit), within `maxDistanceRadians` (`Infinity` = no limit). Sorted by
    /// increasing distance.
    #[wasm_bindgen(js_name = "closestEdgesToPoint")]
    pub fn closest_edges_to_point(
        &self,
        point: &Point,
        max_results: i32,
        max_distance_radians: f64,
    ) -> Vec<EdgeResult> {
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let query = s2rst::s2::closest_edge_query::ClosestEdgeQuery::new(&idx);
        let target = s2rst::s2::closest_edge_query::PointTarget::new(point.0);
        query
            .find_closest_edges(&target, &closest_options(max_results, max_distance_radians))
            .into_iter()
            .map(EdgeResult::from_core)
            .collect()
    }

    /// Up to `maxResults` closest edges to the query edge `(a, b)`.
    #[wasm_bindgen(js_name = "closestEdgesToEdge")]
    pub fn closest_edges_to_edge(
        &self,
        a: &Point,
        b: &Point,
        max_results: i32,
        max_distance_radians: f64,
    ) -> Vec<EdgeResult> {
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let query = s2rst::s2::closest_edge_query::ClosestEdgeQuery::new(&idx);
        let target = s2rst::s2::closest_edge_query::EdgeTarget::new(a.0, b.0);
        query
            .find_closest_edges(&target, &closest_options(max_results, max_distance_radians))
            .into_iter()
            .map(EdgeResult::from_core)
            .collect()
    }

    /// Whether the index contains the point under the given vertex model:
    /// `"open"`, `"semiOpen"`, or `"closed"`. Throws on an unknown model.
    #[wasm_bindgen(js_name = "containsPointWithModel")]
    pub fn contains_point_with_model(&self, p: &Point, model: &str) -> Result<bool, JsValue> {
        let vm = parse_vertex_model(model)?;
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let mut q = s2rst::s2::contains_point_query::ContainsPointQuery::new(&idx, vm);
        Ok(q.contains(p.0))
    }

    /// Edges in the index that cross the query edge `(a, b)`. `crossType` is
    /// `"interior"` (default) or `"all"` (includes shared-vertex touches).
    /// Each result's `distanceRadians` is 0 (not meaningful for crossings).
    #[wasm_bindgen(js_name = "getCrossingEdges")]
    pub fn get_crossing_edges(
        &self,
        a: &Point,
        b: &Point,
        cross_type: &str,
    ) -> Result<Vec<EdgeResult>, JsValue> {
        let ct = parse_crossing_type(cross_type)?;
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let mut q = s2rst::s2::crossing_edge_query::CrossingEdgeQuery::new(&idx);
        let mut out = Vec::new();
        for (shape_id, edges) in q.crossings_edge_map(a.0, b.0, ct) {
            for edge_id in edges {
                out.push(EdgeResult {
                    distance_radians: 0.0,
                    shape_id: shape_id.0,
                    edge_id,
                });
            }
        }
        Ok(out)
    }

    // -- Furthest-edge & Hausdorff (Tier 4.2) ---------------------------------

    /// Up to `maxResults` furthest edges from a point (`maxResults <= 0` = no
    /// limit). Sorted by decreasing distance.
    #[wasm_bindgen(js_name = "furthestEdgesToPoint")]
    pub fn furthest_edges_to_point(&self, point: &Point, max_results: i32) -> Vec<EdgeResult> {
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let query = s2rst::s2::furthest_edge_query::FurthestEdgeQuery::new(&idx);
        let target = s2rst::s2::furthest_edge_query::PointTarget::new(point.0);
        query
            .find_furthest_edges(&target, &furthest_options(max_results))
            .into_iter()
            .map(EdgeResult::from_furthest)
            .collect()
    }

    /// Up to `maxResults` furthest edges from the query edge `(a, b)`.
    #[wasm_bindgen(js_name = "furthestEdgesToEdge")]
    pub fn furthest_edges_to_edge(
        &self,
        a: &Point,
        b: &Point,
        max_results: i32,
    ) -> Vec<EdgeResult> {
        self.inner.borrow_mut().build();
        let idx = self.inner.borrow();
        let query = s2rst::s2::furthest_edge_query::FurthestEdgeQuery::new(&idx);
        let target = s2rst::s2::furthest_edge_query::EdgeTarget::new(a.0, b.0);
        query
            .find_furthest_edges(&target, &furthest_options(max_results))
            .into_iter()
            .map(EdgeResult::from_furthest)
            .collect()
    }

    /// Symmetric Hausdorff distance between this index and `other`, in radians.
    #[wasm_bindgen(js_name = "hausdorffDistance")]
    pub fn hausdorff_distance(&self, other: &ShapeIndex) -> f64 {
        self.inner.borrow_mut().build();
        other.inner.borrow_mut().build();
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let q = s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery::new();
        q.get_distance(&a, &b).to_angle().radians()
    }

    /// Directed Hausdorff distance from this index to `other`, in radians
    /// (the supremum over this index of the nearest distance to `other`).
    #[wasm_bindgen(js_name = "directedHausdorffDistance")]
    pub fn directed_hausdorff_distance(&self, other: &ShapeIndex) -> f64 {
        self.inner.borrow_mut().build();
        other.inner.borrow_mut().build();
        let a = self.inner.borrow();
        let b = other.inner.borrow();
        let q = s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery::new();
        q.get_directed_distance(&b, &a).to_angle().radians()
    }
}

fn furthest_options(max_results: i32) -> s2rst::s2::furthest_edge_query::Options {
    let mut opts = s2rst::s2::furthest_edge_query::Options::default();
    if max_results > 0 {
        opts.max_results = max_results;
    }
    opts
}

fn closest_options(
    max_results: i32,
    max_distance_radians: f64,
) -> s2rst::s2::closest_edge_query::Options {
    let mut opts = s2rst::s2::closest_edge_query::Options::default();
    if max_results > 0 {
        opts.max_results = max_results;
    }
    if max_distance_radians.is_finite() {
        opts.max_distance = s2rst::s1::ChordAngle::from_radians(max_distance_radians);
    }
    opts
}

fn parse_vertex_model(s: &str) -> Result<s2rst::s2::contains_point_query::VertexModel, JsValue> {
    use s2rst::s2::contains_point_query::VertexModel;
    match s {
        "open" => Ok(VertexModel::Open),
        "semiOpen" | "semi_open" => Ok(VertexModel::SemiOpen),
        "closed" => Ok(VertexModel::Closed),
        other => Err(crate::error::js_err(format!(
            "unknown vertex model {other:?}; expected \"open\", \"semiOpen\", or \"closed\""
        ))),
    }
}

fn parse_crossing_type(s: &str) -> Result<s2rst::s2::crossing_edge_query::CrossingType, JsValue> {
    use s2rst::s2::crossing_edge_query::CrossingType;
    match s {
        "" | "interior" => Ok(CrossingType::Interior),
        "all" => Ok(CrossingType::All),
        other => Err(crate::error::js_err(format!(
            "unknown crossing type {other:?}; expected \"interior\" or \"all\""
        ))),
    }
}

/// A query result referencing an edge in a `ShapeIndex`.
#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
pub struct EdgeResult {
    distance_radians: f64,
    shape_id: i32,
    edge_id: i32,
}

#[wasm_bindgen]
impl EdgeResult {
    /// Distance from the target to this edge, in radians (0 for crossing results).
    #[wasm_bindgen(getter, js_name = "distanceRadians")]
    pub fn distance_radians(&self) -> f64 {
        self.distance_radians
    }

    /// The shape's id within the index.
    #[wasm_bindgen(getter, js_name = "shapeId")]
    pub fn shape_id(&self) -> i32 {
        self.shape_id
    }

    /// The edge's id within the shape (−1 denotes a shape interior).
    #[wasm_bindgen(getter, js_name = "edgeId")]
    pub fn edge_id(&self) -> i32 {
        self.edge_id
    }
}

impl EdgeResult {
    fn from_core(r: s2rst::s2::closest_edge_query::Result) -> EdgeResult {
        EdgeResult {
            distance_radians: r.distance.to_angle().radians(),
            shape_id: r.shape_id.0,
            edge_id: r.edge_id,
        }
    }

    fn from_furthest(r: s2rst::s2::furthest_edge_query::Result) -> EdgeResult {
        EdgeResult {
            distance_radians: r.distance.to_angle().radians(),
            shape_id: r.shape_id.0,
            edge_id: r.edge_id,
        }
    }
}

// ---------------------------------------------------------------------------
// BooleanOperation convenience functions
// ---------------------------------------------------------------------------

/// Test whether index A contains index B.
#[wasm_bindgen(js_name = "booleanContains")]
pub fn boolean_contains(a: &mut ShapeIndex, b: &mut ShapeIndex) -> bool {
    let options = s2rst::s2::boolean_operation::Options::default();
    s2rst::s2::boolean_operation::S2BooleanOperation::contains(
        &mut a.inner.borrow_mut(),
        &mut b.inner.borrow_mut(),
        options,
    )
}

/// Test whether index A intersects index B.
#[wasm_bindgen(js_name = "booleanIntersects")]
pub fn boolean_intersects(a: &mut ShapeIndex, b: &mut ShapeIndex) -> bool {
    let options = s2rst::s2::boolean_operation::Options::default();
    s2rst::s2::boolean_operation::S2BooleanOperation::intersects(
        &mut a.inner.borrow_mut(),
        &mut b.inner.borrow_mut(),
        options,
    )
}

/// Test whether index A equals index B.
#[wasm_bindgen(js_name = "booleanEquals")]
pub fn boolean_equals(a: &mut ShapeIndex, b: &mut ShapeIndex) -> bool {
    let options = s2rst::s2::boolean_operation::Options::default();
    s2rst::s2::boolean_operation::S2BooleanOperation::equals(
        &mut a.inner.borrow_mut(),
        &mut b.inner.borrow_mut(),
        options,
    )
}

// -- Text round-trip (Tier 1.6) ----------------------------------------------

/// Parse a `ShapeIndex` from the `"points # polylines # polygons"` text format.
/// Throws unless there are exactly two `#` separators (core would otherwise
/// panic, producing an uncatchable trap).
#[wasm_bindgen(js_name = "makeIndex")]
pub fn make_index(s: &str) -> Result<ShapeIndex, JsValue> {
    if s.split('#').count() != 3 {
        return Err(crate::error::js_err(
            "makeIndex: expected exactly two '#' separators (points # polylines # polygons)",
        ));
    }
    let idx = s2rst::s2::text_format::make_index(s);
    Ok(ShapeIndex {
        inner: Rc::new(RefCell::new(idx)),
    })
}

/// Format a `ShapeIndex` in the `"points # polylines # polygons"` text format.
#[wasm_bindgen(js_name = "indexToString")]
pub fn index_to_string(index: &ShapeIndex) -> String {
    let guard = index.inner.borrow();
    s2rst::s2::text_format::index_to_string(&guard)
}
