// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval set operations (P-ISET, spec 004 data-model §4.2), against `arc_spec`.

use s2rst::s1::Interval;
use std::f64::consts::PI;

use crate::arc_spec::{self, canon, member};
use crate::value_domain::{any_interval, any_point};

fn valid(x: Interval) -> bool {
    arc_spec::valid(x.lo, x.hi)
}

/// P-ISET-01 (correct result): the complement holds every point outside the interval. Apart from
/// a single point, it shares no interior point with the interval, and taking it twice gives the
/// interval back.
///
/// For every valid `x` and point `p`, with `c = x.complement()`: `c` is valid;
/// `member(x, p) ∨ member(c, p)`; `interior(x, p) ⇒ ¬member(c, p)`; if `x.lo != x.hi`,
/// `interior(c, p) ⇒ ¬member(x, p)` and `c.complement()` is `x` bit for bit (upstream
/// `Complement`: a single point's complement is the full circle).
#[kani::proof]
fn p_iset_01() {
    let (x, p) = (any_interval(), any_point());
    let c = x.complement();
    assert!(valid(c));
    assert!(member(x, p) || member(c, p));
    if arc_spec::interior(x, p) {
        assert!(!member(c, p));
    }
    if x.lo != x.hi {
        if arc_spec::interior(c, p) {
            assert!(!member(x, p));
        }
        let cc = c.complement();
        assert_eq!(
            (cc.lo.to_bits(), cc.hi.to_bits()),
            (x.lo.to_bits(), x.hi.to_bits())
        );
    } else {
        assert!(arc_spec::full(c));
    }
    kani::cover!(
        arc_spec::empty(x) && arc_spec::full(c),
        "the empty interval's complement"
    );
    kani::cover!(
        x.lo == x.hi && arc_spec::interior(c, p) && member(x, p),
        "a single point"
    );
}

/// P-ISET-02 (correct result): the union is valid and contains both intervals; its bounds are
/// bounds of the two, or it is full.
///
/// For every valid `x`, `y`, with `u = x.union(y)`: `u` is valid, `arc_spec::contains(u, x)`,
/// `arc_spec::contains(u, y)`, and either `u` is full or `u.lo ∈ {x.lo, y.lo}` and
/// `u.hi ∈ {x.hi, y.hi}`.
#[kani::proof]
fn p_iset_02() {
    let (x, y) = (any_interval(), any_interval());
    let u = x.union(y);
    assert!(valid(u));
    assert!(arc_spec::contains(u, x) && arc_spec::contains(u, y));
    let from_bounds = (u.lo == x.lo || u.lo == y.lo) && (u.hi == x.hi || u.hi == y.hi);
    assert!(arc_spec::full(u) || from_bounds);
    kani::cover!(
        arc_spec::full(u) && !arc_spec::full(x) && !arc_spec::full(y),
        "two cover all"
    );
    kani::cover!(
        u.lo == x.lo && u.hi == y.hi && !arc_spec::contains(x, y),
        "a joined pair"
    );
}

/// P-ISET-03 (correct result): the intersection is valid, holds every common point, and is
/// empty exactly when the two do not intersect.
///
/// For every valid `x`, `y` and point `p`, with `i = x.intersection(y)`: `i` is valid;
/// `member(x, p) ∧ member(y, p) ⇒ member(i, p)`; `empty(i) == ¬arc_spec::intersects(x, y)`.
#[kani::proof]
fn p_iset_03() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    let i = x.intersection(y);
    assert!(valid(i));
    if member(x, p) && member(y, p) {
        assert!(member(i, p));
    }
    assert_eq!(arc_spec::empty(i), !arc_spec::intersects(x, y));
    kani::cover!(
        !arc_spec::empty(i) && i.lo == y.lo && i.hi == x.hi,
        "an overlap"
    );
}

/// P-ISET-04 (correct result): adding a point gives a valid interval that contains the point
/// and the original, and changes nothing if the point was already in it.
///
/// For every valid `x` and point `p`, with `a = x.add_point(p)`: `a` is valid, `member(a, p)`,
/// `arc_spec::contains(a, x)`, and `member(x, p) ⇒ a` is `x` bit for bit.
#[kani::proof]
fn p_iset_04() {
    let (x, p) = (any_interval(), any_point());
    let a = x.add_point(p);
    assert!(valid(a));
    assert!(member(a, p));
    assert!(arc_spec::contains(a, x));
    if member(x, p) {
        assert_eq!(
            (a.lo.to_bits(), a.hi.to_bits()),
            (x.lo.to_bits(), x.hi.to_bits())
        );
    }
    kani::cover!(
        !member(x, p) && a.hi == canon(p),
        "extended at the high end"
    );
    kani::cover!(!member(x, p) && a.lo == canon(p), "extended at the low end");
}

/// P-ISET-05 (correct result): the arc through two points contains both and is the shorter
/// arc, within one ulp of π.
///
/// For every pair of points `p`, `q`, with `r = from_point_pair(p, q)`: `r` is valid,
/// `member(r, p) ∧ member(r, q)`, and `r.length() ≤ next_up(π)`. The ulp: upstream computes the
/// arc's choice with `PositiveDistance` and the length with `GetLength`, which round the same
/// quantity in different orders (found by the pilot: `p = 2 − ulp`, `q = 2 − π`).
#[kani::proof]
fn p_iset_05() {
    let (p, q) = (any_point(), any_point());
    let r = Interval::from_point_pair(p, q);
    assert!(valid(r));
    assert!(member(r, p) && member(r, q));
    assert!(r.length() <= PI.next_up());
    kani::cover!(r.length() > PI, "one ulp above π");
    kani::cover!(arc_spec::is_corner(r), "the corner");
}

/// P-ISET-06 (correct result): projection gives a point of a non-empty interval: the point
/// itself if it is in the interval, and otherwise one of its bounds.
///
/// For every non-empty valid `x` and point `p`, with `q = x.project(p)`: `member(x, q)`; if
/// `member(x, p)`, `q` is `canon(p)` bit for bit; otherwise `q` is `x.lo` or `x.hi`. (That the
/// bound is the nearer one compares two rounded distances, as upstream does; it is left to the
/// differential tests.)
#[kani::proof]
fn p_iset_06() {
    let x = any_interval();
    kani::assume(!arc_spec::empty(x));
    let p = any_point();
    let q = x.project(p);
    assert!(member(x, q));
    if member(x, p) {
        assert_eq!(q.to_bits(), canon(p).to_bits());
    } else {
        assert!(q.to_bits() == x.lo.to_bits() || q.to_bits() == x.hi.to_bits());
    }
    kani::cover!(!member(x, p) && q == x.lo, "the low bound");
    kani::cover!(!member(x, p) && q == x.hi, "the high bound");
}
