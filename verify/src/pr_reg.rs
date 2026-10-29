// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The generic `Region` laws for `PointRegion` (spec 011 US2, FR-011; data-model §4.1; research
//! R0, R6). The laws hold over unit points, upstream's precondition; the harnesses take box
//! points, which hold every unit point, or every triple without NaN.
//!
//! **P-PRREG-02**, the covering covers every contained cell (`laws::region::covering_covers`),
//! is proven by composition: the region contains no cell (P-PRCEL-01), so the law's premise
//! never holds.
//!
//! **P-PRREG-03**, the vertex law (`laws::region::no_intersect_no_vertex`), reduces to one fact:
//! a cell contains each of its own vertices. If the region of `p` does not intersect a cell,
//! then `c.contains_point(p)` is false (P-PRCEL-02); were `p` equal to a vertex `v` of `c` as a
//! number, `c.contains_point(v)` would be false too, against the fact (`contains_point` reads
//! its argument's values only, and `±0` give the same u-v coordinates). The fact is stated per
//! face, weekly, but is **not proven**: each face's harness times out at 60 minutes (2026-10-29),
//! and unsplit it timed out at 12 minutes with CBMC's `sqrt` and with `sqrt_between` (R0, R0c).
//! The harnesses stay in `known-timeouts.txt`; the survey's `PV` section (80,000 vertices, C++
//! and library alike), the `POINTREGION_CELL` vertex rows and the twin check the fact.
//!
//! The rectangle and covering laws are stated for the region's own point (`q = p` bit for bit).
//! A point equal to it as a number with another zero's sign has the same `LatLng` and the same
//! leaf (both add `+ 0.0` or divide by the axis coordinate, so a zero's sign does not reach the
//! result), which the pure stubs, keyed by bits, do not know; the twins check those points.

use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{Face, Region};

use crate::laws;
use crate::value_domain::{any_box_point, any_cell, any_cell_vertex, any_plain_triple};

/// P-PRREG-01 (law): containing a cell implies intersecting it
/// (`laws::region::contains_implies_intersects`).
///
/// Exactly: for every box point and every valid cell, `contains_cell(c)` implies
/// `intersects_cell(c)` (trivially: the region contains no cell).
///
/// Stubs: `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn p_prreg_01() {
    let r = PointRegion::new(any_box_point());
    let (c, _) = any_cell();
    laws::region::contains_implies_intersects(&r, &c);
}

macro_rules! p_prreg_03 {
    ($($name:ident = $face:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-PRREG-03 (law), face ", stringify!($face), ": a cell contains each of its own ",
            "vertices, so the region of a vertex intersects the cell (the fact the vertex law ",
            "reduces to, module documentation).\n\n",
            "Exactly: for every valid cell on face ", stringify!($face), " and every vertex `k`, ",
            "`PointRegion::new(c.vertex(k)).intersects_cell(&c)`.\n\n",
            "Under contract: `sqrt_between` (the vertex's normalisation). Stubs: `any_uv_bound`. ",
            "Weekly."
        )]
        #[cfg(feature = "slow")]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
        #[kani::stub(f64::sqrt, crate::stubs::sqrt_between)]
        fn $name() {
            let (c, _) = any_cell();
            kani::assume(c.face() == Face::ALL[$face]);
            let v = c.vertex(any_cell_vertex());
            assert!(PointRegion::new(v).intersects_cell(&c));
        }
    )* };
}

p_prreg_03!(
    p_prreg_03__f0 = 0,
    p_prreg_03__f1 = 1,
    p_prreg_03__f2 = 2,
    p_prreg_03__f3 = 3,
    p_prreg_03__f4 = 4,
    p_prreg_03__f5 = 5,
);

/// P-PRREG-04 (law): the bounding cap contains every point the region contains
/// (`laws::region::cap_bound_contains_points`).
///
/// Exactly: for every two box points `p`, `q`, if `PointRegion::new(p)` contains `q`, its cap
/// bound contains `q` (their chord distance is zero).
#[kani::proof]
fn p_prreg_04() {
    let (p, q) = (any_box_point(), any_box_point());
    let r = PointRegion::new(p);
    laws::region::cap_bound_contains_points(&r, &q);
    laws::region::cap_bound_contains_points(&r, &p);
}

/// P-PRREG-05 (law): the bounding rectangle contains the region's point
/// (`laws::region::rect_bound_contains_points`, spec 011 FR-010).
///
/// Exactly: for every triple without NaN, `rect_bound().contains_point(p)`.
///
/// Under contract: `pure_from_point` (P-LLFP-02; one value per point, so the bound's `LatLng` is
/// the test's).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
fn p_prreg_05() {
    crate::stubs::reset();
    let p = any_plain_triple();
    let r = PointRegion::new(p);
    laws::region::rect_bound_contains_points(&r, &p);
    kani::cover!(r.contains_point(&p), "the point itself");
}

/// P-PRREG-06 (law): the covering contains the region's point: one of its at most four cells
/// contains the point's leaf (`laws::region::covering_contains_points`, spec 011 FR-010).
///
/// Exactly: for every box point, `covering_contains_points::<_, 4>(&r, &p)`.
///
/// Under contract: `pure_leaf` (L-PR-01), `asin_contract`. Weekly (R0b: 452.8 s).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, crate::stubs::pure_leaf)]
fn p_prreg_06() {
    crate::stubs::reset();
    let p = any_box_point();
    let r = PointRegion::new(p);
    laws::region::covering_contains_points::<_, 4>(&r, &p);
}
