// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Rectangle bounds, vertices, centre and size (P-LBND, spec 005 data-model §4.2).

use s2rst::s2::{LatLng, RectVertex};

use crate::value_domain::any_rect;

fn bits(ll: LatLng) -> (u64, u64) {
    (ll.lat.radians().to_bits(), ll.lng.radians().to_bits())
}

/// P-LBND-01 (correct result): the low and high corners, the four vertices, the centre and the
/// size are the corresponding combinations of the two intervals' bounds, centres and lengths,
/// with the vertices in upstream's counter-clockwise order (`GetVertex`, `s2latlng_rect.cc:79`).
///
/// For every valid `r`, bit for bit: `lo() = (lat.lo, lng.lo)`; `hi() = (lat.hi, lng.hi)`;
/// vertices 0–3 are `(lat.lo, lng.lo)`, `(lat.lo, lng.hi)`, `(lat.hi, lng.hi)`,
/// `(lat.hi, lng.lo)`; `center() = (lat.center(), lng.center())`;
/// `size() = (lat.length(), lng.length())`.
///
/// Weekly: it took 693.6 s per change (2026-10-13) and timed out at 12 minutes twice, in
/// spec 005's first run and in spec 006's final run of all six parts (2026-10-14).
#[kani::proof]
fn p_lbnd_01() {
    let r = any_rect();
    let (lat, lng) = (r.lat, r.lng);
    let pair = |a: f64, b: f64| (a.to_bits(), b.to_bits());
    assert_eq!(bits(r.lo()), pair(lat.lo, lng.lo));
    assert_eq!(bits(r.hi()), pair(lat.hi, lng.hi));
    assert_eq!(bits(r.vertex(RectVertex::LowerLeft)), pair(lat.lo, lng.lo));
    assert_eq!(bits(r.vertex(RectVertex::LowerRight)), pair(lat.lo, lng.hi));
    assert_eq!(bits(r.vertex(RectVertex::UpperRight)), pair(lat.hi, lng.hi));
    assert_eq!(bits(r.vertex(RectVertex::UpperLeft)), pair(lat.hi, lng.lo));
    assert_eq!(bits(r.center()), pair(lat.center(), lng.center()));
    assert_eq!(bits(r.size()), pair(lat.length(), lng.length()));
}
