// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UDEN: denormalisation (FR-005, FR-015). Loop bound: each cell's new level is at most two
//! below its own, so it has at most 16 descendants; larger gaps and more than one cell are the
//! uncovered part, exercised by `denormalize_clamps_to_max_level`, `denormalize_matches_cpp`
//! and `prop_cell_union_denormalize_preserves_coverage`.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::coords::Level;
use s2rst::s2::{CellUnion, LevelMod};

use crate::domain::any_level;
use crate::spec::Decomposed;
use crate::union_domain::{
    any_leaf, any_level_mod, any_valid_union, out_covers, out_ids_equal, out_valid,
};
use crate::union_spec;

/// A valid union of exactly `N` cells, a `min_level` and a `level_mod` in 1–3, within the
/// loop bound (a level gap of at most 2 per cell).
fn inputs<const N: usize>() -> (CellUnion, [Decomposed; N], Level, u8) {
    let (v, ds) = any_valid_union::<N>();
    let m = any_level();
    let k = any_level_mod();
    for d in &ds {
        kani::assume(union_spec::denorm_level(d.level, m.as_u8(), k) - d.level <= 2);
    }
    (CellUnion::from_verbatim(v), ds, m, k)
}

macro_rules! p_uden {
    ($($(#[$attr:meta])* $d1:ident, $d1v:ident, $d2:ident = $n:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-UDEN-01 (correct result): `denormalize(min_level, level_mod)` returns exactly what ",
            "upstream's algorithm returns, and the result is valid.\n\n",
            "First part: `denormalize(U, m, k)` equals the model's ",
            "`denormalize(U, m, k)`, the C++ algorithm with its clamp (FR-015), for every valid ",
            "union of exactly ", stringify!($n), " cells within the loop bound (`Denormalize`). ",
            "The property is split by conjunct: one harness for each did not fit 4 GB together."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(17)]
        fn $d1() {
            let (u, ds, m, k) = inputs::<$n>();
            let out = u.denormalize(m, LevelMod::new(k));
            let want = union_spec::denormalize(&ds, m.as_u8(), k);
            assert!(out_ids_equal::<{ 16 * $n }>(out.cell_ids(), &want));
            kani::cover!(want.len() > $n || $n == 0, "a cell replaced by its descendants");
            kani::cover!(
                ds.first().is_some_and(|d| d.level == 29) && m.as_u8() == 1 && k == 3 || $n == 0,
                "rounded up past 30 and clamped (FR-015)"
            );
        }

        #[doc = concat!(
            "P-UDEN-01 (correct result): `denormalize(min_level, level_mod)` returns exactly what ",
            "upstream's algorithm returns, and the result is valid.\n\n",
            "Second part: `denormalize(U, m, k)` is a valid union (FR-004), for every ",
            "valid union of exactly ", stringify!($n), " cells within the loop bound."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(17)]
        fn $d1v() {
            let (u, _, m, k) = inputs::<$n>();
            let out = u.denormalize(m, LevelMod::new(k));
            assert!(out_valid::<{ 16 * $n }>(out.cell_ids()));
            kani::cover!(out.num_cells() > $n || $n == 0, "a cell replaced by its descendants");
        }

        #[doc = concat!(
            "P-UDEN-02 (correct result): every cell `denormalize` returns is at `min_level` or ",
            "finer, a multiple of `level_mod` levels below it (or a leaf), and together they ",
            "cover exactly what the input covers.\n\n",
            "Every member of `denormalize(U, m, k)` has level `≥ m` and ",
            "`(level − m) mod k = 0` unless its level is 30, and it covers exactly what `U` ",
            "covers, for every valid union of exactly ", stringify!($n), " cells within the ",
            "loop bound (`Denormalize` doc)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(17)]
        fn $d2() {
            let (u, ds, m, k) = inputs::<$n>();
            let out = u.denormalize(m, LevelMod::new(k));
            let ids = out.cell_ids();
            assert!(ids.len() <= 16 * $n);
            for i in 0..16 * $n {
                if i < ids.len() {
                    let l = ids[i].level().as_u8();
                    assert!(l >= m.as_u8());
                    assert!(l == 30 || (l - m.as_u8()) % k == 0);
                }
            }
            let (leaf, x) = any_leaf();
            assert_eq!(out_covers::<{ 16 * $n }>(ids, leaf), union_spec::covers(&ds, x));
            kani::cover!(out.num_cells() > $n || $n == 0, "a cell replaced by its descendants");
        }
    )* };
}

p_uden!(
    p_uden_01__n0_ids,
    p_uden_01__n0_valid,
    p_uden_02__n0 = 0,
    // The one-cell members run in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_uden_01__n1_ids,
    p_uden_01__n1_valid,
    p_uden_02__n1 = 1,
);
