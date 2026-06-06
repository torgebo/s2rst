// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use std::sync::{Arc, Mutex, MutexGuard};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s2;
use s2rst::s2::edge_vector_shape::EdgeVectorShape;
use s2rst::s2::lax_loop::LaxLoop;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::point_vector::PointVector;
use s2rst::s2::shape::{Dimension, Edge, ReferencePoint, Shape};

use crate::hash_util::hash_f64s;
use crate::s2point::{PyS2Point, PyS2PointIter};

// Poison recovery: a poisoned mutex means a previous operation panicked while
// holding the lock. EdgeVectorShape is a plain Vec<Edge> + Dimension with no
// internal invariants that a partial operation could violate, so reading or
// continuing to mutate the inner value is safe. We never propagate the panic
// across the FFI boundary (which would abort the Python interpreter).
fn lock_evs(m: &Mutex<EdgeVectorShape>) -> MutexGuard<'_, EdgeVectorShape> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ---------------------------------------------------------------------------
// Edge (value type returned by Shape.edge)
// ---------------------------------------------------------------------------

/// An edge of a Shape: a pair of S2Points (v0, v1).
#[pyclass(name = "Edge", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyEdge(pub(crate) Edge);

#[pymethods]
impl PyEdge {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create an edge from two endpoints.
    #[new]
    fn new(v0: &PyS2Point, v1: &PyS2Point) -> Self {
        PyEdge(Edge::new(v0.0, v1.0))
    }

    /// First endpoint.
    #[getter]
    fn v0(&self) -> PyS2Point {
        PyS2Point(self.0.v0)
    }

    /// Second endpoint.
    #[getter]
    fn v1(&self) -> PyS2Point {
        PyS2Point(self.0.v1)
    }

    /// Edge with v0 and v1 swapped.
    fn reversed(&self) -> Self {
        PyEdge(self.0.reversed())
    }

    /// Whether the two endpoints are identical.
    fn is_degenerate(&self) -> bool {
        self.0.is_degenerate()
    }

    fn __eq__(&self, other: &PyEdge) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[
            self.0.v0.x(),
            self.0.v0.y(),
            self.0.v0.z(),
            self.0.v1.x(),
            self.0.v1.y(),
            self.0.v1.z(),
        ])
    }

    fn __repr__(&self) -> String {
        format!(
            "Edge(({:.6}, {:.6}, {:.6}), ({:.6}, {:.6}, {:.6}))",
            self.0.v0.x(),
            self.0.v0.y(),
            self.0.v0.z(),
            self.0.v1.x(),
            self.0.v1.y(),
            self.0.v1.z(),
        )
    }
}

// ---------------------------------------------------------------------------
// ReferencePoint (value type returned by Shape.reference_point)
// ---------------------------------------------------------------------------

/// A reference point with a containment flag, used for dimension-2 shapes.
#[pyclass(name = "ReferencePoint", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyReferencePoint(pub(crate) ReferencePoint);

#[pymethods]
impl PyReferencePoint {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// The reference point on the sphere.
    #[getter]
    fn point(&self) -> PyS2Point {
        PyS2Point(self.0.point)
    }

    /// Whether the reference point is contained in the shape's interior.
    #[getter]
    fn contained(&self) -> bool {
        self.0.contained
    }

    fn __eq__(&self, other: &PyReferencePoint) -> bool {
        self.0.point == other.0.point && self.0.contained == other.0.contained
    }

    fn __hash__(&self) -> u64 {
        let p = self.0.point;
        hash_f64s(&[
            p.x(),
            p.y(),
            p.z(),
            if self.0.contained { 1.0 } else { 0.0 },
        ])
    }

    fn __repr__(&self) -> String {
        format!("ReferencePoint(contained={})", self.0.contained)
    }
}

// ---------------------------------------------------------------------------
// PyShape — enum dispatch wrapper for any concrete shape
// ---------------------------------------------------------------------------

/// A polymorphic Shape object wrapping any concrete shape type.
#[pyclass(name = "Shape", module = "s2rst")]
#[derive(Clone)]
pub struct PyShape {
    inner: ShapeKind,
}

enum ShapeKind {
    LaxLoop(LaxLoop),
    LaxPolyline(LaxPolyline),
    LaxPolygon(LaxPolygon),
    PointVector(PointVector),
    EdgeVector(Arc<Mutex<EdgeVectorShape>>),
}

impl Clone for ShapeKind {
    fn clone(&self) -> Self {
        match self {
            ShapeKind::LaxLoop(s) => ShapeKind::LaxLoop(s.clone()),
            ShapeKind::LaxPolyline(s) => ShapeKind::LaxPolyline(s.clone()),
            ShapeKind::LaxPolygon(s) => ShapeKind::LaxPolygon(s.clone()),
            ShapeKind::PointVector(s) => ShapeKind::PointVector(s.clone()),
            ShapeKind::EdgeVector(s) => ShapeKind::EdgeVector(s.clone()),
        }
    }
}

impl ShapeKind {
    fn with_shape<T>(&self, f: impl FnOnce(&dyn Shape) -> T) -> T {
        match self {
            ShapeKind::LaxLoop(s) => f(s),
            ShapeKind::LaxPolyline(s) => f(s),
            ShapeKind::LaxPolygon(s) => f(s),
            ShapeKind::PointVector(s) => f(s),
            ShapeKind::EdgeVector(s) => {
                let guard = lock_evs(s);
                f(&*guard)
            }
        }
    }
}

impl PyShape {
    /// Run `f` with a `&dyn Shape` view of the inner shape (crate-internal,
    /// used by the chain-interpolation query and shape utilities).
    pub(crate) fn with_shape<T>(&self, f: impl FnOnce(&dyn Shape) -> T) -> T {
        self.inner.with_shape(f)
    }
}

#[pymethods]
impl PyShape {
    fn __copy__(&self) -> Self {
        self.clone()
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        let inner = match &self.inner {
            ShapeKind::LaxLoop(s) => ShapeKind::LaxLoop(s.clone()),
            ShapeKind::LaxPolyline(s) => ShapeKind::LaxPolyline(s.clone()),
            ShapeKind::LaxPolygon(s) => ShapeKind::LaxPolygon(s.clone()),
            ShapeKind::PointVector(s) => ShapeKind::PointVector(s.clone()),
            ShapeKind::EdgeVector(s) => {
                ShapeKind::EdgeVector(Arc::new(Mutex::new(lock_evs(s).clone())))
            }
        };
        PyShape { inner }
    }

    /// Number of edges in this shape.
    fn num_edges(&self) -> usize {
        self.inner.with_shape(|s| s.num_edges())
    }

    /// The i-th edge.
    fn edge(&self, id: usize) -> PyResult<PyEdge> {
        self.inner.with_shape(|s| {
            if id >= s.num_edges() {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "edge index out of range",
                ));
            }
            Ok(PyEdge(s.edge(id)))
        })
    }

    /// The geometric dimension: 0 (points), 1 (polylines), 2 (polygons).
    fn dimension(&self) -> u8 {
        self.inner.with_shape(|s| u8::from(s.dimension()))
    }

    /// Number of edge chains.
    fn num_chains(&self) -> usize {
        self.inner.with_shape(|s| s.num_chains())
    }

    /// The k-th chain as (start, length).
    fn chain(&self, chain_id: usize) -> PyResult<(usize, usize)> {
        self.inner.with_shape(|s| {
            if chain_id >= s.num_chains() {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "chain index out of range",
                ));
            }
            let c = s.chain(chain_id);
            Ok((c.start, c.length))
        })
    }

    /// An edge within a chain, specified by (chain_id, offset).
    fn chain_edge(&self, chain_id: usize, offset: usize) -> PyResult<PyEdge> {
        self.inner.with_shape(|s| {
            if chain_id >= s.num_chains() {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "chain index out of range",
                ));
            }
            let c = s.chain(chain_id);
            if offset >= c.length {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "chain offset out of range",
                ));
            }
            Ok(PyEdge(s.chain_edge(chain_id, offset)))
        })
    }

    /// The chain and offset for a given edge id.
    fn chain_position(&self, edge_id: usize) -> PyResult<(usize, usize)> {
        self.inner.with_shape(|s| {
            if edge_id >= s.num_edges() {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "edge index out of range",
                ));
            }
            let cp = s.chain_position(edge_id);
            Ok((cp.chain_id, cp.offset))
        })
    }

    /// A reference point with containment flag (for dimension-2 shapes).
    fn reference_point(&self) -> PyReferencePoint {
        self.inner
            .with_shape(|s| PyReferencePoint(s.reference_point()))
    }

    /// Whether this shape has no edges and no interior.
    fn is_empty(&self) -> bool {
        self.inner.with_shape(|s| s.is_empty())
    }

    /// Whether this shape contains all points on the sphere.
    fn is_full(&self) -> bool {
        self.inner.with_shape(|s| s.is_full())
    }

    /// Whether this shape has an interior (dimension == 2).
    fn has_interior(&self) -> bool {
        self.inner.with_shape(|s| s.has_interior())
    }

    fn __repr__(&self) -> String {
        let kind = match &self.inner {
            ShapeKind::LaxLoop(_) => "LaxLoop",
            ShapeKind::LaxPolyline(_) => "LaxPolyline",
            ShapeKind::LaxPolygon(_) => "LaxPolygon",
            ShapeKind::PointVector(_) => "PointVector",
            ShapeKind::EdgeVector(_) => "EdgeVector",
        };
        self.inner.with_shape(|s| {
            format!(
                "Shape({}, dim={}, edges={}, chains={})",
                kind,
                u8::from(s.dimension()),
                s.num_edges(),
                s.num_chains(),
            )
        })
    }
}

// ---------------------------------------------------------------------------
// LaxLoop
// ---------------------------------------------------------------------------

/// A lightweight closed loop (dimension 2).
#[pyclass(name = "LaxLoop", module = "s2rst")]
#[derive(Clone)]
pub struct PyLaxLoop(pub(crate) LaxLoop);

#[pymethods]
impl PyLaxLoop {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    /// Create from a list of vertices in CCW order. The closing edge is implicit.
    #[new]
    fn new(vertices: Vec<PyS2Point>) -> Self {
        PyLaxLoop(LaxLoop::new(vertices.into_iter().map(|p| p.0).collect()))
    }

    /// Number of vertices.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// The i-th vertex.
    fn vertex(&self, i: usize) -> PyResult<PyS2Point> {
        if i >= self.0.num_vertices() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index out of range",
            ));
        }
        Ok(PyS2Point(self.0.vertex(i)))
    }

    /// Wrap as a polymorphic Shape.
    fn as_shape(&self) -> PyShape {
        PyShape {
            inner: ShapeKind::LaxLoop(self.0.clone()),
        }
    }

    fn __len__(&self) -> usize {
        self.0.num_vertices()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyS2Point> {
        let n = self.0.num_vertices() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyS2Point(self.0.vertex(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyS2PointIter {
        PyS2PointIter::new(
            (0..self.0.num_vertices())
                .map(|i| self.0.vertex(i))
                .collect(),
        )
    }

    fn __repr__(&self) -> String {
        format!("LaxLoop({} vertices)", self.0.num_vertices())
    }
}

// ---------------------------------------------------------------------------
// LaxPolyline
// ---------------------------------------------------------------------------

/// A lightweight open polyline (dimension 1).
#[pyclass(name = "LaxPolyline", module = "s2rst")]
#[derive(Clone)]
pub struct PyLaxPolyline(pub(crate) LaxPolyline);

#[pymethods]
impl PyLaxPolyline {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    /// Create from a list of vertices in order.
    #[new]
    fn new(vertices: Vec<PyS2Point>) -> Self {
        PyLaxPolyline(LaxPolyline::new(
            vertices.into_iter().map(|p| p.0).collect(),
        ))
    }

    /// Number of vertices.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// The i-th vertex.
    fn vertex(&self, i: usize) -> PyResult<PyS2Point> {
        if i >= self.0.num_vertices() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index out of range",
            ));
        }
        Ok(PyS2Point(self.0.vertex(i)))
    }

    /// Wrap as a polymorphic Shape.
    fn as_shape(&self) -> PyShape {
        PyShape {
            inner: ShapeKind::LaxPolyline(self.0.clone()),
        }
    }

    fn __len__(&self) -> usize {
        self.0.num_vertices()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyS2Point> {
        let n = self.0.num_vertices() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyS2Point(self.0.vertex(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyS2PointIter {
        PyS2PointIter::new(
            (0..self.0.num_vertices())
                .map(|i| self.0.vertex(i))
                .collect(),
        )
    }

    fn __repr__(&self) -> String {
        format!("LaxPolyline({} vertices)", self.0.num_vertices())
    }
}

// ---------------------------------------------------------------------------
// LaxPolygon
// ---------------------------------------------------------------------------

/// A lightweight polygon with multiple loops (dimension 2).
#[pyclass(name = "LaxPolygon", module = "s2rst")]
#[derive(Clone)]
pub struct PyLaxPolygon(pub(crate) LaxPolygon);

#[pymethods]
impl PyLaxPolygon {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    fn __bool__(&self) -> bool {
        !Shape::is_empty(&self.0)
    }

    /// Create from a list of loops, where each loop is a list of S2Points.
    #[new]
    fn new(loops: Vec<Vec<PyS2Point>>) -> Self {
        let owned: Vec<Vec<s2::Point>> = loops
            .into_iter()
            .map(|l| l.into_iter().map(|p| p.0).collect())
            .collect();
        PyLaxPolygon(LaxPolygon::from_loops_owned(owned))
    }

    /// The empty polygon (no loops, no interior).
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyLaxPolygon(LaxPolygon::empty())
    }

    /// The full polygon (covers entire sphere).
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyLaxPolygon(LaxPolygon::full())
    }

    /// Number of loops.
    fn num_loops(&self) -> usize {
        self.0.num_loops()
    }

    /// Total number of vertices across all loops.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// Number of vertices in the i-th loop.
    fn num_loop_vertices(&self, i: usize) -> PyResult<usize> {
        if i >= self.0.num_loops() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "loop index out of range",
            ));
        }
        Ok(self.0.num_loop_vertices(i))
    }

    /// The j-th vertex of the i-th loop.
    fn loop_vertex(&self, i: usize, j: usize) -> PyResult<PyS2Point> {
        if i >= self.0.num_loops() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "loop index out of range",
            ));
        }
        if j >= self.0.num_loop_vertices(i) {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index out of range",
            ));
        }
        Ok(PyS2Point(self.0.loop_vertex(i, j)))
    }

    /// Wrap as a polymorphic Shape.
    fn as_shape(&self) -> PyShape {
        PyShape {
            inner: ShapeKind::LaxPolygon(self.0.clone()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "LaxPolygon({} loops, {} vertices)",
            self.0.num_loops(),
            self.0.num_vertices()
        )
    }
}

// ---------------------------------------------------------------------------
// PointVector
// ---------------------------------------------------------------------------

/// A set of points as a shape (dimension 0).
#[pyclass(name = "PointVector", module = "s2rst")]
#[derive(Clone)]
pub struct PyPointVector(pub(crate) PointVector);

#[pymethods]
impl PyPointVector {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    /// Create from a list of points.
    #[new]
    fn new(points: Vec<PyS2Point>) -> Self {
        PyPointVector(PointVector::new(points.into_iter().map(|p| p.0).collect()))
    }

    /// The i-th point.
    fn point(&self, i: usize) -> PyResult<PyS2Point> {
        if i >= self.0.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "point index out of range",
            ));
        }
        Ok(PyS2Point(self.0.point(i)))
    }

    /// Wrap as a polymorphic Shape.
    fn as_shape(&self) -> PyShape {
        PyShape {
            inner: ShapeKind::PointVector(self.0.clone()),
        }
    }

    fn __len__(&self) -> usize {
        self.0.len()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyS2Point> {
        let n = self.0.len() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyS2Point(self.0.point(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyS2PointIter {
        PyS2PointIter::new((0..self.0.len()).map(|i| self.0.point(i)).collect())
    }

    fn __repr__(&self) -> String {
        format!("PointVector({} points)", self.0.len())
    }
}

// ---------------------------------------------------------------------------
// EdgeVectorShape
// ---------------------------------------------------------------------------

/// An arbitrary collection of edges as a shape (default dimension 1).
#[pyclass(name = "EdgeVectorShape", module = "s2rst")]
pub struct PyEdgeVectorShape(pub(crate) Arc<Mutex<EdgeVectorShape>>);

#[pymethods]
impl PyEdgeVectorShape {
    fn __copy__(&self) -> Self {
        PyEdgeVectorShape(Arc::new(Mutex::new(lock_evs(&self.0).clone())))
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        PyEdgeVectorShape(Arc::new(Mutex::new(lock_evs(&self.0).clone())))
    }

    /// Create an empty edge vector.
    #[new]
    fn new() -> Self {
        PyEdgeVectorShape(Arc::new(Mutex::new(EdgeVectorShape::new())))
    }

    /// Create from a list of Edge objects.
    #[classmethod]
    fn from_edges(_cls: &Bound<'_, PyType>, edges: Vec<PyEdge>) -> Self {
        let raw: Vec<(s2::Point, s2::Point)> = edges.iter().map(|e| (e.0.v0, e.0.v1)).collect();
        PyEdgeVectorShape(Arc::new(Mutex::new(EdgeVectorShape::from_edges(raw))))
    }

    /// Create from a single edge.
    #[classmethod]
    fn from_edge(_cls: &Bound<'_, PyType>, a: &PyS2Point, b: &PyS2Point) -> Self {
        PyEdgeVectorShape(Arc::new(Mutex::new(EdgeVectorShape::from_edge(a.0, b.0))))
    }

    /// Add an edge. Only valid before converting to a Shape.
    fn add(&self, a: &PyS2Point, b: &PyS2Point) {
        lock_evs(&self.0).add(a.0, b.0);
    }

    /// Override the dimension (0, 1, or 2). Only valid before converting to a Shape.
    fn set_dimension(&self, dim: u8) -> PyResult<()> {
        let d = Dimension::try_from(dim)
            .map_err(|_| pyo3::exceptions::PyValueError::new_err("dimension must be 0, 1, or 2"))?;
        lock_evs(&self.0).set_dimension(d);
        Ok(())
    }

    /// Wrap as a polymorphic Shape.
    fn as_shape(&self) -> PyShape {
        PyShape {
            inner: ShapeKind::EdgeVector(self.0.clone()),
        }
    }

    fn __len__(&self) -> usize {
        lock_evs(&self.0).num_edges()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyEdge> {
        let g = lock_evs(&self.0);
        let n = g.num_edges() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyEdge(g.edge(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyEdgeIter {
        let g = lock_evs(&self.0);
        let edges: Vec<Edge> = (0..g.num_edges()).map(|i| g.edge(i)).collect();
        PyEdgeIter { edges, idx: 0 }
    }

    fn __repr__(&self) -> String {
        format!("EdgeVectorShape({} edges)", lock_evs(&self.0).num_edges())
    }
}

/// Snapshot-based iterator over Edges. Constructed by `EdgeVectorShape.__iter__`.
#[pyclass]
struct PyEdgeIter {
    edges: Vec<Edge>,
    idx: usize,
}

#[pymethods]
impl PyEdgeIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyEdge> {
        if self.idx < self.edges.len() {
            let e = self.edges[self.idx];
            self.idx += 1;
            Some(PyEdge(e))
        } else {
            None
        }
    }
}
