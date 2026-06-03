// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Index for (`CellId`, label) pairs with efficient range lookups.
//!
//! [`CellIndex`] stores a collection of `(CellId, label)` pairs and
//! organizes them into non-overlapping leaf cell ranges covering the
//! sphere. Iterators allow efficient range and contents queries.
//!
//! Corresponds to C++ `s2cell_index.h`, Go `s2/cell_index.go`.

#![expect(
    clippy::cast_sign_loss,
    reason = "cell index (i32) used as Vec indices"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "cell index values (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for cell index — always in range"
)]
use crate::s2::coords::MAX_CELL_LEVEL;
use crate::s2::{CellId, CellUnion};

/// Special label indicating the contents iterator is done.
const DONE_CONTENTS: i32 = -1;

/// A node in the cell tree. Cells are organized so that ancestors
/// of a given node contain that node.
#[derive(Clone, Debug)]
struct CellIndexNode {
    cell_id: CellId,
    label: i32,
    parent: i32,
}

impl Default for CellIndexNode {
    fn default() -> Self {
        CellIndexNode {
            cell_id: CellId::none(),
            label: DONE_CONTENTS,
            parent: -1,
        }
    }
}

/// A range of leaf `CellIds`. The range starts at `start_id` and ends
/// at the `start_id` of the next `RangeNode`.
#[derive(Clone, Debug)]
struct RangeNode {
    start_id: CellId,
    contents: i32,
}

/// Stores a collection of `(CellId, label)` pairs for efficient range queries.
///
/// The `CellIds` may overlap or contain duplicates. Each pair receives a
/// non-negative `i32` label, typically used to map query results back to
/// client data.
///
/// # Usage
///
/// ```ignore
/// let mut index = CellIndex::new();
/// index.add(cell_id_1, 0);
/// index.add(cell_id_2, 1);
/// index.build();
/// ```
///
/// After [`build`](CellIndex::build), use [`CellIndexRangeIterator`] and
/// [`CellIndexContentsIterator`] to query the index.
#[derive(Debug)]
pub struct CellIndex {
    cell_tree: Vec<CellIndexNode>,
    range_nodes: Vec<RangeNode>,
}

impl CellIndex {
    /// Creates a new empty `CellIndex`.
    pub fn new() -> Self {
        CellIndex {
            cell_tree: Vec::new(),
            range_nodes: Vec::new(),
        }
    }

    /// Adds a `(CellId, label)` pair to the index.
    ///
    /// # Panics
    ///
    /// Panics if `label` is negative.
    pub fn add(&mut self, id: CellId, label: i32) {
        debug_assert!(id.is_valid());
        assert!(label >= 0, "labels must be non-negative");
        self.cell_tree.push(CellIndexNode {
            cell_id: id,
            label,
            parent: -1,
        });
    }

    /// Adds all cells from a `CellUnion` with the same label.
    pub fn add_cell_union(&mut self, cu: &CellUnion, label: i32) {
        for &cell in cu {
            self.add(cell, label);
        }
    }

    /// Builds the index for use. Should only be called once.
    pub fn build(&mut self) {
        // Delta represents a push or pop instruction for the cell stack.
        struct Delta {
            start_id: CellId,
            cell_id: CellId,
            label: i32,
        }

        let sentinel = CellId::sentinel();
        let mut deltas = Vec::with_capacity(2 * self.cell_tree.len() + 2);

        // Create two deltas per (cellID, label): one to push, one to pop.
        for node in &self.cell_tree {
            deltas.push(Delta {
                start_id: node.cell_id.range_min(),
                cell_id: node.cell_id,
                label: node.label,
            });
            deltas.push(Delta {
                start_id: node.cell_id.range_max().next(),
                cell_id: sentinel,
                label: -1,
            });
        }

        // Sentinel deltas at the beginning and end of the CellId range.
        deltas.push(Delta {
            start_id: CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL),
            cell_id: CellId::none(),
            label: -1,
        });
        deltas.push(Delta {
            start_id: CellId::from_face(5).child_end_at_level(MAX_CELL_LEVEL),
            cell_id: CellId::none(),
            label: -1,
        });

        // Sort: by start_id, then reverse by cell_id (larger cells first),
        // then by label.
        deltas.sort_by(|a, b| {
            a.start_id
                .cmp(&b.start_id)
                .then(b.cell_id.cmp(&a.cell_id))
                .then(a.label.cmp(&b.label))
        });

        // Walk through deltas to build the cell tree and range nodes.
        self.cell_tree.clear();
        self.range_nodes.clear();
        let mut contents: i32 = -1;

        let mut i = 0;
        while i < deltas.len() {
            let start_id = deltas[i].start_id;
            // Process all deltas at the same start_id.
            while i < deltas.len() && deltas[i].start_id == start_id {
                if deltas[i].label >= 0 {
                    // Push: add to cell tree.
                    self.cell_tree.push(CellIndexNode {
                        cell_id: deltas[i].cell_id,
                        label: deltas[i].label,
                        parent: contents,
                    });
                    contents = (self.cell_tree.len() - 1) as i32;
                } else if deltas[i].cell_id == sentinel {
                    // Pop: restore parent.
                    contents = self.cell_tree[contents as usize].parent;
                }
                i += 1;
            }
            self.range_nodes.push(RangeNode { start_id, contents });
        }
    }
}

impl Default for CellIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Range Iterator ─────────────────────────────────────────────────────

/// Iterates over non-overlapping leaf cell ranges covering the sphere.
///
/// Use [`CellIndexContentsIterator`] to visit the `(CellId, label)` pairs
/// that intersect the current range.
#[derive(Debug)]
pub struct CellIndexRangeIterator<'a> {
    range_nodes: &'a [RangeNode],
    pos: usize,
    non_empty: bool,
}

impl<'a> CellIndexRangeIterator<'a> {
    /// Creates a range iterator. Initially unpositioned; call
    /// [`begin`](Self::begin) or [`seek`](Self::seek) before use.
    pub fn new(index: &'a CellIndex) -> Self {
        debug_assert!(!index.range_nodes.is_empty(), "Call build() first");
        CellIndexRangeIterator {
            range_nodes: &index.range_nodes,
            pos: 0,
            non_empty: false,
        }
    }

    /// Creates a non-empty range iterator that skips ranges with no contents.
    pub fn new_non_empty(index: &'a CellIndex) -> Self {
        debug_assert!(!index.range_nodes.is_empty(), "Call build() first");
        CellIndexRangeIterator {
            range_nodes: &index.range_nodes,
            pos: 0,
            non_empty: true,
        }
    }

    /// Returns the start `CellId` of the current range.
    /// If `done()`, returns `CellId::End(MAX_CELL_LEVEL)`.
    pub fn start_id(&self) -> CellId {
        self.range_nodes[self.pos].start_id
    }

    /// Returns the non-inclusive end `CellId` of the current range.
    ///
    /// Requires `!done()`.
    pub fn limit_id(&self) -> CellId {
        debug_assert!(!self.done());
        self.range_nodes[self.pos + 1].start_id
    }

    /// Returns true if the current range has no `(CellId, label)` pairs.
    pub fn is_empty(&self) -> bool {
        self.range_nodes[self.pos].contents == DONE_CONTENTS
    }

    /// Positions the iterator at the first range.
    pub fn begin(&mut self) {
        self.pos = 0;
        while self.non_empty && self.is_empty() && !self.done() {
            self.pos += 1;
        }
    }

    /// Moves to the previous range. Returns `false` if already at the beginning.
    pub fn prev(&mut self) -> bool {
        if self.non_empty {
            return self.non_empty_prev();
        }
        self.prev_inner()
    }

    fn prev_inner(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        true
    }

    fn non_empty_prev(&mut self) -> bool {
        while self.prev_inner() {
            if !self.is_empty() {
                return true;
            }
        }
        // Return iterator to its original position.
        if self.is_empty() && !self.done() {
            self.next();
        }
        false
    }

    /// Advances to the next range.
    pub fn next(&mut self) {
        debug_assert!(!self.done());
        self.pos += 1;
        while self.non_empty && self.is_empty() && !self.done() {
            self.pos += 1;
        }
    }

    /// Advances by `n` positions. Returns `false` if it would go past the end.
    pub fn advance(&mut self, n: usize) -> bool {
        if n >= self.range_nodes.len() - 1 - self.pos {
            return false;
        }
        self.pos += n;
        true
    }

    /// Positions the iterator past the end.
    pub fn finish(&mut self) {
        self.pos = self.range_nodes.len() - 1;
    }

    /// Returns true if positioned past the last valid range.
    pub fn done(&self) -> bool {
        self.pos >= self.range_nodes.len() - 1
    }

    /// Positions at the first range with `start_id >= target`.
    pub fn seek(&mut self, target: CellId) {
        // Binary search for the first range_node with start_id > target, then back up one.
        let found = self.range_nodes.partition_point(|rn| rn.start_id <= target);
        self.pos = if found > 0 { found - 1 } else { 0 };

        while self.non_empty && self.is_empty() && !self.done() {
            self.pos += 1;
        }
    }
}

impl Iterator for CellIndexRangeIterator<'_> {
    /// Yields `(start_id, limit_id)` pairs for each leaf cell range.
    type Item = (CellId, CellId);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let start = self.start_id();
        let limit = self.limit_id();
        // Advance using the inherent method (respects non_empty skipping).
        CellIndexRangeIterator::next(self);
        Some((start, limit))
    }
}

// ─── Contents Iterator ──────────────────────────────────────────────────

/// Visits `(CellId, label)` pairs that cover a set of leaf cell ranges.
///
/// When multiple ranges are visited in monotonically increasing order,
/// each pair is reported exactly once (duplicates are suppressed).
#[derive(Debug)]
pub struct CellIndexContentsIterator<'a> {
    cell_tree: &'a [CellIndexNode],
    node_cutoff: i32,
    next_node_cutoff: i32,
    prev_start_id: CellId,
    node: CellIndexNode,
}

impl<'a> CellIndexContentsIterator<'a> {
    /// Creates a contents iterator. Must call [`start_union`](Self::start_union) before use.
    pub fn new(index: &'a CellIndex) -> Self {
        CellIndexContentsIterator {
            cell_tree: &index.cell_tree,
            prev_start_id: CellId(0),
            node_cutoff: -1,
            next_node_cutoff: -1,
            node: CellIndexNode {
                cell_id: CellId::none(),
                label: DONE_CONTENTS,
                parent: -1,
            },
        }
    }

    /// Clears duplicate-suppression state.
    pub fn clear(&mut self) {
        self.prev_start_id = CellId(0);
        self.node_cutoff = -1;
        self.next_node_cutoff = -1;
        self.node.label = DONE_CONTENTS;
    }

    /// Returns the current `CellId`.
    pub fn cell_id(&self) -> CellId {
        debug_assert!(!self.done());
        self.node.cell_id
    }

    /// Returns the current label.
    pub fn label(&self) -> i32 {
        debug_assert!(!self.done());
        self.node.label
    }

    /// Advances to the next `(CellId, label)` pair in the current range.
    pub fn next(&mut self) {
        debug_assert!(!self.done());
        if self.node.parent <= self.node_cutoff {
            self.node_cutoff = self.next_node_cutoff;
            self.node.label = DONE_CONTENTS;
        } else {
            self.node = self.cell_tree[self.node.parent as usize].clone();
        }
    }

    /// Returns true if all pairs in the current range have been visited.
    pub fn done(&self) -> bool {
        self.node.label == DONE_CONTENTS
    }

    /// Positions at the first `(CellId, label)` pair covering the range
    /// pointed to by the given range iterator.
    pub fn start_union(&mut self, r: &CellIndexRangeIterator) {
        if r.start_id() < self.prev_start_id {
            self.node_cutoff = -1;
        }
        self.prev_start_id = r.start_id();

        let contents = r.range_nodes[r.pos].contents;
        if contents <= self.node_cutoff {
            self.node.label = DONE_CONTENTS;
        } else {
            self.node = self.cell_tree[contents as usize].clone();
        }

        self.next_node_cutoff = contents;
    }
}

impl Iterator for CellIndexContentsIterator<'_> {
    /// Yields `(CellId, label)` pairs for the current range.
    type Item = (CellId, i32);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let cell_id = self.cell_id();
        let label = self.label();
        // Advance using the inherent method.
        CellIndexContentsIterator::next(self);
        Some((cell_id, label))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::CellId;

    #[test]
    fn test_empty_index() {
        let mut index = CellIndex::new();
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        ri.begin();
        // Even an empty index has range nodes (sentinel values).
        // All ranges should be empty.
        while !ri.done() {
            assert!(ri.is_empty());
            ri.next();
        }
    }

    #[test]
    fn test_one_face_cell() {
        let mut index = CellIndex::new();
        let face0 = CellId::from_face(0);
        index.add(face0, 0);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        // Seek to face 0's range_min.
        ri.seek(face0.range_min());
        assert!(!ri.done());
        assert!(!ri.is_empty());

        // Contents should include our cell.
        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.cell_id(), face0);
        assert_eq!(ci.label(), 0);
        ci.next();
        assert!(ci.done());
    }

    #[test]
    fn test_one_leaf_cell() {
        let mut index = CellIndex::new();
        let leaf = CellId::from_face(3).child_begin_at_level(MAX_CELL_LEVEL);
        index.add(leaf, 42);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        ri.seek(leaf);
        assert!(!ri.done());
        assert!(!ri.is_empty());

        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.cell_id(), leaf);
        assert_eq!(ci.label(), 42);
    }

    #[test]
    fn test_duplicate_values() {
        let mut index = CellIndex::new();
        let cell = CellId::from_face(2);
        index.add(cell, 1);
        index.add(cell, 2);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        ri.seek(cell.range_min());
        assert!(!ri.is_empty());

        // Should find both labels.
        let mut labels = Vec::new();
        ci.start_union(&ri);
        while !ci.done() {
            assert_eq!(ci.cell_id(), cell);
            labels.push(ci.label());
            ci.next();
        }
        labels.sort_unstable();
        assert_eq!(labels, vec![1, 2]);
    }

    #[test]
    fn test_disjoint_cells() {
        let mut index = CellIndex::new();
        let c0 = CellId::from_face(0);
        let c5 = CellId::from_face(5);
        index.add(c0, 10);
        index.add(c5, 20);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        // Seek to face 0.
        ri.seek(c0.range_min());
        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.label(), 10);

        // Seek to face 5.
        ri.seek(c5.range_min());
        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.label(), 20);
    }

    #[test]
    fn test_nested_cells() {
        let mut index = CellIndex::new();
        let parent = CellId::from_face(1);
        let child = parent.children()[0];
        index.add(parent, 0);
        index.add(child, 1);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        // Seek to the child's range. Should find both parent and child.
        ri.seek(child.range_min());
        let mut labels = Vec::new();
        ci.start_union(&ri);
        while !ci.done() {
            labels.push(ci.label());
            ci.next();
        }
        labels.sort_unstable();
        assert_eq!(labels, vec![0, 1]);
    }

    #[test]
    fn test_non_empty_range_iterator() {
        let mut index = CellIndex::new();
        let face0 = CellId::from_face(0);
        index.add(face0, 0);
        index.build();

        let mut ri = CellIndexRangeIterator::new_non_empty(&index);
        ri.begin();

        // Should skip empty ranges and find the one non-empty range.
        assert!(!ri.done());
        assert!(!ri.is_empty());
    }

    #[test]
    fn test_range_iterator_finish() {
        let mut index = CellIndex::new();
        index.add(CellId::from_face(0), 0);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        ri.finish();
        assert!(ri.done());
    }

    #[test]
    fn test_range_iterator_prev() {
        let mut index = CellIndex::new();
        index.add(CellId::from_face(0), 0);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        ri.begin();
        // At beginning, prev should return false.
        assert!(!ri.prev());
        // Advance and then go back.
        ri.next();
        if !ri.done() {
            assert!(ri.prev());
        }
    }

    #[test]
    fn test_add_cell_union() {
        let mut index = CellIndex::new();
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0), CellId::from_face(1)]);
        index.add_cell_union(&cu, 99);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        // Both faces should be present with label 99.
        ri.seek(CellId::from_face(0).range_min());
        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.label(), 99);

        ci.clear();
        ri.seek(CellId::from_face(1).range_min());
        ci.start_union(&ri);
        assert!(!ci.done());
        assert_eq!(ci.label(), 99);
    }

    #[test]
    fn test_contents_iterator_dedup() {
        // When visiting ranges in monotonically increasing order,
        // contents should be reported exactly once.
        let mut index = CellIndex::new();
        let parent = CellId::from_face(0);
        index.add(parent, 0);
        index.build();

        let mut ri = CellIndexRangeIterator::new(&index);
        let mut ci = CellIndexContentsIterator::new(&index);

        // Visit multiple ranges covered by the same parent cell.
        let mut seen_count = 0;
        ri.begin();
        while !ri.done() {
            ci.start_union(&ri);
            while !ci.done() {
                if ci.label() == 0 {
                    seen_count += 1;
                }
                ci.next();
            }
            ri.next();
        }

        // With dedup, parent label should be seen exactly once.
        assert_eq!(seen_count, 1);
    }
}
