// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for `S2BufferOperation` — expanding/contracting geometry by a
//! radius (the Minkowski sum with a spherical disc). A positive radius expands;
//! a negative radius contracts (erodes thin features).
//!
//! Exposed as a `BufferOptions` value object plus one convenience function per
//! input geometry kind (`buffer_point`, `buffer_polyline`, `buffer_loop`,
//! `buffer_polygon`), each returning a `Polygon`.

use pyo3::prelude::*;

use s2rst::s2::Point;
use s2rst::s2::buffer_operation::{self, MAX_CIRCLE_SEGMENTS, MIN_ERROR_FRACTION};

use crate::angle::PyAngle;
use crate::enums::{PyEndCapStyle, PyPolylineSide};
use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::s2point::PyS2Point;
use crate::snap::resolve_snap;

/// Options controlling a buffer operation.
///
/// `radius` is the buffer distance (a positive `Angle` expands, a negative one
/// contracts). `error_fraction` and `circle_segments` are two ways to trade
/// geometric fidelity against output size; both are clamped to their valid
/// ranges. `end_cap_style` and `polyline_side` only affect polyline buffering.
#[pyclass(name = "BufferOptions", module = "s2rst")]
pub struct PyBufferOptions {
    radius: s2rst::s1::Angle,
    error_fraction: Option<f64>,
    circle_segments: Option<f64>,
    end_cap_style: PyEndCapStyle,
    polyline_side: PyPolylineSide,
    snap_function: Option<Py<PyAny>>,
}

#[pymethods]
impl PyBufferOptions {
    #[new]
    #[pyo3(signature = (
        radius,
        *,
        error_fraction = None,
        circle_segments = None,
        end_cap_style = PyEndCapStyle::Round,
        polyline_side = PyPolylineSide::Both,
        snap_function = None,
    ))]
    fn new(
        radius: &PyAngle,
        error_fraction: Option<f64>,
        circle_segments: Option<f64>,
        end_cap_style: PyEndCapStyle,
        polyline_side: PyPolylineSide,
        snap_function: Option<Py<PyAny>>,
    ) -> Self {
        PyBufferOptions {
            radius: radius.0,
            error_fraction,
            circle_segments,
            end_cap_style,
            polyline_side,
            snap_function,
        }
    }

    /// The buffer radius (positive expands, negative contracts).
    #[getter]
    fn radius(&self) -> PyAngle {
        PyAngle(self.radius)
    }

    /// The end cap style applied when buffering polylines.
    #[getter]
    fn end_cap_style(&self) -> PyEndCapStyle {
        self.end_cap_style
    }

    /// Which side(s) of a polyline are buffered.
    #[getter]
    fn polyline_side(&self) -> PyPolylineSide {
        self.polyline_side
    }

    fn __repr__(&self) -> String {
        format!("BufferOptions(radius={}°)", self.radius.degrees())
    }
}

impl PyBufferOptions {
    /// Builds a fresh core `BufferOptions` (core's type is not `Clone`, so this
    /// is constructed per call). Out-of-range fidelity settings are clamped to
    /// avoid tripping the core's debug assertions.
    fn build(&self, py: Python<'_>) -> PyResult<buffer_operation::BufferOptions> {
        let mut o = buffer_operation::BufferOptions::new(self.radius);
        if let Some(f) = self.error_fraction {
            o.set_error_fraction(f.clamp(MIN_ERROR_FRACTION, 1.0));
        }
        if let Some(n) = self.circle_segments {
            o.set_circle_segments(n.clamp(2.0, MAX_CIRCLE_SEGMENTS));
        }
        o.set_end_cap_style(self.end_cap_style.to_core());
        o.set_polyline_side(self.polyline_side.to_core());
        o.set_snap_function(resolve_snap(
            self.snap_function.as_ref().map(|o| o.bind(py)),
        )?);
        Ok(o)
    }
}

/// Buffer a single point into a disc-shaped `Polygon`.
#[pyfunction]
pub fn buffer_point(
    py: Python<'_>,
    point: &PyS2Point,
    options: &PyBufferOptions,
) -> PyResult<PyPolygon> {
    Ok(PyPolygon(buffer_operation::buffer_point(
        point.0,
        options.build(py)?,
    )))
}

/// Buffer a polyline's path into a `Polygon`.
#[pyfunction]
pub fn buffer_polyline(
    py: Python<'_>,
    polyline: &PyPolyline,
    options: &PyBufferOptions,
) -> PyResult<PyPolygon> {
    let verts: Vec<Point> = polyline.0.vertices_vec().to_vec();
    Ok(PyPolygon(buffer_operation::buffer_polyline(
        &verts,
        options.build(py)?,
    )))
}

/// Buffer a loop's boundary into a `Polygon`.
#[pyfunction]
pub fn buffer_loop(
    py: Python<'_>,
    loop_: &PyLoop,
    options: &PyBufferOptions,
) -> PyResult<PyPolygon> {
    let verts: Vec<Point> = loop_.0.vertices().to_vec();
    Ok(PyPolygon(buffer_operation::buffer_loop(
        &verts,
        options.build(py)?,
    )))
}

/// Buffer a polygon (positive radius expands, negative contracts).
#[pyfunction]
pub fn buffer_polygon(
    py: Python<'_>,
    polygon: &PyPolygon,
    options: &PyBufferOptions,
) -> PyResult<PyPolygon> {
    Ok(PyPolygon(buffer_operation::buffer_polygon(
        &polygon.0,
        options.build(py)?,
    )))
}
