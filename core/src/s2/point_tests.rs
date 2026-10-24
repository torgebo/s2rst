// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for the fixes and the documented limits found during the formal
//! verification of `s2::Point` (specs/006-formal-verify-r2-rect-point). Written for this
//! crate, not ported from upstream S2.

use super::*;

/// `2^k` for `k` in `[−1074, −1023]`, a subnormal, from its bits (never `powi`, which is
/// inexact there).
fn subnormal_pow2(k: i32) -> f64 {
    assert!((-1074..=-1023).contains(&k));
    f64::from_bits(1u64 << (k + 1074))
}

/// `2^k` for `k` in `[−1022, 1023]`, from its bits.
fn pow2(k: i32) -> f64 {
    assert!((-1022..=1023).contains(&k));
    f64::from_bits(u64::try_from(1023 + k).unwrap() << 52)
}

fn bits(p: Point) -> [u64; 3] {
    [p.0.x, p.0.y, p.0.z].map(f64::to_bits)
}

fn point(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

/// `(x, 0, −x/2)` for `x = 2^k` scales to `(1, 0, −0.5)`.
fn check_half_pair(k: i32) {
    let x = subnormal_pow2(k);
    let p = point(x, 0.0, -x / 2.0);
    assert_eq!(
        bits(p.ensure_normalizable()),
        bits(point(1.0, 0.0, -0.5)),
        "2^{k}"
    );
}

/// FR-020 (K5): below `2⁻¹⁰²³` the old scaling divided by a zero or wrapped its exponent, and
/// returned NaN, infinities or zero. The fix scales the largest component into [1, 2) for
/// every non-zero finite vector.
#[test]
fn ensure_normalizable_scales_two_to_the_minus_1024() {
    check_half_pair(-1024);
}

/// FR-020 (K5), one binade lower.
#[test]
fn ensure_normalizable_scales_two_to_the_minus_1025() {
    check_half_pair(-1025);
}

/// FR-020 (K5): the smallest subnormal scales to exactly 1.
#[test]
fn ensure_normalizable_scales_the_smallest_subnormal() {
    let p = point(subnormal_pow2(-1074), 0.0, 0.0);
    assert_eq!(bits(p.ensure_normalizable()), bits(point(1.0, 0.0, 0.0)));
}

/// FR-020: a normalizable vector and the zero vector come back unchanged.
#[test]
fn ensure_normalizable_keeps_normalizable_and_zero_vectors() {
    for p in [
        point(1.0, 0.0, 0.0),
        point(pow2(-242), -0.0, 0.0),
        point(-3.0, 1e300, 0.5),
        point(0.0, 0.0, 0.0),
        point(-0.0, 0.0, -0.0),
    ] {
        assert_eq!(bits(p.ensure_normalizable()), bits(p), "{p:?}");
    }
}

/// FR-020: where upstream's `ldexp(2, −1 − ilogb(p_max)) * p` is finite (a largest component
/// from `2⁻²⁴³` down to `2⁻¹⁰²²`), the result is upstream's, bit for bit.
#[test]
fn ensure_normalizable_matches_upstream_where_upstream_is_finite() {
    for k in 243..=1022 {
        let m = 1.5 * pow2(-k);
        let p = point(-m / 3.0, m, m * 0.7);
        // ilogb(m) = −k, so upstream's factor is ldexp(2, k − 1) = 2^k, normal for k ≤ 1022.
        let factor = pow2(k);
        let upstream = point(factor * p.0.x, factor * p.0.y, factor * p.0.z);
        let r = p.ensure_normalizable();
        assert_eq!(bits(r), bits(upstream), "2^-{k}");
        assert!((1.0..2.0).contains(&r.0.y), "2^-{k}: {r:?}");
    }
}

/// K10 (FR-022): for equal points `point_cross` returns `ortho(p)`, upstream's `S2::Ortho`, as
/// C++'s `RobustCrossProd` does (Go returns the axis-aligned `(0, −1, 0)`).
#[test]
fn point_cross_of_equal_points_is_ortho() {
    let p = point(1.0, 0.0, 0.0);
    let r = p.point_cross(p);
    assert_eq!(bits(r), bits(ortho(p)));
    assert_eq!(r.x(), 0.0);
    assert!(
        (r.y() + 0.999_986).abs() < 1e-6 && (r.z() - 0.0053).abs() < 1e-6,
        "{r:?}"
    );
}

/// K11 (FR-022, U-013): distinct parallel unit points also take the `ortho` branch, so the
/// swap law fails there: `q.point_cross(p)` is `ortho(q)`, not `−p.point_cross(q)`. Go's
/// `PointCross` gives the same.
#[test]
fn point_cross_of_parallel_points_is_ortho_of_the_first() {
    let p = point(1.0, 0.0, 0.0);
    let q = point(1.0 + 2.0 * f64::EPSILON, 0.0, 0.0);
    assert!(p.is_unit() && q.is_unit() && p != q);
    let (f, g) = (p.point_cross(q), q.point_cross(p));
    assert_eq!(bits(f), bits(ortho(p)));
    assert_eq!(bits(g), bits(ortho(q)));
    assert_ne!(g, -f);
}

/// K12 (FR-022): where the squared length underflows to zero, `normalize` returns positive
/// zeros, as Go; C++ keeps each coordinate's sign.
#[test]
fn normalize_returns_positive_zeros_where_the_squared_length_is_zero() {
    let r = point(-1e-170, 1e-170, -0.0).normalize();
    assert_eq!(bits(r), bits(point(0.0, 0.0, 0.0)));
}
