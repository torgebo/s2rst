// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::{r1, s1};

use crate::hash_util::hash_f64s;

// ---------------------------------------------------------------------------
// R1Interval
// ---------------------------------------------------------------------------

/// A closed interval [lo, hi] on the real line.
#[pyclass(name = "R1Interval", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyR1Interval(pub(crate) r1::Interval);

#[pymethods]
impl PyR1Interval {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    #[new]
    fn new(lo: f64, hi: f64) -> Self {
        PyR1Interval(r1::Interval::new(lo, hi))
    }

    /// An empty interval.
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyR1Interval(r1::Interval::empty())
    }

    /// An interval containing a single point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: f64) -> Self {
        PyR1Interval(r1::Interval::from_point(p))
    }

    /// The minimal interval containing two points.
    #[classmethod]
    fn from_point_pair(_cls: &Bound<'_, PyType>, p1: f64, p2: f64) -> Self {
        PyR1Interval(r1::Interval::from_point_pair(p1, p2))
    }

    /// Lower bound of the interval.
    #[getter]
    fn lo(&self) -> f64 {
        self.0.lo
    }

    /// Upper bound of the interval.
    #[getter]
    fn hi(&self) -> f64 {
        self.0.hi
    }

    /// Whether the interval is empty (lo > hi).
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Midpoint of the interval, (lo + hi) / 2.
    fn center(&self) -> f64 {
        self.0.center()
    }

    /// Length of the interval, hi - lo. Zero for an empty interval.
    fn length(&self) -> f64 {
        self.0.length()
    }

    /// Whether the closed interval contains point p.
    fn contains(&self, p: f64) -> bool {
        self.0.contains(p)
    }

    /// Whether the open interior (lo, hi) contains point p.
    fn interior_contains(&self, p: f64) -> bool {
        self.0.interior_contains(p)
    }

    /// Whether this interval contains the other interval.
    fn contains_interval(&self, other: &PyR1Interval) -> bool {
        self.0.contains_interval(other.0)
    }

    /// Whether this interval's open interior contains the other.
    fn interior_contains_interval(&self, other: &PyR1Interval) -> bool {
        self.0.interior_contains_interval(other.0)
    }

    /// Whether this interval shares any point with the other.
    fn intersects(&self, other: &PyR1Interval) -> bool {
        self.0.intersects(other.0)
    }

    /// Whether this interval's open interior shares any point with the other.
    fn interior_intersects(&self, other: &PyR1Interval) -> bool {
        self.0.interior_intersects(other.0)
    }

    /// Maximum of `min over q in other |p - q|` for `p` in self.
    fn directed_hausdorff_distance(&self, other: &PyR1Interval) -> f64 {
        self.0.directed_hausdorff_distance(other.0)
    }

    /// Smallest interval containing this interval and the point p.
    fn add_point(&self, p: f64) -> Self {
        PyR1Interval(self.0.add_point(p))
    }

    /// Smallest interval containing this interval and the other.
    fn add_interval(&self, other: &PyR1Interval) -> Self {
        PyR1Interval(self.0.add_interval(other.0))
    }

    /// Closest point in the interval to p.
    fn project(&self, p: f64) -> f64 {
        self.0.project(p)
    }

    /// Interval expanded by `margin` on both sides (or shrunk for negative).
    fn expanded(&self, margin: f64) -> Self {
        PyR1Interval(self.0.expanded(margin))
    }

    /// Smallest interval containing both intervals.
    fn union(&self, other: &PyR1Interval) -> Self {
        PyR1Interval(self.0.union(other.0))
    }

    /// Intersection of the two intervals (may be empty).
    fn intersection(&self, other: &PyR1Interval) -> Self {
        PyR1Interval(self.0.intersection(other.0))
    }

    /// Whether two intervals are equal within `max_error` (default 1e-15).
    fn approx_eq(&self, other: &PyR1Interval, max_error: Option<f64>) -> bool {
        self.0.approx_eq_with(other.0, max_error.unwrap_or(1e-15))
    }

    fn __eq__(&self, other: &PyR1Interval) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.lo, self.0.hi])
    }

    fn __repr__(&self) -> String {
        format!("R1Interval({}, {})", self.0.lo, self.0.hi)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __len__(&self) -> usize {
        2
    }

    fn __getitem__(&self, i: isize) -> PyResult<f64> {
        let idx = if i < 0 { i + 2 } else { i };
        match idx {
            0 => Ok(self.0.lo),
            1 => Ok(self.0.hi),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// S1Interval
// ---------------------------------------------------------------------------

/// A closed interval on the unit circle.
///
/// Points are in [-pi, pi]. The interval can be "inverted" (lo > hi),
/// meaning it passes through the point (-1, 0).
#[pyclass(name = "S1Interval", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyS1Interval(pub(crate) s1::Interval);

#[pymethods]
impl PyS1Interval {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    #[new]
    fn new(lo: f64, hi: f64) -> Self {
        PyS1Interval(s1::Interval::new(lo, hi))
    }

    /// An empty interval.
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyS1Interval(s1::Interval::empty())
    }

    /// The full interval [-pi, pi].
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyS1Interval(s1::Interval::full())
    }

    /// An interval containing a single point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: f64) -> Self {
        PyS1Interval(s1::Interval::from_point(p))
    }

    /// The minimal interval containing two points.
    #[classmethod]
    fn from_point_pair(_cls: &Bound<'_, PyType>, p1: f64, p2: f64) -> Self {
        PyS1Interval(s1::Interval::from_point_pair(p1, p2))
    }

    /// Lower bound of the interval, in radians.
    #[getter]
    fn lo(&self) -> f64 {
        self.0.lo
    }

    /// Upper bound of the interval, in radians.
    #[getter]
    fn hi(&self) -> f64 {
        self.0.hi
    }

    /// Whether the interval has valid bounds.
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Whether the interval covers the entire circle.
    fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Whether the interval is empty.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the interval is inverted (lo > hi, passing through (-1, 0)).
    fn is_inverted(&self) -> bool {
        self.0.is_inverted()
    }

    /// Midpoint of the interval, in radians.
    fn center(&self) -> f64 {
        self.0.center()
    }

    /// Angular length of the interval, in radians.
    fn length(&self) -> f64 {
        self.0.length()
    }

    /// The complement: the interval covering the rest of the circle.
    fn complement(&self) -> Self {
        PyS1Interval(self.0.complement())
    }

    /// Midpoint of the complement interval.
    fn complement_center(&self) -> f64 {
        self.0.complement_center()
    }

    /// Whether the closed interval contains point p.
    fn contains(&self, p: f64) -> bool {
        self.0.contains(p)
    }

    /// Whether the open interior contains point p.
    fn interior_contains(&self, p: f64) -> bool {
        self.0.interior_contains(p)
    }

    /// Whether this interval contains the other interval.
    fn contains_interval(&self, other: &PyS1Interval) -> bool {
        self.0.contains_interval(other.0)
    }

    /// Whether this interval's open interior contains the other.
    fn interior_contains_interval(&self, other: &PyS1Interval) -> bool {
        self.0.interior_contains_interval(other.0)
    }

    /// Whether this interval shares any point with the other.
    fn intersects(&self, other: &PyS1Interval) -> bool {
        self.0.intersects(other.0)
    }

    /// Whether this interval's open interior shares any point with the other.
    fn interior_intersects(&self, other: &PyS1Interval) -> bool {
        self.0.interior_intersects(other.0)
    }

    /// Maximum of `min over q in other |p - q|` for `p` in self (angular).
    fn directed_hausdorff_distance(&self, other: &PyS1Interval) -> f64 {
        self.0.directed_hausdorff_distance(other.0)
    }

    /// Smallest interval containing this interval and the point p.
    fn add_point(&self, p: f64) -> Self {
        PyS1Interval(self.0.add_point(p))
    }

    /// Closest point in the interval to p.
    fn project(&self, p: f64) -> f64 {
        self.0.project(p)
    }

    /// Interval expanded by `margin` radians on both sides.
    fn expanded(&self, margin: f64) -> Self {
        PyS1Interval(self.0.expanded(margin))
    }

    /// Smallest interval containing both intervals.
    fn union(&self, other: &PyS1Interval) -> Self {
        PyS1Interval(self.0.union(other.0))
    }

    /// Intersection of the two intervals (may be empty or two pieces; returns one).
    fn intersection(&self, other: &PyS1Interval) -> Self {
        PyS1Interval(self.0.intersection(other.0))
    }

    /// Whether two intervals are equal within `max_error` (default 1e-15).
    fn approx_eq(&self, other: &PyS1Interval, max_error: Option<f64>) -> bool {
        self.0.approx_eq_with(other.0, max_error.unwrap_or(1e-15))
    }

    fn __eq__(&self, other: &PyS1Interval) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.lo, self.0.hi])
    }

    fn __repr__(&self) -> String {
        format!("S1Interval({}, {})", self.0.lo, self.0.hi)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __len__(&self) -> usize {
        2
    }

    fn __getitem__(&self, i: isize) -> PyResult<f64> {
        let idx = if i < 0 { i + 2 } else { i };
        match idx {
            0 => Ok(self.0.lo),
            1 => Ok(self.0.hi),
            _ => Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            )),
        }
    }
}
