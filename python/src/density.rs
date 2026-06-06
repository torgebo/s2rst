// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for `S2DensityTree` and `DensityClusterQuery` — represent the
//! spatial density of geometry and partition it into balanced coverings.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use s2rst::s2::density_cluster_query::DensityClusterQuery;
use s2rst::s2::density_tree::S2DensityTree;

use crate::cells::PyCellUnion;
use crate::index::PyShapeIndex;

/// A hierarchical representation of the density (weight) of geometry by cell.
#[pyclass(name = "S2DensityTree", module = "s2rst")]
pub struct PyS2DensityTree(S2DensityTree);

#[pymethods]
impl PyS2DensityTree {
    #[new]
    fn new() -> Self {
        Self(S2DensityTree::new())
    }

    /// Build the tree from the vertex density of `index`, using at most
    /// `approximate_size_bytes` of encoded storage and subdividing to at most
    /// `max_level`.
    fn init_to_vertex_density(
        &mut self,
        index: &PyShapeIndex,
        approximate_size_bytes: i64,
        max_level: u8,
    ) -> PyResult<()> {
        self.0
            .init_to_vertex_density(&index.0, approximate_size_bytes, max_level)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Whether the tree is empty.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The encoded size of the tree, in bytes.
    fn encoded_size(&self) -> usize {
        self.0.encoded_size()
    }

    fn __repr__(&self) -> String {
        format!("S2DensityTree(encoded_size={})", self.0.encoded_size())
    }
}

/// Partitions a density tree into `CellUnion` coverings of roughly equal weight.
#[pyclass(name = "DensityClusterQuery", module = "s2rst")]
pub struct PyDensityClusterQuery(DensityClusterQuery);

#[pymethods]
impl PyDensityClusterQuery {
    #[new]
    fn new(desired_weight: i64) -> Self {
        Self(DensityClusterQuery::new(desired_weight))
    }

    /// Partition `density` into coverings, each near the desired weight.
    fn coverings(&self, density: &PyS2DensityTree) -> PyResult<Vec<PyCellUnion>> {
        self.0
            .coverings(&density.0)
            .map(|v| v.into_iter().map(PyCellUnion).collect())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        "DensityClusterQuery(...)".to_string()
    }
}
