// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A lightweight open polyline shape.
//!
//! [`LaxPolyline`] is similar to [`Polyline`](super::polyline::Polyline)
//! except that adjacent vertices are allowed to be identical or antipodal,
//! and the representation is slightly more compact.
//!
//! Corresponds to C++ `s2lax_polyline_shape.h`, Go `s2/lax_polyline.go`.

use crate::s2::Point;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

/// An open polyline represented as a [`Shape`] (dimension 1).
///
/// Unlike [`Polyline`](super::polyline::Polyline), this type allows
/// identical or antipodal adjacent vertices. Polylines with fewer than
/// 2 vertices have no edges.
///
/// # Examples
///
/// ```
/// use s2rst::s2::lax_polyline::LaxPolyline;
/// use s2rst::s2::shape::{Dimension, Shape};
/// use s2rst::s2::LatLng;
///
/// let line = LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
///     LatLng::from_degrees(2.0, 0.0).to_point(),
/// ]);
/// assert_eq!(line.num_edges(), 2);
/// assert_eq!(line.dimension(), Dimension::Polyline);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LaxPolyline {
    vertices: Vec<Point>,
}

impl LaxPolyline {
    /// Creates a new `LaxPolyline` from the given vertices.
    pub fn new(vertices: Vec<Point>) -> Self {
        LaxPolyline { vertices }
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the vertex at the given index.
    pub fn vertex(&self, i: usize) -> Point {
        self.vertices[i]
    }

    /// Returns a slice of all vertices.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }
}

impl Shape for LaxPolyline {
    fn num_edges(&self) -> usize {
        if self.vertices.len() < 2 {
            0
        } else {
            self.vertices.len() - 1
        }
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        Edge::new(self.vertices[id], self.vertices[id + 1])
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), false)
    }

    fn num_chains(&self) -> usize {
        if self.num_edges() > 0 { 1 } else { 0 }
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.num_edges())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        Edge::new(self.vertices[offset], self.vertices[offset + 1])
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }

    fn type_tag(&self) -> u32 {
        4 // S2LaxPolylineShape::kTypeTag
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        self.encode_with_hint(w, hint)
    }
}

impl std::ops::Deref for LaxPolyline {
    type Target = [Point];
    fn deref(&self) -> &[Point] {
        &self.vertices
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn lax_polyline_is_send_sync() {
        is_send_sync::<LaxPolyline>();
    }

    #[test]
    fn test_empty() {
        let l = LaxPolyline::new(vec![]);
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
        assert_eq!(l.dimension(), Dimension::Polyline);
        assert!(l.is_empty());
        assert!(!l.is_full());
        assert!(!l.has_interior());
    }

    #[test]
    fn test_single_vertex() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0)]);
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
    }

    #[test]
    fn test_two_vertices() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        assert_eq!(l.num_edges(), 1);
        assert_eq!(l.num_chains(), 1);
        let e = l.edge(0);
        assert_eq!(e.v0, l.vertex(0));
        assert_eq!(e.v1, l.vertex(1));
    }

    #[test]
    fn test_three_vertices() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        assert_eq!(l.num_edges(), 2);
        assert_eq!(l.num_chains(), 1);
        let chain = l.chain(0);
        assert_eq!(chain.start, 0);
        assert_eq!(chain.length, 2);
    }

    #[test]
    fn test_chain_position() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        for i in 0..2 {
            let cp = l.chain_position(i);
            assert_eq!(cp.chain_id, 0);
            assert_eq!(cp.offset, i);
        }
    }

    #[test]
    fn test_reference_point_not_contained() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let rp = l.reference_point();
        assert!(!rp.contained);
    }

    #[test]
    fn test_deref() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let slice: &[Point] = &l;
        assert_eq!(slice.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let lp = LaxPolyline::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        ]);
        let json = serde_json::to_string(&lp).unwrap();
        let back: LaxPolyline = serde_json::from_str(&json).unwrap();
        assert_eq!(lp.num_vertices(), back.num_vertices());
        for i in 0..lp.num_vertices() {
            assert_eq!(lp.vertex(i), back.vertex(i));
        }
    }
}
