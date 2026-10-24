// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle relations (P-LREL, spec 005 data-model §4.2), against the product relations of
//! `rect_spec`, never against the components' operations (contracts/harness-conventions.md
//! "Statements").

use crate::rect_spec;
use crate::value_domain::any_rect;

/// P-LREL-01 (correct result): containment and interior containment are those of the product
/// sets: each factor contains the other's.
///
/// For every valid `a`, `b`: `a.contains(b) == rect_spec::contains(a, b)` and
/// `a.interior_contains(b) == rect_spec::interior_contains(a, b)`.
#[kani::proof]
fn p_lrel_01() {
    let (a, b) = (any_rect(), any_rect());
    let (c, ic) = (a.contains(b), a.interior_contains(b));
    assert_eq!(c, rect_spec::contains(a, b));
    assert_eq!(ic, rect_spec::interior_contains(a, b));
    kani::cover!(
        c && !ic && !rect_spec::empty(b),
        "contained, touching the boundary"
    );
    kani::cover!(
        c && a.is_inverted() && !b.is_inverted(),
        "across the antimeridian"
    );
}

/// P-LREL-02 (correct result): intersection and interior intersection are those of the
/// product sets, and intersection is symmetric.
///
/// For every valid `a`, `b`: `a.intersects(b) == rect_spec::intersects(a, b)`,
/// `a.interior_intersects(b) == rect_spec::interior_intersects(a, b)`, and
/// `a.intersects(b) == b.intersects(a)`.
#[kani::proof]
fn p_lrel_02() {
    let (a, b) = (any_rect(), any_rect());
    let (i, ii) = (a.intersects(b), a.interior_intersects(b));
    assert_eq!(i, rect_spec::intersects(a, b));
    assert_eq!(ii, rect_spec::interior_intersects(a, b));
    assert_eq!(i, b.intersects(a));
    kani::cover!(i && !ii, "touching on the boundary");
    kani::cover!(ii && a.is_inverted(), "overlapping across the antimeridian");
}
