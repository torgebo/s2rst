// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s2::Cap::<op>` for the cap's in-scope entries (spec 008 US1,
//! FR-004; data-model §5).
//!
//! The domains:
//! - `BoxCap`: a centre in [−1, 1]³ and a valid cap's radius (`NEGATIVE` or a length in [0, 4]),
//!   which holds every valid cap (research R4); `BoxPoint` for points;
//! - every `f64` for the height and area constructors, every non-NaN angle for
//!   `from_center_angle`, `FiniteNonNegAngle` for `expanded` (its documented precondition);
//! - `ValidCap` (a unit centre, through `stubs::pure_is_unit`) where a debug check tests
//!   validity.
//!
//! The entries through `asin`, `atan2` and `cos` run under their function contracts (FR-003),
//! and the cell tests over the abstract cell (research R7); each harness names its stubs. The
//! debug checks of the chord-angle sum (spec 007 FR-019) are proven never to fire: every sum a
//! cap computes has non-special operands on these domains.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::{Cap, Cell, CellId, Region};

use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point, any_finite_nonneg_angle};

/// C-s2::Cap::from_center_chord_angle, from_point, from_center_height, from_center_area, empty,
/// full, and the accessors `center`, `chord_radius`, `height`, `area`, `centroid`: complete
/// without panic, overflow or undefined behaviour for every box point and every `f64`.
#[kani::proof]
fn c_construction_and_measures() {
    let c = any_box_point();
    let x: f64 = kani::any();
    let caps = [
        Cap::from_center_chord_angle(c, ChordAngle::from_length2(x)),
        Cap::from_point(c),
        Cap::from_center_height(c, x),
        Cap::from_center_area(c, x),
        Cap::empty(),
        Cap::full(),
    ];
    for cap in caps {
        let _ = (
            cap.center(),
            cap.chord_radius(),
            cap.height(),
            cap.area(),
            cap.centroid(),
        );
    }
    kani::cover!(caps[2].is_empty(), "a negative height");
    kani::cover!(caps[3].is_full(), "an area of 4π or more");
}

/// C-s2::Cap::is_valid, is_empty, is_full, equal, PartialEq, cap_bound, and Region::cap_bound:
/// complete for every pair of box caps.
#[kani::proof]
fn c_classifications_and_equality() {
    let (a, b) = (any_box_cap(), any_box_cap());
    let _ = (a.is_valid(), a.is_empty(), a.is_full(), a.equal(b), a == b);
    let _ = (a.cap_bound(), Region::cap_bound(&a));
    kani::cover!(a.is_empty(), "an empty cap");
    kani::cover!(a.is_full(), "a full cap");
}

/// C-s2::Cap::from_center_angle: completes for a unit centre and every non-NaN angle; its
/// debug-build validity check never fires there (the clamp to π keeps the radius at most 4,
/// and a negative angle gives `NEGATIVE`).
///
/// Stubs: `pure_is_unit` (L-CP-07, L-CP-04): the centre's unit length is assumed and then
/// checked by the constructor, one computation to the checker (research R0b, 1.7 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn c_from_center_angle() {
    stubs::reset();
    let c = any_box_point();
    kani::assume(c.is_unit());
    let a: f64 = kani::any();
    kani::assume(!a.is_nan());
    let cap = Cap::from_center_angle(c, Angle::from_radians(a));
    kani::cover!(cap.is_full(), "an angle of π or more");
    kani::cover!(cap.is_empty(), "a negative angle");
}

/// C-s2::Cap::contains_point, interior_contains_point, and Region::contains_point: complete
/// for every box cap and box point.
#[kani::proof]
fn c_point_containment() {
    let (a, p) = (any_box_cap(), any_box_point());
    let _ = (
        a.contains_point(p),
        a.interior_contains_point(p),
        Region::contains_point(&a, &p),
    );
    kani::cover!(a.contains_point(p), "a contained point");
}

/// C-s2::Cap::contains: completes for every pair of box caps; its chord-angle sum has
/// non-special operands (the other cap is not empty when it is reached).
#[kani::proof]
fn c_contains() {
    let (a, b) = (any_box_cap(), any_box_cap());
    let r = a.contains(b);
    kani::cover!(r && !b.is_empty(), "a non-empty contained cap");
    kani::cover!(!r, "a cap not contained");
}

/// C-s2::Cap::intersects and interior_intersects: complete for every pair of box caps; their
/// sums are reached only with two non-empty caps.
#[kani::proof]
fn c_intersects() {
    let (a, b) = (any_box_cap(), any_box_cap());
    let (i, ii) = (a.intersects(b), a.interior_intersects(b));
    kani::cover!(ii, "caps whose interiors intersect");
    kani::cover!(!i, "disjoint caps");
}

/// C-s2::Cap::complement: completes for every box cap.
#[kani::proof]
fn c_complement() {
    let a = any_box_cap();
    let k = a.complement();
    kani::cover!(a.is_empty() && k.is_full(), "the empty cap's complement");
}

/// C-s2::Cap::expanded: completes for every box cap and every finite non-negative distance,
/// its documented domain (a negative, NaN or infinite distance fails a debug check, as
/// upstream's; `cap.rs`'s `# Panics`).
#[kani::proof]
fn c_expanded() {
    let (a, d) = (any_box_cap(), any_finite_nonneg_angle());
    let e = a.expanded(d);
    kani::cover!(e.is_full() && !a.is_full(), "grown to the full cap");
    kani::cover!(a.is_empty() && e.is_empty(), "the empty cap stays empty");
}

/// C-s2::Cap::add_point: completes for every box cap and box point.
#[kani::proof]
fn c_add_point() {
    let (a, p) = (any_box_cap(), any_box_point());
    let c = a.add_point(p);
    kani::cover!(
        a.is_empty() && !c.is_empty(),
        "the empty cap takes the point"
    );
    kani::cover!(
        !a.is_empty() && c.chord_radius().length2() > a.chord_radius().length2(),
        "a non-empty cap grows"
    );
}

/// C-s2::Cap::add_cap: completes for every pair of box caps.
#[kani::proof]
fn c_add_cap() {
    let (a, b) = (any_box_cap(), any_box_cap());
    let c = a.add_cap(b);
    kani::cover!(b.is_empty(), "adding the empty cap");
    kani::cover!(
        !a.is_empty() && c.chord_radius().length2() > a.chord_radius().length2(),
        "a non-empty cap grows"
    );
}

/// C-Region for s2::Cap::contains_cell and intersects_cell (upstream's cell tests, FR-018), the
/// vertex part: complete for every box cap and every abstract cell (vertices any four box
/// points, research R7), with upstream's edge test (`Cap::intersects_cell_edges`) abstracted to
/// any answer. The edge test's own crash freedom is the weekly `c_cell_tests__edges_*` members;
/// together they cover both tests (composition).
///
/// Stubs: `fixed_vertex` (the abstract cell), `any_edge_result`.
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(
    s2rst::s2::cap::Cap::intersects_cell_edges,
    crate::stubs::any_edge_result
)]
fn c_cell_tests__vertices() {
    stubs::draw_abstract_cell();
    let a = any_box_cap();
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let _ = (a.contains_cell(&cell), a.intersects_cell(&cell));
    kani::cover!(a.intersects_cell(&cell), "an intersecting cell");
}

/// C-Region for s2::Cap::contains_cell, the edge test included: completes for every box cap and
/// every abstract cell (vertices, edge normals and the centre test abstracted). Weekly: both cell
/// tests in one harness timed out at 12 minutes (2026-10-24); the edge test alone took 385 s in
/// research R0b.
///
/// Stubs: `fixed_vertex`, `fixed_edge`, `any_contains_point` (the abstract cell).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::edge_raw, crate::stubs::fixed_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
fn c_cell_tests__edges_contains() {
    stubs::draw_abstract_cell();
    let a = any_box_cap();
    let r = a.contains_cell(&Cell::from_cell_id(CellId::from_face(0)));
    kani::cover!(r, "a contained cell");
    kani::cover!(!r, "a cell not contained");
}

/// C-Region for s2::Cap::intersects_cell, the edge test included: as
/// `c_cell_tests__edges_contains`. Weekly.
///
/// Stubs: `fixed_vertex`, `fixed_edge`, `any_contains_point` (the abstract cell).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::edge_raw, crate::stubs::fixed_edge)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
fn c_cell_tests__edges_intersects() {
    stubs::draw_abstract_cell();
    let a = any_box_cap();
    let r = a.intersects_cell(&Cell::from_cell_id(CellId::from_face(0)));
    kani::cover!(r, "an intersecting cell");
    kani::cover!(!r, "a disjoint cell");
}

/// C-s2::Cap::angle_radius: completes for every box cap, under `asin`'s contract.
///
/// Under contract: `asin_contract`.
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn c_angle_radius() {
    let a = any_box_cap();
    let r = a.angle_radius().radians();
    kani::cover!(r == -1.0, "the empty cap's −1 radian");
}

/// C-s2::Cap::rect_bound and Region::rect_bound: complete for every box cap, under the
/// contracts of `asin`, `atan2` and `cos` (research R0b, 84.6 s for the validity property).
///
/// Under contract: `asin_contract`, `atan2_contract`, `cos_contract`.
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
fn c_rect_bound() {
    let a = any_box_cap();
    let (r, _) = (a.rect_bound(), Region::rect_bound(&a));
    kani::cover!(r.is_empty(), "the empty rectangle");
    kani::cover!(!r.is_empty() && !r.is_full(), "a proper rectangle");
}

/// C-s2::Cap::approx_eq and approx_eq_with: complete for every pair of box caps and every
/// `f64` tolerance, under `atan2`'s contract (the centres' angle).
///
/// Under contract: `atan2_contract`.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn c_approx_eq() {
    let (a, b) = (any_box_cap(), any_box_cap());
    let e: f64 = kani::any();
    let (x, _) = (a.approx_eq(b), a.approx_eq_with(b, e));
    kani::cover!(x, "approximately equal caps");
    kani::cover!(!x, "caps not approximately equal");
}

/// C-s2::Cap::cell_union_bound and Region::cell_union_bound: complete for every valid cap,
/// under `asin`'s contract. Weekly: R0 timed out at 12 minutes (`CellId::from_point` and
/// `vertex_neighbors`).
///
/// P-CPBND-03 (correct result): the covering has at most six ids, the six faces or the vertex
/// neighbours.
///
/// **Not proven:** times out at 60 minutes weekly too (`known-timeouts.txt`, a tool limit). The
/// statement rests on the C++ vectors (`cpp_cap_diff`, every row) and the tests.
///
/// Under contract: `asin_contract`. Stubs: `pure_is_unit`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn c_cell_union_bound() {
    stubs::reset();
    let a = crate::value_domain::any_valid_cap();
    let ids = a.cell_union_bound();
    assert!(ids.len() <= 6);
    let _ = Region::cell_union_bound(&a);
}

/// C-s2::Cap::union: completes for every pair of valid caps, under the contracts of `asin` and
/// `atan2`. Weekly: R0 timed out at 12 minutes. Crash freedom only: neither upstream's union
/// nor the library's always contains both caps (K9; upstream's is U-015 in `UPSTREAM.md`).
///
/// **Not proven, and in no CI leg:** out of memory at 8 GiB, and alone at 16 GiB a timeout at
/// 60 minutes (a tool limit). `union`'s crash freedom rests on the C++ vectors and the tests.
///
/// Under contract: `asin_contract`, `atan2_contract`. Stubs: `pure_is_unit`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn c_union() {
    stubs::reset();
    let (a, b) = (
        crate::value_domain::any_valid_cap(),
        crate::value_domain::any_valid_cap(),
    );
    let _ = a.union(b);
}

/// C-Region for s2::Cap::contains_cell and intersects_cell over every valid cell with the
/// cell's real edges and `contains_point`; the vertices from the abstract cell (their
/// normalisation is spec 006's). Weekly.
///
/// **Not proven:** times out at 60 minutes weekly (`known-timeouts.txt`, a tool limit). The
/// abstract-cell harnesses prove the same tests with the edges abstracted too.
///
/// Stubs: `fixed_vertex`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
fn c_cell_tests__real() {
    stubs::draw_abstract_cell();
    let a = crate::value_domain::any_nonempty_cap();
    let (cell, _) = crate::value_domain::any_cell();
    let _ = (a.contains_cell(&cell), a.intersects_cell(&cell));
}
