// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s1::Interval::<op>` for the 30 in-scope operations of the
//! circular interval (spec 004 US1, FR-004; data-model §5.2).
//!
//! Each harness draws its inputs from the operation's domain in `values/INVENTORY.md`:
//! - every valid interval, built as a struct literal (`ValidInterval`);
//! - a point in [−π, π] for the debug-checked point operations (K3, FR-019);
//! - a non-empty interval for `project`;
//! - `i ≤ 1` for `bound`;
//! - every margin except NaN for `expanded` (K5, documented);
//! - every `f64` for `approx_eq_with`'s error bound.
//!
//! Kani builds with debug assertions on, so every `debug_assert!` on the path, including the
//! internal checks of `new_checked`, `intersection` and `directed_hausdorff_distance`, is an
//! obligation: the proof shows it never fires in the domain. The covers show the domains
//! non-empty and the outcomes FR-009 names reachable: inverted and non-inverted, empty, full
//! and single-point intervals, and the corner.

use s2rst::s1::Interval;
use std::f64::consts::PI;

use crate::arc_spec;
use crate::value_domain::{any_f64, any_interval, any_point};

/// C-s1::Interval::new, empty, full, from_point, from_point_pair and default: complete without
/// panic, arithmetic overflow or undefined behaviour for every pair of points in [−π, π].
#[kani::proof]
fn c_constructors() {
    let (p, q) = (any_point(), any_point());
    let n = Interval::new(p, q);
    let pair = Interval::from_point_pair(p, q);
    let single = Interval::from_point(p);
    let (e, f, d) = (Interval::empty(), Interval::full(), Interval::default());
    kani::cover!(p == -PI && q == PI, "the full interval through new");
    kani::cover!(n.lo > n.hi, "an inverted interval");
    kani::cover!(
        arc_spec::is_corner(pair),
        "the corner through from_point_pair"
    );
    kani::cover!(single.lo == PI && p == -PI, "−π as a single point");
    kani::cover!(e.lo == d.lo && f.hi == PI, "the special intervals");
}

/// C-s1::Interval::is_valid: completes without panic, arithmetic overflow or undefined
/// behaviour for every pair of `f64` bounds, NaN and the infinities included.
#[kani::proof]
fn c_is_valid() {
    let x = Interval {
        lo: any_f64(),
        hi: any_f64(),
    };
    let v = x.is_valid();
    kani::cover!(v, "valid");
    kani::cover!(!v && x.lo.is_nan(), "NaN");
}

/// C-s1::Interval::is_full, is_empty, is_inverted, center, length, complement,
/// complement_center, bound and eq: complete without panic, arithmetic overflow or undefined
/// behaviour for every valid interval (and every pair, for `eq`; `i ≤ 1` for `bound`).
#[kani::proof]
fn c_classifiers_and_measures() {
    let x = any_interval();
    let y = any_interval();
    let i: usize = kani::any();
    kani::assume(i <= 1);
    let _ = (x.is_full(), x.is_empty(), x.is_inverted());
    let _ = (
        x.center(),
        x.length(),
        x.complement(),
        x.complement_center(),
    );
    let _ = (x.bound(i), x == y);
    kani::cover!(x.is_empty(), "empty");
    kani::cover!(x.is_full(), "full");
    kani::cover!(x.lo == x.hi, "a single point");
    kani::cover!(arc_spec::is_corner(x), "the corner");
    kani::cover!(x.is_inverted() && !x.is_empty(), "inverted");
}

/// C-s1::Interval::contains, interior_contains and add_point: complete without panic,
/// arithmetic overflow or undefined behaviour for every valid interval and every point in
/// [−π, π] (their documented precondition, FR-019).
#[kani::proof]
fn c_point_operations() {
    let x = any_interval();
    let p = any_point();
    let (c, ic) = (x.contains(p), x.interior_contains(p));
    let a = x.add_point(p);
    kani::cover!(c && !ic, "on the boundary");
    kani::cover!(!c && a.lo == p, "extended at the low end");
    kani::cover!(
        x.is_empty() && a.lo == a.hi,
        "the empty interval becomes a point"
    );
}

/// C-s1::Interval::project: completes without panic, arithmetic overflow or undefined
/// behaviour for every non-empty valid interval and every point in [−π, π] (FR-019).
#[kani::proof]
fn c_project() {
    let x = any_interval();
    kani::assume(!arc_spec::empty(x));
    let p = any_point();
    let q = x.project(p);
    kani::cover!(q == p, "inside");
    kani::cover!(q != p && q == x.lo, "the low bound is nearer");
    kani::cover!(q != p && q == x.hi, "the high bound is nearer");
}

/// C-s1::Interval::contains_interval, interior_contains_interval, intersects and
/// interior_intersects: complete without panic, arithmetic overflow or undefined behaviour for
/// every pair of valid intervals.
#[kani::proof]
fn c_relations() {
    let (x, y) = (any_interval(), any_interval());
    let r = (
        x.contains_interval(y),
        x.interior_contains_interval(y),
        x.intersects(y),
        x.interior_intersects(y),
    );
    kani::cover!(r.0 && !r.1, "contained, not in the interior");
    kani::cover!(r.2 && !r.3, "meeting at a bound only");
    kani::cover!(
        !r.2 && x.is_inverted(),
        "disjoint from an inverted interval"
    );
}

/// C-s1::Interval::union and intersection: complete without panic, arithmetic overflow or
/// undefined behaviour for every pair of valid intervals, with `intersection`'s internal check
/// never firing.
#[kani::proof]
fn c_union_intersection() {
    let (x, y) = (any_interval(), any_interval());
    let u = x.union(y);
    let i = x.intersection(y);
    kani::cover!(
        u.is_full() && !x.is_full() && !y.is_full(),
        "two intervals cover the circle"
    );
    kani::cover!(i.is_empty() && !x.is_empty() && !y.is_empty(), "disjoint");
}

/// C-s1::Interval::directed_hausdorff_distance: completes without panic, arithmetic overflow or
/// undefined behaviour for every pair of valid intervals, with its internal check and the
/// helper arcs' checks never firing (D-005).
#[kani::proof]
fn c_directed_hausdorff_distance() {
    let (x, y) = (any_interval(), any_interval());
    let d = x.directed_hausdorff_distance(y);
    kani::cover!(d == 0.0 && !x.is_empty(), "contained");
    kani::cover!(d == PI, "to the empty interval");
    kani::cover!(
        y.lo == 0.0 && y.hi > 0.0 && y.hi < 1e-15 && !y.contains_interval(x),
        "a short y near 0 (D-005's path)"
    );
}

/// C-s1::Interval::expanded: completes without panic, arithmetic overflow or undefined
/// behaviour for every valid interval and every margin except NaN (K5: a NaN margin trips
/// `new`'s check, as documented).
#[kani::proof]
fn c_expanded() {
    let x = any_interval();
    let m = any_f64();
    kani::assume(!m.is_nan());
    let e = x.expanded(m);
    kani::cover!(e.is_full() && !x.is_full(), "expanded to full");
    kani::cover!(e.is_empty() && !x.is_empty(), "shrunk to empty");
    kani::cover!(m == f64::INFINITY, "an infinite margin");
}

/// C-s1::Interval::approx_eq_with and approx_eq: complete without panic, arithmetic overflow or
/// undefined behaviour for every pair of valid intervals and every error bound, NaN included.
#[kani::proof]
fn c_approx_eq() {
    let (x, y) = (any_interval(), any_interval());
    let e = any_f64();
    let a = x.approx_eq_with(y, e);
    let b = x.approx_eq(y);
    kani::cover!(a && x != y, "approximately but not exactly equal");
    kani::cover!(b, "equal within 1e-15");
}

// ---------------------------------------------------------------------------------------------
// Outside the domain: the documented panics (gap 1; `crash.rs` explains the `n_*` and `r_*`
// conventions). The point checks are `debug_assert!`s, so each has a release counterpart.
// ---------------------------------------------------------------------------------------------

/// Every `f64` outside [−π, π], NaN and the infinities included.
fn any_bad_point() -> f64 {
    let p: f64 = kani::any();
    kani::assume(!arc_spec::in_range(p));
    p
}

/// N-s1::Interval::new, N-from_point_pair: in debug builds, each panics for every pair of
/// endpoints of which one is outside [−π, π] or NaN.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_constructors() {
    let (p, q) = (any_f64(), any_f64());
    kani::assume(!arc_spec::in_range(p) || !arc_spec::in_range(q));
    if kani::any::<bool>() {
        let _ = Interval::new(p, q);
    } else {
        let _ = Interval::from_point_pair(p, q);
    }
    kani::cover!(true, "returns");
}

/// R-s1::Interval::new, R-from_point_pair: in a build without debug assertions, each completes
/// without panic or overflow on the same pairs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_constructors() {
    let (p, q) = (any_f64(), any_f64());
    kani::assume(!arc_spec::in_range(p) || !arc_spec::in_range(q));
    let (n, pair) = (Interval::new(p, q), Interval::from_point_pair(p, q));
    kani::cover!(p.is_nan() && n.lo.is_nan(), "a NaN endpoint");
    kani::cover!(
        q == f64::INFINITY && !pair.is_valid(),
        "an infinite endpoint"
    );
}

/// N-s1::Interval::contains, N-interior_contains, N-add_point: in debug builds, each panics for
/// every valid interval and every point outside [−π, π] or NaN.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_point_operations() {
    let (x, p) = (any_interval(), any_bad_point());
    let op: u8 = kani::any();
    match op {
        0 => {
            let _ = x.contains(p);
        }
        1 => {
            let _ = x.interior_contains(p);
        }
        _ => {
            let _ = x.add_point(p);
        }
    }
    kani::cover!(true, "returns");
}

/// R-s1::Interval::contains, R-interior_contains, R-add_point: in a build without debug
/// assertions, each completes without panic or overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_point_operations() {
    let (x, p) = (any_interval(), any_bad_point());
    let (c, _, a) = (x.contains(p), x.interior_contains(p), x.add_point(p));
    kani::cover!(p.is_nan() && !c, "a NaN point");
    kani::cover!(!a.is_valid(), "an invalid result");
}

/// The complement of `project`'s domain: an empty valid interval with any point, or a valid
/// interval with a point outside [−π, π] or NaN.
fn outside_project() -> (Interval, f64) {
    let x = any_interval();
    let p = any_f64();
    kani::assume(arc_spec::empty(x) || !arc_spec::in_range(p));
    (x, p)
}

/// N-s1::Interval::project: in debug builds, panics for every empty interval and every point
/// outside [−π, π] or NaN.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_project() {
    let (x, p) = outside_project();
    let _ = x.project(p);
    kani::cover!(true, "returns");
}

/// R-s1::Interval::project: in a build without debug assertions, completes without panic or
/// overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_project() {
    let (x, p) = outside_project();
    let _ = x.project(p);
    kani::cover!(arc_spec::empty(x), "the empty interval");
    kani::cover!(!arc_spec::empty(x) && p.is_nan(), "a NaN point");
}

/// N-s1::Interval::expanded: in debug builds, panics for every valid interval but the full one
/// and a NaN margin (through `new`'s check). Over every valid interval the first version of this
/// harness returned (2026-10-26): the full interval, as `o_expanded_nan_full` proves.
#[cfg(debug_assertions)]
#[kani::proof]
#[kani::should_panic]
fn n_expanded() {
    let x = any_interval();
    kani::assume(!arc_spec::full(x));
    let _ = x.expanded(f64::NAN);
    kani::cover!(true, "returns");
}

/// O-s1::Interval::expanded (outside its domain, returns): the full interval with a NaN margin
/// returns itself, without a panic, in every build (its `# Panics`).
#[kani::proof]
fn o_expanded_nan_full() {
    let e = Interval::full().expanded(f64::NAN);
    assert!(e.is_full());
    kani::cover!(true, "returns the full interval");
}

/// R-s1::Interval::expanded: in a build without debug assertions, completes without panic or
/// overflow on the same inputs.
#[cfg(not(debug_assertions))]
#[kani::proof]
fn r_expanded() {
    let x = any_interval();
    kani::assume(!arc_spec::full(x));
    let e = x.expanded(f64::NAN);
    kani::cover!(!e.is_valid(), "an invalid result");
}

/// N-s1::Interval::bound: panics for every valid interval and every index above 1, in every
/// build (an index out of bounds).
#[kani::proof]
#[kani::should_panic]
fn n_bound() {
    let x = any_interval();
    let i: usize = kani::any();
    kani::assume(i > 1);
    let _ = x.bound(i);
    kani::cover!(true, "returns");
}
