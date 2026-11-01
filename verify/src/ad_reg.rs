// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Börresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The region adaptors preserve the `Region` laws (spec 012 US2, FR-005; data-model §4.1;
//! research R0, R5). Each statement takes abstract members (`any_region`), assumes a law of every
//! member's answers (read through the methods the adaptor calls, so one value each), and asserts
//! the law of the union and of the intersection. Since every implementor's answers are among the
//! abstract region's, the law holds for an adaptor of any members that satisfy it.
//!
//! **Any number of members.** The proofs cover up to three members (two for the vertex law, whose
//! four vertices per member must fit the abstract region's ten table slots); every number follows
//! by the argument below, which does not need the folds to be associative (the rectangle folds are
//! not: over circular intervals `union` picks the shorter gap and `intersection` keeps the shorter
//! operand of a two-piece overlap, `ad_mean`'s P-ADBND-01). Each adaptor answer is a left fold over
//! its members in order.
//! - **P-ADREG-01, -02.** The cell and point tests are `any` (union) and `all` (intersection) of the
//!   members' answers to the same input, and each law is an implication between one member's
//!   answers to that input; it passes through `any` and `all` for every `n` at once.
//! - **P-ADREG-03.** For a point the adaptor contains, each step of the rectangle fold keeps an
//!   invariant, by induction over the members folded so far. The union's accumulator, from the
//!   empty rectangle, contains every member rectangle folded so far, since each step contains both
//!   its operands (spec 005's P-LSET-01); the member that contains the point has it in its
//!   rectangle (the member's law), so the result holds it. The intersection's accumulator, from the
//!   full rectangle, holds every point that all the members folded so far hold, since each step
//!   holds every common point (P-LSET-02); every member contains the point, so the result holds it.
//!
//! The proofs at small `n` check that the code is these folds, over abstract members.
//!
//! **Not proven:**
//! - **P-ADREG-04**, the bounding cap: the adaptors' cap bound is their rectangle's `cap_bound`,
//!   so the law follows from P-ADREG-03 and `Rect`'s own bounding-cap law, spec 010's P-LREG-03,
//!   which is not proven (it needs `atan2`'s and `sin_cos`'s values; tested, and widened against
//!   U-016).
//! - **P-ADREG-05**, the covering laws: the covering is the cap's, `Cap`'s covering law (spec
//!   008's P-CPREG-04, `asin`'s value).

use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;
use s2rst::s2::{CellVertex, Region};

use crate::any_region::{members, reset};
use crate::laws;
use crate::value_domain::{any_cell, any_plain_triple};

macro_rules! p_adreg_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-ADREG-01 (law): containment implies intersection is preserved ",
            "(`laws::region::contains_implies_intersects`).\n\nExactly: for exactly ",
            stringify!($n), " abstract members that satisfy it for a valid cell, the union and ",
            "the intersection satisfy it.\n\nStubs: `any_uv_bound`."
        )]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        fn $name() {
            reset();
            let (c, _) = any_cell();
            for m in &members::<$n>() {
                kani::assume(!m.contains_cell(&c) || m.intersects_cell(&c));
            }
            laws::region::contains_implies_intersects(&RegionUnion::from_regions(members::<$n>()), &c);
            laws::region::contains_implies_intersects(
                &RegionIntersection::from_regions(members::<$n>()),
                &c,
            );
        }
    )* };
}

p_adreg_01!(p_adreg_01__n1 = 1, p_adreg_01__n2 = 2, p_adreg_01__n3 = 3,);

macro_rules! p_adreg_02 {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-ADREG-02 (law): the vertex law is preserved (`laws::region::no_intersect_no_vertex`).",
            "\n\nExactly: for exactly ", stringify!($n), " abstract members that satisfy it for ",
            "the abstract cell (any four box points as vertices), the union and the intersection ",
            "satisfy it.\n\nStubs: `fixed_vertex` (the abstract cell), `any_uv_bound`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        #[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
        fn $name() {
            reset();
            crate::stubs::draw_abstract_cell();
            let (c, _) = any_cell();
            for m in &members::<$n>() {
                if !m.intersects_cell(&c) {
                    for k in CellVertex::ALL {
                        kani::assume(!m.contains_point(&c.vertex(k)));
                    }
                }
            }
            laws::region::no_intersect_no_vertex(&RegionUnion::from_regions(members::<$n>()), &c);
            laws::region::no_intersect_no_vertex(
                &RegionIntersection::from_regions(members::<$n>()),
                &c,
            );
        }
    )* };
}

p_adreg_02!(
    p_adreg_02__n1 = 1,
    // R0b: 235.2 s (union), 264.6 s (intersection); three members do not fit the tables.
    #[cfg(feature = "slow")]
    p_adreg_02__n2 = 2,
);

macro_rules! p_adreg_03 {
    ($($(#[$attr:meta])* $name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-ADREG-03 (law): the bounding-rectangle law is preserved ",
            "(`laws::region::rect_bound_contains_points`).\n\nExactly: for exactly ",
            stringify!($n), " abstract members that satisfy it for a point without NaN, the ",
            "union and the intersection satisfy it.\n\nUnder contract: `pure_from_point` ",
            "(P-LLFP-02)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
        fn $name() {
            reset();
            crate::stubs::reset();
            let p = any_plain_triple();
            for m in &members::<$n>() {
                kani::assume(!m.contains_point(&p) || m.rect_bound().contains_point(p));
            }
            laws::region::rect_bound_contains_points(&RegionUnion::from_regions(members::<$n>()), &p);
            laws::region::rect_bound_contains_points(
                &RegionIntersection::from_regions(members::<$n>()),
                &p,
            );
        }
    )* };
}

p_adreg_03!(
    p_adreg_03__n1 = 1,
    // R0: 118.3 s and 80.2 s (two members), 441.6 s and 236.1 s (three): weekly.
    #[cfg(feature = "slow")]
    p_adreg_03__n2 = 2,
    #[cfg(feature = "slow")]
    p_adreg_03__n3 = 3,
);
