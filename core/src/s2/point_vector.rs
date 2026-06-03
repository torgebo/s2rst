// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A [`Shape`] representing a set of points.
//!
//! Each point is represented as a degenerate edge (v0 == v1). This type is
//! useful for adding a collection of points to a [`ShapeIndex`](super::shape_index::ShapeIndex).
//!
//! Corresponds to C++ `s2point_vector_shape.h`, Go `s2/point_vector.go`.

use crate::s2::Point;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

/// A set of points represented as a [`Shape`] (dimension 0).
///
/// Each point is stored as a degenerate edge where v0 == v1.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PointVector {
    points: Vec<Point>,
}

impl PointVector {
    /// Creates a new `PointVector` from a list of points.
    pub fn new(points: Vec<Point>) -> Self {
        PointVector { points }
    }

    /// Returns the number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Reports whether this vector is empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Returns the point at the given index.
    pub fn point(&self, i: usize) -> Point {
        self.points[i]
    }

    /// Returns a slice of all points.
    pub fn points(&self) -> &[Point] {
        &self.points
    }
}

impl<'a> IntoIterator for &'a PointVector {
    type Item = &'a Point;
    type IntoIter = std::slice::Iter<'a, Point>;

    fn into_iter(self) -> Self::IntoIter {
        self.points.iter()
    }
}

impl IntoIterator for PointVector {
    type Item = Point;
    type IntoIter = std::vec::IntoIter<Point>;

    fn into_iter(self) -> Self::IntoIter {
        self.points.into_iter()
    }
}

impl Shape for PointVector {
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

    fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge {
        debug_assert_eq!(offset, 0);
        Edge::new(self.points[chain_id], self.points[chain_id])
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition::new(edge_id, 0)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Point
    }

    fn type_tag(&self) -> u32 {
        3 // S2PointVectorShape::kTypeTag
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        self.encode_with_hint(w, hint)
    }
}

impl std::ops::Deref for PointVector {
    type Target = [Point];
    fn deref(&self) -> &[Point] {
        &self.points
    }
}

impl From<Vec<Point>> for PointVector {
    fn from(points: Vec<Point>) -> Self {
        PointVector::new(points)
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
    fn point_vector_is_send_sync() {
        is_send_sync::<PointVector>();
    }

    #[test]
    fn test_empty() {
        let pv = PointVector::new(vec![]);
        assert_eq!(pv.num_edges(), 0);
        assert_eq!(pv.num_chains(), 0);
        assert_eq!(pv.dimension(), Dimension::Point);
        assert!(pv.is_empty());
        assert!(!pv.is_full());
    }

    #[test]
    fn test_single_point() {
        let pv = PointVector::new(vec![p(0.0, 0.0)]);
        assert_eq!(pv.num_edges(), 1);
        assert_eq!(pv.num_chains(), 1);
        let e = pv.edge(0);
        assert_eq!(e.v0, e.v1);
        assert!(e.is_degenerate());
    }

    #[test]
    fn test_multiple_points() {
        let pv = PointVector::new(vec![p(0.0, 0.0), p(45.0, 90.0), p(-30.0, -60.0)]);
        assert_eq!(pv.num_edges(), 3);
        assert_eq!(pv.num_chains(), 3);
        assert_eq!(pv.len(), 3);

        for i in 0..3 {
            let chain = pv.chain(i);
            assert_eq!(chain.start, i);
            assert_eq!(chain.length, 1);

            let cp = pv.chain_position(i);
            assert_eq!(cp.chain_id, i);
            assert_eq!(cp.offset, 0);
        }
    }

    #[test]
    fn test_deref() {
        let pv = PointVector::new(vec![p(1.0, 2.0), p(3.0, 4.0)]);
        let slice: &[Point] = &pv;
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_from_vec() {
        let pv: PointVector = vec![p(0.0, 0.0)].into();
        assert_eq!(pv.len(), 1);
    }

    #[test]
    fn test_not_full() {
        let pv = PointVector::new(vec![p(0.0, 0.0)]);
        assert!(!pv.is_full());
        assert!(!pv.has_interior());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let pv = PointVector::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        ]);
        let json = serde_json::to_string(&pv).unwrap();
        let back: PointVector = serde_json::from_str(&json).unwrap();
        assert_eq!(pv.len(), back.len());
        for i in 0..pv.len() {
            assert_eq!(pv.point(i), back.point(i));
        }
    }
}
