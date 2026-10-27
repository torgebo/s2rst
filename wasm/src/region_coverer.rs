// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::bounded::{check_level_order, level_arg, level_mod_arg};
use crate::cap::Cap;
use crate::cell_id::CellId;
use crate::cell_union::CellUnion;
use crate::polygon::Polygon;
use crate::rect::Rect;
use crate::s2loop::Loop;

/// A region coverer — computes cell coverings of regions.
#[wasm_bindgen]
pub struct RegionCoverer(s2rst::s2::region_coverer::RegionCoverer);

impl Default for RegionCoverer {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionCoverer {
    /// An error unless the minimum level is at most the maximum level, which the core's
    /// coverings require (spec 009, B12).
    fn check_levels(&self) -> Result<(), JsValue> {
        check_level_order(self.0.min_level, self.0.max_level)
    }
}

#[wasm_bindgen]
impl RegionCoverer {
    /// Create a new region coverer with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> RegionCoverer {
        RegionCoverer(s2rst::s2::region_coverer::RegionCoverer::new())
    }

    /// Set the minimum level.
    ///
    /// Throws unless `level` is an integer in 0..=30. Like a successful call, a
    /// failed one consumes this object.
    #[wasm_bindgen(js_name = "setMinLevel")]
    pub fn set_min_level(self, level: f64) -> Result<RegionCoverer, JsValue> {
        Ok(RegionCoverer(self.0.min_level(level_arg(level)?)))
    }

    /// Set the maximum level.
    ///
    /// Throws unless `level` is an integer in 0..=30. Like a successful call, a
    /// failed one consumes this object.
    #[wasm_bindgen(js_name = "setMaxLevel")]
    pub fn set_max_level(self, level: f64) -> Result<RegionCoverer, JsValue> {
        Ok(RegionCoverer(self.0.max_level(level_arg(level)?)))
    }

    /// Set the level modulus.
    ///
    /// Throws unless `modulo` is an integer in 1..=3 (before spec 009, other values were
    /// silently clamped, and `NaN` became 1). Like a successful call, a failed one consumes this
    /// object.
    #[wasm_bindgen(js_name = "setLevelMod")]
    pub fn set_level_mod(self, modulo: f64) -> Result<RegionCoverer, JsValue> {
        Ok(RegionCoverer(self.0.level_mod(level_mod_arg(modulo)?)))
    }

    /// Set the maximum number of cells.
    #[wasm_bindgen(js_name = "setMaxCells")]
    pub fn set_max_cells(self, cells: usize) -> RegionCoverer {
        RegionCoverer(self.0.max_cells(cells))
    }

    /// Compute a covering for a `Cap`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "coveringCap")]
    pub fn covering_cap(&self, cap: &Cap) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.covering(&cap.0)))
    }

    /// Compute a covering for a `Rect`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "coveringRect")]
    pub fn covering_rect(&self, rect: &Rect) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.covering(&rect.0)))
    }

    /// Compute a covering for an `S2Loop`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "coveringLoop")]
    pub fn covering_loop(&self, loop_: &Loop) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.covering(&loop_.0)))
    }

    /// Compute a covering for a `Polygon`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "coveringPolygon")]
    pub fn covering_polygon(&self, polygon: &Polygon) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.covering(&polygon.0)))
    }

    /// Compute an interior covering for a `Cap`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "interiorCoveringCap")]
    pub fn interior_covering_cap(&self, cap: &Cap) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.interior_covering(&cap.0)))
    }

    /// Compute an interior covering for a `Rect`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "interiorCoveringRect")]
    pub fn interior_covering_rect(&self, rect: &Rect) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.interior_covering(&rect.0)))
    }

    /// Compute an interior covering for a `Polygon`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "interiorCoveringPolygon")]
    pub fn interior_covering_polygon(&self, polygon: &Polygon) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.interior_covering(&polygon.0)))
    }

    /// Compute an interior covering for an `S2Loop`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "interiorCoveringLoop")]
    pub fn interior_covering_loop(&self, loop_: &Loop) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.interior_covering(&loop_.0)))
    }

    /// Compute a covering for a `CellUnion`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "coveringCellUnion")]
    pub fn covering_cell_union(&self, cu: &CellUnion) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.covering(&cu.0)))
    }

    /// A fast (non-optimal) covering for a `Cap` — cheaper but less tight.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "fastCoveringCap")]
    pub fn fast_covering_cap(&self, cap: &Cap) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.fast_covering(&cap.0)))
    }

    /// A fast (non-optimal) covering for a `Rect`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "fastCoveringRect")]
    pub fn fast_covering_rect(&self, rect: &Rect) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.fast_covering(&rect.0)))
    }

    /// A fast (non-optimal) covering for a `Polygon`.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "fastCoveringPolygon")]
    pub fn fast_covering_polygon(&self, polygon: &Polygon) -> Result<CellUnion, JsValue> {
        self.check_levels()?;
        Ok(CellUnion(self.0.fast_covering(&polygon.0)))
    }

    /// Whether `covering` is canonical for this coverer's parameters.
    ///
    /// Throws unless the minimum level is at most the maximum level.
    #[wasm_bindgen(js_name = "isCanonical")]
    pub fn is_canonical(&self, covering: Vec<CellId>) -> Result<bool, JsValue> {
        self.check_levels()?;
        let ids: Vec<s2rst::s2::CellId> = covering.iter().map(|c| c.0).collect();
        Ok(self.0.is_canonical(&ids))
    }
}
