// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Unit tests of [`crate::cap_spec`] (spec 008 data-model §1): the description on hand-computed
//! values, independent of the library's `Cap` operations.

use super::*;
use s2rst::s1::ChordAngle;

fn pt(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

fn cap(c: Point, l: f64) -> Cap {
    Cap::from_center_chord_angle(c, ChordAngle::from_length2(l))
}

#[test]
fn member_at_the_boundary_and_beyond() {
    let x = pt(1.0, 0.0, 0.0);
    let y = pt(0.0, 1.0, 0.0);
    // The squared chord from x to y is exactly 2.
    assert_eq!(chord2(x, y), 2.0);
    assert!(member(cap(x, 2.0), y));
    assert!(!member(cap(x, 1.999_999_999_999_999_8), y));
    assert!(!interior_member(cap(x, 2.0), y));
    assert!(interior_member(cap(x, 2.000_000_000_000_000_4), y));
}

#[test]
fn empty_and_full() {
    let x = pt(1.0, 0.0, 0.0);
    let minus_x = pt(-1.0, 0.0, 0.0);
    let e = cap(x, -1.0);
    let f = cap(x, 4.0);
    assert!(empty(e) && !full(e));
    assert!(full(f) && !empty(f));
    assert!(!member(e, x));
    assert!(member(f, minus_x));
    assert!(interior_member(f, minus_x));
    assert!(valid(e) && valid(f));
    assert!(!valid(cap(pt(2.0, 0.0, 0.0), 1.0)));
}

#[test]
fn a_point_cap_contains_its_point_only() {
    let x = pt(1.0, 0.0, 0.0);
    let c = cap(x, 0.0);
    assert!(member(c, x));
    assert!(!member(c, pt(0.0, 1.0, 0.0)));
    assert!(!interior_member(c, x));
}

#[test]
fn the_squared_chord_is_clamped_at_four() {
    // Antipodal points slightly off the sphere: (1 + 1e-15)² · 4 > 4 before the clamp.
    let a = pt(1.0 + 1e-15, 0.0, 0.0);
    let b = pt(-1.0 - 1e-15, 0.0, 0.0);
    assert_eq!(chord2(a, b), 4.0);
}

#[test]
fn restatements_on_hand_computed_values() {
    assert_eq!(upstream_height(1.0), 0.5);
    assert_eq!(upstream_area(2.0), 2.0 * PI);
    assert_eq!(upstream_area(-1.0), 0.0);
    assert_eq!(upstream_from_height(1.0), 2.0);
    assert_eq!(upstream_from_height(3.0), 4.0);
    // Upstream keeps a negative length (`std::min(4.0, x)`); the library maps it to −1.
    assert_eq!(upstream_from_height(-0.5), -1.0);
    assert_eq!(upstream_from_height(-0.25), -0.5);
    assert!(upstream_from_height(f64::NAN) == 4.0);
    assert_eq!(upstream_from_area(4.0 * PI), 4.0);
    // K4: upstream's complement of length² 1 is exactly 3 (the library gave
    // 2.99999999999999956 before FR-019; probe/cpp/survey_cpp.txt).
    assert_eq!(upstream_complement_length2(1.0), 3.0);
    assert_eq!(upstream_complement_length2(1.7), 2.3);
    assert_eq!(upstream_add_cap_margin(1.0), 4.02 * f64::EPSILON);
    let c = upstream_centroid(pt(0.0, 0.0, 1.0), 2.0);
    // The hemisphere: r = 1 − 0.5 · 1 = 0.5, area 2π, so the centroid is (0, 0, π).
    assert_eq!((c.0.x, c.0.y, c.0.z), (0.0, 0.0, PI));
    let e = upstream_centroid(pt(0.0, 0.0, 1.0), -1.0);
    assert_eq!((e.0.x, e.0.y, e.0.z), (0.0, 0.0, 0.0));
}

#[test]
fn the_covering_level_is_upstreams() {
    // K1's radius (probe/cpp/survey_cpp.txt: upstream's level 1).
    assert_eq!(upstream_level(0.117_851_130_197_757_95), 1);
    // A radius exactly a min width: deriv / r is a power of two.
    let d = 2.0 * SQRT_2 / 3.0;
    assert_eq!(upstream_level(d / 8.0), 2);
    // No positive radius: level 30, so 29.
    assert_eq!(upstream_level(0.0), 29);
    assert_eq!(upstream_level(-1.0), 29);
    // Larger than the level-0 width: the six faces.
    assert_eq!(upstream_level(3.0), -1);
    // Tiny radii clamp at 30.
    assert_eq!(upstream_level(1e-300), 29);
    assert_eq!(upstream_level(f64::from_bits(1)), 29);
}

#[test]
fn ilogb_is_exact_for_subnormals() {
    assert_eq!(ilogb(1.0), 0);
    assert_eq!(ilogb(1.999_999_999_999_999_8), 0);
    assert_eq!(ilogb(2.0), 1);
    assert_eq!(ilogb(f64::from_bits(1)), -1074);
    assert_eq!(ilogb(f64::MIN_POSITIVE), -1022);
    assert_eq!(ilogb(f64::MIN_POSITIVE / 2.0), -1023);
    assert_eq!(ilogb(f64::INFINITY), i32::MAX);
}

/// The body of `fn <name>(x: f64) -> f64 { … }` in `src`, its lines trimmed, blank and comment
/// lines dropped.
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

/// The drift guard of `reduce_longitude_copy` (P-CPBND-05): its body is the library's.
#[test]
fn reduce_longitude_copy_is_the_librarys() {
    let lib = include_str!("../../core/src/s2/cap.rs");
    let here = include_str!("cap_spec.rs");
    assert_eq!(
        body(lib, "fn reduce_longitude(x: f64) -> f64 {"),
        body(here, "pub fn reduce_longitude_copy(x: f64) -> f64 {"),
        "core/src/s2/cap.rs's reduce_longitude changed: review P-CPBND-05 and update the copy"
    );
}
