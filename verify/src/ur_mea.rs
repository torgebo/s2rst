// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle measures (P-QMEA, spec 006 data-model §4.1): the centre and the size.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::uv_spec;
use crate::value_domain::any_finite_uv_rect;

/// P-QMEA-01 (correct result), the size: the two intervals' lengths, bit for bit, non-negative
/// for a non-empty rectangle.
///
/// For every `r` in `FiniteUvRect`: `r.size()` is `(x.hi − x.lo, y.hi − y.lo)`, bit for bit,
/// and if `r` is not empty both components are `≥ 0`.
#[kani::proof]
fn p_qmea_01__size() {
    let r = any_finite_uv_rect();
    let s = r.size();
    assert_eq!(s.x.to_bits(), (r.x.hi - r.x.lo).to_bits());
    assert_eq!(s.y.to_bits(), (r.y.hi - r.y.lo).to_bits());
    if !uv_spec::empty(r) {
        assert!(s.x >= 0.0 && s.y >= 0.0);
    }
    kani::cover!(s.x.is_infinite(), "a size that overflows");
}

/// P-QMEA-01 (correct result), the centre (weekly): the centre of a finite non-empty rectangle
/// lies in it. It took 421 s with the size clause per change (2026-10-14).
///
/// For every non-empty `r` in `FiniteUvRect`: `uv_spec::member(r, r.center())`.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_qmea_01__center() {
    let r = any_finite_uv_rect();
    kani::assume(!uv_spec::empty(r));
    assert!(uv_spec::member(r, r.center()));
    kani::cover!(
        r.x.lo > f64::MAX / 2.0,
        "a centre beyond f64::MAX / 2 (U-010)"
    );
}
