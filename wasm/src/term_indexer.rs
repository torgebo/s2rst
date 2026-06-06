// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Region term indexing for inverted-index containment search. Index a region
//! (or point) under the strings from `indexTerms*`, and at query time match
//! against `queryTerms*`; a non-empty intersection means the indexed region and
//! the query region may intersect / contain one another.

use wasm_bindgen::prelude::*;

use crate::cap::Cap;
use crate::cell_union::CellUnion;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::rect::Rect;

/// Generates index/query terms for region containment search.
#[wasm_bindgen]
pub struct RegionTermIndexer(s2rst::s2::region_term_indexer::RegionTermIndexer);

impl Default for RegionTermIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl RegionTermIndexer {
    /// Create with default options. Setters consume and return `self`.
    #[wasm_bindgen(constructor)]
    pub fn new() -> RegionTermIndexer {
        RegionTermIndexer(s2rst::s2::region_term_indexer::RegionTermIndexer::new())
    }

    /// Set the maximum number of cells per covering.
    #[wasm_bindgen(js_name = "setMaxCells")]
    pub fn set_max_cells(mut self, max_cells: usize) -> RegionTermIndexer {
        self.0.options_mut().max_cells = max_cells;
        self
    }

    /// Set the minimum cell level (0–30).
    #[wasm_bindgen(js_name = "setMinLevel")]
    pub fn set_min_level(mut self, level: u8) -> RegionTermIndexer {
        self.0.options_mut().min_level = s2rst::s2::Level::new(level);
        self
    }

    /// Set the maximum cell level (0–30).
    #[wasm_bindgen(js_name = "setMaxLevel")]
    pub fn set_max_level(mut self, level: u8) -> RegionTermIndexer {
        self.0.options_mut().max_level = s2rst::s2::Level::new(level);
        self
    }

    /// If true, only point-containment queries are supported (smaller index).
    #[wasm_bindgen(js_name = "setIndexContainsPointsOnly")]
    pub fn set_index_contains_points_only(mut self, value: bool) -> RegionTermIndexer {
        self.0.options_mut().index_contains_points_only = value;
        self
    }

    // -- Point terms --------------------------------------------------------

    /// Terms to index for a point.
    #[wasm_bindgen(js_name = "indexTermsForPoint")]
    pub fn index_terms_for_point(&self, point: &Point) -> Vec<String> {
        self.0.get_index_terms_for_point(point.0)
    }

    /// Terms to query for a point.
    #[wasm_bindgen(js_name = "queryTermsForPoint")]
    pub fn query_terms_for_point(&self, point: &Point) -> Vec<String> {
        self.0.get_query_terms_for_point(point.0)
    }

    // -- Covering terms -----------------------------------------------------

    /// Terms to index for a precomputed `CellUnion` covering.
    #[wasm_bindgen(js_name = "indexTermsForCovering")]
    pub fn index_terms_for_covering(&self, covering: &CellUnion) -> Vec<String> {
        self.0.get_index_terms_for_covering(&covering.0)
    }

    /// Terms to query for a precomputed `CellUnion` covering.
    #[wasm_bindgen(js_name = "queryTermsForCovering")]
    pub fn query_terms_for_covering(&self, covering: &CellUnion) -> Vec<String> {
        self.0.get_query_terms_for_covering(&covering.0)
    }

    // -- Typed region terms -------------------------------------------------

    /// Terms to index for a `Cap`.
    #[wasm_bindgen(js_name = "indexTermsForCap")]
    pub fn index_terms_for_cap(&self, cap: &Cap) -> Vec<String> {
        self.0.get_index_terms(&cap.0)
    }

    /// Terms to query for a `Cap`.
    #[wasm_bindgen(js_name = "queryTermsForCap")]
    pub fn query_terms_for_cap(&self, cap: &Cap) -> Vec<String> {
        self.0.get_query_terms(&cap.0)
    }

    /// Terms to index for a `Rect`.
    #[wasm_bindgen(js_name = "indexTermsForRect")]
    pub fn index_terms_for_rect(&self, rect: &Rect) -> Vec<String> {
        self.0.get_index_terms(&rect.0)
    }

    /// Terms to query for a `Rect`.
    #[wasm_bindgen(js_name = "queryTermsForRect")]
    pub fn query_terms_for_rect(&self, rect: &Rect) -> Vec<String> {
        self.0.get_query_terms(&rect.0)
    }

    /// Terms to index for a `Polygon`.
    #[wasm_bindgen(js_name = "indexTermsForPolygon")]
    pub fn index_terms_for_polygon(&self, polygon: &Polygon) -> Vec<String> {
        self.0.get_index_terms(&polygon.0)
    }

    /// Terms to query for a `Polygon`.
    #[wasm_bindgen(js_name = "queryTermsForPolygon")]
    pub fn query_terms_for_polygon(&self, polygon: &Polygon) -> Vec<String> {
        self.0.get_query_terms(&polygon.0)
    }
}
