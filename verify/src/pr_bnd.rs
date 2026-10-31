// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `PointRegion`'s bounds (spec 011 data-model §4.1): upstream's `GetCapBound`
//! (`S2Cap::FromPoint`), `GetRectBound` (`S2LatLngRect(ll, ll)`) and `GetCellUnionBound` (the
//! cap's covering).

use s2rst::s2::point_region::PointRegion;
use s2rst::s2::{LatLng, Region};
use std::f64::consts::PI;

use crate::rect_spec;
use crate::value_domain::{any_plain_triple, any_triple};

/// P-PRBND-01 (correct result): the bounding cap is the point's zero-radius cap: its centre is
/// the point bit for bit and its squared chord radius +0.
///
/// Exactly: for every triple `p`, `cap_bound()` has centre bits `p`'s and `length2` bits 0.
#[kani::proof]
fn p_prbnd_01() {
    let p = any_triple();
    let c = PointRegion::new(p).cap_bound();
    let q = c.center();
    assert!(
        q.0.x.to_bits() == p.0.x.to_bits()
            && q.0.y.to_bits() == p.0.y.to_bits()
            && q.0.z.to_bits() == p.0.z.to_bits()
    );
    assert!(c.chord_radius().length2().to_bits() == 0);
}

/// P-PRBND-02 (correct result): the bounding rectangle is the single `LatLng` of the point, a
/// valid rectangle: both latitude bounds the latitude, both longitude bounds the longitude
/// (π for −π, as `s1::Interval::from_point`), for every triple without a NaN coordinate.
///
/// Under contract: `pure_from_point` (P-LLFP-02: the `LatLng` of a point without NaN is valid;
/// one value per point, so the statement reads the library's).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::LatLng::from_point, crate::stubs::pure_from_point)]
fn p_prbnd_02() {
    crate::stubs::reset();
    let p = any_plain_triple();
    let b = PointRegion::new(p).rect_bound();
    let ll = LatLng::from_point(p);
    let (lat, lng) = (ll.lat.radians(), ll.lng.radians());
    let lng = if lng == -PI { PI } else { lng };
    assert!(b.lat.lo.to_bits() == lat.to_bits() && b.lat.hi.to_bits() == lat.to_bits());
    assert!(b.lng.lo.to_bits() == lng.to_bits() && b.lng.hi.to_bits() == lng.to_bits());
    assert!(rect_spec::valid(b));
}

/// P-PRBND-03 (correct result): the covering is three or four cells at level 29, one of them the
/// point's leaf's level-29 ancestor (upstream: the zero-radius cap's level is
/// `kMinWidth.GetLevelForMinValue(0) − 1 = 29`, and its cells are those around the leaf's
/// closest vertex).
///
/// Exactly: for every triple, `cell_union_bound()` has 3 or 4 ids, each at level 29, and one
/// equals `CellId::from_point(p).parent_at_level(29)`.
///
/// Under contract: `pure_leaf` (L-PR-01), `asin_contract`. Weekly (R0b: 563.4 s for the crash).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(s2rst::s2::CellId::from_point, crate::stubs::pure_leaf)]
fn p_prbnd_03() {
    crate::stubs::reset();
    let p = any_triple();
    let ids = PointRegion::new(p).cell_union_bound();
    let l29 = s2rst::s2::Level::new(29);
    let parent = s2rst::s2::CellId::from_point(&p).parent_at_level(l29);
    assert!(ids.len() == 3 || ids.len() == 4);
    let mut found = false;
    for k in 0..4 {
        if k < ids.len() {
            assert!(ids[k].level() == l29);
            if ids[k] == parent {
                found = true;
            }
        }
    }
    assert!(found);
}
