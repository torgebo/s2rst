// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap::complement` (P-CPCMP, spec 008 data-model §4).

use crate::cap_spec;
use crate::stubs;
use crate::value_domain::{any_box_cap, any_valid_cap};

/// P-CPCMP-01 (exact output): the complement is upstream's: centre `−c` and radius
/// `FromLength2(4 − r²)`, bit for bit; the full and empty caps are each other's complement.
///
/// For every `a` in `BoxCap`: if `a` is full, the complement is empty; if empty, full; otherwise
/// its centre is `−a.center()` and its length² is `upstream_complement_length2(r²)`, bit for bit.
#[kani::proof]
fn p_cpcmp_01() {
    let a = any_box_cap();
    let k = a.complement();
    if cap_spec::full(a) {
        assert!(k.is_empty());
    } else if cap_spec::empty(a) {
        assert!(k.is_full());
    } else {
        let (c, kc) = (a.center(), k.center());
        assert!(kc.0.x.to_bits() == (-c.0.x).to_bits());
        assert!(kc.0.y.to_bits() == (-c.0.y).to_bits());
        assert!(kc.0.z.to_bits() == (-c.0.z).to_bits());
        let want = cap_spec::upstream_complement_length2(cap_spec::radius2(a));
        assert!(k.chord_radius().length2().to_bits() == want.to_bits());
    }
}

/// P-CPCMP-02 (validity): the complement of a valid cap is valid.
///
/// For every `a` in `ValidCap`: `a.complement().is_valid()`.
///
/// Stubs: `pure_is_unit` (L-CP-07, L-CP-04: the negated centre has the same squared length)
/// (research R0c, 4.7 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn p_cpcmp_02() {
    stubs::reset();
    let a = any_valid_cap();
    assert!(a.complement().is_valid());
}

/// P-CPCMP-03 (correct result): the complement of the complement is the cap where `4 − r²` is
/// exact, in particular for every r² ≥ 2; elsewhere its length² is within 2⁻⁵¹ of the original.
/// The exceptions are upstream's: a single-point cap's complement is full (r² = 0), and so is
/// that of a cap with r² below half an ulp of 4, whose `4 − r²` rounds to 4.
///
/// For every `a` in `BoxCap` neither empty nor full, with `r² ≥ 2⁻⁵¹`: the double complement's
/// centre is `a`'s bit for bit; its length² is `r²` bit for bit when `r² ≥ 2`, and within 2⁻⁵¹
/// of it otherwise.
#[kani::proof]
fn p_cpcmp_03() {
    let a = any_box_cap();
    let r = cap_spec::radius2(a);
    kani::assume(
        !cap_spec::empty(a) && !cap_spec::full(a) && r >= f64::from_bits((1023 - 51) << 52),
    );
    let k = a.complement().complement();
    assert!(k.center().0.x.to_bits() == a.center().0.x.to_bits());
    assert!(k.center().0.z.to_bits() == a.center().0.z.to_bits());
    let l = k.chord_radius().length2();
    if r >= 2.0 {
        assert!(l.to_bits() == r.to_bits());
    } else {
        assert!((l - r).abs() <= f64::from_bits((1023 - 51) << 52));
    }
    kani::cover!(r < 2.0 && l != r, "an inexact double complement");
}
