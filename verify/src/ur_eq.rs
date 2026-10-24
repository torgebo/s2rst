// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! u-v rectangle equality (P-QEQ, spec 006 data-model §4.1): the generic equality laws and
//! upstream's meaning.

use s2rst::r1;

use crate::value_domain::any_uv_rect;
use crate::{laws, real_spec};

/// Upstream's interval equality (`r1interval.h:202`): both empty, or the same bounds.
fn same_interval(x: r1::Interval, y: r1::Interval) -> bool {
    (real_spec::empty(x) && real_spec::empty(y)) || (x.lo == y.lo && x.hi == y.hi)
}

/// P-QEQ-01 (law): the generic equality laws hold, and equality is upstream's: two rectangles
/// are equal exactly when both intervals are, so every two empty rectangles are equal.
///
/// For every `a`, `b`, `c` in `UvRect`: `laws::eq::all(a, b, c)`, and `a == b` exactly when
/// each axis of `a` and `b` is equal as upstream's `R1Interval::operator==` says: both empty,
/// or the same bounds.
#[kani::proof]
fn p_qeq_01() {
    let (a, b, c) = (any_uv_rect(), any_uv_rect(), any_uv_rect());
    laws::eq::all(&a, &b, &c);
    assert_eq!(a == b, same_interval(a.x, b.x) && same_interval(a.y, b.y));
    kani::cover!(a == b && a.x.lo != b.x.lo, "two different empty rectangles");
}
