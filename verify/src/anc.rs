// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Common-ancestor properties P-ANC-* (FR-005), stated against the reference model.
//!
//! Upstream finding U-002 shaped the domain: C++ `GetCommonAncestorLevel` assumes
//! `bits != 0` ("because lsb() is non-zero"), which holds for valid cells. These properties
//! work over Valid × Valid, which never contains `none`.

use s2rst::s2::Level;

use crate::domain::any_valid;
use crate::spec;

/// P-ANC-01 (correct result): `common_ancestor_level(b)` is the finest level at which the two cells
/// have the same ancestor, and none on different faces.
///
/// `a.common_ancestor_level(b).map(u8::from) == spec::common_ancestor_level(da, db)`. Cites U-002.
#[kani::proof]
#[kani::unwind(32)]
fn p_anc_01() {
    let (a, da) = any_valid();
    let (b, db) = any_valid();
    let got = a.common_ancestor_level(b).map(Level::as_u8);
    assert_eq!(got, spec::common_ancestor_level(da, db));
    kani::cover!(got.is_none(), "different faces");
    kani::cover!(got == Some(30), "same leaf");
    kani::cover!(got == Some(0), "only the face");
}

/// P-ANC-02 (law): the common ancestor level is symmetric, a cell's with itself is its own level,
/// and the two cells have the same ancestor at every level up to it.
///
/// Symmetric (`cal(a, b) == cal(b, a)`) and reflexive (`cal(a, a) == Some(a.level())`), and
/// `a.parent_at_level(l) == b.parent_at_level(l)` for every `l ≤` the result. Cites U-002.
#[kani::proof]
fn p_anc_02() {
    let (a, _) = any_valid();
    let (b, _) = any_valid();
    let r = a.common_ancestor_level(b);
    assert_eq!(r, b.common_ancestor_level(a));
    assert_eq!(a.common_ancestor_level(a), Some(a.level()));
    if let Some(top) = r {
        let l: u8 = kani::any();
        kani::assume(l <= top.as_u8());
        let l = Level::new(l);
        assert_eq!(a.parent_at_level(l), b.parent_at_level(l));
        kani::cover!(l == top, "at the common ancestor level");
    }
    kani::cover!(r.is_some(), "same face");
}
