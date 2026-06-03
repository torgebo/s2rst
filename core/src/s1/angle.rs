// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A one-dimensional angle, stored internally as radians.
//!
//! Corresponds to C++ `S1Angle`, Go `s1.Angle`, Java `S1Angle`.
//!
//! Conversion to and from radians is exact. Conversions between E5, E6, E7,
//! and degrees are not always exact because degrees are converted to radians
//! first.
//!
//! The following conversions between degrees and radians are exact:
//!
//! ```text
//!     Angle::from_degrees(180) == Angle::from_radians(PI)
//!     Angle::from_degrees(180/n) == Angle::from_radians(PI/n)  for n = 1..8
//! ```
//!
//! These identities also hold when scaled by any power of 2.

#![expect(
    clippy::cast_possible_truncation,
    reason = "E5/E6/E7 angle encoding (f64->i32) — values clamped to valid range"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "E5/E6/E7 angle encoding — f64->i32 range checked by domain"
)]
use std::f64::consts::PI;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A one-dimensional angle (newtype over f64 radians).
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s1::Angle;
///
/// // Create from degrees and radians
/// let a = Angle::from_degrees(90.0);
/// assert!((a.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
///
/// // Arithmetic
/// let b = Angle::from_degrees(45.0);
/// let sum = a + b;
/// assert!((sum.degrees() - 135.0).abs() < 1e-13);
///
/// // E6/E7 conversions
/// assert_eq!(Angle::from_degrees(12.345678).e6(), 12345678);
/// assert_eq!(Angle::from_e7(900000000).degrees(), 90.0);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Angle(f64);

impl Angle {
    /// The zero angle.
    pub const ZERO: Angle = Angle(0.0);

    /// An angle larger than any finite angle.
    pub const INFINITY: Angle = Angle(f64::INFINITY);

    // --- Constructors ---

    /// Creates an angle from a value in radians (exact).
    #[inline]
    pub const fn from_radians(radians: f64) -> Self {
        Angle(radians)
    }

    /// Creates an angle from a value in degrees.
    #[inline]
    pub fn from_degrees(degrees: f64) -> Self {
        Angle(degrees * (PI / 180.0))
    }

    /// Creates an angle from E5 representation (degrees × 10⁵).
    #[inline]
    pub fn from_e5(e5: i32) -> Self {
        Self::from_degrees(f64::from(e5) * 1e-5)
    }

    /// Creates an angle from E6 representation (degrees × 10⁶).
    #[inline]
    pub fn from_e6(e6: i32) -> Self {
        Self::from_degrees(f64::from(e6) * 1e-6)
    }

    /// Creates an angle from E7 representation (degrees × 10⁷).
    #[inline]
    pub fn from_e7(e7: i32) -> Self {
        Self::from_degrees(f64::from(e7) * 1e-7)
    }

    /// Creates an angle from unsigned E6 representation.
    #[inline]
    pub fn from_unsigned_e6(e6: u32) -> Self {
        Self::from_e6(e6 as i32)
    }

    /// Creates an angle from unsigned E7 representation.
    #[inline]
    pub fn from_unsigned_e7(e7: u32) -> Self {
        Self::from_e7(e7 as i32)
    }

    // --- Accessors ---

    /// Returns the angle in radians (exact).
    #[inline]
    pub fn radians(self) -> f64 {
        self.0
    }

    /// Returns the angle in degrees.
    #[inline]
    pub fn degrees(self) -> f64 {
        self.0 * (180.0 / PI)
    }

    /// Returns the angle in E5 representation (degrees × 10⁵, rounded).
    #[inline]
    pub fn e5(self) -> i32 {
        (self.degrees() * 1e5).round() as i32
    }

    /// Returns the angle in E6 representation (degrees × 10⁶, rounded).
    #[inline]
    pub fn e6(self) -> i32 {
        (self.degrees() * 1e6).round() as i32
    }

    /// Returns the angle in E7 representation (degrees × 10⁷, rounded).
    #[inline]
    pub fn e7(self) -> i32 {
        (self.degrees() * 1e7).round() as i32
    }

    // --- Operations ---

    /// Returns the absolute value of this angle.
    #[inline]
    pub fn abs(self) -> Angle {
        Angle(self.0.abs())
    }

    /// Returns an equivalent angle in (-π, π].
    pub fn normalized(self) -> Angle {
        let rem = self.0 - (self.0 / (2.0 * PI)).round() * (2.0 * PI);
        if rem <= -PI { Angle(PI) } else { Angle(rem) }
    }

    /// Returns the sine of this angle.
    #[inline]
    pub fn sin(self) -> f64 {
        self.0.sin()
    }

    /// Returns the cosine of this angle.
    #[inline]
    pub fn cos(self) -> f64 {
        self.0.cos()
    }

    /// Returns the tangent of this angle.
    #[inline]
    pub fn tan(self) -> f64 {
        self.0.tan()
    }

    /// Returns (sin, cos) of this angle.
    #[inline]
    pub fn sin_cos(self) -> (f64, f64) {
        self.0.sin_cos()
    }

    /// Reports whether this angle is infinite.
    #[inline]
    pub fn is_infinite(self) -> bool {
        self.0.is_infinite()
    }

    /// Reports whether two angles are approximately equal (within 1e-15 radians).
    #[inline]
    pub fn approx_eq(self, other: Angle) -> bool {
        (self.0 - other.0).abs() <= 1e-15
    }
}

// --- Arithmetic operator impls ---

impl Neg for Angle {
    type Output = Angle;
    #[inline]
    fn neg(self) -> Angle {
        Angle(-self.0)
    }
}

impl Add for Angle {
    type Output = Angle;
    #[inline]
    fn add(self, rhs: Angle) -> Angle {
        Angle(self.0 + rhs.0)
    }
}

impl Sub for Angle {
    type Output = Angle;
    #[inline]
    fn sub(self, rhs: Angle) -> Angle {
        Angle(self.0 - rhs.0)
    }
}

impl Mul<f64> for Angle {
    type Output = Angle;
    #[inline]
    fn mul(self, rhs: f64) -> Angle {
        Angle(self.0 * rhs)
    }
}

impl Mul<Angle> for f64 {
    type Output = Angle;
    #[inline]
    fn mul(self, rhs: Angle) -> Angle {
        Angle(self * rhs.0)
    }
}

impl Div<f64> for Angle {
    type Output = Angle;
    #[inline]
    fn div(self, rhs: f64) -> Angle {
        Angle(self.0 / rhs)
    }
}

impl Div<Angle> for Angle {
    type Output = f64;
    #[inline]
    fn div(self, rhs: Angle) -> f64 {
        self.0 / rhs.0
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.7}", self.degrees())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn angle_is_send_sync() {
        is_send_sync::<Angle>();
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-15
    }

    #[test]
    fn test_default_constructor() {
        let a = Angle::default();
        assert_eq!(a.radians(), 0.0);
        assert_eq!(a, Angle::ZERO);
    }

    #[test]
    fn test_infinity() {
        assert!(Angle::from_radians(1e30) < Angle::INFINITY);
        assert!(-Angle::INFINITY < Angle::ZERO);
        assert_eq!(Angle::INFINITY, Angle::INFINITY);
    }

    #[test]
    fn test_zero() {
        assert_eq!(Angle::from_radians(0.0), Angle::ZERO);
    }

    #[test]
    fn test_pi_radians_exactly_180_degrees() {
        assert_eq!(Angle::from_radians(PI).radians(), PI);
        assert_eq!(Angle::from_radians(PI).degrees(), 180.0);
        assert_eq!(Angle::from_degrees(180.0).radians(), PI);
        assert_eq!(Angle::from_degrees(180.0).degrees(), 180.0);
        assert_eq!(Angle::from_radians(PI / 2.0).degrees(), 90.0);
        assert_eq!(Angle::from_radians(-PI / 2.0).degrees(), -90.0);
        assert_eq!(Angle::from_degrees(-45.0).radians(), -PI / 4.0);
    }

    #[test]
    fn test_e5_e6_e7_representations() {
        // E5 has a small rounding variance (Go also allows 1e-15 tolerance here).
        assert!(approx_eq(
            Angle::from_degrees(-45.0).radians(),
            Angle::from_e5(-4500000).radians()
        ));
        assert_eq!(
            Angle::from_degrees(-60.0).radians(),
            Angle::from_e6(-60000000).radians()
        );
        assert_eq!(
            Angle::from_degrees(75.0).radians(),
            Angle::from_e7(750000000).radians()
        );

        assert_eq!(Angle::from_degrees(-172.56123).e5(), -17256123);
        assert_eq!(Angle::from_degrees(12.345678).e6(), 12345678);
        assert_eq!(Angle::from_degrees(-12.3456789).e7(), -123456789);
    }

    #[test]
    fn test_e5_e6_e7_rounding() {
        // Values near 0.5 boundary.
        let rounding_cases: [(f64, i32); 4] = [
            (0.500000001, 1),
            (-0.500000001, -1),
            (0.499999999, 0),
            (-0.499999999, 0),
        ];
        for (have, want) in &rounding_cases {
            assert_eq!(
                Angle::from_degrees(have * 1e-5).e5(),
                *want,
                "Angle::from_degrees({have} * 1e-5).e5()",
            );
            assert_eq!(
                Angle::from_degrees(have * 1e-6).e6(),
                *want,
                "Angle::from_degrees({have} * 1e-6).e6()",
            );
            assert_eq!(
                Angle::from_degrees(have * 1e-7).e7(),
                *want,
                "Angle::from_degrees({have} * 1e-7).e7()",
            );
        }
    }

    #[test]
    fn test_unsigned_e6_e7() {
        assert_eq!(
            Angle::from_degrees(60.0).radians(),
            Angle::from_unsigned_e6(60000000u32).radians()
        );
        // Intentional two's complement for negative angle encoding
        #[expect(
            clippy::cast_sign_loss,
            reason = "intentional two's complement reinterpretation"
        )]
        let neg_e6: u32 = (-60000000i32) as u32;
        assert_eq!(
            Angle::from_degrees(-60.0).radians(),
            Angle::from_unsigned_e6(neg_e6).radians()
        );
        assert_eq!(
            Angle::from_degrees(75.0).radians(),
            Angle::from_unsigned_e7(750000000u32).radians()
        );
        // Intentional two's complement for negative angle encoding
        #[expect(
            clippy::cast_sign_loss,
            reason = "intentional two's complement reinterpretation"
        )]
        let neg_e7: u32 = (-750000000i32) as u32;
        assert_eq!(
            Angle::from_degrees(-75.0).radians(),
            Angle::from_unsigned_e7(neg_e7).radians()
        );
    }

    #[test]
    fn test_normalized() {
        let cases: [(f64, f64); 6] = [
            (360.0, 0.0),
            (-90.0, -90.0),
            (-180.0, 180.0),
            (180.0, 180.0),
            (540.0, 180.0),
            (-270.0, 90.0),
        ];
        for (deg_in, deg_want) in &cases {
            let got = Angle::from_degrees(*deg_in).normalized().degrees();
            assert_eq!(
                got, *deg_want,
                "Angle::from_degrees({deg_in}).normalized().degrees() = {got}, want {deg_want}"
            );
        }
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(Angle::from_radians(-0.3).abs().radians(), 0.3);
        assert_eq!((-Angle::from_radians(0.1)).radians(), -0.1);
        assert!(approx_eq(
            (Angle::from_radians(0.1) + Angle::from_radians(0.3)).radians(),
            0.4
        ));
        assert!(approx_eq(
            (Angle::from_radians(0.1) - Angle::from_radians(0.3)).radians(),
            -0.2
        ));
        assert!(approx_eq((2.0 * Angle::from_radians(0.3)).radians(), 0.6));
        assert!(approx_eq((Angle::from_radians(0.3) * 2.0).radians(), 0.6));
        assert_eq!((Angle::from_radians(0.3) / 2.0).radians(), 0.15);
        assert_eq!(Angle::from_radians(0.3) / Angle::from_radians(0.6), 0.5);
    }

    #[test]
    fn test_trigonometry() {
        assert!(approx_eq(Angle::from_degrees(0.0).cos(), 1.0));
        assert!(approx_eq(Angle::from_degrees(90.0).sin(), 1.0));
        assert!(approx_eq(Angle::from_degrees(45.0).tan(), 1.0));

        // Verify sin_cos matches individual sin/cos.
        // Use approximate comparisons for Miri compatibility (soft-float rounding
        // can produce 1-2 ULP differences between sin_cos and separate sin/cos).
        for k in -1000..=1000 {
            let a = Angle::from_radians(f64::from(k));
            let (s, c) = a.sin_cos();
            assert!(
                (s - a.sin()).abs() < 1e-14,
                "sin_cos sin mismatch at k={k}: {s} vs {}",
                a.sin()
            );
            assert!(
                (c - a.cos()).abs() < 1e-14,
                "sin_cos cos mismatch at k={k}: {c} vs {}",
                a.cos()
            );
        }
    }

    #[test]
    fn test_formatting() {
        assert_eq!(format!("{}", Angle::from_degrees(180.0)), "180.0000000");
    }

    #[test]
    fn test_degrees_vs_e6() {
        for i in 0..=180 {
            assert_eq!(
                Angle::from_degrees(f64::from(i)),
                Angle::from_e6(1_000_000 * i),
                "Degrees({i}) != E6({})",
                1_000_000 * i,
            );
        }
    }

    #[test]
    fn test_degrees_vs_e7() {
        for i in 0..=180 {
            assert_eq!(
                Angle::from_degrees(f64::from(i)),
                Angle::from_e7(10_000_000 * i),
                "Degrees({i}) != E7({})",
                10_000_000 * i,
            );
        }
    }

    #[test]
    fn test_e6_vs_e7() {
        // Use a deterministic set of values (C++ uses random, we use a spread).
        for i in (0..180_000_000).step_by(179_999) {
            assert_eq!(
                Angle::from_e6(i),
                Angle::from_e7(10 * i),
                "E6({i}) != E7({})",
                10 * i,
            );
        }
    }

    #[test]
    fn test_degrees_vs_radians() {
        // 45°×k identities.
        for k in -8i32..=8 {
            assert_eq!(
                Angle::from_degrees(45.0 * f64::from(k)),
                Angle::from_radians(f64::from(k) * PI / 4.0),
                "Degrees(45*{k}) != Radians({k}*π/4)",
            );
            assert_eq!(
                Angle::from_degrees(45.0 * f64::from(k)).degrees(),
                45.0 * f64::from(k),
                "Degrees(45*{k}).degrees() != 45*{k}",
            );
        }

        // Power-of-2 subdivision identities.
        for k in 0u32..30 {
            let n = (1u64 << k) as f64;
            let cases: [(f64, f64); 5] = [
                (180.0, 1.0),
                (60.0, 3.0),
                (36.0, 5.0),
                (20.0, 9.0),
                (4.0, 45.0),
            ];
            for (deg, rad_denom) in &cases {
                assert_eq!(
                    Angle::from_degrees(deg / n),
                    Angle::from_radians(PI / (rad_denom * n)),
                    "Degrees({deg}/{n}) != Radians(π/({rad_denom}*{n}))",
                );
            }
        }

        // Verify non-identity: 60° converted through radians is not exactly 60.
        assert_ne!(Angle::from_degrees(60.0).degrees(), 60.0);
    }

    #[test]
    fn test_approx_eq() {
        let a = Angle::from_degrees(60.0);
        assert!(a.approx_eq(Angle::from_radians(PI / 3.0)));
        assert!(!Angle::from_radians(1.0).approx_eq(Angle::from_radians(2.0)));
    }

    #[test]
    fn test_constructors_that_measure_angles() {
        // From C++: ConstructorsThatMeasureAngles - angle between two S2Points.
        use crate::r3::Vector;
        use crate::s2::Point;

        // Angle between (1,0,0) and (0,0,2) should be π/2.
        // Note: (0,0,2) is not unit length; Point::distance normalizes internally
        // via Vector::angle which uses atan2.
        let p1 = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let p2 = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        });
        assert!(approx_eq(p1.distance(p2).radians(), PI / 2.0));

        // Angle between identical points should be 0.
        assert_eq!(p1.distance(p1).radians(), 0.0);

        // Angle between opposite points should be π.
        let p3 = Point(Vector {
            x: -1.0,
            y: 0.0,
            z: 0.0,
        });
        assert!(approx_eq(p1.distance(p3).radians(), PI));
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e15, 1e15)
        } else {
            0.0
        }
    }

    #[quickcheck]
    fn prop_from_radians_roundtrip(r: f64) -> bool {
        let r = clamp_finite(r);
        Angle::from_radians(r).radians() == r
    }

    #[quickcheck]
    fn prop_double_neg(r: f64) -> bool {
        let r = clamp_finite(r);
        let a = Angle::from_radians(r);
        (-(-a)).radians() == a.radians()
    }

    #[quickcheck]
    fn prop_add_sub_inverse(a: f64, b: f64) -> bool {
        let a = Angle::from_radians(clamp_finite(a).clamp(-1e6, 1e6));
        let b = Angle::from_radians(clamp_finite(b).clamp(-1e6, 1e6));
        // (a + b) - b ≈ a (within floating-point tolerance)
        let result = ((a + b) - b).radians();
        let expected = a.radians();
        (result - expected).abs() < 1e-6 * expected.abs().max(1.0)
    }

    #[quickcheck]
    fn prop_degrees_approx_roundtrip(d: f64) -> bool {
        let d = clamp_finite(d).clamp(-1e9, 1e9);
        let got = Angle::from_degrees(d).degrees();
        (got - d).abs() < 1e-9 * d.abs().max(1.0)
    }

    #[quickcheck]
    fn prop_e6_roundtrip_for_integers(n: i16) -> bool {
        let n = i32::from(n);
        Angle::from_e6(n).e6() == n
    }

    #[quickcheck]
    fn prop_abs_non_negative(r: f64) -> bool {
        let r = clamp_finite(r);
        Angle::from_radians(r).abs().radians() >= 0.0
    }

    #[quickcheck]
    fn prop_normalized_in_range(r: f64) -> bool {
        let r = clamp_finite(r).clamp(-1e8, 1e8);
        let n = Angle::from_radians(r).normalized().radians();
        n > -PI && n <= PI
    }

    #[quickcheck]
    fn prop_sin_cos_identity(r: f64) -> bool {
        let r = clamp_finite(r);
        let a = Angle::from_radians(r);
        let s = a.sin();
        let c = a.cos();
        (s * s + c * c - 1.0).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_add_commutative(a: f64, b: f64) -> bool {
        let a = Angle::from_radians(clamp_finite(a).clamp(-1e6, 1e6));
        let b = Angle::from_radians(clamp_finite(b).clamp(-1e6, 1e6));
        (a + b).radians() == (b + a).radians()
    }

    #[quickcheck]
    fn prop_e5_roundtrip_for_integers(n: i16) -> bool {
        let n = i32::from(n);
        Angle::from_e5(n).e5() == n
    }

    #[quickcheck]
    fn prop_e7_roundtrip_for_integers(n: i16) -> bool {
        let n = i32::from(n);
        Angle::from_e7(n).e7() == n
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(r: i32) -> bool {
        let a = Angle::from_radians(f64::from(r) / 1000.0);
        let json = serde_json::to_string(&a).unwrap();
        let back: Angle = serde_json::from_str(&json).unwrap();
        back == a
    }
}
