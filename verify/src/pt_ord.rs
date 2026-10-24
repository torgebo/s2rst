// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The point's lexicographic order (P-PTORD, spec 006 data-model §4.2), against upstream's
//! `operator<` (`point_spec::order`), and the generic order laws (`laws::ord`).

use s2rst::s2::Point;
use std::cmp::Ordering;

use crate::value_domain::{any_plain_triple, any_triple};
use crate::{laws, point_spec};

/// P-PTORD-01 (correct result): `cmp_point` is upstream's lexicographic order for every triple,
/// NaN included, and the ordering agrees with it.
///
/// For every `a`, `b` in `AnyTriple`: `a.cmp_point(b) == point_spec::order(a, b)` and
/// `a.partial_cmp(&b) == Some(point_spec::order(a, b))`.
#[kani::proof]
fn p_ptord_01() {
    let (a, b) = (any_triple(), any_triple());
    let want = point_spec::order(a, b);
    let c = a.cmp_point(b);
    assert!(c == want);
    assert!(a.partial_cmp(&b) == Some(want));
    kani::cover!(c == Ordering::Less && a.x() == b.x(), "decided by y or z");
    kani::cover!(
        c == Ordering::Equal && a != b,
        "a NaN pair decides nothing (K8)"
    );
}

/// P-PTORD-02 (law): without NaN the order is a total preorder whose ties are exactly equality:
/// the generic order laws (`laws::ord`).
///
/// For every `a`, `b`, `c` in `PlainTriple`: `laws::ord::all(Point::cmp_point, a, b, c)`.
#[kani::proof]
fn p_ptord_02() {
    let (a, b, c) = (any_plain_triple(), any_plain_triple(), any_plain_triple());
    let cmp = |p: &Point, q: &Point| p.cmp_point(*q);
    laws::ord::all(&cmp, &a, &b, &c);
    kani::cover!(
        a.cmp_point(b) == Ordering::Less && b.cmp_point(c) == Ordering::Less,
        "a chain"
    );
}
