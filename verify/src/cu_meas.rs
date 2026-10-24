// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UMEAS: the leaf-cell count (FR-005, FR-016). For valid unions of any size the bound
//! `6·4^30 < 2^63` follows from disjointness (the written argument in `TRUSTED_BASE.md`).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellUnion;

use crate::spec;
use crate::union_domain::any_valid_union;
use crate::union_spec;

macro_rules! p_umeas_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UMEAS-01 (correct result): `leaf_cells_covered()` is the number of leaves the ",
            "union covers, and never overflows.\n\n",
            "`leaf_cells_covered(U) = leaf_count(U)`, which is at most `6·4^30` and ",
            "never overflows, for every valid union of exactly ", stringify!($n), " cells ",
            "(`LeafCellsCovered()`; FR-016)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (v, ds) = any_valid_union::<$n>();
            let got = CellUnion::from_verbatim(v).leaf_cells_covered();
            let want = union_spec::leaf_count(&ds);
            assert!(got >= 0);
            assert_eq!(got as u128, want);
            assert!(want <= 6 * u128::from(spec::pow4(30)));
            kani::cover!(want == u128::from(spec::pow4(30)) * $n, "all face cells");
        }
    )* };
}

p_umeas_01!(
    p_umeas_01__n0 = 0,
    p_umeas_01__n1 = 1,
    p_umeas_01__n2 = 2,
    p_umeas_01__n3 = 3
);
