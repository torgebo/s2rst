// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s2;

use crate::angle::{PyAngle, PyChordAngle};
use crate::cells::PyCellId;
use crate::hash_util::hash_f64s;
use crate::interval::{PyR1Interval, PyS1Interval};
use crate::s2point::{PyLatLng, PyS2Point};

// ---------------------------------------------------------------------------
// Cap
// ---------------------------------------------------------------------------

/// A spherical cap: the set of all points within a given angle of a center
/// point. Equivalently, a disc on the unit sphere.
#[pyclass(name = "Cap", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyCap(pub(crate) s2::Cap);

#[pymethods]
impl PyCap {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from center point and chord angle radius.
    #[new]
    fn new(center: &PyS2Point, radius: &PyChordAngle) -> Self {
        PyCap(s2::Cap::from_center_chord_angle(center.0, radius.0))
    }

    /// Create from center point and angular radius.
    #[classmethod]
    fn from_center_angle(_cls: &Bound<'_, PyType>, center: &PyS2Point, angle: &PyAngle) -> Self {
        PyCap(s2::Cap::from_center_angle(center.0, angle.0))
    }

    /// Create from center point and chord angle radius.
    #[classmethod]
    fn from_center_chord_angle(
        _cls: &Bound<'_, PyType>,
        center: &PyS2Point,
        radius: &PyChordAngle,
    ) -> Self {
        PyCap(s2::Cap::from_center_chord_angle(center.0, radius.0))
    }

    /// Create a cap containing a single point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: &PyS2Point) -> Self {
        PyCap(s2::Cap::from_point(p.0))
    }

    /// Create from center point and height (between 0 and 2).
    #[classmethod]
    fn from_center_height(_cls: &Bound<'_, PyType>, center: &PyS2Point, height: f64) -> Self {
        PyCap(s2::Cap::from_center_height(center.0, height))
    }

    /// Create from center point and area (in steradians).
    #[classmethod]
    fn from_center_area(_cls: &Bound<'_, PyType>, center: &PyS2Point, area: f64) -> Self {
        PyCap(s2::Cap::from_center_area(center.0, area))
    }

    /// The empty cap containing no points.
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyCap(s2::Cap::empty())
    }

    /// The full cap containing all points.
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyCap(s2::Cap::full())
    }

    // --- Accessors ---

    /// Center point of the cap.
    #[getter]
    fn center(&self) -> PyS2Point {
        PyS2Point(self.0.center())
    }

    /// Chord angle radius.
    fn chord_radius(&self) -> PyChordAngle {
        PyChordAngle(self.0.chord_radius())
    }

    /// Angular radius.
    fn angle_radius(&self) -> PyAngle {
        PyAngle(self.0.angle_radius())
    }

    /// Height of the cap (distance from the center plane to the base plane).
    fn height(&self) -> f64 {
        self.0.height()
    }

    /// Surface area of the cap in steradians.
    fn area(&self) -> f64 {
        self.0.area()
    }

    /// The centroid of the cap (mass center on the surface).
    fn centroid(&self) -> PyS2Point {
        PyS2Point(self.0.centroid())
    }

    // --- Predicates ---

    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn is_full(&self) -> bool {
        self.0.is_full()
    }

    // --- Containment ---

    /// Whether this cap contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether the interior of this cap contains the given point.
    fn interior_contains_point(&self, p: &PyS2Point) -> bool {
        self.0.interior_contains_point(p.0)
    }

    /// Whether this cap contains the other cap.
    fn contains_cap(&self, other: &PyCap) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this cap intersects the other cap.
    fn intersects(&self, other: &PyCap) -> bool {
        self.0.intersects(other.0)
    }

    /// Whether the interior of this cap intersects the other cap.
    fn interior_intersects(&self, other: &PyCap) -> bool {
        self.0.interior_intersects(other.0)
    }

    // --- Operations ---

    /// The complement of this cap (everything it doesn't contain).
    fn complement(&self) -> Self {
        PyCap(self.0.complement())
    }

    /// A cap expanded by the given angular distance.
    fn expanded(&self, distance: &PyAngle) -> Self {
        PyCap(self.0.expanded(distance.0))
    }

    /// The smallest cap containing both this cap and the other.
    fn union(&self, other: &PyCap) -> Self {
        PyCap(self.0.union(other.0))
    }

    /// Expand this cap to include the given point.
    fn add_point(&self, p: &PyS2Point) -> Self {
        PyCap(self.0.add_point(p.0))
    }

    /// Expand this cap to include the other cap.
    fn add_cap(&self, other: &PyCap) -> Self {
        PyCap(self.0.add_cap(other.0))
    }

    // --- Region bounds ---

    /// A bounding cap (returns self).
    fn cap_bound(&self) -> Self {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle.
    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this cap.
    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    // --- Comparison ---

    fn approx_equal(&self, other: &PyCap) -> bool {
        self.0.approx_eq(other.0)
    }

    fn approx_equal_eps(&self, other: &PyCap, max_error: f64) -> bool {
        self.0.approx_eq_with(other.0, max_error)
    }

    // --- Python operators ---

    fn __eq__(&self, other: &PyCap) -> bool {
        self.0.equal(other.0)
    }

    fn __hash__(&self) -> u64 {
        let c = self.0.center();
        hash_f64s(&[c.x(), c.y(), c.z(), self.0.chord_radius().length2()])
    }

    fn __repr__(&self) -> String {
        let c = self.0.center();
        format!(
            "Cap(center=({:.6}, {:.6}, {:.6}), angle_radius={:.6}°)",
            c.x(),
            c.y(),
            c.z(),
            self.0.angle_radius().degrees()
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Rect (S2 lat/lng rectangle)
// ---------------------------------------------------------------------------

/// A latitude-longitude rectangle on the unit sphere.
#[pyclass(name = "Rect", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyRect(pub(crate) s2::Rect);

#[pymethods]
impl PyRect {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from latitude (R1Interval) and longitude (S1Interval).
    #[new]
    fn new(lat: &PyR1Interval, lng: &PyS1Interval) -> Self {
        PyRect(s2::Rect::new(lat.0, lng.0))
    }

    /// The empty rectangle containing no points.
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyRect(s2::Rect::empty())
    }

    /// The full rectangle containing all points.
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyRect(s2::Rect::full())
    }

    /// A rectangle containing a single lat/lng point.
    #[classmethod]
    fn from_lat_lng(_cls: &Bound<'_, PyType>, ll: &PyLatLng) -> Self {
        PyRect(s2::Rect::from_lat_lng(ll.0))
    }

    /// A rectangle with the given center and size (as LatLng).
    #[classmethod]
    fn from_center_size(_cls: &Bound<'_, PyType>, center: &PyLatLng, size: &PyLatLng) -> Self {
        PyRect(s2::Rect::from_center_size(center.0, size.0))
    }

    // --- Accessors ---

    /// The latitude interval.
    #[getter]
    fn lat(&self) -> PyR1Interval {
        PyR1Interval(self.0.lat)
    }

    /// The longitude interval.
    #[getter]
    fn lng(&self) -> PyS1Interval {
        PyS1Interval(self.0.lng)
    }

    /// Low corner as LatLng.
    fn lo(&self) -> PyLatLng {
        PyLatLng(self.0.lo())
    }

    /// High corner as LatLng.
    fn hi(&self) -> PyLatLng {
        PyLatLng(self.0.hi())
    }

    /// Center of the rectangle as LatLng.
    fn center(&self) -> PyLatLng {
        PyLatLng(self.0.center())
    }

    /// Size of the rectangle as LatLng (lat_size, lng_size).
    fn size(&self) -> PyLatLng {
        PyLatLng(self.0.size())
    }

    /// The k-th vertex (0=SW, 1=SE, 2=NE, 3=NW).
    fn vertex(&self, k: u8) -> PyResult<PyLatLng> {
        let rv = match k {
            0 => s2::RectVertex::LowerLeft,
            1 => s2::RectVertex::LowerRight,
            2 => s2::RectVertex::UpperRight,
            3 => s2::RectVertex::UpperLeft,
            _ => {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "vertex index must be 0, 1, 2, or 3",
                ));
            }
        };
        Ok(PyLatLng(self.0.vertex(rv)))
    }

    /// Surface area of the rectangle in steradians.
    fn area(&self) -> f64 {
        self.0.area()
    }

    /// The centroid of the rectangle.
    fn centroid(&self) -> PyS2Point {
        PyS2Point(self.0.centroid())
    }

    // --- Predicates ---

    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn is_full(&self) -> bool {
        self.0.is_full()
    }

    /// Whether this rectangle is a single point.
    fn is_point(&self) -> bool {
        self.0.is_point()
    }

    // --- Containment ---

    /// Whether this rectangle contains the given lat/lng.
    fn contains_lat_lng(&self, ll: &PyLatLng) -> bool {
        self.0.contains_lat_lng(ll.0)
    }

    /// Whether this rectangle contains the given S2Point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this rectangle contains the other rectangle.
    fn contains_rect(&self, other: &PyRect) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this rectangle intersects the other rectangle.
    fn intersects(&self, other: &PyRect) -> bool {
        self.0.intersects(other.0)
    }

    // --- Operations ---

    /// Expand to include the given lat/lng point.
    fn add_point(&self, ll: &PyLatLng) -> Self {
        PyRect(self.0.add_point(ll.0))
    }

    /// Closure of the rectangle at the poles.
    fn polar_closure(&self) -> Self {
        PyRect(self.0.polar_closure())
    }

    /// Expand by the given margin (as LatLng of half-widths).
    fn expanded(&self, margin: &PyLatLng) -> Self {
        PyRect(self.0.expanded(margin.0))
    }

    /// The smallest rectangle containing both this and the other.
    fn union(&self, other: &PyRect) -> Self {
        PyRect(self.0.union(other.0))
    }

    /// The intersection of this and the other rectangle.
    fn intersection(&self, other: &PyRect) -> Self {
        PyRect(self.0.intersection(other.0))
    }

    /// Expand to account for error when computing bounding rectangles of subregions.
    fn expand_for_subregions(&self) -> Self {
        PyRect(self.0.expand_for_subregions())
    }

    // --- Region bounds ---

    /// A bounding spherical cap.
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle (returns self).
    fn rect_bound(&self) -> Self {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this rectangle.
    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    // --- Comparison ---

    fn approx_equal(&self, other: &PyRect) -> bool {
        self.0.approx_eq(other.0)
    }

    // --- Python operators ---

    fn __eq__(&self, other: &PyRect) -> bool {
        self.0.lat == other.0.lat && self.0.lng == other.0.lng
    }

    fn __hash__(&self) -> u64 {
        hash_f64s(&[self.0.lat.lo, self.0.lat.hi, self.0.lng.lo, self.0.lng.hi])
    }

    fn __repr__(&self) -> String {
        format!(
            "Rect(lat=[{:.6}°, {:.6}°], lng=[{:.6}°, {:.6}°])",
            self.0.lat.lo.to_degrees(),
            self.0.lat.hi.to_degrees(),
            self.0.lng.lo.to_degrees(),
            self.0.lng.hi.to_degrees(),
        )
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}
