// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell` containment and the region interface (P-CCNT, spec 004 data-model §4.1, §4.3).

use s2rst::s2::{Cell, Region};

use crate::cell_spec;
use crate::laws;
use crate::value_domain::any_cell;

/// P-CCNT-01 (correct result): one cell contains another exactly when the model's identifier
/// containment holds, and two cells intersect exactly when one contains the other.
///
/// For every pair of valid cells: `a.contains_cell(b) == cell_spec::contains(da, db)` and
/// `a.intersects_cell(b) == cell_spec::intersects(da, db)`.
#[kani::proof]
#[kani::unwind(9)]
fn p_ccnt_01() {
    let (a, da) = any_cell();
    let (b, db) = any_cell();
    assert_eq!(a.contains_cell(b), cell_spec::contains(da, db));
    assert_eq!(a.intersects_cell(b), cell_spec::intersects(da, db));
    kani::cover!(
        a.contains_cell(b) && da.level < db.level,
        "a proper descendant"
    );
    kani::cover!(b.contains_cell(a) && !a.contains_cell(b), "an ancestor");
    kani::cover!(
        !a.intersects_cell(b) && da.face == db.face,
        "disjoint on one face"
    );
}

/// P-CCNT-02 (correct result): the region interface agrees with the inherent operations, and a
/// cell's covering is the union of its own identifier.
///
/// For every pair of valid cells: `Region::contains_cell(&a, &b) == a.contains_cell(b)`, the
/// same for `intersects_cell`, and both `cell_union_bound`s are exactly `[a.id()]`.
#[kani::proof]
#[kani::unwind(9)]
fn p_ccnt_02() {
    let (a, _) = any_cell();
    let (b, _) = any_cell();
    let r: &dyn Region = &a;
    assert_eq!(r.contains_cell(&b), a.contains_cell(b));
    assert_eq!(r.intersects_cell(&b), a.intersects_cell(b));
    let (inherent, region) = (a.cell_union_bound(), r.cell_union_bound());
    assert_eq!(inherent.len(), 1);
    assert_eq!(region.len(), 1);
    assert_eq!(inherent[0], a.id());
    assert_eq!(region[0], a.id());
    kani::cover!(r.contains_cell(&b), "containment through the region");
}

/// P-CCNT-03 (law): the generic `Region` laws, for `Cell`: containing a cell implies
/// intersecting it, and every cell a region contains is covered by its covering bound.
///
/// `laws::region::all::<Cell, 1>(&a, &b)` for every pair of valid cells (spec 004 §4.3, research
/// R15; the quickcheck twins are `region_cell_contains_implies_intersects` and
/// `region_cell_union_bound_covers` in `core/tests/trait_property_tests.rs`).
#[kani::proof]
#[kani::unwind(9)]
fn p_ccnt_03() {
    let (a, _) = any_cell();
    let (b, _) = any_cell();
    laws::region::all::<Cell, 1>(&a, &b);
    kani::cover!(
        a.contains_cell(b) && a.id() != b.id(),
        "a contained descendant"
    );
}
