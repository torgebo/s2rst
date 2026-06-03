// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A 2D point / vector in ℝ².
//!
//! Corresponds to C++ `R2Point` / `Vector2_d`.

use std::fmt;
use std::ops::{Add, Div, Index, Mul, Neg, Sub};

/// Selects a coordinate axis of a 2D point.
///
/// # Examples
///
/// ```
/// use s2rst::r2::{Axis, Point};
///
/// let p = Point::new(3.0, 4.0);
/// assert_eq!(p[Axis::X], 3.0);
/// assert_eq!(p[Axis::Y], 4.0);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Axis {
    /// The x coordinate.
    #[default]
    X,
    /// The y coordinate.
    Y,
}

/// A point (or vector) in 2D Euclidean space.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::r2::Point;
///
/// let a = Point::new(3.0, 4.0);
/// let b = Point::new(1.0, 2.0);
///
/// // Arithmetic
/// assert_eq!(a + b, Point::new(4.0, 6.0));
/// assert_eq!(a - b, Point::new(2.0, 2.0));
///
/// // Dot and cross products
/// assert_eq!(a.dot(b), 11.0);   // 3*1 + 4*2
/// assert_eq!(a.cross(b), 2.0);  // 3*2 - 4*1
///
/// // Norm and normalization
/// assert_eq!(a.norm(), 5.0);
/// let n = a.normalize();
/// assert!((n.norm() - 1.0).abs() < 1e-15);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    /// The x-coordinate.
    pub x: f64,
    /// The y-coordinate.
    pub y: f64,
}

impl Point {
    /// Creates a new point.
    #[inline]
    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    /// Returns the dot product of `self` and `other`.
    #[inline]
    pub fn dot(self, other: Point) -> f64 {
        self.x * other.x + self.y * other.y
    }

    /// Returns the cross product (z-component of the 3D cross product).
    #[inline]
    pub fn cross(self, other: Point) -> f64 {
        self.x * other.y - self.y * other.x
    }

    /// Returns a counterclockwise orthogonal vector with the same norm.
    #[inline]
    pub fn ortho(self) -> Point {
        Point {
            x: -self.y,
            y: self.x,
        }
    }

    /// Returns the squared Euclidean norm.
    #[inline]
    pub fn norm2(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Returns the Euclidean norm.
    #[inline]
    pub fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }

    /// Returns a unit vector in the same direction, or zero if the vector
    /// is zero-length.
    #[inline]
    pub fn normalize(self) -> Point {
        if self.x == 0.0 && self.y == 0.0 {
            return self;
        }
        self * (1.0 / self.norm())
    }

    /// Returns the angle from `self` to `other` in the counterclockwise
    /// direction, in radians. Range: \[-π, π\].
    #[inline]
    pub fn angle(self, other: Point) -> f64 {
        f64::atan2(self.cross(other), self.dot(other))
    }

    /// Returns the component-wise absolute value.
    #[inline]
    pub fn abs(self) -> Point {
        Point {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }
}

// --- Arithmetic operator impls ---

impl Add for Point {
    type Output = Point;
    #[inline]
    fn add(self, rhs: Point) -> Point {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point {
    type Output = Point;
    #[inline]
    fn sub(self, rhs: Point) -> Point {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul<f64> for Point {
    type Output = Point;
    #[inline]
    fn mul(self, rhs: f64) -> Point {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<Point> for f64 {
    type Output = Point;
    #[inline]
    fn mul(self, rhs: Point) -> Point {
        rhs * self
    }
}

impl Div<f64> for Point {
    type Output = Point;
    #[inline]
    fn div(self, rhs: f64) -> Point {
        Point {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl Neg for Point {
    type Output = Point;
    #[inline]
    fn neg(self) -> Point {
        Point {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Index<Axis> for Point {
    type Output = f64;
    #[inline]
    fn index(&self, axis: Axis) -> &f64 {
        match axis {
            Axis::X => &self.x,
            Axis::Y => &self.y,
        }
    }
}

impl From<(f64, f64)> for Point {
    fn from((x, y): (f64, f64)) -> Self {
        Point { x, y }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn point_is_send_sync() {
        is_send_sync::<Point>();
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-14
    }

    fn points_approx_eq(a: Point, b: Point) -> bool {
        approx_eq(a.x, b.x) && approx_eq(a.y, b.y)
    }

    #[test]
    fn test_ortho() {
        let cases = [
            (Point::new(0.0, 0.0), Point::new(0.0, 0.0)),
            (Point::new(0.0, 1.0), Point::new(-1.0, 0.0)),
            (Point::new(1.0, 1.0), Point::new(-1.0, 1.0)),
            (Point::new(-4.0, 7.0), Point::new(-7.0, -4.0)),
        ];
        for (p, want) in &cases {
            assert!(
                points_approx_eq(p.ortho(), *want),
                "{p}.ortho() = {:?}, want {want:?}",
                p.ortho()
            );
        }
    }

    #[test]
    fn test_dot() {
        assert!(approx_eq(
            Point::new(0.0, 0.0).dot(Point::new(0.0, 0.0)),
            0.0
        ));
        assert!(approx_eq(
            Point::new(1.0, 1.0).dot(Point::new(4.0, 3.0)),
            7.0
        ));
        assert!(approx_eq(
            Point::new(-4.0, 7.0).dot(Point::new(1.0, 5.0)),
            31.0
        ));
    }

    #[test]
    fn test_cross() {
        assert!(approx_eq(
            Point::new(1.0, 1.0).cross(Point::new(-1.0, -1.0)),
            0.0
        ));
        assert!(approx_eq(
            Point::new(1.0, 1.0).cross(Point::new(4.0, 3.0)),
            -1.0
        ));
        assert!(approx_eq(
            Point::new(1.0, 5.0).cross(Point::new(-2.0, 3.0)),
            13.0
        ));
    }

    #[test]
    fn test_norm() {
        assert!(approx_eq(Point::new(0.0, 0.0).norm(), 0.0));
        assert!(approx_eq(Point::new(0.0, 1.0).norm(), 1.0));
        assert!(approx_eq(Point::new(3.0, 4.0).norm(), 5.0));
        assert!(approx_eq(Point::new(3.0, -4.0).norm(), 5.0));
        assert!(approx_eq(Point::new(2.0, 2.0).norm(), 2.0 * f64::sqrt(2.0)));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(Point::new(0.0, 0.0).normalize(), Point::new(0.0, 0.0));
        assert!(points_approx_eq(
            Point::new(3.0, 4.0).normalize(),
            Point::new(0.6, 0.8)
        ));
        assert!(points_approx_eq(
            Point::new(3.0, -4.0).normalize(),
            Point::new(0.6, -0.8)
        ));
    }

    #[test]
    fn test_arithmetic() {
        let a = Point::new(1.0, 2.0);
        let b = Point::new(3.0, 4.0);
        assert_eq!(a + b, Point::new(4.0, 6.0));
        assert_eq!(a - b, Point::new(-2.0, -2.0));
        assert_eq!(a * 3.0, Point::new(3.0, 6.0));
        assert_eq!(3.0 * a, Point::new(3.0, 6.0));
        assert_eq!(a / 2.0, Point::new(0.5, 1.0));
        assert_eq!(-a, Point::new(-1.0, -2.0));
    }

    #[test]
    fn test_index() {
        let p = Point::new(1.0, 2.0);
        assert_eq!(p[Axis::X], 1.0);
        assert_eq!(p[Axis::Y], 2.0);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    fn pt(x: f64, y: f64) -> Point {
        Point::new(clamp_finite(x), clamp_finite(y))
    }

    #[quickcheck]
    fn prop_dot_commutative(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
        let a = pt(ax, ay);
        let b = pt(bx, by);
        (a.dot(b) - b.dot(a)).abs() < 1e-10
    }

    #[quickcheck]
    fn prop_ortho_is_orthogonal(x: f64, y: f64) -> bool {
        let p = pt(x, y);
        p.dot(p.ortho()).abs() < 1e-10
    }

    #[quickcheck]
    fn prop_normalize_unit(x: f64, y: f64) -> bool {
        let p = pt(x, y);
        let n = p.normalize();
        if p.norm2() == 0.0 {
            n == Point::default()
        } else {
            (n.norm() - 1.0).abs() < 1e-14
        }
    }

    #[quickcheck]
    fn prop_add_commutative(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
        let a = pt(ax, ay);
        let b = pt(bx, by);
        (a + b) == (b + a)
    }

    #[quickcheck]
    fn prop_cross_antisymmetric(ax: f64, ay: f64, bx: f64, by: f64) -> bool {
        let a = pt(ax, ay);
        let b = pt(bx, by);
        let tol = 1e-6 * a.norm() * b.norm();
        (a.cross(b) + b.cross(a)).abs() <= tol + 1e-30
    }

    #[quickcheck]
    fn prop_norm2_equals_dot_self(x: f64, y: f64) -> bool {
        let p = pt(x, y);
        (p.norm2() - p.dot(p)).abs() < 1e-10
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x: i32, y: i32) -> bool {
        let p = Point::new(f64::from(x), f64::from(y));
        let json = serde_json::to_string(&p).unwrap();
        let back: Point = serde_json::from_str(&json).unwrap();
        back == p
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_axis_roundtrip() {
        for a in [Axis::X, Axis::Y] {
            let json = serde_json::to_string(&a).unwrap();
            let back: Axis = serde_json::from_str(&json).unwrap();
            assert_eq!(a, back);
        }
    }
}
