// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for the S2 binary encoding: round-trip geometry to/from
//! `bytes`. Pairs with `RegionCoverer` for persisting cell coverings.

use std::io;

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::{CellUnion, Loop, Polygon};

use crate::cells::PyCellUnion;
use crate::geometry::{PyLoop, PyPolygon, PyPolyline};

fn io_err(e: &io::Error) -> PyErr {
    PyValueError::new_err(format!("s2 codec error: {e}"))
}

/// Encode geometry (a `Polygon`, `Polyline`, `Loop`, or `CellUnion`) to the S2
/// binary format. Use the matching `decode_*` to read it back.
#[pyfunction]
pub fn encode<'py>(py: Python<'py>, obj: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    let mut buf = Vec::new();
    macro_rules! try_encode {
        ($ty:ty) => {
            if let Ok(o) = obj.downcast::<$ty>() {
                o.borrow().0.encode(&mut buf).map_err(|e| io_err(&e))?;
                return Ok(PyBytes::new(py, &buf));
            }
        };
    }
    try_encode!(PyPolygon);
    try_encode!(PyPolyline);
    try_encode!(PyLoop);
    try_encode!(PyCellUnion);
    Err(PyTypeError::new_err(
        "encode() expects a Polygon, Polyline, Loop, or CellUnion",
    ))
}

#[pyfunction]
pub fn decode_polygon(data: &[u8]) -> PyResult<PyPolygon> {
    Ok(PyPolygon(
        Polygon::decode(&mut { data }).map_err(|e| io_err(&e))?,
    ))
}

#[pyfunction]
pub fn decode_polyline(data: &[u8]) -> PyResult<PyPolyline> {
    Ok(PyPolyline(
        Polyline::decode(&mut { data }).map_err(|e| io_err(&e))?,
    ))
}

#[pyfunction]
pub fn decode_loop(data: &[u8]) -> PyResult<PyLoop> {
    Ok(PyLoop(Loop::decode(&mut { data }).map_err(|e| io_err(&e))?))
}

#[pyfunction]
pub fn decode_cell_union(data: &[u8]) -> PyResult<PyCellUnion> {
    Ok(PyCellUnion(
        CellUnion::decode(&mut { data }).map_err(|e| io_err(&e))?,
    ))
}
