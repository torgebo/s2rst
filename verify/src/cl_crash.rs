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
