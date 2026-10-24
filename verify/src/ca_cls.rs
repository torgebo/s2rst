// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The chord angle's classifications (P-CACLS, spec 007 data-model §4.1), against upstream's
//! definitions on the canonical values (`s1chord_angle.h:361–446`).

use crate::chord_spec;
use crate::value_domain::any_chord;

/// P-CACLS-01 (correct result): `is_zero`, `is_negative`, `is_infinity`, `is_special` and
/// `is_valid` are upstream's definitions; `is_infinity` holds at +∞ only, and `is_valid` for
/// every value.
///
/// For every canonical `c` with length `l`: `is_zero` is `l == 0`, `is_negative` is `l < 0`,
/// `is_infinity` is `l == +∞`, `is_special` is `chord_spec::special(l)`, and `is_valid` holds.
#[kani::proof]
fn p_cacls_01() {
    let c = any_chord();
    let l = c.length2();
    assert!(c.is_zero() == (l == 0.0));
    assert!(c.is_negative() == (l < 0.0));
    assert!(c.is_infinity() == (l == f64::INFINITY));
    assert!(c.is_special() == chord_spec::special(l));
    assert!(c.is_valid());
    kani::cover!(c.is_special() && c.is_negative(), "NEGATIVE");
    kani::cover!(c.is_infinity(), "INFINITY");
}
