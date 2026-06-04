// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for `text_format` — human-readable parsing and formatting of
//! S2 geometry (e.g. `"10:20, 30:40"` for a list of lat:lng points).

use std::panic::{AssertUnwindSafe, catch_unwind};

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use s2rst::s2::text_format;

use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::s2point::{PyLatLng, PyS2Point};

/// Run a parser that panics on malformed input, converting a panic into a
/// `ValueError` instead of crashing the interpreter.
fn parse<T>(what: &str, f: impl FnOnce() -> T) -> PyResult<T> {
    catch_unwind(AssertUnwindSafe(f))
        .map_err(|_| PyValueError::new_err(format!("could not parse {what}")))
}

#[pyfunction]
pub fn parse_point(s: &str) -> PyResult<PyS2Point> {
    parse("point", || PyS2Point(text_format::parse_point(s)))
}

#[pyfunction]
pub fn parse_points(s: &str) -> PyResult<Vec<PyS2Point>> {
    parse("points", || {
        text_format::parse_points(s)
            .into_iter()
            .map(PyS2Point)
            .collect()
    })
}

#[pyfunction]
pub fn parse_latlngs(s: &str) -> PyResult<Vec<PyLatLng>> {
    parse("latlngs", || {
        text_format::parse_latlngs(s)
            .into_iter()
            .map(PyLatLng)
            .collect()
    })
}

#[pyfunction]
pub fn make_loop(s: &str) -> PyResult<PyLoop> {
    parse("loop", || PyLoop(text_format::make_loop(s)))
}

#[pyfunction]
pub fn make_polygon(s: &str) -> PyResult<PyPolygon> {
    parse("polygon", || PyPolygon(text_format::make_polygon(s)))
}

#[pyfunction]
pub fn make_polyline(s: &str) -> PyResult<PyPolyline> {
    parse("polyline", || PyPolyline(text_format::make_polyline(s)))
}

#[pyfunction]
pub fn point_to_string(point: &PyS2Point) -> String {
    text_format::point_to_string(point.0)
}

#[pyfunction]
pub fn points_to_string(points: Vec<PyS2Point>) -> String {
    let pts: Vec<_> = points.iter().map(|p| p.0).collect();
    text_format::points_to_string(&pts)
}

#[pyfunction]
pub fn latlng_to_string(latlng: &PyLatLng) -> String {
    text_format::latlng_to_string(latlng.0)
}

#[pyfunction]
pub fn loop_to_string(loop_: &PyLoop) -> String {
    text_format::loop_to_string(&loop_.0)
}

#[pyfunction]
pub fn polygon_to_string(polygon: &PyPolygon) -> String {
    text_format::polygon_to_string(&polygon.0)
}

#[pyfunction]
pub fn polyline_to_string(polyline: &PyPolyline) -> String {
    text_format::polyline_to_string(&polyline.0)
}
