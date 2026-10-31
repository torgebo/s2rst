// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Börresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The region adaptors preserve the `Region` laws (spec 012 US2, FR-005; data-model §4.1;
//! research R0, R5). Each statement takes abstract members (`any_region`), assumes a law of every
//! member's answers (read through the methods the adaptor calls, so one value each), and asserts
//! the law of the union and of the intersection. Since every implementor's answers are among the
//! abstract region's, the law holds for an adaptor of any members that satisfy it.
//!
//! **Any number of members.** Each adaptor answer is a fold over its members (`any`, `all`,
//! `Rect::union` from the empty rectangle, `Rect::intersection` from the full one), each fold
//! associative, and each law a statement about one member's answers. The proofs cover up to three
//! members (two for the vertex law, whose four vertices per member must fit the abstract region's
//! ten table slots); a law for `n + 1` members follows from the law for an adaptor of `n` (by
//! induction) combined with one more member, which the two-member proofs cover, with spec 005's
//! P-LSET-01 (`union` contains both) and P-LSET-02 (`intersection` holds every common point) for
//! the rectangles.
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
