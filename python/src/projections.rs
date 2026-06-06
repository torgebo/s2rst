// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for map projections between the sphere (S2) and the plane (R2).

// `from_lat_lng` mirrors the core `Projection` trait method name; it takes
// `&self` (it projects *from* a lat/lng *using* this projection).
#![allow(clippy::wrong_self_convention)]

use pyo3::prelude::*;

use s2rst::s2::projections::Projection;
use s2rst::s2::projections::{MercatorProjection, PlateCarreeProjection};

use crate::points::PyR2Point;
use crate::s2point::{PyLatLng, PyS2Point};

// ---------------------------------------------------------------------------
// PlateCarreeProjection
// ---------------------------------------------------------------------------

/// The "plate carree" projection: maps (longitude, latitude) linearly to
/// (x, y).
///
/// With `x_scale = 180`, x-coordinates span [-180, 180] and y-coordinates
/// span [-90, 90] (degrees).
#[pyclass(name = "PlateCarreeProjection", module = "s2rst")]
pub struct PyPlateCarreeProjection(pub(crate) PlateCarreeProjection);

#[pymethods]
impl PyPlateCarreeProjection {
    /// Create a projection where x-coordinates span [-`x_scale`, `x_scale`]
    /// and y-coordinates span [-`x_scale`/2, `x_scale`/2].
    #[new]
    fn new(x_scale: f64) -> Self {
        Self(PlateCarreeProjection::new(x_scale))
    }

    /// Convert a point on the sphere to a projected 2D point.
    fn project(&self, p: &PyS2Point) -> PyR2Point {
        PyR2Point(self.0.project(p.0))
    }

    /// Convert a projected 2D point to a point on the sphere.
    fn unproject(&self, p: &PyR2Point) -> PyS2Point {
        PyS2Point(self.0.unproject(p.0))
    }

    /// Project directly from a `LatLng`.
    fn from_lat_lng(&self, ll: &PyLatLng) -> PyR2Point {
        PyR2Point(self.0.from_lat_lng(ll.0))
    }

    /// Unproject directly to a `LatLng`.
    fn to_lat_lng(&self, p: &PyR2Point) -> PyLatLng {
        PyLatLng(self.0.to_lat_lng(p.0))
    }

    /// Interpolate the given fraction of the distance along the line from
    /// `a` to `b`. Fractions outside [0, 1] result in extrapolation.
    fn interpolate(&self, f: f64, a: &PyR2Point, b: &PyR2Point) -> PyR2Point {
        PyR2Point(self.0.interpolate(f, a.0, b.0))
    }

    /// The coordinate wrapping distance along each axis. A value of zero
    /// means no wrapping on that axis.
    fn wrap_distance(&self) -> PyR2Point {
        PyR2Point(self.0.wrap_distance())
    }

    /// Wrap the coordinates of `b` if necessary to obtain the shortest edge
    /// from `a` to `b`.
    fn wrap_destination(&self, a: &PyR2Point, b: &PyR2Point) -> PyR2Point {
        PyR2Point(self.0.wrap_destination(a.0, b.0))
    }

    fn __repr__(&self) -> String {
        "PlateCarreeProjection(...)".to_string()
    }
}

// ---------------------------------------------------------------------------
// MercatorProjection
// ---------------------------------------------------------------------------

/// The spherical Mercator projection. Maps longitude linearly to x, and
/// latitude non-linearly to y. The x-axis is finite and wraps; the y-axis is
/// infinite (the poles have y = ±∞).
///
/// With `max_lng = 180`, the longitude axis spans [-180, 180] (degrees).
#[pyclass(name = "MercatorProjection", module = "s2rst")]
pub struct PyMercatorProjection(pub(crate) MercatorProjection);

#[pymethods]
impl PyMercatorProjection {
    /// Create a projection where the longitude axis spans
    /// [-`max_lng`, `max_lng`].
    #[new]
    fn new(max_lng: f64) -> Self {
        Self(MercatorProjection::new(max_lng))
    }

    /// Convert a point on the sphere to a projected 2D point.
    fn project(&self, p: &PyS2Point) -> PyR2Point {
        PyR2Point(self.0.project(p.0))
    }

    /// Convert a projected 2D point to a point on the sphere.
    fn unproject(&self, p: &PyR2Point) -> PyS2Point {
        PyS2Point(self.0.unproject(p.0))
    }

    /// Project directly from a `LatLng`.
    fn from_lat_lng(&self, ll: &PyLatLng) -> PyR2Point {
        PyR2Point(self.0.from_lat_lng(ll.0))
    }

    /// Unproject directly to a `LatLng`.
    fn to_lat_lng(&self, p: &PyR2Point) -> PyLatLng {
        PyLatLng(self.0.to_lat_lng(p.0))
    }

    /// Interpolate the given fraction of the distance along the line from
    /// `a` to `b`. Fractions outside [0, 1] result in extrapolation.
    fn interpolate(&self, f: f64, a: &PyR2Point, b: &PyR2Point) -> PyR2Point {
        PyR2Point(self.0.interpolate(f, a.0, b.0))
    }

    /// The coordinate wrapping distance along each axis. A value of zero
    /// means no wrapping on that axis.
    fn wrap_distance(&self) -> PyR2Point {
        PyR2Point(self.0.wrap_distance())
    }

    /// Wrap the coordinates of `b` if necessary to obtain the shortest edge
    /// from `a` to `b`.
    fn wrap_destination(&self, a: &PyR2Point, b: &PyR2Point) -> PyR2Point {
        PyR2Point(self.0.wrap_destination(a.0, b.0))
    }

    fn __repr__(&self) -> String {
        "MercatorProjection(...)".to_string()
    }
}
