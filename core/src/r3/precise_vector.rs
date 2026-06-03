// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A 3D vector with exact (arbitrary-precision) components.
//!
//! Corresponds to Go `r3.PreciseVector`, C++ `Vector3<ExactFloat>`,
//! and Java `BigPoint`.
//!
//! Operations like `Normalize` and `Norm` are NOT supported because
//! they require division and square root. Only addition, subtraction,
//! multiplication, dot product, and cross product are available.

use super::Vector;
use super::exact_float::ExactFloat;
use std::fmt;

/// A vector in 3D space with exact (arbitrary-precision) components.
///
/// # Examples
///
/// ```
/// use s2rst::r3::PreciseVector;
///
/// let a = PreciseVector::new(1.0, 0.0, 0.0);
/// let b = PreciseVector::new(0.0, 1.0, 0.0);
///
/// // Cross product of x̂ and ŷ is ẑ.
/// let c = a.cross(&b);
/// assert_eq!(c.z.to_f64(), 1.0);
///
/// // Dot product of orthogonal vectors is zero.
/// assert!(a.dot(&b).is_zero());
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PreciseVector {
    /// The x-coordinate.
    pub x: ExactFloat,
    /// The y-coordinate.
    pub y: ExactFloat,
    /// The z-coordinate.
    pub z: ExactFloat,
}

impl PreciseVector {
    /// Creates a precise vector from three f64 values (exact conversion).
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        PreciseVector {
            x: ExactFloat::from(x),
            y: ExactFloat::from(y),
            z: ExactFloat::from(z),
        }
    }

    /// Creates a precise vector from an `r3::Vector`.
    pub fn from_vector(v: Vector) -> Self {
        Self::new(v.x, v.y, v.z)
    }

    /// Converts back to an `r3::Vector` by rounding to f64 and normalizing.
    pub fn to_vector(&self) -> Vector {
        Vector::new(self.x.to_f64(), self.y.to_f64(), self.z.to_f64()).normalize()
    }

    /// Reports whether all components are exactly equal.
    pub fn equal(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }

    /// Reports whether all components are exactly zero.
    pub fn is_zero(&self) -> bool {
        self.x.is_zero() && self.y.is_zero() && self.z.is_zero()
    }

    /// Reports whether this is exactly a unit vector (norm² == 1).
    pub fn is_unit(&self) -> bool {
        self.norm2() == ExactFloat::one()
    }

    /// Returns the exact squared norm.
    pub fn norm2(&self) -> ExactFloat {
        self.dot(self)
    }

    /// Returns the vector with component-wise absolute value.
    pub fn abs(&self) -> Self {
        PreciseVector {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Exact vector addition.
    pub fn add(&self, other: &Self) -> Self {
        PreciseVector {
            x: self.x.add(&other.x),
            y: self.y.add(&other.y),
            z: self.z.add(&other.z),
        }
    }

    /// Exact vector subtraction.
    pub fn sub(&self, other: &Self) -> Self {
        PreciseVector {
            x: self.x.sub(&other.x),
            y: self.y.sub(&other.y),
            z: self.z.sub(&other.z),
        }
    }

    /// Exact scalar multiplication.
    pub fn mul(&self, f: &ExactFloat) -> Self {
        PreciseVector {
            x: self.x.mul(f),
            y: self.y.mul(f),
            z: self.z.mul(f),
        }
    }

    /// Scalar multiplication by an f64 value (exact conversion then multiply).
    pub fn mul_f64(&self, f: f64) -> Self {
        self.mul(&ExactFloat::from(f))
    }

    /// Exact dot product.
    pub fn dot(&self, other: &Self) -> ExactFloat {
        self.x
            .mul(&other.x)
            .add(&self.y.mul(&other.y))
            .add(&self.z.mul(&other.z))
    }

    /// Exact cross product.
    pub fn cross(&self, other: &Self) -> Self {
        PreciseVector {
            x: self.y.mul(&other.z).sub(&self.z.mul(&other.y)),
            y: self.z.mul(&other.x).sub(&self.x.mul(&other.z)),
            z: self.x.mul(&other.y).sub(&self.y.mul(&other.x)),
        }
    }

    /// Returns the index (0, 1, or 2) of the largest absolute component.
    pub fn largest_component(&self) -> usize {
        let t = self.abs();
        if t.x > t.y {
            if t.x > t.z { 0 } else { 2 }
        } else if t.y > t.z {
            1
        } else {
            2
        }
    }

    /// Returns the index (0, 1, or 2) of the smallest absolute component.
    pub fn smallest_component(&self) -> usize {
        let t = self.abs();
        if t.x < t.y {
            if t.x < t.z { 0 } else { 2 }
        } else if t.y < t.z {
            1
        } else {
            2
        }
    }
}

impl Default for PreciseVector {
    fn default() -> Self {
        PreciseVector::new(0.0, 0.0, 0.0)
    }
}

impl fmt::Display for PreciseVector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn precise_vector_is_send_sync() {
        is_send_sync::<PreciseVector>();
    }

    fn precise_eq(a: &ExactFloat, b: &ExactFloat) -> bool {
        a == b
    }

    #[test]
    fn test_roundtrip() {
        let cases = [
            Vector::new(0.0, 0.0, 0.0),
            Vector::new(1.0, 2.0, 3.0),
            Vector::new(3.0, -4.0, 12.0),
            Vector::new(1.0, 1e-16, 1e-32),
        ];
        for v in &cases {
            let got = PreciseVector::from_vector(*v).to_vector();
            let want = v.normalize();
            assert!(
                got.approx_eq(want),
                "PreciseVector::from_vector({v:?}).to_vector() = {got:?}, want {want:?}"
            );
        }
    }

    #[test]
    fn test_is_unit() {
        let epsilon = 1e-14;
        assert!(!PreciseVector::new(0.0, 0.0, 0.0).is_unit());
        assert!(PreciseVector::new(1.0, 0.0, 0.0).is_unit());
        assert!(PreciseVector::new(0.0, 1.0, 0.0).is_unit());
        assert!(PreciseVector::new(0.0, 0.0, 1.0).is_unit());
        // Note: exact arithmetic — 1+2ε is NOT exactly unit.
        assert!(!PreciseVector::new(1.0 + 2.0 * epsilon, 0.0, 0.0).is_unit());
        assert!(!PreciseVector::new(1.0, 1.0, 1.0).is_unit());
    }

    #[test]
    fn test_norm2() {
        let cases: Vec<(PreciseVector, f64)> = vec![
            (PreciseVector::new(0.0, 0.0, 0.0), 0.0),
            (PreciseVector::new(0.0, 1.0, 0.0), 1.0),
            (PreciseVector::new(1.0, 1.0, 1.0), 3.0),
            (PreciseVector::new(1.0, 2.0, 3.0), 14.0),
            (PreciseVector::new(3.0, -4.0, 12.0), 169.0),
        ];
        for (v, want) in &cases {
            let got = v.norm2();
            let want_ef = ExactFloat::from(*want);
            assert!(
                precise_eq(&got, &want_ef),
                "{v}.norm2() = {got}, want {want}"
            );
        }
    }

    #[test]
    fn test_add() {
        let cases = [
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(1.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 2.0, 3.0),
                PreciseVector::new(4.0, 5.0, 7.0),
                PreciseVector::new(5.0, 7.0, 10.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                PreciseVector::new(1.0, -6.0, -6.0),
                PreciseVector::new(2.0, -9.0, -1.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            let got = v1.add(v2);
            assert!(got.equal(want), "{v1} + {v2} = {got}, want {want}");
        }
    }

    #[test]
    fn test_sub() {
        let cases = [
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(1.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 2.0, 3.0),
                PreciseVector::new(4.0, 5.0, 7.0),
                PreciseVector::new(-3.0, -3.0, -4.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                PreciseVector::new(1.0, -6.0, -6.0),
                PreciseVector::new(0.0, 3.0, 11.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            let got = v1.sub(v2);
            assert!(got.equal(want), "{v1} - {v2} = {got}, want {want}");
        }
    }

    #[test]
    fn test_mul() {
        let cases: Vec<(PreciseVector, f64, PreciseVector)> = vec![
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                3.0,
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                1.0,
                PreciseVector::new(1.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                0.0,
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                3.0,
                PreciseVector::new(3.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                -1.0,
                PreciseVector::new(-1.0, 3.0, -5.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                2.0,
                PreciseVector::new(2.0, -6.0, 10.0),
            ),
        ];
        for (v, f, want) in &cases {
            let ef = ExactFloat::from(*f);
            let got = v.mul(&ef);
            assert!(got.equal(want), "{v}.mul({f}) = {got}, want {want}");
        }
    }

    #[test]
    fn test_mul_f64() {
        let cases: Vec<(PreciseVector, f64, PreciseVector)> = vec![
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                3.0,
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                1.0,
                PreciseVector::new(1.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                0.0,
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                3.0,
                PreciseVector::new(3.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                -1.0,
                PreciseVector::new(-1.0, 3.0, -5.0),
            ),
            (
                PreciseVector::new(1.0, -3.0, 5.0),
                2.0,
                PreciseVector::new(2.0, -6.0, 10.0),
            ),
        ];
        for (v, f, want) in &cases {
            let got = v.mul_f64(*f);
            assert!(got.equal(want), "{v}.mul_f64({f}) = {got}, want {want}");
        }
    }

    #[test]
    fn test_dot() {
        let zero = ExactFloat::zero();
        let one = ExactFloat::one();
        let neg3 = ExactFloat::from(-3.0);

        // Dot with self (unit vectors).
        assert!(precise_eq(
            &PreciseVector::new(1.0, 0.0, 0.0).dot(&PreciseVector::new(1.0, 0.0, 0.0)),
            &one,
        ));
        assert!(precise_eq(
            &PreciseVector::new(0.0, 1.0, 0.0).dot(&PreciseVector::new(0.0, 1.0, 0.0)),
            &one,
        ));
        assert!(precise_eq(
            &PreciseVector::new(0.0, 0.0, 1.0).dot(&PreciseVector::new(0.0, 0.0, 1.0)),
            &one,
        ));
        // Perpendicular.
        assert!(precise_eq(
            &PreciseVector::new(1.0, 0.0, 0.0).dot(&PreciseVector::new(0.0, 1.0, 0.0)),
            &zero,
        ));
        assert!(precise_eq(
            &PreciseVector::new(1.0, 0.0, 0.0).dot(&PreciseVector::new(0.0, 1.0, 1.0)),
            &zero,
        ));
        // General.
        assert!(precise_eq(
            &PreciseVector::new(1.0, 1.0, 1.0).dot(&PreciseVector::new(-1.0, -1.0, -1.0)),
            &neg3,
        ));
        // Dot commutes.
        let v1 = PreciseVector::new(1.0, 1.0, 1.0);
        let v2 = PreciseVector::new(-1.0, -1.0, -1.0);
        assert!(precise_eq(&v1.dot(&v2), &v2.dot(&v1)));
    }

    #[test]
    fn test_cross() {
        let cases = [
            // Self cross = zero.
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            // Basis vector crosses.
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 1.0, 0.0),
                PreciseVector::new(0.0, 0.0, 1.0),
            ),
            (
                PreciseVector::new(0.0, 1.0, 0.0),
                PreciseVector::new(0.0, 0.0, 1.0),
                PreciseVector::new(1.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(0.0, 0.0, 1.0),
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 1.0, 0.0),
            ),
            (
                PreciseVector::new(0.0, 1.0, 0.0),
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, -1.0),
            ),
            // General.
            (
                PreciseVector::new(1.0, 2.0, 3.0),
                PreciseVector::new(-4.0, 5.0, -6.0),
                PreciseVector::new(-27.0, -6.0, 13.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            let got = v1.cross(v2);
            assert!(got.equal(want), "{v1} x {v2} = {got}, want {want}");
        }
    }

    #[test]
    fn test_identities() {
        let zero = ExactFloat::zero();
        let pairs = [
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 0.0, 0.0),
            ),
            (
                PreciseVector::new(0.0, 0.0, 0.0),
                PreciseVector::new(0.0, 1.0, 2.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 1.0, 0.0),
            ),
            (
                PreciseVector::new(1.0, 0.0, 0.0),
                PreciseVector::new(0.0, 1.0, 1.0),
            ),
            (
                PreciseVector::new(1.0, 1.0, 1.0),
                PreciseVector::new(-1.0, -1.0, -1.0),
            ),
            (
                PreciseVector::new(1.0, 2.0, 2.0),
                PreciseVector::new(-0.3, 0.4, -1.2),
            ),
        ];
        for (v1, v2) in &pairs {
            let c1 = v1.cross(v2);
            let c2 = v2.cross(v1);
            let d1 = v1.dot(v2);
            let d2 = v2.dot(v1);

            // Dot commutes (exact).
            assert!(precise_eq(&d1, &d2), "dot not commutative for {v1}, {v2}");
            // Cross anti-commutes (exact).
            assert!(
                c1.equal(&c2.mul_f64(-1.0)),
                "cross not anti-commutative for {v1}, {v2}: {c1} vs {c2}"
            );
            // Cross is orthogonal to both inputs (exact).
            assert!(precise_eq(&v1.dot(&c1), &zero), "{v1} . ({v1} x {v2}) != 0");
            assert!(precise_eq(&v2.dot(&c1), &zero), "{v2} . ({v1} x {v2}) != 0");
        }
    }

    #[test]
    fn test_largest_smallest_component() {
        let cases: Vec<(PreciseVector, usize, usize)> = vec![
            (PreciseVector::new(0.0, 0.0, 0.0), 2, 2),
            (PreciseVector::new(1.0, 0.0, 0.0), 0, 2),
            (PreciseVector::new(1.0, -1.0, 0.0), 1, 2),
            (PreciseVector::new(-1.0, -1.1, -1.1), 2, 0),
            (PreciseVector::new(0.5, -0.4, -0.5), 2, 1),
            (PreciseVector::new(1e-15, 1e-14, 1e-13), 2, 0),
        ];
        for (v, largest, smallest) in &cases {
            assert_eq!(
                v.largest_component(),
                *largest,
                "{v}.largest_component() = {}, want {largest}",
                v.largest_component()
            );
            assert_eq!(
                v.smallest_component(),
                *smallest,
                "{v}.smallest_component() = {}, want {smallest}",
                v.smallest_component()
            );
        }
    }

    #[test]
    fn test_is_zero() {
        assert!(PreciseVector::new(0.0, 0.0, 0.0).is_zero());
        // Negative zero is still zero.
        assert!(PreciseVector::new(0.0, -0.0, 0.0).is_zero());
        // Non-zero.
        assert!(!PreciseVector::new(0.0, 0.0, 1.0).is_zero());

        // 1e20 + 1 - 1e20 should equal 1 (exact arithmetic).
        let x = PreciseVector::new(1e20, 0.0, 0.0);
        let y = PreciseVector::new(1.0, 0.0, 0.0);
        let result = x.add(&y).add(&x.mul_f64(-1.0));
        assert!(!result.is_zero());

        // (1e20+1) - (1e20+1) should be zero.
        let xy = x.add(&y);
        assert!(xy.sub(&xy).is_zero());
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn pvec(x: f64, y: f64, z: f64) -> PreciseVector {
        fn clamp(v: f64) -> f64 {
            if v.is_finite() {
                v.clamp(-1e10, 1e10)
            } else {
                0.0
            }
        }
        PreciseVector::new(clamp(x), clamp(y), clamp(z))
    }

    #[quickcheck]
    fn prop_dot_commutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = pvec(ax, ay, az);
        let b = pvec(bx, by, bz);
        // Exact: must be exactly equal.
        a.dot(&b) == b.dot(&a)
    }

    #[quickcheck]
    fn prop_cross_orthogonal(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = pvec(ax, ay, az);
        let b = pvec(bx, by, bz);
        let c = a.cross(&b);
        // Exact: must be exactly zero.
        a.dot(&c).is_zero() && b.dot(&c).is_zero()
    }

    #[quickcheck]
    fn prop_cross_anticommutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = pvec(ax, ay, az);
        let b = pvec(bx, by, bz);
        let c1 = a.cross(&b);
        let c2 = b.cross(&a);
        // c1 == -c2 (exact).
        c1.equal(&c2.mul_f64(-1.0))
    }

    #[quickcheck]
    fn prop_add_commutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = pvec(ax, ay, az);
        let b = pvec(bx, by, bz);
        a.add(&b).equal(&b.add(&a))
    }

    #[quickcheck]
    fn prop_sub_is_add_neg(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = pvec(ax, ay, az);
        let b = pvec(bx, by, bz);
        // a - b == a + (-b) (exact)
        a.sub(&b).equal(&a.add(&b.mul_f64(-1.0)))
    }

    #[quickcheck]
    fn prop_norm2_non_negative(x: f64, y: f64, z: f64) -> bool {
        let v = pvec(x, y, z);
        // Exact: norm2 >= 0.
        v.norm2().signum() >= 0
    }

    #[quickcheck]
    fn prop_self_sub_self_is_zero(x: f64, y: f64, z: f64) -> bool {
        let v = pvec(x, y, z);
        v.sub(&v).is_zero()
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x: i32, y: i32, z: i32) -> bool {
        let v = PreciseVector::new(f64::from(x), f64::from(y), f64::from(z));
        let json = serde_json::to_string(&v).unwrap();
        let back: PreciseVector = serde_json::from_str(&json).unwrap();
        back == v
    }
}
