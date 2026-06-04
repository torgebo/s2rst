// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for `S2Builder` — robustly assembling snapped geometry.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use s2rst::s1::Angle;
use s2rst::s2::builder::layer::Layer;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::builder::polyline_layer::S2PolylineLayer;
use s2rst::s2::builder::snap::{IdentitySnapFunction, S2CellIdSnapFunction, SnapFunction};
use s2rst::s2::builder::{Options, S2Builder};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::{Loop, Point};

use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::s2point::PyS2Point;

enum Input {
    Edge(Point, Point),
    Loop(Loop),
    Polyline(Polyline),
    LoopPoints(Vec<Point>),
    PolylinePoints(Vec<Point>),
}

/// Assembles edges, loops, and polylines into snapped, topologically valid
/// geometry. Add inputs, then call `build_polygon()` or `build_polyline()`.
///
/// By default vertices are not snapped; pass `snap_level` to snap them to S2
/// cell centers at that level (lower level = coarser snapping).
#[pyclass(name = "S2Builder")]
pub struct PyS2Builder {
    snap_level: Option<u8>,
    inputs: Vec<Input>,
}

#[pymethods]
impl PyS2Builder {
    #[new]
    #[pyo3(signature = (*, snap_level=None))]
    fn new(snap_level: Option<u8>) -> Self {
        PyS2Builder {
            snap_level,
            inputs: Vec::new(),
        }
    }

    /// Add a single directed edge.
    fn add_edge(&mut self, v0: &PyS2Point, v1: &PyS2Point) {
        self.inputs.push(Input::Edge(v0.0, v1.0));
    }

    /// Add all edges of a closed loop.
    fn add_loop(&mut self, loop_: &PyLoop) {
        self.inputs.push(Input::Loop(loop_.0.clone()));
    }

    /// Add all edges of a polyline (an open chain).
    fn add_polyline(&mut self, polyline: &PyPolyline) {
        self.inputs.push(Input::Polyline(polyline.0.clone()));
    }

    /// Add a closed loop from a list of vertices.
    fn add_loop_from_points(&mut self, vertices: Vec<PyS2Point>) {
        self.inputs
            .push(Input::LoopPoints(vertices.iter().map(|p| p.0).collect()));
    }

    /// Add a polyline from a list of vertices.
    fn add_polyline_from_points(&mut self, vertices: Vec<PyS2Point>) {
        self.inputs.push(Input::PolylinePoints(
            vertices.iter().map(|p| p.0).collect(),
        ));
    }

    /// The number of inputs added so far.
    fn __len__(&self) -> usize {
        self.inputs.len()
    }

    /// Assemble the added geometry into a `Polygon`.
    fn build_polygon(&self) -> PyResult<PyPolygon> {
        let mut builder = S2Builder::new(self.options());
        builder.start_layer(Box::new(S2PolygonLayer::new()));
        self.feed(&mut builder);
        let layers = builder
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        take_layer::<S2PolygonLayer>(layers)?
            .take_output()
            .map(PyPolygon)
            .ok_or_else(no_output)
    }

    /// Assemble the added geometry into a `Polyline`.
    fn build_polyline(&self) -> PyResult<PyPolyline> {
        let mut builder = S2Builder::new(self.options());
        builder.start_layer(Box::new(S2PolylineLayer::new()));
        self.feed(&mut builder);
        let layers = builder
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        take_layer::<S2PolylineLayer>(layers)?
            .take_output()
            .map(PyPolyline)
            .ok_or_else(no_output)
    }
}

impl PyS2Builder {
    fn options(&self) -> Options {
        let snap: Box<dyn SnapFunction> = match self.snap_level {
            Some(level) => Box::new(S2CellIdSnapFunction::new(level)),
            None => Box::new(IdentitySnapFunction::new(Angle::ZERO)),
        };
        Options::new(snap)
    }

    fn feed(&self, builder: &mut S2Builder) {
        for input in &self.inputs {
            match input {
                Input::Edge(v0, v1) => builder.add_edge(*v0, *v1),
                Input::Loop(l) => builder.add_loop(l),
                Input::Polyline(p) => builder.add_polyline(p),
                Input::LoopPoints(v) => builder.add_loop_from_points(v),
                Input::PolylinePoints(v) => builder.add_polyline_from_points(v),
            }
        }
    }
}

fn take_layer<T: Layer + 'static>(layers: Vec<Box<dyn Layer>>) -> PyResult<Box<T>> {
    layers
        .into_iter()
        .next()
        .ok_or_else(|| PyRuntimeError::new_err("builder produced no layer"))?
        .into_any()
        .downcast::<T>()
        .map_err(|_| PyRuntimeError::new_err("unexpected builder layer type"))
}

fn no_output() -> PyErr {
    PyRuntimeError::new_err("builder produced no output")
}
