// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval membership of a number (P-RMEM, spec 005 data-model §4.1), against the
//! real-set description (`real_spec`).

use crate::real_spec;
use crate::value_domain::{any_number, any_real};

/// P-RMEM-01 (correct result): a number is in an interval exactly when it lies between the
/// bounds.
///
/// For every `x` in `Real` and non-NaN `p`: `x.contains(p) == real_spec::member(x, p)`.
#[kani::proof]
fn p_rmem_01() {
    let (x, p) = (any_real(), any_number());
    let c = x.contains(p);
    assert_eq!(c, real_spec::member(x, p));
    kani::cover!(c && p == x.lo, "the low bound");
    kani::cover!(c && p.is_infinite(), "an infinite member");
}

/// P-RMEM-02 (correct result): a number is in the interior exactly when it lies strictly
/// between the bounds.
///
/// For every `x` in `Real` and non-NaN `p`: `x.interior_contains(p) == real_spec::interior(x, p)`.
#[kani::proof]
fn p_rmem_02() {
    let (x, p) = (any_real(), any_number());
    let i = x.interior_contains(p);
    assert_eq!(i, real_spec::interior(x, p));
    kani::cover!(i, "in the interior");
    kani::cover!(!i && x.contains(p), "on the boundary");
}
