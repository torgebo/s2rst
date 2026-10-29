// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `CellUnion`'s point test (spec 011 data-model §4.2): upstream's `Contains(S2CellId(p))`.

use s2rst::s2::{CellId, CellUnion, Region};

use crate::union_domain::any_valid_union;
use crate::value_domain::any_triple;

macro_rules! p_upt_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UPT-01 (correct result): the union contains a point exactly when it contains the ",
            "point's leaf cell, through both `contains_point` and `Region::contains_point`; with ",
            "002's P-UMEM-01 to -03 (`contains_cell_id` is the model's), the point test is ",
            "`union_spec::contains_id` of the leaf.\n\n",
            "Exactly: for every triple `p` and every valid union of exactly ", stringify!($n),
            " cells, `contains_point(p) == contains_cell_id(CellId::from_point(&p))`, and the ",
            "same for `Region::contains_point`.\n\nUnder contract: `pure_leaf` (L-PR-01; one ",
            "leaf per point, so the statement reads the library's)."
        )]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::CellId::from_point, crate::stubs::pure_leaf)]
        fn $name() {
            crate::stubs::reset();
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let p = any_triple();
            let leaf = CellId::from_point(&p);
            let x = u.contains_cell_id(leaf);
            assert!(u.contains_point(p) == x);
            assert!(Region::contains_point(&u, &p) == x);
            kani::cover!(x || $n == 0, "a contained point");
        }
    )* };
}

p_upt_01!(
    p_upt_01__n0 = 0,
    p_upt_01__n1 = 1,
    p_upt_01__n2 = 2,
    p_upt_01__n3 = 3,
);
