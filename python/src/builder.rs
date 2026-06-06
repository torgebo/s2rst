// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python binding for `S2Builder` — robustly assembling snapped geometry.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use s2rst::s1::Angle;
use s2rst::s2::builder::lax_polygon_layer::LaxPolygonLayer;
use s2rst::s2::builder::layer::Layer;
use s2rst::s2::builder::point_vector_layer::S2PointVectorLayer;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::builder::polyline_layer::S2PolylineLayer;
use s2rst::s2::builder::snap::{IdentitySnapFunction, S2CellIdSnapFunction, SnapFunction};
use s2rst::s2::builder::{Options, S2Builder};
use s2rst::s2::polyline::Polyline;
use s2rst::s2::{Loop, Point};

use crate::angle::PyAngle;
use crate::geometry::{PyLoop, PyPolygon, PyPolyline};
use crate::s2point::PyS2Point;
use crate::shapes::PyLaxPolygon;
use crate::snap::resolve_snap;

enum Input {
    Point(Point),
    Edge(Point, Point),
    Loop(Loop),
    Polyline(Polyline),
    LoopPoints(Vec<Point>),
    PolylinePoints(Vec<Point>),
}

/// Assembles points, edges, loops, and polylines into snapped, topologically
/// valid geometry. Add inputs, then call one of the `build_*()` methods.
///
/// Snapping is controlled by `snap_function` (an `IdentitySnapFunction`,
/// `S2CellIdSnapFunction`, or `IntLatLngSnapFunction`); the legacy `snap_level`
/// keyword is still accepted and maps to an `S2CellIdSnapFunction`.
#[pyclass(name = "S2Builder")]
pub struct PyS2Builder {
    snap_function: Option<Py<PyAny>>,
    snap_level: Option<u8>,
    split_crossing_edges: bool,
    simplify_edge_chains: bool,
    intersection_tolerance: Angle,
    idempotent: bool,
    inputs: Vec<Input>,
}

#[pymethods]
impl PyS2Builder {
    #[new]
    #[pyo3(signature = (
        *,
        snap_function = None,
        snap_level = None,
        split_crossing_edges = false,
        simplify_edge_chains = false,
        intersection_tolerance = None,
        idempotent = true,
    ))]
    fn new(
        snap_function: Option<Py<PyAny>>,
        snap_level: Option<u8>,
        split_crossing_edges: bool,
        simplify_edge_chains: bool,
        intersection_tolerance: Option<PyAngle>,
        idempotent: bool,
    ) -> Self {
        PyS2Builder {
            snap_function,
            snap_level,
            split_crossing_edges,
            simplify_edge_chains,
            intersection_tolerance: intersection_tolerance.map(|a| a.0).unwrap_or(Angle::ZERO),
            idempotent,
            inputs: Vec::new(),
        }
    }

    /// Add a single point (dimension-0 input).
    fn add_point(&mut self, v: &PyS2Point) {
        self.inputs.push(Input::Point(v.0));
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
    fn build_polygon(&self, py: Python<'_>) -> PyResult<PyPolygon> {
        let mut builder = S2Builder::new(self.options(py)?);
        builder.start_layer(Box::new(S2PolygonLayer::new()));
        self.feed(&mut builder);
        let layers = self.build(builder)?;
        take_layer::<S2PolygonLayer>(layers)?
            .take_output()
            .map(PyPolygon)
            .ok_or_else(no_output)
    }

    /// Assemble the added geometry into a `Polyline`.
    fn build_polyline(&self, py: Python<'_>) -> PyResult<PyPolyline> {
        let mut builder = S2Builder::new(self.options(py)?);
        builder.start_layer(Box::new(S2PolylineLayer::new()));
        self.feed(&mut builder);
        let layers = self.build(builder)?;
        take_layer::<S2PolylineLayer>(layers)?
            .take_output()
            .map(PyPolyline)
            .ok_or_else(no_output)
    }

    /// Assemble the added geometry into a `LaxPolygon`.
    fn build_lax_polygon(&self, py: Python<'_>) -> PyResult<PyLaxPolygon> {
        let mut builder = S2Builder::new(self.options(py)?);
        builder.start_layer(Box::new(LaxPolygonLayer::new()));
        self.feed(&mut builder);
        let layers = self.build(builder)?;
        take_layer::<LaxPolygonLayer>(layers)?
            .take_output()
            .map(PyLaxPolygon)
            .ok_or_else(no_output)
    }

    /// Assemble the added points into a list of snapped points.
    fn build_points(&self, py: Python<'_>) -> PyResult<Vec<PyS2Point>> {
        let mut builder = S2Builder::new(self.options(py)?);
        builder.start_layer(Box::new(S2PointVectorLayer::new()));
        self.feed(&mut builder);
        let layers = self.build(builder)?;
        Ok(take_layer::<S2PointVectorLayer>(layers)?
            .take_output()
            .ok_or_else(no_output)?
            .into_iter()
            .map(PyS2Point)
            .collect())
    }
}

impl PyS2Builder {
    fn options(&self, py: Python<'_>) -> PyResult<Options> {
        let snap: Box<dyn SnapFunction> = match &self.snap_function {
            Some(obj) => resolve_snap(Some(obj.bind(py)))?,
            None => match self.snap_level {
                Some(level) => Box::new(S2CellIdSnapFunction::new(level)),
                None => Box::new(IdentitySnapFunction::new(Angle::ZERO)),
            },
        };
        let mut opts = Options::new(snap);
        opts.split_crossing_edges = self.split_crossing_edges;
        opts.simplify_edge_chains = self.simplify_edge_chains;
        opts.intersection_tolerance = self.intersection_tolerance;
        opts.idempotent = self.idempotent;
        Ok(opts)
    }

    fn build(&self, mut builder: S2Builder) -> PyResult<Vec<Box<dyn Layer>>> {
        builder
            .build()
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    fn feed(&self, builder: &mut S2Builder) {
        for input in &self.inputs {
            match input {
                Input::Point(v) => builder.add_point(*v),
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
