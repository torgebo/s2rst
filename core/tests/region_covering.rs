// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Integration tests for `RegionCoverer` and `CellUnion`, ported from C++ `s2region_coverer_test.cc`
//! and `s2cell_union_test.cc`.

use s2rst::s1::Angle;
use s2rst::s2::coords::Level;
use s2rst::s2::region_coverer::RegionCoverer;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, LatLng, Point};

// ---------------------------------------------------------------------------
// RegionCoverer tests
// ---------------------------------------------------------------------------

#[test]
fn test_covering_small_cap() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let cap = Cap::from_center_angle(center, Angle::from_degrees(5.0));
    let coverer = RegionCoverer::new().max_cells(8);
    let covering = coverer.covering(&cap);

    assert!(covering.num_cells() > 0, "covering should be non-empty");
    assert!(
        covering.num_cells() <= 8,
        "covering should respect max_cells"
    );
    assert!(
        covering.contains_point(center),
        "covering should contain cap center"
    );
}

#[test]
fn test_covering_level_constraints() {
    let center = Point::from_coords(0.0, 1.0, 0.0);
    let cap = Cap::from_center_angle(center, Angle::from_degrees(10.0));
    let coverer = RegionCoverer::new()
        .min_level(5)
        .max_level(10)
        .max_cells(100);
    let covering = coverer.covering(&cap);

    for id in covering.cell_ids() {
        let level = id.level();
        assert!(
            (5..=10).contains(&level.as_u8()),
            "cell level {level} outside [5, 10]"
        );
    }
}

#[test]
fn test_covering_max_cells() {
    let center = Point::from_coords(0.0, 0.0, 1.0);
    let cap = Cap::from_center_angle(center, Angle::from_degrees(20.0));
    let coverer = RegionCoverer::new().max_cells(8);
    let covering = coverer.covering(&cap);
    assert!(
        covering.num_cells() <= 8,
        "covering exceeds max_cells limit"
    );
}

#[test]
fn test_covering_deterministic() {
    let center = Point::from_coords(1.0, 1.0, 1.0).normalize();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(3.0));
    let coverer = RegionCoverer::new().max_cells(6);

    let c1 = coverer.covering(&cap);
    let c2 = coverer.covering(&cap);

    assert_eq!(
        c1.num_cells(),
        c2.num_cells(),
        "coverings should have same size"
    );
    for (a, b) in c1.cell_ids().iter().zip(c2.cell_ids().iter()) {
        assert_eq!(a.0, b.0, "coverings should be identical");
    }
}

#[test]
fn test_covering_contains_region_center() {
    let center = LatLng::from_degrees(37.7749, -122.4194).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(0.1));
    let coverer = RegionCoverer::new().max_cells(4);
    let covering = coverer.covering(&cap);
    assert!(
        covering.contains_point(center),
        "covering should contain the cap center"
    );
}

#[test]
fn test_interior_covering_is_contained() {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let cap = Cap::from_center_angle(center, Angle::from_degrees(15.0));
    let coverer = RegionCoverer::new().max_cells(20);

    let interior = coverer.interior_covering(&cap);
    let outer = coverer.covering(&cap);

    // Every cell in the interior covering should be fully inside the outer covering.
    for id in interior.cell_ids() {
        assert!(
            outer.contains_cell_id(*id),
            "interior cell {id:?} should be covered by outer covering"
        );
    }
}

// ---------------------------------------------------------------------------
// CellUnion tests
// ---------------------------------------------------------------------------

#[test]
fn test_cell_union_normalize_siblings() {
    // Four siblings should merge into their parent.
    let parent = CellId::from_face_pos_level(0, 0, 5);
    let children = parent.children();
    let mut cu = CellUnion::from_cell_ids(children.to_vec());
    cu.normalize();

    assert!(cu.is_normalized());
    assert_eq!(
        cu.num_cells(),
        1,
        "four siblings should merge into one parent"
    );
    assert_eq!(cu.cell_ids()[0], parent, "merged cell should be the parent");
}

#[test]
fn test_cell_union_contains_child() {
    let parent = CellId::from_face_pos_level(0, 0, 5);
    let cu = CellUnion::from_cell_ids(vec![parent]);
    let child = parent.children()[2];
    assert!(
        cu.contains_cell_id(child),
        "parent cell union should contain child"
    );
}

#[test]
fn test_cell_union_intersects() {
    let id1 = CellId::from_face_pos_level(0, 0, 5);
    let id2 = id1.children()[0]; // child of id1

    let cu1 = CellUnion::from_cell_ids(vec![id1]);
    let cu2 = CellUnion::from_cell_ids(vec![id2]);
    assert!(
        cu1.intersects_union(&cu2),
        "parent and child should intersect"
    );
}

#[test]
fn test_cell_union_disjoint() {
    let id1 = CellId::from_face_pos_level(0, 0, 10);
    let id2 = CellId::from_face_pos_level(3, 0, 10);

    let cu1 = CellUnion::from_cell_ids(vec![id1]);
    let cu2 = CellUnion::from_cell_ids(vec![id2]);

    assert!(
        !cu1.contains_union(&cu2),
        "opposite face cells should not contain"
    );
    assert!(
        !cu1.intersects_union(&cu2),
        "opposite face cells should not intersect"
    );
}

#[test]
fn test_cell_union_leaf_cells_covered() {
    // A single face cell (level 0) covers a known number of leaf cells.
    let face_id = CellId::from_face(0);
    let cu = CellUnion::from_cell_ids(vec![face_id]);
    let leaves = cu.leaf_cells_covered();
    // A level-0 cell contains 4^30 leaf cells.
    let expected = 4_i64.pow(30);
    assert_eq!(leaves, expected, "face cell should cover 4^30 leaves");
}

#[test]
fn test_cell_union_denormalize() {
    let id = CellId::from_face_pos_level(0, 0, 4);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let denorm = cu.denormalize(Level::new(6), 2);

    // All resulting cells should be at level 6 (min_level=6, level_mod=2).
    for cell_id in denorm.cell_ids() {
        let level = cell_id.level();
        assert!(
            level >= 6,
            "denormalized cell level {level} should be >= min_level 6"
        );
        assert!(
            (level.as_u8() - 6) % 2 == 0,
            "denormalized cell level {level} should satisfy level_mod=2"
        );
    }
}

#[test]
fn test_cell_union_contains_point() {
    let id = CellId::from_face_pos_level(0, 0, 10);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let center = id.to_point();
    assert!(
        cu.contains_point(center),
        "cell union should contain cell center"
    );
}

#[test]
fn test_cell_union_empty() {
    let cu = CellUnion::new();
    assert_eq!(cu.num_cells(), 0);
    assert!(cu.is_normalized());
    assert_eq!(cu.leaf_cells_covered(), 0);
}

#[test]
fn test_covering_cell_as_region() {
    // Covering a single cell should return that cell (or its parent).
    let cell = Cell::from_cell_id(CellId::from_face_pos_level(2, 0, 10));
    let coverer = RegionCoverer::new().max_cells(1);
    let covering = coverer.covering(&cell);

    assert!(covering.num_cells() >= 1);
    // The covering should at least contain the cell's center.
    assert!(covering.contains_point(cell.id().to_point()));
}
