// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval representation (P-RIV, spec 005 data-model §4.1): emptiness, the
//! constructors, equality and `bound`, against the real-set description (`real_spec`).

use s2rst::r1::{Endpoint, Interval};

use crate::laws;
use crate::real_spec;
use crate::value_domain::{any_number, any_real};

fn bits(x: Interval) -> (u64, u64) {
    (x.lo.to_bits(), x.hi.to_bits())
}

/// P-RIV-01 (correct result): an interval is empty exactly when its low bound exceeds its
/// high one, and the constructors give the documented intervals: `[lo, hi]` as given, a
/// single point, the two points in order, the pair conversion, and the empty default
/// `[1, 0]`.
///
/// For every `x` in `Real` and non-NaN `p`, `q`: `x.is_empty() == real_spec::empty(x)`;
/// `new(p, q)` and `From((p, q))` are `[p, q]`; `from_point(p)` is `[p, p]`;
/// `from_point_pair(p, q)` is `[p, q]` if `p ≤ q`, else `[q, p]`; `empty()` and `default()`
/// are `[1, 0]`; all bit for bit.
#[kani::proof]
fn p_riv_01() {
    let x = any_real();
    assert_eq!(x.is_empty(), real_spec::empty(x));
    let (p, q) = (any_number(), any_number());
    let pq = (p.to_bits(), q.to_bits());
    assert_eq!(bits(Interval::new(p, q)), pq);
    assert_eq!(bits(Interval::from((p, q))), pq);
    assert_eq!(bits(Interval::from_point(p)), (p.to_bits(), p.to_bits()));
    let want = if p <= q {
        pq
    } else {
        (q.to_bits(), p.to_bits())
    };
    assert_eq!(bits(Interval::from_point_pair(p, q)), want);
    let one_zero = (1.0_f64.to_bits(), 0.0_f64.to_bits());
    assert_eq!(bits(Interval::empty()), one_zero);
    assert_eq!(bits(Interval::default()), one_zero);
    kani::cover!(
        x.is_empty() && x.lo.is_infinite(),
        "an empty interval with an infinite bound"
    );
    kani::cover!(p > q, "from_point_pair swaps");
}

/// P-RIV-02 (law): equality is reflexive, symmetric and transitive (`laws::eq`), and every
/// two empty intervals are equal, as upstream's `operator==` says.
///
/// For every `x`, `y`, `z` in `Real`: `laws::eq::all(x, y, z)`; `empty(x) ∧ empty(y) ⇒ x == y`;
/// `¬empty(x) ⇒ (x == y ⇔ x.lo == y.lo ∧ x.hi == y.hi)`.
#[kani::proof]
fn p_riv_02() {
    let (x, y, z) = (any_real(), any_real(), any_real());
    laws::eq::all(&x, &y, &z);
    if real_spec::empty(x) && real_spec::empty(y) {
        assert!(x == y);
    }
    if !real_spec::empty(x) {
        assert_eq!(x == y, x.lo == y.lo && x.hi == y.hi);
    }
    kani::cover!(x == y && x.lo != y.lo, "two different empty intervals");
}

/// P-RIV-03 (correct result): `bound` returns the low or the high bound.
///
/// For every `x` in `Real`: `x.bound(Lo)` is `x.lo` and `x.bound(Hi)` is `x.hi`, bit for bit.
#[kani::proof]
fn p_riv_03() {
    let x = any_real();
    assert_eq!(x.bound(Endpoint::Lo).to_bits(), x.lo.to_bits());
    assert_eq!(x.bound(Endpoint::Hi).to_bits(), x.hi.to_bits());
}
