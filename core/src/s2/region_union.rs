// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Union of possibly overlapping regions.
//!
//! [`RegionUnion`] represents a union of possibly overlapping [`Region`]s.
//! It is convenient for computing a covering of a set of regions.
//!
//! Note that when using [`RegionCoverer`](super::region_coverer::RegionCoverer)
//! to compute coverings of `RegionUnion`s, overlapping or tiling regions may
//! produce coverings with fewer than the requested number of cells, because
//! `contains_cell` only returns true if the cell is fully contained by one
//! region.
//!
//! Corresponds to C++ `s2region_union.h`, Go `s2/regionunion.go`.

use crate::s2::region::Region;
use crate::s2::{Cap, Cell, CellId, Point, Rect};

/// A union of possibly overlapping regions.
#[derive(Debug)]
pub struct RegionUnion {
    regions: Vec<Box<dyn Region>>,
}

impl RegionUnion {
    /// Creates a new empty `RegionUnion`.
    pub fn new() -> Self {
        RegionUnion {
            regions: Vec::new(),
        }
    }

    /// Creates a `RegionUnion` from a vector of regions.
    pub fn from_regions(regions: Vec<Box<dyn Region>>) -> Self {
        RegionUnion { regions }
    }

    /// Adds a region to the union.
    pub fn add(&mut self, region: Box<dyn Region>) {
        self.regions.push(region);
    }

    /// Returns the number of regions in the union.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Returns true if the union contains no regions.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Returns an iterator over the regions.
    pub fn iter(&self) -> std::slice::Iter<'_, Box<dyn Region>> {
        self.regions.iter()
    }
}

impl<'a> IntoIterator for &'a RegionUnion {
    type Item = &'a Box<dyn Region>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Region>>;

    fn into_iter(self) -> Self::IntoIter {
        self.regions.iter()
    }
}

impl IntoIterator for RegionUnion {
    type Item = Box<dyn Region>;
    type IntoIter = std::vec::IntoIter<Box<dyn Region>>;

    fn into_iter(self) -> Self::IntoIter {
        self.regions.into_iter()
    }
}

impl Default for RegionUnion {
    fn default() -> Self {
        Self::new()
    }
}

impl Region for RegionUnion {
    fn cap_bound(&self) -> Cap {
        self.rect_bound().cap_bound()
    }

    fn rect_bound(&self) -> Rect {
        let mut ret = Rect::empty();
        for region in &self.regions {
            ret = ret.union(region.rect_bound());
        }
        ret
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        for region in &self.regions {
            if region.contains_cell(cell) {
                return true;
            }
        }
        false
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        for region in &self.regions {
            if region.intersects_cell(cell) {
                return true;
            }
        }
        false
    }

    fn contains_point(&self, p: &Point) -> bool {
        for region in &self.regions {
            if region.contains_point(p) {
                return true;
            }
        }
        false
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty_region_union_has_empty_cap() {
        let ru = RegionUnion::new();
        assert!(ru.cap_bound().is_empty());
    }

    #[test]
    fn test_empty_region_union_has_empty_rect() {
        let ru = RegionUnion::new();
        assert!(ru.rect_bound().is_empty());
    }

    #[test]
    fn test_region_union_of_two_points_has_correct_bound() {
        let p1 = p(0.0, 0.0);
        let p2 = p(0.0, 10.0);
        let cap1 = Cap::from_point(p1);
        let cap2 = Cap::from_point(p2);
        let ru = RegionUnion::from_regions(vec![Box::new(cap1), Box::new(cap2)]);

        let bound = ru.rect_bound();
        assert!(!bound.is_empty());
    }

    #[test]
    fn test_region_union_contains_point() {
        let p1 = p(0.0, 0.0);
        let p2 = p(0.0, 90.0);
        let cap1 = Cap::from_center_angle(p1, s1::Angle::from_degrees(5.0));
        let cap2 = Cap::from_center_angle(p2, s1::Angle::from_degrees(5.0));
        let ru = RegionUnion::from_regions(vec![Box::new(cap1), Box::new(cap2)]);

        // Points near cap centers should be contained
        assert!(ru.contains_point(&p(0.0, 0.0)));
        assert!(ru.contains_point(&p(0.0, 90.0)));

        // A point far from both should not be contained
        assert!(!ru.contains_point(&p(45.0, 45.0)));
    }

    #[test]
    fn test_region_union_intersects_cell() {
        let center = p(0.0, 0.0);
        let cap = Cap::from_center_angle(center, s1::Angle::from_degrees(5.0));
        let ru = RegionUnion::from_regions(vec![Box::new(cap)]);

        // Face 0 cell should intersect (it contains the equator at lng=0)
        let face0 = Cell::from(CellId::from_face(0));
        assert!(ru.intersects_cell(&face0));
    }

    #[test]
    fn test_region_union_contains_cell() {
        // A full cap should contain any cell
        let cap = Cap::full();
        let ru = RegionUnion::from_regions(vec![Box::new(cap)]);

        let face0 = Cell::from(CellId::from_face(0));
        assert!(ru.contains_cell(&face0));
    }

    #[test]
    fn test_region_union_default() {
        let ru = RegionUnion::default();
        assert!(ru.is_empty());
        assert_eq!(ru.len(), 0);
    }

    #[test]
    fn test_region_union_add() {
        let mut ru = RegionUnion::new();
        assert_eq!(ru.len(), 0);
        ru.add(Box::new(Cap::empty()));
        assert_eq!(ru.len(), 1);
        ru.add(Box::new(Cap::full()));
        assert_eq!(ru.len(), 2);
    }
}
