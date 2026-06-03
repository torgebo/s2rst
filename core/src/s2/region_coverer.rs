// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Cell-based covering of spherical regions.
//!
//! [`RegionCoverer`] computes an approximation of a [`Region`] as a
//! union of [`CellId`]s. Two modes are available:
//!
//! - A **covering** completely contains the region (every point in the region
//!   is in some cell). This is useful as an over-approximation for spatial
//!   database queries — any query result must intersect the covering.
//! - An **interior covering** is completely contained by the region (every
//!   cell is inside the region). This is an under-approximation — any point
//!   in the interior covering is guaranteed to be inside the region.
//!
//! The number and size of cells is controlled by `max_cells`, `min_level`,
//! `max_level`, and `level_mod`. Increasing `max_cells` produces tighter
//! approximations at the cost of more cells.

#![expect(
    clippy::cast_sign_loss,
    reason = "level (i32) cast to u8 after clamping to 0..MAX_CELL_LEVEL"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "level (i32->u8) after clamping and excess (i64->usize)"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for level arithmetic — bounded by MAX_CELL_LEVEL"
)]
use std::collections::{BinaryHeap, HashSet};

use crate::s2::coords::{Level, MAX_CELL_LEVEL};
use crate::s2::{Cell, CellId, CellUnion, Point, Region};

// ─── Public Configuration ───────────────────────────────────────────────

/// Computes cell coverings and interior coverings for [`Region`]s.
///
/// Use the builder-style methods to configure, then call [`covering`](Self::covering) or
/// [`interior_covering`](Self::interior_covering).
///
/// # Examples
///
/// ```
/// use s2rst::s1::Angle;
/// use s2rst::s2::{Cap, LatLng};
/// use s2rst::s2::region_coverer::RegionCoverer;
///
/// let center = LatLng::from_degrees(40.0, -74.0).to_point();
/// let cap = Cap::from_center_angle(center, Angle::from_degrees(1.0));
///
/// let coverer = RegionCoverer::new()
///     .max_level(12)
///     .max_cells(8);
/// let covering = coverer.covering(&cap);
/// assert!(!covering.is_empty());
/// assert!(covering.len() <= 8);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RegionCoverer {
    /// Minimum cell level (default 0).
    pub min_level: Level,
    /// Maximum cell level (default `MAX_CELL_LEVEL`).
    pub max_level: Level,
    /// Level modulo: 1, 2, or 3. Controls branching factor (4, 16, or 64).
    /// Only cells where `(level - min_level) % level_mod == 0` are used.
    /// Default 1.
    pub level_mod: u8,
    /// Maximum desired cells in the approximation (default 8).
    pub max_cells: usize,
}

impl Default for RegionCoverer {
    fn default() -> Self {
        RegionCoverer {
            min_level: Level::MIN,
            max_level: Level::MAX,
            level_mod: 1,
            max_cells: 8,
        }
    }
}

impl RegionCoverer {
    /// Creates a new `RegionCoverer` with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the minimum cell level.
    pub fn min_level(mut self, level: impl Into<Level>) -> Self {
        self.min_level = level.into();
        self
    }

    /// Sets the maximum cell level.
    pub fn max_level(mut self, level: impl Into<Level>) -> Self {
        self.max_level = level.into();
        self
    }

    /// Sets the level modulo (1, 2, or 3).
    pub fn level_mod(mut self, modulo: u8) -> Self {
        self.level_mod = modulo.clamp(1, 3);
        self
    }

    /// Sets the maximum number of cells.
    pub fn max_cells(mut self, cells: usize) -> Self {
        self.max_cells = cells.max(1);
        self
    }

    /// Returns a normalized `CellUnion` that covers the given region.
    ///
    /// The covering respects all parameters. If `level_mod > 1`, the result
    /// is denormalized to ensure all cells satisfy the level constraints.
    pub fn covering(&self, region: &dyn Region) -> CellUnion {
        debug_assert!(self.min_level <= self.max_level);
        let mut c = Coverer::new(self);
        c.interior_covering = false;
        c.covering_internal(region);
        let mut cu = c.result;
        cu.normalize();
        if self.min_level > 0 || self.level_mod > 1 {
            cu = cu.denormalize(self.min_level, self.level_mod);
        }
        cu
    }

    /// Returns a normalized `CellUnion` contained within the given region.
    pub fn interior_covering(&self, region: &dyn Region) -> CellUnion {
        debug_assert!(self.min_level <= self.max_level);
        let mut c = Coverer::new(self);
        c.interior_covering = true;
        c.covering_internal(region);
        let mut cu = c.result;
        cu.normalize();
        if self.min_level > 0 || self.level_mod > 1 {
            cu = cu.denormalize(self.min_level, self.level_mod);
        }
        cu
    }

    /// Returns a `CellUnion` that covers the region, without denormalization.
    pub fn cell_union(&self, region: &dyn Region) -> CellUnion {
        let mut c = Coverer::new(self);
        c.interior_covering = false;
        c.covering_internal(region);
        let mut cu = c.result;
        cu.normalize();
        cu
    }

    /// Returns a fast (but less tight) covering.
    ///
    /// Starts with the region's `cell_union_bound()` and normalizes it
    /// to satisfy the covering parameters.
    pub fn fast_covering(&self, region: &dyn Region) -> CellUnion {
        let c = Coverer::new(self);
        let ids = region.cell_union_bound();
        let mut cu = CellUnion::from_cell_ids(ids);
        c.normalize_covering(&mut cu);
        cu
    }

    /// Returns a set of cells at the given level that cover `region`,
    /// starting from the cell containing `start`.
    ///
    /// Uses a flood-fill algorithm that expands outward from the starting
    /// cell to all edge-connected cells that intersect the region. The
    /// output cells are in arbitrary order.
    ///
    /// This can be faster than [`covering`](Self::covering) for long narrow regions such as
    /// polylines, but slower for compact regions with many output cells.
    pub fn get_simple_covering(
        region: &dyn Region,
        start: &Point,
        level: impl Into<Level>,
        output: &mut Vec<CellId>,
    ) {
        let level = level.into();
        Self::flood_fill(
            region,
            CellId::from_point(start).parent_at_level(level),
            output,
        );
    }

    /// Returns all edge-connected cells at the same level as `start` that
    /// intersect `region`, in arbitrary order.
    pub fn flood_fill(region: &dyn Region, start: CellId, output: &mut Vec<CellId>) {
        let mut all = HashSet::new();
        let mut frontier = Vec::new();
        output.clear();
        all.insert(start);
        frontier.push(start);
        while let Some(id) = frontier.pop() {
            if !region.intersects_cell(&Cell::from(id)) {
                continue;
            }
            output.push(id);
            for nbr in id.edge_neighbors() {
                if all.insert(nbr) {
                    frontier.push(nbr);
                }
            }
        }
    }

    /// Returns the maximum level that will actually be used in coverings,
    /// adjusted for `level_mod`.
    fn true_max_level(&self) -> Level {
        if self.level_mod == 1 {
            return self.max_level;
        }
        self.max_level - (self.max_level - self.min_level) % self.level_mod
    }

    /// Adjusts `level` downward so that it satisfies `level_mod`.
    fn adjust_level(&self, level: Level) -> Level {
        if self.level_mod > 1 && level > self.min_level {
            level - ((level - self.min_level) % self.level_mod)
        } else {
            level
        }
    }

    /// Reports whether the given list of cells is a valid covering that
    /// conforms to the current covering parameters. In particular:
    ///
    ///  - All cells must be valid.
    ///  - Cells must be sorted and non-overlapping.
    ///  - Cell levels must satisfy `min_level`, `max_level`, and `level_mod`.
    ///  - If the covering has more than `max_cells` cells, no two cells may
    ///    have a common ancestor at `min_level` or higher.
    ///  - There must be no sequence of `4^level_mod` cells that all share
    ///    a common parent (and could thus be replaced by that parent).
    pub fn is_canonical(&self, covering: &[CellId]) -> bool {
        let min_level = self.min_level;
        let max_level = self.true_max_level();
        let level_mod = self.level_mod;
        let too_many_cells = covering.len() > self.max_cells;
        let mut same_parent_count: usize = 1;
        let mut prev_id = CellId::none();

        for &id in covering {
            if !id.is_valid() {
                return false;
            }

            // Check that the CellId level is acceptable.
            let level = id.level();
            if level < min_level || level > max_level {
                return false;
            }
            if level_mod > 1 && !(level - min_level).is_multiple_of(level_mod) {
                return false;
            }

            if prev_id != CellId::none() {
                // Check that cells are sorted and non-overlapping.
                if prev_id.range_max() >= id.range_min() {
                    return false;
                }

                // If there are too many cells, check that no pair of adjacent
                // cells could be replaced by an ancestor.
                if too_many_cells
                    && let Some(ancestor_level) = id.common_ancestor_level(prev_id)
                    && ancestor_level >= min_level
                {
                    return false;
                }

                // Check that there are no sequences of (4 ** level_mod) cells
                // that all have the same parent (at a valid level).
                let plevel = level.as_i32() - i32::from(level_mod);
                if plevel < min_level.as_i32()
                    || level != prev_id.level()
                    || id.parent_at_level(Level::new(plevel as u8))
                        != prev_id.parent_at_level(Level::new(plevel as u8))
                {
                    same_parent_count = 1;
                } else {
                    same_parent_count += 1;
                    if same_parent_count == 1usize << (2 * u32::from(level_mod)) {
                        return false;
                    }
                }
            }
            prev_id = id;
        }
        true
    }

    /// Checks whether `covering` contains all `4^level_mod` children of `id`
    /// at the next valid level.
    fn contains_all_children(&self, covering: &[CellId], id: CellId) -> bool {
        let level = id.level() + self.level_mod;
        let pos = covering.partition_point(|&c| c < id.range_min());
        let mut it = pos;
        let mut child = id.child_begin_at_level(level);
        let end = id.child_end_at_level(level);
        while child != end {
            if it >= covering.len() || covering[it] != child {
                return false;
            }
            it += 1;
            child = child.next();
        }
        true
    }

    /// Replaces all descendants of `id` in `covering` with `id`.
    fn replace_cells_with_ancestor(covering: &mut Vec<CellId>, id: CellId) {
        let begin = covering.partition_point(|&c| c < id.range_min());
        let end = covering.partition_point(|&c| c <= id.range_max());
        debug_assert!(begin < end);
        covering[begin] = id;
        if begin + 1 < end {
            covering.drain((begin + 1)..end);
        }
    }

    /// Modifies the given covering so that it conforms to the current
    /// covering parameters (`max_level`, `level_mod`, `min_level`, `max_cells`).
    pub fn canonicalize_covering(&self, covering: &mut Vec<CellId>) {
        // If any cells are too small or don't satisfy level_mod, replace
        // with ancestors.
        if self.max_level < MAX_CELL_LEVEL || self.level_mod > 1 {
            for id in covering.iter_mut() {
                let level = id.level();
                let new_level = self.adjust_level(level.min(self.max_level));
                if new_level != level {
                    *id = id.parent_at_level(new_level);
                }
            }
        }

        // Sort and simplify (from_cell_ids normalizes).
        let mut cu = CellUnion::from_cell_ids(std::mem::take(covering));

        // Make sure covering satisfies min_level and level_mod, possibly
        // at the expense of satisfying max_cells.
        if self.min_level > 0 || self.level_mod > 1 {
            cu = cu.denormalize(self.min_level, self.level_mod);
        }

        *covering = cu.into_iter().collect();

        // If there are too many cells and the covering is very large, use
        // the RegionCoverer to compute a new covering.
        let excess = covering.len() as i64 - self.max_cells as i64;
        if excess <= 0 || self.is_canonical(covering) {
            return;
        }
        if excess as usize * covering.len() > 10000 {
            let cu = CellUnion::from_cell_ids(std::mem::take(covering));
            let result = self.covering(&cu);
            *covering = result.into_iter().collect();
        } else {
            // Repeatedly replace two adjacent cells by their lowest common
            // ancestor until the number of cells is acceptable.
            while covering.len() > self.max_cells {
                let mut best_index: Option<usize> = None;
                let mut best_level: i32 = -1;
                for i in 0..covering.len() - 1 {
                    if let Some(ancestor_level) = covering[i].common_ancestor_level(covering[i + 1])
                    {
                        let level = i32::from(self.adjust_level(ancestor_level));
                        if level > best_level {
                            best_level = level;
                            best_index = Some(i);
                        }
                    }
                }
                let Some(best_index) = best_index.filter(|_| best_level >= self.min_level.as_i32())
                else {
                    break;
                };
                let mut id = covering[best_index].parent_at_level(Level::new(best_level as u8));
                Self::replace_cells_with_ancestor(covering, id);

                // Now repeatedly check whether all children of the parent
                // cell are present, and replace with the parent if so.
                let mut level = best_level;
                while level > self.min_level.as_i32() {
                    level -= i32::from(self.level_mod);
                    id = id.parent_at_level(Level::new(level as u8));
                    if !self.contains_all_children(covering, id) {
                        break;
                    }
                    Self::replace_cells_with_ancestor(covering, id);
                }
            }
        }
        debug_assert!(self.is_canonical(covering));
    }
}

// ─── Internal Algorithm ─────────────────────────────────────────────────

/// A candidate cell for the covering algorithm.
struct Candidate {
    cell: Cell,
    terminal: bool,
    num_children: usize,
    children: Vec<Candidate>,
    priority: i64,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first (max-heap). We negate the priority in computation
        // so that larger cells and fewer children come first.
        self.priority.cmp(&other.priority)
    }
}

/// Internal state for the covering algorithm.
struct Coverer {
    min_level: Level,
    max_level: Level,
    level_mod: u8,
    max_cells: usize,
    interior_covering: bool,
    result: CellUnion,
    pq: BinaryHeap<Candidate>,
}

impl Coverer {
    fn new(rc: &RegionCoverer) -> Self {
        Coverer {
            min_level: rc.min_level.min(Level::MAX),
            max_level: rc.max_level.min(Level::MAX).max(rc.min_level),
            level_mod: rc.level_mod.clamp(1, 3),
            max_cells: rc.max_cells.max(1),
            interior_covering: false,
            result: CellUnion::from_cell_ids(vec![]),
            pq: BinaryHeap::new(),
        }
    }

    /// Returns the adjusted level to satisfy `level_mod`.
    fn adjust_level(&self, level: Level) -> Level {
        if self.level_mod > 1 && level > self.min_level {
            level - ((level - self.min_level) % self.level_mod)
        } else {
            level
        }
    }

    /// Creates a new candidate from a cell if it intersects the region.
    fn new_candidate(&self, cell: Cell, region: &dyn Region) -> Option<Candidate> {
        if !region.intersects_cell(&cell) {
            return None;
        }

        let level = cell.level();
        let mut cand = Candidate {
            cell,
            terminal: false,
            num_children: 0,
            children: Vec::new(),
            priority: 0,
        };

        if level >= self.min_level {
            if self.interior_covering {
                if region.contains_cell(&cell) {
                    cand.terminal = true;
                } else if level.as_u8().saturating_add(self.level_mod) > self.max_level.as_u8() {
                    return None;
                }
            } else if level.as_u8().saturating_add(self.level_mod) > self.max_level.as_u8()
                || region.contains_cell(&cell)
            {
                cand.terminal = true;
            }
        }

        Some(cand)
    }

    /// Expands the children of a candidate recursively.
    /// Returns the number of terminal children found.
    fn expand_children(
        &self,
        cand: &mut Candidate,
        cell: Cell,
        num_levels: u8,
        region: &dyn Region,
    ) -> usize {
        let num_levels = num_levels - 1;
        let mut num_terminals = 0;

        let id = cell.id();
        let mut child_id = id.child_begin();
        let end = id.child_end();

        while child_id != end {
            let child_cell = Cell::from(child_id);
            if num_levels > 0 {
                if region.intersects_cell(&child_cell) {
                    num_terminals += self.expand_children(cand, child_cell, num_levels, region);
                }
            } else if let Some(child) = self.new_candidate(child_cell, region) {
                let is_terminal = child.terminal;
                cand.children.push(child);
                cand.num_children += 1;
                if is_terminal {
                    num_terminals += 1;
                }
            }
            child_id = child_id.next();
        }

        num_terminals
    }

    /// Adds a candidate to the result or queue.
    fn add_candidate(&mut self, cand: Option<Candidate>, region: &dyn Region) {
        let Some(mut cand) = cand else {
            return;
        };

        if cand.terminal {
            self.result = CellUnion::from_cell_ids(
                self.result
                    .iter()
                    .copied()
                    .chain(std::iter::once(cand.cell.id()))
                    .collect(),
            );
            return;
        }

        // Determine expansion levels.
        let num_levels = if cand.cell.level() < self.min_level {
            1
        } else {
            self.level_mod
        };

        let cell = cand.cell;
        let num_terminals = self.expand_children(&mut cand, cell, num_levels, region);
        let max_children_shift = 2 * u32::from(self.level_mod);

        if cand.num_children == 0 {
            return;
        }

        if !self.interior_covering
            && num_terminals == (1usize << max_children_shift)
            && cand.cell.level() >= self.min_level
        {
            // All children are terminal — use parent instead.
            cand.terminal = true;
            cand.children.clear();
            self.add_candidate(Some(cand), region);
        } else {
            // Calculate priority and add children to heap.
            let level = i64::from(cand.cell.level().as_u8());
            cand.priority = -(((level << max_children_shift) + cand.num_children as i64)
                << max_children_shift)
                - num_terminals as i64;
            self.pq.push(cand);
        }
    }

    /// Runs the covering algorithm.
    fn covering_internal(&mut self, region: &dyn Region) {
        self.initial_candidates(region);

        while let Some(cand) = self.pq.pop() {
            if self.interior_covering && self.result.len() >= self.max_cells {
                break;
            }

            let level = cand.cell.level();
            let should_expand = self.interior_covering
                || level < self.min_level
                || cand.num_children == 1
                || self.result.len() + self.pq.len() + cand.num_children <= self.max_cells;

            if should_expand {
                for child in cand.children {
                    if !self.interior_covering || self.result.len() < self.max_cells {
                        self.add_candidate(Some(child), region);
                    }
                }
            } else {
                // Use this cell as-is.
                let mut terminal = Candidate {
                    cell: cand.cell,
                    terminal: true,
                    num_children: 0,
                    children: Vec::new(),
                    priority: 0,
                };
                terminal.terminal = true;
                self.add_candidate(Some(terminal), region);
            }
        }
    }

    /// Creates initial candidates from the region's bounding cap.
    fn initial_candidates(&mut self, region: &dyn Region) {
        let temp = RegionCoverer {
            min_level: Level::MIN,
            max_level: self.max_level,
            level_mod: 1,
            max_cells: self.max_cells.min(4),
        };
        let cells = temp.fast_covering(region);
        let mut adjusted: Vec<CellId> = cells.iter().copied().collect();
        self.adjust_cell_levels(&mut adjusted);

        for id in adjusted {
            let cell = Cell::from(id);
            let cand = self.new_candidate(cell, region);
            self.add_candidate(cand, region);
        }
    }

    /// Adjusts cell levels to satisfy `level_mod`.
    fn adjust_cell_levels(&self, cells: &mut Vec<CellId>) {
        if self.level_mod == 1 {
            return;
        }

        let mut out = 0usize;
        for i in 0..cells.len() {
            let mut id = cells[i];
            let level = id.level();
            let new_level = self.adjust_level(level);
            if new_level != level {
                id = id.parent_at_level(new_level);
            }
            if out > 0 && cells[out - 1].contains(id) {
                continue;
            }
            while out > 0 && id.contains(cells[out - 1]) {
                out -= 1;
            }
            cells[out] = id;
            out += 1;
        }
        cells.truncate(out);
    }

    /// Normalizes a covering to satisfy the covering parameters
    /// (`max_level`, `level_mod`, `min_level`, `max_cells`).
    fn normalize_covering(&self, cu: &mut CellUnion) {
        // If any cells are too small or don't satisfy level_mod, replace with ancestors.
        if self.max_level < MAX_CELL_LEVEL || self.level_mod > 1 {
            let ids: Vec<CellId> = cu
                .iter()
                .map(|&id| {
                    let level = id.level();
                    let new_level = self.adjust_level(level.min(self.max_level));
                    if new_level == level {
                        id
                    } else {
                        id.parent_at_level(new_level)
                    }
                })
                .collect();
            *cu = CellUnion::from_cell_ids(ids);
        }

        // Sort and simplify.
        cu.normalize();

        // Make sure covering satisfies min_level and level_mod,
        // possibly at the expense of satisfying max_cells.
        if self.min_level > 0 || self.level_mod > 1 {
            *cu = cu.denormalize(self.min_level, self.level_mod);
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::region::Region;
    use crate::s2::{Cap, LatLng, Point, Rect};

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_default() {
        let rc = RegionCoverer::new();
        assert_eq!(rc.min_level, 0);
        assert_eq!(rc.max_level, MAX_CELL_LEVEL);
        assert_eq!(rc.level_mod, 1);
        assert_eq!(rc.max_cells, 8);
    }

    #[test]
    fn test_covering_point_cap() {
        // A small cap around a point
        let center = p(45.0, 90.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(1.0));

        let rc = RegionCoverer::new().max_cells(8);
        let covering = rc.covering(&cap);
        assert!(!covering.is_empty());

        // All cells should contain the center
        let center_id = CellId::from_point(&center);
        let mut found = false;
        for &id in &covering {
            if id.contains(center_id) {
                found = true;
            }
        }
        assert!(found, "covering should contain the center point");
    }

    #[test]
    fn test_covering_full_cap() {
        let cap = Cap::full();
        let rc = RegionCoverer::new().max_cells(6);
        let covering = rc.covering(&cap);
        // Full cap should be covered by 6 face cells
        assert!(covering.len() <= 6);
    }

    #[test]
    fn test_covering_empty_cap() {
        let cap = Cap::empty();
        let rc = RegionCoverer::new();
        let covering = rc.covering(&cap);
        assert!(covering.is_empty());
    }

    #[test]
    fn test_covering_rect() {
        let ll1 = LatLng::from_degrees(10.0, 20.0);
        let ll2 = LatLng::from_degrees(20.0, 30.0);
        let rect = Rect::empty().add_point(ll1).add_point(ll2);

        let rc = RegionCoverer::new().max_cells(20).max_level(10);
        let covering = rc.covering(&rect);
        assert!(!covering.is_empty());

        // Verify that the covering actually covers the rect:
        // check a few points inside the rect
        let test_point = p(15.0, 25.0);
        let test_id = CellId::from_point(&test_point);
        let mut covered = false;
        for &id in &covering {
            if id.contains(test_id) {
                covered = true;
                break;
            }
        }
        assert!(covered, "covering should contain test point");
    }

    #[test]
    fn test_covering_max_level() {
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(0.001));

        let rc = RegionCoverer::new().max_level(5).max_cells(100);
        let covering = rc.covering(&cap);

        for &id in &covering {
            assert!(
                id.level() <= 5,
                "cell level {} exceeds max_level 5",
                id.level()
            );
        }
    }

    #[test]
    fn test_interior_covering() {
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(5.0));

        let rc = RegionCoverer::new().max_cells(50);
        let interior = rc.interior_covering(&cap);

        // All interior cells should be contained by the cap
        for &id in &interior {
            let cell = Cell::from(id);
            assert!(
                cap.contains_cell(&cell),
                "interior covering cell not contained by cap"
            );
        }
    }

    #[test]
    fn test_fast_covering() {
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(10.0));

        let rc = RegionCoverer::new();
        let covering = rc.fast_covering(&cap);
        assert!(!covering.is_empty());
    }

    #[test]
    fn test_covering_cell() {
        // Covering a single cell should return that cell (or its parent).
        let id = CellId::from_point(&p(0.0, 0.0)).parent_at_level(10);
        let cell = Cell::from(id);

        let rc = RegionCoverer::new().max_cells(1);
        let covering = rc.covering(&cell);
        assert!(!covering.is_empty());

        // The covering should contain the cell id
        let mut found = false;
        for &cid in &covering {
            if cid.contains(id) {
                found = true;
            }
        }
        assert!(found, "covering should contain the cell");
    }

    #[test]
    fn test_level_mod() {
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(1.0));

        let rc = RegionCoverer::new().level_mod(2).max_cells(20);
        let covering = rc.covering(&cap);

        for &id in &covering {
            let level = id.level();
            if level > rc.min_level {
                assert!(
                    (level - rc.min_level).is_multiple_of(rc.level_mod),
                    "level {} does not satisfy level_mod {}",
                    level,
                    rc.level_mod
                );
            }
        }
    }

    #[test]
    fn test_random_cells_max_one() {
        // C++ S2RegionCoverer::RandomCells
        // With max_cells=1, covering a single cell should return that cell.
        let rc = RegionCoverer::new().max_cells(1);
        for face in 0..6 {
            for level in [0u8, 5, 15, MAX_CELL_LEVEL] {
                let id = CellId::from_face(face).child_begin_at_level(level);
                let cell = Cell::from(id);
                let covering = rc.covering(&cell);
                assert_eq!(
                    covering.len(),
                    1,
                    "max_cells=1 covering of cell at level {level} should be 1 cell"
                );
                assert_eq!(covering[0], id, "covering should be the cell itself");
            }
        }
    }

    #[test]
    fn test_interior_covering_level_constraint() {
        // Interior covering of a large cap with level constraints should
        // produce cells only at valid levels.
        let cap = Cap::from_center_angle(p(0.0, 0.0), crate::s1::Angle::from_degrees(10.0));
        let rc = RegionCoverer::new().max_cells(50).min_level(3).max_level(8);
        let interior = rc.interior_covering(&cap);

        for &id in &interior {
            assert!(id.level() >= 3, "level {} < min_level 3", id.level());
            assert!(id.level() <= 8, "level {} > max_level 8", id.level());
            // All interior cells must be contained by the cap.
            let cell = Cell::from(id);
            assert!(
                cap.contains_cell(&cell),
                "interior cell at level {} not contained by cap",
                id.level(),
            );
        }
    }

    #[test]
    fn test_covering_deterministic() {
        // Verify that GetCovering is deterministic: running it twice on
        // the same input produces the same result.
        let center = p(37.7749, -122.4194);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(0.5));

        let rc = RegionCoverer::new().max_cells(20).max_level(15);
        let covering1 = rc.covering(&cap);
        let covering2 = rc.covering(&cap);
        assert_eq!(covering1, covering2, "covering should be deterministic");
    }

    #[test]
    fn test_covering_min_level() {
        // All cells in the covering should have level >= min_level.
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(5.0));

        let rc = RegionCoverer::new().min_level(3).max_cells(100);
        let covering = rc.covering(&cap);

        for &id in &covering {
            assert!(
                id.level() >= 3,
                "cell level {} is below min_level 3",
                id.level()
            );
        }
    }

    #[test]
    fn test_covering_normalized() {
        // The covering should be a normalized cell union (no cell contains another).
        let center = p(48.8566, 2.3522);
        let cap = Cap::from_center_angle(center, crate::s1::Angle::from_degrees(2.0));

        let rc = RegionCoverer::new().max_cells(30);
        let covering = rc.covering(&cap);

        // Check no cell contains another.
        for i in 0..covering.len() {
            for j in (i + 1)..covering.len() {
                assert!(
                    !covering[i].contains(covering[j]),
                    "cell {i} contains cell {j}",
                );
                assert!(
                    !covering[j].contains(covering[i]),
                    "cell {j} contains cell {i}",
                );
            }
        }
    }

    #[test]
    fn test_covering_polyline() {
        // Cover a polyline.
        use crate::s2::polyline::Polyline;
        let vertices: Vec<Point> = [(0.0, 0.0), (0.0, 10.0), (5.0, 15.0)]
            .iter()
            .map(|&(lat, lng)| p(lat, lng))
            .collect();
        let polyline = Polyline::new(vertices);

        let rc = RegionCoverer::new().max_cells(50).max_level(12);
        let covering = rc.covering(&polyline);
        assert!(
            !covering.is_empty(),
            "polyline covering should not be empty"
        );

        // Verify each endpoint is covered.
        for &(lat, lng) in &[(0.0, 0.0), (0.0, 10.0), (5.0, 15.0)] {
            let pt_id = CellId::from_point(&p(lat, lng));
            let covered = covering.iter().any(|&id| id.contains(pt_id));
            assert!(covered, "endpoint ({lat}, {lng}) should be covered");
        }
    }

    #[test]
    fn test_fast_covering_has_cells() {
        // Fast covering of a large region should return many cells.
        let cap = Cap::from_center_angle(p(0.0, 0.0), crate::s1::Angle::from_degrees(30.0));
        let rc = RegionCoverer::new();
        let covering = rc.fast_covering(&cap);
        assert!(
            covering.len() >= 4,
            "fast covering of 30° cap should have >= 4 cells, got {}",
            covering.len()
        );
    }

    // ─── IsCanonical tests ──────────────────────────────────────────────

    /// Helper: parse debug strings into `CellIds` and test `is_canonical`.
    fn is_canonical(strs: &[&str], rc: &RegionCoverer) -> bool {
        let ids: Vec<CellId> = strs
            .iter()
            .map(|s| CellId::from_debug_string(s).unwrap_or_default())
            .collect();
        rc.is_canonical(&ids)
    }

    #[test]
    fn test_is_canonical_invalid_cell_id() {
        let rc = RegionCoverer::new();
        assert!(is_canonical(&["1/"], &rc));
        // "invalid" parses to CellId::none() which is not valid.
        assert!(!is_canonical(&["invalid"], &rc));
    }

    #[test]
    fn test_is_canonical_unsorted() {
        let rc = RegionCoverer::new();
        assert!(is_canonical(&["1/1", "1/3"], &rc));
        assert!(!is_canonical(&["1/3", "1/1"], &rc));
    }

    #[test]
    fn test_is_canonical_overlapping() {
        let rc = RegionCoverer::new();
        assert!(is_canonical(&["1/2", "1/33"], &rc));
        assert!(!is_canonical(&["1/3", "1/33"], &rc));
    }

    #[test]
    fn test_is_canonical_min_level() {
        let rc = RegionCoverer::new().min_level(2);
        assert!(is_canonical(&["1/31"], &rc));
        assert!(!is_canonical(&["1/3"], &rc));
    }

    #[test]
    fn test_is_canonical_max_level() {
        let rc = RegionCoverer::new().max_level(2);
        assert!(is_canonical(&["1/31"], &rc));
        assert!(!is_canonical(&["1/312"], &rc));
    }

    #[test]
    fn test_is_canonical_level_mod() {
        let rc = RegionCoverer::new().level_mod(2);
        assert!(is_canonical(&["1/31"], &rc));
        assert!(!is_canonical(&["1/312"], &rc));
    }

    #[test]
    fn test_is_canonical_max_cells() {
        let rc = RegionCoverer::new().max_cells(2);
        assert!(is_canonical(&["1/1", "1/3"], &rc));
        assert!(!is_canonical(&["1/1", "1/3", "2/"], &rc));
        assert!(is_canonical(&["1/123", "2/1", "3/0122"], &rc));
    }

    #[test]
    fn test_is_canonical_normalized() {
        let rc = RegionCoverer::new();
        assert!(is_canonical(&["1/01", "1/02", "1/03", "1/10", "1/11"], &rc));
        assert!(!is_canonical(
            &["1/00", "1/01", "1/02", "1/03", "1/10"],
            &rc
        ));

        assert!(is_canonical(&["0/22", "1/01", "1/02", "1/03", "1/10"], &rc));
        assert!(!is_canonical(
            &["0/22", "1/00", "1/01", "1/02", "1/03"],
            &rc
        ));

        let rc2 = RegionCoverer::new().max_cells(20).level_mod(2);
        assert!(is_canonical(
            &[
                "1/1101", "1/1102", "1/1103", "1/1110", "1/1111", "1/1112", "1/1113", "1/1120",
                "1/1121", "1/1122", "1/1123", "1/1130", "1/1131", "1/1132", "1/1133", "1/1200",
            ],
            &rc2
        ));
        assert!(!is_canonical(
            &[
                "1/1100", "1/1101", "1/1102", "1/1103", "1/1110", "1/1111", "1/1112", "1/1113",
                "1/1120", "1/1121", "1/1122", "1/1123", "1/1130", "1/1131", "1/1132", "1/1133",
            ],
            &rc2
        ));
    }

    // ─── CanonicalizeCovering tests ─────────────────────────────────────

    fn test_canonicalize(input_str: &[&str], expected_str: &[&str], rc: &RegionCoverer) {
        let mut actual: Vec<CellId> = input_str
            .iter()
            .map(|s| CellId::from_debug_string(s).unwrap_or_default())
            .collect();
        let expected: Vec<CellId> = expected_str
            .iter()
            .map(|s| CellId::from_debug_string(s).unwrap_or_default())
            .collect();

        assert!(!rc.is_canonical(&actual), "input should not be canonical");
        rc.canonicalize_covering(&mut actual);
        assert!(rc.is_canonical(&actual), "result should be canonical");
        assert_eq!(
            expected, actual,
            "canonicalized covering does not match expected"
        );
    }

    #[test]
    fn test_canonicalize_unsorted_duplicate() {
        let rc = RegionCoverer::new();
        test_canonicalize(
            &["1/200", "1/13122", "1/20", "1/131", "1/13100"],
            &["1/131", "1/20"],
            &rc,
        );
    }

    #[test]
    fn test_canonicalize_max_level_exceeded() {
        let rc = RegionCoverer::new().max_level(2);
        test_canonicalize(
            &["0/3001", "0/3002", "4/012301230123"],
            &["0/30", "4/01"],
            &rc,
        );
    }

    #[test]
    fn test_canonicalize_wrong_level_mod() {
        let rc = RegionCoverer::new().min_level(1).level_mod(3);
        test_canonicalize(
            &["0/0", "1/11", "2/222", "3/3333"],
            &["0/0", "1/1", "2/2", "3/3333"],
            &rc,
        );
    }

    #[test]
    fn test_canonicalize_replaced_by_parent() {
        // 16 children should be replaced by their parent when level_mod == 2.
        let rc = RegionCoverer::new().level_mod(2);
        test_canonicalize(
            &[
                "0/00", "0/01", "0/02", "0/03", "0/10", "0/11", "0/12", "0/13", "0/20", "0/21",
                "0/22", "0/23", "0/30", "0/31", "0/32", "0/33",
            ],
            &["0/"],
            &rc,
        );
    }

    #[test]
    fn test_canonicalize_denormalized() {
        // All 4 children may be used when necessary to satisfy min_level/level_mod.
        let rc = RegionCoverer::new().min_level(1).level_mod(2);
        // Note: we skip the S2CellUnion variant test per C++ (denormalized input
        // changes when converted to S2CellUnion).
        let mut actual: Vec<CellId> = ["0/", "1/130", "1/131", "1/132", "1/133"]
            .iter()
            .map(|s| CellId::from_debug_string(s).unwrap_or_default())
            .collect();
        let expected: Vec<CellId> = [
            "0/0", "0/1", "0/2", "0/3", "1/130", "1/131", "1/132", "1/133",
        ]
        .iter()
        .map(|s| CellId::from_debug_string(s).unwrap_or_default())
        .collect();

        rc.canonicalize_covering(&mut actual);
        assert!(rc.is_canonical(&actual), "result should be canonical");
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_canonicalize_max_cells_merges_smallest() {
        // When there are too many cells, the smallest cells are merged first.
        let rc = RegionCoverer::new().max_cells(3);
        test_canonicalize(
            &["0/", "1/0", "1/1", "2/01300", "2/0131313"],
            &["0/", "1/", "2/013"],
            &rc,
        );
    }

    #[test]
    fn test_canonicalize_max_cells_merges_repeatedly() {
        // When merging creates a cell whose 4 children are all present,
        // those should be merged into their parent (repeatedly if necessary).
        let rc = RegionCoverer::new().max_cells(8);
        test_canonicalize(
            &[
                "0/0121", "0/0123", "1/0", "1/1", "1/2", "1/30", "1/32", "1/33", "1/311", "1/312",
                "1/313", "1/3100", "1/3101", "1/3103", "1/31021", "1/31023",
            ],
            &["0/0121", "0/0123", "1/"],
            &rc,
        );
    }

    // ─── GetSimpleCovering / FloodFill tests ────────────────────────────

    #[test]
    fn test_simple_covering_cap() {
        // Cover a small cap at various levels and verify properties.
        use crate::s1::Angle;
        for level in [1u8, 5, 10, 15] {
            let center = p(37.0, -122.0);
            let cap = Cap::from_center_angle(center, Angle::from_degrees(0.5));
            let mut covering = Vec::new();
            RegionCoverer::get_simple_covering(&cap, &center, level, &mut covering);
            assert!(!covering.is_empty(), "covering at level {level} is empty");

            // All cells should be at the requested level.
            for &id in &covering {
                assert_eq!(id.level(), level, "cell not at level {level}");
            }

            // The covering should contain the center.
            let center_id = CellId::from_point(&center).parent_at_level(level);
            assert!(
                covering.contains(&center_id),
                "covering should contain the center cell"
            );

            // Every cell should intersect the cap.
            for &id in &covering {
                let cell = Cell::from(id);
                assert!(
                    cap.intersects_cell(&cell),
                    "covering cell does not intersect region"
                );
            }
        }
    }

    #[test]
    fn test_simple_covering_covers_region() {
        // Verify that the simple covering actually covers the region by
        // checking that sample points inside the cap are covered.
        use crate::s1::Angle;
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, Angle::from_degrees(2.0));
        let level = 8;
        let mut covering = Vec::new();
        RegionCoverer::get_simple_covering(&cap, &center, level, &mut covering);

        // Check a grid of points inside the cap.
        for dlat in -3..=3 {
            for dlng in -3..=3 {
                let pt = p(f64::from(dlat) * 0.5, f64::from(dlng) * 0.5);
                if !cap.contains_point(pt) {
                    continue;
                }
                let pt_id = CellId::from_point(&pt).parent_at_level(level);
                assert!(
                    covering.contains(&pt_id),
                    "point ({}, {}) inside cap not covered",
                    f64::from(dlat) * 0.5,
                    f64::from(dlng) * 0.5,
                );
            }
        }
    }

    #[test]
    fn test_flood_fill_single_cell() {
        // Flood fill with a region that contains exactly one cell should
        // return just that cell.
        let id = CellId::from_point(&p(0.0, 0.0)).parent_at_level(10);
        let cell = Cell::from(id);
        let mut output = Vec::new();
        RegionCoverer::flood_fill(&cell, id, &mut output);
        assert_eq!(output, vec![id]);
    }

    #[test]
    fn test_flood_fill_face() {
        // Flood fill starting from any cell on face 0 with the full cap
        // should cover the entire face (and no more at that level).
        let level = 1u8;
        let start = CellId::from_face(0).child_begin_at_level(level);
        let cap = Cap::full();
        let mut output = Vec::new();
        RegionCoverer::flood_fill(&cap, start, &mut output);
        // At level 1, there are 6*4 = 24 cells total. FloodFill should
        // find all of them since the full cap intersects everything.
        assert_eq!(
            output.len(),
            6 * 4usize.pow(u32::from(level)),
            "flood fill of full cap at level {level} should cover all cells"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let rc = RegionCoverer {
            min_level: Level::new(2),
            max_level: Level::new(10),
            level_mod: 2,
            max_cells: 16,
        };
        let json = serde_json::to_string(&rc).unwrap();
        let back: RegionCoverer = serde_json::from_str(&json).unwrap();
        assert_eq!(rc.min_level, back.min_level);
        assert_eq!(rc.max_level, back.max_level);
        assert_eq!(rc.level_mod, back.level_mod);
        assert_eq!(rc.max_cells, back.max_cells);
    }
}
