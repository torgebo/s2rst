// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! [`ShapeIndexBufferedRegion`] wraps a [`ShapeIndex`] to implement the
//! [`Region`] trait with a given buffer radius.
//!
//! This expands the indexed geometry by a fixed radius so that
//! containment/intersection tests behave as if each edge and vertex had been
//! dilated by that amount. It is used primarily with [`RegionCoverer`](super::region_coverer::RegionCoverer) to
//! compute cell coverings of buffered geometry.
//!
//! Corresponds to C++ `s2shape_index_buffered_region.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "level (i32) cast to u8 after clamping to valid range"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "level (i32->u8) after clamping"
)]
use crate::s1;
use crate::s1::ChordAngle;
use crate::s2::closest_edge_query::{CellTarget, ClosestEdgeQuery, PointTarget};
use crate::s2::coords::Level;
use crate::s2::metric::MIN_WIDTH;
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_index_region::ShapeIndexRegion;
use crate::s2::{Cap, Cell, CellId, Point, Rect, Region};

/// A [`ShapeIndex`] expanded by a buffer radius, implementing [`Region`].
///
/// All containment and intersection tests are performed as if every point
/// of the indexed geometry had been dilated by `radius`. This is useful
/// for computing cell coverings of geometry with a safety margin.
#[derive(Debug)]
pub struct ShapeIndexBufferedRegion<'a> {
    index: &'a ShapeIndex,
    radius: ChordAngle,
    radius_successor: ChordAngle,
}

impl<'a> ShapeIndexBufferedRegion<'a> {
    /// Creates a new buffered region for `index` with the given `radius`.
    pub fn new(index: &'a ShapeIndex, radius: ChordAngle) -> Self {
        ShapeIndexBufferedRegion {
            index,
            radius,
            radius_successor: radius.successor(),
        }
    }

    /// Creates a new buffered region using an `s1::Angle` for the radius.
    pub fn from_angle(index: &'a ShapeIndex, radius: s1::Angle) -> Self {
        Self::new(index, ChordAngle::from_angle(radius))
    }

    /// Returns the underlying index.
    pub fn index(&self) -> &'a ShapeIndex {
        self.index
    }

    /// Returns the buffer radius.
    pub fn radius(&self) -> ChordAngle {
        self.radius
    }
}

impl Region for ShapeIndexBufferedRegion<'_> {
    fn cap_bound(&self) -> Cap {
        let orig = ShapeIndexRegion::new(self.index).cap_bound();
        if orig.is_empty() {
            return Cap::empty();
        }
        let combined = if orig.chord_radius().is_special() || self.radius.is_special() {
            ChordAngle::STRAIGHT
        } else {
            orig.chord_radius() + self.radius
        };
        Cap::from_center_chord_angle(orig.center(), combined)
    }

    fn rect_bound(&self) -> Rect {
        let orig = ShapeIndexRegion::new(self.index).rect_bound();
        orig.expanded_by_distance(self.radius.to_angle())
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        let radians = self.radius.to_angle().radians();
        let max_level = i32::from(MIN_WIDTH.min_level(radians)) - 1;
        if max_level < 0 {
            return Cap::full().cell_union_bound();
        }

        let orig_cells = ShapeIndexRegion::new(self.index).cell_union_bound();
        let mut result = Vec::new();
        for id in orig_cells {
            if id.is_face() {
                return Cap::full().cell_union_bound();
            }
            let level = std::cmp::min(Level::new(max_level as u8), id.level() - 1u8);
            let neighbors = id.vertex_neighbors(level);
            result.extend(neighbors);
        }
        result
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        // If the buffer radius covers the whole globe, everything is contained.
        if self.radius_successor > ChordAngle::STRAIGHT {
            return true;
        }

        // If the unbuffered region already contains the cell, we're done.
        if ShapeIndexRegion::new(self.index).contains_cell(cell) {
            return true;
        }

        // Approximate the cell by its bounding cap.
        let cap = cell.cap_bound();
        if self.radius < cap.chord_radius() {
            return false;
        }

        // Check if distance to cell center plus cap radius <= buffer radius.
        let query = ClosestEdgeQuery::new(self.index);
        let target = PointTarget::new(cell.center());
        query.is_distance_less(&target, self.radius_successor - cap.chord_radius())
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        let query = ClosestEdgeQuery::new(self.index);
        let target = CellTarget::new(*cell);
        query.is_distance_less(&target, self.radius_successor)
    }

    fn contains_point(&self, p: &Point) -> bool {
        let query = ClosestEdgeQuery::new(self.index);
        let target = PointTarget::new(*p);
        query.is_distance_less(&target, self.radius_successor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::region_coverer::RegionCoverer;
    use crate::s2::text_format;

    fn make_index_with_points(points: &[Point]) -> ShapeIndex {
        use crate::s2::point_vector::PointVector;
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(points.to_vec());
        index.add(Box::new(pv));
        index.build();
        index
    }

    fn make_index_with_polyline(text: &str) -> ShapeIndex {
        use crate::s2::lax_polyline::LaxPolyline;
        let pl = text_format::make_polyline(text);
        let vertices: Vec<Point> = (0..pl.num_vertices()).map(|i| pl.vertex(i)).collect();
        let mut index = ShapeIndex::new();
        let lax = LaxPolyline::new(vertices);
        index.add(Box::new(lax));
        index.build();
        index
    }

    #[test]
    fn test_empty_index() {
        let index = ShapeIndex::new();
        let region =
            ShapeIndexBufferedRegion::new(&index, ChordAngle::from_angle(Angle::from_degrees(1.0)));
        let cells = region.cell_union_bound();
        assert!(cells.is_empty());
    }

    #[test]
    fn test_point_zero_radius() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let index = make_index_with_points(&[p]);
        let region = ShapeIndexBufferedRegion::new(&index, ChordAngle::ZERO);

        // The point itself should be contained (using successor semantics).
        assert!(region.contains_point(&p));

        // A far-away point should not be contained.
        let far = Point::from_coords(0.0, 1.0, 0.0);
        assert!(!region.contains_point(&far));
    }

    #[test]
    fn test_point_with_radius() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let index = make_index_with_points(&[p]);
        let radius = ChordAngle::from_angle(Angle::from_degrees(5.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        // A nearby point within radius should be contained.
        let nearby = Point::from_coords(1.0, 0.01, 0.0).normalize();
        assert!(region.contains_point(&nearby));

        // A far-away point should not be contained.
        let far = Point::from_coords(0.0, 1.0, 0.0);
        assert!(!region.contains_point(&far));
    }

    #[test]
    fn test_point_set() {
        // Three well-separated points with a 5-degree buffer.
        let p1 = text_format::parse_point("0:0");
        let p2 = text_format::parse_point("0:90");
        let p3 = text_format::parse_point("90:0");
        let index = make_index_with_points(&[p1, p2, p3]);
        let radius = ChordAngle::from_angle(Angle::from_degrees(5.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        // Points near each of the three should be contained.
        assert!(region.contains_point(&text_format::parse_point("1:0")));
        assert!(region.contains_point(&text_format::parse_point("0:91")));
        assert!(region.contains_point(&text_format::parse_point("89:0")));

        // A far-away point should not be contained.
        assert!(!region.contains_point(&text_format::parse_point("45:45")));
    }

    #[test]
    fn test_polyline_buffered() {
        let index = make_index_with_polyline("0:0, 0:10, 10:10");
        let radius = ChordAngle::from_angle(Angle::from_degrees(2.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        // A point on the polyline should be contained.
        let on_line = Point::from_coords(1.0, 0.0, 0.0);
        assert!(region.contains_point(&on_line));

        // A point near the polyline within radius should be contained.
        let nearby = text_format::parse_point("1:5");
        assert!(region.contains_point(&nearby));

        // A point far from the polyline should not be contained.
        let far = text_format::parse_point("45:45");
        assert!(!region.contains_point(&far));
    }

    #[test]
    fn test_huge_buffer_radius() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let index = make_index_with_points(&[p]);
        let radius = ChordAngle::from_angle(Angle::from_degrees(200.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        // With a huge radius, the cell_union_bound should be the full sphere.
        let cells = region.cell_union_bound();
        assert_eq!(cells.len(), 6);
    }

    #[test]
    fn test_cap_bound_not_empty() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let index = make_index_with_points(&[p]);
        let radius = ChordAngle::from_angle(Angle::from_degrees(10.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        let cap = region.cap_bound();
        // The cap bound should not be empty.
        assert!(!cap.is_empty());
        // It should contain the original point.
        assert!(cap.contains_point(p));
    }

    #[test]
    fn test_rect_bound_expansion() {
        let index = make_index_with_polyline("0:0, 0:10");
        let radius = ChordAngle::from_angle(Angle::from_degrees(1.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);

        let rect = region.rect_bound();
        // The rect bound should be larger than the original polyline extent.
        let orig_rect = ShapeIndexRegion::new(region.index()).rect_bound();
        assert!(rect.lat.length() > orig_rect.lat.length());
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2shape_index_buffered_region_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_full_polygon() {
        // C++ TEST(S2ShapeIndexBufferedRegion, FullPolygon)
        let index = text_format::make_index("# # full");
        let radius = ChordAngle::from_angle(Angle::from_degrees(2.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);
        let covering = RegionCoverer::new().covering(&region);
        assert_eq!(6, covering.num_cells());
        for &id in covering.cell_ids() {
            assert!(id.is_face(), "expected face cell, got {id:?}");
        }
    }

    #[test]
    fn test_full_after_buffering() {
        // C++ TEST(S2ShapeIndexBufferedRegion, FullAfterBuffering)
        let index = text_format::make_index("0:0 | 0:90 | 0:180 | 0:-90 | 90:0 | -90:0 # #");
        let radius = ChordAngle::from_angle(Angle::from_degrees(60.0));
        let region = ShapeIndexBufferedRegion::new(&index, radius);
        let coverer = RegionCoverer::new().max_cells(1000);
        let covering = coverer.covering(&region);
        assert_eq!(6, covering.num_cells());
        for &id in covering.cell_ids() {
            assert!(id.is_face());
        }
    }
}
