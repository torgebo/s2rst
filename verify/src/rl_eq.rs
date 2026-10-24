// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle equality laws and the rectangle as its own bound (P-LEQ, spec 005 data-model
//! §4.2).
//!
//! Spec 004's `laws::region` laws take a cell and go through `contains_cell` and
//! `intersects_cell`, which are excluded for the rectangle (floating-point geometry, FR-003).
//! They are not instantiated here; the quickcheck twins `region_rect_*` in
//! `core/tests/trait_property_tests.rs` cover them.

use s2rst::s2::{Rect, Region};

use crate::laws;
use crate::rect_spec;
use crate::value_domain::any_rect;

fn bits(r: Rect) -> [u64; 4] {
    [r.lat.lo, r.lat.hi, r.lng.lo, r.lng.hi].map(f64::to_bits)
}

/// P-LEQ-01 (law): equality is reflexive, symmetric and transitive on valid rectangles, and
/// it is upstream's: every two empty rectangles are equal, and otherwise the bounds are.
///
/// For every valid `a`, `b`, `c`: `laws::eq::all(a, b, c)`;
/// `a == b ⇔ (empty(a) ∧ empty(b)) ∨ (the four bounds are equal, as numbers)`
/// (`S2LatLngRect::operator==`, component-wise, with `R1Interval`'s empty intervals alike).
#[kani::proof]
fn p_leq_01() {
    let (a, b, c) = (any_rect(), any_rect(), any_rect());
    laws::eq::all(&a, &b, &c);
    let same_bounds = a.lat.lo == b.lat.lo
        && a.lat.hi == b.lat.hi
        && a.lng.lo == b.lng.lo
        && a.lng.hi == b.lng.hi;
    let both_empty = rect_spec::empty(a) && rect_spec::empty(b);
    assert_eq!(a == b, both_empty || same_bounds);
    kani::cover!(
        a == b && a.lat.lo != b.lat.lo,
        "two different empty rectangles"
    );
    kani::cover!(a == b && !rect_spec::empty(a), "equal non-empty rectangles");
}

/// P-LEQ-02 (correct result): the rectangle's bound, inherent and through the region
/// interface, is the rectangle itself.
///
/// For every valid `r`: `r.rect_bound()` and `Region::rect_bound(&r)` are `r`, bit for bit.
#[kani::proof]
fn p_leq_02() {
    let r = any_rect();
    assert_eq!(bits(r.rect_bound()), bits(r));
    assert_eq!(bits(Region::rect_bound(&r)), bits(r));
}
