// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell` equality (P-CEQ, spec 004 data-model §4.1, §4.3).

use s2rst::s2::Cell;

use crate::laws;
use crate::value_domain::any_cell;

/// P-CEQ-01 (law): the generic equality laws, for `Cell`: equality is an equivalence. Two cells
/// are equal exactly when their identifiers are.
///
/// `laws::eq::all(&a, &b, &c)` for every triple of valid cells (spec 004 §4.3), and
/// `(a == b) == (a.id() == b.id())`.
#[kani::proof]
#[kani::unwind(9)]
fn p_ceq_01() {
    let (a, _) = any_cell();
    let (b, _) = any_cell();
    let (c, _) = any_cell();
    laws::eq::all(&a, &b, &c);
    assert_eq!(a == b, a.id() == b.id());
    kani::cover!(a == b && b == c, "three equal cells");
}

/// P-CEQ-02 (law): two cells built from the same identifier have identical fields, bounds
/// included, so equal cells cannot differ in any field.
///
/// The integer fields, for every valid `id`: `from_cell_id(id)` twice gives the same id, face,
/// level and orientation. The bounds are the family `cl_bnd::p_ceq_02__l<L>` (they square a
/// symbolic float; research R8). With FR-018, deserialisation builds cells the same way
/// (`core/src/s2/cell_tests.rs`).
#[kani::proof]
#[kani::unwind(9)]
fn p_ceq_02() {
    let (a, _) = any_cell();
    let b = Cell::from_cell_id(a.id());
    assert_eq!(
        (a.id(), a.face(), a.level(), a.orientation()),
        (b.id(), b.face(), b.level(), b.orientation())
    );
    kani::cover!(a.is_leaf(), "a leaf");
}
