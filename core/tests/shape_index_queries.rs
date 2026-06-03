// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for `ShapeIndex` and query types, ported from C++ tests.

use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::crossing_edge_query::{CrossingEdgeQuery, CrossingType};
use s2rst::s2::shape::{Dimension, Shape};
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::text_format;
use s2rst::s2::{LatLng, Loop, Point};

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

// ---------------------------------------------------------------------------
// ShapeIndex basic tests
// ---------------------------------------------------------------------------

#[test]
fn test_shape_index_empty() {
    let index = ShapeIndex::new();
    assert_eq!(index.len(), 0);
    assert!(index.is_empty());
    assert_eq!(index.num_edges(), 0);
}

#[test]
fn test_shape_index_single_loop() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    let num_verts = l.num_vertices();
    index.add(Box::new(l));
    index.build();

    assert_eq!(index.len(), 1);
    assert!(!index.is_empty());
    assert_eq!(index.num_edges(), num_verts);
}

#[test]
fn test_shape_index_multiple_shapes() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    let pl = text_format::make_polyline("20:20, 30:30, 40:20");

    let id0 = index.add(Box::new(l));
    let id1 = index.add(Box::new(pl));
    index.build();

    assert_eq!(index.len(), 2);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
    assert!(index.shape(0).is_some());
    assert!(index.shape(1).is_some());
    assert!(index.shape(2).is_none());
}

#[test]
fn test_shape_index_rebuild_idempotent() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    index.add(Box::new(l));
    index.build();
    let edges1 = index.num_edges();

    index.build();
    let edges2 = index.num_edges();
    assert_eq!(edges1, edges2, "rebuild should be idempotent");
}

// ---------------------------------------------------------------------------
// ContainsPointQuery tests
// ---------------------------------------------------------------------------

#[test]
fn test_contains_point_semiopen() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("-10:-10, -10:10, 10:10, 10:-10");
    index.add(Box::new(l));
    index.build();

    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    let inside = p(0.0, 0.0);
    let outside = p(50.0, 50.0);

    assert!(query.contains(inside), "interior point should be contained");
    assert!(
        !query.contains(outside),
        "exterior point should not be contained"
    );
}

#[test]
fn test_contains_point_closed() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    index.add(Box::new(l));
    index.build();

    let mut query = ContainsPointQuery::new(&index, VertexModel::Closed);
    // Under the Closed model, all vertices should be contained.
    let vertex = text_format::parse_point("0:0");
    assert!(
        query.contains(vertex),
        "vertex should be contained under Closed model"
    );
}

#[test]
fn test_contains_point_open() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    index.add(Box::new(l));
    index.build();

    let mut query = ContainsPointQuery::new(&index, VertexModel::Open);
    // Under the Open model, no vertices should be contained.
    let vertex = text_format::parse_point("0:0");
    assert!(
        !query.contains(vertex),
        "vertex should not be contained under Open model"
    );
}

#[test]
fn test_contains_point_polygon() {
    let mut index = ShapeIndex::new();
    let poly = text_format::make_polygon("-10:-10, -10:10, 10:10, 10:-10");
    index.add(Box::new(poly));
    index.build();

    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    let inside = p(0.0, 0.0);
    assert!(
        query.contains(inside),
        "polygon interior should contain point"
    );
}

#[test]
fn test_containing_shape_ids() {
    let mut index = ShapeIndex::new();
    // Two overlapping loops.
    let l1 = text_format::make_loop("-20:-20, -20:20, 20:20, 20:-20");
    let l2 = text_format::make_loop("-10:-10, -10:10, 10:10, 10:-10");
    index.add(Box::new(l1));
    index.add(Box::new(l2));
    index.build();

    let mut query = ContainsPointQuery::new(&index, VertexModel::SemiOpen);
    let center = p(0.0, 0.0);
    let ids = query.containing_shape_ids(center);
    assert_eq!(ids.len(), 2, "center point should be inside both shapes");
    assert!(ids.contains(&s2rst::s2::shape::ShapeId(0)));
    assert!(ids.contains(&s2rst::s2::shape::ShapeId(1)));
}

// ---------------------------------------------------------------------------
// CrossingEdgeQuery tests
// ---------------------------------------------------------------------------

#[test]
fn test_crossing_edge_query_basic() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("-10:-10, -10:10, 10:10, 10:-10");
    index.add(Box::new(l));
    index.build();

    let mut query = CrossingEdgeQuery::new(&index);
    // An edge that crosses through the square loop boundary.
    let a = p(0.0, -20.0);
    let b = p(0.0, 20.0);
    let shape = index.shape(0).unwrap();
    let crossings = query.crossings(a, b, shape, 0, CrossingType::All);
    assert!(
        !crossings.is_empty(),
        "edge crossing through loop should find crossings"
    );
}

#[test]
fn test_crossing_edge_query_no_crossing() {
    let mut index = ShapeIndex::new();
    let l = text_format::make_loop("-10:-10, -10:10, 10:10, 10:-10");
    index.add(Box::new(l));
    index.build();

    let mut query = CrossingEdgeQuery::new(&index);
    // An edge entirely inside the loop (no boundary crossing).
    let a = p(0.0, 0.0);
    let b = p(1.0, 1.0);
    let shape = index.shape(0).unwrap();
    let crossings = query.crossings(a, b, shape, 0, CrossingType::Interior);
    assert!(
        crossings.is_empty(),
        "edge inside loop should have no interior crossings"
    );
}

// ---------------------------------------------------------------------------
// Shape trait tests
// ---------------------------------------------------------------------------

#[test]
fn test_shape_trait_loop() {
    let l = text_format::make_loop("0:0, 0:10, 10:0");
    assert_eq!(l.dimension(), Dimension::Polygon);
    assert!(!l.is_empty());
    assert!(!l.is_full());
    assert_eq!(l.num_edges(), 3);
    assert_eq!(l.num_chains(), 1);
}

#[test]
fn test_shape_trait_polyline() {
    let pl = text_format::make_polyline("0:0, 10:10, 20:0");
    assert_eq!(pl.dimension(), Dimension::Polyline);
    assert!(!pl.is_empty());
    assert!(!pl.is_full());
    assert_eq!(pl.num_edges(), 2); // 3 vertices = 2 edges
    assert_eq!(pl.num_chains(), 1);
}

#[test]
fn test_shape_trait_empty_loop() {
    let l = Loop::empty();
    assert!(l.is_empty());
    assert!(!l.is_full());
    assert_eq!(l.num_edges(), 0);
}

#[test]
fn test_shape_trait_full_loop() {
    let l = Loop::full();
    assert!(!l.is_empty());
    assert!(l.is_full());
    assert_eq!(l.num_edges(), 0);
}
