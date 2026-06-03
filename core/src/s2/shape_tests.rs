// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for the `shape` module: the `Dimension`/`ShapeId`/
//! `ShapeEdgeId` conversions and operators, and the `reference_point_for_shape`
//! utility — paths the in-file tests do not reach. Written for this crate, not
//! ported from upstream S2.

use super::*;
use crate::s2::Point;

fn pt(x: f64, y: f64, z: f64) -> Point {
    Point::from_coords(x, y, z)
}

// ─── Dimension ────────────────────────────────────────────────────────────

#[test]
fn dimension_as_usize_and_display() {
    assert_eq!(Dimension::Point.as_usize(), 0);
    assert_eq!(Dimension::Polyline.as_usize(), 1);
    assert_eq!(Dimension::Polygon.as_usize(), 2);
    assert_eq!(Dimension::Point.to_string(), "0");
    assert_eq!(Dimension::Polyline.to_string(), "1");
    assert_eq!(Dimension::Polygon.to_string(), "2");
}

#[test]
fn dimension_into_integer_types() {
    assert_eq!(u8::from(Dimension::Polygon), 2u8);
    assert_eq!(i8::from(Dimension::Polyline), 1i8);
    assert_eq!(i32::from(Dimension::Point), 0i32);
    assert_eq!(usize::from(Dimension::Polygon), 2usize);
}

#[test]
fn dimension_try_from_valid_and_invalid() {
    assert_eq!(Dimension::try_from(0u8), Ok(Dimension::Point));
    assert_eq!(Dimension::try_from(2u8), Ok(Dimension::Polygon));
    assert!(Dimension::try_from(3u8).is_err());

    assert_eq!(Dimension::try_from(1i8), Ok(Dimension::Polyline));
    assert!(Dimension::try_from(-1i8).is_err());

    assert_eq!(Dimension::try_from(2i32), Ok(Dimension::Polygon));
    assert!(Dimension::try_from(99i32).is_err());
}

#[test]
fn dimension_ordering() {
    assert!(Dimension::Point < Dimension::Polyline);
    assert!(Dimension::Polyline < Dimension::Polygon);
}

// ─── ShapeId ──────────────────────────────────────────────────────────────

#[test]
fn shape_id_construction_and_accessors() {
    let id = ShapeId::new(5);
    assert_eq!(id.as_i32(), 5);
    assert_eq!(id.as_usize(), 5);
    assert_eq!(i32::from(id), 5);
    assert_eq!(ShapeId::from(7), ShapeId(7));
}

#[test]
fn shape_id_arithmetic_and_comparison() {
    let mut id = ShapeId::new(3);
    assert_eq!((id + 2).as_i32(), 5);
    assert_eq!((id - 1).as_i32(), 2);
    id += 4;
    assert_eq!(id.as_i32(), 7);
    // PartialEq<i32> / PartialOrd<i32> / Display
    assert!(id == 7);
    assert!(id > 5);
    assert!(id < 10);
    assert_eq!(format!("{id}"), "7");
}

#[test]
#[should_panic(expected = "non-negative")]
fn shape_id_negative_as_usize_panics() {
    let _ = ShapeId::new(-1).as_usize();
}

// ─── ShapeEdgeId / ShapeEdge ──────────────────────────────────────────────

#[test]
fn shape_edge_id_and_shape_edge_construction() {
    let id = ShapeEdgeId::new(2, 4);
    assert_eq!(id.shape_id, ShapeId(2));
    assert_eq!(id.edge_id, 4);

    let e = Edge::new(pt(1.0, 0.0, 0.0), pt(0.0, 1.0, 0.0));
    let se = ShapeEdge::new(id, e);
    assert_eq!(se.id, id);
    assert_eq!(se.edge, e);
}

// ─── Edge::cmp ────────────────────────────────────────────────────────────

#[test]
fn edge_cmp_orders_by_v0_then_v1() {
    use std::cmp::Ordering;
    let a = pt(1.0, 0.0, 0.0);
    let b = pt(0.0, 1.0, 0.0);
    let c = pt(0.0, 0.0, 1.0);
    let e_ab = Edge::new(a, b);
    let e_ac = Edge::new(a, c);
    assert_eq!(e_ab.cmp(&e_ab), Ordering::Equal);
    // Same v0, so the order is decided by v1 (b vs c) — and it is antisymmetric.
    assert_ne!(e_ab.cmp(&e_ac), Ordering::Equal);
    assert_eq!(e_ab.cmp(&e_ac), e_ac.cmp(&e_ab).reverse());
}

// ─── reference_point_for_shape ────────────────────────────────────────────

/// A minimal `Shape` whose edges/chains/dimension are set explicitly, used to
/// drive `reference_point_for_shape` down specific branches.
#[derive(Debug)]
struct TestShape {
    edges: Vec<Edge>,
    chains: Vec<Chain>,
    dim: Dimension,
}

impl Shape for TestShape {
    fn num_edges(&self) -> usize {
        self.edges.len()
    }
    fn edge(&self, id: usize) -> Edge {
        self.edges[id]
    }
    fn reference_point(&self) -> ReferencePoint {
        reference_point_for_shape(self)
    }
    fn num_chains(&self) -> usize {
        self.chains.len()
    }
    fn chain(&self, i: usize) -> Chain {
        self.chains[i]
    }
    fn chain_edge(&self, _chain_id: usize, _offset: usize) -> Edge {
        self.edges[0]
    }
    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition::new(0, edge_id)
    }
    fn dimension(&self) -> Dimension {
        self.dim
    }
}

#[test]
fn reference_point_no_edges_no_chains_is_uncontained() {
    let s = TestShape {
        edges: vec![],
        chains: vec![],
        dim: Dimension::Polygon,
    };
    assert_eq!(
        reference_point_for_shape(&s),
        ReferencePoint::new(Point::origin(), false)
    );
}

#[test]
fn reference_point_no_edges_with_chain_is_full() {
    // No edges but a chain present → an empty loop → the shape is full.
    let s = TestShape {
        edges: vec![],
        chains: vec![Chain::new(0, 0)],
        dim: Dimension::Polygon,
    };
    assert_eq!(
        reference_point_for_shape(&s),
        ReferencePoint::new(Point::origin(), true)
    );
}

#[test]
fn reference_point_triangle_loop_uses_an_unbalanced_vertex() {
    // Three directed edges forming a closed loop a→b→c→a. The first edge's
    // start vertex `a` is unbalanced (one outgoing, one incoming edge), so the
    // utility returns it as the reference point.
    let a = pt(1.0, 0.0, 0.0);
    let b = pt(0.0, 1.0, 0.0);
    let c = pt(0.0, 0.0, 1.0);
    let s = TestShape {
        edges: vec![Edge::new(a, b), Edge::new(b, c), Edge::new(c, a)],
        chains: vec![Chain::new(0, 3)],
        dim: Dimension::Polygon,
    };
    let rp = reference_point_for_shape(&s);
    assert_eq!(rp.point, a);
}
