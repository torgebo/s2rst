// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a cell (spec 004 data-model §1.1): a cell is its identifier's
//! decomposition in [`crate::spec`], with facts derived from it.
//!
//! Written from upstream `s2cell.h` and `s2cell.cc` (the `S2Cell(S2CellId)` constructor,
//! lines 64–71, and `Subdivide`, lines 85–124) and `s2coords.h` (`STtoUV` for the default
//! quadratic projection, lines 324–327, and `IJtoSTMin`), at `v0.14.0-40`. It never calls
//! `s2rst`'s cell or coordinate code (FR-007). The u-v bounds are defined by upstream's
//! floating-point expression, so they are restated here in upstream's operation order, and
//! "bit for bit equal" is a parity statement.

use s2rst::s2::Cell;

use crate::spec::{self, Decomposed};

/// The facts a cell carries, as the model gives them for an identifier's decomposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellModel {
    /// Cube face, `0..=5`.
    pub face: u8,
    /// Subdivision level, `0..=30`.
    pub level: u8,
    /// The Hilbert-curve orientation reached after the cell's own digits, `0..=3`.
    pub orientation: u8,
    /// The lowest leaf `i` coordinate of the cell: a multiple of `size`.
    pub i_lo: u32,
    /// The lowest leaf `j` coordinate of the cell: a multiple of `size`.
    pub j_lo: u32,
    /// The cell's edge length in leaf units, `2^(30 − level)`.
    pub size: u32,
}

/// The model of the cell of a well-formed decomposition: upstream's constructor takes the
/// face and orientation from `ToFaceIJOrientation`, the level from the id, and the bounds from
/// the `(i, j)` range at that level (`IJLevelToBoundUV`).
#[must_use]
pub fn model(d: Decomposed) -> CellModel {
    let (face, i, j, orientation) = spec::face_ij_orientation(d);
    let size = spec::size_ij(d.level);
    CellModel {
        face,
        level: d.level,
        orientation,
        i_lo: i & !(size - 1),
        j_lo: j & !(size - 1),
        size,
    }
}

/// Child `k` of a non-leaf cell (`level ≤ 29`, `k < 4`), in curve order: 001's `spec::child`.
#[must_use]
pub fn child_model(d: Decomposed, k: u8) -> Decomposed {
    spec::child(d, k)
}

/// The orientation of child `k` of a cell with orientation `o`: upstream `Subdivide` XORs
/// `kPosToOrientation[k]` into the parent's.
#[must_use]
pub fn child_orientation(o: u8, k: u8) -> u8 {
    o ^ spec::POS_TO_ORIENTATION[usize::from(k)]
}

/// Upstream `S2::STtoUV` for the quadratic projection (`s2coords.h:324–327`), in its operation
/// order: `s ≥ 0.5 ? (1/3.)·(4·s·s − 1) : (1/3.)·(1 − 4·(1−s)·(1−s))`.
#[must_use]
pub fn st_to_uv(s: f64) -> f64 {
    if s >= 0.5 {
        (1.0 / 3.0) * (4.0 * s * s - 1.0)
    } else {
        (1.0 / 3.0) * (1.0 - 4.0 * (1.0 - s) * (1.0 - s))
    }
}

/// Upstream `S2::IJtoSTMin`: `(1.0 / kLimitIJ) · k`, exact for `0 ≤ k ≤ 2^30`.
#[must_use]
pub fn ij_to_st_min(k: u32) -> f64 {
    (1.0 / f64::from(1u32 << 30)) * f64::from(k)
}

/// The u-v bounds of a cell: `[[u_lo, u_hi], [v_lo, v_hi]]`, each the projection of the
/// matching edge of its `(i, j)` range (upstream `IJLevelToBoundUV`).
#[must_use]
pub fn uv_bounds(m: CellModel) -> [[f64; 2]; 2] {
    [
        [
            st_to_uv(ij_to_st_min(m.i_lo)),
            st_to_uv(ij_to_st_min(m.i_lo + m.size)),
        ],
        [
            st_to_uv(ij_to_st_min(m.j_lo)),
            st_to_uv(ij_to_st_min(m.j_lo + m.size)),
        ],
    ]
}

/// Whether cell `a` contains cell `b`: 001's identifier containment.
#[must_use]
pub fn contains(a: Decomposed, b: Decomposed) -> bool {
    spec::contains(a, b)
}

/// Whether cells `a` and `b` intersect: two cells are nested or disjoint (001's P-RANGE-05),
/// so they intersect exactly when one contains the other.
#[must_use]
pub fn intersects(a: Decomposed, b: Decomposed) -> bool {
    contains(a, b) || contains(b, a)
}

/// Cell consistency (FR-005): `c`'s identifier is `d`'s, its face, level and orientation are
/// the model's, and its u-v bounds are the model's bit for bit.
#[must_use]
pub fn consistent(c: &Cell, d: Decomposed) -> bool {
    let m = model(d);
    let b = c.bound_uv();
    let want = uv_bounds(m);
    c.id().0 == spec::compose(d)
        && c.face().as_u8() == m.face
        && c.level().as_u8() == m.level
        && c.orientation() == m.orientation
        && b.x.lo.to_bits() == want[0][0].to_bits()
        && b.x.hi.to_bits() == want[0][1].to_bits()
        && b.y.lo.to_bits() == want[1][0].to_bits()
        && b.y.hi.to_bits() == want[1][1].to_bits()
}

#[cfg(test)]
#[path = "cell_spec_tests.rs"]
mod cell_spec_tests;
