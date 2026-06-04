// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for `ShapeIndex` and its spatial queries.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::shape_index::ShapeIndex;

use crate::angle::PyChordAngle;
use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::s2point::PyS2Point;
use crate::shapes::{PyLaxLoop, PyLaxPolygon, PyLaxPolyline, PyPointVector};

/// A spatial index over shapes (loops, polylines, polygons), enabling fast
/// point-containment and nearest-edge queries — the heart of S2's query layer.
#[pyclass(name = "ShapeIndex")]
pub struct PyShapeIndex(ShapeIndex);

#[pymethods]
impl PyShapeIndex {
    #[new]
    fn new() -> Self {
        PyShapeIndex(ShapeIndex::new())
    }

    /// Add a shape (`Loop`, `Polyline`, `Polygon`, `LaxLoop`, `LaxPolyline`,
    /// `LaxPolygon`, or `PointVector`) and return its shape id.
    fn add(&mut self, shape: &Bound<'_, PyAny>) -> PyResult<i32> {
        macro_rules! try_add {
            ($ty:ty) => {
                if let Ok(s) = shape.downcast::<$ty>() {
                    return Ok(self.0.add(Box::new(s.borrow().0.clone())).0);
                }
            };
        }
        try_add!(PyLoop);
        try_add!(PyPolyline);
        try_add!(PyPolygon);
        try_add!(PyLaxLoop);
        try_add!(PyLaxPolyline);
        try_add!(PyLaxPolygon);
        try_add!(PyPointVector);
        Err(PyTypeError::new_err(
            "add() expects a Loop, Polyline, Polygon, LaxLoop, LaxPolyline, \
             LaxPolygon, or PointVector",
        ))
    }

    /// Finalize the index. Call once after adding all shapes, before querying.
    fn build(&mut self) {
        self.0.build();
    }

    /// The number of shapes in the index.
    fn __len__(&self) -> usize {
        self.0.len()
    }

    /// Whether the index has no shapes.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The total number of edges across all shapes.
    fn num_edges(&self) -> usize {
        self.0.num_edges()
    }

    /// Whether the indexed geometry contains `point` (semi-open vertex model).
    fn contains_point(&self, point: &PyS2Point) -> bool {
        ContainsPointQuery::new(&self.0, VertexModel::SemiOpen).contains(point.0)
    }

    /// The ids of the shapes whose interior contains `point`.
    fn containing_shape_ids(&self, point: &PyS2Point) -> Vec<i32> {
        ContainsPointQuery::new(&self.0, VertexModel::SemiOpen)
            .containing_shape_ids(point.0)
            .iter()
            .map(|id| id.0)
            .collect()
    }

    /// The distance from `point` to the nearest edge in the index.
    fn distance_to_point(&self, point: &PyS2Point) -> PyChordAngle {
        let query = ClosestEdgeQuery::new(&self.0);
        PyChordAngle(query.get_distance(&PointTarget::new(point.0)))
    }

    /// Whether any edge is within `limit` of `point`.
    fn is_distance_less_to_point(&self, point: &PyS2Point, limit: &PyChordAngle) -> bool {
        let query = ClosestEdgeQuery::new(&self.0);
        query.is_distance_less(&PointTarget::new(point.0), limit.0)
    }

    fn __repr__(&self) -> String {
        format!(
            "ShapeIndex(shapes={}, edges={})",
            self.0.len(),
            self.0.num_edges()
        )
    }
}
