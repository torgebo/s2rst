// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for index-level `S2BooleanOperation` — union / intersection /
//! difference / symmetric difference of two `ShapeIndex` regions, plus the
//! `intersects` / `contains` / `equals` fast-path predicates.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use s2rst::s2::boolean_operation::{Options, S2BooleanOperation};
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;

use crate::enums::{PyOpType, PyPolygonModel, PyPolylineModel};
use crate::geometry::PyPolygon;
use crate::index::PyShapeIndex;
use crate::snap::resolve_snap;

fn make_options(
    polygon_model: PyPolygonModel,
    polyline_model: PyPolylineModel,
    snap_function: Option<&Bound<'_, PyAny>>,
) -> PyResult<Options> {
    Ok(Options {
        snap_function: resolve_snap(snap_function)?,
        polygon_model: polygon_model.to_core(),
        polyline_model: polyline_model.to_core(),
        ..Default::default()
    })
}

// Core's `build`/predicates need two independent `&mut ShapeIndex`; the same
// Python object can't be borrowed mutably twice (and `ShapeIndex` is not
// `Clone`), so require two distinct objects.
fn check_distinct(a: &Bound<'_, PyShapeIndex>, b: &Bound<'_, PyShapeIndex>) -> PyResult<()> {
    if a.is(b) {
        Err(PyValueError::new_err(
            "a and b must be different ShapeIndex objects",
        ))
    } else {
        Ok(())
    }
}

/// Compute a boolean operation over two indexed regions, returning a `Polygon`.
#[pyfunction]
#[pyo3(signature = (
    op, a, b, *,
    polygon_model = PyPolygonModel::SemiOpen,
    polyline_model = PyPolylineModel::Closed,
    snap_function = None,
))]
pub fn boolean_operation(
    op: PyOpType,
    a: &Bound<'_, PyShapeIndex>,
    b: &Bound<'_, PyShapeIndex>,
    polygon_model: PyPolygonModel,
    polyline_model: PyPolylineModel,
    snap_function: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPolygon> {
    check_distinct(a, b)?;
    let opts = make_options(polygon_model, polyline_model, snap_function)?;
    let mut a = a.borrow_mut();
    let mut b = b.borrow_mut();
    let mut bop = S2BooleanOperation::new(op.to_core(), Box::new(S2PolygonLayer::new()), opts);
    let mut layers = bop
        .build(&mut a.0, &mut b.0)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let layer = layers
        .pop()
        .ok_or_else(|| PyValueError::new_err("boolean operation produced no output layer"))?;
    let mut layer = *layer
        .into_any()
        .downcast::<S2PolygonLayer>()
        .map_err(|_| PyValueError::new_err("unexpected output layer type"))?;
    let poly = layer
        .take_output()
        .ok_or_else(|| PyValueError::new_err("output layer had no polygon"))?;
    Ok(PyPolygon(poly))
}

macro_rules! predicate {
    ($name:ident, $core:ident, $doc:literal) => {
        #[doc = $doc]
        #[pyfunction]
        #[pyo3(signature = (a, b, *, polygon_model=PyPolygonModel::SemiOpen, polyline_model=PyPolylineModel::Closed))]
        pub fn $name(
            a: &Bound<'_, PyShapeIndex>,
            b: &Bound<'_, PyShapeIndex>,
            polygon_model: PyPolygonModel,
            polyline_model: PyPolylineModel,
        ) -> PyResult<bool> {
            check_distinct(a, b)?;
            let opts = make_options(polygon_model, polyline_model, None)?;
            let mut a = a.borrow_mut();
            let mut b = b.borrow_mut();
            Ok(S2BooleanOperation::$core(&mut a.0, &mut b.0, opts))
        }
    };
}

predicate!(
    intersects,
    intersects,
    "Whether the two indexed regions intersect."
);
predicate!(
    contains,
    contains,
    "Whether region `a` contains region `b`."
);
predicate!(equals, equals, "Whether the two indexed regions are equal.");
