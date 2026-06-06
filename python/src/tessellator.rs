// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `EdgeTessellator` — approximate a spherical edge with a planar
//! polyline (or vice versa) under a map projection.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s2::edge_tessellator::EdgeTessellator;
use s2rst::s2::projections::{MercatorProjection, PlateCarreeProjection};

use crate::angle::PyAngle;
use crate::points::PyR2Point;
use crate::projections::{PyMercatorProjection, PyPlateCarreeProjection};
use crate::s2point::PyS2Point;

enum Tess {
    PlateCarree(EdgeTessellator<PlateCarreeProjection>),
    Mercator(EdgeTessellator<MercatorProjection>),
}

/// Tessellates edges to within a given tolerance under a projection.
#[pyclass(name = "EdgeTessellator", module = "s2rst")]
pub struct PyEdgeTessellator(Tess);

#[pymethods]
impl PyEdgeTessellator {
    #[new]
    fn new(projection: &Bound<'_, PyAny>, tolerance: &PyAngle) -> PyResult<Self> {
        if let Ok(p) = projection.downcast::<PyPlateCarreeProjection>() {
            Ok(Self(Tess::PlateCarree(EdgeTessellator::new(
                p.borrow().0,
                tolerance.0,
            ))))
        } else if let Ok(p) = projection.downcast::<PyMercatorProjection>() {
            Ok(Self(Tess::Mercator(EdgeTessellator::new(
                p.borrow().0,
                tolerance.0,
            ))))
        } else {
            Err(PyTypeError::new_err(
                "projection must be a PlateCarreeProjection or MercatorProjection",
            ))
        }
    }

    /// The projected (2D) vertices approximating the spherical edge `(a, b)`.
    fn append_projected(&self, a: &PyS2Point, b: &PyS2Point) -> Vec<PyR2Point> {
        let mut v = Vec::new();
        match &self.0 {
            Tess::PlateCarree(t) => t.append_projected(a.0, b.0, &mut v),
            Tess::Mercator(t) => t.append_projected(a.0, b.0, &mut v),
        }
        v.into_iter().map(PyR2Point).collect()
    }

    /// The spherical vertices approximating the projected edge `(a, b)`.
    fn append_unprojected(&self, a: &PyR2Point, b: &PyR2Point) -> Vec<PyS2Point> {
        let mut v = Vec::new();
        match &self.0 {
            Tess::PlateCarree(t) => t.append_unprojected(a.0, b.0, &mut v),
            Tess::Mercator(t) => t.append_unprojected(a.0, b.0, &mut v),
        }
        v.into_iter().map(PyS2Point).collect()
    }

    fn __repr__(&self) -> String {
        "EdgeTessellator(...)".to_string()
    }
}
