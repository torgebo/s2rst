// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the cell description (spec 004 T007), from upstream's
//! `s2cell_test.cc` and `s2coords.h`.

use super::*;
use crate::spec::{Decomposed, INVERT, SWAP, compose};
use s2rst::s2::{Cell, CellId};

fn d(face: u8, level: u8, path: u64) -> Decomposed {
    Decomposed { face, level, path }
}

#[test]
fn projection_endpoints() {
    assert_eq!(st_to_uv(0.0), -1.0);
    assert_eq!(st_to_uv(0.5), 0.0);
    assert_eq!(st_to_uv(1.0), 1.0);
    assert_eq!(ij_to_st_min(0), 0.0);
    assert_eq!(ij_to_st_min(1 << 29), 0.5);
    assert_eq!(ij_to_st_min(1 << 30), 1.0);
}

#[test]
fn face_cells_cover_the_whole_face() {
    for face in 0..6 {
        let m = model(d(face, 0, 0));
        assert_eq!(
            (m.face, m.level, m.i_lo, m.j_lo, m.size),
            (face, 0, 0, 0, 1 << 30)
        );
        // Upstream: a face's orientation is `face & kSwapMask`.
        assert_eq!(m.orientation, face & SWAP);
        assert_eq!(uv_bounds(m), [[-1.0, 1.0], [-1.0, 1.0]]);
    }
}

#[test]
fn level_one_cells_are_the_four_quadrants() {
    // Face 0 has orientation 0: positions 0..3 are (i, j) = (0,0), (0,1), (1,1), (1,0).
    let quadrant = |i: u32, j: u32| [[i as f64 - 1.0, i as f64], [j as f64 - 1.0, j as f64]];
    let want = [
        quadrant(0, 0),
        quadrant(0, 1),
        quadrant(1, 1),
        quadrant(1, 0),
    ];
    for (k, w) in want.iter().enumerate() {
        let m = model(d(0, 1, k as u64));
        assert_eq!(m.size, 1 << 29);
        assert_eq!(uv_bounds(m), *w, "position {k}");
    }
}

#[test]
fn child_orientations_follow_upstream_table() {
    // kPosToOrientation = {kSwapMask, 0, 0, kInvertMask + kSwapMask}.
    assert_eq!(child_orientation(0, 0), SWAP);
    assert_eq!(child_orientation(0, 1), 0);
    assert_eq!(child_orientation(0, 2), 0);
    assert_eq!(child_orientation(0, 3), INVERT | SWAP);
    assert_eq!(child_orientation(SWAP, 0), 0);
    // A child's model orientation is the parent's XOR the table entry.
    let parent = d(3, 4, 0b10_01_11_00);
    for k in 0..4 {
        let o = model(child_model(parent, k)).orientation;
        assert_eq!(o, child_orientation(model(parent).orientation, k));
    }
}

#[test]
fn containment_is_identifier_containment() {
    let a = d(2, 3, 0b01_10_11);
    let child = child_model(a, 2);
    let other = d(2, 3, 0b01_10_10);
    assert!(contains(a, a) && contains(a, child) && !contains(child, a));
    assert!(intersects(a, child) && intersects(child, a));
    assert!(!contains(a, other) && !intersects(a, other));
}

#[test]
fn the_library_builds_consistent_cells() {
    // A sanity check of the description, not a proof: the Kani harnesses prove it.
    for dd in [
        d(0, 0, 0),
        d(5, 1, 3),
        d(1, 10, 0x2_aaaa),
        d(4, 30, (1 << 60) - 1),
    ] {
        let cell = Cell::from_cell_id(CellId(compose(dd)));
        assert!(consistent(&cell, dd), "{dd:?}");
    }
}
