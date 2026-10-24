// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell` identity and construction (P-CID, spec 004 data-model §4.1), against the reference
//! description of a cell (`cell_spec`, on 001's model).

use s2rst::s2::{Cell, CellId};

use crate::cell_spec;
use crate::domain::{any_face, any_level};
use crate::spec::{self, Decomposed};
use crate::value_domain::any_cell;

/// P-CID-01 (correct result): a cell built from an identifier reports that identifier, and the
/// face, level and orientation the model gives for it.
///
/// For every valid `id` with decomposition `d`: `Cell::from_cell_id(id)` has `id() == id` and
/// `(face, level, orientation) == cell_spec::model(d)`'s (upstream `S2Cell(S2CellId)`,
/// `s2cell.cc:64–71`).
#[kani::proof]
#[kani::unwind(31)]
fn p_cid_01() {
    let (c, d) = any_cell();
    let m = cell_spec::model(d);
    assert_eq!(c.id().0, spec::compose(d));
    assert_eq!(c.face().as_u8(), m.face);
    assert_eq!(c.level().as_u8(), m.level);
    assert_eq!(c.orientation(), m.orientation);
    kani::cover!(
        m.orientation == 3 && d.level == 30,
        "a leaf with orientation 3"
    );
    kani::cover!(m.orientation == 1 && d.level == 0, "an odd face");
}

/// P-CID-02 (correct result): a cell is a leaf exactly when its level is 30, and its edge in
/// (i, j) space is 2^(30 − level) long.
///
/// For every valid cell: `is_leaf() == (level == 30)` and `size_ij() == spec::size_ij(level)`.
#[kani::proof]
#[kani::unwind(9)]
fn p_cid_02() {
    let (c, d) = any_cell();
    assert_eq!(c.is_leaf(), d.level == 30);
    assert_eq!(c.size_ij() as u32, spec::size_ij(d.level));
    kani::cover!(c.is_leaf(), "a leaf");
    kani::cover!(c.size_ij() == 1 << 30, "a face cell");
}

/// P-CID-03 (correct result): the face constructor, the face-position-level constructor,
/// conversion from an identifier and the default value give the cell of the identifier the
/// model gives (face 0 for the default).
///
/// - `from_face(f)` is the cell of `compose(f, 0, 0)`;
/// - `from_face_pos_level(f, pos, l)`, for `pos < 2^61`, is the cell of the model's ancestor at
///   level `l` of the leaf with path `pos >> 1` (upstream `FromFacePosLevel`);
/// - `Cell::from(id)` is `from_cell_id(id)`, and `Cell::default()` is `from_face(0)`.
#[kani::proof]
#[kani::unwind(9)]
fn p_cid_03() {
    let f = any_face();
    let face = Cell::from_face(f);
    assert_eq!(
        face.id().0,
        spec::compose(Decomposed {
            face: f,
            level: 0,
            path: 0
        })
    );
    let pos: u64 = kani::any();
    kani::assume(pos < (1u64 << 61));
    let l = any_level();
    let c = Cell::from_face_pos_level(f, pos, l);
    let leaf = Decomposed {
        face: f,
        level: 30,
        path: pos >> 1,
    };
    let want = spec::ancestor(leaf, l.as_u8());
    assert_eq!(c.id().0, spec::compose(want));
    assert_eq!(
        (c.face().as_u8(), c.level().as_u8()),
        (want.face, want.level)
    );
    let from = Cell::from(c.id());
    assert_eq!(from.id(), c.id());
    assert_eq!(
        (from.face(), from.level(), from.orientation()),
        (c.face(), c.level(), c.orientation())
    );
    let default = Cell::default();
    assert_eq!(default.id(), CellId::from_face(0));
    assert_eq!((default.face().as_u8(), default.level().as_u8()), (0, 0));
    kani::cover!(l.as_u8() == 30, "a leaf");
    kani::cover!(f == 5 && l.as_u8() == 15, "a level-15 cell on face 5");
}
