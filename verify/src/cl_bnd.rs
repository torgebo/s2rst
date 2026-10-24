// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell` bounds (P-CBND, spec 004 data-model §4.1) and the children's bounds (P-CCHD-02).
//!
//! The u-v bounds are squares of a symbolic float (`st_to_uv`), so properties of them prove
//! only per level: the `__l<L>` families (research R0, R8). Levels 0–3 run per change; members
//! from level 4 up are in the weekly tier (feature `slow`), pending T031's measurements; levels that do not fit within the weekly limit are
//! recorded as not proven in `values/PROPERTIES.md`, where the exhaustive projection check
//! (`tests/uv_projection.rs`) and the C++ vectors cover them. P-CBND-03 reads the stored bounds
//! without arithmetic and proves whole.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{Cell, CellEdge};

use crate::cell_spec::{self, CellModel};
use crate::spec;
use crate::value_domain::{any_cell, any_cell_at, any_edge};

/// P-CBND-03 (correct result): edge k's u-v coordinate is the matching side of the bounds:
/// bottom is v low, right is u high, top is v high, left is u low.
///
/// For every valid cell and edge: `uv_coord_of_edge(k)` is, bit for bit, `bound_uv().y.lo`,
/// `x.hi`, `y.hi` or `x.lo` for `Bottom`, `Right`, `Top`, `Left` (upstream `GetUVCoordOfEdge`).
#[kani::proof]
#[kani::unwind(9)]
fn p_cbnd_03() {
    let (c, _) = any_cell();
    let k = any_edge();
    let b = c.bound_uv();
    let want = match k {
        CellEdge::Bottom => b.y.lo,
        CellEdge::Right => b.x.hi,
        CellEdge::Top => b.y.hi,
        CellEdge::Left => b.x.lo,
    };
    assert_eq!(c.uv_coord_of_edge(k).to_bits(), want.to_bits());
    kani::cover!(k == CellEdge::Top, "the top edge");
}

/// The model of a cell at `level` with leaf coordinates `(i, j)` from the library's
/// `to_face_ij_orientation`, which 001's P-ENC-01 proves equal to the model's walk
/// (`TRUSTED_BASE.md`: reliance on 001, so the families avoid the 30-step walk).
fn model_at(c: Cell, level: u8) -> CellModel {
    let (face, i, j, orientation) = c.id().to_face_ij_orientation();
    let size = spec::size_ij(level);
    CellModel {
        face: face.as_u8(),
        level,
        orientation,
        i_lo: (i as u32) & !(size - 1),
        j_lo: (j as u32) & !(size - 1),
        size,
    }
}

/// P-CBND-01 at one level: the stored bounds are upstream's projection of the (i, j) range.
fn bound_model_at(level: u8) {
    let (c, _) = any_cell_at(level);
    let want = cell_spec::uv_bounds(model_at(c, level));
    let b = c.bound_uv();
    assert_eq!(b.x.lo.to_bits(), want[0][0].to_bits());
    assert_eq!(b.x.hi.to_bits(), want[0][1].to_bits());
    assert_eq!(b.y.lo.to_bits(), want[1][0].to_bits());
    assert_eq!(b.y.hi.to_bits(), want[1][1].to_bits());
}

/// P-CBND-02 at one level: the bounds are ordered and lie in [−1, 1].
fn bound_ordered_at(level: u8) {
    let (c, _) = any_cell_at(level);
    let b = c.bound_uv();
    assert!(b.x.lo < b.x.hi && b.y.lo < b.y.hi);
    assert!(-1.0 <= b.x.lo && b.x.hi <= 1.0 && -1.0 <= b.y.lo && b.y.hi <= 1.0);
}

/// P-CCHD-02 at one level (`< 30`): each child's bounds, which `children()` computes from the
/// parent's centre, are bit for bit those of the child built from its own identifier.
fn children_bounds_at(level: u8) {
    let (c, _) = any_cell_at(level);
    let kids = c.children().unwrap();
    for kid in kids {
        let direct = Cell::from_cell_id(kid.id()).bound_uv();
        let got = kid.bound_uv();
        assert_eq!(got.x.lo.to_bits(), direct.x.lo.to_bits());
        assert_eq!(got.x.hi.to_bits(), direct.x.hi.to_bits());
        assert_eq!(got.y.lo.to_bits(), direct.y.lo.to_bits());
        assert_eq!(got.y.hi.to_bits(), direct.y.hi.to_bits());
    }
}

/// P-CEQ-02 at one level: two cells built from the same identifier have the same bound bits.
fn same_bounds_at(level: u8) {
    let (a, _) = any_cell_at(level);
    let b = Cell::from_cell_id(a.id());
    let (ua, ub) = (a.bound_uv(), b.bound_uv());
    assert_eq!(ua.x.lo.to_bits(), ub.x.lo.to_bits());
    assert_eq!(ua.x.hi.to_bits(), ub.x.hi.to_bits());
    assert_eq!(ua.y.lo.to_bits(), ub.y.lo.to_bits());
    assert_eq!(ua.y.hi.to_bits(), ub.y.hi.to_bits());
}

/// One member per `name = level`, each `#[kani::proof]` with the given check.
macro_rules! level_family {
    ($check:ident, $what:literal; $($(#[$attr:meta])* $name:ident = $level:literal),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nFor every valid cell at level ", stringify!($level), " (one member of the `__l<L>` family; research R8).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(9)]
        fn $name() {
            $check($level);
        }
    )* };
}

level_family!(bound_model_at, "P-CBND-01 (exact output): the stored u-v bounds are bit for bit \
                               upstream's projection of the identifier's (i, j) range \
                               (`cell_spec::uv_bounds`; upstream `IJLevelToBoundUV`, `STtoUV`).";
    p_cbnd_01__l00 = 0, p_cbnd_01__l01 = 1, p_cbnd_01__l02 = 2, p_cbnd_01__l03 = 3,
    #[cfg(feature = "slow")] p_cbnd_01__l04 = 4, #[cfg(feature = "slow")] p_cbnd_01__l05 = 5, #[cfg(feature = "slow")] p_cbnd_01__l06 = 6, #[cfg(feature = "slow")] p_cbnd_01__l07 = 7,
    #[cfg(feature = "slow")] p_cbnd_01__l08 = 8, #[cfg(feature = "slow")] p_cbnd_01__l09 = 9, #[cfg(feature = "slow")] p_cbnd_01__l10 = 10, #[cfg(feature = "slow")] p_cbnd_01__l11 = 11,
    #[cfg(feature = "slow")] p_cbnd_01__l12 = 12,
);

level_family!(bound_ordered_at, "P-CBND-02 (validity): the bounds are ordered and lie in [−1, 1].";
    p_cbnd_02__l00 = 0, p_cbnd_02__l01 = 1, p_cbnd_02__l02 = 2, p_cbnd_02__l03 = 3,
    #[cfg(feature = "slow")] p_cbnd_02__l04 = 4, #[cfg(feature = "slow")] p_cbnd_02__l05 = 5, #[cfg(feature = "slow")] p_cbnd_02__l06 = 6, #[cfg(feature = "slow")] p_cbnd_02__l07 = 7,
    #[cfg(feature = "slow")] p_cbnd_02__l08 = 8, #[cfg(feature = "slow")] p_cbnd_02__l09 = 9, #[cfg(feature = "slow")] p_cbnd_02__l10 = 10, #[cfg(feature = "slow")] p_cbnd_02__l11 = 11,
    #[cfg(feature = "slow")] p_cbnd_02__l12 = 12,
);

level_family!(children_bounds_at, "P-CCHD-02 (exact output): each child's u-v bounds, which \
                                   `children()` computes from the parent's centre, are bit for \
                                   bit those of the child built from its own identifier \
                                   (upstream's `TestSubdivide`).";
    p_cchd_02__l00 = 0, p_cchd_02__l01 = 1, p_cchd_02__l02 = 2, p_cchd_02__l03 = 3,
    #[cfg(feature = "slow")] p_cchd_02__l04 = 4, #[cfg(feature = "slow")] p_cchd_02__l05 = 5, #[cfg(feature = "slow")] p_cchd_02__l06 = 6,
);

level_family!(same_bounds_at, "P-CEQ-02 (law): two cells built from the same identifier have \
                               the same u-v bound bits.";
    p_ceq_02__l00 = 0, p_ceq_02__l01 = 1, p_ceq_02__l02 = 2, p_ceq_02__l03 = 3,
    #[cfg(feature = "slow")] p_ceq_02__l04 = 4, #[cfg(feature = "slow")] p_ceq_02__l05 = 5, #[cfg(feature = "slow")] p_ceq_02__l06 = 6, #[cfg(feature = "slow")] p_ceq_02__l07 = 7,
    #[cfg(feature = "slow")] p_ceq_02__l08 = 8, #[cfg(feature = "slow")] p_ceq_02__l09 = 9, #[cfg(feature = "slow")] p_ceq_02__l10 = 10,
);
