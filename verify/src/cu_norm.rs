// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UNORM: normalisation (FR-005).
//!
//! Every family stops at two cells: normalising three cells costs about 14 GB (research R0).
//! The sibling merge and its cascades need four cells, so they are unreached here; the NORMALIZE
//! vectors (`normalize_matches_cpp`) and `prop_cell_union_normalize_preserves_coverage`
//! exercise them.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellUnion;

use crate::union_domain::{any_leaf, any_norm_union, any_valid_seq, out_covers, out_normalized};
use crate::union_spec;

/// Whether an output of at most `N` cells is normalised in the model.
fn normalised<const N: usize>(u: &CellUnion) -> bool {
    out_normalized::<N>(u.cell_ids())
}

macro_rules! p_unorm {
    ($($(#[$attr:meta])* $n01:ident, $n02:ident, $n03:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UNORM-01 (validity): `normalize`, `from_cell_ids` and `collect` return a ",
            "normalised union.\n\n",
            "`normalize(S)`, `from_cell_ids(S)` and `FromIterator` return a ",
            "`normalized_union` for every sequence of exactly ", stringify!($n), " valid cells ",
            "(`Normalize()` doc)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(4)]
        fn $n01() {
            let (v, _) = any_valid_seq::<$n>();
            assert!(normalised::<$n>(&CellUnion::from_cell_ids(v.clone())));
            let mut in_place = CellUnion::from_verbatim(v.clone());
            in_place.normalize();
            assert!(normalised::<$n>(&in_place));
            let collected: CellUnion = v.iter().copied().collect();
            assert!(normalised::<$n>(&collected));
            kani::cover!(v.len() == $n, "the domain is non-empty");
        }

        #[doc = concat!(
            "P-UNORM-02 (correct result): `normalize` covers exactly the leaves its input covers.\n\n",
            "`covers(normalize(S), x) ⇔ covers(S, x)` for every leaf `x` and every ",
            "sequence of exactly ", stringify!($n), " valid cells (`Normalize()` doc)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(4)]
        fn $n02() {
            let (v, ds) = any_valid_seq::<$n>();
            let u = CellUnion::from_cell_ids(v);
            let (leaf, x) = any_leaf();
            let covered = union_spec::covers(&ds, x);
            assert_eq!(out_covers::<$n>(u.cell_ids(), leaf), covered);
            kani::cover!(covered || $n == 0, "a covered leaf");
            kani::cover!(!covered, "an uncovered leaf");
        }

        #[doc = concat!(
            "P-UNORM-03 (law): normalising a normalised union changes nothing, so normalising ",
            "twice is the same as once.\n\n",
            "`normalize(U) = U` for every normalised union of exactly ",
            stringify!($n), " cells, so normalising twice changes nothing (`Normalize()`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(4)]
        fn $n03() {
            let (v, _) = any_norm_union::<$n>();
            let mut u = CellUnion::from_verbatim(v.clone());
            u.normalize();
            assert_eq!(u.cell_ids(), &v[..]);
            kani::cover!(v.len() == $n, "the domain is non-empty");
        }
    )* };
}

p_unorm!(
    p_unorm_01__n0,
    p_unorm_02__n0,
    p_unorm_03__n0 = 0,
    p_unorm_01__n1,
    p_unorm_02__n1,
    p_unorm_03__n1 = 1,
    // The two-cell members run in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_unorm_01__n2,
    p_unorm_02__n2,
    p_unorm_03__n2 = 2,
);

/// P-UNORM-04 (law): the order of the input cells does not change what `normalize` returns.
///
/// Swapping the two members of any sequence of two valid cells does not change `normalize(S)`
/// (`Normalize()` sorts first). Transpositions generate every permutation, so the order of the
/// input never matters (written argument, data-model §7); with at most one member there is nothing
/// to swap.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(4)]
fn p_unorm_04__n2() {
    let (v, _) = any_valid_seq::<2>();
    let swapped = vec![v[1], v[0]];
    assert_eq!(
        CellUnion::from_cell_ids(v.clone()),
        CellUnion::from_cell_ids(swapped)
    );
    kani::cover!(v[0] != v[1], "two different cells");
}
