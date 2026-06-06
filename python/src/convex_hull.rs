// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `ConvexHullQuery` — the convex hull of points and shapes.

use pyo3::prelude::*;

use s2rst::s2::convex_hull_query::ConvexHullQuery;

use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::regions::PyCap;
use crate::s2point::PyS2Point;

/// Computes the convex hull of accumulated points, polylines, loops, and
/// polygons as a single `Loop`.
///
/// Add geometry with the `add_*` methods, then call `convex_hull()`. The hull
/// of 0 points is the empty loop; 1–2 points yield a degenerate 3-vertex loop;
/// geometry spanning more than a hemisphere yields the full loop.
#[pyclass(name = "ConvexHullQuery", module = "s2rst")]
pub struct PyConvexHullQuery(ConvexHullQuery);

#[pymethods]
impl PyConvexHullQuery {
    #[new]
    fn new() -> Self {
        Self(ConvexHullQuery::new())
    }

    /// Add a single point to the hull input.
    fn add_point(&mut self, p: &PyS2Point) {
        self.0.add_point(p.0);
    }

    /// Add several points to the hull input.
    fn add_points(&mut self, points: Vec<PyS2Point>) {
        let pts: Vec<_> = points.iter().map(|p| p.0).collect();
        self.0.add_points(&pts);
    }

    /// Add a polyline's vertices to the hull input.
    fn add_polyline(&mut self, polyline: &PyPolyline) {
        self.0.add_polyline(&polyline.0);
    }

    /// Add a loop's vertices to the hull input.
    fn add_loop(&mut self, loop_: &PyLoop) {
        self.0.add_loop(&loop_.0);
    }

    /// Add a polygon's vertices to the hull input.
    fn add_polygon(&mut self, polygon: &PyPolygon) {
        self.0.add_polygon(&polygon.0);
    }

    /// A bounding cap of the accumulated input.
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// Compute the convex hull of all accumulated geometry.
    fn convex_hull(&mut self) -> PyLoop {
        PyLoop(self.0.convex_hull())
    }

    fn __repr__(&self) -> String {
        "ConvexHullQuery(...)".to_string()
    }
}
