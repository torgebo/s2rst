// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! [`CellRangeIterator`] wraps a [`ShapeIndexIterator`] to cache the min/max
//! leaf cell ID ranges for the current cell.
//!
//! This enables efficient range-based spatial joins and comparisons.
//!
//! Corresponds to C++ `s2cell_range_iterator.h`.

use crate::s2::shape_index::{CellRelation, ShapeIndex, ShapeIndexCell, ShapeIndexIterator};
use crate::s2::{CellId, Point};

/// A wrapper around [`ShapeIndexIterator`] that caches the min/max leaf cell
/// ranges, enabling efficient spatial join operations.
#[derive(Clone, Debug)]
pub struct CellRangeIterator<'a> {
    it: ShapeIndexIterator<'a>,
    range_min: CellId,
    range_max: CellId,
}

impl<'a> CellRangeIterator<'a> {
    /// Creates a new range iterator for the given index, positioned at the
    /// first cell.
    pub fn new(index: &'a ShapeIndex) -> Self {
        let it = index.iter();
        let mut ri = CellRangeIterator {
            it,
            range_min: CellId::sentinel(),
            range_max: CellId::sentinel(),
        };
        ri.refresh();
        ri
    }

    /// Returns the current cell ID.
    pub fn id(&self) -> CellId {
        self.it.cell_id()
    }

    /// Returns the minimum leaf cell ID covered by the current cell.
    pub fn range_min(&self) -> CellId {
        self.range_min
    }

    /// Returns the maximum leaf cell ID covered by the current cell.
    pub fn range_max(&self) -> CellId {
        self.range_max
    }

    /// Returns true if the iterator is past the last cell.
    pub fn done(&self) -> bool {
        self.it.done()
    }

    /// Returns the current index cell, if any.
    pub fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        self.it.index_cell()
    }

    /// Positions at the first cell.
    pub fn begin(&mut self) {
        self.it.begin();
        self.refresh();
    }

    /// Advances to the next cell.
    pub fn next(&mut self) {
        self.it.next();
        self.refresh();
    }

    /// Moves to the previous cell. Returns false if already at the beginning.
    pub fn prev(&mut self) -> bool {
        let ok = self.it.prev();
        self.refresh();
        ok
    }

    /// Seeks to the first cell with ID >= `target`.
    pub fn seek(&mut self, target: CellId) {
        self.it.seek(target);
        self.refresh();
    }

    /// Positions past the last cell.
    pub fn finish(&mut self) {
        self.it.end();
        self.refresh();
    }

    /// Locates the cell containing the given point.
    pub fn locate_point(&mut self, target: Point) {
        self.it.locate_point(target);
        self.refresh();
    }

    /// Locates the cell containing or adjacent to the given cell ID.
    pub fn locate_cell_id(&mut self, target: CellId) -> CellRelation {
        let rel = self.it.locate_cell_id(target);
        self.refresh();
        rel
    }

    /// Seeks to the first cell whose range overlaps or follows `target`.
    ///
    /// If the current cell does not overlap "target", it is possible that the
    /// previous cell is the one we are looking for.  This can only happen when
    /// the previous cell contains "target" but has a smaller `S2CellId`.
    pub fn seek_to(&mut self, target: &CellRangeIterator<'_>) {
        self.it.seek(target.range_min);
        if (self.it.done() || self.it.cell_id().range_min() > target.range_max)
            && self.it.prev()
            && self.it.cell_id().range_max() < target.id()
        {
            self.it.next();
        }
        self.refresh();
    }

    /// Seeks to the first cell whose range starts after `target`.
    pub fn seek_beyond(&mut self, target: &CellRangeIterator<'_>) {
        self.it.seek(target.range_max.next());
        if !self.it.done() && self.it.cell_id().range_min() <= target.range_max {
            self.it.next();
        }
        self.refresh();
    }

    /// Compares the ranges of two iterators.
    ///
    /// Returns -1 if `self` precedes `other`, +1 if `self` follows, or 0
    /// if they overlap.
    pub fn relation(&self, other: &CellRangeIterator<'_>) -> i32 {
        if self.range_max < other.range_min {
            -1
        } else if self.range_min > other.range_max {
            1
        } else {
            0
        }
    }

    fn refresh(&mut self) {
        if self.it.done() {
            self.range_min = CellId::sentinel();
            self.range_max = CellId::sentinel();
        } else {
            let id = self.it.cell_id();
            self.range_min = id.range_min();
            self.range_max = id.range_max();
        }
    }
}

impl<'a> Iterator for CellRangeIterator<'a> {
    type Item = (CellId, &'a ShapeIndexCell);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let cell_id = self.id();
        let cell = self.index_cell()?;
        // Advance using the inherent method (updates cached ranges).
        CellRangeIterator::next(self);
        Some((cell_id, cell))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::point_vector::PointVector;
    use crate::s2::text_format;

    fn make_point_index(points: &[Point]) -> ShapeIndex {
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(points.to_vec());
        index.add(Box::new(pv));
        index.build();
        index
    }

    #[test]
    fn test_empty_index() {
        let index = ShapeIndex::new();
        let it = CellRangeIterator::new(&index);
        assert!(it.done());
    }

    #[test]
    fn test_single_point() {
        let p = text_format::parse_point("0:0");
        let index = make_point_index(&[p]);
        let mut it = CellRangeIterator::new(&index);

        assert!(!it.done());
        assert!(it.range_min() <= it.id());
        assert!(it.range_max() >= it.id());

        it.next();
        assert!(it.done());
    }

    #[test]
    fn test_multiple_cells() {
        // Use a polyline that spans multiple cells.
        let pl = text_format::make_polyline("0:0, 0:90, 90:0");
        let vertices: Vec<Point> = (0..pl.num_vertices()).map(|i| pl.vertex(i)).collect();
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolyline::new(vertices)));
        index.build();

        let mut it = CellRangeIterator::new(&index);
        let mut count = 0;
        let mut prev_max = CellId::from_face(0).range_min(); // sentinel-ish
        while !it.done() {
            // Each cell's range should be non-empty.
            assert!(it.range_min() <= it.range_max());
            count += 1;
            prev_max = it.range_max();
            it.next();
        }
        assert!(count > 1, "polyline should span multiple cells");
        let _ = prev_max;
    }

    #[test]
    fn test_relation() {
        let p1 = text_format::parse_point("0:0");
        let p2 = text_format::parse_point("0:90");
        let index1 = make_point_index(&[p1]);
        let index2 = make_point_index(&[p2]);

        let it1 = CellRangeIterator::new(&index1);
        let it2 = CellRangeIterator::new(&index2);

        // Two points on different faces should have non-overlapping ranges.
        let rel = it1.relation(&it2);
        assert!(rel != 0, "points on different faces should not overlap");
    }

    #[test]
    fn test_seek() {
        let pl = text_format::make_polyline("0:0, 0:90, 90:0");
        let vertices: Vec<Point> = (0..pl.num_vertices()).map(|i| pl.vertex(i)).collect();
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolyline::new(vertices)));
        index.build();

        let mut it = CellRangeIterator::new(&index);
        // Seek to face 3 — should position at or after face 3.
        let target = CellId::from_face(3);
        it.seek(target);
        if !it.done() {
            assert!(it.id() >= target);
        }
    }

    #[test]
    fn test_begin_and_finish() {
        let p = text_format::parse_point("0:0");
        let index = make_point_index(&[p]);
        let mut it = CellRangeIterator::new(&index);

        assert!(!it.done());
        it.finish();
        assert!(it.done());
        it.begin();
        assert!(!it.done());
    }
}
