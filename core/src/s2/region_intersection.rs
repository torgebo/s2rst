// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Intersection of multiple regions on the unit sphere.
//!
//! [`RegionIntersection`] represents the intersection of a set of regions.
//! It is convenient for computing a covering of the intersection of a set
//! of regions.
//!
//! Corresponds to C++ `s2region_intersection.h/cc`.

use crate::s2::{Cap, Cell, CellId, Point, Rect, Region};

/// A region representing the intersection of multiple regions.
///
/// An intersection of no regions covers the entire sphere.
#[derive(Debug)]
pub struct RegionIntersection {
    regions: Vec<Box<dyn Region>>,
}

impl RegionIntersection {
    /// Creates an empty intersection (covers the entire sphere).
    pub fn new() -> Self {
        RegionIntersection {
            regions: Vec::new(),
        }
    }

    /// Creates a region representing the intersection of the given regions.
    pub fn from_regions(regions: Vec<Box<dyn Region>>) -> Self {
        RegionIntersection { regions }
    }

    /// Returns the number of regions in this intersection.
    pub fn num_regions(&self) -> usize {
        self.regions.len()
    }

    /// Returns a reference to the i-th region.
    pub fn region(&self, i: usize) -> &dyn Region {
        self.regions[i].as_ref()
    }
}

impl Default for RegionIntersection {
    fn default() -> Self {
        Self::new()
    }
}

impl Region for RegionIntersection {
    fn cap_bound(&self) -> Cap {
        self.rect_bound().cap_bound()
    }

    fn rect_bound(&self) -> Rect {
        let mut result = Rect::full();
        for region in &self.regions {
            result = result.intersection(region.rect_bound());
        }
        result
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        self.regions.iter().all(|r| r.contains_cell(cell))
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        self.regions.iter().all(|r| r.intersects_cell(cell))
    }

    fn contains_point(&self, p: &Point) -> bool {
        self.regions.iter().all(|r| r.contains_point(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_intersection() {
        // Empty intersection covers the entire sphere.
        let r = RegionIntersection::new();
        assert!(r.rect_bound().is_full());
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_single_region() {
        let cap = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
        let r = RegionIntersection::from_regions(vec![Box::new(cap)]);
        assert_eq!(r.num_regions(), 1);
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
        assert!(!r.contains_point(&Point::from_coords(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_two_overlapping_caps() {
        use crate::s1::ChordAngle;

        let cap1 = Cap::from_center_chord_angle(
            Point::from_coords(1.0, 0.0, 0.0),
            ChordAngle::from_length2(1.0),
        );
        let cap2 = Cap::from_center_chord_angle(
            Point::from_coords(0.0, 1.0, 0.0),
            ChordAngle::from_length2(1.0),
        );
        let r = RegionIntersection::from_regions(vec![Box::new(cap1), Box::new(cap2)]);
        assert_eq!(r.num_regions(), 2);

        // A point near (1,1,0) normalized should be in both caps.
        let p = Point(
            crate::r3::Vector {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            }
            .normalize(),
        );
        assert!(r.contains_point(&p));

        // (1,0,0) might not be in cap2 depending on radius.
        // (-1,0,0) is definitely not in cap1.
        assert!(!r.contains_point(&Point::from_coords(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_non_overlapping_caps() {
        let cap1 = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
        let cap2 = Cap::from_point(Point::from_coords(-1.0, 0.0, 0.0));
        let r = RegionIntersection::from_regions(vec![Box::new(cap1), Box::new(cap2)]);
        // Non-overlapping point caps — nothing in common.
        assert!(!r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
        assert!(!r.contains_point(&Point::from_coords(-1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_cell_containment() {
        let cell = Cell::from_cell_id(CellId::from_face(0));
        let full = Rect::full();
        let r = RegionIntersection::from_regions(vec![Box::new(full)]);
        // Full rect contains all cells.
        assert!(r.contains_cell(&cell));
        assert!(r.intersects_cell(&cell));
    }

    #[test]
    fn test_as_dyn_region() {
        let cap = Cap::from_point(Point::from_coords(1.0, 0.0, 0.0));
        let intersection = RegionIntersection::from_regions(vec![Box::new(cap)]);
        let r: &dyn Region = &intersection;
        assert!(r.contains_point(&Point::from_coords(1.0, 0.0, 0.0)));
    }
}
