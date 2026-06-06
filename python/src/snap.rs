// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Snap functions for the builder and the buffer/winding/boolean operations.
//!
//! The three concrete `SnapFunction` implementations are exposed as small
//! value classes; `resolve_snap` turns one (or `None`) into the
//! `Box<dyn SnapFunction>` the core operations consume.

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;

use s2rst::s1::Angle;
use s2rst::s2::builder::snap::{
    IdentitySnapFunction, IntLatLngSnapFunction, S2CellIdSnapFunction, SnapFunction,
};

use crate::angle::PyAngle;

/// A snap function that does not move vertices; `snap_radius` only controls the
/// minimum vertex separation.
#[pyclass(name = "IdentitySnapFunction", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyIdentitySnapFunction {
    snap_radius: Angle,
}

impl PyIdentitySnapFunction {
    pub(crate) fn to_boxed(self) -> Box<dyn SnapFunction> {
        Box::new(IdentitySnapFunction::new(self.snap_radius))
    }
}

#[pymethods]
impl PyIdentitySnapFunction {
    #[new]
    #[pyo3(signature = (snap_radius=None))]
    fn new(snap_radius: Option<PyAngle>) -> Self {
        Self {
            snap_radius: snap_radius.map(|a| a.0).unwrap_or(Angle::ZERO),
        }
    }

    fn snap_radius(&self) -> PyAngle {
        PyAngle(self.snap_radius)
    }

    fn __repr__(&self) -> String {
        format!(
            "IdentitySnapFunction(snap_radius={})",
            self.snap_radius.radians()
        )
    }
}

/// A snap function that snaps vertices to S2 cell centers at a given level.
#[pyclass(name = "S2CellIdSnapFunction", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyS2CellIdSnapFunction {
    level: u8,
}

impl PyS2CellIdSnapFunction {
    pub(crate) fn to_boxed(self) -> Box<dyn SnapFunction> {
        Box::new(S2CellIdSnapFunction::new(self.level))
    }
}

#[pymethods]
impl PyS2CellIdSnapFunction {
    #[new]
    fn new(level: u8) -> Self {
        Self { level }
    }

    #[getter]
    fn level(&self) -> u8 {
        self.level
    }

    fn snap_radius(&self) -> PyAngle {
        PyAngle(self.to_boxed().snap_radius())
    }

    fn __repr__(&self) -> String {
        format!("S2CellIdSnapFunction(level={})", self.level)
    }
}

/// A snap function that snaps vertices to integer lat/lng coordinates with the
/// given power-of-ten `exponent` (e.g. 6 for E6 / micro-degrees).
#[pyclass(name = "IntLatLngSnapFunction", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyIntLatLngSnapFunction {
    exponent: i32,
}

impl PyIntLatLngSnapFunction {
    pub(crate) fn to_boxed(self) -> Box<dyn SnapFunction> {
        Box::new(IntLatLngSnapFunction::new(self.exponent))
    }
}

#[pymethods]
impl PyIntLatLngSnapFunction {
    #[new]
    fn new(exponent: i32) -> Self {
        Self { exponent }
    }

    #[getter]
    fn exponent(&self) -> i32 {
        self.exponent
    }

    fn snap_radius(&self) -> PyAngle {
        PyAngle(self.to_boxed().snap_radius())
    }

    fn __repr__(&self) -> String {
        format!("IntLatLngSnapFunction(exponent={})", self.exponent)
    }
}

/// Resolve an optional Python snap-function object to a boxed core snap
/// function, defaulting to a zero-radius identity snap.
pub(crate) fn resolve_snap(obj: Option<&Bound<'_, PyAny>>) -> PyResult<Box<dyn SnapFunction>> {
    let Some(o) = obj else {
        return Ok(Box::new(IdentitySnapFunction::new(Angle::ZERO)));
    };
    if let Ok(s) = o.downcast::<PyIdentitySnapFunction>() {
        Ok(s.borrow().to_boxed())
    } else if let Ok(s) = o.downcast::<PyS2CellIdSnapFunction>() {
        Ok(s.borrow().to_boxed())
    } else if let Ok(s) = o.downcast::<PyIntLatLngSnapFunction>() {
        Ok(s.borrow().to_boxed())
    } else {
        Err(PyTypeError::new_err(
            "snap_function must be an IdentitySnapFunction, S2CellIdSnapFunction, \
             or IntLatLngSnapFunction",
        ))
    }
}
