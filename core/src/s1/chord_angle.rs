// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! An angle represented as the squared chord length on the unit sphere.
//!
//! Corresponds to C++ `S1ChordAngle`, Go `s1.ChordAngle`, Java `S1ChordAngle`.
//!
//! `ChordAngle` stores the squared chord length `2 * sin²(angle/2)`, which
//! ranges from 0 to 4. This representation is very efficient for computing and
//! comparing distances, but unlike [`Angle`] it can only represent angles
//! between 0 and π radians. Generally, `ChordAngle` should only be used in
//! loops where many angles need to be calculated and compared; otherwise it is
//! simpler to use `Angle`.

use super::Angle;
use std::fmt;
use std::ops::{Add, Sub};

/// Maximum valid squared chord length (corresponds to π radians / 180°).
const MAX_LENGTH2: f64 = 4.0;

/// An angle represented as a squared chord length on the unit sphere.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s1::{Angle, ChordAngle};
/// use s2rst::s2::Point;
///
/// // Create from an Angle
/// let ca = ChordAngle::from_angle(Angle::from_degrees(90.0));
/// assert!((ca.degrees() - 90.0).abs() < 1e-13);
///
/// // Create from two points on the unit sphere
/// let p = Point::from_coords(1.0, 0.0, 0.0);
/// let q = Point::from_coords(0.0, 1.0, 0.0);
/// let ca2 = p.chord_angle(q);
/// assert!((ca2.length2() - 2.0).abs() < 1e-15); // 90 degrees = length2 of 2
///
/// // Comparisons are efficient (no trig needed)
/// assert!(ChordAngle::RIGHT < ChordAngle::STRAIGHT);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChordAngle(f64);

impl ChordAngle {
    /// The zero angle.
    pub const ZERO: ChordAngle = ChordAngle(0.0);

    /// A right angle (90 degrees). Squared chord length = 2.
    pub const RIGHT: ChordAngle = ChordAngle(2.0);

    /// A straight angle (180 degrees). The maximum finite chord angle.
    /// Squared chord length = 4.
    pub const STRAIGHT: ChordAngle = ChordAngle(MAX_LENGTH2);

    /// An angle larger than any finite chord angle. Only valid operations are
    /// comparisons, `Angle` conversions, and `successor`/`predecessor`.
    pub const INFINITY: ChordAngle = ChordAngle(f64::INFINITY);

    /// A chord angle smaller than zero. Only valid operations are comparisons,
    /// `Angle` conversions, and `successor`/`predecessor`.
    pub const NEGATIVE: ChordAngle = ChordAngle(-1.0);

    // --- Constructors ---

    /// Creates a `ChordAngle` from a squared chord length.
    /// The value is clamped to at most `MAX_LENGTH2` (4.0).
    #[inline]
    pub fn from_length2(length2: f64) -> Self {
        if length2 > MAX_LENGTH2 {
            ChordAngle::STRAIGHT
        } else {
            ChordAngle(length2)
        }
    }

    /// Creates a `ChordAngle` from an [`Angle`].
    #[inline]
    pub fn from_angle(angle: Angle) -> Self {
        if angle.radians() < 0.0 {
            ChordAngle::NEGATIVE
        } else if angle.is_infinite() {
            ChordAngle::INFINITY
        } else {
            let l = 2.0 * (0.5 * angle.radians().min(std::f64::consts::PI)).sin();
            ChordAngle(l * l)
        }
    }

    /// Creates a `ChordAngle` from radians. Convenience for
    /// `ChordAngle::from_angle(Angle::from_radians(r))`.
    #[inline]
    pub fn from_radians(radians: f64) -> Self {
        Self::from_angle(Angle::from_radians(radians))
    }

    /// Creates a `ChordAngle` from degrees. Convenience for
    /// `ChordAngle::from_angle(Angle::from_degrees(d))`.
    #[inline]
    pub fn from_degrees(degrees: f64) -> Self {
        Self::from_angle(Angle::from_degrees(degrees))
    }

    /// Creates a `ChordAngle` from E5 representation (degrees × 10⁵).
    #[inline]
    pub fn from_e5(e5: i32) -> Self {
        Self::from_angle(Angle::from_e5(e5))
    }

    /// Creates a `ChordAngle` from E6 representation (degrees × 10⁶).
    #[inline]
    pub fn from_e6(e6: i32) -> Self {
        Self::from_angle(Angle::from_e6(e6))
    }

    /// Creates a `ChordAngle` from E7 representation (degrees × 10⁷).
    #[inline]
    pub fn from_e7(e7: i32) -> Self {
        Self::from_angle(Angle::from_e7(e7))
    }

    // --- Accessors ---

    /// Returns the squared chord length.
    #[inline]
    pub fn length2(self) -> f64 {
        self.0
    }

    /// Converts this `ChordAngle` to an [`Angle`].
    pub fn to_angle(self) -> Angle {
        if self.is_negative() {
            Angle::from_radians(-1.0)
        } else if self.is_infinity() {
            Angle::INFINITY
        } else {
            Angle::from_radians(2.0 * (0.5 * self.0.sqrt()).asin())
        }
    }

    /// Returns this angle in radians (via conversion to `Angle`).
    #[inline]
    pub fn radians(self) -> f64 {
        self.to_angle().radians()
    }

    /// Returns this angle in degrees (via conversion to `Angle`).
    #[inline]
    pub fn degrees(self) -> f64 {
        self.to_angle().degrees()
    }

    /// Returns this angle in E5 representation (via conversion to `Angle`).
    #[inline]
    pub fn e5(self) -> i32 {
        self.to_angle().e5()
    }

    /// Returns this angle in E6 representation (via conversion to `Angle`).
    #[inline]
    pub fn e6(self) -> i32 {
        self.to_angle().e6()
    }

    /// Returns this angle in E7 representation (via conversion to `Angle`).
    #[inline]
    pub fn e7(self) -> i32 {
        self.to_angle().e7()
    }

    // --- Predicates ---

    /// Reports whether this is the zero angle.
    #[inline]
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Reports whether this is the special negative value.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 < 0.0
    }

    /// Reports whether this is the special infinity value.
    #[inline]
    pub fn is_infinity(self) -> bool {
        self.0.is_infinite()
    }

    /// Reports whether this is a special value (negative or infinity).
    #[inline]
    pub fn is_special(self) -> bool {
        self.is_negative() || self.is_infinity()
    }

    /// Reports whether the internal representation is valid.
    #[inline]
    pub fn is_valid(self) -> bool {
        (self.0 >= 0.0 && self.0 <= MAX_LENGTH2) || self.is_special()
    }

    // --- Successor / Predecessor ---

    /// Returns the smallest representable `ChordAngle` larger than this one.
    ///
    /// Special cases:
    /// - `NEGATIVE.successor()` → `ZERO`
    /// - `STRAIGHT.successor()` → `INFINITY`
    /// - `INFINITY.successor()` → `INFINITY`
    pub fn successor(self) -> ChordAngle {
        if self.0 >= MAX_LENGTH2 {
            ChordAngle::INFINITY
        } else if self.0 < 0.0 {
            ChordAngle::ZERO
        } else {
            ChordAngle(f64_next_after(self.0, 10.0))
        }
    }

    /// Returns the largest representable `ChordAngle` less than this one.
    ///
    /// Special cases:
    /// - `INFINITY.predecessor()` → `STRAIGHT`
    /// - `ZERO.predecessor()` → `NEGATIVE`
    /// - `NEGATIVE.predecessor()` → `NEGATIVE`
    pub fn predecessor(self) -> ChordAngle {
        if self.0 <= 0.0 {
            ChordAngle::NEGATIVE
        } else if self.0 > MAX_LENGTH2 {
            ChordAngle::STRAIGHT
        } else {
            ChordAngle(f64_next_after(self.0, -10.0))
        }
    }

    // --- Error bounds ---

    /// Returns this chord angle adjusted by the given error bound (which
    /// can be positive or negative). Special values are not modified.
    /// The result is clamped to the valid range \[0, 4\].
    pub fn plus_error(self, error: f64) -> ChordAngle {
        if self.is_special() {
            return self;
        }
        ChordAngle((self.0 + error).clamp(0.0, MAX_LENGTH2))
    }

    /// Returns the maximum error in `length2()` for a `ChordAngle` constructed
    /// from two unit-length points (assuming they are normalized to within the
    /// bounds guaranteed by `normalize()`).
    pub fn max_point_error(self) -> f64 {
        4.5 * f64::EPSILON * self.0 + 16.0 * f64::EPSILON * f64::EPSILON
    }

    /// Returns the maximum error in `length2()` for a `ChordAngle` constructed
    /// from an `Angle`.
    pub fn max_angle_error(self) -> f64 {
        f64::EPSILON * self.0
    }

    // --- Trigonometry ---

    /// Returns the sine of this chord angle. More efficient than converting
    /// to `Angle` and computing the sine.
    #[inline]
    pub fn sin(self) -> f64 {
        self.sin2().sqrt()
    }

    /// Returns the square of the sine of this chord angle. More efficient
    /// than `sin()` since it avoids a square root.
    #[inline]
    pub fn sin2(self) -> f64 {
        // sin(2A) = 2 sin(A) cos(A), and length2 = 4 sin²(A), so:
        // sin²(2A) = 4 sin²(A) (1 - sin²(A)) = length2 * (1 - length2/4)
        self.0 * (1.0 - 0.25 * self.0)
    }

    /// Returns the cosine of this chord angle.
    #[inline]
    pub fn cos(self) -> f64 {
        // cos(2A) = 1 - 2 sin²(A) = 1 - length2/2
        1.0 - 0.5 * self.0
    }

    /// Returns the tangent of this chord angle.
    #[inline]
    pub fn tan(self) -> f64 {
        self.sin() / self.cos()
    }
}

impl Default for ChordAngle {
    fn default() -> Self {
        ChordAngle::ZERO
    }
}

// --- Conversions ---

impl From<Angle> for ChordAngle {
    #[inline]
    fn from(angle: Angle) -> Self {
        ChordAngle::from_angle(angle)
    }
}

impl From<ChordAngle> for Angle {
    #[inline]
    fn from(ca: ChordAngle) -> Self {
        ca.to_angle()
    }
}

// --- Arithmetic ---

impl Add for ChordAngle {
    type Output = ChordAngle;

    /// Adds two chord angles. Both must be non-special.
    /// The result is clamped to at most `STRAIGHT` (180°).
    fn add(self, rhs: ChordAngle) -> ChordAngle {
        debug_assert!(!self.is_special());
        debug_assert!(!rhs.is_special());

        let a2 = self.0;
        let b2 = rhs.0;

        if b2 == 0.0 {
            return self;
        }
        if a2 + b2 >= MAX_LENGTH2 {
            return ChordAngle::STRAIGHT;
        }

        // Let a, b be the (non-squared) chord lengths.
        // Let A, B be the corresponding half-angles (a = 2 sin(A), etc.).
        // Then: c = a + b = 2 sin(A+B)
        // Using sin(A+B) = sin(A)cos(B) + sin(B)cos(A)
        //       cos(X)  = sqrt(1 - sin²(X))
        let x = a2 * (1.0 - 0.25 * b2);
        let y = b2 * (1.0 - 0.25 * a2);
        ChordAngle((x + y + 2.0 * (x * y).sqrt()).min(MAX_LENGTH2))
    }
}

impl Sub for ChordAngle {
    type Output = ChordAngle;

    /// Subtracts a chord angle from this one. Both must be non-special.
    /// The result is clamped to at least `ZERO`.
    fn sub(self, rhs: ChordAngle) -> ChordAngle {
        debug_assert!(!self.is_special());
        debug_assert!(!rhs.is_special());

        let a2 = self.0;
        let b2 = rhs.0;

        if b2 == 0.0 {
            return self;
        }
        if a2 <= b2 {
            return ChordAngle::ZERO;
        }

        let x = a2 * (1.0 - 0.25 * b2);
        let y = b2 * (1.0 - 0.25 * a2);
        // Use (sqrt(x) - sqrt(y))² to avoid excessive cancellation error.
        let d = (x.sqrt() - y.sqrt()).max(0.0);
        ChordAngle(d * d)
    }
}

impl fmt::Display for ChordAngle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_angle())
    }
}

/// Returns the next representable f64 after `from` in the direction of `to`.
/// Equivalent to C `nextafter`.
fn f64_next_after(from: f64, to: f64) -> f64 {
    if from == to {
        return to;
    }
    if from.is_nan() || to.is_nan() {
        return f64::NAN;
    }
    if from == 0.0 {
        // Step from ±0 towards `to`.
        let tiny = f64::from_bits(1);
        return if to > 0.0 { tiny } else { -tiny };
    }
    let bits = from.to_bits();
    let next_bits = if (from < to) == (from > 0.0) {
        bits + 1
    } else {
        bits - 1
    };
    f64::from_bits(next_bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn chord_angle_is_send_sync() {
        is_send_sync::<ChordAngle>();
    }

    /// Helper: approximate equality for f64.
    fn float64_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-15
    }

    /// Helper: approximate equality within a given tolerance.
    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn test_default_constructor() {
        let ca = ChordAngle::default();
        assert_eq!(ca.degrees(), 0.0);
    }

    #[test]
    fn test_from_length2() {
        assert_eq!(ChordAngle::from_length2(0.0).degrees(), 0.0);
        assert!(float64_near(
            ChordAngle::from_length2(1.0).degrees(),
            60.0,
            1e-13
        ));
        assert!(float64_near(
            ChordAngle::from_length2(2.0).degrees(),
            90.0,
            1e-13
        ));
        assert!(float64_near(
            ChordAngle::from_length2(4.0).degrees(),
            180.0,
            1e-13
        ));
        assert!(float64_near(
            ChordAngle::from_length2(5.0).degrees(),
            180.0,
            1e-13
        ));
    }

    #[test]
    fn test_zero() {
        assert_eq!(ChordAngle::ZERO.to_angle(), Angle::ZERO);
    }

    #[test]
    fn test_right() {
        assert!(float64_near(
            ChordAngle::RIGHT.to_angle().degrees(),
            90.0,
            1e-13,
        ));
    }

    #[test]
    fn test_straight() {
        assert_eq!(ChordAngle::STRAIGHT.to_angle(), Angle::from_degrees(180.0));
    }

    #[test]
    fn test_infinity() {
        assert!(ChordAngle::INFINITY > ChordAngle::STRAIGHT);
        assert_eq!(ChordAngle::INFINITY, ChordAngle::INFINITY);
        assert_eq!(ChordAngle::INFINITY.to_angle(), Angle::INFINITY);
    }

    #[test]
    fn test_negative() {
        assert!(ChordAngle::NEGATIVE < ChordAngle::ZERO);
        assert_eq!(ChordAngle::NEGATIVE, ChordAngle::NEGATIVE);
        assert!(ChordAngle::NEGATIVE.to_angle() < Angle::ZERO);
    }

    #[test]
    fn test_predicates() {
        assert!(ChordAngle::ZERO.is_zero());
        assert!(!ChordAngle::ZERO.is_negative());
        assert!(!ChordAngle::ZERO.is_special());

        assert!(!ChordAngle::NEGATIVE.is_zero());
        assert!(ChordAngle::NEGATIVE.is_negative());
        assert!(ChordAngle::NEGATIVE.is_special());

        assert!(!ChordAngle::STRAIGHT.is_zero());
        assert!(!ChordAngle::STRAIGHT.is_negative());
        assert!(!ChordAngle::STRAIGHT.is_special());
        assert!(!ChordAngle::STRAIGHT.is_infinity());

        assert!(!ChordAngle::INFINITY.is_zero());
        assert!(!ChordAngle::INFINITY.is_negative());
        assert!(ChordAngle::INFINITY.is_special());
        assert!(ChordAngle::INFINITY.is_infinity());
    }

    #[test]
    fn test_basics() {
        // From Go: TestChordAngleBasics
        let cases = [
            (ChordAngle::NEGATIVE, ChordAngle::NEGATIVE, false, true),
            (ChordAngle::NEGATIVE, ChordAngle::ZERO, true, false),
            (ChordAngle::NEGATIVE, ChordAngle::STRAIGHT, true, false),
            (ChordAngle::NEGATIVE, ChordAngle::INFINITY, true, false),
            (ChordAngle::ZERO, ChordAngle::ZERO, false, true),
            (ChordAngle::ZERO, ChordAngle::STRAIGHT, true, false),
            (ChordAngle::ZERO, ChordAngle::INFINITY, true, false),
            (ChordAngle::STRAIGHT, ChordAngle::STRAIGHT, false, true),
            (ChordAngle::STRAIGHT, ChordAngle::INFINITY, true, false),
            (ChordAngle::INFINITY, ChordAngle::INFINITY, false, true),
            (ChordAngle::INFINITY, ChordAngle::STRAIGHT, false, false),
        ];
        for (a, b, want_lt, want_eq) in &cases {
            assert_eq!(a < b, *want_lt, "{a:?} < {b:?}");
            assert_eq!(a == b, *want_eq, "{a:?} == {b:?}");
        }
    }

    #[test]
    fn test_to_from_angle() {
        assert_eq!(ChordAngle::from_angle(Angle::ZERO).radians(), 0.0);
        assert_eq!(
            ChordAngle::from_angle(Angle::from_radians(PI)).length2(),
            MAX_LENGTH2,
        );
        assert_eq!(
            ChordAngle::from_angle(Angle::from_radians(PI)).radians(),
            PI,
        );
        assert_eq!(
            ChordAngle::from_angle(Angle::INFINITY),
            ChordAngle::INFINITY,
        );
        assert!(ChordAngle::from_angle(Angle::from_radians(-1.0)).is_negative());
        assert!(float64_near(
            ChordAngle::from_angle(Angle::from_radians(1.0)).radians(),
            1.0,
            1e-15,
        ));
    }

    #[test]
    fn test_angle_equality() {
        // From Go: TestChordAngleAngleEquality
        assert_eq!(Angle::INFINITY, ChordAngle::INFINITY.to_angle());
        assert_eq!(Angle::from_degrees(180.0), ChordAngle::STRAIGHT.to_angle(),);
        assert_eq!(Angle::ZERO, ChordAngle::ZERO.to_angle());
        assert!(float64_near(
            ChordAngle::RIGHT.to_angle().degrees(),
            90.0,
            1e-13
        ));
    }

    #[test]
    fn test_from_angle_roundtrip() {
        // From Go: TestChordAngleFromAngle
        for angle in [0.0, 1.0, -1.0, PI] {
            assert_eq!(
                ChordAngle::from_angle(Angle::from_radians(angle))
                    .to_angle()
                    .radians(),
                angle,
                "roundtrip for {angle}",
            );
        }
        assert_eq!(
            ChordAngle::from_angle(Angle::from_radians(PI)),
            ChordAngle::STRAIGHT,
        );
        assert_eq!(
            ChordAngle::from_angle(Angle::INFINITY).to_angle(),
            Angle::INFINITY,
        );
    }

    #[test]
    fn test_successor() {
        assert_eq!(ChordAngle::NEGATIVE.successor(), ChordAngle::ZERO);
        assert_eq!(ChordAngle::STRAIGHT.successor(), ChordAngle::INFINITY);
        assert_eq!(ChordAngle::INFINITY.successor(), ChordAngle::INFINITY);

        let mut x = ChordAngle::NEGATIVE;
        for _ in 0..10 {
            assert!(
                x < x.successor(),
                "{x:?} should be less than {:?}",
                x.successor(),
            );
            x = x.successor();
        }
    }

    #[test]
    fn test_predecessor() {
        assert_eq!(ChordAngle::INFINITY.predecessor(), ChordAngle::STRAIGHT);
        assert_eq!(ChordAngle::ZERO.predecessor(), ChordAngle::NEGATIVE);
        assert_eq!(ChordAngle::NEGATIVE.predecessor(), ChordAngle::NEGATIVE);

        let mut x = ChordAngle::INFINITY;
        for _ in 0..10 {
            assert!(
                x > x.predecessor(),
                "{x:?} should be greater than {:?}",
                x.predecessor(),
            );
            x = x.predecessor();
        }
    }

    #[test]
    fn test_arithmetic() {
        let zero = ChordAngle::ZERO;
        let degree30 = ChordAngle::from_degrees(30.0);
        let degree60 = ChordAngle::from_degrees(60.0);
        let degree90 = ChordAngle::from_degrees(90.0);
        let degree120 = ChordAngle::from_degrees(120.0);
        let degree180 = ChordAngle::STRAIGHT;

        let add_cases: Vec<(ChordAngle, ChordAngle, ChordAngle)> = vec![
            (zero, zero, zero),
            (degree60, zero, degree60),
            (zero, degree60, degree60),
            (degree30, degree60, degree90),
            (degree60, degree30, degree90),
            (degree180, zero, degree180),
            (degree90, degree90, degree180),
            (degree120, degree90, degree180),
            (degree120, degree120, degree180),
            (degree30, degree180, degree180),
            (degree180, degree180, degree180),
        ];

        let sub_cases: Vec<(ChordAngle, ChordAngle, ChordAngle)> = vec![
            (zero, zero, zero),
            (degree60, degree60, zero),
            (degree180, degree180, zero),
            (zero, degree60, zero),
            (degree30, degree90, zero),
            (degree90, degree30, degree60),
            (degree90, degree60, degree30),
            (degree180, zero, degree180),
        ];

        for (a, b, want) in &add_cases {
            let got = *a + *b;
            assert!(
                float64_eq(got.length2(), want.length2()),
                "{:?} + {:?} = {}, want {}",
                a.to_angle().degrees(),
                b.to_angle().degrees(),
                got.length2(),
                want.length2(),
            );
        }

        for (a, b, want) in &sub_cases {
            let got = *a - *b;
            assert!(
                float64_eq(got.length2(), want.length2()),
                "{:?} - {:?} = {}, want {}",
                a.to_angle().degrees(),
                b.to_angle().degrees(),
                got.length2(),
                want.length2(),
            );
        }
    }

    #[test]
    fn test_arithmetic_precision() {
        // From C++: ArithmeticPrecision test
        // Verifies that ChordAngle is capable of adding and subtracting angles
        // extremely accurately up to π/2 radians.
        let eps = ChordAngle::from_radians(1e-15);
        let k90 = ChordAngle::RIGHT;
        let k90_minus_eps = k90 - eps;
        let k90_plus_eps = k90 + eps;
        let max_error = 2.0 * f64::EPSILON;

        assert!(
            float64_near(k90_minus_eps.radians(), PI / 2.0 - eps.radians(), max_error),
            "k90 - eps: {} vs {}",
            k90_minus_eps.radians(),
            PI / 2.0 - eps.radians(),
        );
        assert!(
            float64_near(k90_plus_eps.radians(), PI / 2.0 + eps.radians(), max_error),
            "k90 + eps: {} vs {}",
            k90_plus_eps.radians(),
            PI / 2.0 + eps.radians(),
        );
        assert!(
            float64_near((k90 - k90_minus_eps).radians(), eps.radians(), max_error),
            "k90 - (k90-eps): {} vs {}",
            (k90 - k90_minus_eps).radians(),
            eps.radians(),
        );
        assert!(
            float64_near((k90_plus_eps - k90).radians(), eps.radians(), max_error),
            "(k90+eps) - k90: {} vs {}",
            (k90_plus_eps - k90).radians(),
            eps.radians(),
        );
        assert!(
            float64_near((k90_minus_eps + eps).radians(), PI / 2.0, max_error),
            "(k90-eps) + eps: {} vs {}",
            (k90_minus_eps + eps).radians(),
            PI / 2.0,
        );
    }

    #[test]
    fn test_trigonometry() {
        let epsilon = 1e-14;
        let iters = 40;
        for iter in 0..=iters {
            let radians = PI * f64::from(iter) / f64::from(iters);
            let ca = ChordAngle::from_angle(Angle::from_radians(radians));
            assert!(
                float64_near(radians.sin(), ca.sin(), epsilon),
                "iter={iter}: sin({radians}) = {}, ca.sin() = {}",
                radians.sin(),
                ca.sin(),
            );
            assert!(
                float64_near(radians.cos(), ca.cos(), epsilon),
                "iter={iter}: cos({radians}) = {}, ca.cos() = {}",
                radians.cos(),
                ca.cos(),
            );
            // Since tan(x) is unbounded near pi/2, compare atan of both.
            assert!(
                float64_near(radians.tan().atan(), ca.tan().atan(), epsilon,),
                "iter={iter}: atan(tan({radians})) = {}, atan(ca.tan()) = {}",
                radians.tan().atan(),
                ca.tan().atan(),
            );
        }

        // ChordAngle can represent 90° and 180° exactly.
        let angle90 = ChordAngle::from_length2(2.0);
        let angle180 = ChordAngle::from_length2(4.0);

        assert!(float64_eq(1.0, angle90.sin()));
        assert!(float64_eq(0.0, angle90.cos()));
        assert!(angle90.tan().is_infinite());

        assert!(float64_eq(0.0, angle180.sin()));
        assert!(float64_eq(-1.0, angle180.cos()));
        assert!(float64_eq(0.0, angle180.tan()));
    }

    #[test]
    fn test_plus_error() {
        // Special values are unchanged.
        assert_eq!(ChordAngle::NEGATIVE.plus_error(5.0), ChordAngle::NEGATIVE,);
        assert_eq!(ChordAngle::INFINITY.plus_error(-5.0), ChordAngle::INFINITY,);

        // Clamped to valid range.
        assert_eq!(
            ChordAngle::STRAIGHT.plus_error(5.0),
            ChordAngle::from_length2(4.0),
        );
        assert_eq!(ChordAngle::ZERO.plus_error(-5.0), ChordAngle::ZERO);

        // Normal adjustment.
        assert_eq!(
            ChordAngle::from_length2(1.25).plus_error(0.25),
            ChordAngle::from_length2(1.5),
        );
        assert_eq!(
            ChordAngle::from_length2(1.25).plus_error(-0.25),
            ChordAngle::from_length2(1.0),
        );
    }

    #[test]
    fn test_expanded() {
        // From Go: TestChordAngleExpanded
        let zero = ChordAngle::ZERO;
        let cases = [
            (
                ChordAngle::NEGATIVE,
                5.0,
                ChordAngle::NEGATIVE.plus_error(5.0),
            ),
            (ChordAngle::INFINITY, -5.0, ChordAngle::INFINITY),
            (ChordAngle::STRAIGHT, 5.0, ChordAngle::from_length2(4.0)),
            (zero, -5.0, zero),
            (
                ChordAngle::from_length2(1.25),
                0.25,
                ChordAngle::from_length2(1.5),
            ),
            (
                ChordAngle::from_length2(0.75),
                0.25,
                ChordAngle::from_length2(1.0),
            ),
        ];
        for (have, add, want) in &cases {
            assert_eq!(
                have.plus_error(*add),
                *want,
                "{have:?}.plus_error({add}) = {:?}, want {want:?}",
                have.plus_error(*add),
            );
        }
    }

    #[test]
    fn test_is_valid() {
        assert!(ChordAngle::ZERO.is_valid());
        assert!(ChordAngle::RIGHT.is_valid());
        assert!(ChordAngle::STRAIGHT.is_valid());
        assert!(ChordAngle::INFINITY.is_valid());
        assert!(ChordAngle::NEGATIVE.is_valid());
        assert!(ChordAngle::from_length2(2.5).is_valid());
        assert!(!ChordAngle(5.0).is_valid());
        assert!(!ChordAngle(f64::NAN).is_valid());
    }

    // --- Trigonometry exact values (from C++ TrigonometryExactValues) ---

    #[test]
    fn test_trig_exact_zero() {
        let a = ChordAngle::ZERO;
        assert_eq!(a.sin(), 0.0);
        assert_eq!(a.cos(), 1.0);
        assert_eq!(a.tan(), 0.0);
    }

    #[test]
    fn test_trig_exact_90() {
        // 90°: length2 = 2, sin=1, cos=0, tan=inf
        let a = ChordAngle::from_length2(2.0);
        assert_eq!(a.sin(), 1.0);
        assert_eq!(a.cos(), 0.0);
        assert!(a.tan().is_infinite() && a.tan() > 0.0);
    }

    #[test]
    fn test_trig_exact_180() {
        // 180°: length2 = 4, sin=0, cos=-1, tan=0 (or -0)
        let a = ChordAngle::STRAIGHT;
        assert!(a.sin().abs() < 1e-15);
        assert_eq!(a.cos(), -1.0);
        assert!(a.tan().abs() < 1e-15);
    }

    #[test]
    fn test_trig_identity_sin2_plus_cos2() {
        // sin²(θ) + cos²(θ) = 1 for various angles.
        for deg in [0.0, 30.0, 45.0, 60.0, 90.0, 120.0, 150.0, 180.0] {
            let ca = ChordAngle::from_angle(Angle::from_degrees(deg));
            let s = ca.sin();
            let c = ca.cos();
            let sum = s * s + c * c;
            assert!(
                (sum - 1.0).abs() < 1e-14,
                "sin²+cos² at {deg}° = {sum} (should be 1.0)",
            );
        }
    }

    // --- Special value predicates ---

    #[test]
    fn test_special_value_predicates() {
        assert!(ChordAngle::ZERO.is_zero());
        assert!(!ChordAngle::ZERO.is_negative());
        assert!(!ChordAngle::ZERO.is_infinity());
        assert!(!ChordAngle::ZERO.is_special());
        assert!(ChordAngle::ZERO.is_valid());

        assert!(ChordAngle::NEGATIVE.is_negative());
        assert!(ChordAngle::NEGATIVE.is_special());
        // In our implementation, is_valid() returns true for special values.
        assert!(ChordAngle::NEGATIVE.is_valid());

        assert!(ChordAngle::INFINITY.is_infinity());
        assert!(ChordAngle::INFINITY.is_special());
        assert!(ChordAngle::INFINITY.is_valid());

        assert!(!ChordAngle::RIGHT.is_zero());
        assert!(!ChordAngle::RIGHT.is_special());
        assert!(ChordAngle::RIGHT.is_valid());

        assert!(!ChordAngle::STRAIGHT.is_zero());
        assert!(!ChordAngle::STRAIGHT.is_special());
        assert!(ChordAngle::STRAIGHT.is_valid());
    }

    // --- Large length2 clamping ---

    #[test]
    fn test_from_length2_clamping() {
        // length2 > 4 should clamp to STRAIGHT (180°).
        assert_eq!(ChordAngle::from_length2(4.0).degrees(), 180.0);
        assert_eq!(ChordAngle::from_length2(5.0).degrees(), 180.0);
        assert_eq!(ChordAngle::from_length2(100.0).degrees(), 180.0);
    }

    #[test]
    fn test_two_point_constructor() {
        // From C++: TwoPointConstructor - tests chord angle from two S2Points.
        use crate::r3::Vector;
        use crate::s2::Point;

        let x = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let y = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        let z = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });

        // Same point → 0.
        assert_eq!(z.chord_angle(z).radians(), 0.0);

        // Antipodal points → π.
        assert!(
            float64_near((-z).chord_angle(z).radians(), PI, 1e-7),
            "antipodal points should have chord angle π, got {}",
            (-z).chord_angle(z).radians(),
        );

        // Orthogonal points → π/2.
        assert!(float64_eq(x.chord_angle(z).radians(), PI / 2.0));

        // π/4 case: midpoint of y and z axes.
        let w = Point((y.0 + z.0).normalize());
        assert!(
            float64_near(w.chord_angle(z).radians(), PI / 4.0, 1e-15),
            "midpoint-to-axis should be π/4, got {}",
            w.chord_angle(z).radians(),
        );
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;
    use std::f64::consts::PI;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    #[quickcheck]
    fn prop_from_angle_roundtrip(r: f64) -> bool {
        // ChordAngle::from(Angle::from(ca)) ≈ ca for valid angles
        let r = clamp_finite(r).clamp(0.0, PI);
        let ca = ChordAngle::from_radians(r);
        let roundtrip = ChordAngle::from_angle(ca.to_angle());
        (ca.length2() - roundtrip.length2()).abs() < 1e-14 * ca.length2().max(1e-30)
    }

    #[quickcheck]
    fn prop_order_preservation(a: f64, b: f64) -> bool {
        // a < b iff Angle::from(a) < Angle::from(b)
        let a = clamp_finite(a).clamp(0.0, PI);
        let b = clamp_finite(b).clamp(0.0, PI);
        let ca = ChordAngle::from_radians(a);
        let cb = ChordAngle::from_radians(b);
        let angle_a = ca.to_angle();
        let angle_b = cb.to_angle();
        // Both orderings should agree
        (ca < cb) == (angle_a < angle_b) && (ca == cb) == (angle_a == angle_b)
    }

    #[quickcheck]
    fn prop_add_monotonic(a: f64, b: f64) -> bool {
        // (a + b) >= a and (a + b) >= b for non-negative a, b
        let a = ChordAngle::from_length2(clamp_finite(a).clamp(0.0, 4.0));
        let b = ChordAngle::from_length2(clamp_finite(b).clamp(0.0, 4.0));
        let sum = a + b;
        sum >= a && sum >= b
    }

    #[quickcheck]
    fn prop_sub_bounded(a: f64, b: f64) -> bool {
        // (a - b) <= a and (a - b) >= ZERO
        let a = ChordAngle::from_length2(clamp_finite(a).clamp(0.0, 4.0));
        let b = ChordAngle::from_length2(clamp_finite(b).clamp(0.0, 4.0));
        let diff = a - b;
        diff <= a && diff >= ChordAngle::ZERO
    }

    #[quickcheck]
    fn prop_successor_greater(a: f64) -> bool {
        let a = ChordAngle::from_length2(clamp_finite(a).clamp(0.0, 3.999));
        let s = a.successor();
        s > a
    }

    #[quickcheck]
    fn prop_predecessor_less(a: f64) -> bool {
        let a = ChordAngle::from_length2(clamp_finite(a).clamp(0.001, 4.0));
        let p = a.predecessor();
        p < a
    }

    #[quickcheck]
    fn prop_sin2_cos_identity(r: f64) -> bool {
        // sin²(a) + cos²(a) ≈ 1 for angles in (0, π)
        let r = clamp_finite(r).clamp(0.01, PI - 0.01);
        let ca = ChordAngle::from_radians(r);
        let s2 = ca.sin2();
        let c = ca.cos();
        (s2 + c * c - 1.0).abs() < 1e-12
    }

    #[quickcheck]
    fn prop_length2_in_valid_range(r: f64) -> bool {
        let r = clamp_finite(r).clamp(0.0, PI);
        let ca = ChordAngle::from_radians(r);
        ca.length2() >= 0.0 && ca.length2() <= 4.0
    }

    #[quickcheck]
    fn prop_is_valid_for_constructed(r: f64) -> bool {
        let r = clamp_finite(r).clamp(0.0, PI);
        let ca = ChordAngle::from_radians(r);
        ca.is_valid() && !ca.is_negative() && !ca.is_special()
    }

    #[quickcheck]
    fn prop_add_commutative(a: f64, b: f64) -> bool {
        let a = ChordAngle::from_length2(clamp_finite(a).clamp(0.0, 4.0));
        let b = ChordAngle::from_length2(clamp_finite(b).clamp(0.0, 4.0));
        (a + b).length2() == (b + a).length2()
    }

    #[quickcheck]
    fn prop_from_length2_upper_clamp(l: f64) -> bool {
        let l = clamp_finite(l).clamp(0.0, 100.0);
        let ca = ChordAngle::from_length2(l);
        // from_length2 clamps values > 4 to STRAIGHT (4.0)
        ca.length2() >= 0.0 && ca.length2() <= 4.0
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(n: u32) -> bool {
        let l = f64::from(n % 4001) / 1000.0; // [0.0, 4.0]
        let ca = ChordAngle::from_length2(l);
        let json = serde_json::to_string(&ca).unwrap();
        let back: ChordAngle = serde_json::from_str(&json).unwrap();
        back == ca
    }
}
