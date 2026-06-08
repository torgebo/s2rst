// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Binding for `S2WindingOperation` — N-way boolean polygon operations over a
//! set of (possibly overlapping or self-intersecting) loops, resolved by a
//! winding rule.
//!
//! Exposed as a single `winding_operation` function that takes all loops at
//! once together with a reference point and its known winding number.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use s2rst::s2::Point;
use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::winding_operation::{S2WindingOperation, WindingOptions};

use crate::enums::PyWindingRule;
use crate::geometry::PyPolygon;
use crate::s2point::PyS2Point;
use crate::snap::resolve_snap;

/// Partition the sphere by winding number over the given `loops` and return the
/// region selected by `rule` as a `Polygon`.
///
/// `loops` is a list of closed loops (each a list of points). `ref_point` is a
/// point whose winding number `ref_winding` is known; the result's winding is
/// computed relative to it. `include_degeneracies` keeps sibling edge pairs and
/// isolated vertices in the output.
#[pyfunction]
#[pyo3(signature = (
    loops,
    ref_point,
    ref_winding,
    rule,
    *,
    snap_function = None,
    include_degeneracies = false,
))]
pub fn winding_operation(
    loops: Vec<Vec<PyS2Point>>,
    ref_point: &PyS2Point,
    ref_winding: i32,
    rule: PyWindingRule,
    snap_function: Option<&Bound<'_, PyAny>>,
    include_degeneracies: bool,
) -> PyResult<PyPolygon> {
    let mut options = WindingOptions::with_snap_function(resolve_snap(snap_function)?);
    options.set_include_degeneracies(include_degeneracies);

    let mut op = S2WindingOperation::new(Box::new(S2PolygonLayer::new()), options);
    for loop_pts in &loops {
        let verts: Vec<Point> = loop_pts.iter().map(|p| p.0).collect();
        op.add_loop(&verts);
    }

    let layer = op
        .build(ref_point.0, ref_winding, rule.to_core())
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let mut layer = *layer
        .into_any()
        .downcast::<S2PolygonLayer>()
        .map_err(|_| PyValueError::new_err("unexpected output layer type"))?;
    let poly = layer
        .take_output()
        .ok_or_else(|| PyValueError::new_err("winding operation produced no polygon"))?;
    Ok(PyPolygon(poly))
}
