// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Standalone geometric measure functions over `S2Point` triangles.

use pyo3::prelude::*;

use s2rst::s2::{centroids, point_measures};

use crate::angle::PyAngle;
use crate::s2point::PyS2Point;

/// The area of the spherical triangle (a, b, c), in steradians.
#[pyfunction]
pub fn point_area(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> f64 {
    point_measures::point_area(a.0, b.0, c.0)
}

/// The signed area of the triangle (a, b, c): positive when counter-clockwise.
#[pyfunction]
pub fn signed_area(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> f64 {
    point_measures::signed_area(a.0, b.0, c.0)
}

/// The turning angle (exterior angle) at vertex `b` of the path a -> b -> c.
#[pyfunction]
pub fn turn_angle(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> PyAngle {
    PyAngle(point_measures::turn_angle(a.0, b.0, c.0))
}

/// The true (non-normalized) centroid of the spherical triangle (a, b, c).
#[pyfunction]
pub fn true_centroid(a: &PyS2Point, b: &PyS2Point, c: &PyS2Point) -> PyS2Point {
    PyS2Point(centroids::true_centroid(a.0, b.0, c.0))
}
