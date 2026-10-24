// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-USET: set algebra (FR-004, FR-005), against the model's `covers` for a symbolic leaf.
//! `difference` (P-USET-04) is a tool-limit exclusion and has no harness.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellUnion;

use crate::domain::any_valid;
use crate::spec;
use crate::union_domain::{any_leaf, any_valid_union, out_covers, out_normalized};
use crate::union_spec;

macro_rules! p_uset_pair {
    ($($(#[$attr:meta])* $s1:ident, $s2:ident = ($na:literal, $nb:literal)),* $(,)?) => { $(
        #[doc = concat!(
            "P-USET-01 (correct result): `union(a, b)` covers exactly the leaves that `a` or `b` ",
            "covers, and is normalised.\n\n",
            "`union(A, B)` is normalised and covers exactly `covers(A, x) ∨ ",
            "covers(B, x)`, for valid unions of exactly ", stringify!($na), " and ",
            stringify!($nb), " cells (`Union()`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(3)]
        fn $s1() {
            let (va, da) = any_valid_union::<$na>();
            let (vb, db) = any_valid_union::<$nb>();
            let r = CellUnion::from_verbatim(va).union(&CellUnion::from_verbatim(vb));
            let (leaf, x) = any_leaf();
            let want = union_spec::covers(&da, x) || union_spec::covers(&db, x);
            assert_eq!(out_covers::<{ $na + $nb }>(r.cell_ids(), leaf), want);
            assert!(out_normalized::<{ $na + $nb }>(r.cell_ids()));
            kani::cover!(want || ($na == 0 && $nb == 0), "a covered leaf");
        }

        #[doc = concat!(
            "P-USET-02 (correct result): `intersection(a, b)` covers exactly the leaves both ",
            "cover; it is valid, and normalised when both inputs are.\n\n",
            "`intersection(A, B)` covers exactly `covers(A, x) ∧ covers(B, x)` and is ",
            "valid, and normalised because both inputs are (every valid union of at most three ",
            "cells is normalised), for valid unions of exactly ", stringify!($na), " and ",
            stringify!($nb), " cells (`Intersection()`, `GetIntersection`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(3)]
        fn $s2() {
            let (va, da) = any_valid_union::<$na>();
            let (vb, db) = any_valid_union::<$nb>();
            let r = CellUnion::from_verbatim(va).intersection(&CellUnion::from_verbatim(vb));
            let (leaf, x) = any_leaf();
            let want = union_spec::covers(&da, x) && union_spec::covers(&db, x);
            assert_eq!(out_covers::<{ $na + $nb }>(r.cell_ids(), leaf), want);
            assert!(out_normalized::<{ $na + $nb }>(r.cell_ids()));
            kani::cover!(want || $na == 0 || $nb == 0, "a leaf in both");
        }
    )* };
}

p_uset_pair!(
    p_uset_01__n0_n0,
    p_uset_02__n0_n0 = (0, 0),
    p_uset_01__n0_n1,
    p_uset_02__n0_n1 = (0, 1),
    p_uset_01__n1_n0,
    p_uset_02__n1_n0 = (1, 0),
    // The `1 × 1` members run in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_uset_01__n1_n1,
    p_uset_02__n1_n1 = (1, 1),
);

macro_rules! p_uset_03 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-USET-03 (correct result): `intersection_with_cell_id(c)` covers exactly the ",
            "union's leaves that lie inside `c`, and is normalised when the union is.\n\n",
            "`intersection_with_cell_id(U, c)` covers exactly `covers(U, x) ∧ x ∈ ",
            "range(c)` and is normalised (U is: every valid union of at most three cells is), ",
            "for every valid union of exactly ", stringify!($n), " cells and every valid cell ",
            "(`Intersection(S2CellId)`)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (v, ds) = any_valid_union::<$n>();
            let (c, dc) = any_valid();
            let r = CellUnion::from_verbatim(v).intersection_with_cell_id(c);
            let (leaf, x) = any_leaf();
            let (lo, hi) = spec::range(dc);
            let want = union_spec::covers(&ds, x) && lo <= x && x <= hi;
            assert_eq!(out_covers::<{ $n + 1 }>(r.cell_ids(), leaf), want);
            assert!(out_normalized::<{ $n + 1 }>(r.cell_ids()));
            kani::cover!(want || $n == 0, "a leaf in both");
        }
    )* };
}

// Three cells run out of memory at 4 GB (2026-10-04), so the property stops at two; the crash
// family `c_intersection_with_cell_id` reaches three.
p_uset_03!(p_uset_03__n0 = 0, p_uset_03__n1 = 1, p_uset_03__n2 = 2);
