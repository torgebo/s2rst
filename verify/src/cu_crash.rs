// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-CellUnion::<op>` for the in-scope operations (FR-003).
//!
//! Each harness draws its inputs from the operation's domain in `cell_union/INVENTORY.md`
//! with a concrete union length (`__n<k>`, research R4), assumes exactly the documented
//! preconditions, and calls the public API. Kani checks every panic, arithmetic overflow and
//! undefined behaviour on the way; the covers show that the domain is non-empty and that the
//! interesting outcomes are reachable (FR-008). Each family member gets its own covers,
//! because Kani counts a cover on an unreachable path as unsatisfied.
//!
//! The accessors, the invariant checks and the encoding discharge their crash obligations in
//! shared property harnesses (`cu_acc`, `cu_inv`, `cu_enc`; data-model §3).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{Cell, CellId, CellUnion, Region};

use crate::domain::{any_level, any_valid};
use crate::spec::{self, Decomposed};
use crate::union_domain::{any_level_mod, any_valid_seq, any_valid_union};
use crate::union_spec;

// ---------------------------------------------------------------------------------------------
// Normalising operations over `ValidSeq` (T024). Three cells cost about 14 GB (research R0),
// so the families stop at two: the sibling merge (four cells) is unreached.
// ---------------------------------------------------------------------------------------------

/// The four ways two valid cells can meet in the normalising loop.
fn cover_two(v: &[CellId], ds: &[Decomposed], out: &CellUnion) {
    let (a, b) = (v[0], v[1]);
    let a_in_b = a != b && spec::contains(ds[1], ds[0]);
    let b_in_a = a != b && spec::contains(ds[0], ds[1]);
    kani::cover!(a == b, "duplicate removed");
    // Sorted by id, the contained cell comes first when its id is smaller: the `pop` path.
    kani::cover!(
        (a_in_b && a < b) || (b_in_a && b < a),
        "contained cell first: discarded by the container"
    );
    kani::cover!(
        (a_in_b && b < a) || (b_in_a && a < b),
        "containing cell first: contained cell skipped"
    );
    kani::cover!(out.num_cells() == 2, "disjoint cells kept");
}

macro_rules! normalising {
    ($op:literal, |$v:ident| $build:expr; $($name:ident = $n:literal => $covers:ident),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::", $op, ": completes without panic, overflow or undefined behaviour ",
            "for every sequence of exactly ", stringify!($n), " valid cells, in any order, ",
            "possibly overlapping or repeated (`ValidSeq`; upstream `Normalize` requires valid ",
            "ids)."
        )]
        #[kani::proof]
        #[kani::unwind(3)]
        fn $name() {
            let ($v, ds) = any_valid_seq::<$n>();
            let input = $v.clone();
            let out: CellUnion = $build;
            $covers(&input, &ds, &out);
        }
    )* };
}

normalising!("from_cell_ids", |v| CellUnion::from_cell_ids(v);
    c_from_cell_ids__n2 = 2 => cover_two,
);

normalising!("normalize", |v| {
        let mut u = CellUnion::from_verbatim(v);
        u.normalize();
        u
    };
    c_normalize__n2 = 2 => cover_two,
);

normalising!("from_iter (FromIterator<CellId>)", |v| v.into_iter().collect::<CellUnion>();
    c_from_iter__n2 = 2 => cover_two,
);

// ---------------------------------------------------------------------------------------------
// Queries and the measure over `ValidUnion` (T025).
// ---------------------------------------------------------------------------------------------

macro_rules! cell_query {
    ($unwind:literal, $op:literal, |$u:ident, $c:ident| $call:expr;
     $($name:ident = $n:literal => |$r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::", $op, ": completes without panic, overflow or undefined behaviour ",
            "for every valid union of exactly ", stringify!($n), " cells (`ValidUnion`) and ",
            "every valid cell (the `debug_assert!` precondition)."
        )]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let $u = CellUnion::from_verbatim(v);
            let ($c, _) = any_valid();
            let $r: bool = $call;
            $covers
        }
    )* };
}

cell_query!(5, "contains_cell_id", |u, c| u.contains_cell_id(c);
    c_contains_cell_id__n2 = 2 => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_cell_id__n3 = 3 => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
);

cell_query!(5, "intersects_cell_id", |u, c| u.intersects_cell_id(c);
    c_intersects_cell_id__n2 = 2 => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_cell_id__n3 = 3 => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
);

// Building the `Cell` runs `to_face_ij_orientation`, whose lookup loop needs an unwind of 9
// (as in 001).
cell_query!(9, "contains_cell (Region)", |u, c| u.contains_cell(&Cell::from_cell_id(c));
    c_region_contains_cell__n2 = 2 => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_region_contains_cell__n3 = 3 => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
);

cell_query!(9, "intersects_cell (Region)", |u, c| u.intersects_cell(&Cell::from_cell_id(c));
    c_region_intersects_cell__n2 = 2 => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_region_intersects_cell__n3 = 3 => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
);

macro_rules! union_query {
    ($op:literal, |$a:ident, $b:ident| $call:expr;
     $($name:ident = ($na:literal, $nb:literal) => |$r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::", $op, ": completes without panic, overflow or undefined behaviour ",
            "for every pair of valid unions of exactly ", stringify!($na), " and ",
            stringify!($nb), " cells (`ValidUnion` × `ValidUnion`)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (va, _) = any_valid_union::<$na>();
            let (vb, _) = any_valid_union::<$nb>();
            let ($a, $b) = (CellUnion::from_verbatim(va), CellUnion::from_verbatim(vb));
            let $r: bool = $call;
            $covers
        }
    )* };
}

union_query!("contains_union", |a, b| a.contains_union(&b);
    c_contains_union__n0_n2 = (0, 2) => |r| { kani::cover!(!r, "the empty union contains nothing"); },
    c_contains_union__n0_n3 = (0, 3) => |r| { kani::cover!(!r, "the empty union contains nothing"); },
    c_contains_union__n1_n2 = (1, 2) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n1_n3 = (1, 3) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n2_n0 = (2, 0) => |r| { kani::cover!(r, "every union contains the empty one"); },
    c_contains_union__n2_n1 = (2, 1) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n2_n2 = (2, 2) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n2_n3 = (2, 3) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n3_n0 = (3, 0) => |r| { kani::cover!(r, "every union contains the empty one"); },
    c_contains_union__n3_n1 = (3, 1) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n3_n2 = (3, 2) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
    c_contains_union__n3_n3 = (3, 3) => |r| { kani::cover!(r, "contained"); kani::cover!(!r, "not contained"); },
);

union_query!("intersects_union", |a, b| a.intersects_union(&b);
    c_intersects_union__n0_n2 = (0, 2) => |r| { kani::cover!(!r, "nothing intersects the empty union"); },
    c_intersects_union__n0_n3 = (0, 3) => |r| { kani::cover!(!r, "nothing intersects the empty union"); },
    c_intersects_union__n1_n2 = (1, 2) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n1_n3 = (1, 3) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n2_n0 = (2, 0) => |r| { kani::cover!(!r, "nothing intersects the empty union"); },
    c_intersects_union__n2_n1 = (2, 1) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n2_n2 = (2, 2) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n2_n3 = (2, 3) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n3_n0 = (3, 0) => |r| { kani::cover!(!r, "nothing intersects the empty union"); },
    c_intersects_union__n3_n1 = (3, 1) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n3_n2 = (3, 2) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
    c_intersects_union__n3_n3 = (3, 3) => |r| { kani::cover!(r, "intersecting"); kani::cover!(!r, "disjoint"); },
);

macro_rules! leaf_count {
    ($($name:ident = $n:literal => |$ds:ident, $r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::leaf_cells_covered: completes without panic, overflow or undefined ",
            "behaviour for every valid union of exactly ", stringify!($n), " cells ",
            "(`ValidUnion`, the upstream precondition; FR-016)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            let (v, $ds) = any_valid_union::<$n>();
            let $r = CellUnion::from_verbatim(v).leaf_cells_covered();
            $covers
        }
    )* };
}

leaf_count!(
    c_leaf_cells_covered__n0 = 0 => |_ds, r| { kani::cover!(r == 0, "empty union"); },
    c_leaf_cells_covered__n1 = 1 => |ds, r| {
        kani::cover!(ds[0].level == 0, "a face cell");
        kani::cover!(r == 1, "a leaf");
    },
    c_leaf_cells_covered__n2 = 2 => |ds, r| {
        kani::cover!(ds[0].level == 0 && ds[1].level == 0, "two face cells");
        kani::cover!(r == 2, "two leaves");
    },
    c_leaf_cells_covered__n3 = 3 => |ds, r| {
        kani::cover!(ds.iter().all(|d| d.level == 0), "three face cells");
        kani::cover!(r == 3, "three leaves");
    },
);

// ---------------------------------------------------------------------------------------------
// Set operations over `ValidUnion` (T026).
// ---------------------------------------------------------------------------------------------

macro_rules! set_op {
    ($op:literal, |$a:ident, $b:ident| $call:expr;
     $($(#[$attr:meta])* $name:ident = ($na:literal, $nb:literal) => |$r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::", $op, ": completes without panic, overflow or undefined behaviour ",
            "for every pair of valid unions of exactly ", stringify!($na), " and ",
            stringify!($nb), " cells (`ValidUnion` × `ValidUnion`)."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(3)]
        fn $name() {
            let (va, _) = any_valid_union::<$na>();
            let (vb, _) = any_valid_union::<$nb>();
            let ($a, $b) = (CellUnion::from_verbatim(va), CellUnion::from_verbatim(vb));
            let $r: CellUnion = $call;
            $covers
        }
    )* };
}

set_op!("union", |a, b| a.union(&b);
    c_union__n1_n1 = (1, 1) => |r| {
        kani::cover!(r.num_cells() == 1, "one contains the other, or equal");
        kani::cover!(r.num_cells() == 2, "disjoint");
    },
);

set_op!("intersection", |a, b| a.intersection(&b);
    // `1 × 1` runs in the weekly slow tier, for the per-change budget (T046).
    #[cfg(feature = "slow")]
    c_intersection__n1_n1 = (1, 1) => |r| {
        kani::cover!(r.is_empty(), "disjoint");
        kani::cover!(r.num_cells() == 1, "nested");
    },
);

macro_rules! intersection_with_cell {
    ($($name:ident = $n:literal => |$c:ident, $r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::intersection_with_cell_id: completes without panic, overflow or ",
            "undefined behaviour for every valid union of exactly ", stringify!($n), " cells ",
            "(`ValidUnion`) and every valid cell (the `debug_assert!` precondition)."
        )]
        #[kani::proof]
        #[kani::unwind(4)]
        fn $name() {
            let (v, _) = any_valid_union::<$n>();
            let u = CellUnion::from_verbatim(v);
            let ($c, _) = any_valid();
            let $r = u.intersection_with_cell_id($c);
            $covers
        }
    )* };
}

intersection_with_cell!(
    c_intersection_with_cell_id__n3 = 3 => |c, r| {
        kani::cover!(r.cell_ids() == [c], "the cell itself (contained)");
        kani::cover!(r.num_cells() == 3, "three smaller members");
        kani::cover!(r.is_empty(), "disjoint");
    },
);

// ---------------------------------------------------------------------------------------------
// Denormalisation (T027). Loop bound: each cell is replaced by at most 16 descendants (a level
// gap of at most 2); larger gaps are the uncovered part, exercised by the unit, differential
// and property tests.
// ---------------------------------------------------------------------------------------------

macro_rules! denormalize {
    ($($(#[$attr:meta])* $name:ident = $n:literal => |$ds:ident, $m:ident, $k:ident, $r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "C-CellUnion::denormalize: completes without panic, overflow or undefined ",
            "behaviour for every valid union of exactly ", stringify!($n), " cells ",
            "(`ValidUnion`), every `min_level` and every `level_mod` in 1–3 (the ",
            "`debug_assert!` and upstream `DCHECK`), within the loop bound: each cell's new ",
            "level is at most 2 below its own, so it has at most 16 descendants."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(17)]
        fn $name() {
            let (v, $ds) = any_valid_union::<$n>();
            let $m = any_level();
            let $k = any_level_mod();
            for d in &$ds {
                kani::assume(union_spec::denorm_level(d.level, $m.as_u8(), $k) - d.level <= 2);
            }
            let $r = CellUnion::from_verbatim(v).denormalize($m, $k);
            $covers
        }
    )* };
}

denormalize!(
    c_denormalize__n1 = 1 => |ds, m, k, r| {
        kani::cover!(r.num_cells() == 1, "the cell kept");
        kani::cover!(r.num_cells() > 1, "replaced by its descendants");
        kani::cover!(
            ds[0].level == 29 && m.as_u8() == 1 && k == 3,
            "rounded up past 30 and clamped (FR-015)"
        );
    },
    // Two cells (147 s) runs in the weekly slow tier, for the per-change budget (T046).
    #[cfg(feature = "slow")]
    c_denormalize__n2 = 2 => |ds, m, k, r| {
        kani::cover!(r.num_cells() == 2, "both cells kept");
        kani::cover!(r.num_cells() > 2, "replaced by descendants");
    },
);
