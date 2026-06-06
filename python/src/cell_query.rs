// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `ClosestCellQuery` — nearest labelled cell(s) in a `CellIndex`.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s1::ChordAngle;
use s2rst::s2::CellId;
use s2rst::s2::closest_cell_query as ccq;

use crate::angle::PyChordAngle;
use crate::cells::{PyCell, PyCellId, PyCellUnion};
use crate::index::PyShapeIndex;
use crate::longtail::PyCellIndex;
use crate::s2point::PyS2Point;

/// One result of a closest-cell query.
#[pyclass(frozen, name = "CellQueryResult", module = "s2rst")]
pub struct PyCellQueryResult {
    distance: ChordAngle,
    cell_id: CellId,
    label: i32,
}

impl PyCellQueryResult {
    fn from_result(r: ccq::Result) -> Self {
        Self {
            distance: r.distance,
            cell_id: r.cell_id,
            label: r.label,
        }
    }
}

#[pymethods]
impl PyCellQueryResult {
    /// Distance from the target to this cell.
    #[getter]
    fn distance(&self) -> PyChordAngle {
        PyChordAngle(self.distance)
    }

    /// The matched cell id.
    #[getter]
    fn cell_id(&self) -> PyCellId {
        PyCellId(self.cell_id)
    }

    /// The label stored with the matched cell.
    #[getter]
    fn label(&self) -> i32 {
        self.label
    }

    /// Whether no cell was found.
    fn is_empty(&self) -> bool {
        self.distance == ChordAngle::INFINITY
    }

    fn __repr__(&self) -> String {
        format!(
            "CellQueryResult(distance={}, label={})",
            self.distance.to_angle().radians(),
            self.label
        )
    }
}

macro_rules! dispatch_cell_target {
    ($obj:expr, $build:expr) => {{
        let obj: &Bound<'_, PyAny> = $obj;
        if let Ok(p) = obj.downcast::<PyS2Point>() {
            let mut t = ccq::PointTarget::new(p.borrow().0);
            Ok($build(&mut t))
        } else if let Ok((a, b)) = obj.extract::<(PyS2Point, PyS2Point)>() {
            let mut t = ccq::EdgeTarget::new(a.0, b.0);
            Ok($build(&mut t))
        } else if let Ok(c) = obj.downcast::<PyCell>() {
            let mut t = ccq::CellTarget::new(c.borrow().0);
            Ok($build(&mut t))
        } else if let Ok(cu) = obj.downcast::<PyCellUnion>() {
            let mut t = ccq::CellUnionTarget::new(cu.borrow().0.clone());
            Ok($build(&mut t))
        } else if let Ok(ix) = obj.downcast::<PyShapeIndex>() {
            let ix_ref = ix.borrow();
            let mut t = ccq::ShapeIndexTarget::new(&ix_ref.0);
            Ok($build(&mut t))
        } else {
            Err(PyTypeError::new_err(
                "target must be an S2Point, an (S2Point, S2Point) edge tuple, \
                 a Cell, a CellUnion, or a ShapeIndex",
            ))
        }
    }};
}

/// Finds the cell(s) in a `CellIndex` closest to a target.
#[pyclass(name = "ClosestCellQuery", module = "s2rst")]
pub struct PyClosestCellQuery {
    index: Py<PyCellIndex>,
}

impl PyClosestCellQuery {
    fn options(
        max_results: i32,
        max_distance: Option<&PyChordAngle>,
        max_error: Option<&PyChordAngle>,
    ) -> ccq::Options {
        let mut opts = ccq::Options {
            max_results,
            ..Default::default()
        };
        if let Some(d) = max_distance {
            opts.max_distance = d.0;
        }
        if let Some(e) = max_error {
            opts.max_error = e.0;
        }
        opts
    }
}

#[pymethods]
impl PyClosestCellQuery {
    #[new]
    fn new(index: Py<PyCellIndex>) -> Self {
        Self { index }
    }

    /// The single closest cell to `target`.
    fn find_closest_cell(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
    ) -> PyResult<PyCellQueryResult> {
        let idx = self.index.borrow(py);
        let q = ccq::ClosestCellQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_cell_target!(target, |t| PyCellQueryResult::from_result(
            q.find_closest_cell(t)
        ))
    }

    /// Up to `max_results` closest cells, ordered nearest-first.
    #[pyo3(signature = (target, *, max_results=1, max_distance=None, max_error=None))]
    fn find_closest_cells(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        max_results: i32,
        max_distance: Option<PyChordAngle>,
        max_error: Option<PyChordAngle>,
    ) -> PyResult<Vec<PyCellQueryResult>> {
        let idx = self.index.borrow(py);
        let opts = Self::options(max_results, max_distance.as_ref(), max_error.as_ref());
        let q = ccq::ClosestCellQuery::new(&idx.0, opts);
        dispatch_cell_target!(target, |t| q
            .find_closest_cells(t)
            .into_iter()
            .map(PyCellQueryResult::from_result)
            .collect())
    }

    /// The distance from `target` to the nearest cell.
    fn get_distance(&self, py: Python<'_>, target: &Bound<'_, PyAny>) -> PyResult<PyChordAngle> {
        let idx = self.index.borrow(py);
        let q = ccq::ClosestCellQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_cell_target!(target, |t| PyChordAngle(q.get_distance(t)))
    }

    /// Whether some cell is closer than `limit` to `target`.
    fn is_distance_less(
        &self,
        py: Python<'_>,
        target: &Bound<'_, PyAny>,
        limit: &PyChordAngle,
    ) -> PyResult<bool> {
        let idx = self.index.borrow(py);
        let q = ccq::ClosestCellQuery::new(&idx.0, Self::options(1, None, None));
        dispatch_cell_target!(target, |t| q.is_distance_less(t, limit.0))
    }

    fn __repr__(&self) -> String {
        "ClosestCellQuery(...)".to_string()
    }
}
