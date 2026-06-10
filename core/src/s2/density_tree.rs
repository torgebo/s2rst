// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! [`S2DensityTree`] represents a spatial histogram (quadtree) over the sphere.
//!
//! A density tree is a map from [`CellId`]s to weights with the property that
//! if any cell is present in the tree, all of its ancestors are also present.
//! The weight of each cell is the sum of the weights of the shapes that
//! intersect that cell.
//!
//! The tree is stored in a compact binary encoding matching the C++ format.
//! Cells are parsed lazily during visitation.
//!
//! Corresponds to C++ `s2density_tree.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "tree level/weight values (i32/i64) used as indices — always non-negative"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "tree weight/level values — bounded by tree construction"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64/u32 -> i64/i32 for tree weight arithmetic — bounded by tree structure"
)]
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::ops::ControlFlow;

use crate::s1::Angle;
use crate::s2::builder::{S2Error, S2ErrorCode};
use crate::s2::cell_union::CellUnion;
use crate::s2::coords::Level;
use crate::s2::coords::MAX_CELL_LEVEL;
use crate::s2::metric::MIN_WIDTH;
use crate::s2::shape::{Dimension, Shape};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_index_region::ShapeIndexRegion;
use crate::s2::{Cell, CellId};

const CHILD_MASK_BITS: u32 = 4;
const NUM_CHILDREN: usize = 4;
const NUM_FACES: usize = 6;
const VERSION: &[u8] = b"S2DensityTree0";

/// Maximum weight storable in a single cell (60 bits; low 4 are child mask).
pub const MAX_WEIGHT: i64 = i64::MAX >> CHILD_MASK_BITS;

// ─── S2DensityTree ─────────────────────────────────────────────────────

/// A spatial histogram over the sphere, stored in a compact binary encoding.
#[derive(Clone, Debug, PartialEq)]
pub struct S2DensityTree {
    encoded: Vec<u8>,
    decoded_faces: [i64; NUM_FACES],
}

impl Default for S2DensityTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Action returned by the visitor callback in [`S2DensityTree::visit_cells`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisitAction {
    /// Recurse into children of the current cell.
    #[default]
    EnterCell,
    /// Skip children and proceed to the next sibling.
    SkipCell,
    /// Stop the entire visitation.
    Stop,
}

/// A decoded cell: weight + child offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct DensityCell {
    weight: i64,
    offsets: [i64; NUM_CHILDREN],
}

impl Default for DensityCell {
    fn default() -> Self {
        Self {
            weight: 0,
            offsets: [-1; NUM_CHILDREN],
        }
    }
}

impl DensityCell {
    /// The weight stored in this cell.
    pub fn weight(&self) -> i64 {
        self.weight
    }
    /// Whether this cell has any encoded children.
    pub fn has_children(&self) -> bool {
        self.offsets.iter().any(|&o| o >= 0)
    }
    /// Byte offset of child `i` in the encoded data, or negative if absent.
    pub fn child_offset(&self, i: usize) -> i64 {
        self.offsets[i]
    }

    fn clear(&mut self) {
        self.weight = 0;
        self.offsets = [-1; NUM_CHILDREN];
    }

    fn decode(data: &[u8], pos: usize) -> Result<Self, S2Error> {
        let (bits, mut offset) = decode_varint_at(data, pos).ok_or_else(|| {
            S2Error::new(
                S2ErrorCode::Internal,
                format!("Failed to decode cell at {pos}"),
            )
        })?;
        let weight = (bits >> CHILD_MASK_BITS) as i64;
        let child_mask = (bits & 0xF) as u8;
        let mut offsets = [-1i64; NUM_CHILDREN];
        if child_mask == 0 {
            return Ok(Self { weight, offsets });
        }
        let num_set = child_mask.count_ones() as usize;
        let mut cum: i64 = 0;
        let mut found = 0usize;
        for (i, slot) in offsets.iter_mut().enumerate() {
            if child_mask & (1 << i) != 0 {
                *slot = cum;
                found += 1;
                if found < num_set {
                    let (v, next) = decode_varint_at(data, offset).ok_or_else(|| {
                        S2Error::new(
                            S2ErrorCode::Internal,
                            format!("Failed to decode child offset at {pos}"),
                        )
                    })?;
                    cum = add_i64(cum, v as i64)?;
                    offset = next;
                }
            }
        }
        let header_end = offset;
        for o in &mut offsets {
            if *o >= 0 {
                let abs = add_i64(*o, header_end as i64)?;
                if abs as usize >= data.len() {
                    return Err(S2Error::new(
                        S2ErrorCode::InvalidArgument,
                        format!("child offset out of range at cell {pos}"),
                    ));
                }
                *o = abs;
            }
        }
        Ok(Self { weight, offsets })
    }
}

// ─── DecodedPath ───────────────────────────────────────────────────────

/// Lazy path decoder caching cells from root to the last requested cell.
#[derive(Debug)]
pub struct DecodedPath<'a> {
    tree: &'a S2DensityTree,
    stack: Vec<DensityCell>,
    /// Byte offset each `stack[level]` was decoded from, or `-1` when that level
    /// is empty. Mirrors `stack` and lets `load_cell` reject an offset that
    /// already appears higher on the current root-to-cell path — the same
    /// aliased/cyclic-encoding guard `visit_recursive` applies (see Finding C in
    /// `fuzz_decode_density_tree.md`). A valid tree encodes each cell once at a
    /// unique offset, so a repeat on a single descending path is always corrupt.
    offsets: Vec<i64>,
    last: CellId,
}

impl<'a> DecodedPath<'a> {
    /// Creates a new path decoder for the given tree.
    pub fn new(tree: &'a S2DensityTree) -> Self {
        Self {
            tree,
            stack: (0..=MAX_CELL_LEVEL)
                .map(|_| DensityCell::default())
                .collect(),
            offsets: (0..=MAX_CELL_LEVEL).map(|_| -1i64).collect(),
            last: CellId::sentinel(),
        }
    }

    /// Returns the underlying tree reference.
    pub fn tree(&self) -> &'a S2DensityTree {
        self.tree
    }

    /// Returns the [`DensityCell`] for `cell_id`, decoding as needed.
    pub fn get_cell(&mut self, cell_id: CellId, error: &mut S2Error) -> &DensityCell {
        // Sentinel has invalid face bits, so check explicitly.
        let different_face = self.last == CellId::sentinel()
            || u8::from(self.last.face()) != u8::from(cell_id.face());
        if different_face {
            self.last = cell_id.parent_at_level(0);
            self.load_face(u8::from(cell_id.face()), error);
            if !error.is_ok() {
                return &self.stack[0];
            }
        }
        self.load_cell(cell_id, error)
    }

    fn load_face(&mut self, face: u8, error: &mut S2Error) {
        let offset = self.tree.decoded_faces[face as usize];
        if offset < 0 {
            self.stack[0].clear();
            self.offsets[0] = -1;
        } else {
            match DensityCell::decode(&self.tree.encoded, offset as usize) {
                Ok(c) => {
                    self.stack[0] = c;
                    self.offsets[0] = offset;
                }
                Err(e) => {
                    *error = e;
                    self.stack[0].clear();
                    self.offsets[0] = -1;
                }
            }
        }
    }

    fn load_cell(&mut self, cell_id: CellId, error: &mut S2Error) -> &DensityCell {
        let start_level = self
            .last
            .common_ancestor_level(cell_id)
            .map_or(0, Level::as_usize);
        let cell_level = cell_id.level().as_usize();

        let mut result_level = start_level;
        let mut level = start_level + 1;
        while level <= cell_level {
            let child_pos = cell_id.child_position(level as u8) as usize;
            let offset = self.stack[level - 1].child_offset(child_pos);
            if offset < 0 {
                if self.stack[level - 1].has_children() {
                    self.stack[level].clear();
                    self.offsets[level] = -1;
                    result_level = level;
                } else {
                    result_level = level - 1;
                }
                break;
            }
            // Reject an offset already seen higher on this path: a valid tree
            // references each cell once, so a repeat means an aliased/cyclic
            // encoding. Treated as a decode error (same as `visit_recursive`).
            if self.offsets[..level].contains(&offset) {
                *error = S2Error::new(
                    S2ErrorCode::InvalidArgument,
                    "S2DensityTree cell offset visited twice (aliased/cyclic encoding)",
                );
                self.stack[level].clear();
                self.offsets[level] = -1;
                self.last = cell_id.parent_at_level(level as u8 - 1);
                return &self.stack[level];
            }
            match DensityCell::decode(&self.tree.encoded, offset as usize) {
                Ok(c) => {
                    self.stack[level] = c;
                    self.offsets[level] = offset;
                }
                Err(e) => {
                    *error = e;
                    self.stack[level].clear();
                    self.offsets[level] = -1;
                    self.last = cell_id.parent_at_level(level as u8 - 1);
                    return &self.stack[level];
                }
            }
            result_level = level;
            level += 1;
        }
        self.last = cell_id.parent_at_level(result_level as u8);
        &self.stack[result_level]
    }
}

// ─── Encoding helpers ──────────────────────────────────────────────────

struct ReversibleBytes {
    bytes: Vec<u8>,
}

impl ReversibleBytes {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }
    fn append_bytes(&mut self, data: &[u8]) {
        self.bytes.extend_from_slice(data);
    }
    fn append_varint64(&mut self, mut v: u64) {
        while v >= 0x80 {
            self.bytes.push((v as u8) | 0x80);
            v >>= 7;
        }
        self.bytes.push(v as u8);
    }
    fn reverse_from(&mut self, start: usize) {
        self.bytes[start..].reverse();
    }
    fn size(&self) -> usize {
        self.bytes.len()
    }
    fn reversed(&self) -> Vec<u8> {
        self.bytes.iter().rev().copied().collect()
    }
}

struct ReversedCellEncoder {
    lengths: [u64; NUM_FACES],
    size: usize,
    start: usize,
}

impl ReversedCellEncoder {
    fn new(output: &ReversibleBytes) -> Self {
        Self {
            lengths: [0; NUM_FACES],
            size: 0,
            start: output.size(),
        }
    }
    fn next(&mut self, output: &ReversibleBytes) {
        debug_assert!(self.size < self.lengths.len());
        self.lengths[self.size] = (output.size() - self.start) as u64;
        self.size += 1;
        self.start = output.size();
    }
    fn finish(self, v: u64, output: &mut ReversibleBytes) {
        output.append_varint64(v);
        for i in (1..self.size).rev() {
            output.append_varint64(self.lengths[i]);
        }
        output.reverse_from(self.start);
    }
}

// ─── TreeEncoder ───────────────────────────────────────────────────────

/// Collects cell/weight pairs and encodes them into an `S2DensityTree`.
#[derive(Debug, Default)]
pub struct TreeEncoder {
    weights: BTreeMap<CellId, i64>,
}

impl TreeEncoder {
    /// Creates an empty encoder.
    pub fn new() -> Self {
        Self {
            weights: BTreeMap::new(),
        }
    }

    /// Adds `weight` to `cell` (accumulates if already present).
    pub fn put(&mut self, cell: CellId, weight: i64) {
        *self.weights.entry(cell).or_insert(0) += weight;
    }

    /// Encodes accumulated weights into an [`S2DensityTree`].
    pub fn build(&mut self, tree: &mut S2DensityTree) {
        let mut output = ReversibleBytes::new();
        self.encode_tree_reversed(&mut output);
        output.append_bytes(VERSION);
        output.reverse_from(output.size() - VERSION.len());
        let bytes = output.reversed();
        let mut error = S2Error::ok();
        let faces = decode_header(&bytes, &mut error);
        debug_assert!(error.is_ok(), "{error}");
        tree.encoded = bytes;
        tree.decoded_faces = faces;
    }

    /// Estimates the encoded size in bytes for a cell with the given weight.
    pub fn estimate_size(weight: i64) -> usize {
        let ws = varint_length64((weight as u64) << CHILD_MASK_BITS | 0xF);
        ws + 2 * varint_length64(ws as u64)
    }

    /// Removes all accumulated weights.
    pub fn clear(&mut self) {
        self.weights.clear();
    }

    fn encode_tree_reversed(&self, output: &mut ReversibleBytes) {
        let mut enc = ReversedCellEncoder::new(output);
        let mut mask: u64 = 0;
        for face in (0..NUM_FACES).rev() {
            let fc = CellId::from_face(face as u8);
            if let Some(&w) = self.weights.get(&fc) {
                self.encode_subtree_reversed(fc, w, output);
                enc.next(output);
                mask |= 1 << face;
            }
        }
        enc.finish(mask, output);
    }

    fn encode_subtree_reversed(&self, cell_id: CellId, weight: i64, output: &mut ReversibleBytes) {
        let mut enc = ReversedCellEncoder::new(output);
        let mut mask: u64 = 0;
        if !cell_id.is_leaf() {
            let children = cell_id.children();
            for i in (0..NUM_CHILDREN).rev() {
                if let Some(&cw) = self.weights.get(&children[i]) {
                    self.encode_subtree_reversed(children[i], cw, output);
                    enc.next(output);
                    mask |= 1 << i;
                }
            }
        }
        enc.finish(((weight as u64) << CHILD_MASK_BITS) | mask, output);
    }
}

// ─── BreadthFirstTreeBuilder ───────────────────────────────────────────

/// Builds density trees via breadth-first visitation of a weight function.
#[derive(Debug)]
pub struct BreadthFirstTreeBuilder {
    approximate_size_bytes: i64,
    max_level: u8,
    encoder: TreeEncoder,
}

impl BreadthFirstTreeBuilder {
    /// Creates a builder that targets `approximate_size_bytes` and stops at `max_level`.
    ///
    /// `max_level` is clamped to `0..=`[`MAX_CELL_LEVEL`]; values above the
    /// maximum cell level are treated as the maximum rather than panicking.
    pub fn new(approximate_size_bytes: i64, max_level: u8) -> Self {
        Self {
            approximate_size_bytes,
            max_level: max_level.min(MAX_CELL_LEVEL),
            encoder: TreeEncoder::new(),
        }
    }

    /// Builds the tree by calling `weight_fn` for each cell in breadth-first order.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn build<F>(&mut self, mut weight_fn: F, tree: &mut S2DensityTree) -> Result<(), S2Error>
    where
        F: FnMut(CellId) -> Result<i64, S2Error>,
    {
        let mut ranges = vec![(CellId::begin(MAX_CELL_LEVEL), CellId::end(MAX_CELL_LEVEL))];
        let mut next_ranges: Vec<(CellId, CellId)> = Vec::new();
        let mut size_est: i64 = 0;

        for level in 0..=self.max_level {
            if ranges.is_empty() || size_est >= self.approximate_size_bytes {
                break;
            }
            let mut last_end = CellId::sentinel();
            for &(rs, re) in &ranges {
                let mut cid = rs.parent_at_level(level);
                while cid < re {
                    let w = weight_fn(cid)?;
                    match w.cmp(&0) {
                        std::cmp::Ordering::Equal => {}
                        std::cmp::Ordering::Less => {
                            let aw = (-w).min(MAX_WEIGHT);
                            self.encoder.put(cid, aw);
                            size_est += TreeEncoder::estimate_size(aw) as i64;
                        }
                        std::cmp::Ordering::Greater => {
                            let begin = cid.range_min();
                            let end = cid.range_max().next();
                            if begin == last_end {
                                if let Some(last) = next_ranges.last_mut() {
                                    last.1 = end;
                                }
                            } else {
                                next_ranges.push((begin, end));
                            }
                            last_end = end;
                            let aw = w.min(MAX_WEIGHT);
                            self.encoder.put(cid, aw);
                            size_est += TreeEncoder::estimate_size(aw) as i64;
                        }
                    }
                    cid = cid.next();
                }
            }
            ranges = std::mem::take(&mut next_ranges);
        }
        self.encoder.build(tree);
        Ok(())
    }
}

// ─── IndexCellWeightFunction ───────────────────────────────────────────

pub(crate) struct IndexCellWeightFunction<'a, F> {
    index: &'a ShapeIndex,
    region: ShapeIndexRegion<'a>,
    weight_fn: F,
}

impl<'a, F: Fn(&dyn Shape) -> i64> IndexCellWeightFunction<'a, F> {
    pub(crate) fn new(index: &'a ShapeIndex, weight_fn: F) -> Self {
        Self {
            index,
            region: ShapeIndexRegion::new(index),
            weight_fn,
        }
    }

    pub(crate) fn weigh_cell(&self, cell_id: CellId) -> Result<i64, S2Error> {
        let target = Cell::from(cell_id);
        let mut sum: i64 = 0;
        let mut all_contained = true;
        let _ = self
            .region
            .visit_intersecting_shape_ids(&target, |shape_id, contains| {
                if let Some(shape) = self.index.shape(shape_id) {
                    let w = (self.weight_fn)(shape);
                    debug_assert!(w >= 0);
                    debug_assert!(w <= MAX_WEIGHT);
                    sum += w;
                    all_contained &= contains;
                }
                ControlFlow::Continue(())
            });
        sum = sum.min(MAX_WEIGHT);
        Ok(if all_contained { -sum } else { sum })
    }
}

// ─── FeatureMap ────────────────────────────────────────────────────────

/// Maps shape IDs to feature IDs with associated weights, enabling
/// deduplication when multiple shapes represent the same feature.
///
/// Feature IDs are contiguous integers starting from 0, assigned
/// automatically as new features are registered via [`FeatureMap::from_shapes`].
///
/// # Example
///
/// ```ignore
/// // Two shapes map to feature "building_42" (weight 1), one to "road_7" (weight 5).
/// let map = FeatureMap::from_shapes(3, [
///     (0, "building_42", 1),
///     (1, "building_42", 1),   // same key → same feature ID, weight reused
///     (2, "road_7",      5),
/// ]);
/// assert_eq!(map.num_features(), 2);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FeatureMap {
    /// `shape_id → feature_id`, or `None` if the shape is unmapped.
    shape_to_feature: Vec<Option<usize>>,
    /// `feature_id → weight`.
    feature_weights: Vec<i64>,
}

impl FeatureMap {
    /// Creates a `FeatureMap` from an iterator of `(shape_id, feature_key, weight)`
    /// triples.
    ///
    /// `num_shape_ids` is the total number of shape IDs in the index (used to
    /// size the internal lookup table). Shapes not present in `entries` are
    /// unmapped and will be skipped during density computation.
    ///
    /// `feature_key` can be any hashable type (`&str`, `usize`, a custom ID,
    /// etc.). Shapes with the same key are mapped to the same feature ID. The
    /// weight of a feature is taken from its **first** occurrence.
    pub fn from_shapes<K: Eq + Hash>(
        num_shape_ids: usize,
        entries: impl IntoIterator<Item = (i32, K, i64)>,
    ) -> Self {
        let mut shape_to_feature = vec![None; num_shape_ids];
        let mut feature_weights = Vec::new();
        let mut key_to_id: HashMap<K, usize> = HashMap::new();

        for (shape_id, key, weight) in entries {
            let next_id = key_to_id.len();
            let &mut fid = key_to_id.entry(key).or_insert_with(|| {
                feature_weights.push(weight);
                next_id
            });
            if let Some(slot) = shape_to_feature.get_mut(shape_id as usize) {
                *slot = Some(fid);
            }
        }

        Self {
            shape_to_feature,
            feature_weights,
        }
    }

    /// Returns the feature ID for a given shape ID, or `None` if unmapped.
    #[inline]
    pub fn feature_id(&self, shape_id: impl Into<crate::s2::shape::ShapeId>) -> Option<usize> {
        let shape_id = shape_id.into();
        self.shape_to_feature
            .get(shape_id.as_usize())
            .copied()
            .flatten()
    }

    /// Returns the weight of a feature by its ID.
    #[inline]
    pub fn feature_weight(&self, feature_id: usize) -> i64 {
        self.feature_weights[feature_id]
    }

    /// Returns the number of unique features.
    #[inline]
    pub fn num_features(&self) -> usize {
        self.feature_weights.len()
    }
}

// ─── FeatureCellWeightFunction ─────────────────────────────────────────

/// Weighs cells by summing intersecting features with epoch-based
/// deduplication, so that a feature with multiple shapes is counted only once
/// per cell.
///
/// Corresponds to C++ `S2DensityTree::FeatureCellWeightFunction<T>`.
pub(crate) struct FeatureCellWeightFunction<'a> {
    region: ShapeIndexRegion<'a>,
    feature_map: &'a FeatureMap,
    /// Epoch-based dedup: `last_call[feature_id]` records the epoch when a
    /// feature was last counted.
    last_call: Vec<u32>,
    /// Current epoch, incremented per `weigh_cell` call.
    next_call: u32,
}

impl<'a> FeatureCellWeightFunction<'a> {
    pub(crate) fn new(index: &'a ShapeIndex, feature_map: &'a FeatureMap) -> Self {
        Self {
            region: ShapeIndexRegion::new(index),
            feature_map,
            last_call: vec![0; feature_map.num_features()],
            next_call: 0,
        }
    }

    pub(crate) fn weigh_cell(&mut self, cell_id: CellId) -> Result<i64, S2Error> {
        let target = Cell::from(cell_id);
        let mut sum: i64 = 0;
        let mut all_contained = true;

        // Advance epoch (with wraparound reset matching C++).
        self.next_call += 1;
        if self.next_call == u32::MAX {
            self.next_call = 1;
            self.last_call.fill(0);
        }

        let next = self.next_call;
        let feature_map = self.feature_map;
        let last_call = &mut self.last_call;
        let _ = self
            .region
            .visit_intersecting_shape_ids(&target, |shape_id, contains| {
                if let Some(fid) = feature_map.feature_id(shape_id)
                    && last_call[fid] != next
                {
                    last_call[fid] = next;
                    let w = feature_map.feature_weight(fid);
                    debug_assert!(w >= 0);
                    debug_assert!(w <= MAX_WEIGHT);
                    sum += w;
                    all_contained &= contains;
                }
                ControlFlow::Continue(())
            });
        sum = sum.min(MAX_WEIGHT);
        Ok(if all_contained { -sum } else { sum })
    }
}

// ─── S2DensityTree ─────────────────────────────────────────────────────

impl S2DensityTree {
    /// Creates a new empty density tree.
    pub fn new() -> Self {
        Self {
            encoded: Vec::new(),
            decoded_faces: [-1; NUM_FACES],
        }
    }
    /// Returns the size of the encoded representation in bytes.
    pub fn encoded_size(&self) -> usize {
        self.encoded.len()
    }
    /// Returns `true` if the tree has no encoded data.
    pub fn is_empty(&self) -> bool {
        self.encoded.is_empty()
    }

    /// Builds the tree from a shape index, weighting each shape via `weight_fn`.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init_to_shape_density<F: Fn(&dyn Shape) -> i64>(
        &mut self,
        index: &ShapeIndex,
        weight_fn: F,
        approximate_size_bytes: i64,
        max_level: u8,
    ) -> Result<(), S2Error> {
        let m = IndexCellWeightFunction::new(index, weight_fn);
        let mut b = BreadthFirstTreeBuilder::new(approximate_size_bytes, max_level);
        b.build(|cid| m.weigh_cell(cid), self)
    }

    /// Builds the tree from a shape index using vertex count as weight.
    ///
    /// `max_level` is clamped to `0..=`[`MAX_CELL_LEVEL`].
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init_to_vertex_density(
        &mut self,
        index: &ShapeIndex,
        approximate_size_bytes: i64,
        max_level: u8,
    ) -> Result<(), S2Error> {
        self.init_to_shape_density(
            index,
            |s: &dyn Shape| match s.dimension() {
                Dimension::Point => s.num_chains() as i64,
                Dimension::Polyline => (s.num_chains() + s.num_edges()) as i64,
                Dimension::Polygon => s.num_edges() as i64,
            },
            approximate_size_bytes,
            max_level,
        )
    }

    /// Builds the tree from a shape index with feature-level deduplication.
    ///
    /// Unlike [`init_to_shape_density`](Self::init_to_shape_density), which
    /// counts each shape independently, this method uses a [`FeatureMap`] to
    /// group multiple shapes into the same "feature". A feature is counted only
    /// once per cell intersection, even if several of its shapes intersect that
    /// cell.
    ///
    /// Corresponds to C++ `S2DensityTree::InitToFeatureDensity<T>`.
    ///
    /// `max_level` is clamped to `0..=`[`MAX_CELL_LEVEL`].
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init_to_feature_density(
        &mut self,
        index: &ShapeIndex,
        feature_map: &FeatureMap,
        approximate_size_bytes: i64,
        max_level: u8,
    ) -> Result<(), S2Error> {
        let mut m = FeatureCellWeightFunction::new(index, feature_map);
        let mut b = BreadthFirstTreeBuilder::new(approximate_size_bytes, max_level);
        b.build(|cid| m.weigh_cell(cid), self)
    }

    /// Builds a tree as the sum of the given trees, with an approximate size limit.
    ///
    /// `max_level` is clamped to `0..=`[`MAX_CELL_LEVEL`].
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init_to_sum_density_with_size(
        &mut self,
        trees: &[&S2DensityTree],
        approximate_size_bytes: i64,
        max_level: u8,
    ) -> Result<(), S2Error> {
        let mut paths: Vec<DecodedPath> = trees.iter().map(|t| DecodedPath::new(t)).collect();
        let mut b = BreadthFirstTreeBuilder::new(approximate_size_bytes, max_level);
        b.build(
            |cid| {
                let mut sum: i64 = 0;
                let mut contained = true;
                for p in &mut paths {
                    let mut e = S2Error::ok();
                    let c = p.get_cell(cid, &mut e);
                    if !e.is_ok() {
                        return Err(e);
                    }
                    let w = c.weight();
                    let hc = c.has_children();
                    sum += w;
                    contained &= !hc;
                    sum = sum.min(MAX_WEIGHT);
                }
                Ok(if contained { -sum } else { sum })
            },
            self,
        )
    }

    /// Builds a tree as the exact sum of the given trees.
    ///
    /// `max_level` is clamped to `0..=`[`MAX_CELL_LEVEL`].
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init_to_sum_density(
        &mut self,
        trees: &[&S2DensityTree],
        max_level: u8,
    ) -> Result<(), S2Error> {
        let max_level = Level::try_new(max_level).unwrap_or(Level::MAX);
        let mut error = S2Error::ok();
        let mut enc = TreeEncoder::new();
        for tree in trees {
            let ok = tree.visit_cells_inner(
                |cid, cell| {
                    if cid.level() > max_level {
                        return VisitAction::SkipCell;
                    }
                    enc.put(cid, cell.weight());
                    VisitAction::EnterCell
                },
                &mut error,
            );
            if !ok {
                return Err(error);
            }
        }
        enc.build(self);
        Ok(())
    }

    /// Visits all cells in the tree via depth-first traversal.
    ///
    /// Returns `Ok(())` if all cells were visited (or the visitor returned
    /// `VisitAction::Stop`).
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt.
    pub fn visit_cells<F: FnMut(CellId, &DensityCell) -> VisitAction>(
        &self,
        visitor: F,
    ) -> Result<(), S2Error> {
        let mut error = S2Error::ok();
        self.visit_cells_inner(visitor, &mut error);
        if error.is_ok() { Ok(()) } else { Err(error) }
    }

    fn visit_cells_inner<F: FnMut(CellId, &DensityCell) -> VisitAction>(
        &self,
        mut visitor: F,
        error: &mut S2Error,
    ) -> bool {
        *error = S2Error::ok();
        let mut visited = HashSet::new();
        for face in 0..NUM_FACES {
            let off = self.decoded_faces[face];
            if off < 0 {
                continue;
            }
            if !self.visit_recursive(
                &mut visitor,
                CellId::from_face(face as u8),
                off,
                &mut visited,
                error,
            ) {
                return false;
            }
        }
        true
    }

    fn visit_recursive<F: FnMut(CellId, &DensityCell) -> VisitAction>(
        &self,
        visitor: &mut F,
        cell_id: CellId,
        pos: i64,
        visited: &mut HashSet<usize>,
        error: &mut S2Error,
    ) -> bool {
        // A valid density tree is a TREE: each cell is encoded once and
        // referenced once. Reaching a byte offset twice means the input is
        // corrupt or crafted to force 2^level re-decodes; reject it. Bounds
        // total traversal work and the decoded map to O(encoded.len()).
        if !visited.insert(pos as usize) {
            *error = S2Error::new(
                S2ErrorCode::InvalidArgument,
                "S2DensityTree cell offset visited twice (aliased/cyclic encoding)",
            );
            return false;
        }
        let cell = match DensityCell::decode(&self.encoded, pos as usize) {
            Ok(c) => c,
            Err(e) => {
                *error = e;
                return false;
            }
        };
        match visitor(cell_id, &cell) {
            VisitAction::Stop => false,
            VisitAction::SkipCell => true,
            VisitAction::EnterCell => {
                if !cell_id.is_leaf() && cell.has_children() {
                    let children = cell_id.children();
                    for (i, &child) in children.iter().enumerate() {
                        let off = cell.child_offset(i);
                        if off >= 0 && !self.visit_recursive(visitor, child, off, visited, error) {
                            return false;
                        }
                    }
                }
                true
            }
        }
    }

    /// Returns the raw weight of `cell_id` from the tree.
    pub fn get_cell_weight(
        &self,
        cell_id: CellId,
        path: &mut DecodedPath,
        error: &mut S2Error,
    ) -> i64 {
        *error = S2Error::ok();
        path.get_cell(cell_id, error).weight()
    }

    /// Returns the normalized weight of `cell_id` (proportional to ancestors).
    pub fn get_normal_cell_weight(
        &self,
        cell_id: CellId,
        path: &mut DecodedPath,
        error: &mut S2Error,
    ) -> i64 {
        *error = S2Error::ok();
        let cell_weight = path.get_cell(cell_id, error).weight();
        if !error.is_ok() || cell_weight == 0 {
            return 0;
        }
        Self::normal_weight_impl(cell_id, cell_weight, path, error)
    }

    fn normal_weight_impl(
        cell_id: CellId,
        cell_weight: i64,
        path: &mut DecodedPath,
        error: &mut S2Error,
    ) -> i64 {
        let mut scale = 1.0f64;
        let mut cid = cell_id;
        let mut cw = cell_weight;
        while !cid.is_face() {
            let pid = cid.parent();
            let parent_weight = path.get_cell(pid, error).weight();
            if !error.is_ok() || parent_weight == 0 {
                break;
            }
            if !pid.is_leaf() {
                let children = pid.children();
                let mut sib_sum: i64 = 0;
                for &child in &children {
                    sib_sum += path.get_cell(child, error).weight();
                    if !error.is_ok() {
                        return 0;
                    }
                }
                if sib_sum > 0 {
                    scale *= cw as f64 / sib_sum as f64;
                }
            }
            cw = parent_weight;
            cid = pid;
        }
        (scale * cw as f64).round() as i64
    }

    /// Partitions the tree's cells into groups whose normalized weights
    /// are approximately bounded by `max_weight`.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn get_partitioning(&self, max_weight: i64) -> Result<Vec<CellUnion>, S2Error> {
        let mut error = S2Error::ok();
        let target_weight = max_weight / 16;
        let mut path = DecodedPath::new(self);

        let mut candidate_ids: Vec<CellId> = Vec::new();
        self.visit_cells_inner(
            |cid, cell| {
                if cell.weight() > target_weight && cell.has_children() {
                    VisitAction::EnterCell
                } else {
                    candidate_ids.push(cid);
                    VisitAction::SkipCell
                }
            },
            &mut error,
        );
        if !error.is_ok() {
            return Err(error);
        }

        let mut nodes: BTreeMap<CellId, i64> = BTreeMap::new();
        for &cand_id in &candidate_ids {
            let mut nid = cand_id;
            if let Some((&last, _)) = nodes.last_key_value()
                && last.intersects(nid)
            {
                continue;
            }
            // Move up past pointless splits.
            loop {
                if nid.is_face() {
                    break;
                }
                let pid = nid.parent();
                let parent_weight = path.get_cell(pid, &mut error).weight();
                if parent_weight == 0 {
                    break;
                }
                let pc_children = pid.children();
                let mut all_same = true;
                let mut wcount = 0u32;
                for &child in &pc_children {
                    let cw = path.get_cell(child, &mut error).weight();
                    if cw > 0 {
                        wcount += 1;
                        if cw != parent_weight {
                            all_same = false;
                        }
                    }
                }
                if wcount <= 1 || !all_same {
                    break;
                }
                nid = pid;
                while let Some((&last, _)) = nodes.last_key_value() {
                    if last.intersects(nid) {
                        nodes.remove(&last);
                    } else {
                        break;
                    }
                }
            }
            let nw = self.get_normal_cell_weight(nid, &mut path, &mut error);
            nodes.insert(nid, nw);

            // Try replacing children with parent.
            let mut cur = nid;
            loop {
                if cur.is_face() {
                    break;
                }
                let pid = cur.parent();
                let pnw = self.get_normal_cell_weight(pid, &mut path, &mut error);
                if pnw >= max_weight / 4 {
                    break;
                }
                let pc_children = pid.children();
                let mut wcount = 0;
                for &child in &pc_children {
                    if path.get_cell(child, &mut error).weight() > 0 {
                        wcount += 1;
                    }
                }
                if wcount <= 1 {
                    break;
                }
                let all_present = pc_children.iter().all(|&child| {
                    path.get_cell(child, &mut error).weight() == 0 || nodes.contains_key(&child)
                });
                if !all_present {
                    break;
                }
                for &child in &pc_children {
                    if path.get_cell(child, &mut error).weight() > 0 {
                        nodes.remove(&child);
                    }
                }
                nodes.insert(pid, pnw);
                cur = pid;
            }
        }

        let mut partitions = Vec::new();
        let mut cover = Vec::new();
        let mut cw: i64 = 0;
        for (&nid, &nw) in &nodes {
            if !cover.is_empty() && cw + nw >= max_weight {
                partitions.push(CellUnion::from_verbatim(std::mem::take(&mut cover)));
                cw = 0;
            }
            cover.push(nid);
            cw += nw;
        }
        if !cover.is_empty() {
            partitions.push(CellUnion::from_verbatim(cover));
        }
        Ok(partitions)
    }

    /// Decodes the tree into a map of cell → weight.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn decode(&self) -> Result<BTreeMap<CellId, i64>, S2Error> {
        let mut w = BTreeMap::new();
        let mut error = S2Error::ok();
        self.visit_cells_inner(
            |cid, cell| {
                w.insert(cid, cell.weight());
                VisitAction::EnterCell
            },
            &mut error,
        );
        if error.is_ok() { Ok(w) } else { Err(error) }
    }

    /// Returns a new tree where each cell's weight reflects its proportional
    /// share of the root weight (normalized).
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn normalize(&self) -> Result<S2DensityTree, S2Error> {
        let mut error = S2Error::ok();
        let mut path = DecodedPath::new(self);
        let mut weights: HashMap<CellId, i64> = HashMap::new();

        let mut visit_error = S2Error::ok();
        let ok = self.visit_cells_inner(
            |id, cell| {
                let mut w = i128::from(cell.weight());
                if !id.is_face() {
                    let parent = id.parent();
                    let children = parent.children();
                    let mut sibling_weight: i128 = 0;
                    for &child in &children {
                        let sc = path.get_cell(child, &mut visit_error);
                        if !visit_error.is_ok() {
                            return VisitAction::Stop;
                        }
                        sibling_weight += i128::from(sc.weight());
                    }
                    let pw = *weights.get(&parent).unwrap_or(&0);
                    if sibling_weight > 0 {
                        w = (w * i128::from(pw) - 1) / sibling_weight + 1;
                    }
                }
                weights.insert(id, w as i64);
                VisitAction::EnterCell
            },
            &mut error,
        );
        if !ok {
            let e = if visit_error.is_ok() {
                error
            } else {
                visit_error
            };
            return Err(e);
        }
        if !visit_error.is_ok() {
            return Err(visit_error);
        }

        let mut enc = TreeEncoder::new();
        for (&cid, &w) in &weights {
            enc.put(cid, w);
        }
        let mut tree = S2DensityTree::new();
        enc.build(&mut tree);
        Ok(tree)
    }

    /// Returns a `CellUnion` containing only the leaf cells of the tree.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn leaves(&self) -> Result<CellUnion, S2Error> {
        let mut ids = Vec::new();
        let mut error = S2Error::ok();
        self.visit_cells_inner(
            |cid, cell| {
                if cell.has_children() {
                    VisitAction::EnterCell
                } else {
                    ids.push(cid);
                    VisitAction::SkipCell
                }
            },
            &mut error,
        );
        if error.is_ok() {
            Ok(CellUnion::from_verbatim(ids))
        } else {
            Err(error)
        }
    }

    /// Returns a new tree dilated by `radius`, spreading weights to neighbors.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn dilate(
        tree: &S2DensityTree,
        radius: Angle,
        max_level_diff: u8,
    ) -> Result<S2DensityTree, S2Error> {
        let leaves = tree.leaves()?;

        let mut weights: HashMap<CellId, i64> = HashMap::new();
        // C++: kMinWidth.GetLevelForMinValue(radius.radians())
        let radius_level = MIN_WIDTH.max_level(radius.radians());

        let mut expanded = leaves.clone();
        expanded.expand_by_radius(radius, max_level_diff);
        let dilation_cells = expanded.difference(&leaves);

        let mut error = S2Error::ok();
        tree.visit_cells_inner(
            |cid, node| {
                let e = weights.entry(cid).or_insert(0);
                let dw = (*e).max(node.weight());
                *e = dw;
                if node.has_children() && cid.level() < radius_level {
                    return VisitAction::EnterCell;
                }
                let dl = radius_level.min(cid.level() + max_level_diff);
                if let Some(neighbors) = cid.all_neighbors(dl) {
                    for n in neighbors {
                        if !dilation_cells.intersects_cell_id(n) {
                            continue;
                        }
                        let mut cur = n;
                        loop {
                            let e = weights.entry(cur).or_insert(0);
                            if *e >= dw {
                                break;
                            }
                            *e = dw;
                            if cur.level() == 0 {
                                break;
                            }
                            cur = cur.parent();
                        }
                    }
                }
                VisitAction::SkipCell
            },
            &mut error,
        );

        if !error.is_ok() {
            return Err(error);
        }
        let mut enc = TreeEncoder::new();
        for (&cid, &w) in &weights {
            enc.put(cid, w);
        }
        let mut d = S2DensityTree::new();
        enc.build(&mut d);
        Ok(d)
    }

    /// Appends the encoded tree bytes to `out`.
    pub fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.encoded);
    }

    /// Initializes the tree from previously encoded bytes.
    ///
    /// # Errors
    ///
    /// Returns `Err(S2Error)` if the tree data is corrupt or decoding fails.
    pub fn init(&mut self, data: &[u8]) -> Result<(), S2Error> {
        let mut error = S2Error::ok();
        if data.is_empty() {
            self.encoded.clear();
            self.decoded_faces = [-1; NUM_FACES];
            return Ok(());
        }
        self.encoded = data.to_vec();
        self.decoded_faces = decode_header(data, &mut error);
        if error.is_ok() { Ok(()) } else { Err(error) }
    }
}

// ─── Header decoding ───────────────────────────────────────────────────

/// Decodes a varint starting at `data[pos]`. Returns `(value, next_pos)`.
#[inline]
fn decode_varint_at(data: &[u8], mut pos: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        if pos >= data.len() {
            return None;
        }
        let b = data[pos];
        pos += 1;
        result |= u64::from(b & 0x7F) << shift;
        if b < 0x80 {
            return Some((result, pos));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

/// Adds two `i64` offsets from untrusted decoded tree data, returning `Err` on
/// overflow instead of panicking (mirrors `add_i32` in `point_compression.rs`).
/// Offsets and their deltas come straight off the wire, so all arithmetic on
/// them must be overflow-safe.
fn add_i64(a: i64, b: i64) -> Result<i64, S2Error> {
    a.checked_add(b).ok_or_else(|| {
        S2Error::new(
            S2ErrorCode::InvalidArgument,
            "integer overflow in S2DensityTree decode",
        )
    })
}

fn decode_header(data: &[u8], error: &mut S2Error) -> [i64; NUM_FACES] {
    let mut faces = [-1i64; NUM_FACES];
    if data.len() < VERSION.len() {
        *error = S2Error::new(
            S2ErrorCode::InvalidArgument,
            "Not enough bytes for S2DensityTree header",
        );
        return faces;
    }
    if &data[..VERSION.len()] != VERSION {
        *error = S2Error::new(
            S2ErrorCode::InvalidArgument,
            "Bad magic value for S2DensityTree",
        );
        return faces;
    }
    let mut pos = VERSION.len();
    let Some((bits, next_pos)) = decode_varint_at(data, pos) else {
        *error = S2Error::new(S2ErrorCode::InvalidArgument, "Failed to decode face mask");
        return faces;
    };
    pos = next_pos;
    let face_mask = bits as u8;
    let num_set = face_mask.count_ones() as usize;
    let mut cum: i64 = 0;
    let mut coded: [(usize, i64); NUM_FACES] = [(0, 0); NUM_FACES];
    let mut coded_len = 0usize;
    for f in 0..NUM_FACES {
        if face_mask & (1 << f) != 0 {
            coded[coded_len] = (f, cum);
            coded_len += 1;
            if coded_len < num_set {
                let Some((v, next_pos)) = decode_varint_at(data, pos) else {
                    *error = S2Error::new(S2ErrorCode::Internal, "Failed to decode face length");
                    return faces;
                };
                let Some(next_cum) = cum.checked_add(v as i64) else {
                    *error = S2Error::new(
                        S2ErrorCode::InvalidArgument,
                        "integer overflow in S2DensityTree face length",
                    );
                    return faces;
                };
                cum = next_cum;
                pos = next_pos;
            }
        }
    }
    for &(face, off) in &coded[..coded_len] {
        let Some(abs) = (pos as i64).checked_add(off) else {
            *error = S2Error::new(
                S2ErrorCode::InvalidArgument,
                "integer overflow in S2DensityTree face offset",
            );
            return faces;
        };
        if abs < 0 || abs as usize >= data.len() {
            *error = S2Error::new(
                S2ErrorCode::InvalidArgument,
                "S2DensityTree face offset out of range",
            );
            return faces;
        }
        faces[face] = abs;
    }
    faces
}

fn varint_length64(mut v: u64) -> usize {
    let mut len = 1;
    while v >= 0x80 {
        v >>= 7;
        len += 1;
    }
    len
}

// ─── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "density_tree_tests.rs"]
mod density_tree_tests;

#[cfg(test)]
#[expect(clippy::print_stderr, reason = "test diagnostics")]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::cell_id;
    use crate::s2::coords::{Face, Level};
    use crate::s2::point_vector::PointVector;
    use crate::s2::text_format;

    fn make_point_index(points: &[Point]) -> ShapeIndex {
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(points.to_vec());
        index.add(Box::new(pv));
        index.build();
        index
    }

    fn sum_to_root(bases: &BTreeMap<CellId, i64>) -> BTreeMap<CellId, i64> {
        let mut sum = BTreeMap::new();
        for (&cell, &weight) in bases {
            for level in (0..=cell.level().as_u8()).map(Level::new) {
                *sum.entry(cell.parent_at_level(level)).or_insert(0) += weight;
            }
        }
        sum
    }

    fn tree_cells(tree: &S2DensityTree, only_leaves: bool) -> Vec<CellId> {
        let mut ids = Vec::new();
        tree.visit_cells(|id, cell| {
            if !only_leaves || !cell.has_children() {
                ids.push(id);
            }
            VisitAction::EnterCell
        })
        .expect("visit_cells failed");
        ids
    }

    fn expect_trees_equal(got: &BTreeMap<CellId, i64>, want: &BTreeMap<CellId, i64>) -> bool {
        if got.len() != want.len() {
            eprintln!("size mismatch: got {}, wanted {}", got.len(), want.len());
            return false;
        }
        for (k, v) in want {
            match got.get(k) {
                Some(gv) if gv != v => {
                    eprintln!("value mismatch for {k:?}: got {gv}, want {v}");
                    return false;
                }
                None => {
                    eprintln!("missing key {k:?}");
                    return false;
                }
                _ => {}
            }
        }
        true
    }

    #[test]
    fn test_encode_empty() {
        let mut e = TreeEncoder::new();
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        assert!(t.decode().unwrap().is_empty());
    }

    #[test]
    fn test_encode_one_face() {
        let mut e = TreeEncoder::new();
        e.put(CellId::from_face(3), 17);
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = t.decode().unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(*d.get(&CellId::from_face(3)).unwrap(), 17);
    }

    #[test]
    fn test_encode_one_leaf() {
        let leaf = CellId::from_point(&Point::from_coords(0.0, 1.0, 0.0));
        let expected = sum_to_root(&BTreeMap::from([(leaf, 123)]));
        let mut e = TreeEncoder::new();
        for (&c, &w) in &expected {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        assert!(expect_trees_equal(&t.decode().unwrap(), &expected));
    }

    #[test]
    fn test_encode_each_face() {
        let mut expected = BTreeMap::new();
        for i in 0..6u8 {
            expected.insert(CellId::from_face(i), 10 + i64::from(i));
        }
        let mut e = TreeEncoder::new();
        for (&c, &w) in &expected {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        assert!(expect_trees_equal(&t.decode().unwrap(), &expected));
    }

    #[test]
    fn test_encode_one_branch() {
        let split = cell_id::from_face_ij(Face::F1, 1 << 10, 2 << 10).parent_at_level(10);
        let expected = sum_to_root(&BTreeMap::from([
            (split.child_begin_at_level(20), 1),
            (split.child_end_at_level(20), 17),
        ]));
        let mut e = TreeEncoder::new();
        for (&c, &w) in &expected {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        assert!(expect_trees_equal(&t.decode().unwrap(), &expected));
    }

    #[test]
    fn test_visitor_cancellation() {
        let index = make_point_index(&[text_format::parse_point("0:0")]);
        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10000, 30).unwrap();
        // VisitAction::Stop is not an error — visit_cells still returns Ok.
        t.visit_cells(|_, _| VisitAction::Stop).unwrap();
    }

    #[test]
    fn test_visit_uninitialized_tree() {
        let t = S2DensityTree::new();
        let mut n = 0;
        t.visit_cells(|_, _| {
            n += 1;
            VisitAction::EnterCell
        })
        .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_limits_to_max_weight() {
        let mut index = ShapeIndex::new();
        for p in [
            Point::from_coords(1.0, 2.0, 3.0).normalize(),
            Point::from_coords(1.0, 4.0, 9.0).normalize(),
            Point::from_coords(1.0, 6.0, 10.0).normalize(),
        ] {
            index.add(Box::new(PointVector::new(vec![p])));
        }
        index.build();
        let mut t = S2DensityTree::new();
        t.init_to_shape_density(&index, |_| MAX_WEIGHT, 10000, 30)
            .unwrap();
        for &w in t.decode().unwrap().values() {
            assert_eq!(w, MAX_WEIGHT);
        }
    }

    #[test]
    fn test_vertex_density_single_point() {
        let index = make_point_index(&[text_format::parse_point("0:0")]);
        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10000, 5).unwrap();
        assert!(!t.is_empty());
    }

    #[test]
    fn test_can_normalize_tree() {
        let mut points = Vec::new();
        for lat in (-80..=80).step_by(20) {
            for lng in (-170..=170).step_by(20) {
                points.push(text_format::parse_point(&format!("{lat}:{lng}")));
            }
        }
        let mut index = ShapeIndex::new();
        for p in &points {
            index.add(Box::new(PointVector::new(vec![*p])));
        }
        index.build();

        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10000, 20).unwrap();
        let normalized = t.normalize().unwrap();
        assert_eq!(tree_cells(&t, false), tree_cells(&normalized, false));

        let nd = normalized.decode().unwrap();
        for (&id, &weight) in &nd {
            if id.is_leaf() {
                continue;
            }
            let csum: i64 = id
                .children()
                .iter()
                .map(|c| nd.get(c).copied().unwrap_or(0))
                .sum();
            if csum > 0 {
                let np = id.children().iter().filter(|c| nd.contains_key(c)).count() as i64;
                assert!(
                    csum >= weight && csum <= weight + np.max(1),
                    "normalize mismatch at {id:?}: w={weight}, sum={csum}"
                );
            }
        }
    }

    #[test]
    fn test_normalize_balances() {
        let f0 = CellId::from_face(0);
        let leaves = BTreeMap::from([(f0, 3i64), (f0.children()[0], 2), (f0.children()[1], 4)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&leaves) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);

        let el = BTreeMap::from([(f0, 9i64), (f0.children()[0], 3), (f0.children()[1], 6)]);
        let mut ee = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&el) {
            ee.put(c, w);
        }
        let mut expected = S2DensityTree::new();
        ee.build(&mut expected);
        let n = t.normalize().unwrap();
        assert_eq!(tree_cells(&expected, false), tree_cells(&n, false));
    }

    #[test]
    fn test_normalize_disjoint() {
        let f0 = CellId::from_face(0);
        let leaves = BTreeMap::from([
            (f0.children()[0], 1i64),
            (f0.children()[1].children()[2], 1),
            (f0.children()[2], 1),
        ]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&leaves) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let n = t.normalize().unwrap();
        assert_eq!(tree_cells(&t, false), tree_cells(&n, false));
    }

    #[test]
    fn test_leaves_returns_leaves() {
        let index = make_point_index(&[text_format::parse_point("0:0")]);
        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10000, 10).unwrap();
        let leaves = t.leaves().unwrap();
        assert_eq!(leaves.cell_ids(), &tree_cells(&t, true));
    }

    #[test]
    fn test_decoded_path_scales() {
        let mut err = S2Error::ok();
        let parent = CellId::from_face_pos_level(0, 0, 5);
        let base = BTreeMap::from([(parent, 100i64)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&base) {
            e.put(c, w);
        }
        for i in 0..4 {
            e.put(parent.children()[i], 100);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let mut p = DecodedPath::new(&t);
        for i in 0..4 {
            let cid = parent.children()[i];
            assert_eq!(t.get_normal_cell_weight(cid, &mut p, &mut err), 25);
            assert_eq!(t.get_cell_weight(cid, &mut p, &mut err), 100);
        }
    }

    #[test]
    fn test_decoded_path_correctness() {
        let mut err = S2Error::ok();
        let f2 = CellId::from_face(2);
        let c22 = f2.children()[2];
        let base = BTreeMap::from([(c22.children()[2], 100i64), (c22.children()[3], 120)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&base) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let mut p = DecodedPath::new(&t);
        for f in 0..6u8 {
            if f == 2 {
                continue;
            }
            assert_eq!(p.get_cell(CellId::from_face(f), &mut err).weight(), 0);
        }
        assert_eq!(p.get_cell(f2, &mut err).weight(), 220);
        assert_eq!(p.get_cell(f2.children()[2], &mut err).weight(), 220);
        assert_eq!(p.get_cell(f2.children()[3], &mut err).weight(), 0);
        assert_eq!(p.get_cell(c22.children()[2], &mut err).weight(), 100);
        assert_eq!(p.get_cell(c22.children()[3], &mut err).weight(), 120);
    }

    #[test]
    fn test_sum_empty() {
        let mut t = S2DensityTree::new();
        t.init_to_sum_density(&[], 30).unwrap();
        assert!(t.decode().unwrap().is_empty());
    }

    #[test]
    fn test_sum_one() {
        let f1 = CellId::from_face(1);
        let mut e = TreeEncoder::new();
        e.put(f1, 3);
        e.put(f1.children()[1], 1);
        e.put(f1.children()[2], 2);
        let mut t1 = S2DensityTree::new();
        e.build(&mut t1);
        let mut s = S2DensityTree::new();
        s.init_to_sum_density(&[&t1], 30).unwrap();
        let expected = BTreeMap::from([(f1, 3i64), (f1.children()[1], 1), (f1.children()[2], 2)]);
        assert!(expect_trees_equal(&s.decode().unwrap(), &expected));
    }

    #[test]
    fn test_sum_disjoint() {
        let f2 = CellId::from_face(2);
        let f3 = CellId::from_face(3);
        let mut e1 = TreeEncoder::new();
        e1.put(f2, 4);
        let mut t1 = S2DensityTree::new();
        e1.build(&mut t1);
        let mut e2 = TreeEncoder::new();
        e2.put(f3, 2);
        e2.put(f3.children()[0], 2);
        let mut t2 = S2DensityTree::new();
        e2.build(&mut t2);
        let mut s = S2DensityTree::new();
        s.init_to_sum_density(&[&t1, &t2], 30).unwrap();
        let expected = BTreeMap::from([(f2, 4i64), (f3, 2), (f3.children()[0], 2)]);
        assert!(expect_trees_equal(&s.decode().unwrap(), &expected));
    }

    #[test]
    fn test_oversize_cells() {
        let mut e = TreeEncoder::new();
        for i in 0..6u8 {
            for (&c, &w) in &sum_to_root(&BTreeMap::from([(
                CellId::from_face_pos_level(i, 0, 10),
                1000i64,
            )])) {
                e.put(c, w);
            }
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let parts = t.get_partitioning(10).unwrap();
        assert_eq!(parts.len(), 6);
        for p in &parts {
            assert_eq!(p.num_cells(), 1);
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let index = make_point_index(&[text_format::parse_point("0:0")]);
        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10000, 10).unwrap();
        let mut buf = Vec::new();
        t.encode(&mut buf);
        let mut t2 = S2DensityTree::new();
        t2.init(&buf).unwrap();
        assert_eq!(t.decode().unwrap(), t2.decode().unwrap());
    }

    #[test]
    fn test_small_dilation() {
        let mut e = TreeEncoder::new();
        e.put(CellId::from_debug_string("1/").unwrap(), 4);
        e.put(CellId::from_debug_string("1/1").unwrap(), 2);
        e.put(CellId::from_debug_string("1/11").unwrap(), 2);
        e.put(CellId::from_debug_string("1/3").unwrap(), 2);
        e.put(CellId::from_debug_string("1/33").unwrap(), 2);
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, Angle::from_radians(1000.0 / 6_371_010.0), 0).unwrap();
        assert!(tree_cells(&d, false).len() > tree_cells(&t, false).len());
    }

    #[test]
    fn test_dilation_at_face_center() {
        let cw = BTreeMap::from([
            (CellId::from_token("0ffffffd5"), 1i64),
            (CellId::from_token("10000002b"), 1),
        ]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&cw) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, Angle::from_radians(300.0 / 6_371_010.0), 0).unwrap();
        assert!(tree_cells(&d, true).len() >= 4);
    }

    // ── Additional tests ported from C++ ────────────────────────────────

    #[test]
    fn test_encode_random_branches() {
        use crate::s2::testing::random_cell_id;
        use rand::SeedableRng;

        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        for weight in 1i64..100 {
            let mut expected = BTreeMap::new();
            for _ in 0..50 {
                expected.insert(random_cell_id(&mut rng), weight);
            }
            let expected = sum_to_root(&expected);
            let mut e = TreeEncoder::new();
            for (&c, &w) in &expected {
                e.put(c, w);
            }
            let mut t = S2DensityTree::new();
            e.build(&mut t);
            assert!(
                expect_trees_equal(&t.decode().unwrap(), &expected),
                "failed at weight={weight}"
            );
        }
    }

    #[test]
    fn test_normalize_does_not_overflow() {
        let f0 = CellId::from_face(0);
        let max32 = i64::from(i32::MAX);
        let max64 = i64::MAX;
        let leaves = BTreeMap::from([
            (f0.children()[1].children()[2], max32),
            (f0.children()[1].children()[3], max64 - max32 - 1),
            (f0.children()[2], 1),
        ]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&leaves) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let n = t.normalize().unwrap();
        assert_eq!(tree_cells(&t, false), tree_cells(&n, false));
    }

    #[test]
    fn test_decoded_path_random_descendants() {
        let mut err = S2Error::ok();
        use rand::{Rng, SeedableRng};

        let f2 = CellId::from_face(2);
        let c22 = f2.children()[2];
        let base = BTreeMap::from([(c22.children()[2], 100i64), (c22.children()[3], 120)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&base) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let mut p = DecodedPath::new(&t);

        let max_level = MAX_CELL_LEVEL;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(123);

        // Random descendants of a non-leaf cell with no children → weight 0.
        for _ in 0..100 {
            let mut id = f2.children()[3];
            let depth = rng.gen_range(0..=(max_level - id.level().as_u8() - 1));
            for _ in 0..depth {
                id = id.children()[rng.gen_range(0..4)];
            }
            assert_eq!(p.get_cell(id, &mut err).weight(), 0);
        }

        // Random descendants of a leaf cell → that leaf's weight.
        for _ in 0..100 {
            for (leaf, expected_w) in [(c22.children()[2], 100), (c22.children()[3], 120)] {
                let mut id = leaf;
                let depth = rng.gen_range(0..=(max_level - id.level().as_u8() - 1));
                for _ in 0..depth {
                    id = id.children()[rng.gen_range(0..4)];
                }
                assert_eq!(p.get_cell(id, &mut err).weight(), expected_w);
            }
        }
    }

    #[test]
    fn test_partitioning_removes_pointless_splits() {
        let parent = CellId::from_face_pos_level(0, 0, 4);
        let base = BTreeMap::from([(parent, 20i64)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&base) {
            e.put(c, w);
        }
        for i in 0..4 {
            e.put(parent.children()[i], 20);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let parts = t.get_partitioning(100).unwrap();
        for p in &parts {
            for &cell in p.cell_ids() {
                assert_eq!(cell.level(), 4);
            }
        }
    }

    #[test]
    fn test_partitioning_replaces_children_with_parent() {
        let f0cell = CellId::from_face_pos_level(0, 0, 4);
        let f1cell = CellId::from_face_pos_level(1, 0, 4);

        let base = BTreeMap::from([(f0cell, 20i64), (f1cell, 40)]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&base) {
            e.put(c, w);
        }
        // Face 0: children should merge (parent not too large).
        for i in 0..4 {
            e.put(f0cell.children()[i], 18);
        }
        // Face 1: children should NOT merge (parent too large).
        for i in 0..4 {
            e.put(f1cell.children()[i], 18);
        }

        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let parts = t.get_partitioning(100).unwrap();
        for p in &parts {
            for &cell in p.cell_ids() {
                if u8::from(cell.face()) == 0 {
                    assert_eq!(cell.level(), 4);
                } else if u8::from(cell.face()) == 1 {
                    assert_eq!(cell.level(), 5);
                } else {
                    panic!("unexpected face: {:?}", cell.face());
                }
            }
        }
    }

    // Helper: builds trees from specified roots for sum tests.
    fn build_sum_tree_fixture(
        weights: &BTreeMap<CellId, i64>,
        roots: &[CellId],
    ) -> Vec<S2DensityTree> {
        fn insert_subtree(enc: &mut TreeEncoder, weights: &BTreeMap<CellId, i64>, cell: CellId) {
            if let Some(&w) = weights.get(&cell) {
                enc.put(cell, w);
                if !cell.is_leaf() {
                    for &child in &cell.children() {
                        insert_subtree(enc, weights, child);
                    }
                }
            }
        }

        let mut trees = Vec::new();
        for &root in roots {
            let mut enc = TreeEncoder::new();
            insert_subtree(&mut enc, weights, root);
            if let Some(&root_w) = weights.get(&root) {
                let mut cur = root;
                while cur.level() > 0 {
                    cur = cur.parent();
                    enc.put(cur, root_w);
                }
            }
            let mut t = S2DensityTree::new();
            enc.build(&mut t);
            trees.push(t);
        }
        trees
    }

    fn sum_fixture_weights() -> BTreeMap<CellId, i64> {
        let f1 = CellId::from_face(1);
        let f2 = CellId::from_face(2);
        let f3 = CellId::from_face(3);
        BTreeMap::from([
            (f1, 3),
            (f1.children()[1], 1),
            (f1.children()[2], 2),
            (CellId::from_face_pos_level(1, 0, 30), 4),
            (f2, 4),
            (f3, 2),
            (f3.children()[0], 2),
            (CellId::from_face_pos_level(3, 0, 30), 2),
        ])
    }

    fn check_sum(
        expected: &BTreeMap<CellId, i64>,
        roots: &[CellId],
        max_level: u8,
        with_size_limit: bool,
    ) {
        let weights = sum_fixture_weights();
        let trees = build_sum_tree_fixture(&weights, roots);
        let tree_refs: Vec<&S2DensityTree> = trees.iter().collect();
        let mut sum_tree = S2DensityTree::new();
        if with_size_limit {
            sum_tree
                .init_to_sum_density_with_size(&tree_refs, 1000, max_level)
                .unwrap();
        } else {
            sum_tree.init_to_sum_density(&tree_refs, max_level).unwrap();
        }
        let decoded = sum_tree.decode().unwrap();
        assert!(
            expect_trees_equal(&decoded, expected),
            "with_size_limit={with_size_limit}"
        );
    }

    #[test]
    fn test_sum_nested() {
        let f1 = CellId::from_face(1);
        let expected = BTreeMap::from([(f1, 4i64), (f1.children()[1], 2), (f1.children()[2], 2)]);
        for with_size in [false, true] {
            check_sum(&expected, &[f1, f1.children()[1]], 30, with_size);
        }
    }

    #[test]
    fn test_sum_leaves() {
        let l1 = CellId::from_face_pos_level(1, 0, 30);
        let l3 = CellId::from_face_pos_level(3, 0, 30);
        let expected = sum_to_root(&BTreeMap::from([(l1, 4i64), (l3, 2)]));
        for with_size in [false, true] {
            check_sum(&expected, &[l1, l3], 30, with_size);
        }
    }

    #[test]
    fn test_sum_leaves_level_limited() {
        let l1 = CellId::from_face_pos_level(1, 0, 30);
        let l3 = CellId::from_face_pos_level(3, 0, 30);
        let expected = sum_to_root(&BTreeMap::from([
            (CellId::from_face_pos_level(1, 0, 20), 4i64),
            (CellId::from_face_pos_level(3, 0, 20), 2),
        ]));
        for with_size in [false, true] {
            check_sum(&expected, &[l1, l3], 20, with_size);
        }
    }

    #[test]
    fn test_sum_max_level() {
        let cell = CellId::from_face(5).children()[2].children()[1].children()[0];
        for max_level in 0..=cell.level().as_u8() {
            let mut b = BreadthFirstTreeBuilder::new(10000, max_level);
            let mut tree = S2DensityTree::new();
            let cell_copy = cell;
            b.build(|cid| Ok(i64::from(cid.intersects(cell_copy))), &mut tree)
                .unwrap();
            let actual = tree.decode().unwrap();
            let expected = sum_to_root(&BTreeMap::from([(cell.parent_at_level(max_level), 1i64)]));
            assert!(
                expect_trees_equal(&actual, &expected),
                "failed at max_level={max_level}"
            );
        }
    }

    #[test]
    fn test_sum_empty_and_non_empty() {
        let index = make_point_index(&[text_format::parse_point("0:0")]);
        let mut tree = S2DensityTree::new();
        tree.init_to_vertex_density(&index, 1000, 10).unwrap();

        let empty_tree = S2DensityTree::new();
        let trees = vec![&tree, &empty_tree];

        let mut sum_tree = S2DensityTree::new();
        sum_tree.init_to_sum_density(&trees, 10).unwrap();

        let decoded = tree.decode().unwrap();
        assert!(!decoded.is_empty());
        assert_eq!(decoded, sum_tree.decode().unwrap());
    }

    #[test]
    fn test_small_dilation_constrained_to_leaf_level() {
        use crate::s2::earth::meters_to_angle;

        let mut e = TreeEncoder::new();
        e.put(CellId::from_debug_string("1/").unwrap(), 4);
        e.put(CellId::from_debug_string("1/1").unwrap(), 2);
        e.put(CellId::from_debug_string("1/11").unwrap(), 2);
        e.put(CellId::from_debug_string("1/3").unwrap(), 2);
        e.put(CellId::from_debug_string("1/33").unwrap(), 2);
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, meters_to_angle(1000.0), 0).unwrap();

        let mut actual: Vec<String> = Vec::new();
        d.visit_cells(|cid, _| {
            actual.push(cid.to_debug_string());
            VisitAction::EnterCell
        })
        .unwrap();

        let mut expected: Vec<&str> = vec![
            "0/", "0/2", "0/22", "0/23", "1/", "1/1", "1/10", "1/11", "1/12", "1/13", "1/3",
            "1/30", "1/31", "1/32", "1/33", "2/", "2/0", "2/00", "2/01", "3/", "3/1", "3/10",
            "3/11", "5/", "5/1", "5/11", "5/12",
        ];
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_small_dilation_relative_to_leaf_size() {
        use crate::s2::earth::meters_to_angle;

        let mut e = TreeEncoder::new();
        e.put(CellId::from_debug_string("1/").unwrap(), 4);
        e.put(CellId::from_debug_string("1/1").unwrap(), 2);
        e.put(CellId::from_debug_string("1/11").unwrap(), 2);
        e.put(CellId::from_debug_string("1/3").unwrap(), 2);
        e.put(CellId::from_debug_string("1/33").unwrap(), 2);
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, meters_to_angle(1000.0), 1).unwrap();
        assert_eq!(24, tree_cells(&d, true).len());
    }

    #[test]
    fn test_dilation_uses_maximum() {
        use crate::s2::earth::meters_to_angle;

        let mut e1 = TreeEncoder::new();
        e1.put(CellId::from_token("3"), 10);
        e1.put(CellId::from_token("3c"), 2);
        e1.put(CellId::from_token("3d"), 2);
        e1.put(CellId::from_token("34"), 8);
        e1.put(CellId::from_token("31"), 8);
        let mut t1 = S2DensityTree::new();
        e1.build(&mut t1);

        let mut e2 = TreeEncoder::new();
        e2.put(CellId::from_token("3"), 10);
        e2.put(CellId::from_token("3c"), 8);
        e2.put(CellId::from_token("3d"), 8);
        e2.put(CellId::from_token("34"), 2);
        e2.put(CellId::from_token("31"), 2);
        let mut t2 = S2DensityTree::new();
        e2.build(&mut t2);
        let d1 = S2DensityTree::dilate(&t1, meters_to_angle(1000.0), 0).unwrap();
        let d2 = S2DensityTree::dilate(&t2, meters_to_angle(1000.0), 0).unwrap();

        let w1 = d1.decode().unwrap();
        let w2 = d2.decode().unwrap();
        let cell_3b = CellId::from_token("3b");
        assert_eq!(*w1.get(&cell_3b).unwrap(), 8, "tree1 3b weight");
        assert_eq!(*w2.get(&cell_3b).unwrap(), 8, "tree2 3b weight");
    }

    #[test]
    fn test_dilation_larger_than_leaf_size() {
        use crate::s2::earth::meters_to_angle;

        let mut e = TreeEncoder::new();
        e.put(CellId::from_debug_string("1/").unwrap(), 4);
        e.put(CellId::from_debug_string("1/1").unwrap(), 2);
        e.put(CellId::from_debug_string("1/11").unwrap(), 2);
        e.put(CellId::from_debug_string("1/111").unwrap(), 2);
        e.put(CellId::from_debug_string("1/1111").unwrap(), 2);
        e.put(CellId::from_debug_string("1/11111").unwrap(), 2);
        e.put(CellId::from_debug_string("1/13").unwrap(), 2);
        e.put(CellId::from_debug_string("1/133").unwrap(), 2);
        e.put(CellId::from_debug_string("1/1333").unwrap(), 2);
        e.put(CellId::from_debug_string("1/13333").unwrap(), 2);
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, meters_to_angle(1_000_000.0), 4).unwrap();

        let mut actual: Vec<String> = Vec::new();
        d.visit_cells(|cid, _| {
            actual.push(cid.to_debug_string());
            VisitAction::EnterCell
        })
        .unwrap();

        let mut expected: Vec<&str> = vec![
            "1/", "1/0", "1/02", "1/03", "1/1", "1/10", "1/11", "1/12", "1/13", "1/2", "1/20",
            "1/21", "1/3", "1/31", "3/", "3/1", "3/10", "3/11", "5/", "5/1", "5/11", "5/12",
        ];
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_dilation_at_face_center_exact() {
        use crate::s2::earth::meters_to_angle;

        let cw = BTreeMap::from([
            (CellId::from_token("0ffffffd5"), 1i64),
            (CellId::from_token("10000002b"), 1),
        ]);
        let mut e = TreeEncoder::new();
        for (&c, &w) in &sum_to_root(&cw) {
            e.put(c, w);
        }
        let mut t = S2DensityTree::new();
        e.build(&mut t);
        let d = S2DensityTree::dilate(&t, meters_to_angle(300.0), 0).unwrap();

        let mut actual: Vec<String> = tree_cells(&d, true).iter().map(|c| c.to_token()).collect();
        actual.sort_unstable();

        let mut expected: Vec<&str> = vec![
            "0fffffe5", "0fffffe3", "1000001d", "1000001b", "0ffffffb", "0ffffffd", "10000003",
            "10000005", "0ffffff9", "0fffffff", "10000001", "10000007",
        ];
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    // ── S2Coder roundtrip tests ─────────────────────────────────────────

    #[test]
    fn test_encode_decode_uninitialized() {
        let t = S2DensityTree::new();
        let mut buf = Vec::new();
        t.encode(&mut buf);
        assert!(buf.is_empty());

        let mut t2 = S2DensityTree::new();
        t2.init(&buf).unwrap();
        assert!(t2.decode().unwrap().is_empty());
    }

    #[test]
    fn test_s2coder_roundtrip_multitype() {
        // C++ uses: "0:0 | 1:1 | 2:2 | 3:3 | 4:4 # #"
        let index = text_format::make_index("0:0 | 1:1 | 2:2 | 3:3 | 4:4 # #");
        let mut t = S2DensityTree::new();
        t.init_to_vertex_density(&index, 10_000, 20).unwrap();

        let mut buf = Vec::new();
        t.encode(&mut buf);
        let mut t2 = S2DensityTree::new();
        t2.init(&buf).unwrap();
        assert_eq!(t.decode().unwrap(), t2.decode().unwrap());
    }

    // ── CoveringsTest ───────────────────────────────────────────────────

    use crate::s2::Loop;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::region::Region;
    use crate::s2::region_coverer::RegionCoverer;

    /// Builds a temporary `ShapeIndex` containing a single shape that is a
    /// copy of `shape` (by copying all its edges). This is used by
    /// `compute_weight` to test per-shape intersection, mirroring the C++
    /// `S2WrappedShape` pattern without needing shared ownership.
    fn single_shape_index(shape: &dyn Shape) -> ShapeIndex {
        let mut idx = ShapeIndex::new();
        match shape.dimension() {
            Dimension::Point => {
                // Point shapes: collect the first vertex of each edge.
                let mut pts = Vec::new();
                for i in 0..shape.num_edges() {
                    pts.push(shape.edge(i).v0);
                }
                idx.add(Box::new(PointVector::new(pts)));
            }
            Dimension::Polyline => {
                // Polyline shapes: rebuild each chain as a LaxPolyline.
                let mut all_pts = Vec::new();
                for c in 0..shape.num_chains() {
                    let chain = shape.chain(c);
                    for j in 0..chain.length {
                        let e = shape.chain_edge(c, j);
                        if j == 0 {
                            all_pts.push(e.v0);
                        }
                        all_pts.push(e.v1);
                    }
                }
                idx.add(Box::new(LaxPolyline::new(all_pts)));
            }
            Dimension::Polygon => {
                // Polygon shapes: rebuild each chain as a loop.
                let mut loops = Vec::new();
                for c in 0..shape.num_chains() {
                    let chain = shape.chain(c);
                    let mut pts = Vec::new();
                    for j in 0..chain.length {
                        pts.push(shape.chain_edge(c, j).v0);
                    }
                    loops.push(pts);
                }
                idx.add(Box::new(LaxPolygon::from_loops_owned(loops)));
            }
        }
        idx.build();
        idx
    }

    /// Computes the expected weight of a cell by summing over each shape
    /// that intersects it (via a per-shape temporary index).
    fn compute_weight(index: &ShapeIndex, weights: &[i64], cell_id: CellId) -> i64 {
        let mut sum: i64 = 0;
        for (shape_id, &w) in weights.iter().enumerate() {
            let Some(shape) = index.shape(shape_id as i32) else {
                continue;
            };
            let single = single_shape_index(shape);
            let region = ShapeIndexRegion::new(&single);
            if region.intersects_cell(&Cell::from(cell_id)) {
                sum += w;
            }
        }
        sum
    }

    /// Core `CoveringsTest` verifier. Matches C++ `CoveringsTest::CheckCoverings`.
    fn check_coverings(index: &ShapeIndex, weights: &[i64]) {
        let region = ShapeIndexRegion::new(index);
        let coverer = RegionCoverer::new().max_cells(64);
        let cover = coverer.covering(&region);

        let measure = IndexCellWeightFunction::new(index, |shape: &dyn Shape| {
            // Look up weight by matching shape pointer identity within the index.
            for (id, &w) in weights.iter().enumerate() {
                if let Some(s) = index.shape(id as i32)
                    && std::ptr::eq(s, shape)
                {
                    return w;
                }
            }
            0
        });

        // Verify cover cells have correct weight.
        for &cell_id in cover.cell_ids() {
            let w = compute_weight(index, weights, cell_id);
            let expected = if region.contains_cell(&Cell::from(cell_id)) {
                -w
            } else {
                w
            };
            let actual = measure.weigh_cell(cell_id).unwrap();
            assert_eq!(
                expected, actual,
                "cover cell {cell_id:?}: expected {expected}, got {actual}"
            );
        }

        // Verify complement cells are zero (unless on the border).
        let full_loop = Loop::full();
        let full_cover = coverer.covering(&full_loop.cap_bound());
        let index_cover = coverer.covering(&region);
        let complement = full_cover.difference(&index_cover);
        for &cell_id in complement.cell_ids() {
            let expected = if cover.intersects_cell_id(cell_id) {
                compute_weight(index, weights, cell_id)
            } else {
                0
            };
            let actual = measure.weigh_cell(cell_id).unwrap();
            assert_eq!(
                expected, actual,
                "complement cell {cell_id:?}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_coverings_empty() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(Loop::empty()));
        index.build();
        check_coverings(&index, &[1]);
    }

    #[test]
    fn test_coverings_point() {
        use crate::s2::testing::random_point;
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0xCAFE);

        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVector::new(vec![random_point(&mut rng)])));
        index.build();
        check_coverings(&index, &[1]);
    }

    #[test]
    fn test_coverings_line() {
        use crate::s2::earth::km_to_angle;
        use crate::s2::testing::{make_regular_points, random_point};
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0xBEEF);

        let center = random_point(&mut rng);
        let pts = make_regular_points(center, km_to_angle(1.0), 3);
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolyline::new(pts)));
        index.build();
        check_coverings(&index, &[1]);
    }

    #[test]
    fn test_coverings_polygon() {
        use crate::s2::earth::km_to_angle;
        use crate::s2::testing::{make_regular_points, random_point};
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0xDEAD);

        let center = random_point(&mut rng);
        let pts = make_regular_points(center, km_to_angle(1.0), 5);
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolygon::from_loops_owned(vec![pts])));
        index.build();
        check_coverings(&index, &[1]);
    }

    #[test]
    fn test_coverings_multiple() {
        use crate::s2::earth::km_to_angle;
        use crate::s2::testing::{make_regular_points, random_point};
        use rand::SeedableRng;
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0xFACE);

        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVector::new(vec![random_point(&mut rng)])));
        let poly_center = random_point(&mut rng);
        let poly_pts = make_regular_points(poly_center, km_to_angle(1.0), 5);
        index.add(Box::new(LaxPolygon::from_loops_owned(vec![poly_pts])));
        let line_center = random_point(&mut rng);
        let line_pts = make_regular_points(line_center, km_to_angle(1.0), 3);
        index.add(Box::new(LaxPolyline::new(line_pts)));
        index.build();
        check_coverings(&index, &[1, 2, 3]);
    }

    // ── InitToFeatureDensity test ───────────────────────────────────────

    #[test]
    fn test_init_to_feature_density() {
        // Matches C++ TEST(S2DensityTreeTest, InitToFeatureDensity).
        let p = crate::s2::LatLng::from_degrees(5.0, 5.0).to_point();
        let q = crate::s2::LatLng::from_degrees(-5.0, 5.0).to_point();

        let mut index = ShapeIndex::new();
        // Feature "TwoShapes" (weight 1): two point shapes at p and q.
        index.add(Box::new(PointVector::new(vec![p]))); // shape 0
        index.add(Box::new(PointVector::new(vec![q]))); // shape 1
        // Feature "OneShapes" (weight 5): one point shape at p.
        index.add(Box::new(PointVector::new(vec![p]))); // shape 2
        index.build();

        let feature_map = FeatureMap::from_shapes(
            index.num_shape_ids(),
            [
                (0, "TwoShapes", 1_i64),
                (1, "TwoShapes", 1),
                (2, "OneShapes", 5),
            ],
        );
        assert_eq!(feature_map.num_features(), 2);

        let mut tree = S2DensityTree::new();
        tree.init_to_feature_density(&index, &feature_map, 100, 1)
            .unwrap();

        let parsed = tree.decode().unwrap();

        // TwoShapes is NOT double counted: both shapes at p map to the same
        // feature, so its weight of 1 is counted only once.
        let cell_p = CellId::from(&p).parent_at_level(1);
        let cell_q = CellId::from(&q).parent_at_level(1);
        let face = CellId::from(&p).parent_at_level(0);

        assert_eq!(parsed[&face], 6, "face: TwoShapes(1) + OneShapes(5)");
        assert_eq!(
            parsed[&cell_p], 6,
            "cell_p: TwoShapes(1) + OneShapes(5), not 7"
        );
        assert_eq!(parsed[&cell_q], 1, "cell_q: TwoShapes(1) only");
    }

    #[test]
    fn test_feature_density_1_to_1() {
        // When every shape is its own feature, init_to_feature_density should
        // produce the same result as init_to_shape_density with weight 1.
        let index = text_format::make_index("0:0 | 1:1 | 2:2 # #");
        let feature_map = FeatureMap::from_shapes(
            index.num_shape_ids(),
            (0..index.num_shape_ids() as i32).map(|id| (id, id, 1_i64)),
        );

        let mut tree_feat = S2DensityTree::new();
        let mut tree_shape = S2DensityTree::new();

        tree_feat
            .init_to_feature_density(&index, &feature_map, 10_000, 20)
            .unwrap();
        tree_shape
            .init_to_shape_density(&index, |_| 1, 10_000, 20)
            .unwrap();

        assert_eq!(
            tree_feat.decode().unwrap(),
            tree_shape.decode().unwrap(),
            "1:1 feature mapping should match shape density"
        );
    }
}
