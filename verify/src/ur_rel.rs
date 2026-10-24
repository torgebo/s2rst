// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Relations between u-v rectangles (P-QREL, spec 006 data-model §4.1), against the relations
//! of the product sets (`uv_spec`).

use crate::uv_spec;
use crate::value_domain::any_uv_rect;

/// P-QREL-01 (correct result): containment and interior containment are the products of the
/// intervals'.
///
/// For every `a`, `b` in `UvRect`: `a.contains(b) == uv_spec::contains(a, b)` and
/// `a.interior_contains(b) == uv_spec::interior_contains(a, b)`.
#[kani::proof]
fn p_qrel_01() {
    let (a, b) = (any_uv_rect(), any_uv_rect());
    let (c, ic) = (a.contains(b), a.interior_contains(b));
    assert_eq!(c, uv_spec::contains(a, b));
    assert_eq!(ic, uv_spec::interior_contains(a, b));
    kani::cover!(
        c && !ic && !b.is_empty(),
        "contained, touching the boundary"
    );
    kani::cover!(
        c && a.is_empty(),
        "the empty rectangle contains only empty ones"
    );
}

/// P-QREL-02 (correct result): intersection and interior intersection are the products of the
/// intervals', and intersection is symmetric.
///
/// For every `a`, `b` in `UvRect`: `a.intersects(b) == uv_spec::intersects(a, b)`,
/// `a.interior_intersects(b) == uv_spec::interior_intersects(a, b)`, and
/// `a.intersects(b) == b.intersects(a)`.
#[kani::proof]
fn p_qrel_02() {
    let (a, b) = (any_uv_rect(), any_uv_rect());
    let (i, ii) = (a.intersects(b), a.interior_intersects(b));
    assert_eq!(i, uv_spec::intersects(a, b));
    assert_eq!(ii, uv_spec::interior_intersects(a, b));
    assert_eq!(i, b.intersects(a));
    kani::cover!(i && !ii, "touching on the boundary");
    kani::cover!(ii, "overlapping interiors");
}
