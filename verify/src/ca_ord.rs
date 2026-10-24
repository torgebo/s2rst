// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's order (P-CAORD, spec 007 data-model §4.1): with only canonical values
//! (option B, FR-017), equality and the ordering are a total order, spec 006's generic order
//! laws and spec 004's equality laws, and the order is `chord_spec`'s.

use s2rst::s1::ChordAngle;
use std::cmp::Ordering;

use crate::value_domain::any_chord;
use crate::{chord_spec, laws};

/// P-CAORD-01 (law): equality and the ordering are a total order on the canonical values, and
/// the order is the reals' (−1 below every length, +∞ above).
///
/// For every canonical `a`, `b`, `c`: `laws::ord::all` with `partial_cmp(..).unwrap()` (never
/// `None`), `laws::eq::all`, and `a.partial_cmp(b)` is `Less` exactly when
/// `chord_spec::less(a, b)` and `Greater` exactly when `chord_spec::less(b, a)`.
#[kani::proof]
fn p_caord_01() {
    let (a, b, c) = (any_chord(), any_chord(), any_chord());
    assert!(a.partial_cmp(&b).is_some());
    let cmp = |x: &ChordAngle, y: &ChordAngle| x.partial_cmp(y).unwrap();
    laws::ord::all(&cmp, &a, &b, &c);
    laws::eq::all(&a, &b, &c);
    let o = cmp(&a, &b);
    assert!((o == Ordering::Less) == chord_spec::less(a.length2(), b.length2()));
    assert!((o == Ordering::Greater) == chord_spec::less(b.length2(), a.length2()));
    kani::cover!(
        a == ChordAngle::NEGATIVE && o == Ordering::Less,
        "NEGATIVE below"
    );
    kani::cover!(
        a.length2() == 0.0 && b.length2() == 0.0 && a.length2().to_bits() != b.length2().to_bits(),
        "the two zeros are equal"
    );
}
