// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash freedom of `CellUnion`'s floating-point methods (spec 011 US1, FR-003; data-model §5.2;
//! research R0), 002's rows 12, 26, 27, 35, 36, 40, over valid unions of exactly `n` cells
//! (`__n<k>`: a symbolic length costs two to three orders of magnitude, 002 research R0).
//!
//! Each cell function a method calls has its own crash proof, so the harnesses here replace it
//! by a stub that allows every value it returns, and prove the method's own code:
//! `Cell::rect_bound` (`cl_crash::c_fg_rect_bound`), `Cell::cap_bound` (`cl_crash::c_fg_cap_bound`),
//! `CellId::to_point` (`cl_crash::c_fg_center`), `Cell::approx_area` (`cl_crash::c_fg_approx_area`),
//! `CellId::from_point` (`cl_crash::c_fg_from_point`, with its documented panics). The other
//! rows inherit a tool limit: `exact_area` (row 28) `Cell::exact_area`'s out-of-memory (the
//! coverage review's xlarge tier), `cell_union_bound` (row 37) `Cap::cell_union_bound`'s timeout
//! (spec 008), and `expand_by_radius` (row 25) `expand_at_level`'s `all_neighbors` (001 R12; its
//! debug panic on a NaN radius, reached first, is `uf_exp::n_expand_by_radius_nan`).

use s2rst::s2::{CellUnion, Region};

use crate::union_domain::any_valid_union;
use crate::value_domain::any_triple;

macro_rules! c_contains_point {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::contains_point, C-Region::contains_point: complete for every triple ",
            "and every valid union of exactly ", stringify!($n), " cells, given ",
            "`CellId::from_point`'s leaf.\n\nUnder contract: `pure_leaf` (L-PR-01)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::CellId::from_point, crate::stubs::pure_leaf)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let p = any_triple();
            let x = u.contains_point(p);
            let y = Region::contains_point(&u, &p);
            kani::cover!(x || $n == 0, "a contained point");
            kani::cover!(!y, "a point outside");
        }
    )* };
}

c_contains_point!(
    c_contains_point__n0 = 0,
    c_contains_point__n1 = 1,
    c_contains_point__n2 = 2,
    c_contains_point__n3 = 3,
);

macro_rules! c_average_based_area {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::average_based_area: completes for every valid union of exactly ",
            stringify!($n), " cells (the leaf count cannot overflow for a valid union, 002's ",
            "P-UMEAS-01)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let a = CellUnion::from_verbatim(v).average_based_area();
            kani::cover!(a > 0.0, "a positive area");
        }
    )* };
}

c_average_based_area!(
    c_average_based_area__n1 = 1,
    c_average_based_area__n2 = 2,
    c_average_based_area__n3 = 3,
);

macro_rules! c_approx_area {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::approx_area: completes for every valid union of exactly ",
            stringify!($n), " cells, given each cell's area (a sum of finite values cannot ",
            "panic; `Cell::approx_area`'s own proof is `cl_crash::c_fg_approx_area`).\n\n",
            "Stubs: `pure_cell_area`, `any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::approx_area, crate::stubs::pure_cell_area)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let a = CellUnion::from_verbatim(v).approx_area();
            kani::cover!(a > 0.0, "a positive area");
        }
    )* };
}

c_approx_area!(
    c_approx_area__n1 = 1,
    c_approx_area__n2 = 2,
    #[cfg(feature = "slow")]
    c_approx_area__n3 = 3,
);

macro_rules! c_cap_bound {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-Region::cap_bound for `CellUnion`: completes for every valid union of exactly ",
            stringify!($n), " cells, given each cell's centre and cap bound.\n\n",
            "Stubs: `pure_cell_cap`, `any_center`, `any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::cap_bound, crate::stubs::pure_cell_cap)]
        #[kani::stub(s2rst::s2::CellId::to_point, crate::stubs::any_center)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let c = CellUnion::from_verbatim(v).cap_bound();
            kani::cover!(!c.is_empty(), "a non-empty cap");
        }
    )* };
}

c_cap_bound!(
    // Weekly: 165.5 s of solver time, but about nine minutes of wall time per change (2026-10-29),
    // which held the per-change leg over SC-006's 10 minutes.
    #[cfg(feature = "slow")]
    c_cap_bound__n1 = 1,
    // R0b: 462.7 s.
    #[cfg(feature = "slow")]
    c_cap_bound__n2 = 2,
);

/// C-Region::rect_bound for `CellUnion`: completes for every valid one-cell union, with the real
/// `Cell::rect_bound`.
///
/// Under contract: `atan2_contract`. Stubs: `any_uv_bound` (R0b: 93.1 s).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_rect_bound__n1() {
    let (v, _) = any_valid_union::<1>();
    let b = CellUnion::from_verbatim(v).rect_bound();
    kani::cover!(!b.is_empty(), "a non-empty bound");
}

macro_rules! c_rect_bound {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "C-Region::rect_bound for `CellUnion`: completes for every valid union of exactly ",
            stringify!($n), " cells, given each cell's bound (L-LL-01: a valid, non-empty ",
            "rectangle).\n\nStubs: `any_cell_bound`, `any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::Cell::rect_bound, crate::stubs::any_cell_bound)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let b = CellUnion::from_verbatim(v).rect_bound();
            kani::cover!(!b.is_empty(), "a non-empty bound");
        }
    )* };
}

c_rect_bound!(
    // Weekly from 2026-10-29, as `p_ubnd_01__n2`: the per-change leg's budget.
    #[cfg(feature = "slow")]
    c_rect_bound__n2 = 2,
    #[cfg(feature = "slow")]
    c_rect_bound__n3 = 3,
);
