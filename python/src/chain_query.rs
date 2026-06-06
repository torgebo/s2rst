// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `S2ChainInterpolationQuery` — interpolate along a shape's chain.

use pyo3::prelude::*;

use s2rst::s1::Angle;
use s2rst::s2::Point;
use s2rst::s2::chain_interpolation_query::{ChainInterpolationResult, S2ChainInterpolationQuery};

use crate::angle::PyAngle;
use crate::s2point::PyS2Point;
use crate::shapes::PyShape;

/// A point interpolated along a shape's chain.
#[pyclass(frozen, name = "ChainInterpolationResult", module = "s2rst")]
pub struct PyChainInterpolationResult {
    point: Point,
    edge_id: usize,
    distance: Angle,
}

impl PyChainInterpolationResult {
    fn from_core(r: ChainInterpolationResult) -> Self {
        Self {
            point: r.point,
            edge_id: r.edge_id,
            distance: r.distance,
        }
    }
}

#[pymethods]
impl PyChainInterpolationResult {
    /// The interpolated point.
    #[getter]
    fn point(&self) -> PyS2Point {
        PyS2Point(self.point)
    }

    /// The edge id within the shape that contains the point.
    #[getter]
    fn edge_id(&self) -> usize {
        self.edge_id
    }

    /// The arc-length distance from the start of the chain to the point.
    #[getter]
    fn distance(&self) -> PyAngle {
        PyAngle(self.distance)
    }

    fn __repr__(&self) -> String {
        format!(
            "ChainInterpolationResult(edge_id={}, distance={})",
            self.edge_id,
            self.distance.radians()
        )
    }
}

/// Interpolates points along the edges of a shape (optionally a single chain).
#[pyclass(name = "ChainInterpolationQuery", module = "s2rst")]
pub struct PyChainInterpolationQuery {
    shape: Py<PyShape>,
    chain_id: Option<usize>,
}

#[pymethods]
impl PyChainInterpolationQuery {
    #[new]
    #[pyo3(signature = (shape, *, chain_id=None))]
    fn new(shape: Py<PyShape>, chain_id: Option<usize>) -> Self {
        Self { shape, chain_id }
    }

    /// The total arc length of the chain(s).
    fn get_length(&self, py: Python<'_>) -> PyAngle {
        let sh = self.shape.borrow(py);
        sh.with_shape(|s| {
            let q = S2ChainInterpolationQuery::with_chain(s, self.chain_id);
            PyAngle(q.get_length())
        })
    }

    /// The point at the given fraction (0.0 = start, 1.0 = end), or None if the
    /// shape has no edges.
    fn at_fraction(&self, py: Python<'_>, fraction: f64) -> Option<PyChainInterpolationResult> {
        let sh = self.shape.borrow(py);
        sh.with_shape(|s| {
            S2ChainInterpolationQuery::with_chain(s, self.chain_id)
                .at_fraction(fraction)
                .map(PyChainInterpolationResult::from_core)
        })
    }

    /// The point at the given arc-length distance, or None if there are no edges.
    fn at_distance(
        &self,
        py: Python<'_>,
        distance: &PyAngle,
    ) -> Option<PyChainInterpolationResult> {
        let sh = self.shape.borrow(py);
        sh.with_shape(|s| {
            S2ChainInterpolationQuery::with_chain(s, self.chain_id)
                .at_distance(distance.0)
                .map(PyChainInterpolationResult::from_core)
        })
    }

    /// The vertices of the sub-chain between two fractions.
    fn slice(&self, py: Python<'_>, begin_fraction: f64, end_fraction: f64) -> Vec<PyS2Point> {
        let sh = self.shape.borrow(py);
        sh.with_shape(|s| {
            S2ChainInterpolationQuery::with_chain(s, self.chain_id)
                .slice(begin_fraction, end_fraction)
                .into_iter()
                .map(PyS2Point)
                .collect()
        })
    }

    fn __repr__(&self) -> String {
        "ChainInterpolationQuery(...)".to_string()
    }
}
