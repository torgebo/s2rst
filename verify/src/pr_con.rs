// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `PointRegion`'s construction (spec 011 data-model §4.1).

use s2rst::s2::point_region::PointRegion;

use crate::value_domain::any_triple;

/// P-PRCON-01 (correct result): the region keeps the point it was given, bit for bit, as
/// upstream's constructor keeps `point_`.
///
/// Exactly: for every triple `p`, `PointRegion::new(p).point()` has `p`'s bits.
#[kani::proof]
fn p_prcon_01() {
    let p = any_triple();
    let q = PointRegion::new(p).point();
    assert!(
        q.0.x.to_bits() == p.0.x.to_bits()
            && q.0.y.to_bits() == p.0.y.to_bits()
            && q.0.z.to_bits() == p.0.z.to_bits()
    );
}
