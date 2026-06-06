// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::cell_id::CellId;
use crate::point::Point;

/// A union of S2 cells, used to approximate regions on the sphere.
#[wasm_bindgen]
pub struct CellUnion(pub(crate) s2rst::s2::CellUnion);

impl Default for CellUnion {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl CellUnion {
    /// Empty cell union.
    #[wasm_bindgen(constructor)]
    pub fn new() -> CellUnion {
        CellUnion(s2rst::s2::CellUnion::new())
    }

    /// Create from an array of `CellId` objects (normalizes).
    #[wasm_bindgen(js_name = "fromCellIds")]
    pub fn from_cell_ids(ids: Vec<CellId>) -> CellUnion {
        let inner: Vec<s2rst::s2::CellId> = ids.iter().map(|c| c.0).collect();
        CellUnion(s2rst::s2::CellUnion::from_cell_ids(inner))
    }

    /// Normalized union covering the leaf-cell range `[begin, end)`.
    #[wasm_bindgen(js_name = "fromRange")]
    pub fn from_range(begin: &CellId, end: &CellId) -> CellUnion {
        CellUnion(s2rst::s2::CellUnion::from_range(begin.0, end.0))
    }

    /// Union covering the inclusive leaf-cell range `[min, max]`.
    #[wasm_bindgen(js_name = "fromMinMax")]
    pub fn from_min_max(min_id: &CellId, max_id: &CellId) -> CellUnion {
        CellUnion(s2rst::s2::CellUnion::from_min_max(min_id.0, max_id.0))
    }

    /// Union covering the leaf-cell range `[begin, end)`.
    #[wasm_bindgen(js_name = "fromBeginEnd")]
    pub fn from_begin_end(begin: &CellId, end: &CellId) -> CellUnion {
        CellUnion(s2rst::s2::CellUnion::from_begin_end(begin.0, end.0))
    }

    /// Create from token strings (normalizes).
    #[wasm_bindgen(js_name = "fromTokens")]
    pub fn from_tokens(tokens: Vec<String>) -> CellUnion {
        let ids: Vec<s2rst::s2::CellId> = tokens
            .iter()
            .map(|t| s2rst::s2::CellId::from_token(t))
            .collect();
        CellUnion(s2rst::s2::CellUnion::from_cell_ids(ids))
    }

    /// The whole sphere.
    #[wasm_bindgen(js_name = "wholeSphere")]
    pub fn whole_sphere() -> CellUnion {
        CellUnion(s2rst::s2::CellUnion::whole_sphere())
    }

    /// Number of cells.
    #[wasm_bindgen(js_name = "numCells")]
    pub fn num_cells(&self) -> usize {
        self.0.num_cells()
    }

    /// Whether empty.
    #[wasm_bindgen(js_name = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get cell ids as an array of `CellId`.
    #[wasm_bindgen(js_name = "cellIds")]
    pub fn cell_ids(&self) -> Vec<CellId> {
        self.0.cell_ids().iter().map(|c| CellId(*c)).collect()
    }

    /// Get cell ids as hex token strings.
    pub fn tokens(&self) -> Vec<String> {
        self.0.cell_ids().iter().map(|c| c.to_token()).collect()
    }

    /// Whether valid.
    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Whether normalized.
    #[wasm_bindgen(js_name = "isNormalized")]
    pub fn is_normalized(&self) -> bool {
        self.0.is_normalized()
    }

    /// Normalize in place.
    pub fn normalize(&mut self) {
        self.0.normalize();
    }

    /// Whether this union contains the given cell id.
    #[wasm_bindgen(js_name = "containsCellId")]
    pub fn contains_cell_id(&self, id: &CellId) -> bool {
        self.0.contains_cell_id(id.0)
    }

    /// Whether this union intersects the given cell id.
    #[wasm_bindgen(js_name = "intersectsCellId")]
    pub fn intersects_cell_id(&self, id: &CellId) -> bool {
        self.0.intersects_cell_id(id.0)
    }

    /// Whether this union contains the given point.
    #[wasm_bindgen(js_name = "containsPoint")]
    pub fn contains_point(&self, p: &Point) -> bool {
        self.0.contains_point(p.0)
    }

    /// Whether this union contains another union.
    #[wasm_bindgen(js_name = "containsUnion")]
    pub fn contains_union(&self, other: &CellUnion) -> bool {
        self.0.contains_union(&other.0)
    }

    /// Whether this union intersects another union.
    #[wasm_bindgen(js_name = "intersectsUnion")]
    pub fn intersects_union(&self, other: &CellUnion) -> bool {
        self.0.intersects_union(&other.0)
    }

    /// Union with another `CellUnion`.
    #[wasm_bindgen(js_name = "union")]
    pub fn union_with(&self, other: &CellUnion) -> CellUnion {
        CellUnion(self.0.union(&other.0))
    }

    /// Intersection with another `CellUnion`.
    #[wasm_bindgen(js_name = "intersection")]
    pub fn intersection_with(&self, other: &CellUnion) -> CellUnion {
        CellUnion(self.0.intersection(&other.0))
    }

    /// Difference with another `CellUnion`.
    #[wasm_bindgen(js_name = "difference")]
    pub fn difference_with(&self, other: &CellUnion) -> CellUnion {
        CellUnion(self.0.difference(&other.0))
    }

    /// Expand to include all cells at the given level adjacent to the union.
    #[wasm_bindgen(js_name = "expandAtLevel")]
    pub fn expand_at_level(&mut self, level: u8) {
        self.0.expand_at_level(level.into());
    }

    /// Expand by a minimum radius.
    #[wasm_bindgen(js_name = "expandByRadius")]
    pub fn expand_by_radius(&mut self, min_radius: &Angle, max_level_diff: u8) {
        self.0.expand_by_radius(min_radius.0, max_level_diff);
    }

    /// Approximate area in steradians.
    #[wasm_bindgen(js_name = "approxArea")]
    pub fn approx_area(&self) -> f64 {
        self.0.approx_area()
    }

    /// Exact area in steradians.
    #[wasm_bindgen(js_name = "exactArea")]
    pub fn exact_area(&self) -> f64 {
        self.0.exact_area()
    }

    /// Encode to the S2 binary format (`Uint8Array`).
    pub fn encode(&self) -> Vec<u8> {
        use s2rst::s2::encoding::S2Encode;
        let mut buf = Vec::new();
        self.0
            .encode(&mut buf)
            .expect("encoding to a Vec is infallible");
        buf
    }

    /// Decode from the S2 binary format. Throws on malformed data.
    pub fn decode(bytes: &[u8]) -> Result<CellUnion, JsValue> {
        use s2rst::s2::encoding::S2Decode;
        let mut cur = std::io::Cursor::new(bytes);
        s2rst::s2::CellUnion::decode(&mut cur)
            .map(CellUnion)
            .map_err(crate::error::js_err)
    }
}
