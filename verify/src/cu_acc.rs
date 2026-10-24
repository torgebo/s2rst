// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UACC: accessors and constructors that store their input (FR-005).
//!
//! `p_uacc_01__n<k>` is shared (data-model §3): over every sequence of exactly `k` ids, it
//! also discharges the crash obligations of `new`, `from_verbatim`, `num_cells`, `cell_ids`,
//! `is_empty`, `Deref` and both `IntoIterator` impls (`C-CellUnion::<op>`, FR-003).
//!
//! Covers sit only on paths every member reaches: Kani counts an unreachable cover as
//! unsatisfied, so the empty member is written out separately.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::{CellId, CellUnion};

use crate::union_domain::any_seq;

/// The P-UACC-01 assertions for a union built verbatim from `v`, whose raw ids are `raw`.
fn check_stored(v: &[CellId], raw: &[u64], u: &CellUnion) {
    assert_eq!(u.cell_ids().len(), raw.len());
    for (c, &x) in u.cell_ids().iter().zip(raw.iter()) {
        assert_eq!(c.0, x);
    }
    assert_eq!(&**u, v);
    assert!(u.into_iter().copied().eq(v.iter().copied()));
    assert!(u.clone().into_iter().eq(v.iter().copied()));
    assert_eq!(u.num_cells(), raw.len());
    assert_eq!(u.is_empty(), raw.is_empty());
}

/// P-UACC-01 (correct result): `new()` is empty; `from_verbatim(v)` stores exactly `v`;
/// `cell_ids()`, dereferencing and iterating give those ids in order; `num_cells()` counts them.
///
/// For the empty union, with `new` and the derived `Default`: both are empty, and
/// `from_verbatim(vec![])` stores nothing. Also discharges `C-CellUnion::new` (whole domain)
/// and the accessor obligations for `Seq(0)`.
#[kani::proof]
#[kani::unwind(3)]
fn p_uacc_01__n0() {
    let (v, raw) = any_seq::<0>();
    let u = CellUnion::from_verbatim(v.clone());
    check_stored(&v, &raw, &u);
    assert!(CellUnion::new().is_empty());
    assert_eq!(CellUnion::default(), CellUnion::new());
    assert_eq!(u, CellUnion::new());
    kani::cover!(u.is_empty(), "empty union");
}

macro_rules! p_uacc_01 {
    ($($name:ident = $n:literal),* $(,)?) => { $(
        /// P-UACC-01 (correct result): `new()` is empty; `from_verbatim(v)` stores exactly `v`;
        /// `cell_ids()`, dereferencing and iterating give those ids in order; `num_cells()` counts
        /// them.
        ///
        /// `from_verbatim(v)` stores exactly `v`; `cell_ids`, `Deref` and both `into_iter`s yield
        /// exactly the stored ids in order; `num_cells` is their count and `is_empty` holds iff it
        /// is 0 (`cell_ids()`, `size()`, `empty()`, `FromVerbatim`). Also discharges the crash
        /// obligations of those operations for `Seq(n)`, `n = k`.
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            let (v, raw) = any_seq::<$n>();
            let u = CellUnion::from_verbatim(v.clone());
            check_stored(&v, &raw, &u);
            kani::cover!(CellId(raw[0]).is_valid(), "valid first id");
            kani::cover!(!CellId(raw[0]).is_valid(), "invalid first id");
        }
    )* };
}

p_uacc_01!(p_uacc_01__n1 = 1, p_uacc_01__n2 = 2, p_uacc_01__n3 = 3);
