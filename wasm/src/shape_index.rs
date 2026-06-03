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
