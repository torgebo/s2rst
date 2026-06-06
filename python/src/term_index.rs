// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Python bindings for `RegionTermIndexer` and `RegionSharder` — turning
//! spatial regions into string index/query terms, and assigning regions to
//! shards based on spatial overlap.

use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;

use s2rst::s2::region_sharder::RegionSharder;
use s2rst::s2::region_term_indexer::{Options, RegionTermIndexer};
use s2rst::s2::{CellUnion, Level, Region};

use crate::cells::PyCellUnion;
use crate::regions::{PyCap, PyRect};
use crate::s2point::PyS2Point;

/// Convert a Python cell-level integer into a core `Level`, mapping the
/// out-of-range case to a `ValueError` instead of panicking.
fn to_level(value: u8, name: &str) -> PyResult<Level> {
    Level::try_new(value)
        .ok_or_else(|| PyValueError::new_err(format!("{name} must be in 0..=30, got {value}")))
}

/// Dispatch a closure over the supported concrete region types (`Cap`, `Rect`,
/// `CellUnion`), borrowing each as `&dyn Region`. Mirrors `coverer.rs`.
fn with_region<R>(
    region: &Bound<'_, PyAny>,
    method: &str,
    f: impl FnOnce(&dyn Region) -> R,
) -> PyResult<R> {
    macro_rules! try_region {
        ($ty:ty) => {
            if let Ok(r) = region.downcast::<$ty>() {
                let r = r.borrow();
                return Ok(f(&r.0));
            }
        };
    }
    try_region!(PyCap);
    try_region!(PyRect);
    try_region!(PyCellUnion);
    Err(PyTypeError::new_err(format!(
        "{method}() expects a Cap, Rect, or CellUnion"
    )))
}

/// Converts spatial regions into string terms for database indexing and
/// querying.
///
/// Index a document's region with `get_index_terms` (or
/// `get_index_terms_for_point`), store each returned term in an inverted index,
/// then look up the `get_query_terms` of a query region and union the matches.
#[pyclass(name = "RegionTermIndexer", module = "s2rst")]
pub struct PyRegionTermIndexer(RegionTermIndexer);

#[pymethods]
impl PyRegionTermIndexer {
    /// Create a region term indexer. All settings are keyword-only and default
    /// to the S2 norms: `max_cells=8`, `min_level=4`, `max_level=16`,
    /// `level_mod=1`, `index_contains_points_only=False`,
    /// `optimize_for_space=False`, `marker_character='$'`.
    #[new]
    #[pyo3(signature = (
        *,
        max_cells = 8,
        min_level = 4,
        max_level = 16,
        level_mod = 1,
        index_contains_points_only = false,
        optimize_for_space = false,
        marker_character = '$',
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        max_cells: usize,
        min_level: u8,
        max_level: u8,
        level_mod: usize,
        index_contains_points_only: bool,
        optimize_for_space: bool,
        marker_character: char,
    ) -> PyResult<Self> {
        let options = Options {
            max_cells,
            min_level: to_level(min_level, "min_level")?,
            max_level: to_level(max_level, "max_level")?,
            level_mod,
            index_contains_points_only,
            optimize_for_space,
            marker_character,
        };
        Ok(PyRegionTermIndexer(RegionTermIndexer::with_options(
            options,
        )))
    }

    #[getter]
    fn max_cells(&self) -> usize {
        self.0.options().max_cells
    }

    #[getter]
    fn min_level(&self) -> u8 {
        u8::from(self.0.options().min_level)
    }

    #[getter]
    fn max_level(&self) -> u8 {
        u8::from(self.0.options().max_level)
    }

    #[getter]
    fn level_mod(&self) -> usize {
        self.0.options().level_mod
    }

    #[getter]
    fn index_contains_points_only(&self) -> bool {
        self.0.options().index_contains_points_only
    }

    #[getter]
    fn optimize_for_space(&self) -> bool {
        self.0.options().optimize_for_space
    }

    #[getter]
    fn marker_character(&self) -> char {
        self.0.options().marker_character
    }

    /// Generate index terms for a region (`Cap`, `Rect`, or `CellUnion`).
    ///
    /// Store each returned term in the database for the document.
    fn get_index_terms(&self, region: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
        with_region(region, "get_index_terms", |r| self.0.get_index_terms(r))
    }

    /// Generate index terms for a single point.
    fn get_index_terms_for_point(&self, point: &PyS2Point) -> Vec<String> {
        self.0.get_index_terms_for_point(point.0)
    }

    /// Generate query terms for a region (`Cap`, `Rect`, or `CellUnion`).
    ///
    /// Look up each returned term in the database and union the results.
    fn get_query_terms(&self, region: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
        with_region(region, "get_query_terms", |r| self.0.get_query_terms(r))
    }

    /// Generate query terms for a single point.
    fn get_query_terms_for_point(&self, point: &PyS2Point) -> Vec<String> {
        self.0.get_query_terms_for_point(point.0)
    }

    fn __repr__(&self) -> String {
        let o = self.0.options();
        format!(
            "RegionTermIndexer(max_cells={}, min_level={}, max_level={}, level_mod={}, \
             index_contains_points_only={}, optimize_for_space={}, marker_character={:?})",
            o.max_cells,
            u8::from(o.min_level),
            u8::from(o.max_level),
            o.level_mod,
            if o.index_contains_points_only {
                "True"
            } else {
                "False"
            },
            if o.optimize_for_space {
                "True"
            } else {
                "False"
            },
            o.marker_character,
        )
    }
}

/// Assigns regions to shards based on spatial overlap.
///
/// Shards are defined as a list of `CellUnion`s; each shard is identified by its
/// 0-based index in that list. Use `get_intersecting_shards` to find every shard
/// a region touches, or `get_most_intersecting_shard` for the single best match.
#[pyclass(name = "RegionSharder", module = "s2rst")]
pub struct PyRegionSharder(RegionSharder);

#[pymethods]
impl PyRegionSharder {
    /// Create a sharder from a list of shard cell unions. Each shard is
    /// identified by its index in the list.
    #[new]
    fn new(shards: Vec<PyCellUnion>) -> Self {
        let shards: Vec<CellUnion> = shards.into_iter().map(|s| s.0).collect();
        PyRegionSharder(RegionSharder::new(&shards))
    }

    /// Return the index of the shard with the most overlap with `region`
    /// (a `Cap`, `Rect`, or `CellUnion`), or `default_shard` if none overlap.
    fn get_most_intersecting_shard(
        &self,
        region: &Bound<'_, PyAny>,
        default_shard: i32,
    ) -> PyResult<i32> {
        with_region(region, "get_most_intersecting_shard", |r| {
            self.0.get_most_intersecting_shard(r, default_shard)
        })
    }

    /// Return the indices of all shards that intersect `region` (a `Cap`,
    /// `Rect`, or `CellUnion`).
    fn get_intersecting_shards(&self, region: &Bound<'_, PyAny>) -> PyResult<Vec<i32>> {
        with_region(region, "get_intersecting_shards", |r| {
            self.0.get_intersecting_shards(r)
        })
    }
}
