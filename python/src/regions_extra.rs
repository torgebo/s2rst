// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for the remaining region types: the single-point region and the
//! region algebra (`RegionUnion`, `RegionIntersection`).
//!
//! Each of these implements the core `Region` trait, so they all expose the
//! same bounding/containment surface (`cap_bound`, `rect_bound`,
//! `contains_point`) as `Cap` and `Rect`.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use s2rst::s2::Region;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;

use crate::regions::{PyCap, PyRect};
use crate::s2point::PyS2Point;

// ---------------------------------------------------------------------------
// Helper: turn an arbitrary Python region wrapper into a `Box<dyn Region>`.
// ---------------------------------------------------------------------------

/// Accepts any of the concrete exposed region types (`Cap`, `Rect`,
/// `CellUnion`, `PointRegion`) and boxes a clone of the inner core value as a
/// trait object. Used by `RegionUnion`/`RegionIntersection` to ingest regions
/// from Python.
fn region_from_pyany(obj: &Bound<'_, PyAny>) -> PyResult<Box<dyn Region>> {
    if let Ok(cap) = obj.extract::<PyCap>() {
        return Ok(Box::new(cap.0));
    }
    if let Ok(rect) = obj.extract::<PyRect>() {
        return Ok(Box::new(rect.0));
    }
    if let Ok(cu) = obj.extract::<crate::cells::PyCellUnion>() {
        return Ok(Box::new(cu.0.clone()));
    }
    if let Ok(pr) = obj.extract::<PyPointRegion>() {
        return Ok(Box::new(pr.0));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a region: Cap, Rect, CellUnion, or PointRegion",
    ))
}

// ---------------------------------------------------------------------------
// PointRegion
// ---------------------------------------------------------------------------

/// A region that contains a single point on the unit sphere.
///
/// Mainly useful for completeness and uniform handling alongside the other
/// region types: it implements the same `cap_bound`/`rect_bound`/
/// `contains_point` surface.
#[pyclass(name = "PointRegion", module = "s2rst")]
#[derive(Clone, Copy)]
pub struct PyPointRegion(pub(crate) PointRegion);

#[pymethods]
impl PyPointRegion {
    fn __copy__(&self) -> Self {
        Self(self.0)
    }

    fn __deepcopy__(&self, _memo: &Bound<'_, PyDict>) -> Self {
        Self(self.0)
    }

    /// Create a region containing the given point.
    #[new]
    fn new(point: &PyS2Point) -> Self {
        PyPointRegion(PointRegion::new(point.0))
    }

    /// The contained point.
    fn point(&self) -> PyS2Point {
        PyS2Point(self.0.point())
    }

    // --- Region bounds ---

    /// A bounding spherical cap (a zero-radius cap at the point).
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle (a single lat/lng point).
    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this region.
    fn cell_union_bound(&self) -> Vec<crate::cells::PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(crate::cells::PyCellId)
            .collect()
    }

    /// Whether this region contains the given point (exact equality).
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(&p.0)
    }

    fn __eq__(&self, other: &PyPointRegion) -> bool {
        self.0 == other.0
    }

    fn __repr__(&self) -> String {
        let p = self.0.point();
        format!("PointRegion(({:.6}, {:.6}, {:.6}))", p.x(), p.y(), p.z())
    }
}

// ---------------------------------------------------------------------------
// RegionUnion
// ---------------------------------------------------------------------------

/// A union of possibly overlapping regions.
///
/// Add regions with `add()`; the union's bounds and containment combine those
/// of its members. A point is contained if it lies in *any* member region.
#[pyclass(name = "RegionUnion", module = "s2rst", unsendable)]
pub struct PyRegionUnion(RegionUnion);

#[pymethods]
impl PyRegionUnion {
    /// Create an empty region union.
    #[new]
    fn new() -> Self {
        PyRegionUnion(RegionUnion::new())
    }

    /// Add a region (`Cap`, `Rect`, `CellUnion`, or `PointRegion`) to the
    /// union.
    fn add(&mut self, region: &Bound<'_, PyAny>) -> PyResult<()> {
        self.0.add(region_from_pyany(region)?);
        Ok(())
    }

    /// The number of regions in the union.
    fn __len__(&self) -> usize {
        self.0.len()
    }

    /// Whether the union contains no regions.
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // --- Region bounds ---

    /// A bounding spherical cap for the whole union.
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle for the whole union.
    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this union.
    fn cell_union_bound(&self) -> Vec<crate::cells::PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(crate::cells::PyCellId)
            .collect()
    }

    /// Whether any member region contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(&p.0)
    }

    fn __repr__(&self) -> String {
        format!("RegionUnion({} regions)", self.0.len())
    }
}

// ---------------------------------------------------------------------------
// RegionIntersection
// ---------------------------------------------------------------------------

/// The intersection of a set of regions.
///
/// An intersection of no regions covers the entire sphere. A point is
/// contained only if it lies in *every* member region. Construct from an
/// optional list of regions (`Cap`, `Rect`, `CellUnion`, or `PointRegion`).
#[pyclass(name = "RegionIntersection", module = "s2rst", unsendable)]
pub struct PyRegionIntersection(RegionIntersection);

#[pymethods]
impl PyRegionIntersection {
    /// Create an intersection from an optional list of regions. With no
    /// argument (or an empty list) the intersection covers the whole sphere.
    #[new]
    #[pyo3(signature = (regions = None))]
    fn new(regions: Option<Vec<Bound<'_, PyAny>>>) -> PyResult<Self> {
        match regions {
            None => Ok(PyRegionIntersection(RegionIntersection::new())),
            Some(list) => {
                let mut boxed: Vec<Box<dyn Region>> = Vec::with_capacity(list.len());
                for obj in &list {
                    boxed.push(region_from_pyany(obj)?);
                }
                Ok(PyRegionIntersection(RegionIntersection::from_regions(
                    boxed,
                )))
            }
        }
    }

    /// The number of regions in this intersection.
    fn __len__(&self) -> usize {
        self.0.num_regions()
    }

    // --- Region bounds ---

    /// A bounding spherical cap for the intersection.
    fn cap_bound(&self) -> PyCap {
        PyCap(self.0.cap_bound())
    }

    /// A bounding latitude-longitude rectangle for the intersection.
    fn rect_bound(&self) -> PyRect {
        PyRect(self.0.rect_bound())
    }

    /// A small set of CellIds covering this intersection.
    fn cell_union_bound(&self) -> Vec<crate::cells::PyCellId> {
        self.0
            .cell_union_bound()
            .into_iter()
            .map(crate::cells::PyCellId)
            .collect()
    }

    /// Whether every member region contains the given point.
    fn contains_point(&self, p: &PyS2Point) -> bool {
        self.0.contains_point(&p.0)
    }

    fn __repr__(&self) -> String {
        format!("RegionIntersection({} regions)", self.0.num_regions())
    }
}
