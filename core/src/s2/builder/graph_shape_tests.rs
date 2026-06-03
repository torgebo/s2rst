// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for [`super::GraphShape`] — written for this crate, not
//! ported from upstream S2.

use super::GraphShape;
use crate::s2::Point;
use crate::s2::builder::graph::VertexId;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

fn pt(x: f64, y: f64, z: f64) -> Point {
    Point::from_coords(x, y, z)
}

/// A two-edge open path: `v0 → v1 → v2`.
fn sample() -> (Vec<Point>, GraphShape) {
    let verts = vec![pt(1.0, 0.0, 0.0), pt(0.0, 1.0, 0.0), pt(0.0, 0.0, 1.0)];
    let edges = vec![
        (VertexId::new(0), VertexId::new(1)),
        (VertexId::new(1), VertexId::new(2)),
    ];
    let shape = GraphShape::from_parts(verts.clone(), edges);
    (verts, shape)
}

#[test]
fn from_parts_reports_edges_and_geometry() {
    let (verts, shape) = sample();
    assert_eq!(shape.num_edges(), 2);
    assert_eq!(shape.edge(0), Edge::new(verts[0], verts[1]));
    assert_eq!(shape.edge(1), Edge::new(verts[1], verts[2]));
}

#[test]
fn dimension_is_polyline_without_interior() {
    let (_verts, shape) = sample();
    assert_eq!(shape.dimension(), Dimension::Polyline);
    assert!(!shape.has_interior());
}

#[test]
fn reference_point_is_origin_and_uncontained() {
    let (_verts, shape) = sample();
    assert_eq!(
        shape.reference_point(),
        ReferencePoint::new(Point::origin(), false)
    );
}

#[test]
fn every_edge_is_its_own_singleton_chain() {
    let (_verts, shape) = sample();
    assert_eq!(shape.num_chains(), shape.num_edges());
    for i in 0..shape.num_chains() {
        assert_eq!(shape.chain(i), Chain::new(i, 1));
        // `chain_edge` ignores the within-chain offset: every chain has length 1.
        assert_eq!(shape.chain_edge(i, 0), shape.edge(i));
        assert_eq!(shape.chain_edge(i, 7), shape.edge(i));
        // Inverse mapping: edge `i` sits at offset 0 of chain `i`.
        assert_eq!(shape.chain_position(i), ChainPosition::new(i, 0));
    }
}

#[test]
fn empty_graph_shape_is_empty_and_not_full() {
    let shape = GraphShape::from_parts(Vec::new(), Vec::new());
    assert_eq!(shape.num_edges(), 0);
    assert_eq!(shape.num_chains(), 0);
    assert!(shape.is_empty());
    assert!(!shape.is_full());
}
