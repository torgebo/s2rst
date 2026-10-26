// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap` measures: height, area, centroid (P-CPMEA, spec 008 data-model §4).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::cap_spec;
use crate::value_domain::any_box_cap;

/// P-CPMEA-01 (exact output): `height` is upstream's formula, bit for bit, and the area is
/// never negative.
///
/// For every `a` in `BoxCap` with length² `l`: `a.height()` is `upstream_height(l)`, bit for bit,
/// and `a.area() >= 0`.
#[kani::proof]
fn p_cpmea_01() {
    let a = any_box_cap();
    let l = a.chord_radius().length2();
    assert!(a.height().to_bits() == cap_spec::upstream_height(l).to_bits());
    assert!(a.area() >= 0.0);
}

/// P-CPMEA-01 (exact output), the area: `area` is upstream's formula, bit for bit. Weekly: the
/// product compared with its identical restatement timed out at 12 minutes (2026-10-24), as
/// spec 007's products did.
///
/// For every `a` in `BoxCap`: `a.area()` is `upstream_area(l)`, bit for bit.
///
/// It found D-009: for r² = −0 the library's `0.0_f64.max(h)` was −0 in a debug build, upstream's
/// `std::max` +0. Proven since the fix (1195.3 s, 2026-10-25).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_cpmea_01__area() {
    let a = any_box_cap();
    let l = a.chord_radius().length2();
    assert!(a.area().to_bits() == cap_spec::upstream_area(l).to_bits());
}

/// P-CPMEA-01 (exact output), the centroid: `centroid` is upstream's formula, bit for bit.
/// Weekly, as `__area`.
///
/// For every `a` in `BoxCap`: `a.centroid()` is `upstream_centroid(centre, l)`, bit for bit.
#[cfg(feature = "slow")]
#[kani::proof]
fn p_cpmea_01__centroid() {
    let a = any_box_cap();
    let (c, l) = (a.center(), a.chord_radius().length2());
    let (got, want) = (a.centroid(), cap_spec::upstream_centroid(c, l));
    assert!(got.0.x.to_bits() == want.0.x.to_bits());
    assert!(got.0.y.to_bits() == want.0.y.to_bits());
    assert!(got.0.z.to_bits() == want.0.z.to_bits());
}
