// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! [`ShapeIndexRegion`] wraps a [`ShapeIndex`] to implement the [`Region`] trait.
//!
//! This allows [`RegionCoverer`](super::region_coverer::RegionCoverer) to work with shape indexes and enables
//! the index to be used by query types that accept a `Region`.
//!
//! Corresponds to C++ `s2shape_index_region.h`, Go `s2/shapeindex_region.go`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
use std::ops::ControlFlow;

use crate::r2;
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::coords::Level;
use crate::s2::edge_clipping::{self, FACE_CLIP_ERROR_UV_COORD, INTERSECTS_RECT_ERROR_UV_DIST};
use crate::s2::shape::{Dimension, ShapeId};
use crate::s2::shape_index::{CellRelation, ClippedShape, ShapeIndex};
use crate::s2::{Cap, Cell, CellId, CellUnion, Point, Rect, Region};

/// A [`ShapeIndex`] wrapped to implement the [`Region`] trait.
///
/// Provides bounding methods and point/cell containment tests based on
/// the indexed geometry.
#[derive(Debug)]
pub struct ShapeIndexRegion<'a> {
    index: &'a ShapeIndex,
}

impl<'a> ShapeIndexRegion<'a> {
    /// Creates a new `ShapeIndexRegion` for the given index.
    ///
    /// The index must already be built.
    pub fn new(index: &'a ShapeIndex) -> Self {
        ShapeIndexRegion { index }
    }

    /// Returns the underlying index.
    pub fn index(&self) -> &'a ShapeIndex {
        self.index
    }

    /// Computes the bounding `CellUnion` for this collection of geometry.
    ///
    /// Returns at most 4 cells for a single-face index, or up to 6 cells
    /// if the index spans multiple faces.
    fn compute_cell_union_bound(&self) -> Vec<CellId> {
        let mut it = self.index.iter();

        // Find the last CellId in the index.
        it.end();
        if !it.prev() {
            return Vec::new(); // Empty index.
        }
        let last_index_id = it.cell_id();

        it.begin();
        let first_index_id = it.cell_id();

        if first_index_id == last_index_id {
            // Single cell: just return it.
            return vec![first_index_id];
        }

        // Multiple cells. Choose a level such that the entire index can be
        // spanned with at most 6 cells (multi-face) or 4 cells (single-face).
        let level = match first_index_id.common_ancestor_level(last_index_id) {
            Some(l) => l + 1u8,
            None => Level::MIN, // No common ancestor (different faces).
        };

        let mut cell_ids = Vec::new();
        let last_id = last_index_id.parent_at_level(level);

        let mut id = first_index_id.parent_at_level(level);
        loop {
            let is_last = id == last_id;

            // Skip cells that don't contain any index cells.
            if id.range_max() >= it.cell_id() {
                // Find the range of index cells within this cell.
                let first = it.cell_id();
                it.seek(id.range_max().next());
                it.prev();
                Self::cover_range(first, it.cell_id(), &mut cell_ids);
                it.next();
            }

            if is_last {
                break;
            }
            id = id.next();
        }

        cell_ids
    }

    /// Reports whether the clipped shape contains the given point, using
    /// `cell_center` as the origin for edge-crossing counts.
    ///
    /// Corresponds to C++ `S2ShapeIndexRegion::Contains(clipped, p)`, which
    /// delegates to `S2ContainsPointQuery::ShapeContains(cell_id, clipped, p)`.
    fn clipped_contains(
        index: &ShapeIndex,
        clipped: &ClippedShape,
        cell_center: Point,
        p: Point,
    ) -> bool {
        let q = ContainsPointQuery::new(index, VertexModel::SemiOpen);
        q.shape_contains_impl(clipped, cell_center, p)
    }

    /// Reports whether any edge of the clipped shape intersects the (padded)
    /// interior of the target cell.
    ///
    /// Corresponds to C++ `S2ShapeIndexRegion::AnyEdgeIntersects`.
    fn any_edge_intersects(index: &ShapeIndex, clipped: &ClippedShape, target: &Cell) -> bool {
        let max_error = FACE_CLIP_ERROR_UV_COORD + INTERSECTS_RECT_ERROR_UV_DIST;
        let bound = target
            .bound_uv()
            .expanded(r2::Point::new(max_error, max_error));
        let face = target.face();
        let Some(shape) = index.shape(clipped.shape_id) else {
            return false;
        };
        for &edge_id in &clipped.edges {
            let edge = shape.edge(edge_id as usize);
            if let Some((p0, p1)) =
                edge_clipping::clip_to_padded_face(edge.v0, edge.v1, face, max_error)
                && edge_clipping::edge_intersects_rect(p0, p1, bound)
            {
                return true;
            }
        }
        false
    }

    /// Visits all shapes that intersect the given cell, calling `visitor` with
    /// the `shape_id` and a flag indicating whether the shape fully contains the
    /// target cell.
    ///
    /// The visitor should return `ControlFlow::Continue(())` to keep visiting,
    /// or `ControlFlow::Break(())` to stop early.
    ///
    /// Corresponds to C++ `S2ShapeIndexRegion::VisitIntersectingShapeIds`.
    pub fn visit_intersecting_shape_ids<F>(&self, target: &Cell, mut visitor: F) -> ControlFlow<()>
    where
        F: FnMut(ShapeId, bool) -> ControlFlow<()>,
    {
        let mut it = self.index.iter();
        let rel = it.locate_cell_id(target.id());

        match rel {
            CellRelation::Disjoint => ControlFlow::Continue(()),
            CellRelation::Subdivided => {
                let mut shape_not_contains: std::collections::HashMap<ShapeId, bool> =
                    std::collections::HashMap::new();
                let max = target.id().range_max();
                while !it.done() && it.cell_id() <= max {
                    if let Some(index_cell) = it.index_cell() {
                        for clipped in &index_cell.shapes {
                            let entry = shape_not_contains.entry(clipped.shape_id).or_insert(false);
                            *entry |= clipped.num_edges() > 0 || !clipped.contains_center;
                        }
                    }
                    it.next();
                }
                for (&shape_id, &not_contains) in &shape_not_contains {
                    visitor(shape_id, !not_contains)?;
                }
                ControlFlow::Continue(())
            }
            CellRelation::Indexed => {
                let Some(index_cell) = it.index_cell() else {
                    return ControlFlow::Continue(());
                };
                let center = it.center();
                for clipped in &index_cell.shapes {
                    let contains = if it.cell_id() == target.id() {
                        clipped.num_edges() == 0 && clipped.contains_center
                    } else if Self::any_edge_intersects(self.index, clipped, target) {
                        false
                    } else if Self::clipped_contains(self.index, clipped, center, target.center()) {
                        true
                    } else {
                        continue; // Disjoint.
                    };
                    visitor(clipped.shape_id, contains)?;
                }
                ControlFlow::Continue(())
            }
        }
    }

    /// Computes the smallest `CellId` that covers the range [first, last].
    fn cover_range(first: CellId, last: CellId, out: &mut Vec<CellId>) {
        if first == last {
            out.push(first);
            return;
        }
        match first.common_ancestor_level(last) {
            Some(level) => out.push(first.parent_at_level(level)),
            None => out.push(CellId::none()),
        }
    }
}

impl Region for ShapeIndexRegion<'_> {
    fn cap_bound(&self) -> Cap {
        let ids = self.compute_cell_union_bound();
        let cu = CellUnion::from_cell_ids(ids);
        cu.cap_bound()
    }

    fn rect_bound(&self) -> Rect {
        let ids = self.compute_cell_union_bound();
        let cu = CellUnion::from_cell_ids(ids);
        cu.rect_bound()
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.compute_cell_union_bound()
    }

    fn contains_cell(&self, cell: &Cell) -> bool {
        // If the relation is DISJOINT or SUBDIVIDED, the cell is not contained.
        // (Index cells are subdivided only if they nearly intersect too many
        // edges, so SUBDIVIDED means the target is not fully contained.)
        let mut it = self.index.iter();
        let rel = it.locate_cell_id(cell.id());
        if rel != CellRelation::Indexed {
            return false;
        }

        // The iterator points to an index cell containing "cell".
        // If any shape contains the target cell, we return true.
        debug_assert!(it.cell_id().contains(cell.id()));
        let Some(index_cell) = it.index_cell() else {
            return false;
        };
        let center = it.center();
        for clipped in &index_cell.shapes {
            // The shape contains the target cell iff the shape contains the
            // cell center and none of its edges intersects the (padded) cell
            // interior.
            if it.cell_id() == cell.id() {
                if clipped.num_edges() == 0 && clipped.contains_center {
                    return true;
                }
            } else {
                // It is faster to call any_edge_intersects() before contains().
                if let Some(shape) = self.index.shape(clipped.shape_id)
                    && shape.dimension() == Dimension::Polygon
                    && !Self::any_edge_intersects(self.index, clipped, cell)
                    && Self::clipped_contains(self.index, clipped, center, cell.center())
                {
                    return true;
                }
            }
        }
        false
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        let mut it = self.index.iter();
        let rel = it.locate_cell_id(cell.id());

        // If the target does not overlap any index cell, there is no
        // intersection.
        if rel == CellRelation::Disjoint {
            return false;
        }

        // If the target is subdivided into one or more index cells, then there
        // is an intersection to within the S2ShapeIndex error bound.
        if rel == CellRelation::Subdivided {
            return true;
        }

        // Otherwise, the iterator points to an index cell containing the
        // target.  If the target is an index cell itself, there is an
        // intersection because index cells are created only if they have at
        // least one edge or are entirely contained by a loop.
        debug_assert!(it.cell_id().contains(cell.id()));
        if it.cell_id() == cell.id() {
            return true;
        }

        // Test whether any shape intersects the target cell or contains its
        // center.
        let Some(index_cell) = it.index_cell() else {
            return false;
        };
        let center = it.center();
        for clipped in &index_cell.shapes {
            if Self::any_edge_intersects(self.index, clipped, cell) {
                return true;
            }
            if Self::clipped_contains(self.index, clipped, center, cell.center()) {
                return true;
            }
        }
        false
    }

    fn contains_point(&self, p: &Point) -> bool {
        let mut q = ContainsPointQuery::new(self.index, VertexModel::SemiOpen);
        q.contains(*p)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::shape::{Chain, ChainPosition, Edge, ReferencePoint, Shape};

    #[derive(Debug)]
    struct PointVectorShape {
        points: Vec<Point>,
    }

    impl Shape for PointVectorShape {
        fn num_edges(&self) -> usize {
            self.points.len()
        }
        fn edge(&self, id: usize) -> Edge {
            Edge::new(self.points[id], self.points[id])
        }
        fn reference_point(&self) -> ReferencePoint {
            ReferencePoint::default()
        }
        fn num_chains(&self) -> usize {
            self.points.len()
        }
        fn chain(&self, chain_id: usize) -> Chain {
            Chain::new(chain_id, 1)
        }
        fn chain_edge(&self, chain_id: usize, _offset: usize) -> Edge {
            self.edge(chain_id)
        }
        fn chain_position(&self, edge_id: usize) -> ChainPosition {
            ChainPosition::new(edge_id, 0)
        }
        fn dimension(&self) -> Dimension {
            Dimension::Point
        }
    }

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty_index_region() {
        let mut index = ShapeIndex::new();
        index.build();
        let region = ShapeIndexRegion::new(&index);
        assert!(region.cap_bound().is_empty());
        assert!(region.rect_bound().is_empty());
        assert!(!region.contains_point(&p(0.0, 0.0)));
    }

    #[test]
    fn test_single_point_region() {
        let mut index = ShapeIndex::new();
        let target = p(10.0, 20.0);
        index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        index.build();

        let region = ShapeIndexRegion::new(&index);
        assert!(!region.cap_bound().is_empty());
        assert!(!region.rect_bound().is_empty());
        let bounds = region.cell_union_bound();
        assert!(!bounds.is_empty());
    }

    #[test]
    fn test_multiple_points_region() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0), p(45.0, 90.0), p(-30.0, -60.0)],
        }));
        index.build();

        let region = ShapeIndexRegion::new(&index);
        let bounds = region.cell_union_bound();
        assert!(bounds.len() <= 6);

        // The bounding cap should contain all points.
        let cap = region.cap_bound();
        assert!(cap.contains_point(p(0.0, 0.0)));
        assert!(cap.contains_point(p(45.0, 90.0)));
        assert!(cap.contains_point(p(-30.0, -60.0)));
    }

    #[test]
    fn test_intersects_cell_region() {
        let mut index = ShapeIndex::new();
        let target = p(0.0, 0.0);
        index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        index.build();

        let region = ShapeIndexRegion::new(&index);
        // The face cell should intersect.
        let face_cell = Cell::from_cell_id(CellId::from_face(0));
        assert!(region.intersects_cell(&face_cell));

        // A cell on the opposite side of the sphere should not.
        let far_cell = Cell::from_cell_id(CellId::from_point(&p(0.0, 180.0)));
        assert!(!region.intersects_cell(&far_cell));
    }

    /// Makes a `ShapeIndex` containing a single `LaxLoop` polygon.
    fn make_polygon_index(vertices: &[Point]) -> ShapeIndex {
        use crate::s2::lax_loop::LaxLoop;
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxLoop::new(vertices.to_vec())));
        index.build();
        index
    }

    #[test]
    fn test_polygon_contains_cell() {
        // A large polygon (entire face 0 roughly) should contain a small
        // cell deep within it.
        let index = make_polygon_index(&[p(1.0, 1.0), p(1.0, -1.0), p(-1.0, -1.0), p(-1.0, 1.0)]);
        let region = ShapeIndexRegion::new(&index);

        // A very small cell near the center of the polygon.
        let center_id = CellId::from_point(&p(0.0, 0.0));
        let small_cell = Cell::from_cell_id(center_id.parent_at_level(20));
        assert!(
            region.contains_cell(&small_cell),
            "A small cell inside a large polygon should be contained"
        );
    }

    #[test]
    fn test_polygon_does_not_contain_outside_cell() {
        // A small polygon near (0,0).
        let index = make_polygon_index(&[
            p(0.01, 0.01),
            p(0.01, -0.01),
            p(-0.01, -0.01),
            p(-0.01, 0.01),
        ]);
        let region = ShapeIndexRegion::new(&index);

        // A cell far from the polygon.
        let far_id = CellId::from_point(&p(45.0, 90.0));
        let far_cell = Cell::from_cell_id(far_id.parent_at_level(10));
        assert!(!region.contains_cell(&far_cell));
    }

    #[test]
    fn test_polygon_intersects_cell_edge_crossing() {
        // A polygon that crosses through a cell: the cell's edges cross
        // the polygon boundary so intersects_cell should return true.
        let index = make_polygon_index(&[p(1.0, 1.0), p(1.0, -1.0), p(-1.0, -1.0), p(-1.0, 1.0)]);
        let region = ShapeIndexRegion::new(&index);

        // A cell that overlaps the boundary of the polygon.
        // Use a cell near one of the polygon edges.
        let edge_id = CellId::from_point(&p(1.0, 0.0));
        let edge_cell = Cell::from_cell_id(edge_id.parent_at_level(15));
        assert!(
            region.intersects_cell(&edge_cell),
            "A cell overlapping a polygon boundary should intersect"
        );
    }

    #[test]
    fn test_polygon_intersects_cell_interior() {
        // A cell fully inside the polygon should still register as
        // intersecting.
        let index = make_polygon_index(&[
            p(10.0, 10.0),
            p(10.0, -10.0),
            p(-10.0, -10.0),
            p(-10.0, 10.0),
        ]);
        let region = ShapeIndexRegion::new(&index);

        let interior_id = CellId::from_point(&p(0.0, 0.0));
        let interior_cell = Cell::from_cell_id(interior_id.parent_at_level(20));
        assert!(region.intersects_cell(&interior_cell));
    }

    #[test]
    fn test_polygon_does_not_intersect_far_cell() {
        let index = make_polygon_index(&[
            p(0.01, 0.01),
            p(0.01, -0.01),
            p(-0.01, -0.01),
            p(-0.01, 0.01),
        ]);
        let region = ShapeIndexRegion::new(&index);

        let far_id = CellId::from_point(&p(45.0, 90.0));
        let far_cell = Cell::from_cell_id(far_id.parent_at_level(10));
        assert!(!region.intersects_cell(&far_cell));
    }

    #[test]
    fn test_dyn_region() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0)],
        }));
        index.build();

        let region = ShapeIndexRegion::new(&index);
        let r: &dyn Region = &region;
        assert!(!r.cap_bound().is_empty());
    }
}
