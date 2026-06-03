// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! An `S2Shape` representing an arbitrary set of edges.
//!
//! [`EdgeVectorShape`] stores edges as pairs of points, with each edge forming
//! its own chain. Mainly useful for testing, but works for any collection of
//! edges where memory efficiency isn't critical.
//!
//! Corresponds to C++ `s2edge_vector_shape.h`.

use crate::s2::Point;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

/// An `S2Shape` representing an arbitrary set of edges.
///
/// Each edge forms its own chain. The default dimension is 1 (polyline-like).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeVectorShape {
    edges: Vec<(Point, Point)>,
    dim: Dimension,
}

impl Default for EdgeVectorShape {
    fn default() -> Self {
        EdgeVectorShape {
            edges: Vec::new(),
            dim: Dimension::Polyline,
        }
    }
}

impl EdgeVectorShape {
    /// Creates an empty edge vector shape.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a shape from a vector of edge pairs.
    pub fn from_edges(edges: Vec<(Point, Point)>) -> Self {
        EdgeVectorShape {
            edges,
            dim: Dimension::Polyline,
        }
    }

    /// Creates a shape containing a single edge.
    pub fn from_edge(a: Point, b: Point) -> Self {
        EdgeVectorShape {
            edges: vec![(a, b)],
            dim: Dimension::Polyline,
        }
    }

    /// Sets the dimension of this shape.
    pub fn set_dimension(&mut self, dim: Dimension) {
        self.dim = dim;
    }

    /// Adds an edge to this shape.
    ///
    /// This should only be called before adding the shape to a `ShapeIndex`.
    pub fn add(&mut self, a: Point, b: Point) {
        self.edges.push((a, b));
    }
}

impl Shape for EdgeVectorShape {
    fn num_edges(&self) -> usize {
        self.edges.len()
    }

    fn edge(&self, id: usize) -> Edge {
        Edge {
            v0: self.edges[id].0,
            v1: self.edges[id].1,
        }
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::default()
    }

    fn num_chains(&self) -> usize {
        self.edges.len()
    }

    fn chain(&self, chain_id: usize) -> Chain {
        Chain {
            start: chain_id,
            length: 1,
        }
    }

    fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge {
        debug_assert_eq!(offset, 0);
        Edge {
            v0: self.edges[chain_id].0,
            v1: self.edges[chain_id].1,
        }
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition {
            chain_id: edge_id,
            offset: 0,
        }
    }

    fn dimension(&self) -> Dimension {
        self.dim
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty() {
        let shape = EdgeVectorShape::new();
        assert_eq!(shape.num_edges(), 0);
        assert_eq!(shape.num_chains(), 0);
        assert_eq!(shape.dimension(), Dimension::Polyline);
        assert!(shape.is_empty()); // dimension 1 with 0 edges → empty
        assert!(!shape.is_full());
    }

    #[test]
    fn test_edge_access() {
        let a = p(0.0, 0.0);
        let b = p(1.0, 0.0);
        let c = p(2.0, 0.0);

        let shape = EdgeVectorShape::from_edges(vec![(a, b), (b, c)]);
        assert_eq!(shape.num_edges(), 2);
        assert_eq!(shape.num_chains(), 2);
        assert_eq!(shape.edge(0).v0, a);
        assert_eq!(shape.edge(0).v1, b);
        assert_eq!(shape.edge(1).v0, b);
        assert_eq!(shape.edge(1).v1, c);

        // Each edge is its own chain.
        assert_eq!(shape.chain(0).start, 0);
        assert_eq!(shape.chain(0).length, 1);
        assert_eq!(shape.chain(1).start, 1);
        assert_eq!(shape.chain(1).length, 1);

        assert_eq!(shape.chain_position(0).chain_id, 0);
        assert_eq!(shape.chain_position(0).offset, 0);
        assert_eq!(shape.chain_position(1).chain_id, 1);
    }

    #[test]
    fn test_singleton_constructor() {
        let a = p(0.0, 0.0);
        let b = p(1.0, 0.0);
        let shape = EdgeVectorShape::from_edge(a, b);
        assert_eq!(shape.num_edges(), 1);
        assert_eq!(shape.edge(0).v0, a);
        assert_eq!(shape.edge(0).v1, b);
    }

    #[test]
    fn test_add_edges() {
        let mut shape = EdgeVectorShape::new();
        shape.add(p(0.0, 0.0), p(1.0, 0.0));
        shape.add(p(1.0, 0.0), p(2.0, 0.0));
        assert_eq!(shape.num_edges(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let shape = EdgeVectorShape::from_edges(vec![(a, b), (b, c)]);
        let json = serde_json::to_string(&shape).unwrap();
        let back: EdgeVectorShape = serde_json::from_str(&json).unwrap();
        assert_eq!(shape.num_edges(), back.num_edges());
        for i in 0..shape.num_edges() {
            assert_eq!(shape.edge(i), back.edge(i));
        }
    }
}
