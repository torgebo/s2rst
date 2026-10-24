// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UMEM: membership queries (FR-005), against the model's `contains_id`, `intersects_id`
//! and `covers` over valid unions (`ValidUnion`).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{Cell, CellUnion, Region};

use crate::domain::{any_valid, any_valid_at};
use crate::spec;
use crate::union_domain::any_valid_union;
use crate::union_spec;

macro_rules! p_umem_cell {
    ($($(#[$attr:meta])* $m1:ident, $m2:ident, $m3:ident, $m5:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UMEM-01 (correct result): `contains_cell_id(c)` holds exactly when one of the ",
            "union's cells contains `c`.\n\n",
            "`contains_cell_id(U, c) ⇔ contains_id(U, c)` for every valid union of ",
            "exactly ", stringify!($n), " cells and every valid cell (`Contains(S2CellId)`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(5)]
        fn $m1() {
            let (v, ds) = any_valid_union::<$n>();
            let (c, dc) = any_valid();
            let want = union_spec::contains_id(&ds, dc);
            assert_eq!(CellUnion::from_verbatim(v).contains_cell_id(c), want);
            kani::cover!(want || $n == 0, "contained");
            kani::cover!(!want, "not contained");
        }

        #[doc = concat!(
            "P-UMEM-02 (correct result): `intersects_cell_id(c)` holds exactly when one of the ",
            "union's cells overlaps `c`.\n\n",
            "`intersects_cell_id(U, c) ⇔ intersects_id(U, c)` for every valid union ",
            "of exactly ", stringify!($n), " cells and every valid cell (`Intersects(S2CellId)`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(5)]
        fn $m2() {
            let (v, ds) = any_valid_union::<$n>();
            let (c, dc) = any_valid();
            let want = union_spec::intersects_id(&ds, dc);
            assert_eq!(CellUnion::from_verbatim(v).intersects_cell_id(c), want);
            kani::cover!(want || $n == 0, "intersecting");
            kani::cover!(!want, "disjoint");
        }

        #[doc = concat!(
            "P-UMEM-03 (correct result): when `contains_cell_id(c)` holds, the union covers every ",
            "leaf of `c`.\n\n",
            "If `contains_cell_id(U, c)`, then `covers(U, x)` for every leaf `x` of ",
            "`c`, for every valid union of exactly ", stringify!($n), " cells (`Contains`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(5)]
        fn $m3() {
            let (v, ds) = any_valid_union::<$n>();
            let (c, dc) = any_valid();
            let (_, dx) = any_valid_at(30);
            kani::assume(spec::contains(dc, dx));
            let contained = CellUnion::from_verbatim(v).contains_cell_id(c);
            if contained {
                assert!(union_spec::covers(&ds, spec::leaf_index(dx)));
            }
            kani::cover!((contained && dc.level < 30) || $n == 0, "a contained cell above leaf level");
        }

        #[doc = concat!(
            "P-UMEM-05 (law): the `Region` methods `contains_cell` and `intersects_cell` answer ",
            "as the cell-id queries do on the cell's id.\n\n",
            "`Region::contains_cell` and `Region::intersects_cell` equal the model's ",
            "`contains_id` and `intersects_id` on the cell's id, for every valid union of exactly ",
            stringify!($n), " cells (`Contains/MayIntersect(S2Cell)`). Building the `Cell` needs ",
            "an unwind of 9 for `to_face_ij_orientation`."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(9)]
        fn $m5() {
            let (v, ds) = any_valid_union::<$n>();
            let (c, dc) = any_valid();
            let u = CellUnion::from_verbatim(v);
            let cell = Cell::from_cell_id(c);
            let (contains, intersects) =
                (union_spec::contains_id(&ds, dc), union_spec::intersects_id(&ds, dc));
            assert_eq!(u.contains_cell(&cell), contains);
            assert_eq!(u.intersects_cell(&cell), intersects);
            kani::cover!(contains || $n == 0, "contained");
            kani::cover!(!intersects, "disjoint");
        }
    )* };
}

p_umem_cell!(
    p_umem_01__n0,
    p_umem_02__n0,
    p_umem_03__n0,
    p_umem_05__n0 = 0,
    p_umem_01__n1,
    p_umem_02__n1,
    p_umem_03__n1,
    p_umem_05__n1 = 1,
    // The two- and three-cell members run in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_umem_01__n2,
    p_umem_02__n2,
    p_umem_03__n2,
    p_umem_05__n2 = 2,
    #[cfg(feature = "slow")]
    p_umem_01__n3,
    p_umem_02__n3,
    p_umem_03__n3,
    p_umem_05__n3 = 3,
);

macro_rules! p_umem_04 {
    ($($(#[$attr:meta])* $name:ident = ($na:literal, $nb:literal)),* $(,)?) => { $(
        #[doc = concat!(
            "P-UMEM-04 (correct result): `contains_union(b)` holds exactly when every cell of `b` ",
            "is contained in a cell of the union, and `intersects_union(b)` when some cell of `b` ",
            "overlaps one.\n\n",
            "`contains_union(A, B) ⇔ ∀ b ∈ B: contains_id(A, b)` and ",
            "`intersects_union(A, B) ⇔ ∃ b ∈ B: intersects_id(A, b)`, both in the model (with ",
            "upstream's caveat for unions that are not normalised), for valid unions of exactly ",
            stringify!($na), " and ", stringify!($nb), " cells (`Contains/Intersects(S2CellUnion)`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (va, da) = any_valid_union::<$na>();
            let (vb, db) = any_valid_union::<$nb>();
            let (a, b) = (CellUnion::from_verbatim(va), CellUnion::from_verbatim(vb));
            let contains = db.iter().all(|&x| union_spec::contains_id(&da, x));
            let intersects = db.iter().any(|&x| union_spec::intersects_id(&da, x));
            assert_eq!(a.contains_union(&b), contains);
            assert_eq!(a.intersects_union(&b), intersects);
            kani::cover!(contains || ($na == 0 && $nb > 0), "contained");
            kani::cover!(!intersects, "disjoint");
        }
    )* };
}

// Pairs with a union of two or three cells run in the weekly slow tier (T044).
p_umem_04!(
    p_umem_04__n0_n0 = (0, 0),
    p_umem_04__n0_n1 = (0, 1),
    #[cfg(feature = "slow")]
    p_umem_04__n0_n2 = (0, 2),
    #[cfg(feature = "slow")]
    p_umem_04__n0_n3 = (0, 3),
    p_umem_04__n1_n0 = (1, 0),
    p_umem_04__n1_n1 = (1, 1),
    #[cfg(feature = "slow")]
    p_umem_04__n1_n2 = (1, 2),
    #[cfg(feature = "slow")]
    p_umem_04__n1_n3 = (1, 3),
    #[cfg(feature = "slow")]
    p_umem_04__n2_n0 = (2, 0),
    #[cfg(feature = "slow")]
    p_umem_04__n2_n1 = (2, 1),
    #[cfg(feature = "slow")]
    p_umem_04__n2_n2 = (2, 2),
    #[cfg(feature = "slow")]
    p_umem_04__n2_n3 = (2, 3),
    #[cfg(feature = "slow")]
    p_umem_04__n3_n0 = (3, 0),
    #[cfg(feature = "slow")]
    p_umem_04__n3_n1 = (3, 1),
    #[cfg(feature = "slow")]
    p_umem_04__n3_n2 = (3, 2),
    #[cfg(feature = "slow")]
    p_umem_04__n3_n3 = (3, 3),
);
