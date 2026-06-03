// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::cap::Cap;
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

#[wasm_bindgen]
impl RegionCoverer {
    /// Create a new region coverer with default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> RegionCoverer {
        RegionCoverer(s2rst::s2::region_coverer::RegionCoverer::new())
    }

    /// Set the minimum level.
    #[wasm_bindgen(js_name = "setMinLevel")]
    pub fn set_min_level(self, level: u8) -> RegionCoverer {
        RegionCoverer(self.0.min_level(level))
    }

    /// Set the maximum level.
    #[wasm_bindgen(js_name = "setMaxLevel")]
    pub fn set_max_level(self, level: u8) -> RegionCoverer {
        RegionCoverer(self.0.max_level(level))
    }

    /// Set the level modulus.
    #[wasm_bindgen(js_name = "setLevelMod")]
    pub fn set_level_mod(self, modulo: u8) -> RegionCoverer {
        RegionCoverer(self.0.level_mod(modulo))
    }

    /// Set the maximum number of cells.
    #[wasm_bindgen(js_name = "setMaxCells")]
    pub fn set_max_cells(self, cells: usize) -> RegionCoverer {
        RegionCoverer(self.0.max_cells(cells))
    }

    /// Compute a covering for a `Cap`.
    #[wasm_bindgen(js_name = "coveringCap")]
    pub fn covering_cap(&self, cap: &Cap) -> CellUnion {
        CellUnion(self.0.covering(&cap.0))
    }

    /// Compute a covering for a `Rect`.
    #[wasm_bindgen(js_name = "coveringRect")]
    pub fn covering_rect(&self, rect: &Rect) -> CellUnion {
        CellUnion(self.0.covering(&rect.0))
    }

    /// Compute a covering for an `S2Loop`.
    #[wasm_bindgen(js_name = "coveringLoop")]
    pub fn covering_loop(&self, loop_: &Loop) -> CellUnion {
        CellUnion(self.0.covering(&loop_.0))
    }

    /// Compute a covering for a `Polygon`.
    #[wasm_bindgen(js_name = "coveringPolygon")]
    pub fn covering_polygon(&self, polygon: &Polygon) -> CellUnion {
        CellUnion(self.0.covering(&polygon.0))
    }

    /// Compute an interior covering for a `Cap`.
    #[wasm_bindgen(js_name = "interiorCoveringCap")]
    pub fn interior_covering_cap(&self, cap: &Cap) -> CellUnion {
        CellUnion(self.0.interior_covering(&cap.0))
    }

    /// Compute an interior covering for a `Rect`.
    #[wasm_bindgen(js_name = "interiorCoveringRect")]
    pub fn interior_covering_rect(&self, rect: &Rect) -> CellUnion {
        CellUnion(self.0.interior_covering(&rect.0))
    }

    /// Compute an interior covering for a `Polygon`.
    #[wasm_bindgen(js_name = "interiorCoveringPolygon")]
    pub fn interior_covering_polygon(&self, polygon: &Polygon) -> CellUnion {
        CellUnion(self.0.interior_covering(&polygon.0))
    }
}
