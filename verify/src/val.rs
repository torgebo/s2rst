// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Validity properties P-VAL-* (FR-005): what an id's bits mean, stated against the reference
//! model `spec`. Each harness's doc gives its kind and plain-language statement first
//! (`verify/PROPERTIES_BY_KIND.md`), then the exact statement.

use s2rst::s2::{CellId, Face, lsb_for_level, size_ij};

use crate::domain::{any_decomposed, any_face, any_level, any_u64, any_valid};
use crate::spec::{self, Decomposed};

/// P-VAL-01 (model check): in the cell model, building an id from a face, a level and a path,
/// then splitting it, gives back the same parts.
///
/// `decompose(compose(d)) == Some(d)` for every well-formed `d`.
#[kani::proof]
#[kani::unwind(32)]
fn p_val_01() {
    let d = any_decomposed();
    assert_eq!(spec::decompose(spec::compose(d)), Some(d));
    kani::cover!(d.level == 30, "leaf");
    kani::cover!(d.level == 0, "face");
}

/// P-VAL-02 (model check): in the cell model, every id that splits into parts is rebuilt
/// exactly from them.
///
/// `compose(decompose(x)?) == x` whenever `decompose(x)` is some.
#[kani::proof]
#[kani::unwind(32)]
fn p_val_02() {
    let x: u64 = kani::any();
    let d = spec::decompose(x);
    if let Some(d) = d {
        assert_eq!(spec::compose(d), x);
    }
    kani::cover!(d.is_some(), "valid");
    kani::cover!(d.is_none(), "invalid");
}

/// P-VAL-03 (validity): `is_valid()` holds exactly for the ids that name a cell: a face from 0
/// to 5, and one of the 31 level marker bits set with every bit below it clear.
///
/// `CellId(x).is_valid() == spec::valid(x)`.
#[kani::proof]
#[kani::unwind(32)]
fn p_val_03() {
    let c = any_u64();
    let v = c.is_valid();
    assert_eq!(v, spec::valid(c.0));
    kani::cover!(v, "valid");
    kani::cover!(!v && c.0 >> 61 <= 5, "invalid with a real face");
}

/// P-VAL-04 (round trip): converting a `u64` to a `CellId` and back gives the same number.
///
/// `CellId::from(x).id() == x` and `u64::from(CellId(x)) == x`.
#[kani::proof]
fn p_val_04() {
    let x: u64 = kani::any();
    assert_eq!(CellId::from(x).id(), x);
    assert_eq!(u64::from(CellId(x)), x);
    kani::cover!(x == u64::MAX, "all ones");
}

/// P-VAL-05 (correct result): `none()` is 0, `sentinel()` is all ones, `default()` is `none()`,
/// and neither is a valid cell.
///
/// `none().id() == 0`, `sentinel().id() == u64::MAX`, `CellId::default() == none()`, and
/// neither is valid.
#[kani::proof]
#[kani::unwind(32)]
fn p_val_05() {
    assert_eq!(CellId::none().id(), spec::NONE);
    assert_eq!(CellId::sentinel().id(), spec::SENTINEL);
    assert_eq!(CellId::default(), CellId::none());
    assert!(!CellId::none().is_valid() && !spec::valid(spec::NONE));
    assert!(!CellId::sentinel().is_valid() && !spec::valid(spec::SENTINEL));
    kani::cover!(CellId::default().0 == 0, "default is none");
}

/// P-VAL-06 (correct result): `face()`, `level()` and `pos()` read the face, the level and the
/// position bits of the id.
///
/// `face() == d.face`, `level() == d.level`, `pos() == id & (2^61 − 1)`.
#[kani::proof]
fn p_val_06() {
    let (c, d) = any_valid();
    assert_eq!(c.face().as_u8(), d.face);
    assert_eq!(c.level().as_u8(), d.level);
    assert_eq!(c.pos(), c.0 & ((1u64 << 61) - 1));
    kani::cover!(d.face == 5 && d.level == 30, "leaf on face 5");
}

/// P-VAL-07 (correct result): `lsb()` is the level's marker bit; `is_leaf()` holds exactly at
/// level 30 and `is_face()` exactly at level 0.
///
/// `lsb() == spec::lsb_for_level(d.level)`, `is_leaf() ⇔ d.level == 30`,
/// `is_face() ⇔ d.level == 0`.
#[kani::proof]
fn p_val_07() {
    let (c, d) = any_valid();
    assert_eq!(c.lsb(), spec::lsb_for_level(d.level));
    assert_eq!(c.is_leaf(), d.level == 30);
    assert_eq!(c.is_face(), d.level == 0);
    kani::cover!(c.is_leaf(), "leaf");
    kani::cover!(c.is_face(), "face");
}

/// P-VAL-08 (correct result): the level constants: `lsb_for_level(l)` is the level's marker
/// bit, and `size_ij(l)` is a cell's edge in leaves.
///
/// `lsb_for_level(l) == 1 << (60 − 2l)` and `size_ij(l) == 1 << (30 − l)`.
#[kani::proof]
fn p_val_08() {
    let l = any_level();
    assert_eq!(lsb_for_level(l), spec::lsb_for_level(l.as_u8()));
    assert_eq!(size_ij(l), spec::size_ij(l.as_u8()) as i32);
    kani::cover!(l.as_u8() == 0, "level 0");
    kani::cover!(l.as_u8() == 30, "level 30");
}

/// P-VAL-09 (correct result): `from_face(f)` is the level-0 cell of face `f`.
///
/// `from_face(f) == compose({f, 0, 0})`.
#[kani::proof]
fn p_val_09() {
    let f = any_face();
    assert_eq!(
        CellId::from_face(Face::from_u8(f)).0,
        spec::compose(Decomposed {
            face: f,
            level: 0,
            path: 0
        })
    );
    kani::cover!(f == 5, "face 5");
}

/// P-VAL-10 (correct result): `from_face_pos_level(f, pos, l)` is the level-`l` cell that
/// contains the leaf at position `pos`.
///
/// `from_face_pos_level(f, pos, l) == compose(ancestor({f, 30, pos >> 1}, l))`, for
/// `pos < 2^61`.
#[kani::proof]
fn p_val_10() {
    let f = any_face();
    let pos: u64 = kani::any();
    kani::assume(pos < (1u64 << 61));
    let l = any_level();
    let leaf = Decomposed {
        face: f,
        level: 30,
        path: pos >> 1,
    };
    let want = spec::compose(spec::ancestor(leaf, l.as_u8()));
    assert_eq!(
        CellId::from_face_pos_level(Face::from_u8(f), pos, l).0,
        want
    );
    kani::cover!(l.as_u8() == 0, "face");
    kani::cover!(
        l.as_u8() == 30 && pos & 1 == 0,
        "leaf from an even position"
    );
}
