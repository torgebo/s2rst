// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// GraphShape: wraps graph edge data as a Shape (dimension=1).
//
// C++ ref: s2builderutil_graph_shape.h
//
// Since Shape requires Send + Sync, GraphShape owns its data rather than
// holding a reference to a Graph.

use crate::s2::Point;
use crate::s2::builder::graph::{Graph, VertexId};
use crate::s2::shape::Dimension;
use crate::s2::shape::{Chain, ChainPosition, Edge, ReferencePoint, Shape};

/// A Shape that wraps a copy of graph vertices and edges.
/// Each graph edge becomes its own chain (dimension=1).
#[derive(Debug)]
pub(crate) struct GraphShape {
    vertices: Vec<Point>,
    edges: Vec<(VertexId, VertexId)>,
}

impl GraphShape {
    pub(crate) fn from_graph(graph: &Graph) -> Self {
        GraphShape {
            vertices: graph.vertices().to_vec(),
            edges: graph.edges().to_vec(),
        }
    }

    /// Creates a `GraphShape` from owned vertex and edge data.
    pub(crate) fn from_parts(vertices: Vec<Point>, edges: Vec<(VertexId, VertexId)>) -> Self {
        GraphShape { vertices, edges }
    }
}

impl Shape for GraphShape {
    fn num_edges(&self) -> usize {
        self.edges.len()
    }

    fn edge(&self, id: usize) -> Edge {
        let (v0, v1) = self.edges[id];
        Edge::new(self.vertices[v0.as_usize()], self.vertices[v1.as_usize()])
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), false)
    }

    fn num_chains(&self) -> usize {
        self.edges.len()
    }

    fn chain(&self, i: usize) -> Chain {
        Chain::new(i, 1)
    }

    fn chain_edge(&self, i: usize, _j: usize) -> Edge {
        self.edge(i)
    }

    fn chain_position(&self, e: usize) -> ChainPosition {
        ChainPosition::new(e, 0)
    }
}

#[cfg(test)]
#[path = "graph_shape_tests.rs"]
mod graph_shape_tests;
