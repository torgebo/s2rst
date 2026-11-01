// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The exhaustive check of the u-v projection at every boundary value (spec 004 research R8).
//!
//! A cell's stored u-v bounds are `st_to_uv(ij_to_st_min(k))` for the edges `k` of its
//! `(i, j)` range, and `children()` computes a child's inner bound from the parent's centre,
//! `st_to_uv(si_ti_to_st(2k))`. Kani cannot prove properties of these floats beyond low levels:
//! they square a symbolic `s` (research R0). But the boundary values are finite: every edge of
//! every cell is one of the `2^30 + 1` values `k ∈ [0, 2^30]`. This test checks all of them,
//! so for the float step it is complete, not a sample:
//!
//! - the library's projection is bit for bit upstream's formula, as `cell_spec` restates it;
//! - the centre scaling (`si_ti_to_st(2k)`) and the edge scaling (`ij_to_st_min(k)`) agree, so
//!   a child's bounds computed from its parent's centre are those computed from its own range;
//! - the quadrant premise (spec 013, lemma L-CR-02): the edge value is in [−1, 1], increasing in
//!   `k`, negative below `2²⁹`, `+0` at it and positive above. A cell below level 0 has
//!   `size ≤ 2²⁹` and a low edge that is a multiple of `size`, so its two edges lie on one side of
//!   `2²⁹` (or on it): its u-v interval does not cross 0, and no bound is `−0`.
//!
//! The link from a cell's identifier to the `k` of its edges is P-CID-01's integer fact above
//! the levels the bound harnesses reach; `TRUSTED_BASE.md` says so.

use s2rst::s2::coords::{ij_to_st_min, si_ti_to_st, st_to_uv};
use s2rst_verify::cell_spec;

#[test]
fn projection_at_every_boundary_value() {
    let (mut model_diffs, mut scale_diffs, mut quadrant_misses) = (0u64, 0u64, 0u64);
    let mut prev = f64::NEG_INFINITY;
    for k in 0..=(1u32 << 30) {
        let s = ij_to_st_min(k as i32);
        let lib = st_to_uv(s);
        let half = 1u32 << 29;
        let sign_ok = match k.cmp(&half) {
            std::cmp::Ordering::Less => lib < 0.0,
            std::cmp::Ordering::Equal => lib.to_bits() == 0,
            std::cmp::Ordering::Greater => lib > 0.0,
        };
        if !sign_ok || lib.abs() > 1.0 || lib < prev {
            quadrant_misses += 1;
        }
        prev = lib;
        let model = cell_spec::st_to_uv(cell_spec::ij_to_st_min(k));
        if lib.to_bits() != model.to_bits() {
            model_diffs += 1;
        }
        if si_ti_to_st(2 * k).to_bits() != s.to_bits() {
            scale_diffs += 1;
        }
    }
    assert_eq!(quadrant_misses, 0, "edge values off the quadrant premise");
    assert_eq!(
        (model_diffs, scale_diffs),
        (0, 0),
        "boundary values where the library differs from upstream's projection, and where the \
         centre and edge scalings differ"
    );
}
