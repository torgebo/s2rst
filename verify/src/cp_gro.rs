// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap` growth: `add_point`, `add_cap`, `expanded` (P-CPGRO, spec 008 data-model §4).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point, any_finite_nonneg_angle};

/// P-CPGRO-01 (correct result): after `add_point(p)` the cap contains `p` (upstream's own claim:
/// `AddPoint` and `Contains` compute the same distance), and its radius did not shrink.
///
/// For every `a` in `BoxCap` and `p` in `BoxPoint`: `a.add_point(p).contains_point(p)`, and
/// its radius is at least `a`'s.
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02) (research R0c, 6.8 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
fn p_cpgro_01() {
    stubs::reset();
    let (a, p) = (any_box_cap(), any_box_point());
    let g = a.add_point(p);
    assert!(g.contains_point(p));
    assert!(g.chord_radius() >= a.chord_radius());
    kani::cover!(
        g.chord_radius() > a.chord_radius() && !a.is_empty(),
        "a grown cap"
    );
}

/// P-CPGRO-02 (correct result): after `add_cap(b)` the cap contains `b` (the claim upstream's
/// round-up keeps), and its radius did not shrink, unless the cap was empty (then it is `b`).
///
/// For every `a`, `b` in `BoxCap`: `a.add_cap(b).contains(b)`, and if `a` is not empty its
/// radius is at least `a`'s.
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02), `pure_add` (P-CAARI-01, L-CP-08, L-CP-05) (research
/// R0c, 17.0 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
fn p_cpgro_02() {
    stubs::reset();
    let (a, b) = (any_box_cap(), any_box_cap());
    let g = a.add_cap(b);
    assert!(g.contains(b));
    if !a.is_empty() {
        assert!(g.chord_radius() >= a.chord_radius());
    }
}

/// P-CPGRO-03 (correct result): `expanded` by a finite non-negative distance contains the
/// original cap; an empty cap stays empty.
///
/// For every `a` in `BoxCap` and `d` in `FiniteNonNegAngle`: `a.expanded(d).contains(a)`, and
/// if `a` is empty so is the result.
///
/// Stubs: `pure_chord` (L-CP-01), `pure_add` (P-CAARI-01, L-CP-08, L-CP-05).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
#[kani::stub(<s2rst::s1::ChordAngle as std::ops::Add>::add, crate::stubs::pure_add)]
fn p_cpgro_03() {
    stubs::reset();
    let (a, d) = (any_box_cap(), any_finite_nonneg_angle());
    let g = a.expanded(d);
    assert!(g.contains(a));
    if a.is_empty() {
        assert!(g.is_empty());
    }
}

/// P-CPGRO-03 (correct result) on the real code, without stubs: as `p_cpgro_03`. Weekly
/// (research R0, 336.9 s).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_cpgro_03__real() {
    let (a, d) = (any_box_cap(), any_finite_nonneg_angle());
    assert!(a.expanded(d).contains(a));
}
