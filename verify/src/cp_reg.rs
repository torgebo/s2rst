// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The generic `Region` laws instantiated for `Cap` (P-CPREG, spec 008 data-model §4; research
//! R7).
//!
//! The cell laws run over the abstract cell: `Cell::vertex` and `Cell::edge_raw` return a table
//! of any four box points and any four box vectors (`stubs::draw_abstract_cell`). The laws depend
//! only on the order of upstream's tests (the vertices before the edges, `s2cap.cc:266–290`), so
//! proving them for any values proves them for every real cell. The point tests on the vertices
//! use the pure chord, so the law and the cap's test read one distance per vertex.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{Cell, CellId};

use crate::laws;
use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point};

fn face0() -> Cell {
    Cell::from_cell_id(CellId::from_face(0))
}

/// P-CPREG-01 (law): containing a cell implies intersecting it
/// (`laws::region::contains_implies_intersects`), with upstream's edge test abstracted to any
/// answer.
///
/// For every `cap` in `BoxCap` and every abstract cell: `cap.contains_cell(c)` implies
/// `cap.intersects_cell(c)`.
///
/// Stubs: `fixed_vertex` (the abstract cell), `pure_chord` (L-CP-01, L-CP-02),
/// `any_edge_result` on `Cap::intersects_cell_edges` (research R0c, 53.0 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(
    s2rst::s2::cap::Cap::intersects_cell_edges,
    crate::stubs::any_edge_result
)]
fn p_cpreg_01__abstract() {
    stubs::reset();
    stubs::draw_abstract_cell();
    let cap = any_box_cap();
    let cell = face0();
    laws::region::contains_implies_intersects(&cap, &cell);
    kani::cover!(
        s2rst::s2::Region::contains_cell(&cap, &cell),
        "a contained cell"
    );
}

/// P-CPREG-01 (law), with upstream's real edge test over the abstract cell. Weekly (R0c,
/// 696.9 s).
///
/// Stubs: `fixed_vertex`, `fixed_edge`, `any_contains_point` (the abstract cell), `pure_chord`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::edge_raw, crate::stubs::fixed_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
fn p_cpreg_01__real() {
    stubs::reset();
    stubs::draw_abstract_cell();
    let cap = any_box_cap();
    laws::region::contains_implies_intersects(&cap, &face0());
}

/// P-CPREG-02 (law): a cell the cap does not intersect has no vertex the cap contains
/// (`laws::region::no_intersect_no_vertex`).
///
/// For every `cap` in `BoxCap` and every abstract cell: if `!cap.intersects_cell(c)`, no
/// `c.vertex(k)` is contained.
///
/// Stubs: `fixed_vertex`, `pure_chord`, `any_edge_result` (research R0c, 51.1 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(
    s2rst::s2::cap::Cap::intersects_cell_edges,
    crate::stubs::any_edge_result
)]
fn p_cpreg_02() {
    stubs::reset();
    stubs::draw_abstract_cell();
    let cap = any_box_cap();
    laws::region::no_intersect_no_vertex(&cap, &face0());
}

/// P-CPREG-03 (law): the bounding cap contains every point the cap contains
/// (`laws::region::cap_bound_contains_points`).
///
/// For every `cap` in `BoxCap` and `p` in `BoxPoint`: if `cap.contains_point(p)`, then
/// `cap.cap_bound().contains_point(p)`.
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02): `cap_bound` is the cap itself, and the law computes
/// the same distance twice.
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
fn p_cpreg_03() {
    stubs::reset();
    let (cap, p) = (any_box_cap(), any_box_point());
    laws::region::cap_bound_contains_points(&cap, &p);
}
