// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `PointRegion`'s equality (spec 011 data-model §4.1).

use s2rst::s2::point_region::PointRegion;

use crate::laws;
use crate::region_spec::pr_contains;
use crate::value_domain::any_plain_triple;

/// P-PREQ-01 (law): two regions are equal exactly when their points are equal as numbers, and
/// equality is an equivalence relation on regions whose points have no NaN coordinate
/// (`laws::eq::all`).
///
/// Exactly: for every three triples without NaN, `(a == b) == pr_contains(pa, pb)`, and
/// `laws::eq::all(a, b, c)`.
#[kani::proof]
fn p_preq_01() {
    let (pa, pb, pc) = (any_plain_triple(), any_plain_triple(), any_plain_triple());
    let (a, b, c) = (
        PointRegion::new(pa),
        PointRegion::new(pb),
        PointRegion::new(pc),
    );
    assert!((a == b) == pr_contains(pa, pb));
    laws::eq::all(&a, &b, &c);
    kani::cover!(a == b && b == c, "three equal regions");
}
