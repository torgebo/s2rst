// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Regression tests for `LatLng`'s survey findings (spec 010 FR-017, FR-018): `normalized`
//! reduces the longitude with upstream's exact `remainder(lng, 2π)`, and `get_distance` clamps
//! before the square root. The expected values are upstream's, from the survey probe
//! (`specs/010-formal-verify-latlng/probe/cpp/survey_cpp.txt`, sections N, DS and S).

use super::*;
use quickcheck_macros::quickcheck;

/// Upstream's `remainder(x, 2π)`: the exact IEEE-754 remainder, ties to even, the sign of a
/// zero result kept (unlike `Angle::normalized`, no −π to π).
fn remainder_2pi(x: f64) -> f64 {
    crate::test_util::ieee_remainder_2pi(x)
}

/// The probe's special longitudes and upstream's normalised longitudes (section N, rows 0–13),
/// with the latitude 0.3 kept.
#[test]
fn normalized_matches_cpp() {
    for (lng, want) in [
        (PI, 0x4009_21fb_5444_2d18_u64),
        (-PI, 0xc009_21fb_5444_2d18),
        (3.0 * PI, 0xc009_21fb_5444_2d18),
        (-3.0 * PI, 0x4009_21fb_5444_2d18),
        (5.0 * PI, 0x4009_21fb_5444_2d18),
        (2.0 * PI, 0x0000_0000_0000_0000),
        (-2.0 * PI, 0x8000_0000_0000_0000),
        (PI + 1e-15, 0xc009_21fb_5444_2d16),
        (1e10, 0xbfe0_4b9e_24d9_d140),
        (1e16, 0x4005_1912_8f76_be50),
        (-2.462_748_394_469_102e16, 0xc009_1eb0_c183_0050),
        (1e300, 0xbfe7_264f_c07a_22c0),
        (0.0, 0x0000_0000_0000_0000),
        (-0.0, 0x8000_0000_0000_0000),
    ] {
        let n = LatLng::from_radians(0.3, lng).normalized();
        assert_eq!(n.lat.radians(), 0.3, "latitude for {lng:e}");
        assert_eq!(
            n.lng.radians().to_bits(),
            want,
            "normalized longitude of {lng:e}: {}",
            n.lng.radians()
        );
        assert!(n.is_valid(), "normalized({lng:e}) is not valid");
    }
}

/// K1's invalid result: the longitude 10¹⁶ used to normalise to −4.0.
#[test]
fn normalized_of_1e16_is_valid() {
    let n = LatLng::from_radians(0.3, 1e16).normalized();
    assert_eq!(n.lng.radians(), 2.637_242_432_414_304);
}

/// Upstream's `ToStringInDegrees` keeps the sign of a zero longitude (section S, row 3).
#[test]
fn to_string_in_degrees_keeps_negative_zero() {
    assert_eq!(
        LatLng::from_degrees(-0.0, -0.0).to_string_in_degrees(),
        "-0.000000,-0.000000"
    );
}

/// The ties of the remainder: an odd multiple of π keeps π when the quotient rounds to an
/// even number of turns, and becomes −π (with the sign of the input) when it rounds to an odd
/// one. `f64` holds `(2k + 1)·π` exactly only for small `k`; the description decides.
#[test]
fn normalized_ties_round_to_even() {
    for k in [1.0, 3.0, 5.0, 7.0] {
        for x in [k * PI, -k * PI] {
            let n = LatLng::from_radians(0.0, x).normalized().lng.radians();
            assert_eq!(n.to_bits(), remainder_2pi(x).to_bits(), "{k}π");
            assert_eq!(n.abs(), PI, "{k}π");
        }
    }
}

/// FR-017: the longitude is the exact remainder, bit for bit, at every magnitude (drawn as a
/// bit pattern, so every binade is as likely as any other), and the result is valid for every
/// finite input; a non-finite input gives `invalid()`.
#[quickcheck]
fn prop_normalized_is_the_exact_remainder(lat_bits: u64, lng_bits: u64) -> bool {
    let (lat, lng) = (f64::from_bits(lat_bits), f64::from_bits(lng_bits));
    let n = LatLng::from_radians(lat, lng).normalized();
    if !lat.is_finite() || !lng.is_finite() {
        let inv = LatLng::invalid();
        return n.lat.radians().to_bits() == inv.lat.radians().to_bits()
            && n.lng.radians().to_bits() == inv.lng.radians().to_bits();
    }
    n.lng.radians().to_bits() == remainder_2pi(lng).to_bits()
        && n.lat.radians().to_bits() == lat.clamp(-FRAC_PI_2, FRAC_PI_2).to_bits()
        && n.is_valid()
}

/// A valid `LatLng` normalises to itself, bit for bit.
#[quickcheck]
fn prop_normalized_keeps_valid_input(lat: f64, lng: f64) -> bool {
    let ll = LatLng::from_radians(lat, lng);
    if !ll.is_valid() {
        return true;
    }
    let n = ll.normalized();
    n.lat.radians().to_bits() == lat.to_bits() && n.lng.radians().to_bits() == lng.to_bits()
}

/// FR-018 (K3): one point written twice with an out-of-range latitude makes the haversine
/// term negative after rounding; upstream's `sqrt(min(1, x))` gives NaN there (section DS,
/// row 5), where clamping after the root gave π.
#[test]
fn get_distance_clamps_before_the_root() {
    let a = LatLng::from_radians(3.124_793_493_547_795, 0.0);
    let b = LatLng::from_radians(0.016_799_160_041_997_9, PI);
    assert!(a.get_distance(b).radians().is_nan());
}

/// The other out-of-range rows of section DS are upstream's too.
#[test]
fn get_distance_out_of_range_matches_cpp() {
    for ((a1, a2), (b1, b2), want) in [
        ((2.0, 0.0), (0.0, 0.0), 0x4000_0000_0000_0000_u64),
        ((2.0, 0.0), (0.0, 1.0), 0x3ffc_c2e3_a649_5132),
        ((-2.0, 0.5), (1.0, 2.0), 0x4003_bcb6_c9d0_667d),
        ((f64::NAN, 0.0), (0.0, 0.0), 0x4009_21fb_5444_2d18),
        ((0.0, 4.0), (0.0, -4.0), 0x3ffb_7812_aeef_4b9e),
    ] {
        let d = LatLng::from_radians(a1, a2).get_distance(LatLng::from_radians(b1, b2));
        assert_eq!(d.radians().to_bits(), want, "({a1}, {a2}) to ({b1}, {b2})");
    }
}

/// `Deserialize` is derived, with no validity check: every finite pair, valid or not, is
/// accepted and comes back bit for bit (P-LLDES-01). The round trip goes through a
/// `serde_json::Value`, which holds the `f64`s themselves: JSON text would test
/// `serde_json`'s decimal parser, which is not correctly rounded.
#[cfg(feature = "serde")]
#[quickcheck]
fn prop_serde_accepts_every_pair(lat_bits: u64, lng_bits: u64) -> bool {
    let (lat, lng) = (f64::from_bits(lat_bits), f64::from_bits(lng_bits));
    if !lat.is_finite() || !lng.is_finite() {
        // JSON has no NaN or infinities; serde_json writes them as null.
        return true;
    }
    let ll = LatLng::from_radians(lat, lng);
    let back: LatLng = serde_json::from_value(serde_json::to_value(ll).unwrap()).unwrap();
    back.lat.radians().to_bits() == lat.to_bits() && back.lng.radians().to_bits() == lng.to_bits()
}

/// `from_point` is valid for every point without a NaN coordinate, infinities and the zero
/// vector included, and invalid with one (P-LLFP-02, here on the real `atan2`).
#[quickcheck]
fn prop_from_point_valid_without_nan(x: u64, y: u64, z: u64) -> bool {
    let v = Vector {
        x: f64::from_bits(x),
        y: f64::from_bits(y),
        z: f64::from_bits(z),
    };
    let nan = v.x.is_nan() || v.y.is_nan() || v.z.is_nan();
    LatLng::from_point(Point(v)).is_valid() != nan
}

#[test]
fn from_point_valid_at_the_special_vectors() {
    let (inf, sub) = (f64::INFINITY, f64::from_bits(1));
    for (x, y, z) in [
        (0.0, 0.0, 0.0),
        (-0.0, -0.0, -0.0),
        (inf, inf, inf),
        (-inf, 0.0, inf),
        (-inf, -inf, -inf),
        (sub, -sub, sub),
        (-1.0, -0.0, 0.0),
        (f64::MAX, f64::MAX, -f64::MAX),
    ] {
        let ll = LatLng::from_point(Point(Vector { x, y, z }));
        assert!(ll.is_valid(), "({x}, {y}, {z}) gives {ll:?}");
    }
    let ll = LatLng::from_point(Point(Vector {
        x: 0.0,
        y: 0.0,
        z: f64::NAN,
    }));
    assert!(!ll.is_valid());
}
