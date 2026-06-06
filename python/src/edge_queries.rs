// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for the closest / furthest edge queries over a `ShapeIndex`.
//!
//! A query borrows its index (held alive as a `Py<PyShapeIndex>`) and is
//! constructed fresh per call. Targets are passed as bare geometry — an
//! `S2Point`, an `(S2Point, S2Point)` edge tuple, a `Cell`, or another
//! `ShapeIndex` — and dispatched by type.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s1::ChordAngle;
use s2rst::s2::closest_edge_query as closest;
use s2rst::s2::furthest_edge_query as furthest;

use crate::angle::PyChordAngle;
use crate::cells::PyCell;
use crate::index::PyShapeIndex;
use crate::s2point::PyS2Point;

/// One result of a closest / furthest edge query.
#[pyclass(frozen, name = "EdgeQueryResult", module = "s2rst")]
pub struct PyEdgeQueryResult {
    distance: ChordAngle,
    shape_id: i32,
    edge_id: i32,
}

impl PyEdgeQueryResult {
    fn from_closest(r: closest::Result) -> Self {
        Self {
            distance: r.distance,
            shape_id: r.shape_id.0,
            edge_id: r.edge_id,
        }
    }

    fn from_furthest(r: furthest::Result) -> Self {
        Self {
            distance: r.distance,
            shape_id: r.shape_id.0,
            edge_id: r.edge_id,
        }
    }
}

#[pymethods]
impl PyEdgeQueryResult {
    /// Distance from the target to this edge (or interior).
    #[getter]
    fn distance(&self) -> PyChordAngle {
        PyChordAngle(self.distance)
    }

    /// The id of the shape this result belongs to (-1 if empty).
    #[getter]
    fn shape_id(&self) -> i32 {
        self.shape_id
    }

    /// The edge id within the shape (-1 for a polygon interior).
    #[getter]
    fn edge_id(&self) -> i32 {
        self.edge_id
    }

    /// Whether this result is a polygon interior rather than an edge.
    fn is_interior(&self) -> bool {
        self.shape_id >= 0 && self.edge_id < 0
    }

    /// Whether no edge was found.
    fn is_empty(&self) -> bool {
        self.shape_id < 0
    }

    fn __repr__(&self) -> String {
        format!(
            "EdgeQueryResult(distance={}, shape_id={}, edge_id={})",
            self.distance.to_angle().radians(),
            self.shape_id,
            self.edge_id
        )
    }
}

/// Build the appropriate target from a Python object and pass it (as
/// `&dyn Target`) to `$build`, returning `PyResult<R>`.
macro_rules! dispatch_target {
    ($module:ident, $obj:expr, $build:expr) => {{
        let obj: &Bound<'_, PyAny> = $obj;
        if let Ok(p) = obj.downcast::<PyS2Point>() {
            Ok($build(&$module::PointTarget::new(p.borrow().0)))
        } else if let Ok((a, b)) = obj.extract::<(PyS2Point, PyS2Point)>() {
            Ok($build(&$module::EdgeTarget::new(a.0, b.0)))
        } else if let Ok(c) = obj.downcast::<PyCell>() {
            Ok($build(&$module::CellTarget::new(c.borrow().0)))
        } else if let Ok(ix) = obj.downcast::<PyShapeIndex>() {
            let ix_ref = ix.borrow();
            Ok($build(&$module::ShapeIndexTarget::new(&ix_ref.0)))
        } else {
            Err(PyTypeError::new_err(
                "target must be an S2Point, an (S2Point, S2Point) edge tuple, \
                 a Cell, or a ShapeIndex",
            ))
        }
    }};
}

/// Finds the edge(s) in a `ShapeIndex` closest to a target.
#[pyclass(name = "ClosestEdgeQuery", module = "s2rst")]
pub struct PyClosestEdgeQuery {
    index: Py<PyShapeIndex>,
}

#[pymethods]
impl PyClosestEdgeQuery {
    #[new]
    fn new(index: Py<PyShapeIndex>) -> Self {
        Self { index }
    }

    /// The single closest edge to `target`.
    fn find_closest_edge(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
    ) -> PyResult<PyEdgeQueryResult> {
        let idx = self.index.borrow(py);
        let q = closest::ClosestEdgeQuery::new(&idx.0);
        dispatch_target!(closest, target, |t| PyEdgeQueryResult::from_closest(
            q.find_closest_edge(t)
        ))
    }

    /// Up to `max_results` closest edges, ordered nearest-first.
    #[pyo3(signature = (target, *, max_results=1, max_distance=None, max_error=None, include_interiors=true))]
    fn find_closest_edges(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        max_results: i32,
        max_distance: Option<PyChordAngle>,
        max_error: Option<PyChordAngle>,
        include_interiors: bool,
    ) -> PyResult<Vec<PyEdgeQueryResult>> {
        let idx = self.index.borrow(py);
        let q = closest::ClosestEdgeQuery::new(&idx.0);
        let mut opts = closest::Options {
            max_results,
            include_interiors,
            ..Default::default()
        };
        if let Some(d) = max_distance {
            opts.max_distance = d.0;
        }
        if let Some(e) = max_error {
            opts.max_error = e.0;
        }
        dispatch_target!(closest, target, |t| q
            .find_closest_edges(t, &opts)
            .into_iter()
            .map(PyEdgeQueryResult::from_closest)
            .collect())
    }

    /// The distance from `target` to the nearest edge.
    fn get_distance(&self, py: Python<'_>, target: &Bound<'_, PyAny>) -> PyResult<PyChordAngle> {
        let idx = self.index.borrow(py);
        let q = closest::ClosestEdgeQuery::new(&idx.0);
        dispatch_target!(closest, target, |t| PyChordAngle(q.get_distance(t)))
    }

    /// Whether some edge is closer than `limit` to `target`.
    fn is_distance_less(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        limit: &PyChordAngle,
    ) -> PyResult<bool> {
        let idx = self.index.borrow(py);
        let q = closest::ClosestEdgeQuery::new(&idx.0);
        dispatch_target!(closest, target, |t| q.is_distance_less(t, limit.0))
    }

    fn __repr__(&self) -> String {
        "ClosestEdgeQuery(...)".to_string()
    }
}

/// Finds the edge(s) in a `ShapeIndex` furthest from a target.
#[pyclass(name = "FurthestEdgeQuery", module = "s2rst")]
pub struct PyFurthestEdgeQuery {
    index: Py<PyShapeIndex>,
}

#[pymethods]
impl PyFurthestEdgeQuery {
    #[new]
    fn new(index: Py<PyShapeIndex>) -> Self {
        Self { index }
    }

    /// The single furthest edge from `target`.
    fn find_furthest_edge(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
    ) -> PyResult<PyEdgeQueryResult> {
        let idx = self.index.borrow(py);
        let q = furthest::FurthestEdgeQuery::new(&idx.0);
        dispatch_target!(furthest, target, |t| PyEdgeQueryResult::from_furthest(
            q.find_furthest_edge(t)
        ))
    }

    /// Up to `max_results` furthest edges, ordered furthest-first.
    #[pyo3(signature = (target, *, max_results=1, min_distance=None, max_error=None, include_interiors=true))]
    fn find_furthest_edges(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        max_results: i32,
        min_distance: Option<PyChordAngle>,
        max_error: Option<PyChordAngle>,
        include_interiors: bool,
    ) -> PyResult<Vec<PyEdgeQueryResult>> {
        let idx = self.index.borrow(py);
        let q = furthest::FurthestEdgeQuery::new(&idx.0);
        let mut opts = furthest::Options {
            max_results,
            include_interiors,
            ..Default::default()
        };
        if let Some(d) = min_distance {
            opts.min_distance = d.0;
        }
        if let Some(e) = max_error {
            opts.max_error = e.0;
        }
        dispatch_target!(furthest, target, |t| q
            .find_furthest_edges(t, &opts)
            .into_iter()
            .map(PyEdgeQueryResult::from_furthest)
            .collect())
    }

    /// The distance from `target` to the furthest edge.
    fn get_distance(&self, py: Python<'_>, target: &Bound<'_, PyAny>) -> PyResult<PyChordAngle> {
        let idx = self.index.borrow(py);
        let q = furthest::FurthestEdgeQuery::new(&idx.0);
        dispatch_target!(furthest, target, |t| PyChordAngle(q.get_distance(t)))
    }

    /// Whether some edge is further than `limit` from `target`.
    fn is_distance_greater(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        limit: &PyChordAngle,
    ) -> PyResult<bool> {
        let idx = self.index.borrow(py);
        let q = furthest::FurthestEdgeQuery::new(&idx.0);
        dispatch_target!(furthest, target, |t| q.is_distance_greater(t, limit.0))
    }

    fn __repr__(&self) -> String {
        "FurthestEdgeQuery(...)".to_string()
    }
}
