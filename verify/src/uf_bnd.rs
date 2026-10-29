// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `CellUnion`'s bounds (spec 011 data-model §4.2; research R0).
//!
//! **P-UBND-02** (correct result), proven by composition for every length: the cap bound contains
//! each cell's cap bound. `cap_bound` starts from `Cap::from_point(centre)`, a non-empty cap
//! (radius zero), and folds `add_cap` over the cells' cap bounds. After `add_cap(b)` the cap
//! contains `b` and its radius did not shrink (spec 008's P-CPGRO-02, over `BoxCap²`); a cap that
//! contains `b` still contains it after any later `add_cap` (L-PR-02, below, over `BoxCap³`).
//! By induction on the cells, the result contains every cell's cap bound. Every cap here is a
//! `BoxCap`: the centre is a normalised point and each cell's bound a box cap
//! (`libm_contracts::cell_cap_bound_satisfies_its_contract`). `p_ubnd_02__n1` checks the
//! statement directly at one cell, weekly (R0c: 357.4 s; two cells time out at 12 minutes).
//!
//! **P-UBND-04** (correct result), proven by composition: the constructions are upstream's text,
//! pinned by `region_spec_tests`' drift guards (the centroid and its fallback, the folds, the
//! covering through the cap bound), and their values checked by the C++ vectors.

use s2rst::s2::{Cap, Cell, CellUnion, Rect, Region};

use crate::union_domain::any_valid_union;
use crate::value_domain::any_box_cap;

/// L-PR-02 (lemma): a non-empty cap that contains `b` still contains it after `add_cap(c)`: the
/// centre is kept and the radius does not shrink.
///
/// Exactly: for every `a`, `b`, `c` in `BoxCap` with `a` not empty and `a.contains(b)`,
/// `a.add_cap(c).contains(b)`.
///
/// Stubs: `pure_chord`, `pure_add` (as P-CPGRO-02: one chord distance and one sum per argument
/// pair).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
fn l_pr_02() {
    crate::stubs::reset();
    let (a, b, c) = (any_box_cap(), any_box_cap(), any_box_cap());
    kani::assume(!a.is_empty() && a.contains(b));
    assert!(a.add_cap(c).contains(b));
    kani::cover!(!b.is_empty() && !c.is_empty(), "two non-empty caps");
}

macro_rules! p_ubnd_01 {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UBND-01 (correct result): the rectangle bound contains each cell's rectangle ",
            "bound (upstream's `Union` of them).\n\n",
            "Exactly: for every valid union of exactly ", stringify!($n), " cells, ",
            "`rect_bound().contains(Cell::from_cell_id(id).rect_bound())` for each `id`.\n\n",
            "Stubs: `any_cell_bound` (one valid, non-empty rectangle per cell, L-LL-01), ",
            "`any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::rect_bound, crate::stubs::any_cell_bound)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let b: Rect = CellUnion::from_verbatim(v.clone()).rect_bound();
            for k in 0..$n {
                assert!(b.contains(Cell::from_cell_id(v[k]).rect_bound()));
            }
        }
    )* };
}

p_ubnd_01!(
    p_ubnd_01__n1 = 1,
    // Weekly from 2026-10-29: 91.1 s of solver time, but its wall time held the per-change leg
    // over SC-006's 10 minutes.
    #[cfg(feature = "slow")]
    p_ubnd_01__n2 = 2,
    // R0: 333.7 s.
    #[cfg(feature = "slow")]
    p_ubnd_01__n3 = 3,
);

/// P-UBND-02 (correct result), checked directly at one cell: the cap bound contains the cell's
/// cap bound (the general statement is by composition, module documentation).
///
/// Stubs: `pure_cell_cap`, `any_center`, `pure_chord`, `pure_add`, `any_uv_bound`. Weekly (R0c:
/// 357.4 s).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::cap_bound, crate::stubs::pure_cell_cap)]
#[kani::stub(s2rst::s2::CellId::to_point, crate::stubs::any_center)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
fn p_ubnd_02__n1() {
    crate::stubs::reset();
    let (v, _) = any_valid_union::<1>();
    let b: Cap = CellUnion::from_verbatim(v.clone()).cap_bound();
    assert!(b.contains(Cell::from_cell_id(v[0]).cap_bound()));
}

/// P-UBND-03 (correct result): the empty union's bounds are the empty cap and the empty
/// rectangle, as upstream's `S2Cap::Empty()` and `S2LatLngRect::Empty()`.
///
/// Exactly: `CellUnion::new()`'s `cap_bound()` is empty, its `rect_bound()` is empty.
#[kani::proof]
fn p_ubnd_03() {
    let u = CellUnion::new();
    assert!(u.cap_bound().is_empty());
    assert!(u.rect_bound().is_empty());
}
