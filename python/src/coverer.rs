// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for `RegionCoverer` — approximating a region with S2 cells.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s2::CellUnion;
use s2rst::s2::region_coverer::RegionCoverer;

use crate::cells::PyCellUnion;
use crate::geometry::{PyLoop, PyPolygon};
use crate::regions::{PyCap, PyRect};

/// Approximates a region (`Cap`, `Rect`, `Loop`, or `Polygon`) with a covering
/// of S2 cells — the basis for indexing spherical geometry in a database.
#[pyclass(name = "RegionCoverer")]
pub struct PyRegionCoverer(RegionCoverer);

#[pymethods]
impl PyRegionCoverer {
    /// Create a region coverer. The keyword-only settings default to the S2
    /// norms: `min_level=0`, `max_level=30`, `level_mod=1`, `max_cells=8`.
    #[new]
    #[pyo3(signature = (*, min_level=None, max_level=None, level_mod=None, max_cells=None))]
    fn new(
        min_level: Option<u8>,
        max_level: Option<u8>,
        level_mod: Option<u8>,
        max_cells: Option<usize>,
    ) -> Self {
        let mut c = RegionCoverer::new();
        if let Some(v) = min_level {
            c = c.min_level(v);
        }
        if let Some(v) = max_level {
            c = c.max_level(v);
        }
        if let Some(v) = level_mod {
            c = c.level_mod(v);
        }
        if let Some(v) = max_cells {
            c = c.max_cells(v);
        }
        PyRegionCoverer(c)
    }

    #[getter]
    fn min_level(&self) -> u8 {
        u8::from(self.0.min_level)
    }

    #[getter]
    fn max_level(&self) -> u8 {
        u8::from(self.0.max_level)
    }

    #[getter]
    fn level_mod(&self) -> u8 {
        self.0.level_mod
    }

    #[getter]
    fn max_cells(&self) -> usize {
        self.0.max_cells
    }

    /// Return a `CellUnion` covering `region` (a `Cap`, `Rect`, `Loop`, or
    /// `Polygon`).
    fn covering(&self, region: &Bound<'_, PyAny>) -> PyResult<PyCellUnion> {
        self.cover(region, false).map(PyCellUnion)
    }

    /// Return a covering using only cells that lie entirely inside `region`.
    fn interior_covering(&self, region: &Bound<'_, PyAny>) -> PyResult<PyCellUnion> {
        self.cover(region, true).map(PyCellUnion)
    }

    fn __repr__(&self) -> String {
        format!(
            "RegionCoverer(min_level={}, max_level={}, level_mod={}, max_cells={})",
            u8::from(self.0.min_level),
            u8::from(self.0.max_level),
            self.0.level_mod,
            self.0.max_cells,
        )
    }
}

impl PyRegionCoverer {
    /// Dispatch the covering over the supported concrete region types.
    fn cover(&self, region: &Bound<'_, PyAny>, interior: bool) -> PyResult<CellUnion> {
        macro_rules! try_cover {
            ($ty:ty) => {
                if let Ok(r) = region.downcast::<$ty>() {
                    let r = r.borrow();
                    return Ok(if interior {
                        self.0.interior_covering(&r.0)
                    } else {
                        self.0.covering(&r.0)
                    });
                }
            };
        }
        try_cover!(PyCap);
        try_cover!(PyRect);
        try_cover!(PyLoop);
        try_cover!(PyPolygon);
        Err(PyTypeError::new_err(
            "covering() expects a Cap, Rect, Loop, or Polygon",
        ))
    }
}
