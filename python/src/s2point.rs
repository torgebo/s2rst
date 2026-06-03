// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s2;

use crate::angle::{PyAngle, PyChordAngle};
use crate::hash_util::hash_f64s;
use crate::points::PyVector;

// ---------------------------------------------------------------------------
// S2Point
// ---------------------------------------------------------------------------

/// A point on the unit sphere, represented as a normalized 3D vector.
#[pyclass(name = "S2Point", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyS2Point(pub(crate) s2::Point);

#[pymethods]
impl PyS2Point {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create a normalized point from (x, y, z) coordinates.
    /// If the input is the zero vector, returns the origin.
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        PyS2Point(s2::Point::from_coords(x, y, z))
    }

    /// Create from an existing Vector (NOT normalized).
    #[classmethod]
    fn from_vector(_cls: &Bound<'_, PyType>, v: &PyVector) -> Self {
        PyS2Point(s2::Point::new(v.0))
    }

    /// The reference "origin" point on the sphere (~66km from north pole).
    #[classmethod]
    fn origin(_cls: &Bound<'_, PyType>) -> Self {
        PyS2Point(s2::Point::origin())
    }

    #[getter]
    fn x(&self) -> f64 {
        self.0.x()
    }

    #[getter]
    fn y(&self) -> f64 {
        self.0.y()
    }

    #[getter]
    fn z(&self) -> f64 {
        self.0.z()
    }

    /// The inner r3 Vector.
    fn vector(&self) -> PyVector {
        PyVector(self.0.vector())
    }

    /// Whether this point is approximately unit length.
    fn is_unit(&self) -> bool {
        self.0.is_unit()
    }

    /// Return a normalized copy of this point.
    fn normalize(&self) -> Self {
        PyS2Point(self.0.normalize())
    }

    /// Angle between this point and other, in radians.
    fn distance(&self, other: &PyS2Point) -> PyAngle {
        PyAngle(self.0.distance(other.0))
    }

    /// Chord angle between this point and other. Both must be unit length.
    fn chord_angle(&self, other: &PyS2Point) -> PyChordAngle {
        PyChordAngle(self.0.chord_angle(other.0))
    }

    /// More stable angle using Kahan's formula. Both must be unit length.
    fn stable_angle(&self, other: &PyS2Point) -> PyAngle {
        PyAngle(self.0.stable_angle(other.0))
    }

    /// Whether two points are approximately equal (within 1e-15 radians).
    fn approx_eq(&self, other: &PyS2Point) -> bool {
        self.0.approx_eq(other.0)
    }

    /// Whether two points are within the given angular tolerance.
    fn approx_eq_eps(&self, other: &PyS2Point, eps: &PyAngle) -> bool {
        self.0.approx_eq_angle(other.0, eps.0)
    }

    /// Whether two points are approximately equal within the given max error angle.
    fn approx_equals(&self, other: &PyS2Point, max_error: &PyAngle) -> bool {
        self.0.approx_eq_with(other.0, max_error.0)
    }

    /// Numerically stable cross product. Never returns zero.
    fn point_cross(&self, other: &PyS2Point) -> Self {
        PyS2Point(self.0.point_cross(other.0))
    }

    /// A unit-length reference direction different from self.
    fn reference_dir(&self) -> Self {
        PyS2Point(self.0.reference_dir())
    }

    /// Whether this point can be normalized without precision loss.
    fn is_normalizable(&self) -> bool {
        self.0.is_normalizable()
    }

    /// Scale as necessary to ensure normalization precision.
    fn ensure_normalizable(&self) -> Self {
        PyS2Point(self.0.ensure_normalizable())
    }

    fn __add__(&self, other: &PyS2Point) -> Self {
        PyS2Point(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyS2Point) -> Self {
        PyS2Point(self.0 - other.0)
    }

    fn __neg__(&self) -> Self {
        PyS2Point(-self.0)
    }

    fn __eq__(&self, other: &PyS2Point) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.x(), self.0.y(), self.0.z()])
    }

    fn __lt__(&self, other: &PyS2Point) -> bool {
        self.0 < other.0
    }

    fn __le__(&self, other: &PyS2Point) -> bool {
        self.0 <= other.0
    }

    fn __gt__(&self, other: &PyS2Point) -> bool {
        self.0 > other.0
    }

    fn __ge__(&self, other: &PyS2Point) -> bool {
        self.0 >= other.0
    }

    fn __getitem__(&self, i: isize) -> PyResult<f64> {
        let idx = if i < 0 { i + 3 } else { i };
        match idx {
            0 => Ok(self.0.x()),
            1 => Ok(self.0.y()),
            2 => Ok(self.0.z()),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            )),
        }
    }

    fn __len__(&self) -> usize {
        3
    }

    fn __repr__(&self) -> String {
        format!("S2Point({}, {}, {})", self.0.x(), self.0.y(), self.0.z())
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Snapshot-based iterator over S2Points.
///
/// Constructed by `__iter__` of sequence-like types that contain S2Points
/// (Polyline, Loop, LaxLoop, LaxPolyline, PointVector). The iterator owns
/// a copy of the points so it stays valid if the parent is mutated or
/// dropped.
#[pyclass]
pub(crate) struct PyS2PointIter {
    pts: Vec<s2::Point>,
    idx: usize,
}

impl PyS2PointIter {
    pub(crate) fn new(pts: Vec<s2::Point>) -> Self {
        Self { pts, idx: 0 }
    }
}

#[pymethods]
impl PyS2PointIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyS2Point> {
        if self.idx < self.pts.len() {
            let p = self.pts[self.idx];
            self.idx += 1;
            Some(PyS2Point(p))
        } else {
            None
        }
    }
}

/// S2 ortho: a unit vector orthogonal to the given point.
#[pyfunction]
pub fn s2_ortho(p: &PyS2Point) -> PyS2Point {
    PyS2Point(s2::ortho(p.0))
}

/// Rotate point p about axis by the given angle. Both must be unit length.
#[pyfunction]
pub fn s2_rotate(p: &PyS2Point, axis: &PyS2Point, angle: &PyAngle) -> PyS2Point {
    PyS2Point(s2::rotate(p.0, axis.0, angle.0))
}

// ---------------------------------------------------------------------------
// LatLng
// ---------------------------------------------------------------------------

/// A point on the unit sphere as a (latitude, longitude) pair.
#[pyclass(name = "LatLng", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyLatLng(pub(crate) s2::LatLng);

#[pymethods]
impl PyLatLng {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from two Angle objects.
    #[new]
    fn new(lat: &PyAngle, lng: &PyAngle) -> Self {
        PyLatLng(s2::LatLng::new(lat.0, lng.0))
    }

    /// Create from values in radians.
    #[classmethod]
    fn from_radians(_cls: &Bound<'_, PyType>, lat: f64, lng: f64) -> Self {
        PyLatLng(s2::LatLng::from_radians(lat, lng))
    }

    /// Create from values in degrees.
    #[classmethod]
    fn from_degrees(_cls: &Bound<'_, PyType>, lat: f64, lng: f64) -> Self {
        PyLatLng(s2::LatLng::from_degrees(lat, lng))
    }

    /// Create from E5 representation (degrees * 10^5).
    #[classmethod]
    fn from_e5(_cls: &Bound<'_, PyType>, lat: i32, lng: i32) -> Self {
        PyLatLng(s2::LatLng::from_e5(lat, lng))
    }

    /// Create from E6 representation (degrees * 10^6).
    #[classmethod]
    fn from_e6(_cls: &Bound<'_, PyType>, lat: i32, lng: i32) -> Self {
        PyLatLng(s2::LatLng::from_e6(lat, lng))
    }

    /// Create from E7 representation (degrees * 10^7).
    #[classmethod]
    fn from_e7(_cls: &Bound<'_, PyType>, lat: i32, lng: i32) -> Self {
        PyLatLng(s2::LatLng::from_e7(lat, lng))
    }

    /// Create from an S2Point (direction vector).
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: &PyS2Point) -> Self {
        PyLatLng(s2::LatLng::from_point(p.0))
    }

    /// An invalid LatLng marker value.
    #[classmethod]
    fn invalid(_cls: &Bound<'_, PyType>) -> Self {
        PyLatLng(s2::LatLng::invalid())
    }

    /// Compute the latitude of a direction vector.
    #[staticmethod]
    fn latitude(p: &PyS2Point) -> PyAngle {
        PyAngle(s2::LatLng::latitude(p.0))
    }

    /// Compute the longitude of a direction vector.
    #[staticmethod]
    fn longitude(p: &PyS2Point) -> PyAngle {
        PyAngle(s2::LatLng::longitude(p.0))
    }

    /// The latitude angle.
    #[getter]
    fn lat(&self) -> PyAngle {
        PyAngle(self.0.lat)
    }

    /// The longitude angle.
    #[getter]
    fn lng(&self) -> PyAngle {
        PyAngle(self.0.lng)
    }

    /// Whether lat is in [-pi/2, pi/2] and lng is in [-pi, pi].
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Clamp lat to [-pi/2, pi/2] and reduce lng to [-pi, pi].
    fn normalized(&self) -> Self {
        PyLatLng(self.0.normalized())
    }

    /// Convert to the equivalent unit-length S2Point.
    #[allow(clippy::wrong_self_convention)]
    fn to_point(&self) -> PyS2Point {
        PyS2Point(self.0.to_point())
    }

    /// Surface distance using the Haversine formula.
    fn get_distance(&self, other: &PyLatLng) -> PyAngle {
        PyAngle(self.0.get_distance(other.0))
    }

    /// Whether two LatLngs are approximately equal (within 1e-15 radians per component).
    fn approx_equal(&self, other: &PyLatLng) -> bool {
        self.0.approx_eq(other.0)
    }

    /// Approximately equal with a custom tolerance.
    fn approx_equal_with_max_error(&self, other: &PyLatLng, max_error: &PyAngle) -> bool {
        self.0.approx_eq_with(other.0, max_error.0)
    }

    /// Export as "lat,lng" string in degrees.
    #[allow(clippy::wrong_self_convention)]
    fn to_string_in_degrees(&self) -> String {
        self.0.to_string_in_degrees()
    }

    fn __add__(&self, other: &PyLatLng) -> Self {
        PyLatLng(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyLatLng) -> Self {
        PyLatLng(self.0 - other.0)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        PyLatLng(self.0 * scalar)
    }

    fn __rmul__(&self, scalar: f64) -> Self {
        PyLatLng(scalar * self.0)
    }

    fn __eq__(&self, other: &PyLatLng) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.lat.radians(), self.0.lng.radians()])
    }

    fn __repr__(&self) -> String {
        format!(
            "LatLng({:.7} deg, {:.7} deg)",
            self.0.lat.degrees(),
            self.0.lng.degrees()
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}
