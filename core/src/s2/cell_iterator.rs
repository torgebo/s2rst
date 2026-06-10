// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! The abstract cell-iterator interface shared by cell-keyed containers.
//!
//! Corresponds to C++ `s2cell_iterator.h`. Any container whose contents are
//! keyed by a sorted, non-overlapping set of [`CellId`]s (`ShapeIndex`,
//! `CellIndex`, a future lazy `EncodedS2ShapeIndex`, ...) can expose a
//! [`CellIterator`], which lets joins and queries be written once, generically.
//!
//! The positioning primitives (`seek`, `next`, `prev`, ...) are required
//! methods; the `locate_*` helpers have default implementations (the port of
//! C++ `S2CellIterator::LocateImpl`) expressed in terms of those primitives.

use crate::s2::cell_id::CellId;
use crate::s2::point::Point;
use crate::s2::shape_index::CellRelation;

/// An iterator over a sorted set of non-overlapping `CellId`s.
///
/// Port of the abstract C++ `S2CellIterator`. Implementations must maintain
/// the invariant that the cell ids visited between [`begin`](Self::begin) and
/// [`finish`](Self::finish) are sorted, valid, and mutually disjoint.
pub trait CellIterator {
    /// The current cell id. Must not be called when [`done`](Self::done).
    fn cell_id(&self) -> CellId;

    /// True if the iterator is positioned past the last cell.
    fn done(&self) -> bool;

    /// Positions the iterator at the first cell (if any).
    fn begin(&mut self);

    /// Positions the iterator past the last cell.
    fn finish(&mut self);

    /// Advances to the next cell.
    fn next(&mut self);

    /// Moves to the previous cell; returns false (leaving the position
    /// unchanged) if already at the first cell.
    fn prev(&mut self) -> bool;

    /// Positions the iterator at the first cell with `cell_id() >= target`.
    /// (Such a cell may not exist, in which case the iterator is `done`.)
    fn seek(&mut self, target: CellId);

    /// The center of the current cell.
    fn center(&self) -> Point {
        debug_assert!(!self.done());
        self.cell_id().to_point()
    }

    /// Positions the iterator at the cell containing the given point and
    /// returns true, or returns false if no such cell exists.
    ///
    /// Default implementation: the port of C++ `LocateImpl`, valid for any
    /// conforming iterator.
    fn locate_point(&mut self, p: Point) -> bool {
        // Let I = cell_id(p), the leaf cell containing "p". If "target" is a
        // cell that contains I, then seeking to I will either position the
        // iterator at target itself (target.range_min() <= I), or just after
        // it, in which case stepping back finds it (target.range_max() >= I).
        let target = CellId::from_point(&p);
        self.seek(target);
        if !self.done() && self.cell_id().range_min() <= target {
            return true;
        }
        if self.prev() && self.cell_id().range_max() >= target {
            return true;
        }
        false
    }

    /// Positions the iterator at the cell containing `target` (returning
    /// [`CellRelation::Indexed`]), at the first of the cells subdividing
    /// `target` ([`CellRelation::Subdivided`]), or reports
    /// [`CellRelation::Disjoint`] (position undefined).
    ///
    /// Default implementation: the port of C++ `LocateImpl`.
    fn locate_cell_id(&mut self, target: CellId) -> CellRelation {
        self.seek(target.range_min());
        if !self.done() {
            // The iterator is positioned at the first cell >= range_min.
            if self.cell_id() >= target && self.cell_id().range_min() <= target {
                return CellRelation::Indexed;
            }
            if self.cell_id() <= target.range_max() {
                return CellRelation::Subdivided;
            }
        }
        if self.prev() && self.cell_id().range_max() >= target {
            return CellRelation::Indexed;
        }
        CellRelation::Disjoint
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal `CellIterator` over a plain sorted vec, exercising the
    /// DEFAULT locate_* implementations (the concrete `ShapeIndexIterator`
    /// overrides them with equivalent inline versions).
    struct VecIter {
        ids: Vec<CellId>,
        pos: usize,
    }

    impl CellIterator for VecIter {
        fn cell_id(&self) -> CellId {
            self.ids[self.pos]
        }
        fn done(&self) -> bool {
            self.pos >= self.ids.len()
        }
        fn begin(&mut self) {
            self.pos = 0;
        }
        fn finish(&mut self) {
            self.pos = self.ids.len();
        }
        fn next(&mut self) {
            self.pos += 1;
        }
        fn prev(&mut self) -> bool {
            if self.pos == 0 {
                return false;
            }
            self.pos -= 1;
            true
        }
        fn seek(&mut self, target: CellId) {
            self.pos = self.ids.partition_point(|&id| id < target);
        }
    }

    #[test]
    fn test_default_locate_point_and_cell_id() {
        // Four disjoint level-1 cells on face 0 plus face 2's full cell.
        let mut ids: Vec<CellId> = CellId::from_face(0).children().to_vec();
        ids.push(CellId::from_face(2));
        ids.sort();
        let mut it = VecIter {
            ids: ids.clone(),
            pos: 0,
        };

        // locate_point: the leaf at face 0's center is inside one child.
        let p = CellId::from_face(0).to_point();
        assert!(it.locate_point(p));
        assert!(it.cell_id().contains(CellId::from_point(&p)));

        // A point on face 4 is not covered.
        let q = CellId::from_face(4).to_point();
        assert!(!it.locate_point(q));

        // locate_cell_id: an indexed cell, a descendant, an ancestor, disjoint.
        assert_eq!(it.locate_cell_id(ids[0]), CellRelation::Indexed);
        assert_eq!(
            it.locate_cell_id(ids[0].children()[3]),
            CellRelation::Indexed,
            "a child of an indexed cell is contained (Indexed)"
        );
        assert_eq!(
            it.locate_cell_id(CellId::from_face(0)),
            CellRelation::Subdivided,
            "face 0 is subdivided into its four indexed children"
        );
        assert_eq!(
            it.locate_cell_id(CellId::from_face(4)),
            CellRelation::Disjoint
        );
    }
}
