// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for polyline simplification and DTW-based vertex alignment.
//!
//! [`PyPolylineSimplifier`] wraps the angular-constraint simplifier. The
//! free functions wrap the Dynamic Time Warping alignment operations,
//! flattening `VertexAlignment` into a `(warp_path, cost)` tuple.

use pyo3::prelude::*;

use s2rst::s2::polyline::Polyline;
use s2rst::s2::polyline_alignment;
use s2rst::s2::polyline_alignment::{ConsensusOptions, MedoidOptions, VertexAlignment};
use s2rst::s2::polyline_simplifier::PolylineSimplifier;

use crate::angle::PyChordAngle;
use crate::geometry::PyPolyline;
use crate::s2point::PyS2Point;

// ---------------------------------------------------------------------------
// PolylineSimplifier
// ---------------------------------------------------------------------------

/// Simplifies polylines by tracking an allowable range of output edge
/// directions.
///
/// Call `init(src)` to start a new simplified edge, then constrain it with
/// `target_disc` (the edge must pass through the disc) and `avoid_disc` (the
/// edge must stay clear of the disc). Finally test a candidate endpoint with
/// `extend(dst)`. Containment uses exact arithmetic and is conservative with
/// respect to floating-point error.
#[pyclass(name = "PolylineSimplifier", module = "s2rst")]
pub struct PyPolylineSimplifier(PolylineSimplifier);

#[pymethods]
impl PyPolylineSimplifier {
    /// Create a new simplifier. Call `init` before use.
    #[new]
    fn new() -> Self {
        Self(PolylineSimplifier::new())
    }

    /// Start a new simplified edge at `src`.
    fn init(&mut self, src: &PyS2Point) {
        self.0.init(src.0);
    }

    /// The source vertex of the current output edge.
    fn src(&self) -> PyS2Point {
        PyS2Point(self.0.src())
    }

    /// Whether the edge `(src, dst)` satisfies all targeting requirements so
    /// far. Returns False if the edge would be longer than 90 degrees.
    fn extend(&self, dst: &PyS2Point) -> bool {
        self.0.extend(dst.0)
    }

    /// Require the output edge to pass through the disc centered at `point`
    /// with the given radius. Returns whether that is still possible.
    fn target_disc(&mut self, point: &PyS2Point, radius: &PyChordAngle) -> bool {
        self.0.target_disc(point.0, radius.0)
    }

    /// Require the output edge to avoid the disc centered at `point` with the
    /// given radius. `disc_on_left` selects which side of the edge the disc
    /// must lie on. Returns whether that is still possible.
    fn avoid_disc(&mut self, point: &PyS2Point, radius: &PyChordAngle, disc_on_left: bool) -> bool {
        self.0.avoid_disc(point.0, radius.0, disc_on_left)
    }

    fn __repr__(&self) -> String {
        "PolylineSimplifier(...)".to_string()
    }
}

// ---------------------------------------------------------------------------
// Vertex alignment free functions
// ---------------------------------------------------------------------------

/// Flatten a `VertexAlignment` into `(warp_path, alignment_cost)`.
fn flatten_alignment(alignment: VertexAlignment) -> (Vec<(usize, usize)>, f64) {
    (alignment.warp_path, alignment.alignment_cost)
}

/// The exact optimal vertex alignment between two non-empty polylines.
///
/// Returns `(warp_path, cost)`, where `warp_path` is a list of
/// `(index_into_a, index_into_b)` pairs. Time and space are O(A * B).
#[pyfunction]
pub fn get_exact_vertex_alignment(a: &PyPolyline, b: &PyPolyline) -> (Vec<(usize, usize)>, f64) {
    flatten_alignment(polyline_alignment::get_exact_vertex_alignment(&a.0, &b.0))
}

/// The cost of the exact optimal alignment between two non-empty polylines.
///
/// Space-efficient (O(max(A, B))) variant that returns only the cost.
#[pyfunction]
pub fn get_exact_vertex_alignment_cost(a: &PyPolyline, b: &PyPolyline) -> f64 {
    polyline_alignment::get_exact_vertex_alignment_cost(&a.0, &b.0)
}

/// An approximate optimal alignment using the FastDTW algorithm.
///
/// `radius` controls the search window size; larger is more accurate. The
/// returned cost is never less than the exact alignment cost. Returns
/// `(warp_path, cost)`.
#[pyfunction]
pub fn get_approx_vertex_alignment(
    a: &PyPolyline,
    b: &PyPolyline,
    radius: usize,
) -> (Vec<(usize, usize)>, f64) {
    flatten_alignment(polyline_alignment::get_approx_vertex_alignment(
        &a.0, &b.0, radius,
    ))
}

/// The index of the medoid polyline: the one minimizing summed alignment
/// cost to all others. Uses the default (approximate) medoid options.
///
/// Raises if `polylines` is empty.
#[pyfunction]
pub fn get_medoid_polyline(polylines: Vec<PyPolyline>) -> usize {
    let polys: Vec<Polyline> = polylines.into_iter().map(|p| p.0).collect();
    polyline_alignment::get_medoid_polyline(&polys, &MedoidOptions::default())
}

/// A consensus polyline computed via DTW Barycenter Averaging, using the
/// default consensus options.
///
/// Raises if `polylines` is empty.
#[pyfunction]
pub fn get_consensus_polyline(polylines: Vec<PyPolyline>) -> PyPolyline {
    let polys: Vec<Polyline> = polylines.into_iter().map(|p| p.0).collect();
    PyPolyline(polyline_alignment::get_consensus_polyline(
        &polys,
        &ConsensusOptions::default(),
    ))
}
