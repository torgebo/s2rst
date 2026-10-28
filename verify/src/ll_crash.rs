// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses `C-s2::LatLng::<op>` for the latitude–longitude's in-scope entries
//! (spec 010 US1, FR-004; data-model §5).
//!
//! A `LatLng` has no validity invariant, so its domain is every pair of `f64` (`AnyLatLng2`),
//! narrowed only where an operation would produce a new NaN, which Kani reports as a failed
//! check (`∞ − ∞`, `0 · ∞`), or by a documented precondition (`get_distance`: both normalised).
//! The entries through `atan2`, `sin_cos`, `sin`, `cos` and `asin` run under their function
//! contracts (spec 008 FR-003, the 2026-10-26 coverage review); each harness names its stubs.
//! `normalized`'s proof does not depend on the value of `%`, which Kani models wrongly: every
//! operation after it is a comparison, a negation, or an addition or subtraction of a finite
//! value and the constant 2π.

use s2rst::s1::Angle;
use s2rst::s2::{LatLng, Point};

use crate::value_domain::{any_lat_lng_bits, any_triple, any_valid_lat_lng};

/// Whether `a + b` is `∞ − ∞` (NaN): opposite infinities.
fn opposite_infinities(a: f64, b: f64) -> bool {
    a.is_infinite() && a == -b
}

/// Whether `a − b` is `∞ − ∞` (NaN): equal infinities.
fn equal_infinities(a: f64, b: f64) -> bool {
    a.is_infinite() && a == b
}

/// C-s2::LatLng::new, from_radians, from_degrees, invalid, Default, is_valid and PartialEq:
/// complete without panic, arithmetic overflow or undefined behaviour for every pair of `f64`,
/// NaN and the infinities included.
#[kani::proof]
fn c_construction() {
    let (x, y): (f64, f64) = (kani::any(), kani::any());
    let a = LatLng::new(Angle::from_radians(x), Angle::from_radians(y));
    let b = LatLng::from_degrees(x, y);
    let _ = (
        LatLng::from_radians(x, y),
        LatLng::invalid(),
        LatLng::default(),
    );
    let _ = (a.is_valid(), b.is_valid(), a == b);
    kani::cover!(a.is_valid(), "a valid pair");
    kani::cover!(!b.is_valid(), "an invalid pair");
}

/// C-s2::LatLng::from_e5, from_e6, from_e7, from_unsigned_e6 and from_unsigned_e7: complete for
/// every pair of `i32` and of `u32`.
#[kani::proof]
fn c_e_forms() {
    let (a, b): (i32, i32) = (kani::any(), kani::any());
    let (c, d): (u32, u32) = (kani::any(), kani::any());
    let lls = [
        LatLng::from_e5(a, b),
        LatLng::from_e6(a, b),
        LatLng::from_e7(a, b),
        LatLng::from_unsigned_e6(c, d),
        LatLng::from_unsigned_e7(c, d),
    ];
    kani::cover!(lls[2].is_valid(), "a valid E7 pair");
    kani::cover!(!lls[0].is_valid(), "an invalid E5 pair");
}

/// C-s2::LatLng::latitude, longitude, from_point and `From<Point> for LatLng`: complete for every
/// triple of `f64`, NaN and the infinities included (the squares cannot make `∞ − ∞`).
///
/// Under contract: `atan2_contract`.
#[kani::proof]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
fn c_from_point() {
    let p = any_triple();
    let _ = (LatLng::latitude(p), LatLng::longitude(p));
    let a = LatLng::from_point(p);
    let b = LatLng::from(p);
    kani::cover!(a.is_valid() && b.is_valid(), "a valid result");
    kani::cover!(
        p.0.x.is_infinite() && a.is_valid(),
        "an infinite coordinate"
    );
}

/// C-s2::LatLng::to_point and `From<LatLng> for Point`: complete for every pair of `f64`. The
/// documented precondition (finite coordinates, upstream's debug check) is what makes the result
/// meaningful; outside it the coordinates are NaN, without a panic.
///
/// Under contract: `sin_cos_contract`.
#[kani::proof]
#[kani::stub(f64::sin_cos, crate::stubs::sin_cos_contract)]
fn c_to_point() {
    let ll = any_lat_lng_bits();
    let p = ll.to_point();
    let q = Point::from(ll);
    kani::cover!(p.0.x.is_finite() && q.0.z.is_finite(), "finite coordinates");
    kani::cover!(p.0.x.is_nan(), "NaN coordinates");
}

/// C-s2::LatLng::normalized: completes for every pair of `f64`.
#[kani::proof]
fn c_normalized() {
    let ll = any_lat_lng_bits();
    let n = ll.normalized();
    kani::cover!(
        ll.lat.radians().is_finite() && n.lat.radians() == 0.5,
        "a finite input"
    );
    kani::cover!(!ll.lng.radians().is_finite(), "a non-finite input");
}

/// C-s2::LatLng::get_distance: completes for every pair of valid `LatLng`s, its documented
/// domain ("both must be normalized").
///
/// Under contract: `sin_contract`, `cos_contract`, `asin_contract`.
#[kani::proof]
#[kani::stub(f64::sin, crate::stubs::sin_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn c_get_distance() {
    let (a, b) = (any_valid_lat_lng(), any_valid_lat_lng());
    let d = a.get_distance(b).radians();
    kani::cover!(d > 0.0, "a positive distance");
}

/// Outside `get_distance`'s domain: it completes, without a panic, for every pair of `LatLng`s
/// without an infinite coordinate (the result may be NaN, as upstream's: spec 010 K3, FR-018).
/// An infinite coordinate makes `∞ − ∞`, which Kani reports as a NaN check; the release build
/// computes the NaN without a panic. The root is `sqrt_contract`: outside the domain the
/// haversine term can be negative, and CBMC's built-in `sqrt` computes that NaN through a
/// division its NaN check reports (2026-10-28), where IEEE-754 defines it.
///
/// Under contract: `sin_contract`, `cos_contract`, `sqrt_contract`, `asin_contract`.
#[kani::proof]
#[kani::stub(f64::sqrt, crate::stubs::sqrt_contract)]
#[kani::stub(f64::sin, crate::stubs::sin_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
fn o_get_distance() {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    for x in [a.lat, a.lng, b.lat, b.lng] {
        kani::assume(!x.radians().is_infinite());
    }
    let d = a.get_distance(b).radians();
    kani::cover!(!a.is_valid() && d.is_nan(), "NaN outside the domain");
    kani::cover!(!a.is_valid() && !d.is_nan(), "a number outside the domain");
}

/// C-s2::LatLng::approx_eq and approx_eq_with: complete for every pair whose coordinate
/// differences are not `∞ − ∞`, and every tolerance.
#[kani::proof]
fn c_approx_eq() {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    kani::assume(!equal_infinities(a.lat.radians(), b.lat.radians()));
    kani::assume(!equal_infinities(a.lng.radians(), b.lng.radians()));
    let m: f64 = kani::any();
    let (x, y) = (a.approx_eq(b), a.approx_eq_with(b, Angle::from_radians(m)));
    kani::cover!(x, "approximately equal");
    kani::cover!(!y, "not approximately equal");
}

/// C-`Add`, `Sub`, `Mul<LatLng> for f64` and `Mul<f64> for LatLng`: complete for every pair whose
/// sum or difference is not `∞ − ∞`, and every factor whose product is not `0 · ∞`.
#[kani::proof]
fn c_arithmetic() {
    let (a, b) = (any_lat_lng_bits(), any_lat_lng_bits());
    let (alat, alng, blat, blng) = (
        a.lat.radians(),
        a.lng.radians(),
        b.lat.radians(),
        b.lng.radians(),
    );
    let k: f64 = kani::any();
    if kani::any() {
        kani::assume(!opposite_infinities(alat, blat) && !opposite_infinities(alng, blng));
        let _ = a + b;
    } else if kani::any() {
        kani::assume(!equal_infinities(alat, blat) && !equal_infinities(alng, blng));
        let _ = a - b;
    } else {
        let zero_inf = |x: f64| (x == 0.0 && k.is_infinite()) || (x.is_infinite() && k == 0.0);
        kani::assume(!zero_inf(alat) && !zero_inf(alng));
        let (p, q) = (k * a, a * k);
        kani::cover!(
            p.lat.radians().is_infinite() && q.lng.radians() == 0.0,
            "scaled"
        );
    }
}
