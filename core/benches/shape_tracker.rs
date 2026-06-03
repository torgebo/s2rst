// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::shape::Dimension;
use s2rst::s2::shape_tracker::ShapeTracker;
use s2rst::s2::{Cell, CellId};

// Mark all chains for a point shape
#[library_benchmark]
fn mark_chains_100() -> bool {
    let mut t = ShapeTracker::new(Dimension::Point, 100);
    for i in 0..100 {
        t.mark_chain(i);
    }
    black_box(t.finished())
}

// Add and cancel polygon intervals
#[library_benchmark]
fn add_cancel_intervals() -> bool {
    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);
    for i in 0..50 {
        let base = i * 100;
        t.add_interval(0, 0, 500, base, base + 100);
    }
    for i in (0..50).rev() {
        let base = i * 100;
        t.add_interval(0, 0, 500, base + 100, base);
    }
    black_box(t.finished())
}

// Add all 6 face cell boundaries
#[library_benchmark]
fn add_all_face_boundaries() -> bool {
    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);
    for face in 0..6 {
        let cell = Cell::from_cell_id(CellId::from_face(face));
        t.add_cell_boundary(cell);
    }
    black_box(t.finished())
}

// Process crossings for polygon (simulated)
#[library_benchmark]
fn process_crossings_face() -> bool {
    use s2rst::s2::CellEdge;
    use s2rst::s2::robust_cell_clipper::{Crossing, CrossingType};

    let crossings: Vec<Crossing> = vec![
        Crossing {
            boundary: CellEdge::Bottom,
            crossing_type: CrossingType::Incoming,
            coord: -1.0,
            intercept: -0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Bottom,
            crossing_type: CrossingType::Outgoing,
            coord: -1.0,
            intercept: 0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Right,
            crossing_type: CrossingType::Incoming,
            coord: 1.0,
            intercept: -0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Right,
            crossing_type: CrossingType::Outgoing,
            coord: 1.0,
            intercept: 0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Top,
            crossing_type: CrossingType::Incoming,
            coord: 1.0,
            intercept: 0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Top,
            crossing_type: CrossingType::Outgoing,
            coord: 1.0,
            intercept: -0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Left,
            crossing_type: CrossingType::Incoming,
            coord: -1.0,
            intercept: 0.75,
            edge_index: 0,
        },
        Crossing {
            boundary: CellEdge::Left,
            crossing_type: CrossingType::Outgoing,
            coord: -1.0,
            intercept: -0.75,
            edge_index: 0,
        },
    ];
    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);
    for face in 0..6 {
        let cell = Cell::from_cell_id(CellId::from_face(face));
        t.process_crossings(cell, &crossings);
    }
    black_box(t.finished())
}

// Polyline add/del points
#[library_benchmark]
fn polyline_point_tracking() -> bool {
    let mut t = ShapeTracker::new(Dimension::Polyline, 1);
    t.mark_chain(0);
    for i in 0..50 {
        t.add_point(0, 0, i * 10, i * 7);
    }
    for i in 0..50 {
        t.del_point(0, 0, i * 10, i * 7);
    }
    black_box(t.finished())
}

library_benchmark_group!(
    name = tracker;
    benchmarks =
        mark_chains_100,
        add_cancel_intervals,
        add_all_face_boundaries,
        process_crossings_face,
        polyline_point_tracking,
);

main!(library_benchmark_groups = tracker);
