// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Unit tests of [`crate::latlng_spec`] (spec 010 data-model §1): the description on
//! hand-computed values, independent of the library's `LatLng` operations, and the drift guard
//! of [`reduce_fmod_copy`].

use super::*;

#[test]
fn valid_at_the_bounds() {
    assert!(valid(FRAC_PI_2, PI) && valid(-FRAC_PI_2, -PI) && valid(-0.0, 0.0));
    assert!(!valid(FRAC_PI_2.next_up(), 0.0) && !valid(0.0, -PI.next_up()));
    assert!(!valid(f64::NAN, 0.0) && !valid(0.0, f64::NAN));
    assert!(!valid(f64::INFINITY, 0.0));
    assert!(!valid(INVALID.0, INVALID.1));
}

#[test]
fn clamp_lat_keeps_signed_zeros_and_clamps() {
    assert_eq!(clamp_lat(-0.0).to_bits(), (-0.0f64).to_bits());
    assert_eq!(clamp_lat(2.0), FRAC_PI_2);
    assert_eq!(clamp_lat(-1e300), -FRAC_PI_2);
    assert_eq!(clamp_lat(0.5), 0.5);
}

#[test]
fn reduce_fmod_copy_on_hand_values() {
    let y = 2.0 * PI;
    // Above π: one turn down; below −π: one turn up.
    assert_eq!(reduce_fmod_copy(4.0, 4.0), 4.0 - y);
    assert_eq!(reduce_fmod_copy(-4.0, -4.0), -4.0 + y);
    // The ties: an even quotient keeps r, an odd one flips it.
    assert_eq!(reduce_fmod_copy(PI, PI), PI);
    assert_eq!(reduce_fmod_copy(PI, 3.0 * PI), -PI);
    assert_eq!(reduce_fmod_copy(-PI, -3.0 * PI), PI);
    // Signed zeros pass through.
    assert_eq!(reduce_fmod_copy(-0.0, -0.0).to_bits(), (-0.0f64).to_bits());
}

#[test]
fn restatements_on_hand_values() {
    assert_eq!(upstream_latitude(0.0, 0.0, 1.0), FRAC_PI_2);
    assert_eq!(upstream_latitude(-0.0, 0.0, -0.0).to_bits(), 0);
    assert_eq!(upstream_longitude(-1.0, -0.0), PI);
    assert_eq!(upstream_longitude(0.0, 0.0), 0.0);
    assert_eq!(upstream_to_point(0.0, 0.0), (1.0, 0.0, 0.0));
    assert_eq!(upstream_distance(0.3, 1.0, 0.3, 1.0), 0.0);
    assert!(upstream_distance(f64::NAN, 0.0, 0.0, 0.0) == PI);
    assert!(upstream_approx_eq(0.0, 0.0, 1e-15, -1e-15, 1e-15));
    assert!(!upstream_approx_eq(0.0, 0.0, 2e-15, 0.0, 1e-15));
}

/// The body of `fn <header>` in `src`, one trimmed line per element, comments and blank lines
/// dropped.
fn body(src: &str, header: &str) -> Vec<String> {
    let start = src
        .find(header)
        .unwrap_or_else(|| panic!("{header} not found"));
    let rest = &src[start + header.len()..];
    let end = rest.find("\n}\n").expect("the function's closing brace");
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .map(String::from)
        .collect()
}

/// The drift guard of `reduce_fmod_copy` (P-LLNRM-03): its body is the library's
/// `reduce_fmod`, with `x % (2.0 * y)` (the library's `fmod(x, 4π)`) written `r4`.
#[test]
fn reduce_fmod_copy_is_the_librarys() {
    let lib = include_str!("../../core/src/s1/angle.rs");
    let here = include_str!("latlng_spec.rs");
    let lib_body: Vec<String> = body(lib, "fn reduce_fmod(r: f64, x: f64) -> f64 {")
        .into_iter()
        .map(|l| l.replace("x % (2.0 * y)", "r4"))
        .collect();
    assert_eq!(
        lib_body,
        body(here, "pub fn reduce_fmod_copy(r: f64, r4: f64) -> f64 {")
    );
}

/// The drift guard of the conversions' and the distance's restatements (P-LLFP-01, P-LLTP-01,
/// P-LLDST-01): the library's bodies are the restatements' text, with the library's access to
/// its fields and its return type written as the restatement's. Rust evaluates identical
/// expressions identically (no contraction: `TRUSTED_BASE.md`, "The build"), so the library
/// computes the restatements' values, which `tests/values_spec_vs_cpp.rs` checks against C++.
#[test]
fn conversions_are_the_restatements() {
    // The library's functions are methods: their bodies end at the indented closing brace.
    let method = |src: &str, header: &str| -> Vec<String> {
        let start = src
            .find(header)
            .unwrap_or_else(|| panic!("{header} not found"));
        let rest = &src[start + header.len()..];
        rest[..rest.find("\n    }\n").expect("the method's closing brace")]
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .map(String::from)
            .collect()
    };
    let lib = include_str!("../../core/src/s2/latlng.rs");
    let here = include_str!("latlng_spec.rs");
    let sub = |lines: Vec<String>, from: &[(&str, &str)]| -> Vec<String> {
        lines
            .into_iter()
            .map(|l| from.iter().fold(l, |l, (a, b)| l.replace(a, b)))
            .collect()
    };
    // `latitude` and `longitude`: the point's coordinates are `x`, `y`, `z`, and the result is
    // wrapped in `Angle::from_radians`.
    let coords = [("p.x()", "x"), ("p.y()", "y"), ("p.z()", "z")];
    for (lib_fn, spec_fn) in [
        (
            "pub fn latitude(p: Point) -> Angle {",
            "pub fn upstream_latitude(x: f64, y: f64, z: f64) -> f64 {",
        ),
        (
            "pub fn longitude(p: Point) -> Angle {",
            "pub fn upstream_longitude(x: f64, y: f64) -> f64 {",
        ),
    ] {
        let l = sub(method(lib, lib_fn), &coords);
        assert_eq!(l.len(), 1, "{lib_fn}");
        let inner = l[0]
            .strip_prefix("Angle::from_radians(")
            .and_then(|r| r.strip_suffix(')'))
            .expect("the result is wrapped in Angle::from_radians");
        assert_eq!(vec![inner.to_string()], body(here, spec_fn), "{lib_fn}");
    }
    // `from_point` and `From<Point>` are the pair.
    assert_eq!(
        method(lib, "pub fn from_point(p: Point) -> Self {"),
        [
            "LatLng {",
            "lat: Self::latitude(p),",
            "lng: Self::longitude(p),",
            "}"
        ]
    );
    let from = |header: &str| {
        let start = lib.find(header).expect(header);
        let rest = &lib[start..];
        rest[..rest.find("\n}\n").unwrap()]
            .lines()
            .map(str::trim)
            .map(String::from)
            .collect::<Vec<_>>()
    };
    assert!(from("impl From<Point> for LatLng {").contains(&"LatLng::from_point(p)".to_string()));
    assert!(from("impl From<LatLng> for Point {").contains(&"ll.to_point()".to_string()));
    // `to_point`: the coordinates are the restatement's tuple, in order.
    let l = sub(
        method(lib, "pub fn to_point(self) -> Point {"),
        &[("self.lat.", "lat."), ("self.lng.", "lng.")],
    );
    let r = body(
        here,
        "pub fn upstream_to_point(lat: f64, lng: f64) -> (f64, f64, f64) {",
    );
    assert_eq!(l[..2], r[..2]);
    let fields: Vec<&str> = l[2..]
        .iter()
        .filter_map(|x| {
            ["x: ", "y: ", "z: "]
                .iter()
                .find_map(|k| x.strip_prefix(k))
                .and_then(|e| e.strip_suffix(','))
        })
        .collect();
    assert_eq!(format!("({})", fields.join(", ")), r[2]);
    // `get_distance`: the three formula lines are the restatement's, and the clamp is written
    // `x.min(1.0)` where the restatement writes upstream's `std::min` (L-LL-02 proves them equal).
    let l = method(lib, "pub fn get_distance(self, other: LatLng) -> Angle {");
    let r = body(
        here,
        "pub fn upstream_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {",
    );
    assert_eq!(
        l[..4],
        [
            "let lat1 = self.lat.radians();",
            "let lat2 = other.lat.radians();",
            "let lng1 = self.lng.radians();",
            "let lng2 = other.lng.radians();"
        ]
    );
    assert_eq!(l[4..7], r[..3]);
    assert_eq!(
        l[7..],
        ["Angle::from_radians(2.0 * x.min(1.0).sqrt().asin())"]
    );
    assert_eq!(
        r[3..],
        [
            "let m = if x < 1.0 { x } else { 1.0 };",
            "2.0 * m.sqrt().asin()"
        ]
    );
}

/// The drift guard of P-LLARI-01's scalings: `f64 * LatLng` and `LatLng * f64` scale each
/// coordinate by `Angle * f64`, the same expression, so they agree bit for bit, and are
/// `Angle(radians · k)` (spec 007's P-AGARI-01).
#[test]
fn scalings_are_angle_products() {
    let lib = include_str!("../../core/src/s2/latlng.rs");
    let impl_body = |header: &str| -> Vec<String> {
        let start = lib.find(header).expect(header);
        let rest = &lib[start..];
        rest[..rest.find("\n}\n").unwrap()]
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("lat:") || l.starts_with("lng:"))
            .map(String::from)
            .collect()
    };
    assert_eq!(
        impl_body("impl Mul<LatLng> for f64 {"),
        ["lat: rhs.lat * self,", "lng: rhs.lng * self,"]
    );
    assert_eq!(
        impl_body("impl Mul<f64> for LatLng {"),
        ["lat: self.lat * rhs,", "lng: self.lng * rhs,"]
    );
    let angle = include_str!("../../core/src/s1/angle.rs");
    let start = angle.find("impl Mul<f64> for Angle {").unwrap();
    let rest = &angle[start..];
    assert!(rest[..rest.find("\n}\n").unwrap()].contains("Angle(self.0 * rhs)"));
}
