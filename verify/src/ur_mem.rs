// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle membership (P-QMEM, spec 006 data-model §4.1), against the product of the
//! two real sets (`uv_spec`).

use crate::uv_spec;
use crate::value_domain::{any_uv_point, any_uv_rect};

/// P-QMEM-01 (correct result): a point is in a rectangle exactly when each coordinate is in its
/// interval.
///
/// For every `r` in `UvRect` and `p` in `UvPoint`: `r.contains_point(p) == uv_spec::member(r, p)`.
#[kani::proof]
fn p_qmem_01() {
    let (r, p) = (any_uv_rect(), any_uv_point());
    let c = r.contains_point(p);
    assert_eq!(c, uv_spec::member(r, p));
    kani::cover!(c && (p.x == r.x.lo || p.y == r.y.hi), "on the boundary");
    kani::cover!(!c && !r.is_empty(), "outside");
}

/// P-QMEM-02 (correct result): interior membership is the product of the open intervals.
///
/// For every `r` in `UvRect` and `p` in `UvPoint`:
/// `r.interior_contains_point(p) == uv_spec::interior(r, p)`.
#[kani::proof]
fn p_qmem_02() {
    let (r, p) = (any_uv_rect(), any_uv_point());
    let c = r.interior_contains_point(p);
    assert_eq!(c, uv_spec::interior(r, p));
    kani::cover!(c, "inside");
    kani::cover!(!c && uv_spec::member(r, p), "on the boundary only");
}
