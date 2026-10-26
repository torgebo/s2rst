// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap` classifications (P-CPCLS, spec 008 data-model §4).

use s2rst::s2::Cap;

use crate::cap_spec;
use crate::stubs;
use crate::value_domain::any_box_cap;

/// P-CPCLS-01 (correct result): `is_valid`, `is_empty` and `is_full` are upstream's; empty and
/// full exclude each other; the constants are valid.
///
/// For every `a` in `BoxCap`: `a.is_valid() == cap_spec::valid(a)`, `a.is_empty() ==
/// cap_spec::empty(a)`, `a.is_full() == cap_spec::full(a)`, and not both empty and full.
///
/// Stubs: `pure_is_unit` (L-CP-07, L-CP-04): the library and the description test the centre's
/// unit length once each.
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn p_cpcls_01() {
    stubs::reset();
    let a = any_box_cap();
    assert!(a.is_valid() == cap_spec::valid(a));
    assert!(a.is_empty() == cap_spec::empty(a));
    assert!(a.is_full() == cap_spec::full(a));
    assert!(!(a.is_empty() && a.is_full()));
    assert!(Cap::empty().is_valid() && Cap::full().is_valid());
    kani::cover!(a.is_valid(), "a valid cap");
    kani::cover!(!a.is_valid(), "a cap with a non-unit centre");
}
