// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s2;
use s2rst::s2::Region;
use s2rst::s2::builder::snap::S2CellIdSnapFunction;
use s2rst::s2::polyline::Polyline;

use crate::angle::PyAngle;
use crate::cells::PyCellId;
use crate::regions::{PyCap, PyRect};
use crate::s2point::{PyLatLng, PyS2Point, PyS2PointIter};

// ---------------------------------------------------------------------------
// Polyline
// ---------------------------------------------------------------------------

/// An open polyline on the unit sphere, defined by a sequence of vertices.
#[pyclass(name = "Polyline", module = "s2rst")]
#[derive(Clone)]
pub struct PyPolyline(pub(crate) Polyline);

#[pymethods]
impl PyPolyline {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    /// Create from a list of S2Point vertices.
    #[new]
    fn new(vertices: Vec<PyS2Point>) -> Self {
        PyPolyline(Polyline::new(vertices.into_iter().map(|p| p.0).collect()))
    }

    /// Create from a list of LatLng vertices.
    #[classmethod]
    fn from_lat_lngs(_cls: &Bound<'_, PyType>, latlngs: Vec<PyLatLng>) -> Self {
        let ll: Vec<s2::LatLng> = latlngs.into_iter().map(|l| l.0).collect();
        PyPolyline(Polyline::from_lat_lngs(&ll))
    }

    // --- Accessors ---

    /// Number of vertices.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// The i-th vertex.
    fn vertex(&self, i: usize) -> PyResult<PyS2Point> {
        if i >= self.0.num_vertices() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index out of range",
            ));
        }
        Ok(PyS2Point(self.0.vertex(i)))
    }

    /// All vertices as a list.
    fn vertices(&self) -> Vec<PyS2Point> {
        self.0
            .vertices_vec()
            .iter()
            .map(|p| PyS2Point(*p))
            .collect()
    }

    // --- Geometry ---

    /// Total arc length of the polyline.
    fn length(&self) -> PyAngle {
        PyAngle(self.0.length())
    }

    /// The centroid of the polyline (mass center weighted by edge length).
    fn centroid(&self) -> PyS2Point {
        PyS2Point(self.0.centroid())
    }

    // --- Queries ---

    /// Project a point onto the polyline. Returns (closest_point, next_vertex_index).
    fn project(&self, point: &PyS2Point) -> (PyS2Point, usize) {
        let (p, idx) = self.0.project(point.0);
        (PyS2Point(p), idx)
    }

    /// Interpolate a point at the given fraction (0.0 = start, 1.0 = end).
    /// Returns (point, next_vertex_index).
    fn interpolate(&self, fraction: f64) -> (PyS2Point, usize) {
        let (p, idx) = self.0.interpolate(fraction);
        (PyS2Point(p), idx)
    }

    /// Inverse of interpolate: given a point on the polyline and the next vertex
    /// index, returns the fraction along the polyline.
    fn uninterpolate(&self, point: &PyS2Point, next_vertex: usize) -> f64 {
        self.0.uninterpolate(point.0, next_vertex)
    }

    /// Whether the given point is to the right of the polyline.
    fn is_on_right(&self, point: &PyS2Point) -> bool {
        self.0.is_on_right(point.0)
    }

    /// Whether this polyline intersects the other polyline.
    fn intersects(&self, other: &PyPolyline) -> bool {
        self.0.intersects(&other.0)
    }

    // --- Simplification ---

    /// Subsample vertices using the given tolerance. Returns indices of kept vertices.
    fn subsample_vertices(&self, tolerance: &PyAngle) -> Vec<usize> {
        self.0.subsample_vertices(tolerance.0)
    }

    /// Whether this polyline nearly covers the other (within max_error).
    fn nearly_covers(&self, covered: &PyPolyline, max_error: &PyAngle) -> bool {
        self.0.nearly_covers(&covered.0, max_error.0)
    }

    // --- Mutation ---

    /// Reverse the order of vertices in place.
    fn reverse(&mut self) {
        self.0.reverse();
    }

    // --- Validation ---

    /// Validate the polyline. Returns None if valid, or an error string.
    fn validate(&self) -> Option<String> {
        self.0.validate().err()
    }

    // --- Comparison ---

    fn equal(&self, other: &PyPolyline) -> bool {
        self.0.equal(&other.0)
    }

    fn approx_equal(&self, other: &PyPolyline, max_error: &PyAngle) -> bool {
        self.0.approx_eq_with(&other.0, max_error.0)
    }

    // --- Region bounds ---

    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    // --- Python protocol ---

    fn __len__(&self) -> usize {
        self.0.num_vertices()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyS2Point> {
        let n = self.0.num_vertices() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyS2Point(self.0.vertex(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyS2PointIter {
        PyS2PointIter::new(self.0.vertices_vec().to_vec())
    }

    fn __eq__(&self, other: &PyPolyline) -> bool {
        self.0.equal(&other.0)
    }

    fn __repr__(&self) -> String {
        format!("Polyline({} vertices)", self.0.num_vertices())
    }

    fn __str__(&self) -> String {
        format!("Polyline({} vertices)", self.0.num_vertices())
    }
}

// ---------------------------------------------------------------------------
// Loop
// ---------------------------------------------------------------------------

/// A closed loop on the unit sphere, representing the boundary of a region.
///
/// Vertices are in counter-clockwise order (the interior is on the left).
/// A loop with clockwise vertices represents a hole (complement region).
#[pyclass(name = "Loop", module = "s2rst")]
#[derive(Clone)]
pub struct PyLoop(pub(crate) s2::Loop);

#[pymethods]
impl PyLoop {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    fn __bool__(&self) -> bool {
        !self.0.is_empty_loop()
    }

    /// Create from a list of S2Point vertices. The list must be non-empty;
    /// use `Loop.empty()` or `Loop.full()` for the special empty/full
    /// loops (which carry a single sentinel vertex internally).
    #[new]
    fn new(vertices: Vec<PyS2Point>) -> PyResult<Self> {
        if vertices.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Loop requires at least one vertex; use Loop.empty() or Loop.full() for the empty/full loops",
            ));
        }
        Ok(PyLoop(s2::Loop::new(
            vertices.into_iter().map(|p| p.0).collect(),
        )))
    }

    /// The empty loop (contains no points).
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyLoop(s2::Loop::empty())
    }

    /// The full loop (contains all points on the sphere).
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyLoop(s2::Loop::full())
    }

    /// Create a loop from the four vertices of a cell.
    #[classmethod]
    fn from_cell(_cls: &Bound<'_, PyType>, cell: &crate::cells::PyCell) -> Self {
        PyLoop(s2::Loop::from_cell(&cell.0))
    }

    /// Create a regular polygon (approximating a circle) with the given center,
    /// angular radius, and number of vertices.
    #[classmethod]
    fn make_regular(
        _cls: &Bound<'_, PyType>,
        center: &PyS2Point,
        radius: &PyAngle,
        num_vertices: usize,
    ) -> Self {
        PyLoop(s2::Loop::make_regular(center.0, radius.0, num_vertices))
    }

    // --- Accessors ---

    /// Number of vertices.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// The i-th vertex. Wraps around (i mod num_vertices) for non-empty
    /// loops, mirroring the core's behavior; raises IndexError when the
    /// loop has zero vertices.
    fn vertex(&self, i: usize) -> PyResult<PyS2Point> {
        if self.0.num_vertices() == 0 {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index out of range: loop has no vertices",
            ));
        }
        Ok(PyS2Point(self.0.vertex(i)))
    }

    /// All vertices as a list.
    fn vertices(&self) -> Vec<PyS2Point> {
        self.0.vertices().iter().map(|p| PyS2Point(*p)).collect()
    }

    /// The nesting depth of this loop within a polygon.
    fn depth(&self) -> i32 {
        self.0.depth()
    }

    // --- Status ---

    /// Whether this is the special empty loop (contains no points).
    fn is_empty_loop(&self) -> bool {
        self.0.is_empty_loop()
    }

    /// Whether this is the special full loop (contains all points).
    fn is_full_loop(&self) -> bool {
        self.0.is_full_loop()
    }

    /// Whether this is either the empty or full loop.
    fn is_empty_or_full(&self) -> bool {
        self.0.is_empty_or_full()
    }

    /// Whether this loop is a hole (odd nesting depth).
    fn is_hole(&self) -> bool {
        self.0.is_hole()
    }

    /// Whether the loop area is at most 2*pi (normalized orientation).
    fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    /// +1 for a shell, -1 for a hole.
    fn sign(&self) -> i32 {
        self.0.sign()
    }

    // --- Geometry ---

    /// Interior area of the loop (0 to 4*pi).
    fn area(&self) -> f64 {
        self.0.area()
    }

    /// Sum of turning angles at each vertex.
    fn turning_angle(&self) -> f64 {
        self.0.turning_angle()
    }

    /// Area-weighted centroid of the loop.
    fn centroid(&self) -> PyS2Point {
        PyS2Point(self.0.centroid())
    }

    /// Sum of exterior angles (total curvature).
    fn get_curvature(&self) -> f64 {
        self.0.get_curvature()
    }

    // --- Containment ---

    /// Whether this loop contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(&p.0)
    }

    /// Whether the origin (reference point) is inside the loop.
    fn contains_origin(&self) -> bool {
        self.0.contains_origin()
    }

    /// Whether this loop fully contains the other loop.
    fn contains_loop(&self, other: &PyLoop) -> bool {
        self.0.contains_loop(&other.0)
    }

    /// Whether this loop intersects the other loop.
    fn intersects_loop(&self, other: &PyLoop) -> bool {
        self.0.intersects_loop(&other.0)
    }

    /// Angular distance from `x` to the loop (0 if `x` is inside).
    fn get_distance(&self, x: &PyS2Point) -> PyAngle {
        PyAngle(self.0.get_distance(x.0))
    }

    /// Angular distance from `x` to the loop's boundary (even if `x` is inside).
    fn get_distance_to_boundary(&self, x: &PyS2Point) -> PyAngle {
        PyAngle(self.0.get_distance_to_boundary(x.0))
    }

    /// The closest point in the loop's interior+boundary to `x` (`x` if inside).
    fn project_point(&self, x: &PyS2Point) -> PyS2Point {
        PyS2Point(self.0.project_point(x.0))
    }

    /// The closest point on the loop's boundary to `x`.
    fn project_to_boundary(&self, x: &PyS2Point) -> PyS2Point {
        PyS2Point(self.0.project_to_boundary(x.0))
    }

    // --- Mutation ---

    /// Ensure area <= 2*pi by possibly inverting vertex order.
    fn normalize(&mut self) {
        self.0.normalize();
    }

    /// Reverse vertex order and flip containment.
    fn invert(&mut self) {
        self.0.invert();
    }

    // --- Validation ---

    /// Validate the loop. Returns None if valid, or an error string.
    fn validate(&self) -> Option<String> {
        self.0.validate().err()
    }

    // --- Comparison ---

    fn equal(&self, other: &PyLoop) -> bool {
        self.0.equal(&other.0)
    }

    fn boundary_approx_equals(&self, other: &PyLoop, max_error: &PyAngle) -> bool {
        self.0.boundary_approx_eq(&other.0, max_error.0)
    }

    fn boundary_near(&self, other: &PyLoop, max_error: &PyAngle) -> bool {
        self.0.boundary_near(&other.0, max_error.0)
    }

    // --- Region bounds ---

    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    // --- Python protocol ---

    fn __len__(&self) -> usize {
        self.0.num_vertices()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyS2Point> {
        let n = self.0.num_vertices() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyS2Point(self.0.vertex(idx as usize)))
        }
    }

    fn __iter__(&self) -> PyS2PointIter {
        PyS2PointIter::new(self.0.vertices().to_vec())
    }

    fn __eq__(&self, other: &PyLoop) -> bool {
        self.0.equal(&other.0)
    }

    fn __repr__(&self) -> String {
        if self.0.is_empty_loop() {
            "Loop(empty)".to_string()
        } else if self.0.is_full_loop() {
            "Loop(full)".to_string()
        } else {
            format!("Loop({} vertices)", self.0.num_vertices())
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Snapshot-based iterator over Loops. Constructed by `Polygon.__iter__`.
#[pyclass]
struct PyLoopIter {
    loops: Vec<s2::Loop>,
    idx: usize,
}

#[pymethods]
impl PyLoopIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyLoop> {
        if self.idx < self.loops.len() {
            let l = self.loops[self.idx].clone();
            self.idx += 1;
            Some(PyLoop(l))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Polygon
// ---------------------------------------------------------------------------

/// A polygon on the unit sphere, consisting of zero or more loops.
///
/// The first loop is the outer shell; subsequent loops are holes.
/// Loops may be nested to arbitrary depth.
#[pyclass(name = "Polygon", module = "s2rst")]
#[derive(Clone)]
pub struct PyPolygon(pub(crate) s2::Polygon);

#[pymethods]
impl PyPolygon {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    fn __bool__(&self) -> bool {
        !self.0.is_empty_polygon()
    }

    /// Create from a list of Loops. Nesting is computed automatically.
    #[new]
    fn new(loops: Vec<PyLoop>) -> Self {
        PyPolygon(s2::Polygon::from_loops(
            loops.into_iter().map(|l| l.0).collect(),
        ))
    }

    /// The empty polygon (contains no points).
    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        PyPolygon(s2::Polygon::empty())
    }

    /// The full polygon (contains all points on the sphere).
    #[classmethod]
    fn full(_cls: &Bound<'_, PyType>) -> Self {
        PyPolygon(s2::Polygon::full())
    }

    /// Create a polygon from a single Cell.
    #[classmethod]
    fn from_cell(_cls: &Bound<'_, PyType>, cell: &crate::cells::PyCell) -> Self {
        PyPolygon(s2::Polygon::from_cell(&cell.0))
    }

    /// Create from loops where orientation determines shell vs hole
    /// (CCW = shell, CW = hole). Nesting is computed automatically.
    #[classmethod]
    fn from_oriented_loops(_cls: &Bound<'_, PyType>, loops: Vec<PyLoop>) -> Self {
        PyPolygon(s2::Polygon::from_oriented_loops(
            loops.into_iter().map(|l| l.0).collect(),
        ))
    }

    // --- Accessors ---

    /// Number of loops in the polygon.
    fn num_loops(&self) -> usize {
        self.0.num_loops()
    }

    /// The k-th loop.
    fn loop_(&self, k: usize) -> PyLoop {
        PyLoop(self.0.loop_at(k).clone())
    }

    /// All loops as a list.
    fn loops(&self) -> Vec<PyLoop> {
        self.0.loops().iter().map(|l| PyLoop(l.clone())).collect()
    }

    /// Total number of vertices across all loops.
    fn num_vertices(&self) -> usize {
        self.0.num_vertices()
    }

    /// Whether the polygon has any holes.
    fn has_holes(&self) -> bool {
        self.0.has_holes()
    }

    // --- Status ---

    /// Whether this is the empty polygon (no loops).
    fn is_empty_polygon(&self) -> bool {
        self.0.is_empty_polygon()
    }

    /// Whether this is the full polygon (single full loop).
    fn is_full_polygon(&self) -> bool {
        self.0.is_full_polygon()
    }

    /// Whether the polygon is normalized.
    fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    // --- Geometry ---

    /// Total area of the polygon interior (0 to 4*pi).
    fn area(&self) -> f64 {
        self.0.area()
    }

    /// Area-weighted centroid.
    fn centroid(&self) -> PyS2Point {
        PyS2Point(self.0.centroid())
    }

    /// If all vertices are centers of cells at the same level, return that level.
    fn get_snap_level(&self) -> Option<u8> {
        self.0.get_snap_level().map(u8::from)
    }

    // --- Loop hierarchy ---

    /// Index of the parent loop of loop k, or None for outermost loops.
    fn parent(&self, k: usize) -> Option<usize> {
        self.0.parent(k)
    }

    /// Index of the last descendant of loop k in pre-order.
    fn last_descendant(&self, k: usize) -> usize {
        self.0.last_descendant(k)
    }

    // --- Containment ---

    /// Whether this polygon contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(&p.0)
    }

    /// Whether this polygon fully contains the other polygon.
    fn contains_polygon(&self, other: &PyPolygon) -> bool {
        self.0.contains_polygon(&other.0)
    }

    /// Whether this polygon intersects the other polygon.
    fn intersects_polygon(&self, other: &PyPolygon) -> bool {
        self.0.intersects_polygon(&other.0)
    }

    /// Angular distance from `x` to the polygon (0 if `x` is inside).
    fn get_distance(&self, x: &PyS2Point) -> PyAngle {
        PyAngle(self.0.get_distance(x.0))
    }

    /// Angular distance from `x` to the polygon's boundary (even if inside).
    fn get_distance_to_boundary(&self, x: &PyS2Point) -> PyAngle {
        PyAngle(self.0.get_distance_to_boundary(x.0))
    }

    /// The closest point in the polygon (interior+boundary) to `x`.
    fn project_point(&self, x: &PyS2Point) -> PyS2Point {
        PyS2Point(self.0.project_point(x.0))
    }

    /// The closest point on the polygon's boundary to `x`.
    fn project_to_boundary(&self, x: &PyS2Point) -> PyS2Point {
        PyS2Point(self.0.project_to_boundary(x.0))
    }

    /// The portions of `polyline` that lie inside this polygon.
    fn intersect_with_polyline(&self, polyline: &PyPolyline) -> Vec<PyPolyline> {
        let mut a = self.0.clone();
        a.intersect_with_polyline(&polyline.0)
            .into_iter()
            .map(PyPolyline)
            .collect()
    }

    /// The portions of `polyline` that lie outside this polygon.
    fn subtract_from_polyline(&self, polyline: &PyPolyline) -> Vec<PyPolyline> {
        let mut a = self.0.clone();
        a.subtract_from_polyline(&polyline.0)
            .into_iter()
            .map(PyPolyline)
            .collect()
    }

    /// The fraction of this polygon's boundary covered by `other`, and of
    /// `other`'s boundary covered by this polygon, as `(self, other)`.
    fn get_overlap_fractions(&self, other: &PyPolygon) -> (f64, f64) {
        let mut a = self.0.clone();
        let mut b = other.0.clone();
        s2::Polygon::get_overlap_fractions(&mut a, &mut b)
    }

    /// A copy of `polygon` with all vertices snapped to S2 cell centers at the
    /// given level.
    #[classmethod]
    fn snapped(_cls: &Bound<'_, PyType>, polygon: &PyPolygon, snap_level: u8) -> Self {
        PyPolygon(s2::Polygon::snapped(&polygon.0, snap_level))
    }

    /// A simplified copy of `polygon`: vertices are snapped to S2 cell centers
    /// at the given level and detail finer than that is removed.
    #[classmethod]
    fn simplified(_cls: &Bound<'_, PyType>, polygon: &PyPolygon, snap_level: u8) -> Self {
        PyPolygon(s2::Polygon::simplified(
            &polygon.0,
            Box::new(S2CellIdSnapFunction::new(snap_level)),
        ))
    }

    // --- Mutation ---

    /// Invert the polygon (take the complement).
    fn invert(&mut self) {
        self.0.invert();
    }

    // --- Boolean operations ---

    /// Return the complement of the polygon.
    fn complement(&self) -> Self {
        PyPolygon(s2::Polygon::complement(&self.0))
    }

    /// Return the union of this polygon with another.
    fn union(&self, other: &PyPolygon) -> Self {
        let mut a = self.0.clone();
        let mut b = other.0.clone();
        PyPolygon(s2::Polygon::union(&mut a, &mut b))
    }

    /// Return the intersection of this polygon with another.
    fn intersection(&self, other: &PyPolygon) -> Self {
        let mut a = self.0.clone();
        let mut b = other.0.clone();
        PyPolygon(s2::Polygon::intersection(&mut a, &mut b))
    }

    /// Return this polygon minus the other (set difference).
    fn difference(&self, other: &PyPolygon) -> Self {
        let mut a = self.0.clone();
        let mut b = other.0.clone();
        PyPolygon(s2::Polygon::difference(&mut a, &mut b))
    }

    /// Return the symmetric difference of this polygon and another.
    fn symmetric_difference(&self, other: &PyPolygon) -> Self {
        let mut a = self.0.clone();
        let mut b = other.0.clone();
        PyPolygon(s2::Polygon::symmetric_difference(&mut a, &mut b))
    }

    /// Compute the union of multiple polygons.
    #[classmethod]
    fn destructive_union(_cls: &Bound<'_, PyType>, polygons: Vec<PyPolygon>) -> Self {
        PyPolygon(s2::Polygon::union_all(
            polygons.into_iter().map(|p| p.0).collect(),
        ))
    }

    // --- Validation ---

    /// Validate the polygon. Returns None if valid, or an error string.
    fn validate(&self) -> Option<String> {
        self.0.validate().err()
    }

    // --- Comparison ---

    fn equals(&self, other: &PyPolygon) -> bool {
        self.0.equal(&other.0)
    }

    fn boundary_equals(&self, other: &PyPolygon) -> bool {
        self.0.boundary_equals(&other.0)
    }

    fn boundary_approx_equals(&self, other: &PyPolygon, max_error: &PyAngle) -> bool {
        self.0.boundary_approx_eq(&other.0, max_error.0)
    }

    fn boundary_near(&self, other: &PyPolygon, max_error: &PyAngle) -> bool {
        self.0.boundary_near(&other.0, max_error.0)
    }

    fn approx_contains(&self, other: &PyPolygon, tolerance: &PyAngle) -> bool {
        self.0.approx_contains(&other.0, tolerance.0)
    }

    // --- Region bounds ---

    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    // --- Python protocol ---

    fn __len__(&self) -> usize {
        self.0.num_loops()
    }

    fn __getitem__(&self, k: isize) -> PyResult<PyLoop> {
        let n = self.0.num_loops() as isize;
        let idx = if k < 0 { k + n } else { k };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyLoop(self.0.loop_at(idx as usize).clone()))
        }
    }

    fn __iter__(&self) -> PyLoopIter {
        PyLoopIter {
            loops: self.0.loops().to_vec(),
            idx: 0,
        }
    }

    fn __eq__(&self, other: &PyPolygon) -> bool {
        self.0.equal(&other.0)
    }

    fn __repr__(&self) -> String {
        if self.0.is_empty_polygon() {
            "Polygon(empty)".to_string()
        } else if self.0.is_full_polygon() {
            "Polygon(full)".to_string()
        } else {
            format!(
                "Polygon({} loops, {} vertices)",
                self.0.num_loops(),
                self.0.num_vertices()
            )
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}
