// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The lemmas of spec 010 (data-model §3): `L-LL-01`, which `stubs::any_cell_bound` rests on,
//! and `L-LL-02`, which P-LLDST-01's composition rests on.

use crate::rect_spec;
use crate::value_domain::any_cell;

/// L-LL-01: `Cell::rect_bound` is a valid, non-empty rectangle for every valid cell.
///
/// Exactly: `rect_spec::valid(r) && !rect_spec::empty(r)` for `r = c.rect_bound()` and every
/// valid cell `c`.
///
/// Under contract: `atan2_contract`. Stubs: `any_uv_bound` (gap 2's abstract u-v bound: every
/// real cell's u-v bound is one such rectangle, so the lemma covers every cell).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn l_ll_01() {
    let (c, d) = any_cell();
    let r = c.rect_bound();
    assert!(rect_spec::valid(r));
    assert!(!rect_spec::empty(r));
    kani::cover!(d.level > 0 && !r.is_full(), "a proper rectangle");
}

/// L-LL-02: Rust's `x.min(1.0)` is upstream's `std::min(1.0, x)` (`x < 1.0 ? x : 1.0`), bit for
/// bit, for every `f64`: NaN gives 1 in both, and −0 stays −0.
#[kani::proof]
fn l_ll_02() {
    let x: f64 = kani::any();
    let upstream = if x < 1.0 { x } else { 1.0 };
    assert!(x.min(1.0).to_bits() == upstream.to_bits());
}
