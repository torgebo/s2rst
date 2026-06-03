// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A sorted, normalized collection of [`CellId`]s representing a region.
//!
//! Corresponds to C++ `S2CellUnion`, Go `s2.CellUnion`, Java `S2CellUnion`.
//!
//! A normalized `CellUnion` is sorted in increasing order, contains no
//! duplicates, no cell that is contained by another, and does not contain
//! all four children of any parent cell.

use crate::s1::Angle;
use crate::s2::cell_id::lsb_for_level;
use crate::s2::coords::Level;
use crate::s2::metric::MIN_WIDTH;
use crate::s2::{Cap, Cell, CellId, Point, Rect, Region};
use std::ops::Deref;

/// A sorted, normalized collection of `CellId`s representing a region.
///
/// # Examples
///
/// ```
/// use s2rst::s2::{CellId, CellUnion, LatLng, Point};
///
/// // Create cell IDs for two nearby locations.
/// let nyc = CellId::from_lat_lng(&LatLng::from_degrees(40.7128, -74.0060));
/// let jfk = CellId::from_lat_lng(&LatLng::from_degrees(40.6413, -73.7781));
///
/// // Build a union from parent cells; normalization is automatic.
/// let cu = CellUnion::from_cell_ids(vec![
///     nyc.parent_at_level(12),
///     jfk.parent_at_level(12),
/// ]);
/// assert!(cu.is_normalized());
/// assert_eq!(cu.num_cells(), 2);
///
/// // Containment checks.
/// assert!(cu.contains_cell_id(nyc));
/// assert!(cu.contains_point(LatLng::from_degrees(40.7128, -74.0060).to_point()));
///
/// // A child cell is contained by its parent in the union.
/// let child = nyc.parent_at_level(15);
/// assert!(cu.contains_cell_id(child));
/// ```
#[must_use]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CellUnion(Vec<CellId>);

impl CellUnion {
    /// Creates an empty cell union.
    pub fn new() -> Self {
        CellUnion(Vec::new())
    }

    /// Creates a cell union from the given cell IDs, normalizing the result.
    pub fn from_cell_ids(ids: Vec<CellId>) -> Self {
        let mut cu = CellUnion(ids);
        cu.normalize();
        cu
    }

    /// Creates a cell union from the given cell IDs without normalizing.
    ///
    /// This is useful when the caller knows the cell IDs are already in the
    /// desired form, or when non-normalized output is intentional (e.g.,
    /// partitioning results that should only contain cells actually present
    /// in a density tree).
    pub fn from_verbatim(ids: Vec<CellId>) -> Self {
        CellUnion(ids)
    }

    /// Creates a cell union from a half-open range of leaf cells `[begin, end)`.
    pub fn from_range(begin: CellId, end: CellId) -> Self {
        let mut ids = Vec::new();
        let mut id = begin.parent_at_level(begin.level());
        loop {
            if id >= end {
                break;
            }
            // Find the largest cell that fits.
            while !id.is_face() {
                let parent = id.parent();
                if parent.range_min() < begin || parent.range_max() >= end {
                    break;
                }
                id = parent;
            }
            ids.push(id);
            id = id.next();
        }
        CellUnion(ids)
    }

    /// Returns the number of cells in this union.
    #[inline]
    pub fn num_cells(&self) -> usize {
        self.0.len()
    }

    /// Returns the inner slice of cell IDs.
    #[inline]
    pub fn cell_ids(&self) -> &[CellId] {
        &self.0
    }

    /// Reports whether this union is valid. A valid union has sorted,
    /// non-overlapping cells.
    pub fn is_valid(&self) -> bool {
        for i in 0..self.0.len() {
            if !self.0[i].is_valid() {
                return false;
            }
            if i > 0 {
                if self.0[i - 1] >= self.0[i] {
                    return false;
                }
                if self.0[i - 1].range_max() >= self.0[i].range_min() {
                    return false;
                }
            }
        }
        true
    }

    /// Reports whether this union is normalized.
    pub fn is_normalized(&self) -> bool {
        if !self.is_valid() {
            return false;
        }
        for i in 0..self.0.len() {
            if i + 3 < self.0.len()
                && are_siblings(self.0[i], self.0[i + 1], self.0[i + 2], self.0[i + 3])
            {
                return false;
            }
        }
        true
    }

    /// Normalizes the cell union (sorts, removes duplicates and contained
    /// cells, and merges sibling groups).
    pub fn normalize(&mut self) {
        self.0.sort_unstable();
        let mut output: Vec<CellId> = Vec::with_capacity(self.0.len());

        for ci in self.0.drain(..) {
            let mut ci = ci;

            // Check whether this cell is contained by the previous cell.
            if let Some(&last) = output.last()
                && last.contains(ci)
            {
                continue;
            }

            // Discard any previous cells that are contained by this cell.
            while let Some(&last) = output.last() {
                if ci.contains(last) {
                    output.pop();
                } else {
                    break;
                }
            }

            // Check whether the last 3 cells plus this one can be collapsed
            // into a single higher level cell.
            while output.len() >= 3 {
                let len = output.len();
                if !are_siblings(output[len - 3], output[len - 2], output[len - 1], ci) {
                    break;
                }
                // Replace four children by their parent cell.
                output.truncate(len - 3);
                ci = ci.parent(); // ci.level >= 1 because are_siblings returns false for faces
            }

            output.push(ci);
        }

        self.0 = output;
        debug_assert!(self.is_normalized());
    }

    /// Reports whether this union contains the given cell ID.
    pub fn contains_cell_id(&self, id: CellId) -> bool {
        debug_assert!(id.is_valid());
        // Find the first cell that does not entirely precede `id`.
        // "Entirely precedes" means `c.range_max() < id.range_min()`.
        let pos = self.0.partition_point(|c| c.range_max() < id.range_min());
        pos < self.0.len() && self.0[pos].contains(id)
    }

    /// Reports whether this union intersects the given cell ID.
    pub fn intersects_cell_id(&self, id: CellId) -> bool {
        debug_assert!(id.is_valid());
        // Find the first cell that does not entirely precede `id`.
        let pos = self.0.partition_point(|c| c.range_max() < id.range_min());
        pos < self.0.len() && self.0[pos].intersects(id)
    }

    /// Reports whether this union contains the given point.
    pub fn contains_point(&self, p: Point) -> bool {
        self.contains_cell_id(CellId::from_point(&p))
    }

    /// Reports whether this union contains the other union.
    pub fn contains_union(&self, other: &CellUnion) -> bool {
        other.0.iter().all(|id| self.contains_cell_id(*id))
    }

    /// Reports whether this union intersects the other union.
    pub fn intersects_union(&self, other: &CellUnion) -> bool {
        other.0.iter().any(|id| self.intersects_cell_id(*id))
    }

    /// Returns the number of leaf cells covered by this union.
    pub fn leaf_cells_covered(&self) -> i64 {
        let mut count: i64 = 0;
        for id in &self.0 {
            let level = id.level();
            let leaves =
                1i64 << (2 * (u32::from(crate::s2::coords::MAX_CELL_LEVEL) - level.as_u32()));
            count += leaves;
        }
        count
    }

    /// Creates a cell union covering the entire sphere (all 6 face cells).
    pub fn whole_sphere() -> Self {
        CellUnion::from_cell_ids((0..6).map(CellId::from_face).collect())
    }

    /// Creates a cell union from a closed range of leaf cells `[min_id, max_id]`.
    pub fn from_min_max(min_id: CellId, max_id: CellId) -> Self {
        debug_assert!(max_id.is_valid());
        CellUnion::from_begin_end(min_id, max_id.next())
    }

    /// Creates a cell union from a half-open range of leaf cells `[begin, end)`.
    /// Uses `maximum_tile` for efficient construction.
    pub fn from_begin_end(begin: CellId, end: CellId) -> Self {
        let leaf_end = CellId::end(crate::s2::coords::MAX_CELL_LEVEL);
        debug_assert!(begin.is_leaf());
        debug_assert!(end.is_leaf());
        debug_assert!(begin.is_valid() || begin == leaf_end);
        debug_assert!(end.is_valid() || end == leaf_end);
        debug_assert!(begin <= end);
        let mut ids = Vec::new();
        let mut id = begin.maximum_tile(end);
        while id != end {
            ids.push(id);
            id = id.next().maximum_tile(end);
        }
        CellUnion(ids)
    }

    /// Returns true if this cell union is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the union of this cell union with `other`.
    pub fn union(&self, other: &CellUnion) -> CellUnion {
        let mut ids = Vec::with_capacity(self.0.len() + other.0.len());
        ids.extend_from_slice(&self.0);
        ids.extend_from_slice(&other.0);
        CellUnion::from_cell_ids(ids)
    }

    /// Returns the intersection of this cell union with `other`.
    pub fn intersection(&self, other: &CellUnion) -> CellUnion {
        let mut result = Vec::new();
        get_intersection(&self.0, &other.0, &mut result);
        CellUnion(result)
    }

    /// Returns the intersection of this cell union with a single cell.
    pub fn intersection_with_cell_id(&self, id: CellId) -> CellUnion {
        if self.contains_cell_id(id) {
            return CellUnion(vec![id]);
        }
        let mut result = Vec::new();
        let pos = self.0.partition_point(|c| *c < id.range_min());
        let id_max = id.range_max();
        let mut i = pos;
        while i < self.0.len() && self.0[i] <= id_max {
            result.push(self.0[i]);
            i += 1;
        }
        CellUnion(result)
    }

    /// Returns the difference of this cell union minus `other`.
    pub fn difference(&self, other: &CellUnion) -> CellUnion {
        let mut result = Vec::new();
        for &id in &self.0 {
            get_difference_internal(id, other, &mut result);
        }
        CellUnion(result)
    }

    /// Expands this cell union by adding all neighbors at `expand_level`.
    pub fn expand_at_level(&mut self, expand_level: Level) {
        let mut output = Vec::new();
        let level_lsb = lsb_for_level(expand_level);
        let n = self.0.len();
        let mut i = n;
        while i > 0 {
            i -= 1;
            let mut id = self.0[i];
            if id.lsb() < level_lsb {
                id = id.parent_at_level(expand_level);
                // Skip over any cells contained by this one.
                while i > 0 && id.contains(self.0[i - 1]) {
                    i -= 1;
                }
            }
            output.push(id);
            if let Some(neighbors) = id.all_neighbors(expand_level) {
                output.extend(neighbors);
            }
        }
        self.0 = output;
        self.normalize();
    }

    /// Expands this cell union so that it contains all points whose distance
    /// to the union is at most `min_radius`.
    pub fn expand_by_radius(&mut self, min_radius: Angle, max_level_diff: u8) {
        let min_level = self
            .0
            .iter()
            .map(|id| id.level())
            .min()
            .unwrap_or(Level::MIN);
        // Find the maximum level such that all cells are at least "min_radius" wide.
        // C++: kMinWidth.GetLevelForMinValue(min_radius.radians())
        let radius_level = MIN_WIDTH.max_level(min_radius.radians());
        if radius_level == Level::MIN && min_radius.radians() > MIN_WIDTH.value(0u8) {
            // The requested expansion is greater than the width of a face cell.
            self.expand_at_level(Level::MIN);
        }
        // Clamp to MAX to prevent overflow in Level addition.
        let level =
            Level::new((min_level.as_u8().saturating_add(max_level_diff)).min(Level::MAX.as_u8()))
                .min(radius_level);
        self.expand_at_level(level);
    }

    /// Returns the average area of this cell union, computed using the
    /// average area of leaf cells.
    pub fn average_based_area(&self) -> f64 {
        Cell::average_area_for_level(crate::s2::coords::MAX_CELL_LEVEL)
            * self.leaf_cells_covered() as f64
    }

    /// Returns the approximate area of this cell union, using `Cell::approx_area()`
    /// for each cell.
    pub fn approx_area(&self) -> f64 {
        self.0
            .iter()
            .map(|&id| Cell::from_cell_id(id).approx_area())
            .sum()
    }

    /// Returns the exact area of this cell union, using `Cell::exact_area()`
    /// for each cell.
    pub fn exact_area(&self) -> f64 {
        self.0
            .iter()
            .map(|&id| Cell::from_cell_id(id).exact_area())
            .sum()
    }

    /// Replaces large cells with smaller cells at `min_level` and ensures
    /// `(level - min_level)` is a multiple of `level_mod`.
    pub fn denormalize(&self, min_level: Level, level_mod: u8) -> CellUnion {
        debug_assert!((1..=3).contains(&level_mod));
        let mut result = Vec::new();
        for &id in &self.0 {
            let level = id.level();
            let new_level = if level < min_level {
                min_level
            } else if level_mod > 1 {
                let offset = (level - min_level) % level_mod;
                if offset > 0 {
                    level + (level_mod - offset)
                } else {
                    level
                }
            } else {
                level
            };
            let new_level = new_level.min(Level::MAX);

            if new_level == level {
                result.push(id);
            } else {
                let end = id.child_end_at_level(new_level);
                let mut child = id.child_begin_at_level(new_level);
                while child != end {
                    result.push(child);
                    child = child.next();
                }
            }
        }
        CellUnion(result)
    }
}

/// Computes the intersection of two sorted, non-overlapping cell ID vectors.
fn get_intersection(x: &[CellId], y: &[CellId], out: &mut Vec<CellId>) {
    let mut i = 0;
    let mut j = 0;
    while i < x.len() && j < y.len() {
        let imin = x[i].range_min();
        let jmin = y[j].range_min();
        match imin.cmp(&jmin) {
            std::cmp::Ordering::Greater => {
                // Either y[j] contains x[i] or the two cells are disjoint.
                if x[i] <= y[j].range_max() {
                    out.push(x[i]);
                    i += 1;
                } else {
                    // Advance j to the first cell that might overlap x[i].
                    j = y[j..].partition_point(|c| c.range_max() < x[i].range_min()) + j;
                }
            }
            std::cmp::Ordering::Less => {
                // Symmetric case.
                if y[j] <= x[i].range_max() {
                    out.push(y[j]);
                    j += 1;
                } else {
                    i = x[i..].partition_point(|c| c.range_max() < y[j].range_min()) + i;
                }
            }
            std::cmp::Ordering::Equal => {
                // Same range_min, so one contains the other.
                if x[i] < y[j] {
                    out.push(x[i]);
                    i += 1;
                } else {
                    out.push(y[j]);
                    j += 1;
                }
            }
        }
    }
}

/// Recursively adds the difference between `cell` and `y` to `result`.
fn get_difference_internal(cell: CellId, y: &CellUnion, result: &mut Vec<CellId>) {
    if !y.intersects_cell_id(cell) {
        result.push(cell);
    } else if !y.contains_cell_id(cell) {
        let children = cell.children();
        for child in children {
            get_difference_internal(child, y, result);
        }
    }
}

/// Reports whether the four cell IDs are siblings (children of the same parent).
fn are_siblings(a: CellId, b: CellId, c: CellId, d: CellId) -> bool {
    // Quick XOR check: a ^ b ^ c should equal d if they're the four children.
    if (a.0 ^ b.0 ^ c.0) != d.0 {
        return false;
    }
    // Compute a mask that blocks out the two bits encoding the child position
    // of d with respect to its parent.
    let mask = d.lsb() << 1;
    let mask = !(mask + (mask << 1));
    let id_masked = d.0 & mask;
    (a.0 & mask == id_masked)
        && (b.0 & mask == id_masked)
        && (c.0 & mask == id_masked)
        && !d.is_face()
}

impl Deref for CellUnion {
    type Target = [CellId];
    fn deref(&self) -> &[CellId] {
        &self.0
    }
}

impl std::fmt::Display for CellUnion {
    /// Formats the cell union as `Size:<n> S2CellIds:<token1>,<token2>,...`
    /// matching C++ `S2CellUnion::ToString()`. Truncates to 500 cell tokens.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const MAX_COUNT: usize = 500;
        write!(f, "Size:{} S2CellIds:", self.0.len())?;
        let limit = self.0.len().min(MAX_COUNT);
        for (i, id) in self.0[..limit].iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{}", id.to_token())?;
        }
        if self.0.len() > MAX_COUNT {
            write!(f, ",...")?;
        }
        Ok(())
    }
}

impl FromIterator<CellId> for CellUnion {
    fn from_iter<I: IntoIterator<Item = CellId>>(iter: I) -> Self {
        CellUnion::from_cell_ids(iter.into_iter().collect())
    }
}

impl IntoIterator for CellUnion {
    type Item = CellId;
    type IntoIter = std::vec::IntoIter<CellId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a CellUnion {
    type Item = &'a CellId;
    type IntoIter = std::slice::Iter<'a, CellId>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Region for CellUnion {
    fn cap_bound(&self) -> Cap {
        if self.0.is_empty() {
            return Cap::empty();
        }

        // Use the centroid as the cap center.
        let mut centroid = crate::r3::Vector::default();
        for &id in &self.0 {
            let area = Cell::from_cell_id(id).average_area();
            centroid = centroid + id.to_point().vector() * area;
        }

        let mut cap = if centroid == crate::r3::Vector::default() {
            Cap::from_point(Point::from_coords(1.0, 0.0, 0.0))
        } else {
            Cap::from_point(Point(centroid.normalize()))
        };

        for &id in &self.0 {
            cap = cap.add_cap(Cell::from_cell_id(id).cap_bound());
        }
        cap
    }

    fn rect_bound(&self) -> Rect {
        let mut bound = Rect::empty();
        for &id in &self.0 {
            bound = bound.union(Cell::from_cell_id(id).rect_bound());
        }
        bound
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        self.contains_cell_id(cell.id())
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        self.intersects_cell_id(cell.id())
    }

    fn contains_point(&self, p: &Point) -> bool {
        CellUnion::contains_point(self, *p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::coords::MAX_CELL_LEVEL;

    /// Creates a cell union from cell IDs without normalization (test only).
    fn from_verbatim(ids: Vec<CellId>) -> CellUnion {
        CellUnion(ids)
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn cell_union_is_send_sync() {
        is_send_sync::<CellUnion>();
    }

    #[test]
    fn test_empty() {
        let cu = CellUnion::new();
        assert_eq!(cu.num_cells(), 0);
        assert!(cu.is_valid());
        assert!(cu.is_normalized());
    }

    #[test]
    fn test_from_cell_ids() {
        let ids = vec![CellId::from_face(0), CellId::from_face(1)];
        let cu = CellUnion::from_cell_ids(ids);
        assert_eq!(cu.num_cells(), 2);
        assert!(cu.is_valid());
        assert!(cu.is_normalized());
    }

    #[test]
    fn test_contains_cell_id() {
        let parent = CellId::from_face(0);
        let cu = CellUnion::from_cell_ids(vec![parent]);
        assert!(cu.contains_cell_id(parent));
        // Child should be contained.
        assert!(cu.contains_cell_id(parent.children()[0]));
    }

    #[test]
    fn test_not_contains() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        assert!(!cu.contains_cell_id(CellId::from_face(1)));
    }

    #[test]
    fn test_normalize_removes_contained() {
        let parent = CellId::from_face(0);
        let child = parent.children()[0];
        let cu = CellUnion::from_cell_ids(vec![parent, child]);
        assert_eq!(cu.num_cells(), 1);
        assert_eq!(cu[0], parent);
    }

    #[test]
    fn test_normalize_merges_siblings() {
        let parent = CellId::from_face(0);
        let children = parent.children();
        let cu = CellUnion::from_cell_ids(children.to_vec());
        assert_eq!(cu.num_cells(), 1);
        assert_eq!(cu[0], parent);
    }

    #[test]
    fn test_contains_point() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        assert!(cu.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
        assert!(!cu.contains_point(Point::from_coords(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_intersects() {
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let cu2 = CellUnion::from_cell_ids(vec![CellId::from_face(0).children()[0]]);
        assert!(cu1.intersects_union(&cu2));
    }

    #[test]
    fn test_not_intersects() {
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let cu2 = CellUnion::from_cell_ids(vec![CellId::from_face(3)]);
        assert!(!cu1.intersects_union(&cu2));
    }

    #[test]
    fn test_leaf_cells_covered() {
        // A face cell covers 4^30 leaf cells.
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        assert!(cu.leaf_cells_covered() > 0);
    }

    #[test]
    fn test_region_trait() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let r: &dyn Region = &cu;
        assert!(!r.cap_bound().is_empty());
        assert!(!r.rect_bound().is_empty());
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_deref() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0), CellId::from_face(1)]);
        assert_eq!(cu.len(), 2);
        let _: &[CellId] = &cu;
    }

    #[test]
    fn test_from_iterator() {
        let cu: CellUnion = [CellId::from_face(0), CellId::from_face(1)]
            .into_iter()
            .collect();
        assert_eq!(cu.num_cells(), 2);
    }

    #[test]
    fn test_whole_sphere() {
        // All 6 face cells should cover the whole sphere.
        let cu = CellUnion::from_cell_ids((0..6).map(CellId::from_face).collect());
        assert_eq!(cu.num_cells(), 6);
        assert!(cu.contains_point(Point::from_coords(1.0, 0.0, 0.0)));
        assert!(cu.contains_point(Point::from_coords(-1.0, 0.0, 0.0)));
        assert!(cu.contains_point(Point::from_coords(0.0, 1.0, 0.0)));
        assert!(cu.contains_point(Point::from_coords(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_is_normalized() {
        // A single face cell is normalized.
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        assert!(cu.is_normalized());

        // Non-normalized: parent + child.
        let parent = CellId::from_face(0);
        let mut cu2 = CellUnion(vec![parent, parent.children()[0]]);
        assert!(!cu2.is_normalized());
        cu2.normalize();
        assert!(cu2.is_normalized());
        assert_eq!(cu2.num_cells(), 1);
    }

    #[test]
    fn test_contains_all_children() {
        // A face cell should contain all its children at any level.
        let face0 = CellId::from_face(0);
        let cu = CellUnion::from_cell_ids(vec![face0]);

        // Level 1 children.
        for child in face0.children() {
            assert!(cu.contains_cell_id(child));
        }
        // Level 2 grandchildren.
        for child in face0.children() {
            for grandchild in child.children() {
                assert!(cu.contains_cell_id(grandchild));
            }
        }
    }

    #[test]
    fn test_intersects_with_children() {
        // A cell union of face 0 should intersect any child of face 0.
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let child_cu = CellUnion::from_cell_ids(vec![CellId::from_face(0).children()[2]]);
        assert!(cu.intersects_union(&child_cu));
        assert!(child_cu.intersects_union(&cu));
    }

    #[test]
    fn test_denormalize_levels() {
        // Denormalize at level 2 should produce 16 cells (4^2).
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let den = cu.denormalize(Level::new(2), 1);
        assert_eq!(den.num_cells(), 16);
        for id in den.cell_ids() {
            assert_eq!(id.level(), 2);
        }
    }

    #[test]
    fn test_leaf_cells_covered_exact() {
        // A single level-30 (leaf) cell covers exactly 1 leaf cell.
        let leaf = CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL);
        let cu = CellUnion::from_cell_ids(vec![leaf]);
        assert_eq!(cu.leaf_cells_covered(), 1);
    }

    #[test]
    fn test_denormalize() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let den = cu.denormalize(Level::new(1), 1);
        assert_eq!(den.num_cells(), 4);
        for id in den.cell_ids() {
            assert_eq!(id.level(), 1);
        }
    }

    #[test]
    fn test_whole_sphere_factory() {
        let cu = CellUnion::whole_sphere();
        assert_eq!(cu.num_cells(), 6);
        // Should contain every face.
        for f in 0..6 {
            assert!(cu.contains_cell_id(CellId::from_face(f)));
        }
        // Should contain any arbitrary point.
        assert!(cu.contains_point(Point::from_coords(0.3, 0.7, -0.5)));
    }

    #[test]
    fn test_is_empty() {
        assert!(CellUnion::new().is_empty());
        assert!(!CellUnion::from_cell_ids(vec![CellId::from_face(0)]).is_empty());
    }

    #[test]
    fn test_from_min_max() {
        // from_min_max should create a range covering both endpoints.
        let min_id = CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL);
        let max_id = CellId::from_face(0).children()[0]
            .child_end_at_level(MAX_CELL_LEVEL)
            .prev();
        let cu = CellUnion::from_min_max(min_id, max_id);
        assert!(cu.contains_cell_id(min_id));
        assert!(cu.contains_cell_id(max_id));
    }

    #[test]
    fn test_from_begin_end() {
        // A face's begin..end should reconstruct the face cell.
        let face = CellId::from_face(0);
        let cu = CellUnion::from_begin_end(
            face.child_begin_at_level(MAX_CELL_LEVEL),
            face.child_end_at_level(MAX_CELL_LEVEL),
        );
        assert_eq!(cu.num_cells(), 1);
        assert_eq!(cu[0], face);
    }

    #[test]
    fn test_union() {
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let cu2 = CellUnion::from_cell_ids(vec![CellId::from_face(1)]);
        let result = cu1.union(&cu2);
        assert_eq!(result.num_cells(), 2);
        assert!(result.contains_cell_id(CellId::from_face(0)));
        assert!(result.contains_cell_id(CellId::from_face(1)));
    }

    #[test]
    fn test_intersection() {
        // Intersection of face 0 with its first child should be the child.
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let child = CellId::from_face(0).children()[0];
        let cu2 = CellUnion::from_cell_ids(vec![child]);
        let result = cu1.intersection(&cu2);
        assert_eq!(result.num_cells(), 1);
        assert_eq!(result[0], child);
    }

    #[test]
    fn test_intersection_disjoint() {
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let cu2 = CellUnion::from_cell_ids(vec![CellId::from_face(3)]);
        let result = cu1.intersection(&cu2);
        assert!(result.is_empty());
    }

    #[test]
    fn test_intersection_with_cell_id() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let child = CellId::from_face(0).children()[0];
        let result = cu.intersection_with_cell_id(child);
        assert_eq!(result.num_cells(), 1);
        assert_eq!(result[0], child);
    }

    #[test]
    fn test_difference() {
        // Difference of face 0 minus its first child should leave 3 children.
        let cu1 = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let child = CellId::from_face(0).children()[0];
        let cu2 = CellUnion::from_cell_ids(vec![child]);
        let result = cu1.difference(&cu2);
        // Face minus one quarter = three children.
        assert_eq!(result.num_cells(), 3);
        assert!(!result.contains_cell_id(child));
        // But should still contain the other three children.
        for (i, c) in CellId::from_face(0).children().iter().enumerate() {
            if i > 0 {
                assert!(result.contains_cell_id(*c));
            }
        }
    }

    #[test]
    fn test_expand_at_level() {
        // Start with a small cell and expand.
        let id = CellId::from_face(0).children()[0].children()[0];
        let mut cu = CellUnion::from_cell_ids(vec![id]);
        let orig_num = cu.num_cells();
        cu.expand_at_level(id.level());
        // After expansion, should have more cells and still be normalized.
        assert!(cu.num_cells() > orig_num);
        assert!(cu.is_normalized());
        assert!(cu.contains_cell_id(id));
    }

    #[test]
    fn test_area_methods() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0)]);
        let avg = cu.average_based_area();
        let approx = cu.approx_area();
        let exact = cu.exact_area();
        // A face cell should be approximately 1/6 of the sphere (4*PI/6).
        let expected = 4.0 * std::f64::consts::PI / 6.0;
        assert!((avg - expected).abs() < 1e-10);
        assert!((approx - expected).abs() / expected < 0.01);
        assert!((exact - expected).abs() / expected < 1e-10);
    }

    #[test]
    fn test_whole_sphere_area() {
        let cu = CellUnion::whole_sphere();
        let exact = cu.exact_area();
        let expected = 4.0 * std::f64::consts::PI;
        assert!(
            (exact - expected).abs() / expected < 1e-10,
            "exact={exact}, expected={expected}"
        );
    }

    // ─── C++ port: validity tests ───────────────────────────────────────

    #[test]
    fn test_invalid_cell_id_not_valid() {
        // C++: InvalidCellIdNotValid
        assert!(!CellId::none().is_valid());
        let cu = from_verbatim(vec![CellId::none()]);
        assert!(!cu.is_valid());
    }

    #[test]
    fn test_duplicate_cells_not_valid() {
        // C++: DuplicateCellsNotValid
        let id = CellId::from_point(&Point::from_coords(1.0, 0.0, 0.0));
        let cu = from_verbatim(vec![id, id]);
        assert!(!cu.is_valid());
    }

    #[test]
    fn test_unsorted_cells_not_valid() {
        // C++: UnsortedCellsNotValid
        let id = CellId::from_point(&Point::from_coords(1.0, 0.0, 0.0)).parent_at_level(10);
        let cu = from_verbatim(vec![id, id.prev()]);
        assert!(!cu.is_valid());
    }

    // ─── C++ port: empty operations ─────────────────────────────────────

    #[test]
    fn test_empty_mutable_ops() {
        // C++: EmptyMutableOps
        let mut cu = CellUnion::new();

        // Normalize empty.
        cu.normalize();
        assert!(cu.is_empty());

        // Denormalize empty.
        let output = cu.denormalize(Level::MIN, 2);
        assert!(output.is_empty());

        // Expand by radius on empty.
        cu.expand_by_radius(Angle::from_radians(1.0), 20);
        assert!(cu.is_empty());

        // Expand at level on empty.
        cu.expand_at_level(Level::new(10));
        assert!(cu.is_empty());
    }

    #[test]
    fn test_empty_and_non_empty_boolean_ops() {
        // C++: EmptyAndNonEmptyBooleanOps
        let empty = CellUnion::new();
        let face1_id = CellId::from_face(1);
        let non_empty = CellUnion::from_cell_ids(vec![face1_id]);

        // Contains cell_id.
        assert!(!empty.contains_cell_id(face1_id));
        assert!(non_empty.contains_cell_id(face1_id));

        // Contains union.
        assert!(empty.contains_union(&empty));
        assert!(non_empty.contains_union(&empty));
        assert!(!empty.contains_union(&non_empty));
        assert!(non_empty.contains_union(&non_empty));

        // Intersects cell_id.
        assert!(!empty.intersects_cell_id(face1_id));
        assert!(non_empty.intersects_cell_id(face1_id));

        // Intersects union.
        assert!(!empty.intersects_union(&empty));
        assert!(!non_empty.intersects_union(&empty));
        assert!(!empty.intersects_union(&non_empty));
        assert!(non_empty.intersects_union(&non_empty));

        // Union.
        assert!(empty.union(&empty).is_empty());
        assert_eq!(non_empty.union(&empty).len(), non_empty.len());
        assert_eq!(empty.union(&non_empty).len(), non_empty.len());
        assert_eq!(non_empty.union(&non_empty).len(), non_empty.len());

        // Intersection.
        assert!(empty.intersection_with_cell_id(face1_id).is_empty());
        assert_eq!(
            non_empty.intersection_with_cell_id(face1_id).len(),
            non_empty.len()
        );
        assert!(empty.intersection(&empty).is_empty());
        assert!(non_empty.intersection(&empty).is_empty());
        assert!(empty.intersection(&non_empty).is_empty());
        assert_eq!(non_empty.intersection(&non_empty).len(), non_empty.len());

        // Difference.
        assert!(empty.difference(&empty).is_empty());
        assert_eq!(non_empty.difference(&empty).len(), non_empty.len());
        assert!(empty.difference(&non_empty).is_empty());
        assert!(non_empty.difference(&non_empty).is_empty());
    }

    // ─── C++ port: intersection normalizes input ────────────────────────

    #[test]
    fn test_intersection_one_input_normalized() {
        // C++: IntersectionOneInputNormalized
        let id = CellId::from_face(3);
        let parent = CellUnion::from_cell_ids(vec![id]);
        let ch = id.children();
        let children = from_verbatim(vec![ch[0], ch[1], ch[2], ch[3]]);
        let intersection = parent.intersection(&children);
        // The intersection should contain all four children (equals the
        // children union after normalization).
        assert_eq!(intersection.len(), children.len());
    }

    // ─── C++ port: Display / ToString ───────────────────────────────────

    #[test]
    fn test_to_string_empty() {
        // C++: ToStringEmpty
        assert_eq!(CellUnion::new().to_string(), "Size:0 S2CellIds:");
    }

    #[test]
    fn test_to_string_one_cell() {
        // C++: ToStringOneCell
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(1)]);
        assert_eq!(cu.to_string(), "Size:1 S2CellIds:3");
    }

    #[test]
    fn test_to_string_two_cells() {
        // C++: ToStringTwoCells
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(1), CellId::from_face(2)]);
        assert_eq!(cu.to_string(), "Size:2 S2CellIds:3,5");
    }

    #[test]
    fn test_to_string_truncated() {
        // C++: ToStringOver500Cells
        // Denormalize face 1 to level 6 → 4^6 = 4096 cells.
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(1)]);
        let denorm = cu.denormalize(Level::new(6), 1);
        let s = denorm.to_string();
        // Should have exactly 500 commas (501 tokens truncated to 500).
        assert_eq!(s.matches(',').count(), 500);
        assert!(s.ends_with(",..."));
    }

    // ─── C++ port: expand_by_radius ─────────────────────────────────────

    #[test]
    fn test_expand_by_radius() {
        // Verify that expand_by_radius on a single face cell expands it.
        let id = CellId::from_face(0);
        let mut cu = CellUnion::from_cell_ids(vec![id]);
        let original_size = cu.num_cells();
        cu.expand_by_radius(Angle::from_degrees(1.0), 3);
        // After expansion, should have more cells.
        assert!(
            cu.num_cells() > original_size,
            "expand_by_radius should add cells"
        );
        assert!(cu.is_normalized());
        // Original cell should still be covered.
        assert!(cu.contains_cell_id(id));
    }

    #[test]
    fn test_expand_by_radius_small() {
        // Expand a leaf cell by a tiny radius.
        let center = Point::from_coords(1.0, 0.0, 0.0);
        let id = CellId::from_point(&center);
        let mut cu = CellUnion::from_cell_ids(vec![id]);
        cu.expand_by_radius(Angle::from_radians(1e-10), 5);
        assert!(cu.is_normalized());
        assert!(cu.contains_point(center));
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_normalized_contains_original(face: u8) -> bool {
        let face = face % 6;
        let id = CellId::from_face(face);
        let children = id.children();
        let cu = CellUnion::from_cell_ids(children.to_vec());
        // After normalization (siblings merge to parent), contains all children.
        children.iter().all(|c| cu.contains_cell_id(*c))
    }

    #[quickcheck]
    fn prop_from_cell_ids_is_normalized(face: u8) -> bool {
        let face = face % 6;
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(face)]);
        cu.is_normalized()
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(face: u8) -> bool {
        let face = face % 6;
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(face)]);
        let json = serde_json::to_string(&cu).unwrap();
        let back: CellUnion = serde_json::from_str(&json).unwrap();
        serde_json::to_string(&back).unwrap() == json
    }
}
