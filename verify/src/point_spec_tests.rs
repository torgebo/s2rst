// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the point description (spec 006 T009), on the corners the survey
//! and the planning probe found.

use super::*;

const NAN: f64 = f64::NAN;

fn bits(p: Point) -> [u64; 3] {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

/// The order's NaN cycle (K8): `(1, 0, 0) < (NaN, 5, 0) < (0, 9, 0) < (1, 0, 0)`.
#[test]
fn nan_makes_the_order_a_cycle() {
    let (a, b, c) = (
        point(1.0, 0.0, 0.0),
        point(NAN, 5.0, 0.0),
        point(0.0, 9.0, 0.0),
    );
    assert!(lex_less(a, b));
    assert!(lex_less(b, c));
    assert!(lex_less(c, a));
    assert_eq!(order(a, b), Ordering::Less);
    assert_eq!(order(c, a), Ordering::Less);
    // A point with a NaN coordinate is neither below nor above itself.
    assert_eq!(order(b, b), Ordering::Equal);
    assert!(!same(b, b));
}

#[test]
fn the_first_differing_coordinate_decides() {
    assert_eq!(
        order(point(0.0, 9.0, 9.0), point(1.0, 0.0, 0.0)),
        Ordering::Less
    );
    assert_eq!(
        order(point(1.0, 2.0, 9.0), point(1.0, 3.0, 0.0)),
        Ordering::Less
    );
    assert_eq!(
        order(point(1.0, 2.0, 4.0), point(1.0, 2.0, 3.0)),
        Ordering::Greater
    );
    assert_eq!(
        order(point(1.0, 2.0, 3.0), point(1.0, 2.0, 3.0)),
        Ordering::Equal
    );
}

#[test]
fn signed_zeros_tie() {
    let (a, b) = (point(0.0, -0.0, 0.0), point(-0.0, 0.0, -0.0));
    assert_eq!(order(a, b), Ordering::Equal);
    assert!(same(a, b));
    assert_ne!(bits(a), bits(b));
}

#[test]
fn same_is_ieee_equality_per_coordinate() {
    assert!(same(point(0.0, 0.0, 0.0), point(-0.0, -0.0, -0.0)));
    assert!(!same(point(NAN, 0.0, 0.0), point(NAN, 0.0, 0.0)));
    assert!(!same(
        point(1.0, 0.0, 0.0),
        point(1.0, 0.0, f64::MIN_POSITIVE)
    ));
}

#[test]
fn normalizable_starts_at_two_to_the_minus_242() {
    assert_eq!(TWO_M242, 2f64.powi(-242));
    assert!(normalizable(point(0.0, -TWO_M242, 0.0)));
    assert!(!normalizable(point(0.0, TWO_M242.next_down(), 0.0)));
    assert!(normalizable(point(1.0, 0.0, 0.0)));
    assert!(!normalizable(point(0.0, 0.0, 0.0)));
}

#[test]
fn normable_stops_where_the_squared_length_overflows() {
    // sqrt(f64::MAX) is about 1.3408e154.
    assert!(normable(point(0.0, 0.0, 1.3e154)));
    assert!(!normable(point(0.0, 0.0, 1.4e154)));
    // Three coordinates that are each fine but whose sum overflows.
    assert!(!normable(point(1.0e154, 1.0e154, 1.0e154)));
    assert!(!normable(point(f64::INFINITY, 0.0, 0.0)));
    assert!(!normable(point(NAN, 0.0, 0.0)));
    assert!(normable(point(f64::MIN_POSITIVE, 0.0, 0.0)));
}

#[test]
fn in_box_and_zero() {
    assert!(in_box(point(-1.0, 1.0, 0.0)));
    assert!(!in_box(point(1.0f64.next_up(), 0.0, 0.0)));
    assert!(!in_box(point(NAN, 0.0, 0.0)));
    assert!(zero(point(-0.0, 0.0, -0.0)));
    assert!(!zero(point(0.0, 0.0, 5e-324)));
}

#[test]
fn exponent_reads_normal_and_subnormal_values() {
    assert_eq!(exponent(1.0), 0);
    assert_eq!(exponent(1.999), 0);
    assert_eq!(exponent(-2.0), 1);
    assert_eq!(exponent(0.75), -1);
    assert_eq!(exponent(f64::MIN_POSITIVE), -1022);
    assert_eq!(exponent(f64::MIN_POSITIVE.next_down()), -1023);
    assert_eq!(exponent(f64::from_bits(1)), -1074);
    assert_eq!(exponent(f64::from_bits(3)), -1073);
    assert_eq!(exponent(f64::MAX), 1023);
}

#[test]
fn pow2_is_exact_across_the_range() {
    assert_eq!(pow2(0), 1.0);
    assert_eq!(pow2(1023), 2f64.powi(1023));
    assert_eq!(pow2(1024), f64::INFINITY);
    assert_eq!(pow2(-1022), f64::MIN_POSITIVE);
    assert_eq!(pow2(-1074), f64::from_bits(1));
    assert_eq!(pow2(-1075), 0.0);
}

#[test]
fn scaled_by_is_exact_for_the_smallest_subnormal() {
    // `2⁻¹⁰⁷⁴ · 2¹⁰⁷⁴` is 1, though `2¹⁰⁷⁴` itself is not an `f64`.
    let p = point(f64::from_bits(1), 0.0, -0.0);
    assert_eq!(bits(scaled_by(p, 1074)), bits(point(1.0, 0.0, -0.0)));
    // A normal vector: every coordinate scaled exactly.
    let q = point(0.75, -0.5, 3.0);
    assert_eq!(bits(scaled_by(q, 10)), bits(point(768.0, -512.0, 3072.0)));
    assert_eq!(bits(scaled_by(q, -3)), bits(point(0.09375, -0.0625, 0.375)));
}

#[test]
fn upstream_scaled_overflows_below_two_to_the_minus_1023() {
    // RobustCrossProd(−b, b) for b = (2⁻¹⁰⁷⁴, 0, 1) scales a cross product whose largest
    // coordinate is 2⁻¹⁰⁷⁴: the factor overflows, so C++ returns (NaN, ∞, NaN) (K6, U-012).
    let r = upstream_scaled(point(0.0, f64::from_bits(1), 0.0));
    assert!(r.0.x.is_nan() && r.0.y == f64::INFINITY && r.0.z.is_nan());
    // At 2⁻¹⁰²³ the factor is 2¹⁰²³, still finite.
    let r = upstream_scaled(point(pow2(-1023), 0.0, 0.0));
    assert_eq!(bits(r), bits(point(1.0, 0.0, 0.0)));
    // At 2⁻³⁰⁰, the largest coordinate lands in [1, 2).
    let r = upstream_scaled(point(pow2(-300), 0.0, -pow2(-301)));
    assert_eq!(bits(r), bits(point(1.0, 0.0, -0.5)));
}

#[test]
fn upstream_is_unit_has_a_tolerance_of_five_epsilon() {
    assert!(upstream_is_unit(point(1.0, 0.0, 0.0)));
    assert!(upstream_is_unit(point(1.0 + 2.0 * f64::EPSILON, 0.0, 0.0)));
    assert!(!upstream_is_unit(point(1.0 + 4.0 * f64::EPSILON, 0.0, 0.0)));
    assert!(!upstream_is_unit(point(0.0, 0.0, 0.0)));
    let o = point(ORIGIN[0], ORIGIN[1], ORIGIN[2]);
    assert!(upstream_is_unit(o));
}

#[test]
fn upstream_normalize_multiplies_by_the_reciprocal_norm() {
    let r = upstream_normalize(point(3.0, 0.0, -4.0));
    assert_eq!(bits(r), bits(point(3.0 * 0.2, 0.0, -4.0 * 0.2)));
    // A zero norm multiplies by zero: each zero keeps its coordinate's sign (K12).
    let r = upstream_normalize(point(-0.0, 0.0, -1e-170));
    assert_eq!(bits(r), bits(point(-0.0, 0.0, -0.0)));
    // An overflowing squared length gives 1 / ∞ = 0.
    let r = upstream_normalize(point(1e200, -1.0, 1.0));
    assert_eq!(bits(r), bits(point(0.0, -0.0, 0.0)));
}

#[test]
fn origin_is_upstreams_literal() {
    assert_eq!(
        ORIGIN[0].to_bits(),
        (-0.009_999_466_435_025_02_f64).to_bits()
    );
    assert_eq!(ORIGIN[2], 0.999_946_643_502_502);
}

#[test]
fn go_cross_kernel_is_twice_the_cross_product() {
    // (1, 0, 0) × (0, 1, 0) = (0, 0, 1), so the kernel is (0, 0, 2) as values; the y
    // coordinate is 0 · (−1) − 1 · 0 = −0.
    let r = go_cross_kernel(point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0));
    assert_eq!(bits(r), bits(point(0.0, -0.0, 2.0)));
    // Parallel points: exactly zero, the case `PointCross` replaces by `Ortho` (K11).
    let p = point(1.0, 0.0, 0.0);
    let q = point(1.0 + 2.0 * f64::EPSILON, 0.0, 0.0);
    assert!(zero(go_cross_kernel(p, q)));
    assert!(zero(go_cross_kernel(p, p)));
    assert!(zero(go_cross_kernel(p, point(-1.0, -0.0, -0.0))));
}

#[test]
fn upstream_ortho_sets_the_axis_before_the_largest() {
    // Largest x: the 1 goes to z, so (1, 0, 0) × (0.012, 0.0053, 1) = (0, −1, 0.0053).
    let r = upstream_ortho(point(1.0, 0.0, 0.0));
    let n = (1.0f64 + 0.0053 * 0.0053).sqrt();
    assert!((r.0.y + 1.0 / n).abs() < 1e-15 && (r.0.z - 0.0053 / n).abs() < 1e-15);
    assert_eq!(r.0.x, 0.0);
    // A tie between x and y goes to y, so the 1 goes to x.
    let t = upstream_ortho(point(1.0, 1.0, 0.0));
    assert!(t.0.z.abs() > 0.5, "{t:?}");
}
