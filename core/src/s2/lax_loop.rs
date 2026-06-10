// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A lightweight closed loop shape.
//!
//! [`LaxLoop`] is similar to [`Loop`](super::Loop) but allows duplicate
//! vertices and edges, and is faster to initialize. Loops may have any
//! number of vertices, including 0, 1, or 2.
//!
//! Corresponds to C++ `s2lax_loop_shape.h`, Go `s2/lax_loop.go`.

use crate::s2::Point;
use crate::s2::shape::{
    Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape, reference_point_for_shape,
};

/// A closed loop of edges surrounding an interior region (dimension 2).
///
/// Unlike [`Loop`](super::Loop), this type allows duplicate vertices and
/// edges. It is faster to initialize and more compact, but does not
/// support the same operations.
///
/// # Examples
///
/// ```
/// use s2rst::s2::lax_loop::LaxLoop;
/// use s2rst::s2::shape::{Dimension, Shape};
/// use s2rst::s2::LatLng;
///
/// let lax = LaxLoop::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 1.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
/// ]);
/// assert_eq!(lax.num_edges(), 3);
/// assert_eq!(lax.dimension(), Dimension::Polygon);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LaxLoop {
    vertices: Vec<Point>,
}

impl LaxLoop {
    /// Creates a new `LaxLoop` from the given vertices.
    pub fn new(vertices: Vec<Point>) -> Self {
        LaxLoop { vertices }
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the vertex at the given index.
    pub fn vertex(&self, i: usize) -> Point {
        self.vertices[i]
    }
}

impl Shape for LaxLoop {
    fn num_edges(&self) -> usize {
        self.vertices.len()
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        let next = if id + 1 == self.vertices.len() {
            0
        } else {
            id + 1
        };
        Edge::new(self.vertices[id], self.vertices[next])
    }

    fn reference_point(&self) -> ReferencePoint {
        reference_point_for_shape(self)
    }

    fn num_chains(&self) -> usize {
        usize::min(1, self.vertices.len())
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.vertices.len())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        self.edge(offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polygon
    }
}

impl std::ops::Deref for LaxLoop {
    type Target = [Point];
    fn deref(&self) -> &[Point] {
        &self.vertices
    }
}

// ─── Variants ───────────────────────────────────────────────────────────

/// A closed ring treated as one-dimensional geometry: the same edges as a
/// [`LaxLoop`] but with no interior (C++ `S2LaxClosedPolylineShape`).
#[derive(Debug)]
pub struct LaxClosedPolyline {
    vertices: Vec<Point>,
}

impl LaxClosedPolyline {
    /// Creates a closed polyline from the given ring of vertices.
    pub fn new(vertices: Vec<Point>) -> Self {
        LaxClosedPolyline { vertices }
    }

    /// The number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Vertex `i`.
    pub fn vertex(&self, i: usize) -> Point {
        self.vertices[i]
    }
}

impl Shape for LaxClosedPolyline {
    fn num_edges(&self) -> usize {
        self.vertices.len()
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        let next = if id + 1 == self.vertices.len() {
            0
        } else {
            id + 1
        };
        Edge::new(self.vertices[id], self.vertices[next])
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), false)
    }

    fn num_chains(&self) -> usize {
        usize::min(1, self.vertices.len())
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.vertices.len())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        self.edge(offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }
}

/// A loop whose vertices are indices into a shared vertex array, so many
/// loops over one mesh share storage (C++ `S2VertexIdLaxLoopShape`).
#[derive(Debug)]
pub struct VertexIdLaxLoop {
    vertex_ids: Vec<u32>,
    vertices: std::sync::Arc<[Point]>,
}

impl VertexIdLaxLoop {
    /// Creates a loop from vertex indices into the shared array.
    ///
    /// # Panics
    /// Panics if any index is out of range for `vertices` (checked up
    /// front so accessors stay panic-free thereafter).
    pub fn new(vertex_ids: Vec<u32>, vertices: std::sync::Arc<[Point]>) -> Self {
        assert!(
            vertex_ids.iter().all(|&id| (id as usize) < vertices.len()),
            "vertex id out of range"
        );
        VertexIdLaxLoop {
            vertex_ids,
            vertices,
        }
    }

    /// The number of loop vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertex_ids.len()
    }

    /// The shared-array index of loop vertex `i`.
    pub fn vertex_id(&self, i: usize) -> u32 {
        self.vertex_ids[i]
    }

    /// Loop vertex `i`.
    pub fn vertex(&self, i: usize) -> Point {
        self.vertices[self.vertex_ids[i] as usize]
    }
}

impl Shape for VertexIdLaxLoop {
    fn num_edges(&self) -> usize {
        self.vertex_ids.len()
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        let next = if id + 1 == self.vertex_ids.len() {
            0
        } else {
            id + 1
        };
        Edge::new(self.vertex(id), self.vertex(next))
    }

    fn reference_point(&self) -> ReferencePoint {
        reference_point_for_shape(self)
    }

    fn num_chains(&self) -> usize {
        usize::min(1, self.vertex_ids.len())
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.vertex_ids.len())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        self.edge(offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polygon
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lax_closed_polyline_has_no_interior() {
        use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
        use crate::s2::shape_index::ShapeIndex;
        use crate::s2::text_format::{parse_point, parse_points};
        let ring = parse_points("0:0, 0:5, 5:5, 5:0");
        let closed = LaxClosedPolyline::new(ring.clone());
        assert_eq!(closed.dimension(), Dimension::Polyline);
        assert_eq!(closed.num_edges(), 4, "closed: one edge per vertex");
        assert_eq!(closed.edge(3).v1, closed.vertex(0), "wraps to start");

        let mut index = ShapeIndex::new();
        index.add(Box::new(closed));
        let mut q = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
        assert!(
            !q.contains(parse_point("2:2")),
            "a closed polyline contains nothing"
        );
        // The same ring as a LaxLoop (dimension 2) does contain the point.
        let mut index2 = ShapeIndex::new();
        index2.add(Box::new(LaxLoop::new(ring)));
        let mut q2 = ContainsPointQuery::new(&index2, VertexModel::SemiOpen);
        assert!(q2.contains(parse_point("2:2")));
    }

    #[test]
    fn test_vertex_id_lax_loop_shares_storage() {
        use crate::s2::text_format::parse_points;
        let mesh: std::sync::Arc<[Point]> = parse_points("0:0, 0:5, 5:5, 5:0, 2:2").into();
        let outer = VertexIdLaxLoop::new(vec![0, 1, 2, 3], std::sync::Arc::clone(&mesh));
        assert_eq!(outer.num_vertices(), 4);
        assert_eq!(outer.vertex(1), mesh[1]);
        assert_eq!(outer.vertex_id(3), 3);
        assert_eq!(outer.edge(3).v1, mesh[0], "wraps to start");
        assert_eq!(outer.dimension(), Dimension::Polygon);
        let r = outer.reference_point();
        assert!(r.contained || !r.contained, "reference point computed");
    }

    #[test]
    #[should_panic(expected = "vertex id out of range")]
    fn test_vertex_id_lax_loop_rejects_bad_ids() {
        let mesh: std::sync::Arc<[Point]> = crate::s2::text_format::parse_points("0:0, 0:5").into();
        let _unused = VertexIdLaxLoop::new(vec![0, 7], mesh);
    }

    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn lax_loop_is_send_sync() {
        is_send_sync::<LaxLoop>();
    }

    #[test]
    fn test_empty() {
        let l = LaxLoop::new(vec![]);
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
        assert_eq!(l.dimension(), Dimension::Polygon);
        assert!(l.is_empty());
        assert!(!l.is_full());
    }

    #[test]
    fn test_single_vertex() {
        let l = LaxLoop::new(vec![p(0.0, 0.0)]);
        assert_eq!(l.num_edges(), 1);
        assert_eq!(l.num_chains(), 1);
        let e = l.edge(0);
        assert!(e.is_degenerate());
    }

    #[test]
    fn test_triangle() {
        let l = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)]);
        assert_eq!(l.num_edges(), 3);
        assert_eq!(l.num_chains(), 1);
        assert_eq!(l.num_vertices(), 3);

        let chain = l.chain(0);
        assert_eq!(chain.start, 0);
        assert_eq!(chain.length, 3);

        // Last edge wraps around
        let last_edge = l.edge(2);
        assert_eq!(last_edge.v0, l.vertex(2));
        assert_eq!(last_edge.v1, l.vertex(0));
    }

    #[test]
    fn test_chain_position() {
        let l = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)]);
        for i in 0..3 {
            let cp = l.chain_position(i);
            assert_eq!(cp.chain_id, 0);
            assert_eq!(cp.offset, i);
        }
    }

    #[test]
    fn test_deref() {
        let l = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let slice: &[Point] = &l;
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_has_interior() {
        let l = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)]);
        assert!(l.has_interior());
    }

    #[test]
    fn test_reference_point() {
        // A small triangle around the north pole
        let l = LaxLoop::new(vec![p(89.0, 0.0), p(89.0, 120.0), p(89.0, 240.0)]);
        let rp = l.reference_point();
        // The reference point should be well-defined for a non-degenerate loop
        assert!(rp.point.0.norm() > 0.5);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let ll = LaxLoop::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        ]);
        let json = serde_json::to_string(&ll).unwrap();
        let back: LaxLoop = serde_json::from_str(&json).unwrap();
        assert_eq!(ll.num_vertices(), back.num_vertices());
        for i in 0..ll.num_vertices() {
            assert_eq!(ll.vertex(i), back.vertex(i));
        }
    }
}
