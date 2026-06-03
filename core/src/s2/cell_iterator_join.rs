// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! `S2CellIteratorJoin`: performs an inner join on two [`ShapeIndex`] iterators.
//!
//! Finds all pairs of cells (one from each index) that overlap or are within
//! a given tolerance distance of each other. For each pair, a visitor closure
//! is called with references to the positioned [`CellRangeIterator`]s.
//!
//! Corresponds to C++ `s2cell_iterator_join.h`.

use crate::s1::ChordAngle;
use crate::s2::CellId;
use crate::s2::cell::Cell;
use crate::s2::cell_range_iterator::CellRangeIterator;
use crate::s2::cell_union::CellUnion;
use crate::s2::shape_index::{CellRelation, ShapeIndex};

/// Maximum number of cross-terms before we recurse.
const MAX_CROSS_PRODUCT: usize = 25;
const COVER_LIMIT: usize = MAX_CROSS_PRODUCT / 2;

/// Performs an inner join on two [`ShapeIndex`] iterators.
///
/// Finds all pairs of cells (one from each index) that overlap or are within
/// a given tolerance distance. For each pair, calls a visitor closure.
#[derive(Debug)]
pub struct S2CellIteratorJoin<'a> {
    index_a: &'a ShapeIndex,
    index_b: &'a ShapeIndex,
    tolerance: ChordAngle,
    /// Reusable storage for matched cells.
    matched_cells: Vec<Cell>,
}

impl<'a> S2CellIteratorJoin<'a> {
    /// Creates a new join with zero tolerance (exact overlap only).
    pub fn new(a: &'a ShapeIndex, b: &'a ShapeIndex) -> Self {
        Self::with_tolerance(a, b, ChordAngle::ZERO)
    }

    /// Creates a new join with the given tolerance distance.
    pub fn with_tolerance(a: &'a ShapeIndex, b: &'a ShapeIndex, tolerance: ChordAngle) -> Self {
        S2CellIteratorJoin {
            index_a: a,
            index_b: b,
            tolerance,
            matched_cells: Vec::new(),
        }
    }

    /// Executes the join, calling `visitor` for each pair of overlapping cells.
    ///
    /// The visitor receives references to the two [`CellRangeIterator`]s
    /// positioned at the matching cells. Return `true` to continue iterating,
    /// `false` to stop early.
    ///
    /// Returns `false` if the visitor ever returned `false`, `true` otherwise.
    pub fn join<F>(&mut self, mut visitor: F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>, &CellRangeIterator<'a>) -> bool,
    {
        if self.tolerance == ChordAngle::ZERO {
            self.exact_join(&mut visitor)
        } else {
            self.tolerant_join(&mut visitor)
        }
    }

    /// Performs an exact inner join (zero tolerance).
    fn exact_join<F>(&mut self, visitor: &mut F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>, &CellRangeIterator<'a>) -> bool,
    {
        let mut iter_a = CellRangeIterator::new(self.index_a);
        let mut iter_b = CellRangeIterator::new(self.index_b);

        while !iter_a.done() && !iter_b.done() {
            let order = iter_a.relation(&iter_b);
            match order {
                -1 => {
                    // A precedes B, seek A.
                    let target = iter_b.clone();
                    iter_a.seek_to(&target);
                }
                1 => {
                    // B precedes A, seek B.
                    let target = iter_a.clone();
                    iter_b.seek_to(&target);
                }
                0 => {
                    // Iterators overlap.
                    if !visitor(&iter_a, &iter_b) {
                        return false;
                    }

                    // Move the smaller cell forward.
                    let lsb_a = iter_a.id().lsb();
                    let lsb_b = iter_b.id().lsb();
                    match lsb_a.cmp(&lsb_b) {
                        std::cmp::Ordering::Less => iter_a.next(),
                        std::cmp::Ordering::Greater => iter_b.next(),
                        std::cmp::Ordering::Equal => {
                            // Same size cells that overlap must be the same cell.
                            iter_a.next();
                            iter_b.next();
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        true
    }

    /// Performs a tolerant join (non-zero tolerance).
    fn tolerant_join<F>(&mut self, visitor: &mut F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>, &CellRangeIterator<'a>) -> bool,
    {
        let covering_a = Self::compute_covering(self.index_a);
        let covering_b = Self::compute_covering(self.index_b);
        self.process_nearby(&covering_a, &covering_b, visitor)
    }

    /// Computes a coarse covering of the index's cell range.
    fn compute_covering(index: &ShapeIndex) -> Vec<Cell> {
        let mut iter = CellRangeIterator::new(index);
        if iter.done() {
            return vec![];
        }
        let min = iter.range_min();
        iter.finish();
        if !iter.prev() {
            return vec![];
        }
        let max = iter.range_max();

        let cu = CellUnion::from_range(min, max.next());
        cu.cell_ids().iter().map(|&id| Cell::from(id)).collect()
    }

    /// Filters cell pairs within tolerance and processes them.
    fn process_nearby<F>(&mut self, cells_a: &[Cell], cells_b: &[Cell], visitor: &mut F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>, &CellRangeIterator<'a>) -> bool,
    {
        let mut nearby_cells = Vec::new();
        for cell_a in cells_a {
            nearby_cells.clear();
            for cell_b in cells_b {
                if cell_a.distance_to_cell(*cell_b) <= self.tolerance {
                    nearby_cells.push(*cell_b);
                }
            }
            if !nearby_cells.is_empty() && !self.process_cell_pairs(cell_a, &nearby_cells, visitor)
            {
                return false;
            }
        }
        true
    }

    /// Processes a pair of cells known to be within tolerance.
    ///
    /// If the index coverage is small enough, reports pairs to the visitor.
    /// Otherwise subdivides and recurses.
    fn process_cell_pairs<F>(&mut self, cell_a: &Cell, cells_b: &[Cell], visitor: &mut F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>, &CellRangeIterator<'a>) -> bool,
    {
        let mut iter_a = CellRangeIterator::new(self.index_a);
        let num_covered_a = Self::estimate_covered_cells(&mut iter_a, cell_a.id());

        if num_covered_a == 0 {
            return true;
        }

        // If the A cell covers too many index cells, subdivide it.
        let cells_a_vec: Vec<Cell> = if num_covered_a >= COVER_LIMIT {
            cell_a
                .children()
                .map_or_else(|| vec![*cell_a], |c| c.to_vec())
        } else {
            vec![*cell_a]
        };

        // Scan cells_b: prune empty cells and subdivide large ones.
        let mut iter_b = CellRangeIterator::new(self.index_b);
        let mut subdivided = false;
        let mut subdivided_b: Vec<Cell> = Vec::new();
        for cell_b in cells_b {
            let num_covered_b = Self::estimate_covered_cells(&mut iter_b, cell_b.id());
            if num_covered_b == 0 {
                continue;
            }
            if num_covered_b < COVER_LIMIT {
                subdivided_b.push(*cell_b);
            } else {
                if let Some(children) = cell_b.children() {
                    subdivided_b.extend_from_slice(&children);
                } else {
                    subdivided_b.push(*cell_b);
                }
                subdivided = true;
            }
        }

        // If we had to subdivide, continue recursion.
        if num_covered_a >= COVER_LIMIT || subdivided {
            return self.process_nearby(&cells_a_vec, &subdivided_b, visitor);
        }

        // Both sides are small enough. Pair them up and report to visitor.

        // Pre-compute the B cells to avoid replicating work in the inner loop.
        self.matched_cells.clear();
        let mut scan_iter_b = CellRangeIterator::new(self.index_b);
        for cell_b in cells_b {
            Self::scan_cell_range(&mut scan_iter_b, cell_b.id(), |iter| {
                self.matched_cells.push(Cell::from(iter.id()));
                true
            });
        }

        let mut scan_iter_a = CellRangeIterator::new(self.index_a);
        let tolerance = self.tolerance;
        let matched_cells = &self.matched_cells;

        Self::scan_cell_range(&mut scan_iter_a, cell_a.id(), |iter_a_inner| {
            // Only process if the index cell's range_min is in this cell_a.
            if !cell_a.id().intersects(iter_a_inner.id().range_min()) {
                return true;
            }

            let sub_cell_a = Cell::from(iter_a_inner.id());

            let mut idx = 0;
            let mut success = true;
            for cell_b in cells_b {
                if !success {
                    break;
                }
                let mut visit_iter_b = CellRangeIterator::new(self.index_b);
                Self::scan_cell_range(&mut visit_iter_b, cell_b.id(), |iter_b_inner| {
                    if idx < matched_cells.len()
                        && sub_cell_a.distance_to_cell(matched_cells[idx]) <= tolerance
                        && !visitor(iter_a_inner, iter_b_inner)
                    {
                        success = false;
                        return false;
                    }
                    idx += 1;
                    true
                });
            }
            success
        })
    }

    /// Positions iterator and visits every position that intersects a given `CellId`.
    fn scan_cell_range<F>(iter: &mut CellRangeIterator<'a>, id: CellId, mut visitor: F) -> bool
    where
        F: FnMut(&CellRangeIterator<'a>) -> bool,
    {
        iter.locate_cell_id(id);
        while !iter.done() && iter.id().intersects(id) {
            if !visitor(iter) {
                return false;
            }
            iter.next();
        }
        true
    }

    /// Estimates how many index cells are covered by the given cell.
    fn estimate_covered_cells(iter: &mut CellRangeIterator<'_>, cell: CellId) -> usize {
        match iter.locate_cell_id(cell) {
            CellRelation::Disjoint => 0,
            CellRelation::Indexed => 1,
            CellRelation::Subdivided => {
                let end = cell.range_max();
                let mut matches = 0;
                while !iter.done() && iter.id() <= end {
                    matches += 1;
                    if matches > COVER_LIMIT {
                        return COVER_LIMIT;
                    }
                    iter.next();
                }
                matches
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Collect all cell IDs from a `ShapeIndex`.
    fn collect_cells(index: &ShapeIndex) -> Vec<CellId> {
        let mut cells = Vec::new();
        let mut it = CellRangeIterator::new(index);
        while !it.done() {
            cells.push(it.id());
            it.next();
        }
        cells
    }

    /// Brute-force exact overlapping pairs.
    fn brute_force_exact_pairs(a: &ShapeIndex, b: &ShapeIndex) -> Vec<(CellId, CellId)> {
        let cells_a = collect_cells(a);
        let cells_b = collect_cells(b);
        let mut pairs = Vec::new();
        for &ca in &cells_a {
            for &cb in &cells_b {
                if ca.intersects(cb) {
                    pairs.push((ca, cb));
                }
            }
        }
        pairs
    }

    #[test]
    fn test_exact_join_works() {
        // Build two overlapping fractal polygons with many cells.
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut f1 = S2Fractal::new(42);
        f1.level_for_approx_max_edges(200);
        let center = crate::s2::LatLng::from_degrees(0.0, 0.0).to_point();
        let loop1 = f1.make_loop_at(center, crate::s1::Angle::from_degrees(10.0));
        let poly1 = Polygon::from_loops(vec![loop1]);
        let index_a = poly1.shape_index();

        let mut f2 = S2Fractal::new(99);
        f2.level_for_approx_max_edges(200);
        let loop2 = f2.make_loop_at(center, crate::s1::Angle::from_degrees(8.0));
        let poly2 = Polygon::from_loops(vec![loop2]);
        let index_b = poly2.shape_index();

        // Get brute-force pairs.
        let brute = brute_force_exact_pairs(index_a, index_b);
        assert!(!brute.is_empty(), "expected some overlapping cells");

        // Run exact join.
        let mut join_pairs = Vec::new();
        S2CellIteratorJoin::new(index_a, index_b).join(|a, b| {
            // Every pair must actually overlap.
            assert!(a.id().intersects(b.id()), "non-overlapping pair");
            join_pairs.push((a.id(), b.id()));
            true
        });

        // Join should find exactly the brute-force pairs.
        assert_eq!(
            join_pairs.len(),
            brute.len(),
            "join found {} pairs, brute force found {}",
            join_pairs.len(),
            brute.len()
        );
        for (i, pair) in join_pairs.iter().enumerate() {
            assert_eq!(*pair, brute[i], "mismatch at index {i}");
        }
    }

    #[test]
    fn test_exact_false_join_returns_immediately() {
        // Two overlapping fractals — returning false should stop after 1 pair.
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut f1 = S2Fractal::new(42);
        f1.level_for_approx_max_edges(200);
        let center = crate::s2::LatLng::from_degrees(0.0, 0.0).to_point();
        let loop1 = f1.make_loop_at(center, crate::s1::Angle::from_degrees(10.0));
        let poly1 = Polygon::from_loops(vec![loop1]);
        let index_a = poly1.shape_index();

        let mut f2 = S2Fractal::new(99);
        f2.level_for_approx_max_edges(200);
        let loop2 = f2.make_loop_at(center, crate::s1::Angle::from_degrees(8.0));
        let poly2 = Polygon::from_loops(vec![loop2]);
        let index_b = poly2.shape_index();

        let mut count = 0;
        let result = S2CellIteratorJoin::new(index_a, index_b).join(|_a, _b| {
            count += 1;
            false
        });
        assert!(!result);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_tolerant_false_join_returns_immediately() {
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut f1 = S2Fractal::new(42);
        f1.level_for_approx_max_edges(200);
        let center = crate::s2::LatLng::from_degrees(0.0, 0.0).to_point();
        let loop1 = f1.make_loop_at(center, crate::s1::Angle::from_degrees(10.0));
        let poly1 = Polygon::from_loops(vec![loop1]);
        let index_a = poly1.shape_index();

        let mut f2 = S2Fractal::new(99);
        f2.level_for_approx_max_edges(200);
        let loop2 = f2.make_loop_at(center, crate::s1::Angle::from_degrees(8.0));
        let poly2 = Polygon::from_loops(vec![loop2]);
        let index_b = poly2.shape_index();

        let tolerance = ChordAngle::from_degrees(0.001);
        let mut count = 0;
        let result =
            S2CellIteratorJoin::with_tolerance(index_a, index_b, tolerance).join(|_a, _b| {
                count += 1;
                false
            });
        assert!(!result);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_exact_join_seeking_works() {
        // Create two indexes with non-overlapping regions plus some overlap.
        // This exercises the seeking logic (skipping gaps).
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        // Polygon A centered at (0, 0).
        let mut f1 = S2Fractal::new(7);
        f1.level_for_approx_max_edges(100);
        let c1 = crate::s2::LatLng::from_degrees(0.0, 0.0).to_point();
        let loop1 = f1.make_loop_at(c1, crate::s1::Angle::from_degrees(5.0));
        let poly1 = Polygon::from_loops(vec![loop1]);
        let index_a = poly1.shape_index();

        // Polygon B centered at (0, 4) — partial overlap with A.
        let mut f2 = S2Fractal::new(13);
        f2.level_for_approx_max_edges(100);
        let c2 = crate::s2::LatLng::from_degrees(0.0, 4.0).to_point();
        let loop2 = f2.make_loop_at(c2, crate::s1::Angle::from_degrees(5.0));
        let poly2 = Polygon::from_loops(vec![loop2]);
        let index_b = poly2.shape_index();

        // Run exact join and compare with brute force.
        let brute = brute_force_exact_pairs(index_a, index_b);

        let mut join_pairs = Vec::new();
        S2CellIteratorJoin::new(index_a, index_b).join(|a, b| {
            join_pairs.push((a.id(), b.id()));
            true
        });

        // There should be some pairs (overlap region) but not all cells matched.
        let cells_a = collect_cells(index_a);
        let cells_b = collect_cells(index_b);
        assert!(
            join_pairs.len() < cells_a.len() * cells_b.len(),
            "expected partial overlap, not full cross product"
        );
        assert!(!join_pairs.is_empty(), "expected some overlap");
        assert_eq!(join_pairs.len(), brute.len());
    }

    #[test]
    fn test_near_join_works() {
        // Self-join with tolerance — all brute-force pairs should be found.
        // This uses the same pattern as test_all_pairs_seen but with different params.
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut fractal = S2Fractal::new(77);
        fractal.level_for_approx_max_edges(200);
        let center = crate::s2::LatLng::from_degrees(10.0, 20.0).to_point();
        let loop_ = fractal.make_loop_at(center, crate::s1::Angle::from_degrees(5.0));
        let polygon = Polygon::from_loops(vec![loop_]);
        let index = polygon.shape_index();

        let tolerance = ChordAngle::from_degrees(2.0);

        // Brute force: find all pairs within tolerance.
        let cells = collect_cells(index);
        let mut brute_pairs: HashSet<(CellId, CellId)> = HashSet::new();
        for &ca in &cells {
            for &cb in &cells {
                if Cell::from(ca).distance_to_cell(Cell::from(cb)) < tolerance {
                    brute_pairs.insert((ca, cb));
                }
            }
        }

        // Run tolerant join.
        let mut join_pairs: HashSet<(CellId, CellId)> = HashSet::new();
        S2CellIteratorJoin::with_tolerance(index, index, tolerance).join(|a, b| {
            join_pairs.insert((a.id(), b.id()));
            true
        });

        // All brute-force tolerant pairs should be found.
        assert_eq!(
            join_pairs,
            brute_pairs,
            "join pairs ({}) != brute force pairs ({})",
            join_pairs.len(),
            brute_pairs.len()
        );

        // There should be tolerant-only pairs (not just exact overlap).
        let exact_count = brute_force_exact_pairs(index, index).len();
        assert!(
            join_pairs.len() > exact_count,
            "expected tolerant pairs ({}) > exact pairs ({})",
            join_pairs.len(),
            exact_count
        );
    }

    #[test]
    fn test_tolerant_join_is_left_driven() {
        // Build a fractal polygon that spans face boundaries.
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut fractal = S2Fractal::new(42);
        fractal.level_for_approx_max_edges(100);

        let center = crate::s2::LatLng::from_degrees(0.0, -45.0).to_point();
        let loop_ = fractal.make_loop_at(center, crate::s1::Angle::from_degrees(10.0));
        let polygon = Polygon::from_loops(vec![loop_]);
        let index = polygon.shape_index();

        let tolerance = ChordAngle::from_degrees(2.0);
        let mut cells_seen = HashSet::new();
        let mut curr_cell = CellId::sentinel();

        S2CellIteratorJoin::with_tolerance(index, index, tolerance).join(|a, _b| {
            if a.id() == curr_cell {
                true
            } else {
                let is_new = !cells_seen.contains(&a.id());
                assert!(is_new, "A cell {:?} seen non-contiguously", a.id());
                curr_cell = a.id();
                cells_seen.insert(curr_cell);
                is_new
            }
        });
    }

    #[test]
    fn test_all_pairs_seen() {
        // Build a fractal polygon and verify that the join returns all pairs
        // within tolerance (comparing against brute force).
        use crate::s2::fractal::S2Fractal;
        use crate::s2::polygon::Polygon;

        let mut fractal = S2Fractal::new(123);
        fractal.level_for_approx_max_edges(1000);

        let center = crate::s2::LatLng::from_degrees(0.0, -45.0).to_point();
        let loop_ = fractal.make_loop_at(center, crate::s1::Angle::from_degrees(10.0));
        let polygon = Polygon::from_loops(vec![loop_]);
        let index = polygon.shape_index();

        // Collect all cells from the index.
        let mut cells = Vec::new();
        let mut it = CellRangeIterator::new(index);
        while !it.done() {
            cells.push(Cell::from(it.id()));
            it.next();
        }

        let tolerance = ChordAngle::from_degrees(2.0);

        // Build brute-force pairs.
        let mut brute_pairs: HashSet<(CellId, CellId)> = HashSet::new();
        for c0 in &cells {
            for c1 in &cells {
                if c0.distance_to_cell(*c1) < tolerance {
                    brute_pairs.insert((c0.id(), c1.id()));
                }
            }
        }

        // Build join pairs.
        let mut join_pairs: HashSet<(CellId, CellId)> = HashSet::new();
        S2CellIteratorJoin::with_tolerance(index, index, tolerance).join(|a, b| {
            join_pairs.insert((a.id(), b.id()));
            true
        });

        assert_eq!(
            join_pairs,
            brute_pairs,
            "join pairs ({}) != brute force pairs ({})",
            join_pairs.len(),
            brute_pairs.len()
        );
    }

    #[test]
    fn test_regression_b299938257() {
        // Regression: join wasn't checking for end of iterator before
        // dereferencing to check cell ID.
        use crate::r3::Vector;
        use crate::s2::Point;
        use crate::s2::fractal::S2Fractal;
        use crate::s2::point_vector::PointVector;
        use crate::s2::polygon::Polygon;

        let points = vec![
            Point(Vector::new(
                0.998782953991165789,
                -0.034851647907011431,
                -0.034899476426537568,
            )),
            Point(Vector::new(
                1.000000000000000000,
                -0.000000000000005489,
                -0.000000000000005494,
            )),
            Point(Vector::new(
                0.998782953991165789,
                -0.034851647907011431,
                0.034899476426537568,
            )),
            Point(Vector::new(
                1.000000000000000000,
                -0.000000000000005489,
                0.000000000000005494,
            )),
        ];

        let mut point_index = ShapeIndex::new();
        point_index.add(Box::new(PointVector::new(points)));
        point_index.build();

        let center = crate::s2::LatLng::from_degrees(0.0, 0.0).to_point();
        let mut fractal = S2Fractal::new(99);
        fractal.level_for_approx_max_edges(100);

        let loop_ = fractal.make_loop_at(center, crate::s1::Angle::from_degrees(1.0));
        let polygon = Polygon::from_loops(vec![loop_]);
        let poly_index = polygon.shape_index();

        let tolerance = ChordAngle::from_degrees(0.5);
        let mut count = 0;
        S2CellIteratorJoin::with_tolerance(poly_index, &point_index, tolerance).join(|_a, _b| {
            count += 1;
            true
        });

        // We should see some pairs within range.
        assert!(count > 0, "expected some pairs, got 0");
    }
}
