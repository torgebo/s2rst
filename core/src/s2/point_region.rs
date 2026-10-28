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
///
/// With the `serde` feature, deserialisation rejects a point that is not unit length
/// ([`Point::is_unit`]), as upstream's `S2PointRegion::Decode` does (spec 011 FR-019).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PointRegion {
    point: Point,
}

impl PointRegion {
    /// Creates a new region containing the given point.
    ///
    /// The point should be unit length: upstream's constructor requires it (a debug-build
    /// check), and the `Region` contract's laws hold for unit points. The library does not
    /// check it. Any other point is kept as given, and each method answers for that vector: the
    /// bounds are those of [`Cap::from_point`], [`LatLng::from_point`] and their coverings, the
    /// cell test is [`Cell::contains_point`]'s, and [`Region::contains_point`] compares the
    /// coordinates with `==` (so `(0, 0, 1)` and `(-0, -0, 1)` are one point, and a point with
    /// a NaN coordinate contains nothing, itself included).
    pub fn new(point: Point) -> Self {
        PointRegion { point }
    }

    /// Returns the contained point.
    pub fn point(&self) -> Point {
        self.point
    }
}

/// Rejects a point that is not unit length, as upstream's `S2PointRegion::Decode`
/// (`S2::IsUnitLength`) and `Cap`'s deserialiser do (spec 011 FR-019); the derived impl would
/// accept any three numbers.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PointRegion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        #[derive(serde::Deserialize)]
        #[serde(rename = "PointRegion")]
        struct Fields {
            point: Point,
        }

        let f = Fields::deserialize(deserializer)?;
        if !f.point.is_unit() {
            return Err(D::Error::custom("point region's point is not unit length"));
        }
        Ok(PointRegion { point: f.point })
    }
}

/// # Panics
///
/// For a unit point, none. For other points, in debug builds, where the functions they call
/// check what upstream's checks with `ABSL_DCHECK` (spec 011 K4); release builds skip the
/// checks:
/// - `rect_bound`, if `x` or `y` is NaN: the longitude is then NaN, which
///   [`Rect::from_lat_lng`] rejects;
/// - `intersects_cell`, where [`Cell::contains_point`] panics: the coordinate on the cell
///   face's axis is NaN, or it has the face's sign and another coordinate is NaN or infinite;
/// - `cell_union_bound`, where [`CellId::from_point`] panics: the zero vector, some points with
///   a NaN coordinate, and every point with two or more infinite coordinates.
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
#[path = "point_region_tests.rs"]
mod point_region_tests;

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
