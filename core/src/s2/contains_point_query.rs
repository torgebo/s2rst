// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Point containment queries using a [`ShapeIndex`].
//!
//! [`ContainsPointQuery`] determines whether one or more shapes in a
//! [`ShapeIndex`] contain a given point. Shape boundaries may be modeled
//! as Open, `SemiOpen`, or Closed.
//!
//! Corresponds to C++ `s2contains_point_query.h`, Go `s2/contains_point_query.go`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
use std::ops::ControlFlow;

use crate::s2::Point;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::{Crossing, vertex_crossing};
use crate::s2::shape::{Dimension, Shape, ShapeEdge, ShapeEdgeId, ShapeId};
use crate::s2::shape_index::{ClippedShape, ShapeIndex, ShapeIndexIterator};

/// Controls whether shapes contain their vertices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum VertexModel {
    /// No shapes contain their vertices. `contains` returns true only if the
    /// point is in the interior of some polygon.
    Open,
    /// Polygon containment is defined such that if several polygons tile the
    /// region around a vertex, exactly one contains that vertex. Points and
    /// polylines still do not contain any vertices.
    #[default]
    SemiOpen,
    /// All shapes contain their vertices (including points and polylines).
    Closed,
}

/// Determines whether shapes in a [`ShapeIndex`] contain a given point.
///
/// This type is not thread-safe (it caches an iterator internally). For
/// concurrent queries, create one per thread or use separate instances.
///
/// # Examples
///
/// ```
/// use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::lax_loop::LaxLoop;
/// use s2rst::s2::LatLng;
///
/// let mut index = ShapeIndex::new();
/// index.add(Box::new(LaxLoop::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 10.0).to_point(),
///     LatLng::from_degrees(10.0, 10.0).to_point(),
///     LatLng::from_degrees(10.0, 0.0).to_point(),
/// ])));
/// index.build();
///
/// let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
/// assert!(query.contains(LatLng::from_degrees(5.0, 5.0).to_point()));
/// assert!(!query.contains(LatLng::from_degrees(20.0, 20.0).to_point()));
/// ```
#[derive(Debug)]
pub struct ContainsPointQuery<'a> {
    model: VertexModel,
    index: &'a ShapeIndex,
    iter: ShapeIndexIterator<'a>,
}

impl<'a> ContainsPointQuery<'a> {
    /// Creates a new query for the given index and vertex model.
    #[inline]
    pub fn new(index: &'a ShapeIndex, model: VertexModel) -> Self {
        ContainsPointQuery {
            model,
            index,
            iter: index.iter(),
        }
    }

    /// Reports whether any shape in the index contains the point `p`.
    #[inline]
    pub fn contains(&mut self, p: Point) -> bool {
        if !self.iter.locate_point(p) {
            return false;
        }
        let Some(cell) = self.iter.index_cell() else {
            return false;
        };
        let center = self.iter.center();
        for clipped in &cell.shapes {
            if self.shape_contains_impl(clipped, center, p) {
                return true;
            }
        }
        false
    }

    /// Reports whether the shape with the given `shape_id` contains `p`.
    #[inline]
    pub fn shape_contains(&mut self, shape_id: impl Into<ShapeId>, p: Point) -> bool {
        let shape_id = shape_id.into();
        if !self.iter.locate_point(p) {
            return false;
        }
        let Some(cell) = self.iter.index_cell() else {
            return false;
        };
        let center = self.iter.center();
        match cell.find_by_shape_id(shape_id) {
            Some(clipped) => self.shape_contains_impl(clipped, center, p),
            None => false,
        }
    }

    /// Returns references to all shapes in the index that contain `p`.
    ///
    /// This is a convenience wrapper that collects shape IDs via
    /// [`containing_shape_ids`](Self::containing_shape_ids) and then looks
    /// up each shape in the index. Matches C++ `GetContainingShapes`.
    pub fn containing_shapes(&mut self, p: Point) -> Vec<&'a dyn Shape> {
        let ids = self.containing_shape_ids(p);
        ids.into_iter()
            .filter_map(|id| self.index.shape(id))
            .collect()
    }

    /// Returns a list of shape IDs for all shapes containing `p`.
    pub fn containing_shape_ids(&mut self, p: Point) -> Vec<ShapeId> {
        let mut ids = Vec::new();
        let _ = self.visit_containing_shapes(p, |shape_id, _shape| {
            ids.push(shape_id);
            ControlFlow::Continue(())
        });
        ids
    }

    /// Visits all shapes containing `p`. The callback receives the shape ID
    /// and shape reference. Return `ControlFlow::Break(())` to stop early.
    pub fn visit_containing_shapes<F>(&mut self, p: Point, mut f: F) -> ControlFlow<()>
    where
        F: FnMut(ShapeId, &dyn Shape) -> ControlFlow<()>,
    {
        if !self.iter.locate_point(p) {
            return ControlFlow::Continue(());
        }
        let Some(cell) = self.iter.index_cell() else {
            return ControlFlow::Continue(());
        };
        let center = self.iter.center();
        for clipped in &cell.shapes {
            if self.shape_contains_impl(clipped, center, p)
                && let Some(shape) = self.index.shape(clipped.shape_id)
            {
                f(clipped.shape_id, shape)?;
            }
        }
        ControlFlow::Continue(())
    }

    /// Returns a reference to the underlying index.
    pub fn index(&self) -> &'a ShapeIndex {
        self.index
    }

    /// Visits all edges in the index that are incident to the point `p`
    /// (i.e., `p` is one of the edge endpoints), terminating early if the
    /// visitor returns `ControlFlow::Break(())`. Each edge is visited at
    /// most once.
    pub fn visit_incident_edges<F>(&mut self, p: Point, mut visitor: F) -> ControlFlow<()>
    where
        F: FnMut(&ShapeEdge) -> ControlFlow<()>,
    {
        if !self.iter.locate_point(p) {
            return ControlFlow::Continue(());
        }
        let Some(cell) = self.iter.index_cell() else {
            return ControlFlow::Continue(());
        };
        for clipped in &cell.shapes {
            let num_edges = clipped.num_edges();
            if num_edges == 0 {
                continue;
            }
            let Some(shape) = self.index.shape(clipped.shape_id) else {
                continue;
            };
            for &edge_id in &clipped.edges {
                let edge = shape.edge(edge_id as usize);
                if edge.v0 == p || edge.v1 == p {
                    visitor(&ShapeEdge::new(
                        ShapeEdgeId::new(clipped.shape_id, edge_id),
                        edge,
                    ))?;
                }
            }
        }
        ControlFlow::Continue(())
    }

    /// Internal containment test for a clipped shape.
    ///
    /// Tests whether `clipped` contains `p`, using `center` as the cell center
    /// from which edge crossings are counted.
    pub(crate) fn shape_contains_impl(
        &self,
        clipped: &ClippedShape,
        center: Point,
        p: Point,
    ) -> bool {
        let mut inside = clipped.contains_center;
        let num_edges = clipped.num_edges();
        if num_edges == 0 {
            return inside;
        }

        let Some(shape) = self.index.shape(clipped.shape_id) else {
            return false;
        };

        if shape.dimension() != Dimension::Polygon {
            // Points and polylines: only contained with Closed model.
            if self.model != VertexModel::Closed {
                return false;
            }
            for &edge_id in &clipped.edges {
                let edge = shape.edge(edge_id as usize);
                if edge.v0 == p || edge.v1 == p {
                    return true;
                }
            }
            return false;
        }

        // Polygon (dimension 2): count edge crossings from cell center to p.
        let mut crosser = EdgeCrosser::new(center, p);
        for &edge_id in &clipped.edges {
            let edge = shape.edge(edge_id as usize);
            let sign = crosser.crossing_sign(edge.v0, edge.v1);
            match sign {
                Crossing::DoNotCross => {}
                Crossing::Cross => {
                    inside = !inside;
                }
                Crossing::MaybeCross => {
                    // For Open and Closed models, check if p is a vertex.
                    if self.model != VertexModel::SemiOpen && (edge.v0 == p || edge.v1 == p) {
                        return self.model == VertexModel::Closed;
                    }
                    if vertex_crossing(crosser.a, crosser.b, edge.v0, edge.v1) {
                        inside = !inside;
                    }
                }
            }
        }

        inside
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::shape::{Chain, ChainPosition, Edge, ReferencePoint};

    /// A simple point-set shape for testing.
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
    fn test_contains_point_empty_index() {
        let mut index = ShapeIndex::new();
        index.build();
        let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        assert!(!query.contains(p(0.0, 0.0)));
    }

    #[test]
    fn test_contains_point_closed_model() {
        let mut index = ShapeIndex::new();
        let target = p(10.0, 20.0);
        index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        index.build();

        // Open model: points don't contain vertices.
        let mut q_open = ContainsPointQuery::new(&index, VertexModel::Open);
        assert!(!q_open.contains(target));

        // Closed model: points contain vertices.
        let mut q_closed = ContainsPointQuery::new(&index, VertexModel::Closed);
        assert!(q_closed.contains(target));
    }

    #[test]
    fn test_containing_shape_ids() {
        let mut index = ShapeIndex::new();
        let target = p(10.0, 20.0);
        let id0 = index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        let _id1 = index.add(Box::new(PointVectorShape {
            points: vec![p(30.0, 40.0)],
        }));
        index.build();

        let mut q = ContainsPointQuery::new(&index, VertexModel::Closed);
        let ids = q.containing_shape_ids(target);
        assert_eq!(ids, vec![id0]);
    }

    #[test]
    fn test_shape_contains() {
        let mut index = ShapeIndex::new();
        let target = p(10.0, 20.0);
        let id = index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        index.build();

        let mut q = ContainsPointQuery::new(&index, VertexModel::Closed);
        assert!(q.shape_contains(id, target));
    }

    #[test]
    fn test_no_match() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(10.0, 20.0)],
        }));
        index.build();

        let mut q = ContainsPointQuery::new(&index, VertexModel::Closed);
        // A distant point should not be contained.
        assert!(!q.contains(p(-80.0, -170.0)));
    }

    #[test]
    fn test_contains_point_vertex_model() {
        use crate::s2::lax_loop::LaxLoop;

        // Build a polygon (LaxLoop, dimension 2) as a triangle and add it
        // to a ShapeIndex. Then test containment for a point exactly on a
        // vertex, a point in the interior, and a point outside.
        let v0 = p(0.0, 0.0);
        let v1 = p(0.0, 10.0);
        let v2 = p(10.0, 5.0);
        let shape = LaxLoop::new(vec![v0, v1, v2]);

        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        // An interior point of the triangle.
        let interior = p(3.0, 5.0);

        // SemiOpen model: interior point should be contained, vertex may or may not be.
        let mut q_semi = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        assert!(
            q_semi.contains(interior),
            "SemiOpen: interior point should be contained"
        );

        // Closed model: vertex v0 should be contained, as should the interior.
        let mut q_closed = ContainsPointQuery::new(&index, VertexModel::Closed);
        assert!(
            q_closed.contains(interior),
            "Closed: interior point should be contained"
        );
        // With the Closed model, the vertex itself is contained if it's
        // recognized as on the boundary. Since v0 is a vertex of a
        // dimension-2 shape, the MaybeCross path returns Closed==true.
        assert!(
            q_closed.contains(v0),
            "Closed: vertex v0 should be contained"
        );

        // Open model: the vertex itself should NOT be contained.
        let mut q_open = ContainsPointQuery::new(&index, VertexModel::Open);
        assert!(
            !q_open.contains(v0),
            "Open: vertex v0 should not be contained"
        );
        // But the interior point should still be contained.
        assert!(
            q_open.contains(interior),
            "Open: interior point should be contained"
        );

        // A point well outside should not be contained in any model.
        let outside = p(-50.0, -50.0);
        let mut q_any = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        assert!(
            !q_any.contains(outside),
            "point well outside should not be contained"
        );
    }

    #[test]
    fn test_containing_shapes() {
        // Create an index with a LaxPolygon containing the test point
        // and another that does not.

        use crate::s2::text_format;
        let lp1 = text_format::make_lax_polygon("0:0, 0:10, 10:10, 10:0");
        let lp2 = text_format::make_lax_polygon("20:20, 20:30, 30:30, 30:20");
        let mut index = ShapeIndex::new();
        index.add(Box::new(lp1));
        index.add(Box::new(lp2));
        index.build();

        let interior_point = p(5.0, 5.0);
        let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        let shapes = query.containing_shapes(interior_point);
        assert_eq!(
            shapes.len(),
            1,
            "exactly one shape should contain the point"
        );

        // A point outside both polygons.
        let outside = p(-5.0, -5.0);
        let shapes2 = query.containing_shapes(outside);
        assert!(shapes2.is_empty(), "no shapes should contain outside point");
    }

    #[test]
    fn test_containing_shapes_matches_ids() {
        // Verify that containing_shapes returns the same shapes as
        // containing_shape_ids.
        use crate::s2::text_format;
        let lp = text_format::make_lax_polygon("0:0, 0:10, 10:10, 10:0");
        let mut index = ShapeIndex::new();
        index.add(Box::new(lp));
        index.build();

        let pt = p(5.0, 5.0);
        let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        let ids = query.containing_shape_ids(pt);
        let shapes = query.containing_shapes(pt);
        assert_eq!(ids.len(), shapes.len());
        assert!(!ids.is_empty(), "point should be contained");
        for (id, shape) in ids.iter().zip(shapes.iter()) {
            assert_eq!(index.shape(*id).unwrap().num_edges(), shape.num_edges());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_vertex_model_roundtrip() {
        for vm in [
            VertexModel::Open,
            VertexModel::SemiOpen,
            VertexModel::Closed,
        ] {
            let json = serde_json::to_string(&vm).unwrap();
            let back: VertexModel = serde_json::from_str(&json).unwrap();
            assert_eq!(vm, back);
        }
    }
}
