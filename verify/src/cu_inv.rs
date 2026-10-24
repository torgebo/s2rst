// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UINV: the validity and normalisation checks (FR-004, FR-005).
//!
//! Both families range over every sequence of exactly `k` ids, valid or not (`Seq`), and are
//! shared (data-model §3): they also discharge `C-CellUnion::is_valid` and
//! `C-CellUnion::is_normalized`. The model side runs `spec::decompose`, a 31-level search,
//! on every symbolic id, so the unwind covers that constant loop.
//!
//! Each member's covers are passed in, so that every cover sits on a path the member
//! reaches (Kani counts an unreachable cover as unsatisfied).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{CellId, CellUnion};

use crate::union_domain::any_seq;
use crate::union_spec;

/// Covers for the first two ids of a sequence: an invalid id, and two valid ids that are
/// unsorted, nested or equal. They use the implementation's loop-free `CellId` methods:
/// covers only show reachability, and the model's `decompose` is a 31-level search.
fn cover_pair(a: u64, b: u64) {
    let (ca, cb) = (CellId(a), CellId(b));
    let both = ca.is_valid() && cb.is_valid();
    kani::cover!(!ca.is_valid(), "invalid id");
    kani::cover!(both && cb.range_max() < ca.range_min(), "unsorted");
    kani::cover!(both && a != b && ca.contains(cb), "overlapping");
    kani::cover!(both && a == b, "duplicate");
}

macro_rules! p_uinv_01 {
    ($($(#[$attr:meta])* $name:ident = $k:literal => |$raw:ident, $want:ident| $covers:block),* $(,)?) => { $(
        /// P-UINV-01 (validity): `is_valid()` holds exactly for lists of valid cells in curve order
        /// that do not overlap.
        ///
        /// `is_valid(U) ⇔ valid_union(U)` for every sequence of exactly `k` ids (`IsValid()`). Also
        /// discharges `C-CellUnion::is_valid` for `Seq(n)`, `n = k`.
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(32)]
        fn $name() {
            let (v, $raw) = any_seq::<$k>();
            let got = CellUnion::from_verbatim(v).is_valid();
            let $want = union_spec::valid_union(&$raw);
            assert_eq!(got, $want);
            $covers
        }
    )* };
}

macro_rules! p_uinv_02 {
    ($($(#[$attr:meta])* $name:ident = $k:literal => |$raw:ident, $want:ident| $covers:block),* $(,)?) => { $(
        /// P-UINV-02 (validity): `is_normalized()` holds exactly for valid unions in which no four
        /// consecutive cells are the children of one parent.
        ///
        /// `is_normalized(U) ⇔ normalized_union(U)` for every sequence of exactly `k` ids
        /// (`IsNormalized()`). Also discharges `C-CellUnion::is_normalized` for `Seq(n)`, `n = k`.
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(32)]
        fn $name() {
            let (v, $raw) = any_seq::<$k>();
            let got = CellUnion::from_verbatim(v).is_normalized();
            let $want = union_spec::normalized_union(&$raw);
            assert_eq!(got, $want);
            $covers
        }
    )* };
}

p_uinv_01!(
    p_uinv_01__n0 = 0 => |_raw, want| { kani::cover!(want, "the empty union is valid"); },
    p_uinv_01__n1 = 1 => |_raw, want| {
        kani::cover!(want, "valid");
        kani::cover!(!want, "invalid id");
    },
    p_uinv_01__n2 = 2 => |raw, want| {
        kani::cover!(want, "valid");
        cover_pair(raw[0], raw[1]);
    },
    // Three and four ids run in the weekly slow tier, for the per-change budget (T046).
    #[cfg(feature = "slow")]
    p_uinv_01__n3 = 3 => |raw, want| {
        kani::cover!(want, "valid");
        cover_pair(raw[1], raw[2]);
    },
    #[cfg(feature = "slow")]
    p_uinv_01__n4 = 4 => |raw, want| {
        kani::cover!(want, "valid");
        cover_pair(raw[2], raw[3]);
    },
);

p_uinv_02!(
    p_uinv_02__n0 = 0 => |_raw, want| { kani::cover!(want, "the empty union is normalised"); },
    p_uinv_02__n1 = 1 => |_raw, want| {
        kani::cover!(want, "normalised");
        kani::cover!(!want, "invalid id");
    },
    p_uinv_02__n2 = 2 => |raw, want| {
        kani::cover!(want, "normalised");
        cover_pair(raw[0], raw[1]);
    },
    // Three and four ids (the sibling check) run in the weekly slow tier (T046).
    #[cfg(feature = "slow")]
    p_uinv_02__n3 = 3 => |raw, want| {
        kani::cover!(want, "normalised");
        cover_pair(raw[1], raw[2]);
    },
    #[cfg(feature = "slow")]
    p_uinv_02__n4 = 4 => |raw, want| {
        kani::cover!(want, "normalised");
        kani::cover!(
            union_spec::valid_union(&raw) && !want,
            "four siblings: valid but not normalised"
        );
        cover_pair(raw[0], raw[1]);
    },
);
