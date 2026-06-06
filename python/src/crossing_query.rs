// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `CrossingEdgeQuery` — find indexed edges crossed by a query edge.

use std::collections::HashMap;

use pyo3::prelude::*;

use s2rst::s2::crossing_edge_query::CrossingEdgeQuery;

use crate::enums::PyCrossingType;
use crate::index::PyShapeIndex;
use crate::s2point::PyS2Point;

/// Finds the edges in a `ShapeIndex` that a query edge crosses.
#[pyclass(name = "CrossingEdgeQuery", module = "s2rst")]
pub struct PyCrossingEdgeQuery {
    index: Py<PyShapeIndex>,
}

#[pymethods]
impl PyCrossingEdgeQuery {
    #[new]
    fn new(index: Py<PyShapeIndex>) -> Self {
        Self { index }
    }

    /// The edges crossed by the edge `(a, b)`, as a map of shape id to the list
    /// of crossed edge ids. `cross_type` selects interior-only or all crossings.
    #[pyo3(signature = (a, b, *, cross_type = PyCrossingType::All))]
    fn crossings(
        &self,
        py: Python<'_>,
        a: &PyS2Point,
        b: &PyS2Point,
        cross_type: PyCrossingType,
    ) -> HashMap<i32, Vec<i32>> {
        let idx = self.index.borrow(py);
        let mut q = CrossingEdgeQuery::new(&idx.0);
        q.crossings_edge_map(a.0, b.0, cross_type.to_core())
            .into_iter()
            .map(|(shape_id, edges)| (shape_id.0, edges))
            .collect()
    }

    fn __repr__(&self) -> String {
        "CrossingEdgeQuery(...)".to_string()
    }
}
