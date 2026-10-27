// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-URANGE: range construction (FR-005). `from_begin_end`, `from_min_max` and `from_range`
//! are tool-limit exclusions (P-URANGE-01/02/03, research R0); only `whole_sphere` is proven.

use s2rst::s2::{CellId, CellUnion, Face};

use crate::union_domain::{any_leaf, out_covers};

/// P-URANGE-04 (correct result): `whole_sphere()` is the six face cells in order, and covers every
/// leaf.
///
/// `whole_sphere()` is the six face cells in order and covers every leaf (`WholeSphere()`). No
/// arguments: the whole domain.
#[kani::proof]
#[kani::unwind(8)]
fn p_urange_04() {
    let u = CellUnion::whole_sphere();
    let faces: Vec<CellId> = Face::ALL.into_iter().map(CellId::from_face).collect();
    assert_eq!(u.cell_ids(), &faces[..]);
    let (leaf, _) = any_leaf();
    assert!(out_covers::<6>(u.cell_ids(), leaf));
    kani::cover!(u.num_cells() == 6, "six faces");
}
