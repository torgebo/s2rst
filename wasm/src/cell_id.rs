// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::latlng::LatLng;
use crate::point::Point;

/// A 64-bit identifier for an S2 cell in the hierarchical decomposition.
///
/// Note: JS cannot natively hold u64 without `BigInt`. We expose the raw id
/// as a string and provide `fromToken`/`toToken` for ergonomic interchange.
#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct CellId(pub(crate) s2rst::s2::CellId);

#[wasm_bindgen]
impl CellId {
    /// The "none" (invalid) cell id.
    pub fn none() -> CellId {
        CellId(s2rst::s2::CellId::none())
    }

    /// The sentinel cell id.
    pub fn sentinel() -> CellId {
        CellId(s2rst::s2::CellId::sentinel())
    }

    /// Create from a face (0–5).
    #[wasm_bindgen(js_name = "fromFace")]
    pub fn from_face(face: u8) -> CellId {
        CellId(s2rst::s2::CellId::from_face(face))
    }

    /// Create from a `Point`.
    #[wasm_bindgen(js_name = "fromPoint")]
    pub fn from_point(p: &Point) -> CellId {
        CellId(s2rst::s2::CellId::from_point(&p.0))
    }

    /// Create from a `LatLng`.
    #[wasm_bindgen(js_name = "fromLatLng")]
    pub fn from_lat_lng(ll: &LatLng) -> CellId {
        CellId(s2rst::s2::CellId::from_lat_lng(&ll.0))
    }

    /// Create from a hex token string.
    #[wasm_bindgen(js_name = "fromToken")]
    pub fn from_token(token: &str) -> CellId {
        CellId(s2rst::s2::CellId::from_token(token))
    }

    /// The raw id as a decimal string (for BigInt interop).
    #[wasm_bindgen(getter, js_name = "idString")]
    pub fn id_string(&self) -> String {
        self.0.id().to_string()
    }

    /// The raw u64 as two 32-bit numbers `[hi, lo]` for JS interop.
    #[wasm_bindgen(js_name = "idParts")]
    pub fn id_parts(&self) -> Vec<u32> {
        let id = self.0.id();
        vec![(id >> 32) as u32, id as u32]
    }

    /// Hex token string.
    #[wasm_bindgen(js_name = "toToken")]
    pub fn to_token(&self) -> String {
        self.0.to_token()
    }

    /// Whether this is a valid cell id.
    #[wasm_bindgen(js_name = "isValid")]
    pub fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// The face (0–5).
    #[wasm_bindgen(getter)]
    pub fn face(&self) -> u8 {
        self.0.face().into()
    }

    /// The level (0–30).
    #[wasm_bindgen(getter)]
    pub fn level(&self) -> u8 {
        self.0.level().into()
    }

    /// Whether this is a leaf cell (level 30).
    #[wasm_bindgen(js_name = "isLeaf")]
    pub fn is_leaf(&self) -> bool {
        self.0.is_leaf()
    }

    /// Whether this is a face cell (level 0).
    #[wasm_bindgen(js_name = "isFace")]
    pub fn is_face(&self) -> bool {
        self.0.is_face()
    }

    /// The parent cell id.
    pub fn parent(&self) -> CellId {
        CellId(self.0.parent())
    }

    /// The parent at a given level.
    #[wasm_bindgen(js_name = "parentAtLevel")]
    pub fn parent_at_level(&self, level: u8) -> CellId {
        CellId(self.0.parent_at_level(level))
    }

    /// The four children.
    pub fn children(&self) -> Vec<CellId> {
        self.0.children().iter().map(|c| CellId(*c)).collect()
    }

    // -- Hierarchy / range iteration (Tier 1.4) --------------------------------

    /// Construct from a face, Hilbert-curve position (BigInt), and level.
    #[wasm_bindgen(js_name = "fromFacePosLevel")]
    pub fn from_face_pos_level(face: u8, pos: u64, level: u8) -> CellId {
        CellId(s2rst::s2::CellId::from_face_pos_level(face, pos, level))
    }

    /// Position along the Hilbert curve within the cell's face (BigInt).
    pub fn pos(&self) -> u64 {
        self.0.pos()
    }

    /// The least-significant set bit of the id (BigInt).
    pub fn lsb(&self) -> u64 {
        self.0.lsb()
    }

    /// Which child (0–3) this cell is of its ancestor at `level`.
    #[wasm_bindgen(js_name = "childPosition")]
    pub fn child_position(&self, level: u8) -> u8 {
        self.0.child_position(level)
    }

    /// First child (immediate). Use with `childEnd()` to iterate children.
    #[wasm_bindgen(js_name = "childBegin")]
    pub fn child_begin(&self) -> CellId {
        CellId(self.0.child_begin())
    }

    /// Past-the-end child (immediate).
    #[wasm_bindgen(js_name = "childEnd")]
    pub fn child_end(&self) -> CellId {
        CellId(self.0.child_end())
    }

    /// First descendant at `level`.
    #[wasm_bindgen(js_name = "childBeginAtLevel")]
    pub fn child_begin_at_level(&self, level: u8) -> CellId {
        CellId(self.0.child_begin_at_level(level))
    }

    /// Past-the-end descendant at `level`.
    #[wasm_bindgen(js_name = "childEndAtLevel")]
    pub fn child_end_at_level(&self, level: u8) -> CellId {
        CellId(self.0.child_end_at_level(level))
    }

    /// First cell id at `level` (across the whole sphere).
    pub fn begin(level: u8) -> CellId {
        CellId(s2rst::s2::CellId::begin(level))
    }

    /// Past-the-end cell id at `level` (across the whole sphere).
    pub fn end(level: u8) -> CellId {
        CellId(s2rst::s2::CellId::end(level))
    }

    /// Advance `steps` cells at this id's level (negative steps go back).
    pub fn advance(&self, steps: i64) -> CellId {
        CellId(self.0.advance(steps))
    }

    /// Advance `steps` cells at this id's level, wrapping around the curve.
    #[wasm_bindgen(js_name = "advanceWrap")]
    pub fn advance_wrap(&self, steps: i64) -> CellId {
        CellId(self.0.advance_wrap(steps))
    }

    /// The minimum cell id in this cell's range.
    #[wasm_bindgen(js_name = "rangeMin")]
    pub fn range_min(&self) -> CellId {
        CellId(self.0.range_min())
    }

    /// The maximum cell id in this cell's range.
    #[wasm_bindgen(js_name = "rangeMax")]
    pub fn range_max(&self) -> CellId {
        CellId(self.0.range_max())
    }

    /// Whether this cell contains the given cell.
    pub fn contains(&self, other: &CellId) -> bool {
        self.0.contains(other.0)
    }

    /// Whether this cell intersects the given cell.
    pub fn intersects(&self, other: &CellId) -> bool {
        self.0.intersects(other.0)
    }

    /// Next cell at the same level.
    pub fn next(&self) -> CellId {
        CellId(self.0.next())
    }

    /// Previous cell at the same level.
    pub fn prev(&self) -> CellId {
        CellId(self.0.prev())
    }

    /// Next cell with wrapping.
    #[wasm_bindgen(js_name = "nextWrap")]
    pub fn next_wrap(&self) -> CellId {
        CellId(self.0.next_wrap())
    }

    /// Previous cell with wrapping.
    #[wasm_bindgen(js_name = "prevWrap")]
    pub fn prev_wrap(&self) -> CellId {
        CellId(self.0.prev_wrap())
    }

    /// The center of this cell as a `Point`.
    #[wasm_bindgen(js_name = "toPoint")]
    pub fn to_point(&self) -> Point {
        Point(self.0.to_point())
    }

    /// The center of this cell as a `LatLng`.
    #[wasm_bindgen(js_name = "toLatLng")]
    pub fn to_lat_lng(&self) -> LatLng {
        LatLng(self.0.to_lat_lng())
    }

    /// The four edge-adjacent cells at the same level.
    #[wasm_bindgen(js_name = "edgeNeighbors")]
    pub fn edge_neighbors(&self) -> Vec<CellId> {
        self.0.edge_neighbors().iter().map(|c| CellId(*c)).collect()
    }

    /// Vertex neighbors at the given level.
    #[wasm_bindgen(js_name = "vertexNeighbors")]
    pub fn vertex_neighbors(&self, level: u8) -> Vec<CellId> {
        self.0
            .vertex_neighbors(level)
            .into_iter()
            .map(CellId)
            .collect()
    }

    /// All neighbors at the given level.
    #[wasm_bindgen(js_name = "allNeighbors")]
    pub fn all_neighbors(&self, level: u8) -> Option<Vec<CellId>> {
        self.0
            .all_neighbors(level)
            .map(|v| v.into_iter().map(CellId).collect())
    }

    /// Common ancestor level, or -1 if none.
    #[wasm_bindgen(js_name = "commonAncestorLevel")]
    pub fn common_ancestor_level(&self, other: &CellId) -> i8 {
        self.0
            .common_ancestor_level(other.0)
            .map(|l| -> i8 { u8::from(l) as i8 })
            .unwrap_or(-1)
    }

    /// Human-readable debug string (e.g., "3/012").
    #[wasm_bindgen(js_name = "toDebugString")]
    pub fn to_debug_string(&self) -> String {
        self.0.to_debug_string()
    }

    /// Parse from debug string (e.g., "3/012").
    #[wasm_bindgen(js_name = "fromDebugString")]
    pub fn from_debug_string(s: &str) -> Option<CellId> {
        s2rst::s2::CellId::from_debug_string(s).map(CellId)
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_js(&self) -> String {
        self.0.to_token()
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
    pub fn decode(bytes: &[u8]) -> Result<CellId, JsValue> {
        use s2rst::s2::encoding::S2Decode;
        let mut cur = std::io::Cursor::new(bytes);
        s2rst::s2::CellId::decode(&mut cur)
            .map(CellId)
            .map_err(crate::error::js_err)
    }
}
