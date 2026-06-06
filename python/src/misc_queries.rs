// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for a handful of standalone S2 queries:
//!
//! - [`PyContainsVertexQuery`] — stateful vertex-containment accumulator.
//! - [`PyHausdorffDistanceQuery`] — discrete Hausdorff distance between two
//!   indexed regions.
//! - [`PyShapeNestingQuery`] — shell/hole nesting relationships of a polygon's
//!   chains, exposed as [`PyChainRelation`] values.

use pyo3::prelude::*;

use s2rst::s2::contains_vertex_query::ContainsVertexQuery;
use s2rst::s2::hausdorff_distance_query::{HausdorffOptions, S2HausdorffDistanceQuery};
use s2rst::s2::shape_nesting_query::ShapeNestingQuery;

use crate::angle::PyChordAngle;
use crate::index::PyShapeIndex;
use crate::s2point::PyS2Point;

// ---------------------------------------------------------------------------
// ContainsVertexQuery
// ---------------------------------------------------------------------------

/// Tracks edges entering and leaving a vertex to determine containment.
///
/// Construct with the target vertex, add the incident edges with
/// [`add_edge`](Self::add_edge), then read the result via
/// [`contains_vertex`](Self::contains_vertex).
#[pyclass(name = "ContainsVertexQuery", module = "s2rst")]
pub struct PyContainsVertexQuery {
    inner: ContainsVertexQuery,
}

#[pymethods]
impl PyContainsVertexQuery {
    /// Create a query for the given target vertex.
    #[new]
    fn new(target: &PyS2Point) -> Self {
        PyContainsVertexQuery {
            inner: ContainsVertexQuery::new(target.0),
        }
    }

    /// Add the edge between the target and `v` with the given direction.
    ///
    /// `direction = 1` means outgoing (target -> v), `-1` means incoming
    /// (v -> target), and `0` is degenerate.
    fn add_edge(&mut self, v: &PyS2Point, direction: i32) {
        self.inner.add_edge(v.0, direction);
    }

    /// Report whether the target vertex is contained.
    ///
    /// Returns `1` if contained, `-1` if not, and `0` if the incident edges
    /// formed matched sibling pairs (ambiguous).
    fn contains_vertex(&self) -> i32 {
        self.inner.contains_vertex()
    }

    /// Whether any duplicate edges (same orientation seen twice) were added.
    fn duplicate_edges(&self) -> bool {
        self.inner.duplicate_edges()
    }

    fn __repr__(&self) -> String {
        format!(
            "ContainsVertexQuery(contains_vertex={})",
            self.inner.contains_vertex()
        )
    }
}

// ---------------------------------------------------------------------------
// HausdorffDistanceQuery
// ---------------------------------------------------------------------------

/// Computes discrete (vertex-based) Hausdorff distances between two
/// `ShapeIndex` regions.
#[pyclass(name = "HausdorffDistanceQuery", module = "s2rst")]
pub struct PyHausdorffDistanceQuery {
    inner: S2HausdorffDistanceQuery,
}

// Core's directed query reads only `&ShapeIndex`, so the same Python object may
// safely appear on both sides (e.g. identical-distance checks). No distinctness
// guard is needed.
#[pymethods]
impl PyHausdorffDistanceQuery {
    /// Create a query. With `include_interiors` true (default), points inside a
    /// polygon have zero distance to it.
    #[new]
    #[pyo3(signature = (*, include_interiors = true))]
    fn new(include_interiors: bool) -> Self {
        PyHausdorffDistanceQuery {
            inner: S2HausdorffDistanceQuery::with_options(HausdorffOptions { include_interiors }),
        }
    }

    /// The undirected Hausdorff distance between `a` and `b`
    /// (`ChordAngle.INFINITY` if either index is empty).
    fn get_distance(
        &self,
        a: &Bound<'_, PyShapeIndex>,
        b: &Bound<'_, PyShapeIndex>,
    ) -> PyChordAngle {
        let a = a.borrow();
        let b = b.borrow();
        PyChordAngle(self.inner.get_distance(&a.0, &b.0))
    }

    /// The directed Hausdorff distance from `target` to `source`
    /// (`ChordAngle.INFINITY` if either index is empty).
    fn get_directed_distance(
        &self,
        target: &Bound<'_, PyShapeIndex>,
        source: &Bound<'_, PyShapeIndex>,
    ) -> PyChordAngle {
        let target = target.borrow();
        let source = source.borrow();
        PyChordAngle(self.inner.get_directed_distance(&target.0, &source.0))
    }

    /// Whether the undirected Hausdorff distance between `a` and `b` is less
    /// than `limit`.
    fn is_distance_less(
        &self,
        a: &Bound<'_, PyShapeIndex>,
        b: &Bound<'_, PyShapeIndex>,
        limit: &PyChordAngle,
    ) -> bool {
        let a = a.borrow();
        let b = b.borrow();
        self.inner.is_distance_less(&a.0, &b.0, limit.0)
    }

    fn __repr__(&self) -> String {
        format!(
            "HausdorffDistanceQuery(include_interiors={})",
            self.inner.options().include_interiors
        )
    }
}

// ---------------------------------------------------------------------------
// ShapeNestingQuery + ChainRelation
// ---------------------------------------------------------------------------

/// The shell/hole relationship of a single chain within a polygon shape.
///
/// Shells have no parent and may have holes; holes have a parent shell and no
/// holes of their own.
#[pyclass(frozen, name = "ChainRelation", module = "s2rst")]
pub struct PyChainRelation {
    parent: Option<usize>,
    holes: Vec<usize>,
}

#[pymethods]
impl PyChainRelation {
    /// Whether this chain is a shell (has no parent).
    fn is_shell(&self) -> bool {
        self.parent.is_none()
    }

    /// Whether this chain is a hole (has a parent shell).
    fn is_hole(&self) -> bool {
        self.parent.is_some()
    }

    /// The parent chain id, or `None` if this is a shell.
    fn parent_id(&self) -> Option<usize> {
        self.parent
    }

    /// The chain ids of this chain's holes (empty for a hole).
    fn holes(&self) -> Vec<usize> {
        self.holes.clone()
    }

    fn __repr__(&self) -> String {
        match self.parent {
            Some(p) => format!("ChainRelation(hole, parent_id={p})"),
            None => format!("ChainRelation(shell, holes={:?})", self.holes),
        }
    }
}

/// Determines the nesting relationships between the chains of a polygon shape.
#[pyclass(name = "ShapeNestingQuery", module = "s2rst")]
pub struct PyShapeNestingQuery {
    index: Py<PyShapeIndex>,
}

#[pymethods]
impl PyShapeNestingQuery {
    #[new]
    fn new(index: Py<PyShapeIndex>) -> Self {
        PyShapeNestingQuery { index }
    }

    /// The nesting relations of the chains in shape `shape_id`, in 1:1
    /// correspondence with the shape's chains (chain *i* at index *i*).
    ///
    /// Returns an empty list if the shape id is unknown or has no chains.
    fn compute_shape_nesting(&self, py: Python<'_>, shape_id: i32) -> Vec<PyChainRelation> {
        let idx = self.index.borrow(py);
        let q = ShapeNestingQuery::new(&idx.0);
        q.compute_shape_nesting(shape_id)
            .into_iter()
            .map(|r| PyChainRelation {
                parent: r.parent_id(),
                holes: r.holes().to_vec(),
            })
            .collect()
    }

    fn __repr__(&self) -> String {
        "ShapeNestingQuery(...)".to_string()
    }
}
