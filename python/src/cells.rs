// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};

use s2rst::s2;

use crate::angle::PyAngle;
use crate::regions::{PyCap, PyRect};
use crate::s2point::{PyLatLng, PyS2Point};

fn cell_edge_from_index(k: u8) -> PyResult<s2::CellEdge> {
    match k {
        0 => Ok(s2::CellEdge::Bottom),
        1 => Ok(s2::CellEdge::Right),
        2 => Ok(s2::CellEdge::Top),
        3 => Ok(s2::CellEdge::Left),
        _ => Err(pyo3::exceptions::PyIndexError::new_err(
            "edge index must be 0, 1, 2, or 3",
        )),
    }
}

// ---------------------------------------------------------------------------
// CellId
// ---------------------------------------------------------------------------

/// A 64-bit identifier for a cell in the S2 cell decomposition.
#[pyclass(name = "CellId", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyCellId(pub(crate) s2::CellId);

#[pymethods]
impl PyCellId {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from a raw 64-bit cell ID.
    #[new]
    fn new(id: u64) -> Self {
        PyCellId(s2::CellId(id))
    }

    /// The invalid zero cell ID.
    #[classmethod]
    fn none(_cls: &Bound<'_, PyType>) -> Self {
        PyCellId(s2::CellId::none())
    }

    /// The sentinel cell ID, larger than any valid cell.
    #[classmethod]
    fn sentinel(_cls: &Bound<'_, PyType>) -> Self {
        PyCellId(s2::CellId::sentinel())
    }

    /// Cell for a given face (0-5).
    #[classmethod]
    fn from_face(_cls: &Bound<'_, PyType>, face: u8) -> Self {
        PyCellId(s2::CellId::from_face(face))
    }

    /// Cell from face, Hilbert curve position, and level.
    #[classmethod]
    fn from_face_pos_level(_cls: &Bound<'_, PyType>, face: u8, pos: u64, level: u8) -> Self {
        PyCellId(s2::CellId::from_face_pos_level(face, pos, level))
    }

    /// Leaf cell containing the given point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: &PyS2Point) -> Self {
        PyCellId(s2::CellId::from_point(&p.0))
    }

    /// Leaf cell containing the given lat/lng.
    #[classmethod]
    fn from_lat_lng(_cls: &Bound<'_, PyType>, ll: &PyLatLng) -> Self {
        PyCellId(s2::CellId::from_lat_lng(&ll.0))
    }

    /// Parse from a hex-encoded token string.
    #[classmethod]
    fn from_token(_cls: &Bound<'_, PyType>, token: &str) -> Self {
        PyCellId(s2::CellId::from_token(token))
    }

    /// Parse from "face/childpath" debug format (e.g. "3/012").
    #[classmethod]
    fn from_debug_string(_cls: &Bound<'_, PyType>, s: &str) -> Option<Self> {
        s2::CellId::from_debug_string(s).map(PyCellId)
    }

    /// The raw 64-bit cell ID.
    #[getter]
    fn id(&self) -> u64 {
        self.0.id()
    }

    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Cube face number (0-5).
    fn face(&self) -> u8 {
        u8::from(self.0.face())
    }

    /// Position along the Hilbert curve.
    fn pos(&self) -> u64 {
        self.0.pos()
    }

    /// Subdivision level (0-30).
    fn level(&self) -> u8 {
        u8::from(self.0.level())
    }

    /// Whether this is a leaf cell (level 30).
    fn is_leaf(&self) -> bool {
        self.0.is_leaf()
    }

    /// Whether this is a face cell (level 0).
    fn is_face(&self) -> bool {
        self.0.is_face()
    }

    /// Position of this cell relative to its parent at the given level.
    fn child_position(&self, level: u8) -> u8 {
        self.0.child_position(level)
    }

    // --- Hierarchy ---

    /// Parent cell at the given level.
    fn parent_at_level(&self, level: u8) -> Self {
        PyCellId(self.0.parent_at_level(level))
    }

    /// Immediate parent cell.
    fn parent(&self) -> Self {
        PyCellId(self.0.parent())
    }

    /// Four immediate children as a list.
    fn children(&self) -> [PyCellId; 4] {
        let c = self.0.children();
        [
            PyCellId(c[0]),
            PyCellId(c[1]),
            PyCellId(c[2]),
            PyCellId(c[3]),
        ]
    }

    /// First child in Hilbert order.
    fn child_begin(&self) -> Self {
        PyCellId(self.0.child_begin())
    }

    /// First cell past the last child.
    fn child_end(&self) -> Self {
        PyCellId(self.0.child_end())
    }

    /// First child at the given level.
    fn child_begin_at_level(&self, level: u8) -> Self {
        PyCellId(self.0.child_begin_at_level(level))
    }

    /// First cell past the last child at the given level.
    fn child_end_at_level(&self, level: u8) -> Self {
        PyCellId(self.0.child_end_at_level(level))
    }

    /// Minimum CellId in this cell's range.
    fn range_min(&self) -> Self {
        PyCellId(self.0.range_min())
    }

    /// Maximum CellId in this cell's range.
    fn range_max(&self) -> Self {
        PyCellId(self.0.range_max())
    }

    /// Whether this cell contains the other cell.
    fn contains(&self, other: &PyCellId) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this cell intersects the other cell.
    fn intersects(&self, other: &PyCellId) -> bool {
        self.0.intersects(other.0)
    }

    // --- Traversal ---

    /// Next cell along the Hilbert curve at the same level.
    fn next(&self) -> Self {
        PyCellId(self.0.next())
    }

    /// Previous cell along the Hilbert curve at the same level.
    fn prev(&self) -> Self {
        PyCellId(self.0.prev())
    }

    /// Next cell with wrapping at face boundaries.
    fn next_wrap(&self) -> Self {
        PyCellId(self.0.next_wrap())
    }

    /// Previous cell with wrapping at face boundaries.
    fn prev_wrap(&self) -> Self {
        PyCellId(self.0.prev_wrap())
    }

    /// Advance (or retreat) by the given number of steps.
    fn advance(&self, steps: i64) -> Self {
        PyCellId(self.0.advance(steps))
    }

    /// Advance with wrapping at face boundaries.
    fn advance_wrap(&self, steps: i64) -> Self {
        PyCellId(self.0.advance_wrap(steps))
    }

    /// Number of steps from Begin() at this level.
    fn distance_from_begin(&self) -> i64 {
        self.0.distance_from_begin()
    }

    // --- Geometry ---

    /// Center of this cell as a unit-length S2Point.
    #[allow(clippy::wrong_self_convention)]
    fn to_point(&self) -> PyS2Point {
        PyS2Point(self.0.to_point())
    }

    /// Center of this cell as a LatLng.
    #[allow(clippy::wrong_self_convention)]
    fn to_lat_lng(&self) -> PyLatLng {
        PyLatLng(self.0.to_lat_lng())
    }

    /// Hex-encoded token string.
    #[allow(clippy::wrong_self_convention)]
    fn to_token(&self) -> String {
        self.0.to_token()
    }

    /// Debug string in "face/childpath" format.
    #[allow(clippy::wrong_self_convention)]
    fn to_debug_string(&self) -> String {
        self.0.to_debug_string()
    }

    // --- Neighbors ---

    /// Four edge-adjacent cells at the same level.
    fn edge_neighbors(&self) -> [PyCellId; 4] {
        let n = self.0.edge_neighbors();
        [
            PyCellId(n[0]),
            PyCellId(n[1]),
            PyCellId(n[2]),
            PyCellId(n[3]),
        ]
    }

    /// Cells at the given level sharing a vertex with this cell's closest vertex.
    fn vertex_neighbors(&self, level: u8) -> Vec<PyCellId> {
        self.0
            .vertex_neighbors(level)
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    /// All neighboring cells at the given level.
    fn all_neighbors(&self, level: u8) -> Option<Vec<PyCellId>> {
        self.0
            .all_neighbors(level)
            .map(|v| v.into_iter().map(PyCellId).collect())
    }

    // --- Advanced ---

    /// Level of the lowest common ancestor with the other cell.
    fn common_ancestor_level(&self, other: &PyCellId) -> Option<u8> {
        self.0.common_ancestor_level(other.0).map(u8::from)
    }

    /// Largest cell with the same range_min that fits before limit.
    fn maximum_tile(&self, limit: &PyCellId) -> Self {
        PyCellId(self.0.maximum_tile(limit.0))
    }

    // --- Python operators ---

    fn __eq__(&self, other: &PyCellId) -> bool {
        self.0 == other.0
    }

    fn __lt__(&self, other: &PyCellId) -> bool {
        self.0 < other.0
    }

    fn __le__(&self, other: &PyCellId) -> bool {
        self.0 <= other.0
    }

    fn __gt__(&self, other: &PyCellId) -> bool {
        self.0 > other.0
    }

    fn __ge__(&self, other: &PyCellId) -> bool {
        self.0 >= other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.id()
    }

    fn __int__(&self) -> u64 {
        self.0.id()
    }

    fn __repr__(&self) -> String {
        format!("CellId({})", self.0.to_debug_string())
    }

    fn __str__(&self) -> String {
        self.0.to_debug_string()
    }
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A concrete S2 cell with precomputed geometric bounds.
#[pyclass(name = "Cell", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyCell(pub(crate) s2::Cell);

#[pymethods]
impl PyCell {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create from a CellId.
    #[new]
    fn new(id: &PyCellId) -> Self {
        PyCell(s2::Cell::from_cell_id(id.0))
    }

    /// Leaf cell containing the given point.
    #[classmethod]
    fn from_point(_cls: &Bound<'_, PyType>, p: &PyS2Point) -> Self {
        PyCell(s2::Cell::from_point(p.0))
    }

    /// Leaf cell containing the given lat/lng.
    #[classmethod]
    fn from_lat_lng(_cls: &Bound<'_, PyType>, ll: &PyLatLng) -> Self {
        PyCell(s2::Cell::from_lat_lng(ll.0))
    }

    /// Cube face (0-5).
    fn face(&self) -> u8 {
        u8::from(self.0.face())
    }

    /// Subdivision level (0-30).
    fn level(&self) -> u8 {
        u8::from(self.0.level())
    }

    /// The CellId of this cell.
    fn id(&self) -> PyCellId {
        PyCellId(self.0.id())
    }

    /// Whether this is a leaf cell (level 30).
    fn is_leaf(&self) -> bool {
        self.0.is_leaf()
    }

    /// Normalized k-th vertex (k=0,1,2,3) in CCW order.
    fn vertex(&self, k: usize) -> PyResult<PyS2Point> {
        if k >= 4 {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index must be 0, 1, 2, or 3",
            ));
        }
        Ok(PyS2Point(self.0.vertex(k)))
    }

    /// Unnormalized k-th vertex (k=0,1,2,3).
    fn vertex_raw(&self, k: usize) -> PyResult<PyS2Point> {
        if k >= 4 {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "vertex index must be 0, 1, 2, or 3",
            ));
        }
        Ok(PyS2Point(self.0.vertex_raw(k)))
    }

    /// Normalized inward-facing normal of edge k (k=0,1,2,3 for
    /// Bottom, Right, Top, Left).
    fn edge(&self, k: u8) -> PyResult<PyS2Point> {
        Ok(PyS2Point(self.0.edge(cell_edge_from_index(k)?)))
    }

    /// Unnormalized inward-facing normal of edge k (k=0,1,2,3 for
    /// Bottom, Right, Top, Left).
    fn edge_raw(&self, k: u8) -> PyResult<PyS2Point> {
        Ok(PyS2Point(self.0.edge_raw(cell_edge_from_index(k)?)))
    }

    /// Center as a unit-length S2Point.
    fn center(&self) -> PyS2Point {
        PyS2Point(self.0.center())
    }

    /// Four direct children, or None if this is a leaf cell.
    fn children(&self) -> Option<[PyCell; 4]> {
        self.0
            .children()
            .map(|c| [PyCell(c[0]), PyCell(c[1]), PyCell(c[2]), PyCell(c[3])])
    }

    /// Average area of cells at this level.
    fn average_area(&self) -> f64 {
        self.0.average_area()
    }

    /// Approximate area of this specific cell.
    fn approx_area(&self) -> f64 {
        self.0.approx_area()
    }

    /// Whether this cell contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this cell contains another cell.
    fn contains_cell(&self, other: &PyCell) -> bool {
        self.0.contains_cell(other.0)
    }

    /// Whether this cell intersects another cell.
    fn intersects_cell(&self, other: &PyCell) -> bool {
        self.0.intersects_cell(other.0)
    }

    // --- Region bounds ---

    /// A bounding spherical cap for this cell.
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle for this cell.
    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this cell.
    fn cell_union_bound(&self) -> Vec<PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(PyCellId)
            .collect()
    }

    fn __eq__(&self, other: &PyCell) -> bool {
        self.0 == other.0
    }

    fn __hash__(&self) -> u64 {
        self.0.id().id()
    }

    fn __repr__(&self) -> String {
        format!("Cell({})", self.0.id().to_debug_string())
    }

    fn __str__(&self) -> String {
        self.0.id().to_debug_string()
    }
}

// ---------------------------------------------------------------------------
// CellUnion
// ---------------------------------------------------------------------------

/// A set of S2 cells, automatically normalized (sorted, deduplicated, merged).
#[pyclass(name = "CellUnion", module = "s2rst")]
#[derive(Clone)]
pub struct PyCellUnion(pub(crate) s2::CellUnion);

#[pymethods]
impl PyCellUnion {
    fn __copy__(&self) -> Self {
        Self(self.0.clone())
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0.clone())
    }

    /// Create an empty cell union.
    #[new]
    fn new() -> Self {
        PyCellUnion(s2::CellUnion::new())
    }

    /// Create from a list of CellIds, normalizing the result.
    #[classmethod]
    fn from_cell_ids(_cls: &Bound<'_, PyType>, ids: Vec<PyCellId>) -> Self {
        let raw: Vec<s2::CellId> = ids.into_iter().map(|c| c.0).collect();
        PyCellUnion(s2::CellUnion::from_cell_ids(raw))
    }

    /// Create from a half-open range [begin, end) of leaf cells.
    #[classmethod]
    fn from_range(_cls: &Bound<'_, PyType>, begin: &PyCellId, end: &PyCellId) -> Self {
        PyCellUnion(s2::CellUnion::from_range(begin.0, end.0))
    }

    /// A cell union covering the whole sphere (the six face cells).
    #[classmethod]
    fn whole_sphere(_cls: &Bound<'_, PyType>) -> Self {
        PyCellUnion(s2::CellUnion::whole_sphere())
    }

    /// Create from an inclusive range [min_id, max_id] of leaf cells.
    #[classmethod]
    fn from_min_max(_cls: &Bound<'_, PyType>, min_id: &PyCellId, max_id: &PyCellId) -> Self {
        PyCellUnion(s2::CellUnion::from_min_max(min_id.0, max_id.0))
    }

    /// Number of cells in this union.
    fn num_cells(&self) -> usize {
        self.0.num_cells()
    }

    /// The list of CellIds.
    fn cell_ids(&self) -> Vec<PyCellId> {
        self.0.cell_ids().iter().map(|c| PyCellId(*c)).collect()
    }

    /// Whether this union is valid (sorted, non-overlapping).
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Whether this union is fully normalized.
    fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    /// Normalize in place (sort, deduplicate, merge siblings).
    fn normalize(&mut self) {
        self.0.normalize();
    }

    /// Whether this union contains the given CellId.
    fn contains_cell_id(&self, id: &PyCellId) -> bool {
        self.0.contains_cell_id(id.0)
    }

    /// Whether this union contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this union contains the other union.
    fn contains_union(&self, other: &PyCellUnion) -> bool {
        self.0.contains_union(&other.0)
    }

    /// Whether this union intersects the given CellId.
    fn intersects_cell_id(&self, id: &PyCellId) -> bool {
        self.0.intersects_cell_id(id.0)
    }

    /// Whether this union intersects the other union.
    fn intersects_union(&self, other: &PyCellUnion) -> bool {
        self.0.intersects_union(&other.0)
    }

    /// Number of leaf cells covered by this union.
    fn leaf_cells_covered(&self) -> i64 {
        self.0.leaf_cells_covered()
    }

    /// Replace large cells with smaller cells respecting min_level and level_mod.
    fn denormalize(&self, min_level: u8, level_mod: u8) -> Self {
        PyCellUnion(self.0.denormalize(min_level.into(), level_mod))
    }

    /// Whether this union contains no cells.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The union of this cell union with another (normalized).
    fn union(&self, other: &PyCellUnion) -> Self {
        PyCellUnion(self.0.union(&other.0))
    }

    /// The intersection of this cell union with another (normalized).
    fn intersection(&self, other: &PyCellUnion) -> Self {
        PyCellUnion(self.0.intersection(&other.0))
    }

    /// The intersection of this cell union with a single cell.
    fn intersection_with_cell_id(&self, id: &PyCellId) -> Self {
        PyCellUnion(self.0.intersection_with_cell_id(id.0))
    }

    /// This cell union minus another (set difference).
    fn difference(&self, other: &PyCellUnion) -> Self {
        PyCellUnion(self.0.difference(&other.0))
    }

    /// Expand the union to include all neighbors at `expand_level` (in place).
    fn expand_at_level(&mut self, expand_level: u8) {
        self.0.expand_at_level(expand_level.into());
    }

    /// Expand the union so every point within `min_radius` is covered (in place).
    ///
    /// `max_level_diff` bounds how many levels finer than the existing cells the
    /// expansion may go.
    fn expand_by_radius(&mut self, min_radius: &PyAngle, max_level_diff: u8) {
        self.0.expand_by_radius(min_radius.0, max_level_diff);
    }

    /// Area of the union in steradians, using each cell's average-area metric.
    fn average_based_area(&self) -> f64 {
        self.0.average_based_area()
    }

    /// Approximate area of the union in steradians (fast, small error).
    fn approx_area(&self) -> f64 {
        self.0.approx_area()
    }

    /// Exact area of the union in steradians.
    fn exact_area(&self) -> f64 {
        self.0.exact_area()
    }

    fn __len__(&self) -> usize {
        self.0.num_cells()
    }

    fn __getitem__(&self, i: isize) -> PyResult<PyCellId> {
        let ids = self.0.cell_ids();
        let n = ids.len() as isize;
        let idx = if i < 0 { i + n } else { i };
        if idx < 0 || idx >= n {
            Err(pyo3::exceptions::PyIndexError::new_err(
                "index out of range",
            ))
        } else {
            Ok(PyCellId(ids[idx as usize]))
        }
    }

    fn __contains__(&self, id: &PyCellId) -> bool {
        self.0.contains_cell_id(id.0)
    }

    fn __iter__(&self) -> PyCellUnionIter {
        PyCellUnionIter {
            ids: self.0.cell_ids().to_vec(),
            idx: 0,
        }
    }

    fn __eq__(&self, other: &PyCellUnion) -> bool {
        self.0 == other.0
    }

    fn __repr__(&self) -> String {
        let n = self.0.num_cells();
        if n <= 5 {
            let tokens: Vec<String> = self
                .0
                .cell_ids()
                .iter()
                .map(|c| c.to_debug_string())
                .collect();
            format!("CellUnion([{}])", tokens.join(", "))
        } else {
            format!("CellUnion({} cells)", n)
        }
    }
}

#[pyclass]
struct PyCellUnionIter {
    ids: Vec<s2::CellId>,
    idx: usize,
}

#[pymethods]
impl PyCellUnionIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<PyCellId> {
        if self.idx < self.ids.len() {
            let id = self.ids[self.idx];
            self.idx += 1;
            Some(PyCellId(id))
        } else {
            None
        }
    }
}
