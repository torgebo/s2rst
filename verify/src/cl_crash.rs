// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-Cell::<op>` for the in-scope `Cell` operations (spec 004 US1,
//! FR-004; data-model §5.1).
//!
//! Each harness draws its inputs from the operation's domain in `values/INVENTORY.md`,
//! assumes exactly the documented preconditions (a valid cell identifier, FR-017), and calls
//! the public API. Kani checks every panic, arithmetic overflow, undefined behaviour and debug
//! check on the way; the covers show that the domain is non-empty and that the outcomes FR-009
//! names are reachable. Operations that only read a field are grouped with the constructor, as
//! 001 grouped its accessors.

use s2rst::s2::{Cell, CellEdge, CellId, Region};

use crate::domain::{any_face, any_level};
use crate::value_domain::{any_cell, any_edge};

/// C-Cell::from_cell_id, with the accessors `id`, `face`, `level`, `orientation`, `is_leaf`,
/// `size_ij` and `bound_uv`: they complete without panic, arithmetic overflow or undefined
/// behaviour for every valid cell identifier.
#[kani::proof]
#[kani::unwind(9)]
fn c_from_cell_id() {
    let (c, d) = any_cell();
    let _ = (
        c.id(),
        c.face(),
        c.level(),
        c.orientation(),
        c.is_leaf(),
        c.size_ij(),
    );
    let _ = c.bound_uv();
    kani::cover!(d.level == 0, "a face cell");
    kani::cover!(d.level == 30, "a leaf cell");
    kani::cover!(c.orientation() == 3, "orientation 3");
    kani::cover!(d.face == 5, "face 5");
}

/// C-Cell::from_face: completes without panic, arithmetic overflow or undefined behaviour for
/// every face 0–5 (FR-017).
#[kani::proof]
#[kani::unwind(9)]
fn c_from_face() {
    let f = any_face();
    let c = Cell::from_face(f);
    kani::cover!(f == 0 && c.orientation() == 0, "face 0");
    kani::cover!(f == 5, "face 5");
}

/// C-Cell::from_face_pos_level: completes without panic, arithmetic overflow or undefined
/// behaviour for every face 0–5, position below `2^61` and level 0–30 (FR-017; `CellId`'s
/// constructor documents the position's range).
#[kani::proof]
#[kani::unwind(9)]
fn c_from_face_pos_level() {
    let f = any_face();
    let pos: u64 = kani::any();
    kani::assume(pos < (1u64 << 61));
    let l = any_level();
    let c = Cell::from_face_pos_level(f, pos, l);
    kani::cover!(c.is_leaf(), "a leaf");
    kani::cover!(l.as_u8() == 0, "a face cell");
}

/// C-Cell::from (`impl From<CellId> for Cell`) and C-Cell::default: complete without panic,
/// arithmetic overflow or undefined behaviour for every valid cell identifier.
#[kani::proof]
#[kani::unwind(9)]
fn c_from_and_default() {
    let (c, _) = any_cell();
    let from = Cell::from(c.id());
    let default = Cell::default();
    kani::cover!(from.level() == default.level(), "a face cell");
}

/// C-Cell::children: completes without panic, arithmetic overflow or undefined behaviour for
/// every valid cell.
#[kani::proof]
#[kani::unwind(9)]
fn c_children() {
    let (c, d) = any_cell();
    let kids = c.children();
    kani::cover!(kids.is_none(), "a leaf has no children");
    kani::cover!(
        kids.is_some() && d.level == 29,
        "the children of a level-29 cell"
    );
}

/// C-Cell::contains_cell, C-Cell::intersects_cell and C-Cell::cell_union_bound, inherent and
/// through the `Region` interface: complete without panic, arithmetic overflow or undefined
/// behaviour for every pair of valid cells.
#[kani::proof]
#[kani::unwind(9)]
fn c_containment() {
    let (a, _) = any_cell();
    let (b, _) = any_cell();
    let contains = a.contains_cell(b);
    let intersects = a.intersects_cell(b);
    let bound = a.cell_union_bound();
    let r: &dyn Region = &a;
    let (rc, ri) = (r.contains_cell(&b), r.intersects_cell(&b));
    let rbound = r.cell_union_bound();
    kani::cover!(contains && a.id() != b.id(), "a proper descendant");
    kani::cover!(!intersects, "disjoint cells");
    kani::cover!(
        rc && ri && bound.len() == 1 && rbound.len() == 1,
        "through the region"
    );
}

/// C-Cell::eq (`impl PartialEq for Cell`): completes without panic, arithmetic overflow or
/// undefined behaviour for every pair of valid cells.
#[kani::proof]
#[kani::unwind(9)]
fn c_eq() {
    let (a, _) = any_cell();
    let (b, _) = any_cell();
    let eq = a == b;
    kani::cover!(eq, "equal");
    kani::cover!(!eq, "different");
}

/// C-Cell::uv_coord_of_edge: completes without panic, arithmetic overflow or undefined
/// behaviour for every valid cell and edge.
#[kani::proof]
#[kani::unwind(9)]
fn c_uv_coord_of_edge() {
    let (c, _) = any_cell();
    let k = any_edge();
    let _ = c.uv_coord_of_edge(k);
    kani::cover!(k == CellEdge::Left, "the left edge");
}

/// C-Cell::from_cell_id's precondition is the documented one, no stronger: the cell of the
/// largest valid identifier on face 5 is built (FR-004 acceptance scenario 2).
#[kani::proof]
#[kani::unwind(9)]
fn c_from_cell_id_largest() {
    let c = Cell::from_cell_id(CellId((5u64 << 61) | ((1u64 << 61) - 1)));
    kani::cover!(c.is_leaf(), "the last leaf");
}

// ---------------------------------------------------------------------------------------------
// Outside the domain: the documented panics (gap 1; `crash.rs` explains the `n_*` convention).
// Every panic here is in every build, so there is no `r_*` counterpart.
// ---------------------------------------------------------------------------------------------

/// N-Cell::from_cell_id, N-From<CellId>, N-from_face, N-from_face_pos_level: each panics, in
/// every build, for every identifier with no level marker (62 or more trailing zero bits: `0`,
/// `1 << 62`, `1 << 63`, `3 << 62`) and every identifier on face 6 or 7 (an index out of bounds);
/// `from_face` for faces 6 and 7; `from_face_pos_level` for those faces and every raw level above
/// 30.
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(9)]
fn n_from_cell_id() {
    let x: u64 = kani::any();
    kani::assume(x.trailing_zeros() >= 62 || x >> 61 >= 6);
    let f: u8 = kani::any();
    kani::assume(f == 6 || f == 7);
    let pos: u64 = kani::any();
    kani::assume(pos < 1 << 61);
    let raw: u8 = kani::any();
    kani::assume(raw > 30);
    let op: u8 = kani::any();
    let _ = match op {
        0 => Cell::from_cell_id(CellId(x)),
        1 => Cell::from(CellId(x)),
        2 => Cell::from_face(f),
        3 => Cell::from_face_pos_level(f, pos, any_level()),
        _ => Cell::from_face_pos_level(any_face(), pos, raw),
    };
    kani::cover!(true, "returns");
}

/// O-Cell::from_cell_id (outside its domain, returns): completes without panic, overflow or
/// undefined behaviour for every other invalid identifier, those whose lowest set bit is at an odd
/// position up to 61 on faces 0–5 (its `# Panics`: a cell whose fields are unspecified). With
/// `n_from_cell_id` and `c_from_cell_id`, every `u64` is accounted for.
#[kani::proof]
#[kani::unwind(9)]
fn o_from_cell_id_unspecified() {
    let x: u64 = kani::any();
    let tz = x.trailing_zeros();
    kani::assume(tz % 2 == 1 && tz <= 61 && x >> 61 <= 5);
    let c = Cell::from_cell_id(CellId(x));
    kani::cover!(tz == 61, "the marker above the face cell's");
    kani::cover!(
        tz == 1 && c.face().as_u8() == 5,
        "below a leaf's marker, face 5"
    );
}

// ---------------------------------------------------------------------------------------------
// Floating-point geometry (gap 2 of the 2026-10-26 coverage review): the entries spec 004
// excluded as floating-point geometry, proven crash-free under the function contracts of
// `stubs` where they reach a C library function the checker does not model (spec 008 FR-003).
// Each harness names its contracts. Points range over `BoxPoint` (spec 006 research R4), which
// holds every unit point.
// ---------------------------------------------------------------------------------------------

/// C-Cell::from_point, and through it C-CellId::from_point and `From<&Point> for CellId` (001's
/// float-geometry rows): complete without panic, overflow or undefined behaviour for every box
/// point but the origin (which no face contains: `n_from_point`).
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_from_point() {
    let p = crate::value_domain::any_box_point();
    kani::assume(p.0.x != 0.0 || p.0.y != 0.0 || p.0.z != 0.0);
    let (c, id) = (Cell::from_point(p), CellId::from(&p));
    kani::cover!(c.face().as_u8() == 5 && id.is_leaf(), "a leaf on face 5");
}

/// C-Cell::from_lat_lng, and through it C-CellId::from_lat_lng and `From<&LatLng> for CellId`:
/// complete for every valid latitude–longitude, under `sin_cos`'s contract:
/// the checker's own `sin` and `cos` let the point be the zero vector, which no face contains
/// (`valid_face_xyz_to_uv`'s debug check failed, 2026-10-26).
///
/// Under contract: `sin_cos_contract`. Stubs: `any_uv_bound`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(f64::sin_cos, crate::stubs::sin_cos_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_from_lat_lng() {
    let ll = crate::value_domain::any_valid_lat_lng();
    let (c, id) = (Cell::from_lat_lng(ll), CellId::from(&ll));
    kani::cover!(c.is_leaf() && id.is_leaf(), "a leaf");
}

/// C-Cell::vertex, vertex_raw: complete for every valid cell and every vertex index below 4.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_vertex() {
    let (c, d) = any_cell();
    let k: usize = kani::any();
    kani::assume(k < 4);
    let _ = (c.vertex_raw(k), c.vertex(k));
    kani::cover!(d.level == 30 && k == 3, "a leaf's last vertex");
}

/// C-Cell::edge, edge_raw: complete for every valid cell and every edge.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_edge() {
    let (c, d) = any_cell();
    let e = any_edge();
    let _ = (c.edge_raw(e), c.edge(e));
    kani::cover!(d.level == 0, "a face cell's edge");
}

/// C-Cell::size_st, ij_coord_of_edge, and through `size_st` C-CellId::size_st for every level:
/// complete for every valid cell and every edge.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_size_and_ij() {
    let (c, d) = any_cell();
    let e = any_edge();
    let (_, ij) = (c.size_st(), c.ij_coord_of_edge(e));
    kani::cover!(
        d.level == 30 && ij == 1 << 30,
        "a leaf edge at the face's far side"
    );
}

/// C-Cell::center, center_raw, and through them C-CellId::to_point, to_point_raw (001's
/// float-geometry rows): complete for every valid cell.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_center() {
    let (c, d) = any_cell();
    let _ = (c.center(), c.center_raw());
    kani::cover!(d.level == 30, "a leaf");
}

/// C-Cell::average_area_for_level, average_area: complete for every level and every valid cell.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_average_area() {
    let (c, d) = any_cell();
    let a = (Cell::average_area_for_level(any_level()), c.average_area());
    kani::cover!(d.level == 30 && a.1 > 0.0, "a leaf's average area");
}

/// C-Cell::approx_area: completes for every valid cell at level 2 or more (the others return
/// `average_area`) and every abstract cell (vertices any four box points, spec 008 research R7):
/// it reads the cell's geometry through `vertex` only, so every real cell's vertices are among
/// the abstract ones.
///
/// Stubs: `fixed_vertex` (the abstract cell), `any_uv_bound`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_approx_area() {
    crate::stubs::draw_abstract_cell();
    let (c, d) = any_cell();
    let a = c.approx_area();
    kani::cover!(d.level >= 2 && a > 0.0, "a flat-area estimate");
}

/// C-Cell::exact_area: completes for every valid cell and every abstract cell (vertices any four
/// box points, as `c_fg_approx_area`), under the contracts of `atan2` (the vertices' angles),
/// `tan` and `atan` (l'Huilier's formula).
///
/// Under contract: `atan2_contract`, `tan_contract`, `atan_contract`. Stubs: `fixed_vertex`,
/// `any_uv_bound`.
///
/// **Not proven:** out of memory at 8 GiB weekly (2026-10-27; `point_area`'s robust cross products,
/// through the robust predicates' exact arithmetic), so it is in the xlarge tier (`make
/// verify-xlarge`, a 32 GB machine), which has not run it.
#[cfg(feature = "xlarge")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::tan, crate::stubs::tan_contract)]
#[kani::stub(f64::atan, crate::stubs::atan_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_exact_area() {
    crate::stubs::draw_abstract_cell();
    let (c, _) = any_cell();
    let a = c.exact_area();
    kani::cover!(a > 0.0, "a positive area");
}

/// C-Cell::contains_point and Region::contains_point: complete for every valid cell and every
/// box point.
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_contains_point() {
    let (c, _) = any_cell();
    let p = crate::value_domain::any_box_point();
    let r = (c.contains_point(p), Region::contains_point(&c, &p));
    kani::cover!(r.0, "a contained point");
    kani::cover!(!r.0, "a point outside");
}

/// C-Cell::cap_bound and Region::cap_bound: complete for every valid cell, with its vertices from
/// the abstract cell (any four box points; the centre is the cell's own).
///
/// Stubs: `fixed_vertex` (the abstract cell), `any_uv_bound`.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_cap_bound() {
    crate::stubs::draw_abstract_cell();
    let (c, d) = any_cell();
    let (cap, _) = (c.cap_bound(), Region::cap_bound(&c));
    kani::cover!(d.level == 0 && !cap.is_full(), "a face cell's cap");
}

/// C-Cell::rect_bound and Region::rect_bound: complete for every valid cell, under `atan2`'s
/// contract (the vertices' latitudes and longitudes).
///
/// Under contract: `atan2_contract`. Stubs: `any_uv_bound` (the abstract u-v bound).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_rect_bound() {
    let (c, d) = any_cell();
    let (r, _) = (c.rect_bound(), Region::rect_bound(&c));
    kani::cover!(d.level > 0 && !r.is_full(), "a proper rectangle");
}

/// C-Cell::distance_to_edge: completes for every valid cell, with the abstract cell's vertices
/// and containment (any four box points, any answer), and every pair of box points.
///
/// Stubs: `fixed_vertex`, `any_contains_point` (the abstract cell), `any_uv_bound`.
///
/// **Not proven:** out of memory at 8 GiB weekly (2026-10-27; `edge_pair_closest_points`, through
/// the robust predicates' exact arithmetic), so it is in the xlarge tier (`make verify-xlarge`, a
/// 32 GB machine), which has not run it.
#[cfg(feature = "xlarge")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_distance_to_edge() {
    crate::stubs::draw_abstract_cell();
    let (c, _) = any_cell();
    let (a, b) = (
        crate::value_domain::any_box_point(),
        crate::value_domain::any_box_point(),
    );
    let d = c.distance_to_edge(a, b);
    kani::cover!(d == s2rst::s1::ChordAngle::ZERO, "an endpoint inside");
    kani::cover!(d != s2rst::s1::ChordAngle::ZERO, "apart");
}

/// C-Cell::max_distance_to_edge: completes for every valid cell and every pair of box points,
/// with the abstract cell's vertices and containment where `distance_to_edge` is reached.
///
/// Stubs: `fixed_vertex`, `any_contains_point` (the abstract cell), `any_uv_bound`.
///
/// **Not proven:** out of memory at 8 GiB weekly (2026-10-27; `max_distance_to_point` and
/// `edge_pair_closest_points`, through the robust predicates' exact arithmetic), so it is in the
/// xlarge tier (`make verify-xlarge`, a 32 GB machine), which has not run it.
#[cfg(feature = "xlarge")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::fixed_vertex)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_max_distance_to_edge() {
    crate::stubs::draw_abstract_cell();
    let (c, _) = any_cell();
    let d = c.max_distance_to_edge(
        crate::value_domain::any_box_point(),
        crate::value_domain::any_box_point(),
    );
    kani::cover!(d > s2rst::s1::ChordAngle::RIGHT, "beyond a right angle");
}

/// C-Cell::distance_to_cell, is_distance_less, is_distance_less_or_equal: complete for every
/// pair of valid cells and every chord angle, with the cells' vertices from the abstract pair
/// (any four box points each) and their containment any answer.
///
/// Stubs: `pair_vertex`, `any_contains_point` (the abstract pair), `any_uv_bound`.
///
/// **Not proven:** out of memory at 8 GiB weekly (2026-10-27; 16 `edge_pair_closest_points`,
/// through the robust predicates' exact arithmetic), so it is in the xlarge tier (`make
/// verify-xlarge`, a 32 GB machine), which has not run it.
#[cfg(feature = "xlarge")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::pair_vertex)]
#[kani::stub(s2rst::s2::Cell::contains_point, crate::stubs::any_contains_point)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_distance_to_cell() {
    crate::stubs::draw_abstract_pair();
    let ((a, _), (b, _)) = (any_cell(), any_cell());
    let l = crate::value_domain::any_chord();
    let d = (
        a.distance_to_cell(b),
        a.is_distance_less(b, l),
        a.is_distance_less_or_equal(b, l),
    );
    kani::cover!(d.0 == s2rst::s1::ChordAngle::ZERO, "touching cells");
    kani::cover!(d.0 != s2rst::s1::ChordAngle::ZERO, "apart");
}

/// C-Cell::max_distance_to_cell, is_max_distance_less, is_max_distance_less_or_equal: complete
/// for every pair of valid cells and every chord angle, with the cells' vertices from the abstract
/// pair.
///
/// Stubs: `pair_vertex` (the abstract pair), `any_uv_bound`.
///
/// **Not proven:** out of memory at 8 GiB weekly (2026-10-27; 32 `update_max_distance`, through the
/// robust predicates' exact arithmetic), so it is in the xlarge tier (`make verify-xlarge`, a 32 GB
/// machine), which has not run it.
#[cfg(feature = "xlarge")]
#[kani::proof]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::Cell::vertex, crate::stubs::pair_vertex)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn c_fg_max_distance_to_cell() {
    crate::stubs::draw_abstract_pair();
    let ((a, _), (b, _)) = (any_cell(), any_cell());
    let l = crate::value_domain::any_chord();
    let d = (
        a.max_distance_to_cell(b),
        a.is_max_distance_less(b, l),
        a.is_max_distance_less_or_equal(b, l),
    );
    kani::cover!(d.0 == s2rst::s1::ChordAngle::STRAIGHT, "antipodal cells");
}

/// The zero vector, with any signs.
fn any_zero_vector() -> s2rst::s2::Point {
    let s: [bool; 3] = kani::any();
    let z = |neg: bool| if neg { -0.0 } else { 0.0 };
    crate::point_spec::point(z(s[0]), z(s[1]), z(s[2]))
}

/// N-Cell::from_point, N-CellId::from_point: in debug builds, each panics for the zero vector,
/// with any signs (no face contains it: the face projection's check).
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(9)]
fn n_from_point() {
    let p = any_zero_vector();
    if kani::any::<bool>() {
        let _ = Cell::from_point(p);
    } else {
        let _ = CellId::from_point(&p);
    }
    kani::cover!(true, "returns");
}

/// R-Cell::from_point, R-CellId::from_point: in a build without debug assertions, each completes
/// without panic or overflow for the zero vector, with an unspecified leaf.
#[cfg(not(debug_assertions))]
#[kani::proof]
#[kani::unwind(9)]
fn r_from_point() {
    let p = any_zero_vector();
    let (c, id) = (Cell::from_point(p), CellId::from_point(&p));
    assert!(c.is_leaf() && id.is_leaf());
    kani::cover!(true, "a leaf");
}

/// N-Cell::vertex, N-vertex_raw: each panics for every valid cell and every index above 3, in
/// every build (an index out of bounds).
///
/// Stubs: `any_uv_bound` (the abstract u-v bound: any rectangle in [−1, 1]², which holds every
/// cell's).
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(9)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::any_uv_bound)]
fn n_vertex() {
    let (c, _) = any_cell();
    let k: usize = kani::any();
    kani::assume(k > 3);
    if kani::any::<bool>() {
        let _ = c.vertex(k);
    } else {
        let _ = c.vertex_raw(k);
    }
    kani::cover!(true, "returns");
}
