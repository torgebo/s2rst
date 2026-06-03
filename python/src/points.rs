// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::{r2, r3};

use crate::hash_util::hash_f64s;
use crate::interval::PyR1Interval;

// ---------------------------------------------------------------------------
// R2Point
// ---------------------------------------------------------------------------

/// A point (or vector) in 2D Euclidean space.
#[pyclass(name = "R2Point", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyR2Point(pub(crate) r2::Point);

#[pymethods]
impl PyR2Point {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from (x, y) coordinates.
    #[new]
    fn new(x: f64, y: f64) -> Self {
        PyR2Point(r2::Point::new(x, y))
    }

    /// X coordinate.
    #[getter]
    fn x(&self) -> f64 {
        self.0.x
    }

    /// Y coordinate.
    #[getter]
    fn y(&self) -> f64 {
        self.0.y
    }

    /// Dot product with another point.
    fn dot(&self, other: &PyR2Point) -> f64 {
        self.0.dot(other.0)
    }

    /// Scalar (z component of 3D) cross product with another point.
    fn cross(&self, other: &PyR2Point) -> f64 {
        self.0.cross(other.0)
    }

    /// Counterclockwise orthogonal vector with the same norm.
    fn ortho(&self) -> Self {
        PyR2Point(self.0.ortho())
    }

    /// Squared Euclidean norm (x*x + y*y).
    fn norm2(&self) -> f64 {
        self.0.norm2()
    }

    /// Euclidean norm.
    fn norm(&self) -> f64 {
        self.0.norm()
    }

    /// A unit-length copy of this vector.
    fn normalize(&self) -> Self {
        PyR2Point(self.0.normalize())
    }

    /// Angle from self to other (CCW), in radians. Range [-pi, pi].
    fn angle(&self, other: &PyR2Point) -> f64 {
        self.0.angle(other.0)
    }

    /// Component-wise absolute value.
    fn abs(&self) -> Self {
        PyR2Point(self.0.abs())
    }

    fn __add__(&self, other: &PyR2Point) -> Self {
        PyR2Point(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyR2Point) -> Self {
        PyR2Point(self.0 - other.0)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        PyR2Point(self.0 * scalar)
    }

    fn __rmul__(&self, scalar: f64) -> Self {
        PyR2Point(scalar * self.0)
    }

    fn __truediv__(&self, scalar: f64) -> Self {
        PyR2Point(self.0 / scalar)
    }

    fn __neg__(&self) -> Self {
        PyR2Point(-self.0)
    }

    fn __eq__(&self, other: &PyR2Point) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.x, self.0.y])
    }

    fn __getitem__(&self, i: isize) -> PyResult<f64> {
        let idx = if i < 0 { i + 2 } else { i };
        match idx {
            0 => Ok(self.0.x),
            1 => Ok(self.0.y),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            )),
        }
    }

    fn __len__(&self) -> usize {
        2
    }

    fn __repr__(&self) -> String {
        format!("R2Point({}, {})", self.0.x, self.0.y)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Vector (R3)
// ---------------------------------------------------------------------------

/// A vector in 3D Euclidean space. Foundation type for S2Point.
#[pyclass(name = "Vector", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyVector(pub(crate) r3::Vector);

#[pymethods]
impl PyVector {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from (x, y, z) coordinates.
    #[new]
    fn new(x: f64, y: f64, z: f64) -> Self {
        PyVector(r3::Vector::new(x, y, z))
    }

    /// X coordinate.
    #[getter]
    fn x(&self) -> f64 {
        self.0.x
    }

    /// Y coordinate.
    #[getter]
    fn y(&self) -> f64 {
        self.0.y
    }

    /// Z coordinate.
    #[getter]
    fn z(&self) -> f64 {
        self.0.z
    }

    /// Dot product with another vector.
    fn dot(&self, other: &PyVector) -> f64 {
        self.0.dot(other.0)
    }

    /// Cross product with another vector (right-hand rule).
    fn cross(&self, other: &PyVector) -> Self {
        PyVector(self.0.cross(other.0))
    }

    /// Squared Euclidean norm.
    fn norm2(&self) -> f64 {
        self.0.norm2()
    }

    /// Euclidean norm.
    fn norm(&self) -> f64 {
        self.0.norm()
    }

    /// A unit-length copy of this vector.
    fn normalize(&self) -> Self {
        PyVector(self.0.normalize())
    }

    /// Whether this vector is approximately unit length.
    fn is_unit(&self) -> bool {
        self.0.is_unit()
    }

    /// Component-wise absolute value.
    fn abs(&self) -> Self {
        PyVector(self.0.abs())
    }

    /// Euclidean distance between two vectors.
    fn distance(&self, other: &PyVector) -> f64 {
        self.0.distance(other.0)
    }

    /// Angle between self and other in radians. Range [0, pi].
    fn angle(&self, other: &PyVector) -> f64 {
        self.0.angle(other.0)
    }

    /// Index of the component (0=x, 1=y, 2=z) with largest absolute value.
    fn largest_abs_component(&self) -> usize {
        self.0.largest_abs_component()
    }

    /// Index of the component (0=x, 1=y, 2=z) with smallest absolute value.
    fn smallest_abs_component(&self) -> usize {
        self.0.smallest_abs_component()
    }

    /// A unit vector orthogonal to self.
    fn ortho(&self) -> Self {
        PyVector(self.0.ortho())
    }

    /// Whether two vectors are approximately equal (within 1e-15 per component).
    fn approx_eq(&self, other: &PyVector) -> bool {
        self.0.approx_eq(other.0)
    }

    fn __add__(&self, other: &PyVector) -> Self {
        PyVector(self.0 + other.0)
    }

    fn __sub__(&self, other: &PyVector) -> Self {
        PyVector(self.0 - other.0)
    }

    fn __mul__(&self, scalar: f64) -> Self {
        PyVector(self.0 * scalar)
    }

    fn __rmul__(&self, scalar: f64) -> Self {
        PyVector(scalar * self.0)
    }

    fn __truediv__(&self, scalar: f64) -> Self {
        PyVector(self.0 / scalar)
    }

    fn __neg__(&self) -> Self {
        PyVector(-self.0)
    }

    fn __eq__(&self, other: &PyVector) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.x, self.0.y, self.0.z])
    }

    fn __getitem__(&self, i: isize) -> PyResult<f64> {
        let idx = if i < 0 { i + 3 } else { i };
        match idx {
            0 => Ok(self.0.x),
            1 => Ok(self.0.y),
            2 => Ok(self.0.z),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            )),
        }
    }

    fn __len__(&self) -> usize {
        3
    }

    fn __repr__(&self) -> String {
        format!("Vector({}, {}, {})", self.0.x, self.0.y, self.0.z)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Matrix3x3
// ---------------------------------------------------------------------------

/// A 3x3 matrix in row-major order.
#[pyclass(name = "Matrix3x3", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyMatrix3x3(pub(crate) r3::Matrix3x3);

#[pymethods]
impl PyMatrix3x3 {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from 9 values in row-major order.
    #[new]
    #[allow(clippy::too_many_arguments)]
    fn new(
        m00: f64,
        m01: f64,
        m02: f64,
        m10: f64,
        m11: f64,
        m12: f64,
        m20: f64,
        m21: f64,
        m22: f64,
    ) -> Self {
        PyMatrix3x3(r3::Matrix3x3::new(
            m00, m01, m02, m10, m11, m12, m20, m21, m22,
        ))
    }

    /// Create from three column vectors.
    #[classmethod]
    fn from_cols(_cls: &Bound<'_, PyType>, c0: &PyVector, c1: &PyVector, c2: &PyVector) -> Self {
        PyMatrix3x3(r3::Matrix3x3::from_cols(c0.0, c1.0, c2.0))
    }

    /// The identity matrix.
    #[classmethod]
    fn identity(_cls: &Bound<'_, PyType>) -> Self {
        PyMatrix3x3(r3::Matrix3x3::identity())
    }

    /// The i-th column as a Vector.
    fn col(&self, i: usize) -> PyVector {
        PyVector(self.0.col(i))
    }

    /// The i-th row as a Vector.
    fn row(&self, i: usize) -> PyVector {
        PyVector(self.0.row(i))
    }

    /// Element at (row, col).
    fn get(&self, row: usize, col: usize) -> f64 {
        self.0.get(row, col)
    }

    /// The transposed matrix.
    fn transpose(&self) -> Self {
        PyMatrix3x3(self.0.transpose())
    }

    /// Multiply this matrix by a vector.
    fn mul_vec(&self, v: &PyVector) -> PyVector {
        PyVector(self.0.mul_vec(v.0))
    }

    /// m * v operator.
    fn __matmul__(&self, v: &PyVector) -> PyVector {
        PyVector(self.0.mul_vec(v.0))
    }

    fn __eq__(&self, other: &PyMatrix3x3) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[
            self.0.get(0, 0),
            self.0.get(0, 1),
            self.0.get(0, 2),
            self.0.get(1, 0),
            self.0.get(1, 1),
            self.0.get(1, 2),
            self.0.get(2, 0),
            self.0.get(2, 1),
            self.0.get(2, 2),
        ])
    }

    fn __repr__(&self) -> String {
        format!(
            "Matrix3x3([{}, {}, {}], [{}, {}, {}], [{}, {}, {}])",
            self.0.get(0, 0),
            self.0.get(0, 1),
            self.0.get(0, 2),
            self.0.get(1, 0),
            self.0.get(1, 1),
            self.0.get(1, 2),
            self.0.get(2, 0),
            self.0.get(2, 1),
            self.0.get(2, 2),
        )
    }
}

// ---------------------------------------------------------------------------
// R2Rect
// ---------------------------------------------------------------------------

/// A closed axis-aligned rectangle in the (x, y) plane.
#[pyclass(name = "R2Rect", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyR2Rect(pub(crate) r2::Rect);

#[pymethods]
impl PyR2Rect {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from x and y R1Intervals.
    #[new]
    fn new(x: &PyR1Interval, y: &PyR1Interval) -> Self {
        PyR2Rect(r2::Rect::new(x.0, y.0))
    }

    /// Create from lower-left and upper-right points.
    #[classmethod]
    fn from_points(_cls: &Bound<'_, PyType>, lo: &PyR2Point, hi: &PyR2Point) -> Self {
        PyR2Rect(r2::Rect::from_points(lo.0, hi.0))
    }

    /// The empty rectangle.
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyR2Rect(r2::Rect::empty())
    }

    /// A rectangle containing a single point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: &PyR2Point) -> Self {
        PyR2Rect(r2::Rect::from_point(p.0))
    }

    /// Minimal bounding rectangle of two points.
    #[classmethod]
    fn from_point_pair(_cls: &Bound<'_, PyType>, p1: &PyR2Point, p2: &PyR2Point) -> Self {
        PyR2Rect(r2::Rect::from_point_pair(p1.0, p2.0))
    }

    /// Create from center point and size (width, height).
    #[classmethod]
    fn from_center_size(_cls: &Bound<'_, PyType>, center: &PyR2Point, size: &PyR2Point) -> Self {
        PyR2Rect(r2::Rect::from_center_size(center.0, size.0))
    }

    /// X-axis interval.
    #[getter]
    fn x(&self) -> PyR1Interval {
        PyR1Interval(self.0.x)
    }

    /// Y-axis interval.
    #[getter]
    fn y(&self) -> PyR1Interval {
        PyR1Interval(self.0.y)
    }

    /// Lower-left corner.
    fn lo(&self) -> PyR2Point {
        PyR2Point(self.0.lo())
    }

    /// Upper-right corner.
    fn hi(&self) -> PyR2Point {
        PyR2Point(self.0.hi())
    }

    /// Whether the rectangle has valid bounds.
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Whether the rectangle is empty.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Center of the rectangle.
    fn center(&self) -> PyR2Point {
        PyR2Point(self.0.center())
    }

    /// Size as (width, height).
    fn size(&self) -> PyR2Point {
        PyR2Point(self.0.size())
    }

    /// The k-th vertex (0=lower-left, 1=lower-right, 2=upper-right, 3=upper-left).
    fn vertex(&self, k: i32) -> PyResult<PyR2Point> {
        if !(0..4).contains(&k) {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index must be 0, 1, 2, or 3",
            ));
        }
        Ok(PyR2Point(self.0.vertex(k)))
    }

    /// All four vertices in CCW order starting from lower-left.
    fn vertices(&self) -> [PyR2Point; 4] {
        let v = self.0.vertices();
        [
            PyR2Point(v[0]),
            PyR2Point(v[1]),
            PyR2Point(v[2]),
            PyR2Point(v[3]),
        ]
    }

    /// Whether the closed rectangle contains the point.
    fn contains_point(&self, p: &PyR2Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether the open interior contains the point.
    fn interior_contains_point(&self, p: &PyR2Point) -> bool {
        self.0.interior_contains_point(p.0)
    }

    /// Whether this rectangle contains the other.
    fn contains(&self, other: &PyR2Rect) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this rectangle's open interior contains the other.
    fn interior_contains(&self, other: &PyR2Rect) -> bool {
        self.0.interior_contains(other.0)
    }

    /// Whether this rectangle shares any point with the other.
    fn intersects(&self, other: &PyR2Rect) -> bool {
        self.0.intersects(other.0)
    }

    /// Whether this rectangle's open interior shares any point with the other.
    fn interior_intersects(&self, other: &PyR2Rect) -> bool {
        self.0.interior_intersects(other.0)
    }

    /// Smallest rectangle containing this rectangle and the point.
    fn add_point(&self, p: &PyR2Point) -> Self {
        PyR2Rect(self.0.add_point(p.0))
    }

    /// Smallest rectangle containing this rectangle and the other.
    fn add_rect(&self, other: &PyR2Rect) -> Self {
        PyR2Rect(self.0.add_rect(other.0))
    }

    /// Closest point in the rectangle to p.
    fn project(&self, p: &PyR2Point) -> PyR2Point {
        PyR2Point(self.0.project(p.0))
    }

    /// Rectangle expanded by per-axis margin (margin.x in x, margin.y in y).
    fn expanded(&self, margin: &PyR2Point) -> Self {
        PyR2Rect(self.0.expanded(margin.0))
    }

    /// Rectangle expanded uniformly by `margin` on every side.
    fn expanded_by_margin(&self, margin: f64) -> Self {
        PyR2Rect(self.0.expanded_by_margin(margin))
    }

    /// Smallest rectangle containing both rectangles.
    fn union(&self, other: &PyR2Rect) -> Self {
        PyR2Rect(self.0.union(other.0))
    }

    /// Intersection of the two rectangles (may be empty).
    fn intersection(&self, other: &PyR2Rect) -> Self {
        PyR2Rect(self.0.intersection(other.0))
    }

    /// Whether two rectangles are equal within `max_error` (default 1e-15).
    fn approx_eq(&self, other: &PyR2Rect, max_error: Option<f64>) -> bool {
        self.0.approx_eq_with(other.0, max_error.unwrap_or(1e-15))
    }

    fn __eq__(&self, other: &PyR2Rect) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.x.lo, self.0.x.hi, self.0.y.lo, self.0.y.hi])
    }

    fn __repr__(&self) -> String {
        format!(
            "R2Rect(x=[{}, {}], y=[{}, {}])",
            self.0.x.lo, self.0.x.hi, self.0.y.lo, self.0.y.hi
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}
