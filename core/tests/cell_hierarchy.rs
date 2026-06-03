// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for S2 Cell and `CellId` hierarchy operations.
//!
//! Ported from C++ `s2cell_test.cc` and `s2cell_id_test.cc`.

use s2rst::s2::{Cell, CellId, Face, LatLng, Point, Region};
use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// CellId tests
// ---------------------------------------------------------------------------

/// Face cells (level 0): `CellId::from_face_pos_level(f`, 0, 0) has level=0,
/// face=f, `is_face()=true`.
#[test]
fn test_cellid_face_level() {
    for face in Face::iter() {
        let id = CellId::from_face_pos_level(face, 0, 0);
        assert!(id.is_valid(), "face {face} cell should be valid");
        assert_eq!(id.level(), 0, "face {face} cell should be level 0");
        assert_eq!(
            id.face(),
            face,
            "face {face} cell should report face={face}"
        );
        assert!(id.is_face(), "face {face} cell should be a face cell");
        assert!(!id.is_leaf(), "face {face} cell should not be a leaf");
    }
}

/// Create level-5 cell. `parent()` is level-4. child(0..3) all have level 6,
/// parent is original cell.
#[test]
fn test_cellid_parent_child() {
    let face_id = CellId::from_face_pos_level(3, 0, 0);
    let level5 = face_id.children()[0].children()[1].children()[2].children()[0].children()[1];
    assert_eq!(level5.level(), 5);

    // parent() should be level 4
    let parent = level5.parent();
    assert_eq!(parent.level(), 4);
    assert!(parent.contains(level5));

    // children() all have level 6 and their parent is the original cell
    let children = level5.children();
    assert_eq!(children.len(), 4);
    for (i, child) in children.iter().enumerate() {
        assert_eq!(
            child.level(),
            6,
            "child {i} should be level 6, got {}",
            child.level()
        );
        assert_eq!(
            child.parent(),
            level5,
            "child {i} parent should be the original level-5 cell"
        );
    }
}

/// Parent contains child, child intersects parent. Sibling doesn't contain
/// sibling.
#[test]
fn test_cellid_containment() {
    let parent = CellId::from_face_pos_level(0, 0, 0).children()[0];
    assert_eq!(parent.level(), 1);

    let children = parent.children();
    let child = children[0];

    // Parent contains child
    assert!(parent.contains(child));

    // Child intersects parent
    assert!(child.intersects(parent));

    // Child does not contain parent
    assert!(!child.contains(parent));

    // Sibling doesn't contain sibling
    assert!(!children[0].contains(children[1]));
    assert!(!children[1].contains(children[0]));
    assert!(!children[2].contains(children[3]));

    // But siblings do intersect themselves
    assert!(children[0].intersects(children[0]));

    // Siblings at the same level don't intersect each other
    assert!(!children[0].intersects(children[1]));
    assert!(!children[1].intersects(children[2]));
}

/// `CellId::from_point` -> `to_point` -> `from_point` gives same `cell_id` at max
/// level.
#[test]
fn test_cellid_from_point_roundtrip() {
    let test_points = [
        Point::from_coords(1.0, 0.0, 0.0),
        Point::from_coords(0.0, 1.0, 0.0),
        Point::from_coords(0.0, 0.0, 1.0),
        Point::from_coords(1.0, 1.0, 1.0),
        Point::from_coords(-1.0, 0.5, 0.3),
        Point::from_coords(0.1, -0.8, 0.6),
    ];

    for (i, &p) in test_points.iter().enumerate() {
        let id1 = CellId::from_point(&p);
        assert!(id1.is_valid(), "point {i}: cell id should be valid");
        assert!(
            id1.is_leaf(),
            "point {i}: from_point should return a leaf cell"
        );

        let center = id1.to_point();
        let id2 = CellId::from_point(&center);
        assert_eq!(
            id1, id2,
            "point {i}: roundtrip from_point -> to_point -> from_point should give same cell"
        );
    }
}

/// `from_lat_lng` for (0,0), (90,0), (-90,0) etc. Check face assignments.
#[test]
fn test_cellid_from_lat_lng() {
    // (0,0) -> face 0 (positive x-axis direction)
    let id_00 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 0.0));
    assert!(id_00.is_valid());
    assert_eq!(id_00.face(), Face::F0);

    // (0, 90) -> face 1 (positive y-axis direction)
    let id_090 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 90.0));
    assert!(id_090.is_valid());
    assert_eq!(id_090.face(), Face::F1);

    // (90, 0) -> face 2 (north pole, positive z-axis)
    let id_900 = CellId::from_lat_lng(&LatLng::from_degrees(90.0, 0.0));
    assert!(id_900.is_valid());
    assert_eq!(id_900.face(), Face::F2);

    // (0, 180) -> face 3 (negative x-axis direction)
    let id_0180 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, 180.0));
    assert!(id_0180.is_valid());
    assert_eq!(id_0180.face(), Face::F3);

    // (0, -90) -> face 4 (negative y-axis direction)
    let id_0n90 = CellId::from_lat_lng(&LatLng::from_degrees(0.0, -90.0));
    assert!(id_0n90.is_valid());
    assert_eq!(id_0n90.face(), Face::F4);

    // (-90, 0) -> face 5 (south pole, negative z-axis)
    let id_n900 = CellId::from_lat_lng(&LatLng::from_degrees(-90.0, 0.0));
    assert!(id_n900.is_valid());
    assert_eq!(id_n900.face(), Face::F5);
}

/// `range_min` and `range_max` of a level-5 cell are both level-30 leaves;
/// `range_min.level()` == 30.
#[test]
fn test_cellid_range() {
    let id = CellId::from_face_pos_level(2, 0, 0).children()[1].children()[2].children()[0]
        .children()[3]
        .children()[1];
    assert_eq!(id.level(), 5);

    let rmin = id.range_min();
    let rmax = id.range_max();

    // Both range endpoints are leaves (level 30)
    assert!(rmin.is_leaf(), "range_min should be a leaf");
    assert_eq!(rmin.level(), 30, "range_min should be at level 30");
    assert!(rmax.is_leaf(), "range_max should be a leaf");
    assert_eq!(rmax.level(), 30, "range_max should be at level 30");

    // The parent cell contains both range endpoints
    assert!(id.contains(rmin));
    assert!(id.contains(rmax));

    // range_min <= range_max
    assert!(rmin.id() <= rmax.id());
}

/// `children()` returns 4 cells, all at level+1, all contained by parent.
#[test]
fn test_cellid_children_array() {
    let parent = CellId::from_face_pos_level(1, 0, 0).children()[2];
    assert_eq!(parent.level(), 1);

    let children = parent.children();
    assert_eq!(children.len(), 4);

    for (i, child) in children.iter().enumerate() {
        assert_eq!(
            child.level(),
            parent.level() + 1,
            "child {i} should be at level {}",
            parent.level() + 1
        );
        assert!(parent.contains(*child), "parent should contain child {i}");
        assert!(child.is_valid(), "child {i} should be valid");
    }

    // Children should all be distinct
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(
                children[i], children[j],
                "children {i} and {j} should be distinct"
            );
        }
    }
}

/// advance(0) = self, advance(1) = `next()`, advance(-1) = `prev()`.
#[test]
fn test_cellid_advance() {
    let id = CellId::from_face_pos_level(4, 0, 0).children()[1].children()[0].children()[3];
    assert_eq!(id.level(), 3);

    // advance(0) is identity
    assert_eq!(id.advance(0), id);

    // advance(1) equals next()
    assert_eq!(id.advance(1), id.next());

    // advance(-1) equals prev()
    assert_eq!(id.advance(-1), id.prev());

    // advance(1) followed by advance(-1) returns to the original
    assert_eq!(id.advance(1).advance(-1), id);

    // advance(4) equals calling next() four times
    let mut stepped = id;
    for _ in 0..4 {
        stepped = stepped.next();
    }
    assert_eq!(id.advance(4), stepped);
}

// ---------------------------------------------------------------------------
// Cell tests
// ---------------------------------------------------------------------------

/// Cell from face-0 level-10 cellid: face=0, level=10, 4 vertices not equal.
#[test]
fn test_cell_from_cellid() {
    let id = CellId::from_face_pos_level(0, 0, 0);
    // Navigate down to level 10 by repeatedly taking child 0.
    let mut cid = id;
    for _ in 0..10 {
        cid = cid.children()[0];
    }
    assert_eq!(cid.level(), 10);

    let cell = Cell::from_cell_id(cid);
    assert_eq!(cell.face(), Face::F0);
    assert_eq!(cell.level(), 10);
    assert_eq!(cell.id(), cid);

    // All 4 vertices should be distinct from each other
    for i in 0..4 {
        for j in (i + 1)..4 {
            assert_ne!(
                cell.vertex(i),
                cell.vertex(j),
                "vertex {i} and vertex {j} should be distinct"
            );
        }
    }
}

/// All 4 vertices of a cell are distinct points.
#[test]
fn test_cell_vertices_distinct() {
    // Test on several different cells at various levels and faces
    let test_cells = [
        Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 0)),
        Cell::from_cell_id(CellId::from_face_pos_level(3, 0, 0).children()[2]),
        Cell::from_point(Point::from_coords(0.5, 0.7, -0.3)),
        Cell::from_cell_id(
            CellId::from_face_pos_level(5, 0, 0).children()[1].children()[3].children()[0],
        ),
    ];

    for (ci, cell) in test_cells.iter().enumerate() {
        let vertices: [Point; 4] = [
            cell.vertex(0),
            cell.vertex(1),
            cell.vertex(2),
            cell.vertex(3),
        ];

        for i in 0..4 {
            // Each vertex should be approximately unit length
            let norm = vertices[i].vector().norm();
            assert!(
                (norm - 1.0).abs() < 1e-14,
                "cell {ci}, vertex {i}: norm={norm}, expected ~1.0"
            );

            for j in (i + 1)..4 {
                assert_ne!(
                    vertices[i], vertices[j],
                    "cell {ci}: vertex {i} and vertex {j} should be distinct"
                );
            }
        }
    }
}

/// Cell contains its own center point (`cell_id.to_point()`).
#[test]
fn test_cell_contains_center() {
    for face in Face::iter() {
        // Level 0 face cell
        let cell0 = Cell::from_cell_id(CellId::from_face_pos_level(face, 0, 0));
        let center0 = cell0.id().to_point();
        assert!(
            cell0.contains_point(center0),
            "face {face} level-0 cell should contain its center"
        );

        // A deeper cell
        let deeper_id =
            CellId::from_face_pos_level(face, 0, 0).children()[1].children()[2].children()[0];
        let deeper_cell = Cell::from_cell_id(deeper_id);
        let deeper_center = deeper_id.to_point();
        assert!(
            deeper_cell.contains_point(deeper_center),
            "face {face} level-3 cell should contain its center"
        );
    }

    // Also test a leaf cell
    let leaf_id = CellId::from_point(&Point::from_coords(0.3, -0.6, 0.8));
    let leaf_cell = Cell::from_cell_id(leaf_id);
    assert!(leaf_cell.is_leaf());
    assert!(
        leaf_cell.contains_point(leaf_id.to_point()),
        "leaf cell should contain its center"
    );
}

/// `children()` for level < 30 returns Some with 4 children; for leaf returns
/// None.
#[test]
fn test_cell_children() {
    // Non-leaf cell should have children
    let parent = Cell::from_cell_id(CellId::from_face_pos_level(2, 0, 0).children()[0]);
    assert!(!parent.is_leaf());

    let children = parent.children();
    assert!(children.is_some(), "non-leaf cell should have children");

    let children = children.unwrap();
    assert_eq!(children.len(), 4);
    for (i, child) in children.iter().enumerate() {
        assert_eq!(
            child.level(),
            parent.level() + 1,
            "child {i} should be at level {}",
            parent.level() + 1
        );
        assert!(
            parent.contains_cell(*child),
            "parent should contain child {i}"
        );
    }

    // Leaf cell should return None
    let leaf = Cell::from_point(Point::from_coords(1.0, 0.0, 0.0));
    assert!(leaf.is_leaf());
    assert!(
        leaf.children().is_none(),
        "leaf cell should not have children"
    );
}

/// Parent cell area > child cell area.
#[test]
fn test_cell_area_ordering() {
    let parent = Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 0));
    let child = Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 0).children()[0]);

    let parent_area = parent.approx_area();
    let child_area = child.approx_area();

    assert!(
        parent_area > child_area,
        "parent area ({parent_area}) should be greater than child area ({child_area})"
    );

    // Sanity check: face cell area should be roughly 4*pi/6
    let expected_face_area = 4.0 * PI / 6.0;
    assert!(
        (parent.average_area() - expected_face_area).abs() < 1e-10,
        "face cell average area should be ~{expected_face_area}, got {}",
        parent.average_area()
    );

    // Test across multiple levels
    let mut prev_area = parent_area;
    let mut cid = CellId::from_face_pos_level(0, 0, 0);
    for level in 1..=5u8 {
        cid = cid.children()[0];
        let cell = Cell::from_cell_id(cid);
        let area = cell.approx_area();
        assert!(
            prev_area > area,
            "level {level}: parent area ({prev_area}) should be > child area ({area})"
        );
        prev_area = area;
    }
}

/// `cap_bound()` contains all 4 vertices of the cell.
#[test]
fn test_cell_cap_bound_contains_vertices() {
    let test_cells = [
        Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 0)),
        Cell::from_cell_id(CellId::from_face_pos_level(1, 0, 0).children()[1].children()[2]),
        Cell::from_cell_id(
            CellId::from_face_pos_level(4, 0, 0).children()[0].children()[1].children()[2]
                .children()[3],
        ),
        Cell::from_point(Point::from_coords(-0.5, 0.8, 0.1)),
    ];

    for (ci, cell) in test_cells.iter().enumerate() {
        let cap = cell.cap_bound();
        assert!(!cap.is_empty(), "cell {ci}: cap_bound should not be empty");

        for k in 0..4 {
            let v = cell.vertex(k);
            assert!(
                cap.contains_point(v),
                "cell {ci}: cap_bound should contain vertex {k}"
            );
        }

        // The cap should also contain the cell center
        let center = cell.id().to_point();
        assert!(
            cap.contains_point(center),
            "cell {ci}: cap_bound should contain cell center"
        );
    }
}

/// `rect_bound` is valid, contains all 4 vertices (as `LatLng`).
#[test]
fn test_cell_rect_bound() {
    let test_cells = [
        Cell::from_cell_id(CellId::from_face_pos_level(0, 0, 0)),
        Cell::from_cell_id(CellId::from_face_pos_level(2, 0, 0).children()[0].children()[1]),
        Cell::from_cell_id(
            CellId::from_face_pos_level(5, 0, 0).children()[3].children()[2].children()[1],
        ),
        Cell::from_point(Point::from_coords(0.7, -0.4, 0.6)),
    ];

    for (ci, cell) in test_cells.iter().enumerate() {
        let rect = cell.rect_bound();
        assert!(rect.is_valid(), "cell {ci}: rect_bound should be valid");
        assert!(
            !rect.is_empty(),
            "cell {ci}: rect_bound should not be empty"
        );

        for k in 0..4 {
            let v = cell.vertex(k);
            assert!(
                rect.contains_point(v),
                "cell {ci}: rect_bound should contain vertex {k}"
            );
        }
    }
}

/// Cell at level 5 contains its children, children don't contain parent.
#[test]
fn test_cell_containment_hierarchy() {
    // Build a level-5 cell
    let mut cid = CellId::from_face_pos_level(1, 0, 0);
    for _ in 0..5 {
        cid = cid.children()[1];
    }
    assert_eq!(cid.level(), 5);

    let parent_cell = Cell::from_cell_id(cid);
    let children = parent_cell
        .children()
        .expect("level-5 cell should have children");

    for (i, child) in children.iter().enumerate() {
        // Parent contains each child
        assert!(
            parent_cell.contains_cell(*child),
            "parent should contain child {i}"
        );

        // Child intersects parent
        assert!(
            child.intersects_cell(&parent_cell),
            "child {i} should intersect parent"
        );

        // Child does NOT contain parent
        assert!(
            !child.contains_cell(&parent_cell),
            "child {i} should not contain parent"
        );
    }

    // Children at the same level don't contain each other
    for i in 0..4 {
        for j in 0..4 {
            if i != j {
                assert!(
                    !children[i].contains_cell(children[j]),
                    "child {i} should not contain child {j}"
                );
            }
        }
    }

    // Each child contains its own center but not sibling centers
    for (i, child) in children.iter().enumerate() {
        let center_i = child.id().to_point();
        assert!(
            child.contains_point(&center_i),
            "child {i} should contain its own center"
        );
    }
}
