// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Interval relations (P-IREL, spec 004 data-model §4.2), against the real-arc relations of
//! `arc_spec`: decided by the cyclic order of the bounds, because a float-free gap is still a
//! gap (data-model §1.2, as amended in T010).

use s2rst::s1::Interval;

use crate::arc_spec;
use crate::value_domain::{any_interval, any_point};

/// P-IREL-01 (correct result): if one interval contains another, every point of the second is
/// in the first.
///
/// For every valid `x`, `y` and point `p`: `x.contains_interval(y) ∧ member(y, p) ⇒
/// member(x, p)`.
#[kani::proof]
fn p_irel_01() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    if x.contains_interval(y) && arc_spec::member(y, p) {
        assert!(arc_spec::member(x, p));
    }
    kani::cover!(
        x.contains_interval(y) && arc_spec::member(y, p) && !x.is_full(),
        "a shared point"
    );
}

/// P-IREL-02 (correct result): containment is exactly the containment of real arcs.
///
/// For every valid `x`, `y`: `x.contains_interval(y) == arc_spec::contains(x, y)`.
#[kani::proof]
fn p_irel_02() {
    let (x, y) = (any_interval(), any_interval());
    assert_eq!(x.contains_interval(y), arc_spec::contains(x, y));
    kani::cover!(
        x.contains_interval(y) && x.is_inverted() && !y.is_inverted(),
        "inverted ⊇ plain"
    );
    kani::cover!(
        !x.contains_interval(y) && y.is_full(),
        "only the full interval holds the full one"
    );
}

/// P-IREL-03 (correct result): interior containment is exactly `arc_spec::interior_contains`,
/// and then every point of the second interval is in the first's interior.
///
/// For every valid `x`, `y` and point `p`: `x.interior_contains_interval(y) ==
/// arc_spec::interior_contains(x, y)`, and if it holds, `member(y, p) ⇒ interior(x, p)`.
#[kani::proof]
fn p_irel_03() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    let r = x.interior_contains_interval(y);
    assert_eq!(r, arc_spec::interior_contains(x, y));
    if r && arc_spec::member(y, p) {
        assert!(arc_spec::interior(x, p));
    }
    kani::cover!(r && !y.is_empty() && !x.is_full(), "strictly inside");
}

/// P-IREL-04 (correct result): two intervals intersect exactly when one contains a bound of the
/// other; a shared point implies it; it is symmetric.
///
/// For every valid `x`, `y` and point `p`: `x.intersects(y) == arc_spec::intersects(x, y) ==
/// y.intersects(x)`, and `member(x, p) ∧ member(y, p) ⇒ x.intersects(y)`.
#[kani::proof]
fn p_irel_04() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    let r = x.intersects(y);
    assert_eq!(r, arc_spec::intersects(x, y));
    assert_eq!(r, y.intersects(x));
    if arc_spec::member(x, p) && arc_spec::member(y, p) {
        assert!(r);
    }
    kani::cover!(
        r && x.is_inverted() && y.is_inverted(),
        "two inverted intervals"
    );
    kani::cover!(!r && !x.is_empty() && !y.is_empty(), "disjoint");
}

/// P-IREL-05 (model check): the real-arc relations are consistent with membership: a point is
/// in an interval exactly when the interval contains that point's single-point interval;
/// containment implies every point of `y` is in `x`; interior containment implies every point of
/// `y` is in `x`'s interior.
///
/// For every valid `x`, `y` and point `p`, with `s = [canon(p), canon(p)]`, calling only
/// `arc_spec`: `member(x, p) == contains(x, s)`; `contains(x, y) ∧ member(y, p) ⇒ member(x, p)`;
/// `interior_contains(x, y) ∧ member(y, p) ⇒ interior(x, p)`.
#[kani::proof]
fn p_irel_05() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    let q = arc_spec::canon(p);
    let s = Interval { lo: q, hi: q };
    assert_eq!(arc_spec::member(x, p), arc_spec::contains(x, s));
    if arc_spec::member(y, p) {
        if arc_spec::contains(x, y) {
            assert!(arc_spec::member(x, p));
        }
        if arc_spec::interior_contains(x, y) {
            assert!(arc_spec::interior(x, p));
        }
    }
    kani::cover!(
        arc_spec::contains(x, y) && !arc_spec::contains(y, x),
        "strict containment"
    );
}

/// P-IREL-06 (correct result): interior intersection is exactly `arc_spec::interior_intersects`:
/// the second interval meets the first's interior. It is not symmetric, as upstream documents.
///
/// For every valid `x`, `y` and point `p`: `x.interior_intersects(y) ==
/// arc_spec::interior_intersects(x, y)`, and `interior(x, p) ∧ member(y, p) ⇒
/// x.interior_intersects(y)`.
#[kani::proof]
fn p_irel_06() {
    let (x, y, p) = (any_interval(), any_interval(), any_point());
    let r = x.interior_intersects(y);
    assert_eq!(r, arc_spec::interior_intersects(x, y));
    if arc_spec::interior(x, p) && arc_spec::member(y, p) {
        assert!(r);
    }
    kani::cover!(r && !y.interior_intersects(x), "not symmetric");
}
