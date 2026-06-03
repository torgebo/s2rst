// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A region containing a single point on the unit sphere.
//!
//! [`PointRegion`] wraps an [`Point`] to implement the [`Region`] trait.
//! Mainly useful for completeness and uniform API handling.
//!
//! Corresponds to C++ `s2point_region.h/cc`.

use crate::s2::{Cap, Cell, CellId, LatLng, Point, Rect, Region};

/// A region that contains a single point.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointRegion {
    point: Point,
}

impl PointRegion {
    /// Creates a new region containing the given point.
    pub fn new(point: Point) -> Self {
        PointRegion { point }
    }

    /// Returns the contained point.
    pub fn point(&self) -> Point {
        self.point
    }
}

impl Region for PointRegion {
    fn cap_bound(&self) -> Cap {
        Cap::from_point(self.point)
    }

    fn rect_bound(&self) -> Rect {
        let ll = LatLng::from_point(self.point);
        Rect::from_lat_lng(ll)
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, _cell: &Cell) -> bool {
        false
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        Cell::contains_point(*cell, self.point)
    }

    fn contains_point(&self, p: &Point) -> bool {
        self.point == *p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    #[test]
    fn test_basic() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let r = PointRegion::new(p);
        assert_eq!(r.point(), p);
        assert!(r.contains_point(&p));
        assert!(!r.contains_point(&Point::from_coords(1.0, 0.0, 1.0)));

        // Copy
        let r_copy = r;
        assert_eq!(r_copy.point(), r.point());

        // Cap bound
        assert_eq!(r.cap_bound(), Cap::from_point(p));

        // Rect bound
        let ll = LatLng::from_point(p);
        let expected_rect = Rect::from_lat_lng(ll);
        assert_eq!(r.rect_bound(), expected_rect);

        // A leaf cell is still larger than a point.
        let cell = Cell::from_point(p);
        assert!(!r.contains_cell(&cell));
        assert!(r.intersects_cell(&cell));
    }

    #[test]
    fn test_cell_union_bound() {
        let p = Point::from_coords(0.0, 1.0, 0.0);
        let r = PointRegion::new(p);
        let cells = r.cell_union_bound();
        assert!(!cells.is_empty());
    }

    #[test]
    fn test_as_region() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let r: Box<dyn Region> = Box::new(PointRegion::new(p));
        assert!(r.contains_point(&p));
        assert!(!r.cap_bound().is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let pr = PointRegion::new(Point::from_coords(1.0, 0.0, 0.0));
        let json = serde_json::to_string(&pr).unwrap();
        let back: PointRegion = serde_json::from_str(&json).unwrap();
        assert_eq!(pr, back);
    }
}
