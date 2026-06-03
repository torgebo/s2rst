// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A 3D vector in ℝ³.
//!
//! Corresponds to C++ `Vector3_d` from `util/math/vector.h`.
//! Foundation type for `S2Point`.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Index, Mul, Neg, Sub};

/// Selects a coordinate axis of a 3D vector.
///
/// # Examples
///
/// ```
/// use s2rst::r3::{Axis, Vector};
///
/// let v = Vector::new(1.0, 2.0, 3.0);
/// assert_eq!(v[Axis::X], 1.0);
/// assert_eq!(v[Axis::Y], 2.0);
/// assert_eq!(v[Axis::Z], 3.0);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Axis {
    /// The x coordinate.
    #[default]
    X = 0,
    /// The y coordinate.
    Y = 1,
    /// The z coordinate.
    Z = 2,
}

impl Axis {
    /// All three axes in order.
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    /// Converts a `usize` (0, 1, 2) to the corresponding axis.
    ///
    /// # Panics
    /// Panics if `i >= 3`.
    #[inline]
    pub fn from_index(i: usize) -> Axis {
        Axis::ALL[i]
    }
}

/// A vector in 3D Euclidean space.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::r3::Vector;
///
/// let a = Vector::new(1.0, 2.0, 3.0);
/// let b = Vector::new(4.0, 5.0, 6.0);
///
/// // Dot and cross products
/// assert_eq!(a.dot(b), 32.0);
/// assert_eq!(a.cross(b), Vector::new(-3.0, 6.0, -3.0));
///
/// // Normalization
/// let n = a.normalize();
/// assert!((n.norm() - 1.0).abs() < 1e-15);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vector {
    /// The x-coordinate.
    pub x: f64,
    /// The y-coordinate.
    pub y: f64,
    /// The z-coordinate.
    pub z: f64,
}

impl Vector {
    /// Creates a new vector.
    #[inline]
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vector { x, y, z }
    }

    /// Returns the dot product of `self` and `other`.
    #[inline]
    pub fn dot(self, other: Vector) -> f64 {
        // Two FMAs + one mul (one rounding less than the unfused chain) when
        // the target has hardware FMA. On portable x86-64-v1 builds f64::mul_add
        // falls back to a libm software emulation that is ~50× slower than a
        // single mul, so we keep the unfused form there.
        if cfg!(target_feature = "fma") {
            self.x
                .mul_add(other.x, self.y.mul_add(other.y, self.z * other.z))
        } else {
            self.x * other.x + self.y * other.y + self.z * other.z
        }
    }

    /// Returns the cross product of `self` and `other`.
    #[inline]
    pub fn cross(self, other: Vector) -> Vector {
        // Each 2x2 determinant a*b - c*d compiles to one FMA on hardware-FMA
        // targets. On portable builds without hardware FMA we keep the
        // unfused form to avoid the libm emulation cost. See dot() above.
        if cfg!(target_feature = "fma") {
            Vector {
                x: self.y.mul_add(other.z, -(self.z * other.y)),
                y: self.z.mul_add(other.x, -(self.x * other.z)),
                z: self.x.mul_add(other.y, -(self.y * other.x)),
            }
        } else {
            Vector {
                x: self.y * other.z - self.z * other.y,
                y: self.z * other.x - self.x * other.z,
                z: self.x * other.y - self.y * other.x,
            }
        }
    }

    /// Returns the squared Euclidean norm.
    #[inline]
    pub fn norm2(self) -> f64 {
        self.dot(self)
    }

    /// Returns the Euclidean norm.
    #[inline]
    pub fn norm(self) -> f64 {
        self.norm2().sqrt()
    }

    /// Returns a unit vector in the same direction, or the zero vector if
    /// the input is zero-length.
    #[inline]
    pub fn normalize(self) -> Vector {
        let n2 = self.norm2();
        if n2 == 0.0 {
            return Vector::default();
        }
        self * (1.0 / n2.sqrt())
    }

    /// Reports whether this vector is approximately unit length.
    #[inline]
    pub fn is_unit(self) -> bool {
        const EPSILON: f64 = 5e-14;
        (self.norm2() - 1.0).abs() <= EPSILON
    }

    /// Returns the component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Vector {
        Vector {
            x: self.x.abs(),
            y: self.y.abs(),
            z: self.z.abs(),
        }
    }

    /// Returns the Euclidean distance between `self` and `other`.
    #[inline]
    pub fn distance(self, other: Vector) -> f64 {
        (self - other).norm()
    }

    /// Returns the angle between `self` and `other` in radians.
    /// Range: \[0, π\].
    #[inline]
    pub fn angle(self, other: Vector) -> f64 {
        // FMA in cross() means cross(self, self) is the rounding residual of
        // y*z (small but non-zero) instead of bit-exact zero, so atan2 here
        // would no longer return exactly 0. Short-circuit to preserve the
        // angle(p, p) == 0 invariant relied on by boundary_equals and
        // interpolate(_, p, p).
        if self == other {
            return 0.0;
        }
        f64::atan2(self.cross(other).norm(), self.dot(other))
    }

    /// Returns the index (0, 1, or 2) of the component with the largest
    /// absolute value.
    #[inline]
    pub fn largest_abs_component(self) -> usize {
        let t = self.abs();
        if t.x > t.y {
            if t.x > t.z { 0 } else { 2 }
        } else if t.y > t.z {
            1
        } else {
            2
        }
    }

    /// Returns the index (0, 1, or 2) of the component with the smallest
    /// absolute value.
    #[inline]
    pub fn smallest_abs_component(self) -> usize {
        let t = self.abs();
        if t.x < t.y {
            if t.x < t.z { 0 } else { 2 }
        } else if t.y < t.z {
            1
        } else {
            2
        }
    }

    /// Returns a unit vector orthogonal to `self`.
    ///
    /// `ortho(-v) == -ortho(v)` for all `v`.
    #[inline]
    pub fn ortho(self) -> Vector {
        // Choose the axis with the smallest component to cross with,
        // which is (largest_abs_component - 1) mod 3 in the C++ code.
        // Equivalently, set the component *before* the largest to 1.
        let mut temp = Vector::default();
        match self.largest_abs_component() {
            0 => temp.z = 1.0,
            1 => temp.x = 1.0,
            _ => temp.y = 1.0,
        }
        self.cross(temp).normalize()
    }

    /// Lexicographic comparison. Returns `Ordering`.
    #[inline]
    #[expect(
        clippy::should_implement_trait,
        reason = "named constructor avoids ambiguity with std traits"
    )]
    pub fn cmp(self, other: Vector) -> Ordering {
        self.x
            .partial_cmp(&other.x)
            .unwrap_or(Ordering::Equal)
            .then(self.y.partial_cmp(&other.y).unwrap_or(Ordering::Equal))
            .then(self.z.partial_cmp(&other.z).unwrap_or(Ordering::Equal))
    }

    /// Reports whether all components are within `margin` of `other`.
    #[inline]
    pub fn approx_eq(self, other: Vector) -> bool {
        const EPSILON: f64 = 1e-16;
        (self.x - other.x).abs() < EPSILON
            && (self.y - other.y).abs() < EPSILON
            && (self.z - other.z).abs() < EPSILON
    }

    /// Reports whether all components are within `margin` of `other`.
    #[inline]
    pub fn aequal(self, other: Vector, margin: f64) -> bool {
        (self.x - other.x).abs() <= margin
            && (self.y - other.y).abs() <= margin
            && (self.z - other.z).abs() <= margin
    }

    /// Returns the component-wise product of `self` and `other`.
    #[inline]
    pub fn mul_components(self, other: Vector) -> Vector {
        Vector {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }

    /// Returns the component-wise quotient of `self` and `other`.
    #[inline]
    pub fn div_components(self, other: Vector) -> Vector {
        Vector {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }

    /// Returns the component-wise maximum of `a` and `b`.
    #[inline]
    pub fn max(a: Vector, b: Vector) -> Vector {
        Vector {
            x: a.x.max(b.x),
            y: a.y.max(b.y),
            z: a.z.max(b.z),
        }
    }

    /// Returns the component-wise minimum of `a` and `b`.
    #[inline]
    pub fn min(a: Vector, b: Vector) -> Vector {
        Vector {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            z: a.z.min(b.z),
        }
    }
}

// --- Arithmetic operator impls ---

impl Add for Vector {
    type Output = Vector;
    #[inline]
    fn add(self, rhs: Vector) -> Vector {
        Vector {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for Vector {
    type Output = Vector;
    #[inline]
    fn sub(self, rhs: Vector) -> Vector {
        Vector {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }
}

impl Mul<f64> for Vector {
    type Output = Vector;
    #[inline]
    fn mul(self, rhs: f64) -> Vector {
        Vector {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }
}

impl Mul<Vector> for f64 {
    type Output = Vector;
    #[inline]
    fn mul(self, rhs: Vector) -> Vector {
        rhs * self
    }
}

impl Div<f64> for Vector {
    type Output = Vector;
    #[inline]
    fn div(self, rhs: f64) -> Vector {
        Vector {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl Neg for Vector {
    type Output = Vector;
    #[inline]
    fn neg(self) -> Vector {
        Vector {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Index<Axis> for Vector {
    type Output = f64;
    #[inline]
    fn index(&self, axis: Axis) -> &f64 {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
            Axis::Z => &self.z,
        }
    }
}

impl From<(f64, f64, f64)> for Vector {
    fn from((x, y, z): (f64, f64, f64)) -> Self {
        Vector { x, y, z }
    }
}

impl From<[f64; 3]> for Vector {
    fn from([x, y, z]: [f64; 3]) -> Self {
        Vector { x, y, z }
    }
}

impl fmt::Display for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn vector_is_send_sync() {
        is_send_sync::<Vector>();
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-14
    }

    #[test]
    fn test_norm() {
        assert!(approx_eq(Vector::new(0.0, 0.0, 0.0).norm(), 0.0));
        assert!(approx_eq(Vector::new(0.0, 1.0, 0.0).norm(), 1.0));
        assert!(approx_eq(Vector::new(3.0, -4.0, 12.0).norm(), 13.0));
        assert!(approx_eq(Vector::new(1.0, 1e-16, 1e-32).norm(), 1.0));
    }

    #[test]
    fn test_norm2() {
        assert!(approx_eq(Vector::new(0.0, 0.0, 0.0).norm2(), 0.0));
        assert!(approx_eq(Vector::new(0.0, 1.0, 0.0).norm2(), 1.0));
        assert!(approx_eq(Vector::new(1.0, 1.0, 1.0).norm2(), 3.0));
        assert!(approx_eq(Vector::new(1.0, 2.0, 3.0).norm2(), 14.0));
        assert!(approx_eq(Vector::new(3.0, -4.0, 12.0).norm2(), 169.0));
    }

    #[test]
    fn test_normalize() {
        let cases = [
            Vector::new(1.0, 0.0, 0.0),
            Vector::new(0.0, 1.0, 0.0),
            Vector::new(0.0, 0.0, 1.0),
            Vector::new(1.0, 1.0, 1.0),
            Vector::new(1.0, 1e-16, 1e-32),
            Vector::new(12.34, 56.78, 91.01),
        ];
        for v in &cases {
            let nv = v.normalize();
            // Preserves direction.
            assert!(
                approx_eq(v.x * nv.y, v.y * nv.x),
                "{v:?}.normalize() did not preserve direction"
            );
            assert!(
                approx_eq(v.x * nv.z, v.z * nv.x),
                "{v:?}.normalize() did not preserve direction"
            );
            // Unit length.
            assert!(
                approx_eq(nv.norm(), 1.0),
                "|{v:?}.normalize()| = {}, want 1",
                nv.norm()
            );
        }
        // Zero vector stays zero.
        assert_eq!(Vector::new(0.0, 0.0, 0.0).normalize(), Vector::default());
    }

    #[test]
    fn test_is_unit() {
        const EPSILON: f64 = 1e-14;
        assert!(!Vector::new(0.0, 0.0, 0.0).is_unit());
        assert!(Vector::new(0.0, 1.0, 0.0).is_unit());
        assert!(Vector::new(1.0 + 2.0 * EPSILON, 0.0, 0.0).is_unit());
        assert!(Vector::new(1.0 * (1.0 + EPSILON), 0.0, 0.0).is_unit());
        assert!(!Vector::new(1.0, 1.0, 1.0).is_unit());
        assert!(Vector::new(1.0, 1e-16, 1e-32).is_unit());
    }

    #[test]
    fn test_dot() {
        assert!(approx_eq(
            Vector::new(1.0, 0.0, 0.0).dot(Vector::new(1.0, 0.0, 0.0)),
            1.0
        ));
        assert!(approx_eq(
            Vector::new(1.0, 0.0, 0.0).dot(Vector::new(0.0, 1.0, 0.0)),
            0.0
        ));
        assert!(approx_eq(
            Vector::new(1.0, 0.0, 0.0).dot(Vector::new(0.0, 1.0, 1.0)),
            0.0
        ));
        assert!(approx_eq(
            Vector::new(1.0, 1.0, 1.0).dot(Vector::new(-1.0, -1.0, -1.0)),
            -3.0
        ));
        assert!(approx_eq(
            Vector::new(1.0, 2.0, 2.0).dot(Vector::new(-0.3, 0.4, -1.2)),
            -1.9
        ));
    }

    #[test]
    fn test_cross() {
        let cases = [
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
            ),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 1.0, 0.0),
                Vector::new(0.0, 0.0, 1.0),
            ),
            (
                Vector::new(0.0, 1.0, 0.0),
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, -1.0),
            ),
            (
                Vector::new(1.0, 2.0, 3.0),
                Vector::new(-4.0, 5.0, -6.0),
                Vector::new(-27.0, -6.0, 13.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            assert!(
                v1.cross(*v2).approx_eq(*want),
                "{v1:?} x {v2:?} = {:?}, want {want:?}",
                v1.cross(*v2)
            );
        }
    }

    #[test]
    fn test_add() {
        let cases = [
            (
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
            ),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(1.0, 0.0, 0.0),
            ),
            (
                Vector::new(1.0, 2.0, 3.0),
                Vector::new(4.0, 5.0, 7.0),
                Vector::new(5.0, 7.0, 10.0),
            ),
            (
                Vector::new(1.0, -3.0, 5.0),
                Vector::new(1.0, -6.0, -6.0),
                Vector::new(2.0, -9.0, -1.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            assert!(
                (*v1 + *v2).approx_eq(*want),
                "{v1:?} + {v2:?} = {:?}, want {want:?}",
                *v1 + *v2
            );
        }
    }

    #[test]
    fn test_sub() {
        let cases = [
            (
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
            ),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(1.0, 0.0, 0.0),
            ),
            (
                Vector::new(1.0, 2.0, 3.0),
                Vector::new(4.0, 5.0, 7.0),
                Vector::new(-3.0, -3.0, -4.0),
            ),
            (
                Vector::new(1.0, -3.0, 5.0),
                Vector::new(1.0, -6.0, -6.0),
                Vector::new(0.0, 3.0, 11.0),
            ),
        ];
        for (v1, v2, want) in &cases {
            assert!(
                (*v1 - *v2).approx_eq(*want),
                "{v1:?} - {v2:?} = {:?}, want {want:?}",
                *v1 - *v2
            );
        }
    }

    #[test]
    fn test_mul() {
        let cases = [
            (Vector::new(0.0, 0.0, 0.0), 3.0, Vector::new(0.0, 0.0, 0.0)),
            (Vector::new(1.0, 0.0, 0.0), 1.0, Vector::new(1.0, 0.0, 0.0)),
            (Vector::new(1.0, 0.0, 0.0), 0.0, Vector::new(0.0, 0.0, 0.0)),
            (Vector::new(1.0, 0.0, 0.0), 3.0, Vector::new(3.0, 0.0, 0.0)),
            (
                Vector::new(1.0, -3.0, 5.0),
                -1.0,
                Vector::new(-1.0, 3.0, -5.0),
            ),
            (
                Vector::new(1.0, -3.0, 5.0),
                2.0,
                Vector::new(2.0, -6.0, 10.0),
            ),
        ];
        for (v, m, want) in &cases {
            assert!(
                (*v * *m).approx_eq(*want),
                "{m} * {v:?} = {:?}, want {want:?}",
                *v * *m
            );
        }
    }

    #[test]
    fn test_distance() {
        let cases = [
            (Vector::new(1.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0), 0.0),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 1.0, 0.0),
                std::f64::consts::SQRT_2,
            ),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 1.0, 1.0),
                1.73205080756888,
            ),
            (
                Vector::new(1.0, 1.0, 1.0),
                Vector::new(-1.0, -1.0, -1.0),
                3.46410161513775,
            ),
            (
                Vector::new(1.0, 2.0, 2.0),
                Vector::new(-0.3, 0.4, -1.2),
                3.80657326213486,
            ),
        ];
        for (v1, v2, want) in &cases {
            assert!(
                approx_eq(v1.distance(*v2), *want),
                "{v1:?}.distance({v2:?}) = {}, want {want}",
                v1.distance(*v2)
            );
            // Symmetry.
            assert!(
                approx_eq(v2.distance(*v1), *want),
                "{v2:?}.distance({v1:?}) = {}, want {want}",
                v2.distance(*v1)
            );
        }
    }

    #[test]
    fn test_angle() {
        let cases = [
            (Vector::new(1.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0), 0.0),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 1.0, 0.0),
                PI / 2.0,
            ),
            (
                Vector::new(1.0, 0.0, 0.0),
                Vector::new(0.0, 1.0, 1.0),
                PI / 2.0,
            ),
            (Vector::new(1.0, 0.0, 0.0), Vector::new(-1.0, 0.0, 0.0), PI),
            (
                Vector::new(1.0, 2.0, 3.0),
                Vector::new(2.0, 3.0, -1.0),
                1.2055891055045298,
            ),
        ];
        for (v1, v2, want) in &cases {
            assert!(
                approx_eq(v1.angle(*v2), *want),
                "{v1:?}.angle({v2:?}) = {}, want {want}",
                v1.angle(*v2)
            );
            // Angle is commutative.
            assert!(
                approx_eq(v2.angle(*v1), *want),
                "{v2:?}.angle({v1:?}) = {}, want {want}",
                v2.angle(*v1)
            );
        }
    }

    #[test]
    fn test_ortho() {
        let vectors = [
            Vector::new(1.0, 0.0, 0.0),
            Vector::new(1.0, 1.0, 0.0),
            Vector::new(1.0, 2.0, 3.0),
            Vector::new(1.0, -2.0, -5.0),
            Vector::new(0.012, 0.0053, 0.00457),
            Vector::new(-0.012, -1.0, -0.00457),
        ];
        for v in &vectors {
            let o = v.ortho();
            assert!(
                approx_eq(v.dot(o), 0.0),
                "{v:?}.dot({v:?}.ortho()) = {}, want 0",
                v.dot(o)
            );
            assert!(
                approx_eq(o.norm(), 1.0),
                "|{v:?}.ortho()| = {}, want 1",
                o.norm()
            );
        }
    }

    #[test]
    fn test_ortho_alignment() {
        // Verify specific axis-aligned results match Go/C++ behavior.
        assert_eq!(
            Vector::new(1.0, 0.0, 0.0).ortho(),
            Vector::new(0.0, -1.0, 0.0)
        );
        assert_eq!(
            Vector::new(0.0, 1.0, 0.0).ortho(),
            Vector::new(0.0, 0.0, -1.0)
        );
        assert_eq!(
            Vector::new(0.0, 0.0, 1.0).ortho(),
            Vector::new(-1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn test_largest_smallest_component() {
        let cases: [(Vector, usize, usize); 6] = [
            (Vector::new(0.0, 0.0, 0.0), 2, 2),
            (Vector::new(1.0, 0.0, 0.0), 0, 2),
            (Vector::new(1.0, -1.0, 0.0), 1, 2),
            (Vector::new(-1.0, -1.1, -1.1), 2, 0),
            (Vector::new(0.5, -0.4, -0.5), 2, 1),
            (Vector::new(1e-15, 1e-14, 1e-13), 2, 0),
        ];
        for (v, largest, smallest) in &cases {
            assert_eq!(
                v.largest_abs_component(),
                *largest,
                "{v:?}.largest_abs_component() = {}, want {largest}",
                v.largest_abs_component()
            );
            assert_eq!(
                v.smallest_abs_component(),
                *smallest,
                "{v:?}.smallest_abs_component() = {}, want {smallest}",
                v.smallest_abs_component()
            );
        }
    }

    #[test]
    fn test_cmp() {
        use std::cmp::Ordering::*;
        let cases = [
            (
                Vector::new(0.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Equal,
            ),
            (Vector::new(0.0, 0.0, 0.0), Vector::new(1.0, 0.0, 0.0), Less),
            (
                Vector::new(0.0, 1.0, 0.0),
                Vector::new(0.0, 0.0, 0.0),
                Greater,
            ),
            (Vector::new(1.0, 2.0, 3.0), Vector::new(3.0, 2.0, 1.0), Less),
            (
                Vector::new(-1.0, 0.0, 0.0),
                Vector::new(0.0, 0.0, -1.0),
                Less,
            ),
            (
                Vector::new(8.0, 6.0, 4.0),
                Vector::new(7.0, 5.0, 3.0),
                Greater,
            ),
            (
                Vector::new(-1.0, -0.5, 0.0),
                Vector::new(0.0, 0.0, 0.1),
                Less,
            ),
            (Vector::new(1.0, 2.0, 3.0), Vector::new(2.0, 3.0, 4.0), Less),
            (
                Vector::new(1.23, 4.56, 7.89),
                Vector::new(1.23, 4.56, 7.89),
                Equal,
            ),
        ];
        for (a, b, want) in &cases {
            assert_eq!(
                a.cmp(*b),
                *want,
                "{a:?}.cmp({b:?}) = {:?}, want {want:?}",
                a.cmp(*b)
            );
        }
    }

    #[test]
    fn test_identities() {
        let pairs = [
            (Vector::new(0.0, 0.0, 0.0), Vector::new(0.0, 0.0, 0.0)),
            (Vector::new(0.0, 0.0, 0.0), Vector::new(0.0, 1.0, 2.0)),
            (Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 0.0)),
            (Vector::new(1.0, 0.0, 0.0), Vector::new(0.0, 1.0, 1.0)),
            (Vector::new(1.0, 1.0, 1.0), Vector::new(-1.0, -1.0, -1.0)),
            (Vector::new(1.0, 2.0, 2.0), Vector::new(-0.3, 0.4, -1.2)),
        ];
        for (v1, v2) in &pairs {
            // Angle commutes.
            assert!(
                approx_eq(v1.angle(*v2), v2.angle(*v1)),
                "angle not commutative for {v1:?}, {v2:?}"
            );
            // Dot commutes.
            assert!(
                approx_eq(v1.dot(*v2), v2.dot(*v1)),
                "dot not commutative for {v1:?}, {v2:?}"
            );
            // Cross anti-commutes.
            let c1 = v1.cross(*v2);
            let c2 = v2.cross(*v1);
            assert!(
                c1.approx_eq(-c2),
                "cross not anti-commutative for {v1:?}, {v2:?}: {c1:?} vs -{c2:?}"
            );
            // Cross is orthogonal to both inputs.
            assert!(
                approx_eq(v1.dot(c1), 0.0),
                "{v1:?} . ({v1:?} x {v2:?}) = {}, want 0",
                v1.dot(c1)
            );
            assert!(
                approx_eq(v2.dot(c1), 0.0),
                "{v2:?} . ({v1:?} x {v2:?}) = {}, want 0",
                v2.dot(c1)
            );
        }
    }

    #[test]
    fn test_arithmetic_ops() {
        let a = Vector::new(1.0, 2.0, 3.0);
        let b = Vector::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vector::new(5.0, 7.0, 9.0));
        assert_eq!(a - b, Vector::new(-3.0, -3.0, -3.0));
        assert_eq!(a * 3.0, Vector::new(3.0, 6.0, 9.0));
        assert_eq!(3.0 * a, Vector::new(3.0, 6.0, 9.0));
        assert_eq!(a / 2.0, Vector::new(0.5, 1.0, 1.5));
        assert_eq!(-a, Vector::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn test_index() {
        let v = Vector::new(1.0, 2.0, 3.0);
        assert_eq!(v[Axis::X], 1.0);
        assert_eq!(v[Axis::Y], 2.0);
        assert_eq!(v[Axis::Z], 3.0);
    }

    #[test]
    fn test_from_tuple_and_array() {
        let v1: Vector = (1.0, 2.0, 3.0).into();
        let v2: Vector = [1.0, 2.0, 3.0].into();
        assert_eq!(v1, Vector::new(1.0, 2.0, 3.0));
        assert_eq!(v2, Vector::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Vector::new(1.0, 2.0, 3.0)), "(1, 2, 3)");
    }

    #[test]
    fn test_mul_components() {
        let a = Vector::new(1.0, 2.0, 3.0);
        let b = Vector::new(4.0, 5.0, 6.0);
        assert_eq!(a.mul_components(b), Vector::new(4.0, 10.0, 18.0));
    }

    #[test]
    fn test_div_components() {
        let a = Vector::new(4.0, 10.0, 18.0);
        let b = Vector::new(4.0, 5.0, 6.0);
        assert_eq!(a.div_components(b), Vector::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn test_component_max() {
        let a = Vector::new(1.0, 5.0, 3.0);
        let b = Vector::new(4.0, 2.0, 6.0);
        assert_eq!(Vector::max(a, b), Vector::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn test_component_min() {
        let a = Vector::new(1.0, 5.0, 3.0);
        let b = Vector::new(4.0, 2.0, 6.0);
        assert_eq!(Vector::min(a, b), Vector::new(1.0, 2.0, 3.0));
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn vec_from(x: f64, y: f64, z: f64) -> Vector {
        fn clamp_finite(v: f64) -> f64 {
            if v.is_finite() {
                v.clamp(-1e10, 1e10)
            } else {
                0.0
            }
        }
        Vector::new(clamp_finite(x), clamp_finite(y), clamp_finite(z))
    }

    #[quickcheck]
    fn prop_dot_commutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        (a.dot(b) - b.dot(a)).abs() < 1e-10
    }

    #[quickcheck]
    fn prop_cross_orthogonal(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        let c = a.cross(b);
        // Cross product is orthogonal to both inputs.
        // Use relative tolerance for large vectors.
        let tol = 1e-6 * a.norm() * b.norm() * (a.norm() + b.norm());
        c.dot(a).abs() <= tol + 1e-30 && c.dot(b).abs() <= tol + 1e-30
    }

    #[quickcheck]
    fn prop_cross_anticommutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        let c1 = a.cross(b);
        let c2 = b.cross(a);
        let tol = 1e-6 * a.norm() * b.norm() * (a.norm() + b.norm());
        (c1.x + c2.x).abs() <= tol + 1e-30
            && (c1.y + c2.y).abs() <= tol + 1e-30
            && (c1.z + c2.z).abs() <= tol + 1e-30
    }

    #[quickcheck]
    fn prop_normalize_unit(ax: f64, ay: f64, az: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let n = a.normalize();
        if a.norm2() == 0.0 {
            n == Vector::default()
        } else {
            (n.norm() - 1.0).abs() < 1e-10
        }
    }

    #[quickcheck]
    fn prop_add_commutative(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        (a + b) == (b + a)
    }

    #[quickcheck]
    fn prop_scalar_mul_distributes(
        ax: f64,
        ay: f64,
        az: f64,
        bx: f64,
        by: f64,
        bz: f64,
        s: f64,
    ) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        let s = if s.is_finite() {
            s.clamp(-1e5, 1e5)
        } else {
            1.0
        };
        let lhs = (a + b) * s;
        let rhs = a * s + b * s;
        lhs.aequal(
            rhs,
            1e-6 * (a.norm() + b.norm()).max(1.0) * s.abs().max(1.0),
        )
    }

    #[quickcheck]
    fn prop_norm_non_negative(x: f64, y: f64, z: f64) -> bool {
        let v = vec_from(x, y, z);
        v.norm() >= 0.0 && v.norm2() >= 0.0
    }

    #[quickcheck]
    fn prop_distance_symmetric(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        (a.distance(b) - b.distance(a)).abs() < 1e-10
    }

    #[quickcheck]
    fn prop_angle_symmetric(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        if a.norm2() == 0.0 || b.norm2() == 0.0 {
            return true;
        }
        (a.angle(b) - b.angle(a)).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_ortho_is_orthogonal(x: f64, y: f64, z: f64) -> bool {
        let v = vec_from(x, y, z);
        if v.norm2() == 0.0 {
            return true;
        }
        // Use relative tolerance: error scales with vector magnitude.
        v.dot(v.ortho()).abs() < 1e-10 * v.norm()
    }

    #[quickcheck]
    fn prop_double_neg(x: f64, y: f64, z: f64) -> bool {
        let v = vec_from(x, y, z);
        -(-v) == v
    }

    #[quickcheck]
    fn prop_angle_in_range(ax: f64, ay: f64, az: f64, bx: f64, by: f64, bz: f64) -> bool {
        let a = vec_from(ax, ay, az);
        let b = vec_from(bx, by, bz);
        if a.norm2() == 0.0 || b.norm2() == 0.0 {
            return true;
        }
        let angle = a.angle(b);
        (0.0..=std::f64::consts::PI).contains(&angle)
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x: i32, y: i32, z: i32) -> bool {
        let v = Vector::new(f64::from(x), f64::from(y), f64::from(z));
        let json = serde_json::to_string(&v).unwrap();
        let back: Vector = serde_json::from_str(&json).unwrap();
        back == v
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_axis_roundtrip() {
        for a in [Axis::X, Axis::Y, Axis::Z] {
            let json = serde_json::to_string(&a).unwrap();
            let back: Axis = serde_json::from_str(&json).unwrap();
            assert_eq!(a, back);
        }
    }
}
