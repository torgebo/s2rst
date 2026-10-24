// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell` children (P-CCHD, spec 004 data-model §4.1). P-CCHD-02, the children's u-v bounds,
//! is a level family in `cl_bnd` (research R8).

use crate::cell_spec;
use crate::spec;
use crate::value_domain::any_cell;

/// P-CCHD-01 (correct result): a cell has children exactly when it is not a leaf. Child `k` is
/// the cell of the model's child `k`, on the same face, one level down, with the orientation the
/// model gives.
///
/// For every valid cell with decomposition `d`: `children()` is `None` exactly at level 30;
/// otherwise child `k` has id `compose(spec::child(d, k))`, `d`'s face, level `d.level + 1` and
/// orientation `cell_spec::child_orientation(model(d).orientation, k)` (upstream `Subdivide`,
/// `s2cell.cc:85–124`).
#[kani::proof]
#[kani::unwind(31)]
fn p_cchd_01() {
    let (c, d) = any_cell();
    match c.children() {
        None => assert_eq!(d.level, 30),
        Some(kids) => {
            assert!(d.level < 30);
            let o = cell_spec::model(d).orientation;
            for k in 0..4u8 {
                let kid = kids[usize::from(k)];
                let want = cell_spec::child_model(d, k);
                assert_eq!(kid.id().0, spec::compose(want));
                assert_eq!(kid.face().as_u8(), d.face);
                assert_eq!(kid.level().as_u8(), d.level + 1);
                assert_eq!(kid.orientation(), cell_spec::child_orientation(o, k));
            }
            kani::cover!(o == 3, "an orientation-3 parent");
        }
    }
    kani::cover!(d.level == 29, "the children of a level-29 cell are leaves");
}
