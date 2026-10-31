// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-LLCON: `LatLng`'s constructors (spec 010 data-model §4.1).

use s2rst::s1::Angle;
use s2rst::s2::LatLng;

use crate::latlng_spec;

fn bits(ll: LatLng) -> (u64, u64) {
    (ll.lat.radians().to_bits(), ll.lng.radians().to_bits())
}

/// Bit for bit, any two NaNs alike (the checker may give a NaN any payload).
fn same(x: f64, y: f64) -> bool {
    x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
}

/// `ll` is the pair `(a, b)`.
fn is(ll: LatLng, a: Angle, b: Angle) -> bool {
    same(ll.lat.radians(), a.radians()) && same(ll.lng.radians(), b.radians())
}

/// P-LLCON-01 (correct result): `new` and `from_radians` keep their arguments; the other
/// constructors are the pairs of `Angle`'s; `invalid()` and the default are upstream's
/// constants.
///
/// Exactly: for every `f64` pair, `i32` pair and `u32` pair, bit for bit: `new(a, b)` and
/// `from_radians(x, y)` are `(x, y)`; `from_degrees`, `from_e5/e6/e7`, `from_unsigned_e6/e7` are
/// `(Angle::from_*(x), Angle::from_*(y))`; `invalid()` is `latlng_spec::INVALID`; the default is
/// `(+0, +0)`. Any two NaNs count as equal.
///
/// Stubs: `Angle`'s five arithmetic constructors, uninterpreted (`pure_angle_from_*`: any angle,
/// one per argument), so the statement is about `LatLng`'s constructors, not a second copy of
/// `Angle`'s arithmetic: that copy timed out at 12 minutes (2026-10-28). `Angle`'s constructors
/// mean what spec 007 proves (P-AGCNV-01, P-AGEF-01), so the entry is proven by composition.
#[kani::proof]
#[kani::stub(s2rst::s1::Angle::from_degrees, crate::stubs::pure_angle_from_degrees)]
#[kani::stub(s2rst::s1::Angle::from_e5, crate::stubs::pure_angle_from_e5)]
#[kani::stub(s2rst::s1::Angle::from_e6, crate::stubs::pure_angle_from_e6)]
#[kani::stub(s2rst::s1::Angle::from_e7, crate::stubs::pure_angle_from_e7)]
#[kani::stub(
    s2rst::s1::Angle::from_unsigned_e6,
    crate::stubs::pure_angle_from_unsigned_e6
)]
#[kani::stub(
    s2rst::s1::Angle::from_unsigned_e7,
    crate::stubs::pure_angle_from_unsigned_e7
)]
fn p_llcon_01() {
    crate::stubs::reset();
    let (x, y): (f64, f64) = (kani::any(), kani::any());
    let xy = (x.to_bits(), y.to_bits());
    assert!(bits(LatLng::new(Angle::from_radians(x), Angle::from_radians(y))) == xy);
    assert!(bits(LatLng::from_radians(x, y)) == xy);
    assert!(is(
        LatLng::from_degrees(x, y),
        Angle::from_degrees(x),
        Angle::from_degrees(y)
    ));
    let (i, j): (i32, i32) = (kani::any(), kani::any());
    assert!(is(
        LatLng::from_e5(i, j),
        Angle::from_e5(i),
        Angle::from_e5(j)
    ));
    assert!(is(
        LatLng::from_e6(i, j),
        Angle::from_e6(i),
        Angle::from_e6(j)
    ));
    assert!(is(
        LatLng::from_e7(i, j),
        Angle::from_e7(i),
        Angle::from_e7(j)
    ));
    let (u, v): (u32, u32) = (kani::any(), kani::any());
    assert!(is(
        LatLng::from_unsigned_e6(u, v),
        Angle::from_unsigned_e6(u),
        Angle::from_unsigned_e6(v)
    ));
    assert!(is(
        LatLng::from_unsigned_e7(u, v),
        Angle::from_unsigned_e7(u),
        Angle::from_unsigned_e7(v)
    ));
    let inv = latlng_spec::INVALID;
    assert!(bits(LatLng::invalid()) == (inv.0.to_bits(), inv.1.to_bits()));
    assert!(bits(LatLng::default()) == (0, 0));
}
