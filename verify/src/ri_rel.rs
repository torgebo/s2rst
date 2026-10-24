// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval relations (P-RREL, spec 005 data-model §4.1), against the relations of the
//! sets of real numbers (`real_spec`).

use crate::real_spec;
use crate::value_domain::any_real;

/// P-RREL-01 (correct result): containment and interior containment are those of the sets of
/// real numbers.
///
/// For every `x`, `y` in `Real`: `x.contains_interval(y) == real_spec::contains(x, y)` and
/// `x.interior_contains_interval(y) == real_spec::interior_contains(x, y)`.
#[kani::proof]
fn p_rrel_01() {
    let (x, y) = (any_real(), any_real());
    let (c, ic) = (x.contains_interval(y), x.interior_contains_interval(y));
    assert_eq!(c, real_spec::contains(x, y));
    assert_eq!(ic, real_spec::interior_contains(x, y));
    kani::cover!(c && !ic && !y.is_empty(), "contained, touching a bound");
    kani::cover!(
        c && x.is_empty(),
        "the empty interval contains only empty ones"
    );
}

/// P-RREL-02 (correct result): intersection and interior intersection are those of the sets
/// of real numbers, and intersection is symmetric.
///
/// For every `x`, `y` in `Real`: `x.intersects(y) == real_spec::intersects(x, y)`,
/// `x.interior_intersects(y) == real_spec::interior_intersects(x, y)`, and
/// `x.intersects(y) == y.intersects(x)`.
#[kani::proof]
fn p_rrel_02() {
    let (x, y) = (any_real(), any_real());
    let (i, ii) = (x.intersects(y), x.interior_intersects(y));
    assert_eq!(i, real_spec::intersects(x, y));
    assert_eq!(ii, real_spec::interior_intersects(x, y));
    assert_eq!(i, y.intersects(x));
    kani::cover!(i && !ii, "touching at a bound");
    kani::cover!(ii, "overlapping interiors");
}
