// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The generic `Region` laws for `s2::Rect` (spec 010 US3, FR-001a, FR-010; data-model §4.2;
//! research R6).
//!
//! `Region for Rect` tests a cell through `cell.rect_bound()` (upstream's `Contains(S2Cell)` and
//! `MayIntersect`) and a point through `contains_lat_lng(LatLng::from_point(p))`. A symbolic
//! real cell runs the checker out of memory (spec 008 R0), so the cell's bound is a stub: a pure
//! function of the cell, with the facts the law needs, each proven (L-LL-01) or a tested
//! contract (`cell_rect_bound_contract`).
//!
//! Two laws are not instantiated here:
//! - **P-LREG-03**, the bounding cap contains every contained point: the point's `LatLng`
//!   (`atan2`) and the cap's chord distances (`sin_cos`) must agree in value, which the
//!   contracts do not fix. `Rect::cap_bound`'s widening (FR-019) is checked by
//!   `rect_tests::cap_bound_contains_*`, the quickcheck twin `region_rect_cap_contains_boundary`
//!   and the `RECT_CAPBOUND` rows.
//! - **P-LREG-04**, the covering covers every contained cell: `cell_union_bound` is the bounding
//!   cap's, whose level depends on `asin`'s value, as 008's P-CPREG-04.

use s2rst::s2::LatLng;

use crate::laws;
use crate::rect_spec;
use crate::value_domain::{any_cell, any_rect, any_triple};

/// P-LREG-01 (law): containing a cell implies intersecting it
/// (`laws::region::contains_implies_intersects`).
///
/// Exactly: for every valid rectangle and every valid cell, `contains_cell(c)` implies
/// `intersects_cell(c)`.
///
/// Stubs: `any_cell_bound` (L-LL-01, proven under `atan2_contract`), `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::rect_bound, crate::stubs::any_cell_bound)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn p_lreg_01() {
    crate::stubs::reset();
    let r = any_rect();
    let (c, _) = any_cell();
    laws::region::contains_implies_intersects(&r, &c);
    kani::cover!(s2rst::s2::Region::contains_cell(&r, &c), "a contained cell");
}

/// P-LREG-02 (law): a cell the rectangle does not intersect has no vertex the rectangle contains
/// (`laws::region::no_intersect_no_vertex`).
///
/// Exactly: for every valid rectangle and the abstract cell (any four box points as vertices),
/// if `intersects_cell(c)` is false then `contains_point(c.vertex(k))` is false for each `k`.
///
/// Under contract: `cell_rect_bound_contract` (the cell's bound contains its vertices' `LatLng`s,
/// tested on the real functions). Stubs: `fixed_vertex` (the abstract cell), `pure_from_point`
/// (P-LLFP-02), `any_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
#[kani::stub(s2rst::s2::Cell::rect_bound, crate::stubs::cell_rect_bound_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn p_lreg_02() {
    crate::stubs::reset();
    crate::stubs::draw_abstract_cell();
    let r = any_rect();
    let (c, _) = any_cell();
    laws::region::no_intersect_no_vertex(&r, &c);
    kani::cover!(
        !s2rst::s2::Region::intersects_cell(&r, &c),
        "a cell the rectangle does not intersect"
    );
}

/// P-LMEM-03 (correct result): the rectangle's point test is `contains_lat_lng` of the point's
/// `LatLng`, which is valid for every point without a NaN coordinate (P-LLFP-02), so P-LMEM-01's
/// description applies to it (FR-012).
///
/// Exactly: for every valid rectangle and every triple of `f64`,
/// `contains_point(p) == rect_spec::member(r, ll.lat, ll.lng)` with `ll = LatLng::from_point(p)`.
///
/// Stubs: `pure_from_point` (P-LLFP-02 and purity), so the harness's `from_point` is the
/// test's without recomputing it: two copies of `x² + y²` and two `atan2` keys cost the checker
/// more than the statement (spec 008 research R0).
#[kani::proof]
#[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
fn p_lmem_03() {
    crate::stubs::reset();
    let r = any_rect();
    let p = any_triple();
    let ll = LatLng::from_point(p);
    assert!(r.contains_point(p) == rect_spec::member(r, ll.lat.radians(), ll.lng.radians()));
    kani::cover!(r.contains_point(p), "a contained point");
}
