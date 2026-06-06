// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Specialized spatial indexes: a point index (points + integer labels) and a
//! cell index (cell ranges + integer labels), each with nearest-neighbour
//! queries. Labels are `i32`, matching the core `CellIndex` model.

use wasm_bindgen::prelude::*;

use crate::cell_id::CellId;
use crate::point::Point;

fn chord_max_distance(max_distance_radians: f64) -> s2rst::s1::ChordAngle {
    if max_distance_radians.is_finite() {
        s2rst::s1::ChordAngle::from_radians(max_distance_radians)
    } else {
        s2rst::s1::ChordAngle::INFINITY
    }
}

// ───────────────────────── Point index ─────────────────────────

/// A spatial index of points, each carrying an `i32` label.
#[wasm_bindgen]
pub struct PointIndex(s2rst::s2::point_index::S2PointIndex<i32>);

impl Default for PointIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl PointIndex {
    /// Create a new, empty point index.
    #[wasm_bindgen(constructor)]
    pub fn new() -> PointIndex {
        PointIndex(s2rst::s2::point_index::S2PointIndex::new())
    }

    /// Number of indexed points.
    #[wasm_bindgen(js_name = "numPoints")]
    pub fn num_points(&self) -> usize {
        self.0.num_points()
    }

    /// Add a point with the given `i32` label.
    pub fn add(&mut self, point: &Point, label: i32) {
        self.0.add(point.0, label);
    }

    /// Up to `maxResults` closest indexed points to `target` (`maxResults <= 0`
    /// = no limit), within `maxDistanceRadians` (`Infinity` = no limit).
    #[wasm_bindgen(js_name = "closestPoints")]
    pub fn closest_points(
        &self,
        target: &Point,
        max_results: i32,
        max_distance_radians: f64,
    ) -> Vec<PointResult> {
        let mut opts = s2rst::s2::closest_point_query::Options::default();
        if max_results > 0 {
            opts.max_results = max_results;
        }
        opts.max_distance = chord_max_distance(max_distance_radians);
        let query = s2rst::s2::closest_point_query::ClosestPointQuery::new(&self.0, opts);
        let mut tgt = s2rst::s2::closest_point_query::PointTarget::new(target.0);
        query
            .find_closest_points(&mut tgt)
            .into_iter()
            .map(PointResult::from_core)
            .collect()
    }
}

/// A result from a point-index query: an indexed point, its label, and distance.
#[wasm_bindgen]
pub struct PointResult {
    distance_radians: f64,
    label: i32,
    point: s2rst::s2::Point,
}

#[wasm_bindgen]
impl PointResult {
    /// Distance from the target to this point, in radians.
    #[wasm_bindgen(getter, js_name = "distanceRadians")]
    pub fn distance_radians(&self) -> f64 {
        self.distance_radians
    }

    /// The label associated with this point.
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> i32 {
        self.label
    }

    /// The indexed point.
    #[wasm_bindgen(getter)]
    pub fn point(&self) -> Point {
        Point(self.point)
    }
}

impl PointResult {
    fn from_core(r: s2rst::s2::closest_point_query::Result<i32>) -> PointResult {
        PointResult {
            distance_radians: r.distance.to_angle().radians(),
            label: r.data,
            point: r.point,
        }
    }
}

// ───────────────────────── Cell index ─────────────────────────

/// A spatial index of `CellId`s, each carrying an `i32` label. Call `build()`
/// before querying (queries auto-build, but explicit building is cheaper when
/// reused).
#[wasm_bindgen]
pub struct CellIndex(s2rst::s2::cell_index::CellIndex);

impl Default for CellIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl CellIndex {
    /// Create a new, empty cell index.
    #[wasm_bindgen(constructor)]
    pub fn new() -> CellIndex {
        CellIndex(s2rst::s2::cell_index::CellIndex::new())
    }

    /// Add a `CellId` with the given `i32` label.
    pub fn add(&mut self, id: &CellId, label: i32) {
        self.0.add(id.0, label);
    }

    /// Build the index (forces construction; queries also build on demand).
    pub fn build(&mut self) {
        self.0.build();
    }

    /// Up to `maxResults` closest indexed cells to `target` (`maxResults <= 0`
    /// = no limit), within `maxDistanceRadians` (`Infinity` = no limit).
    #[wasm_bindgen(js_name = "closestCells")]
    pub fn closest_cells(
        &mut self,
        target: &Point,
        max_results: i32,
        max_distance_radians: f64,
    ) -> Vec<CellResult> {
        self.0.build();
        let mut opts = s2rst::s2::closest_cell_query::Options::default();
        if max_results > 0 {
            opts.max_results = max_results;
        }
        opts.max_distance = chord_max_distance(max_distance_radians);
        let query = s2rst::s2::closest_cell_query::ClosestCellQuery::new(&self.0, opts);
        let mut tgt = s2rst::s2::closest_cell_query::PointTarget::new(target.0);
        query
            .find_closest_cells(&mut tgt)
            .into_iter()
            .map(CellResult::from_core)
            .collect()
    }
}

/// A result from a cell-index query: a `CellId`, its label, and distance.
#[wasm_bindgen]
pub struct CellResult {
    distance_radians: f64,
    label: i32,
    cell_id: s2rst::s2::CellId,
}

#[wasm_bindgen]
impl CellResult {
    /// Distance from the target to this cell, in radians.
    #[wasm_bindgen(getter, js_name = "distanceRadians")]
    pub fn distance_radians(&self) -> f64 {
        self.distance_radians
    }

    /// The label associated with this cell.
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> i32 {
        self.label
    }

    /// The `CellId`.
    #[wasm_bindgen(getter, js_name = "cellId")]
    pub fn cell_id(&self) -> CellId {
        CellId(self.cell_id)
    }
}

impl CellResult {
    fn from_core(r: s2rst::s2::closest_cell_query::Result) -> CellResult {
        CellResult {
            distance_radians: r.distance.to_angle().radians(),
            label: r.label,
            cell_id: r.cell_id,
        }
    }
}
