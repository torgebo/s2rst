// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Bindings for three S2 "long-tail" features:
//!
//! - [`PyCellIndex`] — an index of `(CellId, label)` pairs ([`CellIndex`]).
//! - [`PyS2Fractal`] — a generator of fractal loops ([`S2Fractal`]).
//! - [`PyValidationQuery`] — geometry validation over a `ShapeIndex`
//!   ([`S2ValidQuery`]).

use pyo3::prelude::*;

use s2rst::s2::cell_index::{CellIndex, CellIndexContentsIterator, CellIndexRangeIterator};
use s2rst::s2::fractal::S2Fractal;
use s2rst::s2::validation_query::S2ValidQuery;

use crate::angle::PyAngle;
use crate::cells::{PyCellId, PyCellUnion};
use crate::geometry::PyLoop;
use crate::index::PyShapeIndex;
use crate::points::PyMatrix3x3;
use crate::s2point::PyS2Point;

// ---------------------------------------------------------------------------
// CellIndex
// ---------------------------------------------------------------------------

/// An index of `(CellId, label)` pairs supporting efficient range lookups.
///
/// Add pairs with `add` / `add_cell_union`, call `build` once, then enumerate
/// the indexed pairs by iterating (each item is a `(CellId, label)` tuple) or
/// via `cells()`. Labels must be non-negative.
#[pyclass(name = "CellIndex", module = "s2rst")]
pub struct PyCellIndex(pub(crate) CellIndex);

impl PyCellIndex {
    /// Walk the (deduplicated) `(CellId, label)` pairs in the built index.
    ///
    /// Mirrors the canonical traversal: step the range iterator over every
    /// leaf-cell range and, for each, drain the contents iterator. The
    /// contents iterator suppresses duplicates as long as the ranges are
    /// visited in increasing order, so each pair is reported exactly once.
    fn collect_pairs(&self) -> Vec<(s2rst::s2::CellId, i32)> {
        let mut range_it = CellIndexRangeIterator::new(&self.0);
        let mut contents_it = CellIndexContentsIterator::new(&self.0);
        let mut pairs = Vec::new();
        range_it.begin();
        while !range_it.done() {
            contents_it.start_union(&range_it);
            while !contents_it.done() {
                pairs.push((contents_it.cell_id(), contents_it.label()));
                contents_it.next();
            }
            range_it.next();
        }
        pairs
    }
}

#[pymethods]
impl PyCellIndex {
    /// Create a new, empty cell index.
    #[new]
    fn new() -> Self {
        PyCellIndex(CellIndex::new())
    }

    /// Add a `(CellId, label)` pair. `label` must be non-negative.
    fn add(&mut self, cell_id: &PyCellId, label: i32) -> PyResult<()> {
        if label < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "label must be non-negative",
            ));
        }
        self.0.add(cell_id.0, label);
        Ok(())
    }

    /// Add every cell of a `CellUnion` with the same `label`.
    /// `label` must be non-negative.
    fn add_cell_union(&mut self, cu: &PyCellUnion, label: i32) -> PyResult<()> {
        if label < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "label must be non-negative",
            ));
        }
        self.0.add_cell_union(&cu.0, label);
        Ok(())
    }

    /// Build the index. Call once after all pairs have been added, before
    /// iterating or calling `cells()`.
    fn build(&mut self) {
        self.0.build();
    }

    /// The indexed `(CellId, label)` pairs as a list. Call after `build()`.
    fn cells(&self) -> Vec<(PyCellId, i32)> {
        self.collect_pairs()
            .into_iter()
            .map(|(c, l)| (PyCellId(c), l))
            .collect()
    }

    /// The number of indexed `(CellId, label)` pairs. Call after `build()`.
    fn __len__(&self) -> usize {
        self.collect_pairs().len()
    }

    fn __iter__(&self) -> PyCellIndexIter {
        PyCellIndexIter {
            pairs: self.collect_pairs(),
            idx: 0,
        }
    }

    fn __repr__(&self) -> String {
        format!("CellIndex({} cells)", self.collect_pairs().len())
    }
}

/// Snapshot iterator over `(CellId, label)` pairs. Constructed by
/// `CellIndex.__iter__`.
#[pyclass]
struct PyCellIndexIter {
    pairs: Vec<(s2rst::s2::CellId, i32)>,
    idx: usize,
}

#[pymethods]
impl PyCellIndexIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(PyCellId, i32)> {
        if self.idx < self.pairs.len() {
            let (c, l) = self.pairs[self.idx];
            self.idx += 1;
            Some((PyCellId(c), l))
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// S2Fractal
// ---------------------------------------------------------------------------

/// Generates fractal loops on the unit sphere — a testing utility for complex,
/// realistic geometry (e.g. Koch-snowflake coastlines).
///
/// Construct with a random `seed`, configure with `set_max_level`,
/// `set_min_level`, and `set_fractal_dimension`, then build a loop with
/// `make_loop_at` (or `make_loop` with an explicit frame).
#[pyclass(name = "S2Fractal", module = "s2rst")]
pub struct PyS2Fractal(S2Fractal);

#[pymethods]
impl PyS2Fractal {
    /// Create a fractal generator seeded with `seed`.
    #[new]
    fn new(seed: u64) -> Self {
        PyS2Fractal(S2Fractal::new(seed))
    }

    /// Set the maximum subdivision level (vertices at a level = `3 * 4^level`).
    /// Must be non-negative.
    fn set_max_level(&mut self, max_level: i32) -> PyResult<()> {
        if max_level < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "max_level must be non-negative",
            ));
        }
        self.0.set_max_level(max_level);
        Ok(())
    }

    /// The maximum subdivision level.
    fn max_level(&self) -> i32 {
        self.0.max_level()
    }

    /// Set the minimum subdivision level. `-1` (default) makes the minimum
    /// equal the maximum (uniform subdivision); a smaller value enables
    /// variable detail. Must be `>= -1`.
    fn set_min_level(&mut self, min_level: i32) -> PyResult<()> {
        if min_level < -1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "min_level must be >= -1",
            ));
        }
        self.0.set_min_level(min_level);
        Ok(())
    }

    /// The minimum subdivision level.
    fn min_level(&self) -> i32 {
        self.0.min_level()
    }

    /// Set the fractal dimension, in `[1.0, 2.0)` (1.0 = plain triangle,
    /// ~1.26 = Koch snowflake, approaching 2.0 = nearly space-filling).
    fn set_fractal_dimension(&mut self, dimension: f64) -> PyResult<()> {
        if !(1.0..2.0).contains(&dimension) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "dimension must be in [1.0, 2.0)",
            ));
        }
        self.0.set_fractal_dimension(dimension);
        Ok(())
    }

    /// The fractal dimension.
    fn fractal_dimension(&self) -> f64 {
        self.0.fractal_dimension()
    }

    /// Minimum boundary distance from the center, as a factor of the radius.
    fn min_radius_factor(&self) -> f64 {
        self.0.min_radius_factor()
    }

    /// Maximum boundary distance from the center, as a factor of the radius.
    fn max_radius_factor(&self) -> f64 {
        self.0.max_radius_factor()
    }

    /// Generate a fractal loop centered at `center` with the given
    /// `nominal_radius`.
    fn make_loop_at(&mut self, center: &PyS2Point, nominal_radius: &PyAngle) -> PyLoop {
        PyLoop(self.0.make_loop_at(center.0, nominal_radius.0))
    }

    /// Generate a fractal loop centered on the z-axis of `frame` with the
    /// given `nominal_radius`.
    fn make_loop(&mut self, frame: &PyMatrix3x3, nominal_radius: &PyAngle) -> PyLoop {
        PyLoop(self.0.make_loop(&frame.0, nominal_radius.0))
    }

    fn __repr__(&self) -> String {
        format!(
            "S2Fractal(max_level={}, min_level={}, dimension={})",
            self.0.max_level(),
            self.0.min_level(),
            self.0.fractal_dimension()
        )
    }
}

// ---------------------------------------------------------------------------
// ValidationQuery
// ---------------------------------------------------------------------------

/// Validates the geometry in a `ShapeIndex` (the least-strict S2 validation,
/// compatible with boolean operations).
///
/// Checks that points are unit-length and finite, that there are no antipodal
/// edges, that polygon chains are closed/connected/interior-on-left, that
/// polygon interiors are disjoint from other geometry, and that there are no
/// duplicate polygon edges.
#[pyclass(name = "ValidationQuery", module = "s2rst")]
pub struct PyValidationQuery(S2ValidQuery);

#[pymethods]
impl PyValidationQuery {
    /// Create a validation query with default options.
    #[new]
    fn new() -> Self {
        PyValidationQuery(S2ValidQuery::new())
    }

    /// Validate `index`. Returns `None` if the geometry is valid, otherwise a
    /// string describing the first validation failure found.
    fn validate(&self, index: &PyShapeIndex) -> Option<String> {
        self.0.validate(&index.0).err().map(|e| e.to_string())
    }

    fn __repr__(&self) -> String {
        "ValidationQuery(...)".to_string()
    }
}
