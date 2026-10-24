// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval set operations (P-RSET, spec 005 data-model §4.1), against the real-set
//! description (`real_spec`).

use crate::real_spec;
use crate::value_domain::{any_number, any_real};

/// P-RSET-01 (correct result): the union is the least interval containing both.
///
/// For every `x`, `y`, `z` in `Real`, with `u = x.union(y)`: `real_spec::contains(u, x)`,
/// `real_spec::contains(u, y)`, and `contains(z, x) ∧ contains(z, y) ⇒ contains(z, u)`.
#[kani::proof]
fn p_rset_01() {
    let (x, y, z) = (any_real(), any_real(), any_real());
    let u = x.union(y);
    assert!(real_spec::contains(u, x) && real_spec::contains(u, y));
    if real_spec::contains(z, x) && real_spec::contains(z, y) {
        assert!(real_spec::contains(z, u));
    }
    kani::cover!(
        !x.is_empty() && !y.is_empty() && !x.intersects(y),
        "the union bridges a gap"
    );
}

/// P-RSET-02 (correct result): the intersection holds exactly the numbers in both.
///
/// For every `x`, `y` in `Real` and non-NaN `p`:
/// `member(x.intersection(y), p) == member(x, p) ∧ member(y, p)`.
#[kani::proof]
fn p_rset_02() {
    let (x, y, p) = (any_real(), any_real(), any_number());
    let i = x.intersection(y);
    let both = real_spec::member(x, p) && real_spec::member(y, p);
    assert_eq!(real_spec::member(i, p), both);
    kani::cover!(both, "a common number");
    kani::cover!(
        real_spec::empty(i) && !x.is_empty() && !y.is_empty(),
        "disjoint"
    );
}

/// P-RSET-03 (correct result): adding a number gives the least interval containing it and the
/// original; adding an interval is the union.
///
/// For every `x`, `y`, `z` in `Real` and non-NaN `p`, with `a = x.add_point(p)`:
/// `member(a, p)`, `contains(a, x)`, and `contains(z, x) ∧ member(z, p) ⇒ contains(z, a)`;
/// `x.add_interval(y) == x.union(y)` (the type's equality, every empty interval alike).
#[kani::proof]
fn p_rset_03() {
    let (x, y, z, p) = (any_real(), any_real(), any_real(), any_number());
    let a = x.add_point(p);
    assert!(real_spec::member(a, p) && real_spec::contains(a, x));
    if real_spec::contains(z, x) && real_spec::member(z, p) {
        assert!(real_spec::contains(z, a));
    }
    assert!(x.add_interval(y) == x.union(y));
    kani::cover!(
        x.is_empty() && a.lo == p,
        "the empty interval becomes a point"
    );
    kani::cover!(!x.is_empty() && !x.contains(p), "extended");
}
