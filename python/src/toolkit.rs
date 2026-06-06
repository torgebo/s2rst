// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Low-level geometry toolkit: thin bindings over the core S2 predicate,
//! edge-crossing, edge-distance, wedge-relation, and cube-coordinate
//! primitives.
//!
//! These are the building blocks the higher-level types are made of. The free
//! functions take and return the same wrapper types used elsewhere in the
//! bindings (`S2Point`, `Angle`), and the three result enums (`Direction`,
//! `Crossing`, `WedgeRel`) follow the native-enum pattern from `enums.rs`.

use pyo3::prelude::*;

use s2rst::s2;
use s2rst::s2::coords;
use s2rst::s2::edge_crosser::EdgeCrosser;
use s2rst::s2::edge_crossings;
use s2rst::s2::edge_distances;
use s2rst::s2::predicates::{self, Direction};
use s2rst::s2::wedge_relations::{self, WedgeRel};

use std::ops::ControlFlow;

use s2rst::s2::crossing_edge_query::CrossingType;
use s2rst::s2::shape_util;

use crate::angle::PyAngle;
use crate::index::PyShapeIndex;
use crate::s2point::PyS2Point;
use crate::shapes::PyShape;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The orientation of an ordered triple of points, as reported by
/// [`robust_sign`].
#[pyclass(eq, eq_int, hash, frozen, name = "Direction", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyDirection {
    #[pyo3(name = "CLOCKWISE")]
    Clockwise,
    #[pyo3(name = "INDETERMINATE")]
    Indeterminate,
    #[pyo3(name = "COUNTER_CLOCKWISE")]
    CounterClockwise,
}

impl PyDirection {
    pub(crate) fn from_core(d: Direction) -> Self {
        match d {
            Direction::Clockwise => PyDirection::Clockwise,
            Direction::Indeterminate => PyDirection::Indeterminate,
            Direction::CounterClockwise => PyDirection::CounterClockwise,
        }
    }
}

/// How two edges cross, as reported by [`crossing_sign`].
///
/// - `CROSS`: the edges cross at an interior point.
/// - `MAYBE_CROSS`: two vertices from different edges coincide.
/// - `DO_NOT_CROSS`: the edges do not cross.
#[pyclass(eq, eq_int, hash, frozen, name = "Crossing", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyCrossing {
    #[pyo3(name = "CROSS")]
    Cross,
    #[pyo3(name = "MAYBE_CROSS")]
    MaybeCross,
    #[pyo3(name = "DO_NOT_CROSS")]
    DoNotCross,
}

impl PyCrossing {
    pub(crate) fn from_core(c: edge_crossings::Crossing) -> Self {
        match c {
            edge_crossings::Crossing::Cross => PyCrossing::Cross,
            edge_crossings::Crossing::MaybeCross => PyCrossing::MaybeCross,
            edge_crossings::Crossing::DoNotCross => PyCrossing::DoNotCross,
        }
    }
}

/// The relation between two non-empty wedges, as reported by
/// [`wedge_relation`].
#[pyclass(eq, eq_int, hash, frozen, name = "WedgeRel", module = "s2rst")]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyWedgeRel {
    #[pyo3(name = "EQUAL")]
    Equal,
    #[pyo3(name = "PROPERLY_CONTAINS")]
    ProperlyContains,
    #[pyo3(name = "IS_PROPERLY_CONTAINED")]
    IsProperlyContained,
    #[pyo3(name = "PROPERLY_OVERLAPS")]
    ProperlyOverlaps,
    #[pyo3(name = "IS_DISJOINT")]
    IsDisjoint,
}

impl PyWedgeRel {
    pub(crate) fn from_core(w: WedgeRel) -> Self {
        match w {
            WedgeRel::Equal => PyWedgeRel::Equal,
            WedgeRel::ProperlyContains => PyWedgeRel::ProperlyContains,
            WedgeRel::IsProperlyContained => PyWedgeRel::IsProperlyContained,
            WedgeRel::ProperlyOverlaps => PyWedgeRel::ProperlyOverlaps,
            WedgeRel::IsDisjoint => PyWedgeRel::IsDisjoint,
        }
    }
}

// ---------------------------------------------------------------------------
// Predicates
// ---------------------------------------------------------------------------

/// Whether the points (a, b, c) are in strictly counter-clockwise order.
///
/// This is a fast, non-robust test; for degenerate or near-collinear input
/// prefer [`robust_sign`].
#[pyfunction]
pub fn sign(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> bool {
    predicates::sign(a.0, b.0, c.0)
}

/// The robust orientation of the ordered triple (a, b, c).
///
/// Returns `INDETERMINATE` if and only if two of the points are equal.
#[pyfunction]
pub fn robust_sign(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> PyDirection {
    PyDirection::from_core(predicates::robust_sign(a.0, b.0, c.0))
}

/// Whether a, b, c are encountered in that order when sweeping
/// counter-clockwise around the point o.
#[pyfunction]
pub fn ordered_ccw(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point, o: &PyS2Point) -> bool {
    predicates::ordered_ccw(a.0, b.0, c.0, o.0)
}

// ---------------------------------------------------------------------------
// Edge crossings
// ---------------------------------------------------------------------------

/// How edge AB crosses edge CD.
#[pyfunction]
pub fn crossing_sign(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point, d: &PyS2Point) -> PyCrossing {
    PyCrossing::from_core(edge_crossings::crossing_sign(a.0, b.0, c.0, d.0))
}

/// Whether edges AB and CD that share a vertex "cross" for the purposes of
/// point-in-polygon containment.
#[pyfunction]
pub fn vertex_crossing(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point, d: &PyS2Point) -> bool {
    edge_crossings::vertex_crossing(a.0, b.0, c.0, d.0)
}

/// Whether edges AB and CD cross at an interior point or at a shared vertex.
#[pyfunction]
pub fn edge_or_vertex_crossing(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point, d: &PyS2Point) -> bool {
    edge_crossings::edge_or_vertex_crossing(a.0, b.0, c.0, d.0)
}

/// The intersection point of two crossing edges (a0, a1) and (b0, b1).
///
/// The edges must actually cross; the result is within `INTERSECTION_ERROR`
/// of the true intersection.
#[pyfunction]
pub fn intersection(a0: &PyS2Point, a1: &PyS2Point, b0: &PyS2Point, b1: &PyS2Point) -> PyS2Point {
    PyS2Point(edge_crossings::intersection(a0.0, a1.0, b0.0, b1.0))
}

/// A numerically robust cross product `a x b`, returning a vector orthogonal
/// to both that is never (numerically) zero. Not necessarily unit length.
#[pyfunction]
pub fn robust_cross_prod(a: &PyS2Point, b: &PyS2Point) -> PyS2Point {
    PyS2Point(edge_crossings::robust_cross_prod(a.0, b.0))
}

// ---------------------------------------------------------------------------
// EdgeCrosser
// ---------------------------------------------------------------------------

/// An efficient crossing tester for a fixed edge AB against many edges CD.
///
/// Reusing one crosser for a chain of queries against the same AB is faster
/// than calling the free function [`crossing_sign`] repeatedly.
#[pyclass(name = "EdgeCrosser", module = "s2rst")]
pub struct PyEdgeCrosser(EdgeCrosser);

#[pymethods]
impl PyEdgeCrosser {
    /// Create a crosser for the fixed edge from `a` to `b`.
    #[new]
    fn new(a: &PyS2Point, b: &PyS2Point) -> Self {
        PyEdgeCrosser(EdgeCrosser::new(a.0, b.0))
    }

    /// How the fixed edge AB crosses edge CD.
    fn crossing_sign(&mut self, c: &PyS2Point, d: &PyS2Point) -> PyCrossing {
        PyCrossing::from_core(self.0.crossing_sign(c.0, d.0))
    }

    /// Whether AB and CD cross at an interior point or at a shared vertex.
    fn edge_or_vertex_crossing(&mut self, c: &PyS2Point, d: &PyS2Point) -> bool {
        self.0.edge_or_vertex_crossing(c.0, d.0)
    }
}

// ---------------------------------------------------------------------------
// Edge distances
// ---------------------------------------------------------------------------

/// The closest point on edge AB to the point `x` (the foot of the
/// perpendicular, clamped to the segment).
#[pyfunction]
pub fn project(x: &PyS2Point, a: &PyS2Point, b: &PyS2Point) -> PyS2Point {
    PyS2Point(edge_distances::project(x.0, a.0, b.0))
}

/// The point at fraction `t` of the way along the geodesic from `a` to `b`.
///
/// `interpolate(0, a, b) == a` and `interpolate(1, a, b) == b`.
#[pyfunction]
pub fn interpolate(t: f64, a: &PyS2Point, b: &PyS2Point) -> PyS2Point {
    PyS2Point(edge_distances::interpolate(t, a.0, b.0))
}

/// The point along the geodesic from `a` toward `b` at angular distance `ax`
/// from `a`.
#[pyfunction]
pub fn interpolate_at_distance(ax: &PyAngle, a: &PyS2Point, b: &PyS2Point) -> PyS2Point {
    PyS2Point(edge_distances::interpolate_at_distance(ax.0, a.0, b.0))
}

/// The minimum angular distance from the point `x` to the edge AB.
#[pyfunction]
pub fn distance_from_segment(x: &PyS2Point, a: &PyS2Point, b: &PyS2Point) -> PyAngle {
    PyAngle(edge_distances::distance_from_segment(x.0, a.0, b.0))
}

/// The fraction along edge AB of the closest point to `x`
/// (see [`project`]), in `[0, 1]`.
#[pyfunction]
pub fn distance_fraction(x: &PyS2Point, a: &PyS2Point, b: &PyS2Point) -> f64 {
    edge_distances::distance_fraction(x.0, a.0, b.0)
}

// ---------------------------------------------------------------------------
// Wedge relations
// ---------------------------------------------------------------------------

/// The relation between wedge A = (a0, ab1, a2) and wedge B = (b0, ab1, b2),
/// which share the apex vertex `ab1`.
#[pyfunction]
pub fn wedge_relation(
    a0: &PyS2Point,
    ab1: &PyS2Point,
    a2: &PyS2Point,
    b0: &PyS2Point,
    b2: &PyS2Point,
) -> PyWedgeRel {
    PyWedgeRel::from_core(wedge_relations::wedge_relation(
        a0.0, ab1.0, a2.0, b0.0, b2.0,
    ))
}

/// Whether wedge A = (a0, ab1, a2) contains wedge B = (b0, ab1, b2).
#[pyfunction]
pub fn wedge_contains(
    a0: &PyS2Point,
    ab1: &PyS2Point,
    a2: &PyS2Point,
    b0: &PyS2Point,
    b2: &PyS2Point,
) -> bool {
    wedge_relations::wedge_contains(a0.0, ab1.0, a2.0, b0.0, b2.0)
}

// ---------------------------------------------------------------------------
// Cube-face coordinates
// ---------------------------------------------------------------------------

/// Convert a face number `0..=5` to a `Face`, raising `ValueError` otherwise.
fn face_from_int(face: u8) -> PyResult<coords::Face> {
    coords::Face::try_from(face)
        .map_err(|v| pyo3::exceptions::PyValueError::new_err(format!("invalid face: {v}")))
}

/// Convert `(face, u, v)` cube-face coordinates to a direction vector
/// (not necessarily unit length).
#[pyfunction]
pub fn face_uv_to_xyz(face: u8, u: f64, v: f64) -> PyResult<PyS2Point> {
    let face = face_from_int(face)?;
    Ok(PyS2Point(s2::Point::new(coords::face_uv_to_xyz(
        face, u, v,
    ))))
}

/// Convert a direction vector to `(face, u, v)` cube-face coordinates.
#[pyfunction]
pub fn xyz_to_face_uv(p: &PyS2Point) -> (u8, f64, f64) {
    let (face, u, v) = coords::xyz_to_face_uv(&p.0.vector());
    (face.as_u8(), u, v)
}

/// Map a coordinate `s` in `[0, 1]` to a `u`/`v` coordinate in `[-1, 1]`
/// (the S2 quadratic transform).
#[pyfunction]
pub fn st_to_uv(s: f64) -> f64 {
    coords::st_to_uv(s)
}

/// The inverse of [`st_to_uv`]: map `u` in `[-1, 1]` to `s` in `[0, 1]`.
#[pyfunction]
pub fn uv_to_st(u: f64) -> f64 {
    coords::uv_to_st(u)
}

/// The unit-length u-axis of the given face.
#[pyfunction]
pub fn get_u_axis(face: u8) -> PyResult<PyS2Point> {
    let face = face_from_int(face)?;
    Ok(PyS2Point(s2::Point::new(coords::get_u_axis(face))))
}

/// The unit-length v-axis of the given face.
#[pyfunction]
pub fn get_v_axis(face: u8) -> PyResult<PyS2Point> {
    let face = face_from_int(face)?;
    Ok(PyS2Point(s2::Point::new(coords::get_v_axis(face))))
}

// ----- shape_util -----

/// The points of a dimension-0 (point) `shape` as a list.
#[pyfunction]
pub fn shape_to_points(shape: &PyShape) -> PyResult<Vec<PyS2Point>> {
    shape.with_shape(|s| {
        if u8::from(s.dimension()) != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "shape_to_points requires a dimension-0 (point) shape",
            ));
        }
        Ok(shape_util::shape_to_points(s)
            .into_iter()
            .map(PyS2Point)
            .collect())
    })
}

/// Check an index for self-intersection. Returns `None` if valid, otherwise a
/// description of the first error found.
#[pyfunction]
pub fn find_self_intersection(index: &PyShapeIndex) -> Option<String> {
    shape_util::find_self_intersection(&index.0).map(|e| e.to_string())
}

/// All crossing edge pairs within an index, as `(shape_a, edge_a, shape_b,
/// edge_b)` tuples. With `interior_only=True` only interior crossings are
/// reported; otherwise shared-vertex crossings are included too.
#[pyfunction]
#[pyo3(signature = (index, *, interior_only=true))]
pub fn visit_crossing_edge_pairs(
    index: &PyShapeIndex,
    interior_only: bool,
) -> Vec<(i32, i32, i32, i32)> {
    let cross_type = if interior_only {
        CrossingType::Interior
    } else {
        CrossingType::All
    };
    let mut pairs = Vec::new();
    let _ = shape_util::visit_crossing_edge_pairs(&index.0, cross_type, &mut |a, b, _| {
        pairs.push((a.id.shape_id.0, a.id.edge_id, b.id.shape_id.0, b.id.edge_id));
        ControlFlow::Continue(())
    });
    pairs
}
