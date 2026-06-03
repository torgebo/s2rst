// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

//! A lightweight shape wrapper that delegates all calls to an inner shape.
//!
//! Useful for adding an existing shape to a new `ShapeIndex` without
//! copying its underlying data.
//!
//! Corresponds to C++ `s2wrapped_shape.h`.

use std::sync::Arc;

use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

/// A shape that wraps another shape via shared ownership (`Arc`).
///
/// All `Shape` trait methods are delegated to the wrapped shape.
/// This is useful for adding a shape that already exists in one
/// `ShapeIndex` to another `ShapeIndex` without cloning the data.
#[derive(Debug)]
pub struct WrappedShape {
    shape: Arc<dyn Shape>,
}

impl WrappedShape {
    /// Creates a new `WrappedShape` from a shared shape reference.
    pub fn new(shape: Arc<dyn Shape>) -> Self {
        WrappedShape { shape }
    }

    /// Returns the inner shape reference.
    pub fn inner(&self) -> &dyn Shape {
        &*self.shape
    }
}

impl Shape for WrappedShape {
    fn num_edges(&self) -> usize {
        self.shape.num_edges()
    }

    fn edge(&self, id: usize) -> Edge {
        self.shape.edge(id)
    }

    fn reference_point(&self) -> ReferencePoint {
        self.shape.reference_point()
    }

    fn num_chains(&self) -> usize {
        self.shape.num_chains()
    }

    fn chain(&self, chain_id: usize) -> Chain {
        self.shape.chain(chain_id)
    }

    fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge {
        self.shape.chain_edge(chain_id, offset)
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        self.shape.chain_position(edge_id)
    }

    fn dimension(&self) -> Dimension {
        self.shape.dimension()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::edge_vector_shape::EdgeVectorShape;

    #[test]
    fn test_wrapped_shape_delegates() {
        let mut evs = EdgeVectorShape::new();
        evs.add(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        );
        evs.add(
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        );
        let arc: Arc<dyn Shape> = Arc::new(evs);
        let wrapped = WrappedShape::new(arc);

        assert_eq!(wrapped.num_edges(), 2);
        assert_eq!(wrapped.dimension(), Dimension::Polyline);
        assert_eq!(wrapped.num_chains(), 2);
        let e = wrapped.edge(0);
        assert_eq!(e.v0, Point::from_coords(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_wrapped_shape_empty() {
        let evs = EdgeVectorShape::new();
        let arc: Arc<dyn Shape> = Arc::new(evs);
        let wrapped = WrappedShape::new(arc);
        assert!(wrapped.is_empty());
        assert!(!wrapped.is_full());
        assert_eq!(wrapped.num_edges(), 0);
    }

    #[test]
    fn test_wrapped_shape_shared() {
        let mut evs = EdgeVectorShape::new();
        evs.add(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        );
        let arc: Arc<dyn Shape> = Arc::new(evs);
        let w1 = WrappedShape::new(Arc::clone(&arc));
        let w2 = WrappedShape::new(Arc::clone(&arc));
        assert_eq!(w1.num_edges(), w2.num_edges());
        assert_eq!(w1.edge(0), w2.edge(0));
    }
}

#[cfg(test)]
#[path = "wrapped_shape_tests.rs"]
mod wrapped_shape_tests;
