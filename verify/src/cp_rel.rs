// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Relations between caps: containment and intersection (P-CPREL, spec 008 data-model §4).

use s2rst::s1::ChordAngle;
use s2rst::s2::Cap;

use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point, any_nonempty_cap};

/// P-CPREL-01 (law): `contains` is reflexive on non-empty caps; the full cap contains every cap,
/// and every cap contains the empty cap.
///
/// For every `a` in `NonEmptyCap` and `b` in `BoxCap`: `a.contains(a)`; the full cap around any
/// box point contains `b`; `b` contains the empty cap around any box point.
#[kani::proof]
fn p_cprel_01() {
    let (a, b) = (any_nonempty_cap(), any_box_cap());
    assert!(a.contains(a));
    let p = any_box_point();
    assert!(Cap::from_center_chord_angle(p, ChordAngle::STRAIGHT).contains(b));
    assert!(b.contains(Cap::from_center_chord_angle(p, ChordAngle::NEGATIVE)));
}

/// P-CPREL-02 (law): containing a non-empty cap implies intersecting it; the interior
/// intersecting implies intersecting; the empty cap intersects nothing; a cap without interior
/// (radius at most zero) interior-intersects nothing.
///
/// For every `a` in `BoxCap` and `b` in `NonEmptyCap`: `a.contains(b) ⇒ a.intersects(b)`;
/// `a.interior_intersects(b) ⇒ a.intersects(b)`; the empty cap intersects no box cap; if
/// `a`'s length² is at most 0, `!a.interior_intersects(b)` (upstream's "this cap has an
/// interior" check, `s2cap.cc:83`).
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02), `pure_add` (P-CAARI-01, L-CP-08, L-CP-05): the two
/// tests compute the same distance and sums (research R0b, 20.3 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
fn p_cprel_02() {
    stubs::reset();
    let (a, b) = (any_box_cap(), any_nonempty_cap());
    if a.contains(b) {
        assert!(a.intersects(b));
    }
    if a.interior_intersects(b) {
        assert!(a.intersects(b));
    }
    if a.chord_radius().length2() <= 0.0 {
        assert!(!a.interior_intersects(b));
    }
    assert!(!Cap::empty().intersects(a));
    kani::cover!(a.contains(b), "a contained cap");
}

/// P-CPREL-03 (law): `intersects` is symmetric.
///
/// For every `a`, `b` in `BoxCap`: `a.intersects(b) == b.intersects(a)`.
///
/// Stubs: `pure_chord_sym` (L-CP-01 to L-CP-04: the chord's symmetry), `pure_add_sym`
/// (P-CAARI-01, L-CP-08, L-CP-05, L-CP-06: the sum's commutativity, L-CP-06a weekly) (research
/// R0c, 27.0 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord_sym)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add_sym)]
fn p_cprel_03() {
    stubs::reset();
    let (a, b) = (any_box_cap(), any_box_cap());
    assert!(a.intersects(b) == b.intersects(a));
    kani::cover!(a.intersects(b), "intersecting caps");
}
