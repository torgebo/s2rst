// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for the s2rst spherical geometry library.
//!
//! This crate exposes the core S2 geometry types (Angle, Point, Cell,
//! Polygon, ...) to Python via [`pyo3`]. The bindings are thin wrappers
//! around the underlying Rust types, with operator overloads and Python
//! protocol methods (`__len__`, `__getitem__`, `__repr__`, ...) added
//! where appropriate.

use pyo3::prelude::*;

mod angle;
mod cells;
mod geometry;
mod hash_util;
mod interval;
mod points;
mod regions;
mod s2point;
mod shapes;

use angle::{PyAngle, PyChordAngle};
use cells::{PyCell, PyCellId, PyCellUnion};
use geometry::{PyLoop, PyPolygon, PyPolyline};
use interval::{PyR1Interval, PyS1Interval};
use points::{PyMatrix3x3, PyR2Point, PyR2Rect, PyVector};
use regions::{PyCap, PyRect};
use s2point::{PyLatLng, PyS2Point, s2_ortho, s2_rotate};
use shapes::{
    PyEdge, PyEdgeVectorShape, PyLaxLoop, PyLaxPolygon, PyLaxPolyline, PyPointVector,
    PyReferencePoint, PyShape,
};

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    // Phase 1
    m.add_class::<PyAngle>()?;
    m.add_class::<PyChordAngle>()?;
    m.add_class::<PyR1Interval>()?;
    m.add_class::<PyS1Interval>()?;
    // Phase 2
    m.add_class::<PyR2Point>()?;
    m.add_class::<PyVector>()?;
    m.add_class::<PyMatrix3x3>()?;
    m.add_class::<PyR2Rect>()?;
    // Phase 3
    m.add_class::<PyS2Point>()?;
    m.add_class::<PyLatLng>()?;
    m.add_function(wrap_pyfunction!(s2_ortho, m)?)?;
    m.add_function(wrap_pyfunction!(s2_rotate, m)?)?;
    // Phase 4
    m.add_class::<PyCellId>()?;
    m.add_class::<PyCell>()?;
    m.add_class::<PyCellUnion>()?;
    // Phase 5
    m.add_class::<PyCap>()?;
    m.add_class::<PyRect>()?;
    // Phase 6
    m.add_class::<PyPolyline>()?;
    m.add_class::<PyLoop>()?;
    m.add_class::<PyPolygon>()?;
    // Phase 7
    m.add_class::<PyEdge>()?;
    m.add_class::<PyReferencePoint>()?;
    m.add_class::<PyShape>()?;
    m.add_class::<PyLaxLoop>()?;
    m.add_class::<PyLaxPolyline>()?;
    m.add_class::<PyLaxPolygon>()?;
    m.add_class::<PyPointVector>()?;
    m.add_class::<PyEdgeVectorShape>()?;
    Ok(())
}
