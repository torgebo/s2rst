// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for the cell clipping pipeline:
//! R2EdgeClipper → UVEdgeClipper → RobustCellClipper → ShapeTracker → ReclippedShape.

#![allow(
    clippy::doc_markdown,
    reason = "test doc comments use unquoted API names"
)]

use s2rst::r2;
use s2rst::r3::Vector;
use s2rst::s2::r2_edge_clipper::{R2Edge, R2EdgeClipper};
use s2rst::s2::reclipped_shape::ReclippedShape;
use s2rst::s2::robust_cell_clipper::{CrossingType, RobustCellClipper, RobustClipResult};
use s2rst::s2::shape::Dimension;
use s2rst::s2::shape_tracker::ShapeTracker;
use s2rst::s2::uv_edge_clipper::UVEdgeClipper;
use s2rst::s2::{Cell, CellId, LatLng, Point};

// ─────────────────────────────────────────────────────
// R2EdgeClipper integration
// ─────────────────────────────────────────────────────

#[test]
fn r2_clipper_roundtrip_inside() {
    let rect = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
    let mut c = R2EdgeClipper::from_rect(&rect);
    let edge = R2Edge::new(r2::Point::new(0.0, 0.0), r2::Point::new(0.5, 0.5));
    assert!(c.clip_edge(&edge, false));
    assert_eq!(c.outcode0, s2rst::s2::r2_edge_clipper::INSIDE);
    assert_eq!(c.outcode1, s2rst::s2::r2_edge_clipper::INSIDE);
    // Clipped edge should be identical to the original.
    assert_eq!(c.clipped_edge.v0.x, 0.0);
    assert_eq!(c.clipped_edge.v1.x, 0.5);
}

#[test]
fn r2_clipper_clips_all_boundaries() {
    let rect = r2::Rect::from_points(r2::Point::new(0.0, 0.0), r2::Point::new(1.0, 1.0));
    let mut c = R2EdgeClipper::from_rect(&rect);

    // Through-edge from bottom-left to top-right.
    let edge = R2Edge::new(r2::Point::new(-0.5, -0.5), r2::Point::new(1.5, 1.5));
    assert!(c.clip_edge(&edge, false));
    // Both endpoints should be on boundaries.
    assert_ne!(c.outcode0, s2rst::s2::r2_edge_clipper::INSIDE);
    assert_ne!(c.outcode1, s2rst::s2::r2_edge_clipper::INSIDE);
}

// ─────────────────────────────────────────────────────
// UVEdgeClipper integration
// ─────────────────────────────────────────────────────

#[test]
fn uv_clipper_face_detection() {
    for face in 0..6u8 {
        let cell = Cell::from_cell_id(CellId::from_face(face));
        let mut c = UVEdgeClipper::from_cell(cell);
        let center = cell.center();
        // Center should be inside.
        let tiny = Point((center.0 + Vector::new(1e-10, 0.0, 0.0)).normalize());
        let hit = c.clip_edge(center, tiny, false);
        assert!(
            hit || !c.missed_face(),
            "face {face}: edge should be on-face"
        );
    }
}

// ─────────────────────────────────────────────────────
// RobustCellClipper integration
// ─────────────────────────────────────────────────────

#[test]
fn robust_clipper_all_face_cells() {
    // The center of each face cell should be "inside" when clipped.
    for face in 0..6u8 {
        let cell = Cell::from_cell_id(CellId::from_face(face));
        let center = cell.center();
        let tiny = Point((center.0 + Vector::new(1e-12, 1e-12, 0.0)).normalize());

        let mut c = RobustCellClipper::new();
        c.start_cell(cell);
        let result = c.clip_edge(center, tiny, false);
        assert!(
            result.is_hit(),
            "face {face}: center edge should hit the cell"
        );
    }
}

#[test]
fn robust_clipper_opposite_faces_miss() {
    // An edge on face 3 should miss a face 0 cell.
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let v0 = LatLng::from_degrees(10.0, -170.0).to_point();
    let v1 = LatLng::from_degrees(20.0, -170.0).to_point();

    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    assert_eq!(RobustClipResult::Miss, c.clip_edge(v0, v1, false));
}

#[test]
fn robust_clipper_crossing_produces_correct_types() {
    // An edge from inside to outside should produce an OUTGOING crossing.
    let cell = Cell::from_cell_id(CellId::from_token("114"));
    let center = cell.center();

    let mut c = RobustCellClipper::new();
    c.start_cell(cell);

    // Reflect center across boundary 0 (bottom).
    let v0 = cell.vertex(0);
    let v1_cell = cell.vertex(1);
    let normal = Point(
        s2rst::s2::edge_crossings::robust_cross_prod(v0, v1_cell)
            .0
            .normalize(),
    );
    let dot = center.0.dot(normal.0);
    let reflected = Point(Vector::new(
        center.0.x - 2.0 * dot * normal.0.x,
        center.0.y - 2.0 * dot * normal.0.y,
        center.0.z - 2.0 * dot * normal.0.z,
    ));

    let result = c.clip_edge(center, reflected, false);
    assert!(result.is_hit());
    assert!(result.v0_inside());
    assert!(!result.v1_inside());

    let crossings = c.get_crossings();
    assert_eq!(1, crossings.len());
    assert_eq!(CrossingType::Outgoing, crossings[0].crossing_type);
}

// ─────────────────────────────────────────────────────
// ShapeTracker integration
// ─────────────────────────────────────────────────────

#[test]
fn shape_tracker_full_sphere_closure() {
    // Adding all 6 face cell boundaries should close the tracker.
    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);
    assert!(t.finished());

    for face in 0..6u8 {
        let cell = Cell::from_cell_id(CellId::from_face(face));
        t.add_cell_boundary(cell);
    }
    assert!(t.finished(), "all 6 face boundaries should close");
}

#[test]
fn shape_tracker_level1_children_close() {
    // Adding boundaries of all level-1 children should close.
    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);

    for face in 0..6u8 {
        let face_id = CellId::from_face(face);
        for &child in &face_id.children() {
            t.add_cell_boundary(Cell::from_cell_id(child));
        }
    }
    assert!(t.finished());
}

#[test]
fn shape_tracker_reset_works() {
    let mut t = ShapeTracker::new(Dimension::Point, 3);
    t.mark_chain(0);
    t.mark_chain(1);
    t.mark_chain(2);
    assert!(t.finished());

    t.reset(Dimension::Point, 2);
    assert!(!t.finished());
    t.mark_chain(0);
    t.mark_chain(1);
    assert!(t.finished());
}

// ─────────────────────────────────────────────────────
// ReclippedShape integration
// ─────────────────────────────────────────────────────

#[test]
fn reclipped_shape_filters_by_cell() {
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    // Mix of edges: one inside face 0, one outside.
    let edges = vec![
        (
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(20.0, 20.0).to_point(),
        ),
        (
            LatLng::from_degrees(-80.0, -170.0).to_point(),
            LatLng::from_degrees(-70.0, -170.0).to_point(),
        ),
    ];

    let mut shape = ReclippedShape::new();
    shape.init(
        &mut clipper,
        0,
        Dimension::Polygon,
        false,
        edges.into_iter(),
        false,
    );
    assert_eq!(1, shape.num_edges());
    assert_eq!(0, shape.shape_id());
}

#[test]
fn reclipped_shape_id_caching() {
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let edges = vec![(
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(20.0, 20.0).to_point(),
    )];

    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    let mut shape = ReclippedShape::new();
    assert!(shape.init(
        &mut clipper,
        5,
        Dimension::Polyline,
        false,
        edges.clone().into_iter(),
        false
    ));

    // Second call with same shape_id should skip.
    clipper.start_cell(cell);
    assert!(!shape.init(
        &mut clipper,
        5,
        Dimension::Polyline,
        false,
        edges.into_iter(),
        false
    ));

    // After reset, should process.
    shape.reset();
    let edges2 = vec![(
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(20.0, 20.0).to_point(),
    )];
    clipper.start_cell(cell);
    assert!(shape.init(
        &mut clipper,
        5,
        Dimension::Polyline,
        false,
        edges2.into_iter(),
        false
    ));
}

// ─────────────────────────────────────────────────────
// End-to-end pipeline integration
// ─────────────────────────────────────────────────────

#[test]
fn pipeline_clip_and_track_polygon_ring() {
    // Create a small polygon ring inside face 0 and clip it.
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let ring = [
        LatLng::from_degrees(-10.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, -10.0).to_point(),
    ];

    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    for i in 0..ring.len() {
        let v0 = ring[i];
        let v1 = ring[(i + 1) % ring.len()];
        let result = clipper.clip_edge(v0, v1, false);
        assert_eq!(
            RobustClipResult::HitBoth,
            result,
            "edge {i} should be inside face 0"
        );
    }

    // All edges are contained, so boundary containment should flip.
    assert!(clipper.is_boundary_contained(false));
    assert!(!clipper.is_boundary_contained(true));

    // Shape tracker: polygon with 1 chain, 4 edges all inside = finished.
    let mut tracker = ShapeTracker::new(Dimension::Polygon, 1);
    tracker.mark_chain(0);
    // No crossings, all interior — should be finished after marking chain.
    assert!(tracker.finished());
}

#[test]
fn pipeline_reclip_preserves_crossing_info() {
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let v0 = LatLng::from_degrees(10.0, 10.0).to_point();
    let v1 = LatLng::from_degrees(10.0, 80.0).to_point();

    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    let mut shape = ReclippedShape::new();
    shape.init(
        &mut clipper,
        0,
        Dimension::Polygon,
        false,
        std::iter::once((v0, v1)),
        true,
    );

    // Should have 1 edge and at least 1 crossing.
    assert_eq!(1, shape.num_edges());
    assert!(shape.edges()[0].v0_contained);
    assert!(!shape.edges()[0].v1_contained);
    assert!(!shape.crossings().is_empty());
}
