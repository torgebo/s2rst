// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's neighbours (P-CANBR, spec 007 data-model §4.1), against `chord_spec::next`
//! and `prev`, the adjacent canonical values computed from bit patterns.

use s2rst::s1::ChordAngle;

use crate::chord_spec;
use crate::value_domain::any_chord;

/// P-CANBR-01 (correct result): `successor` is the least canonical value above, `INFINITY` at
/// the top.
///
/// For every canonical `c`: `c.successor().length2() == chord_spec::next(c.length2())` (as
/// values; the zeros' signs are not fixed).
#[kani::proof]
fn p_canbr_01() {
    let c = any_chord();
    let s = c.successor().length2();
    assert!(s == chord_spec::next(c.length2()));
    kani::cover!(c == ChordAngle::STRAIGHT, "STRAIGHT");
    kani::cover!(c.length2() > 0.0 && c.length2() < 4.0, "inside");
}

/// P-CANBR-02 (correct result): `predecessor` is the greatest canonical value below,
/// `NEGATIVE` at the bottom.
///
/// For every canonical `c`: `c.predecessor().length2() == chord_spec::prev(c.length2())` (as
/// values).
#[kani::proof]
fn p_canbr_02() {
    let c = any_chord();
    let p = c.predecessor().length2();
    assert!(p == chord_spec::prev(c.length2()));
    kani::cover!(c == ChordAngle::ZERO, "ZERO");
    kani::cover!(c.length2().to_bits() == 1, "the smallest length");
}

/// P-CANBR-03 (law): the neighbours undo each other below the top and above the bottom.
///
/// For every canonical `c`: `c.successor().predecessor() == c` unless `c` is `INFINITY`, and
/// `c.predecessor().successor() == c` unless `c` is `NEGATIVE` (as values).
#[kani::proof]
fn p_canbr_03() {
    let c = any_chord();
    if c != ChordAngle::INFINITY {
        assert!(c.successor().predecessor() == c);
    }
    if c != ChordAngle::NEGATIVE {
        assert!(c.predecessor().successor() == c);
    }
    kani::cover!(c.length2() == 4.0, "STRAIGHT");
}
