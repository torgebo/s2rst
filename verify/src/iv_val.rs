// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval validity and representation (P-IVAL, spec 004 data-model §4.2, §4.3), against the
//! reference description of an arc (`arc_spec`).

use s2rst::s1::Interval;
use std::f64::consts::PI;

use crate::arc_spec;
use crate::laws;
use crate::value_domain::{any_f64, any_interval, any_point};

/// P-IVAL-01 (correct result): the validity check agrees with the model's for every pair of
/// floats, NaN and the infinities included.
///
/// For every `lo`, `hi`: `Interval { lo, hi }.is_valid() == arc_spec::valid(lo, hi)`.
#[kani::proof]
fn p_ival_01() {
    let (lo, hi) = (any_f64(), any_f64());
    let v = Interval { lo, hi }.is_valid();
    assert_eq!(v, arc_spec::valid(lo, hi));
    kani::cover!(v && lo > hi, "an inverted valid pair");
    kani::cover!(!v && lo == -PI, "−π as a low bound");
}

/// P-IVAL-02 (validity): every constructor returns a valid interval for its documented inputs,
/// and `new` turns −π into π except in the full and empty intervals. The default interval is
/// the empty one, as documented.
///
/// For every pair of points `p`, `q` in [−π, π]: `new(p, q)` is valid, with
/// `lo = (p == −π && q != π) ? π : p` and `hi = (q == −π && p != π) ? π : q`;
/// `from_point(p)` is valid and is `[canon(p), canon(p)]`; `from_point_pair(p, q)` is valid;
/// `empty()`, `full()` and `default()` are valid, and `default()` is `empty()`.
#[kani::proof]
fn p_ival_02() {
    let (p, q) = (any_point(), any_point());
    let n = Interval::new(p, q);
    assert!(arc_spec::valid(n.lo, n.hi));
    let want_lo = if p == -PI && q != PI { PI } else { p };
    let want_hi = if q == -PI && p != PI { PI } else { q };
    assert_eq!(
        (n.lo.to_bits(), n.hi.to_bits()),
        (want_lo.to_bits(), want_hi.to_bits())
    );
    let s = Interval::from_point(p);
    assert!(arc_spec::valid(s.lo, s.hi));
    assert_eq!(
        (s.lo.to_bits(), s.hi.to_bits()),
        (arc_spec::canon(p).to_bits(), arc_spec::canon(p).to_bits())
    );
    let pair = Interval::from_point_pair(p, q);
    assert!(arc_spec::valid(pair.lo, pair.hi));
    let (e, f, d) = (Interval::empty(), Interval::full(), Interval::default());
    assert!(arc_spec::valid(e.lo, e.hi) && arc_spec::valid(f.lo, f.hi));
    assert!(arc_spec::empty(e) && arc_spec::full(f) && arc_spec::empty(d));
    kani::cover!(p == -PI && q == PI, "new keeps the full interval");
    kani::cover!(p == PI && q == -PI, "new keeps the empty interval");
    kani::cover!(p == -PI && q == 0.0, "new turns −π into π");
}

/// P-IVAL-03 (correct result): empty, full and inverted agree with the model. The empty and
/// full intervals are the only ones of their kind, the empty one is inverted, and none is both.
///
/// For every valid interval: `is_empty()`, `is_full()` and `is_inverted()` are
/// `arc_spec::empty`, `full` and `inverted`; `empty ⇒ inverted`; `¬(empty ∧ full)`.
#[kani::proof]
fn p_ival_03() {
    let x = any_interval();
    assert_eq!(x.is_empty(), arc_spec::empty(x));
    assert_eq!(x.is_full(), arc_spec::full(x));
    assert_eq!(x.is_inverted(), arc_spec::inverted(x));
    assert!(!x.is_empty() || x.is_inverted());
    assert!(!(x.is_empty() && x.is_full()));
    kani::cover!(x.is_empty(), "empty");
    kani::cover!(x.is_full(), "full");
    kani::cover!(x.is_inverted() && !x.is_empty(), "inverted, not empty");
}

/// P-IVAL-04 (correct result): `bound(0)` is the low bound and `bound(1)` the high one;
/// equality compares both bounds with float `==`.
///
/// For every valid `x`, `y` and `i ≤ 1`: `x.bound(0)` and `x.bound(1)` are `x.lo` and `x.hi`
/// bit for bit, and `(x == y) == (x.lo == y.lo && x.hi == y.hi)`.
#[kani::proof]
fn p_ival_04() {
    let (x, y) = (any_interval(), any_interval());
    assert_eq!(x.bound(0).to_bits(), x.lo.to_bits());
    assert_eq!(x.bound(1).to_bits(), x.hi.to_bits());
    assert_eq!(x == y, x.lo == y.lo && x.hi == y.hi);
    kani::cover!(
        x == y && x.lo.to_bits() != y.lo.to_bits(),
        "equal with different zeros"
    );
}

/// P-IVAL-05 (law): the generic equality laws, for the interval: equality is an equivalence on
/// valid intervals. It is not bit identity: `[0, 0] == [−0, −0]`, although their lengths are
/// `0.0` and `−0.0`.
///
/// `laws::eq::all(&x, &y, &z)` for every triple of valid intervals (spec 004 §4.3).
#[kani::proof]
fn p_ival_05() {
    let (x, y, z) = (any_interval(), any_interval(), any_interval());
    laws::eq::all(&x, &y, &z);
    kani::cover!(
        x == y && y == z && x.lo.to_bits() != z.lo.to_bits(),
        "equal, not identical"
    );
}
