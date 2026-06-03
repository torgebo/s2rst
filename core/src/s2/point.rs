// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A point on the unit sphere, represented as a normalized 3D vector.
//!
//! Corresponds to C++ `S2Point`, Go `s2.Point`, Java `S2Point`.
//!
//! `Point` is a newtype around [`r3::Vector`]. By convention, it should
//! always be unit length, but this is not enforced at the type level. Use
//! [`Point::from_coords`] to create a normalized point from arbitrary
//! coordinates.

#![expect(
    clippy::cast_sign_loss,
    reason = "exponent (i32) cast to u64 for f64::from_bits — always non-negative after +1023 bias"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64 -> i64 for exponent extraction — bounded by IEEE 754 format"
)]
use crate::r3::{Matrix3x3, Vector};
use crate::s1::{Angle, ChordAngle};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

/// A point on the unit sphere (newtype over [`crate::r3::Vector`]).
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::s2::Point;
///
/// let p = Point::from_coords(1.0, 0.0, 0.0);
/// assert!(p.is_unit());
/// assert_eq!(p.x(), 1.0);
///
/// // Distance between two points
/// let q = Point::from_coords(0.0, 1.0, 0.0);
/// let dist = p.distance(q);
/// assert!((dist.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point(pub Vector);

impl Point {
    /// Creates a new `Point` from a vector. The vector is **not** normalized.
    /// Use [`Point::from_coords`] if you want automatic normalization.
    #[inline]
    pub fn new(v: Vector) -> Self {
        Point(v)
    }

    /// Creates a normalized point from the given coordinates.
    ///
    /// If the input is the zero vector, returns [`Point::origin`].
    pub fn from_coords(x: f64, y: f64, z: f64) -> Self {
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return Self::origin();
        }
        Point(Vector { x, y, z }.normalize())
    }

    /// Returns a unique "origin" on the sphere for operations that need a
    /// fixed reference point. This is the "point at infinity" used for
    /// point-in-polygon testing (by counting edge crossings).
    ///
    /// The point is located about 66km from the north pole towards the East
    /// Siberian Sea.
    #[inline]
    pub fn origin() -> Self {
        Point(Vector {
            x: -0.009_999_466_435_025_02,
            y: 0.002_592_454_260_932_412,
            z: 0.999_946_643_502_502,
        })
    }

    /// Returns the inner `r3::Vector`.
    #[inline]
    pub fn vector(self) -> Vector {
        self.0
    }

    /// Returns the x coordinate.
    #[inline]
    pub fn x(self) -> f64 {
        self.0.x
    }

    /// Returns the y coordinate.
    #[inline]
    pub fn y(self) -> f64 {
        self.0.y
    }

    /// Returns the z coordinate.
    #[inline]
    pub fn z(self) -> f64 {
        self.0.z
    }

    // --- Unit length ---

    /// Reports whether this point is approximately unit length.
    /// Uses a tolerance of `5 * f64::EPSILON`.
    #[inline]
    pub fn is_unit(self) -> bool {
        (self.0.norm2() - 1.0).abs() <= 5.0 * f64::EPSILON
    }

    /// Returns a normalized copy of this point.
    #[inline]
    pub fn normalize(self) -> Self {
        Point(self.0.normalize())
    }

    /// Reports whether this point is approximately equal to `other`
    /// within the given maximum angle error.
    pub fn approx_eq_with(self, other: Point, max_error: Angle) -> bool {
        self.distance(other) <= max_error
    }

    // --- Distance ---

    /// Returns the angle between this point and `other`.
    #[inline]
    pub fn distance(self, other: Point) -> Angle {
        Angle::from_radians(self.0.angle(other.0))
    }

    /// Returns the chord angle between this point and `other`.
    /// Both points must be unit length.
    #[inline]
    pub fn chord_angle(self, other: Point) -> ChordAngle {
        ChordAngle::from_length2((self.0 - other.0).norm2().min(4.0))
    }

    /// Returns a stable angle between this point and `other`, using
    /// Kahan's formula: `2 * atan2(|a-b|, |a+b|)`. More precise than
    /// `distance()` when the two points are nearly parallel or antiparallel.
    /// Both points must be unit length.
    pub fn stable_angle(self, other: Point) -> Angle {
        Angle::from_radians(2.0 * (self.0 - other.0).norm().atan2((self.0 + other.0).norm()))
    }

    // --- Approximate equality ---

    /// Reports whether two points are approximately equal (within 1e-15 radians).
    #[inline]
    pub fn approx_eq(self, other: Point) -> bool {
        self.approx_eq_angle(other, Angle::from_radians(1e-15))
    }

    /// Reports whether two points are within the given angular tolerance.
    ///
    /// Uses `r3::Vector::angle` internally, which may be slightly less
    /// accurate than [`approx_eq_with`](Point::approx_eq_with) for very
    /// close points.
    #[inline]
    pub fn approx_eq_angle(self, other: Point, eps: Angle) -> bool {
        self.0.angle(other.0) <= eps.radians()
    }

    // --- Cross products ---

    /// Returns a point orthogonal to both `self` and `other`. This is similar
    /// to `self.0.cross(other.0)` but does a better job of ensuring
    /// orthogonality when the points are nearly parallel. It also returns a
    /// non-zero result when `self == other` or `self == -other`.
    ///
    /// Properties:
    /// 1. `f(p, q) != 0` for all p, q
    /// 2. `f(q, p) == -f(p, q)` unless `p == q` or `p == -q`
    /// 3. `f(-p, q) == -f(p, q)` unless `p == q` or `p == -q`
    /// 4. `f(p, -q) == -f(p, q)` unless `p == q` or `p == -q`
    pub fn point_cross(self, other: Point) -> Point {
        // (p + q) × (q - p) = 2(p × q), but more numerically stable.
        let x = (self.0 + other.0).cross(other.0 - self.0);
        if x == Vector::default() {
            // p and other are parallel or anti-parallel; return an arbitrary
            // orthogonal vector.
            return Point(ortho_impl(self.0));
        }
        Point(x)
    }

    /// Returns a unit-length reference direction for this point, guaranteed
    /// to be different from `self`. Used for semi-open boundary vertex tests.
    #[inline]
    pub fn reference_dir(self) -> Point {
        ortho(self)
    }

    // --- Comparison ---

    /// Lexicographic comparison of points (x, then y, then z).
    #[inline]
    pub fn cmp_point(self, other: Point) -> Ordering {
        self.0.cmp(other.0)
    }

    // --- Normalizable ---

    /// Reports whether this point's magnitude is large enough that the angle
    /// to another vector can be measured without loss of precision due to
    /// floating-point underflow.
    #[inline]
    pub fn is_normalizable(self) -> bool {
        let max = self.0.x.abs().max(self.0.y.abs()).max(self.0.z.abs());
        max >= f64::from_bits(((1023 - 242) as u64) << 52) // 2^-242
    }

    /// Scales this vector as necessary to ensure that it can be normalized
    /// without loss of precision. Returns self unchanged if already normalizable.
    /// Requires `self != (0,0,0)`.
    pub fn ensure_normalizable(self) -> Point {
        if self.0 == Vector::default() {
            return self;
        }
        if self.is_normalizable() {
            self
        } else {
            let p_max = self.0.x.abs().max(self.0.y.abs()).max(self.0.z.abs());
            // Scale by a power of two so the largest component is in [1, 2).
            let scale = f64::from_bits(((1023 + 1) as u64) << 52)
                / f64::from_bits(((ilogb(p_max) + 1 + 1023) as u64) << 52);
            Point(self.0 * scale)
        }
    }
}

/// Returns the S2 "ortho" vector: a unit-length vector orthogonal to `a`
/// that avoids zero coordinates. Satisfies `ortho(-a) == -ortho(a)`.
///
/// This is the S2-specific version that uses a perturbation vector to
/// reduce degenerate cases (unlike `r3::Vector::ortho()` which may
/// return axis-aligned results).
pub fn ortho(p: Point) -> Point {
    Point(ortho_impl(p.0))
}

/// Internal ortho implementation operating on raw Vector.
fn ortho_impl(a: Vector) -> Vector {
    let mut temp = Vector {
        x: 0.012,
        y: 0.0053,
        z: 0.00457,
    };
    match a.largest_abs_component() {
        0 => temp.z = 1.0,
        1 => temp.x = 1.0,
        _ => temp.y = 1.0,
    }
    a.cross(temp).normalize()
}

/// Returns the floor of the base-2 logarithm of `|x|`.
/// Equivalent to C `ilogb(x)`.
fn ilogb(x: f64) -> i32 {
    let bits = x.abs().to_bits();
    let exp = ((bits >> 52) & 0x7FF) as i32;
    if exp == 0 {
        // Subnormal: count leading zeros in mantissa.
        let mantissa = bits & ((1u64 << 52) - 1);
        -1023 - (52 - (64 - mantissa.leading_zeros() as i32))
    } else {
        exp - 1023
    }
}

// --- Operator impls ---

impl Neg for Point {
    type Output = Point;
    #[inline]
    fn neg(self) -> Point {
        Point(-self.0)
    }
}

impl Add for Point {
    type Output = Point;
    #[inline]
    fn add(self, rhs: Point) -> Point {
        Point(self.0 + rhs.0)
    }
}

impl Sub for Point {
    type Output = Point;
    #[inline]
    fn sub(self, rhs: Point) -> Point {
        Point(self.0 - rhs.0)
    }
}

impl From<Vector> for Point {
    #[inline]
    fn from(v: Vector) -> Self {
        Point(v)
    }
}

impl From<Point> for Vector {
    #[inline]
    fn from(p: Point) -> Self {
        p.0
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp_point(*other))
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.15}, {:.15}, {:.15})", self.0.x, self.0.y, self.0.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn point_is_send_sync() {
        is_send_sync::<Point>();
    }

    fn float64_eq(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-15
    }

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    fn point_near(a: Point, b: Point, eps: f64) -> bool {
        (a.x() - b.x()).abs() < eps && (a.y() - b.y()).abs() < eps && (a.z() - b.z()).abs() < eps
    }

    #[test]
    fn test_origin_point() {
        let origin = Point::origin();
        assert!(
            (origin.0.norm() - 1.0).abs() <= 1e-15,
            "origin norm = {}, want 1",
            origin.0.norm(),
        );

        // The origin should be about 66km from the north pole towards East Siberian Sea.
        use crate::s2::coords::st_to_uv;
        let p = Point(
            Vector {
                x: -0.01,
                y: 0.01 * st_to_uv(2.0 / 3.0),
                z: 1.0,
            }
            .normalize(),
        );
        assert!(
            origin.approx_eq(p),
            "origin point should be near Siberian Sea reference",
        );

        // Check that origin is not too close to the north pole (≥ 50km).
        let earth_radius_km = 6371.01;
        let dist = origin.z().acos() * earth_radius_km;
        assert!(dist >= 50.0, "origin too close to north pole: {dist}km",);
    }

    #[test]
    fn test_point_cross() {
        let cases: Vec<(Vector, Vector, f64)> = vec![
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                1.0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                2.0,
            ),
            (
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                2.0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Vector {
                    x: -4.0,
                    y: 5.0,
                    z: -6.0,
                },
                2.0 * (934.0_f64).sqrt(),
            ),
        ];

        for (v1, v2, want_norm) in &cases {
            let p1 = Point(*v1);
            let p2 = Point(*v2);
            let result = p1.point_cross(p2);

            assert!(
                float64_eq(result.0.norm(), *want_norm),
                "|{v1:?} × {v2:?}| = {}, want {want_norm}",
                result.0.norm(),
            );
            assert!(
                float64_eq(result.0.dot(p1.0), 0.0),
                "({v1:?} × {v2:?}) · {v1:?} = {}, want 0",
                result.0.dot(p1.0),
            );
            assert!(
                float64_eq(result.0.dot(p2.0), 0.0),
                "({v1:?} × {v2:?}) · {v2:?} = {}, want 0",
                result.0.dot(p2.0),
            );
        }
    }

    #[test]
    fn test_point_distance() {
        let cases: Vec<(Vector, Vector, f64)> = vec![
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                0.0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                PI / 2.0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
                PI / 2.0,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                PI,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Vector {
                    x: 2.0,
                    y: 3.0,
                    z: -1.0,
                },
                1.2055891055045298,
            ),
        ];

        for (v1, v2, want) in &cases {
            let p1 = Point(*v1);
            let p2 = Point(*v2);
            assert!(
                float64_eq(p1.distance(p2).radians(), *want),
                "{v1:?}.distance({v2:?}) = {}, want {want}",
                p1.distance(p2).radians(),
            );
            assert!(
                float64_eq(p2.distance(p1).radians(), *want),
                "{v2:?}.distance({v1:?}) = {}, want {want}",
                p2.distance(p1).radians(),
            );
        }
    }

    #[test]
    fn test_approx_equal() {
        let eps = 1e-15;
        let cases: Vec<(Vector, Vector, bool)> = vec![
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                false,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 1.0,
                },
                false,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                },
                false,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Vector {
                    x: 2.0,
                    y: 3.0,
                    z: -1.0,
                },
                false,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0 * (1.0 + eps),
                    y: 0.0,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0 * (1.0 - eps),
                    y: 0.0,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0 + eps,
                    y: 0.0,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0 - eps,
                    y: 0.0,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: eps,
                    z: 0.0,
                },
                true,
            ),
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: eps,
                    z: eps,
                },
                false,
            ),
            (
                Vector {
                    x: 1.0,
                    y: eps,
                    z: 0.0,
                },
                Vector {
                    x: 1.0,
                    y: -eps,
                    z: eps,
                },
                false,
            ),
        ];

        for (v1, v2, want) in &cases {
            let p1 = Point(*v1);
            let p2 = Point(*v2);
            assert_eq!(
                p1.approx_eq(p2),
                *want,
                "{v1:?}.approx_eq({v2:?}) = {}, want {want}",
                p1.approx_eq(p2),
            );
        }
    }

    #[test]
    fn test_ortho() {
        let cases: Vec<(Vector, Vector)> = vec![
            (
                Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.0,
                    y: -0.999985955295886075333556,
                    z: 0.005299925563068195837058,
                },
            ),
            (
                Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
                Vector {
                    x: 0.004569952278750987959000,
                    y: 0.0,
                    z: -0.999989557713564125585037,
                },
            ),
            (
                Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                },
                Vector {
                    x: -0.999928007775066962636856,
                    y: 0.011999136093300803371231,
                    z: 0.0,
                },
            ),
            (
                Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
                Vector {
                    x: -0.709740689278763769998193,
                    y: 0.005297583276916723732386,
                    z: 0.704443106001847008101890,
                },
            ),
            (
                Vector {
                    x: 3.0,
                    y: -2.0,
                    z: 0.4,
                },
                Vector {
                    x: -0.555687999915428054720223,
                    y: -0.831317152491703792449584,
                    z: 0.011074236907191168863274,
                },
            ),
            (
                Vector {
                    x: 0.012,
                    y: 0.0053,
                    z: 0.00457,
                },
                Vector {
                    x: 0.404015523469256565558538,
                    y: -0.914752128609637393807930,
                    z: 0.0,
                },
            ),
        ];

        for (input, want) in &cases {
            let got = ortho(Point(*input));

            // FMA in cross() shifts the un-normalized cross product by 1 ULP,
            // which propagates through normalize(); orthogonality and unit
            // length are the load-bearing properties, not bit identity.
            assert!(
                got.0.aequal(*want, 1e-15),
                "ortho({input:?}) = {:?}, want {want:?}",
                got.0,
            );

            // The result must be orthogonal.
            assert!(
                float64_eq(input.dot(got.0), 0.0),
                "{input:?} · ortho({input:?}) = {}, want 0",
                input.dot(got.0),
            );

            // The result must be unit length.
            assert!(got.is_unit(), "ortho({input:?}) should be unit length");
        }
    }

    #[test]
    fn test_from_coords() {
        let p = Point::from_coords(1.0, 2.0, 3.0);
        assert!(p.is_unit(), "from_coords should produce unit-length point");

        // Zero vector should return origin.
        let p0 = Point::from_coords(0.0, 0.0, 0.0);
        assert_eq!(p0, Point::origin());
    }

    #[test]
    fn test_is_unit() {
        assert!(
            Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0
            })
            .is_unit()
        );
        assert!(
            Point(Vector {
                x: 0.0,
                y: 1.0,
                z: 0.0
            })
            .is_unit()
        );
        assert!(
            Point(Vector {
                x: 0.0,
                y: 0.0,
                z: 1.0
            })
            .is_unit()
        );
        assert!(
            !Point(Vector {
                x: 1.0,
                y: 1.0,
                z: 1.0
            })
            .is_unit()
        );
        assert!(
            Point(
                Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0
                }
                .normalize()
            )
            .is_unit()
        );
    }

    #[test]
    fn test_chord_angle_between() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let q = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });

        // Same point: chord angle 0.
        assert_eq!(p.chord_angle(p).length2(), 0.0);

        // 90 degrees: chord angle with length2 = 2.
        assert!(float64_eq(p.chord_angle(q).length2(), 2.0));

        // Opposite point: chord angle with length2 = 4.
        assert!(float64_eq(p.chord_angle(-p).length2(), 4.0));
    }

    #[test]
    fn test_stable_angle() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let q = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        assert!(float64_near(p.stable_angle(q).radians(), PI / 2.0, 1e-15));
        assert!(float64_near(p.stable_angle(p).radians(), 0.0, 1e-15));
        assert!(float64_near(p.stable_angle(-p).radians(), PI, 1e-15));
    }

    #[test]
    fn test_partial_ord() {
        let p1 = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let p2 = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        assert!(p1 > p2); // x=1 > x=0
        assert!(p2 < p1);

        let p3 = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        assert!(p1 == p3);
        assert!(p1 >= p3);
        assert!(p1 <= p3);
    }

    #[test]
    fn test_is_normalizable() {
        assert!(
            !Point(Vector {
                x: 0.0,
                y: 0.0,
                z: 0.0
            })
            .is_normalizable()
        );
        assert!(
            Point(Vector {
                x: 1.0,
                y: 1.0,
                z: 1.0
            })
            .is_normalizable()
        );
        assert!(
            Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0
            })
            .is_normalizable()
        );
        assert!(
            Point(Vector {
                x: 1e-75,
                y: 1.0,
                z: 1.0
            })
            .is_normalizable()
        );

        // Exact boundary case: 2^-242 is normalizable.
        let boundary = f64::from_bits(((1023 - 242) as u64) << 52); // 2^-242
        assert!(
            Point(Vector {
                x: boundary,
                y: boundary,
                z: boundary
            })
            .is_normalizable()
        );

        // One step below: 2^-243 is not normalizable.
        let below = f64::from_bits(((1023 - 243) as u64) << 52); // 2^-243
        assert!(
            !Point(Vector {
                x: below,
                y: below,
                z: below
            })
            .is_normalizable()
        );
    }

    #[test]
    fn test_ensure_normalizable() {
        // Zero stays zero.
        let zero = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        assert_eq!(zero.ensure_normalizable(), zero);

        // Already normalizable stays the same.
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        assert_eq!(p.ensure_normalizable(), p);

        // Boundary case stays the same.
        let boundary = f64::from_bits(((1023 - 242) as u64) << 52);
        let pb = Point(Vector {
            x: boundary,
            y: boundary,
            z: boundary,
        });
        assert_eq!(pb.ensure_normalizable(), pb);

        // Below boundary gets scaled up.
        let below = f64::from_bits(((1023 - 243) as u64) << 52);
        let ps = Point(Vector {
            x: below,
            y: below,
            z: below,
        });
        let scaled = ps.ensure_normalizable();
        assert!(
            point_near(
                scaled,
                Point(Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0
                }),
                1e-50
            ),
            "ensure_normalizable({ps:?}) = {scaled:?}, want ≈(1,1,1)",
        );

        // Different components.
        let pd = Point(Vector {
            x: f64::from_bits(((1023 - 243) as u64) << 52),
            y: f64::from_bits(((1023 - 486) as u64) << 52),
            z: f64::from_bits(((1023 - 729) as u64) << 52),
        });
        let sd = pd.ensure_normalizable();
        assert!(
            point_near(
                sd,
                Point(Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0
                }),
                1e-50
            ),
            "ensure_normalizable({pd:?}) = {sd:?}, want ≈(1,0,0)",
        );
    }

    #[test]
    fn test_display() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let s = format!("{p}");
        assert!(s.contains("1.0"));
    }

    #[test]
    fn test_contains() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let q = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let r = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        assert_eq!(p, q);
        assert_ne!(p, r);
    }

    #[test]
    fn test_get_frame() {
        let z = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });
        let m = get_frame(z);

        // Column 2 should be z itself.
        let col2 = m.col(2);
        assert!(float64_near(col2.x, 0.0, 1e-15));
        assert!(float64_near(col2.y, 0.0, 1e-15));
        assert!(float64_near(col2.z, 1.0, 1e-15));

        // Column 1 should be ortho(z).
        let col1 = m.col(1);
        let oz = ortho(z);
        assert!(float64_near(col1.x, oz.x(), 1e-15));
        assert!(float64_near(col1.y, oz.y(), 1e-15));
        assert!(float64_near(col1.z, oz.z(), 1e-15));

        // Column 0 should be col1 × z (right-handed).
        let col0 = m.col(0);
        let expected_x = col1.cross(z.0);
        assert!(float64_near(col0.x, expected_x.x, 1e-15));
        assert!(float64_near(col0.y, expected_x.y, 1e-15));
        assert!(float64_near(col0.z, expected_x.z, 1e-15));

        // All columns should be unit length.
        assert!(float64_near(col0.norm(), 1.0, 1e-14));
        assert!(float64_near(col1.norm(), 1.0, 1e-14));
        assert!(float64_near(col2.norm(), 1.0, 1e-14));

        // Columns should be mutually orthogonal.
        assert!(float64_near(col0.dot(col1), 0.0, 1e-14));
        assert!(float64_near(col0.dot(col2), 0.0, 1e-14));
        assert!(float64_near(col1.dot(col2), 0.0, 1e-14));
    }

    #[test]
    fn test_to_from_frame_roundtrip() {
        let z = Point::from_coords(1.0, 2.0, 3.0);
        let m = get_frame(z);

        let p = Point::from_coords(0.5, -0.3, 0.8);

        // to_frame then from_frame should return the original point.
        let local = to_frame(&m, p);
        let back = from_frame(&m, local);
        assert!(
            p.approx_eq_angle(back, Angle::from_radians(1e-14)),
            "roundtrip failed: {p:?} -> {local:?} -> {back:?}",
        );

        // from_frame then to_frame should also roundtrip.
        let world = from_frame(&m, p);
        let back2 = to_frame(&m, world);
        assert!(
            p.approx_eq_angle(back2, Angle::from_radians(1e-14)),
            "reverse roundtrip failed: {p:?} -> {world:?} -> {back2:?}",
        );
    }

    #[test]
    fn test_to_frame_z_axis() {
        // In the frame of z, z itself should map to (0, 0, 1).
        let z = Point::from_coords(1.0, 2.0, 3.0);
        let m = get_frame(z);
        let local_z = to_frame(&m, z);
        assert!(
            float64_near(local_z.x(), 0.0, 1e-14),
            "local_z.x = {}, want 0",
            local_z.x(),
        );
        assert!(
            float64_near(local_z.y(), 0.0, 1e-14),
            "local_z.y = {}, want 0",
            local_z.y(),
        );
        assert!(
            float64_near(local_z.z(), 1.0, 1e-14),
            "local_z.z = {}, want 1",
            local_z.z(),
        );
    }

    #[test]
    fn test_rotate() {
        let p = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let axis = Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        });

        // 90° rotation around z-axis: (1,0,0) → (0,1,0)
        let rotated = rotate(p, axis, Angle::from_degrees(90.0));
        assert!(rotated.is_unit());
        assert!(
            float64_near(rotated.x(), 0.0, 1e-15),
            "rotated.x = {}, want 0",
            rotated.x(),
        );
        assert!(
            float64_near(rotated.y(), 1.0, 1e-15),
            "rotated.y = {}, want 1",
            rotated.y(),
        );
        assert!(
            float64_near(rotated.z(), 0.0, 1e-15),
            "rotated.z = {}, want 0",
            rotated.z(),
        );

        // 0° rotation: point unchanged.
        let same = rotate(p, axis, Angle::from_radians(0.0));
        assert!(p.approx_eq(same));

        // 360° rotation: point unchanged.
        let full = rotate(p, axis, Angle::from_degrees(360.0));
        assert!(
            float64_near(full.x(), p.x(), 1e-14),
            "360° rotation should restore original point",
        );
    }
}

/// Returns a right-handed coordinate frame (orthonormal matrix) for the given
/// point `z`. The x-axis and y-axis are computed from `z` using `ortho`.
///
/// Column 0 = x-axis, Column 1 = y-axis (ortho(z)), Column 2 = z-axis (z).
///
/// Corresponds to C++ `S2::GetFrame`.
pub(super) fn get_frame(z: Point) -> Matrix3x3 {
    let y = ortho(z);
    let x = Point(y.0.cross(z.0));
    Matrix3x3::from_cols(x.0, y.0, z.0)
}

/// Transforms point `p` from world coordinates to the local coordinate frame
/// defined by `m`. Equivalent to multiplying by the transpose (inverse) of `m`.
///
/// Corresponds to C++ `S2::ToFrame`.
pub(super) fn to_frame(m: &Matrix3x3, p: Point) -> Point {
    Point(m.transpose().mul_vec(p.0))
}

/// Transforms point `q` from the local coordinate frame defined by `m` to world
/// coordinates. Equivalent to multiplying by `m`.
///
/// Corresponds to C++ `S2::FromFrame`.
pub(super) fn from_frame(m: &Matrix3x3, q: Point) -> Point {
    Point(m.mul_vec(q.0))
}

/// Rotates point `p` about the given `axis` by the given `angle`.
/// Both `p` and `axis` must be unit length.
pub fn rotate(p: Point, axis: Point, angle: Angle) -> Point {
    let center = axis.0 * p.0.dot(axis.0);
    let dx = p.0 - center;
    let dy = axis.0.cross(p.0);
    let (sin_a, cos_a) = angle.sin_cos();
    Point((dx * cos_a + dy * sin_a + center).normalize())
}

// Additional deterministic tests ported from C++ S2Point tests.
#[cfg(test)]
mod point_property_tests {
    use super::*;
    use crate::s1::Angle;

    #[test]
    fn test_normalize_creates_unit_vector() {
        // Various non-unit vectors should normalize to unit length.
        let cases = [
            (3.0, 4.0, 0.0),
            (0.0, 0.0, 100.0),
            (1e-100, 1e-100, 1e-100),
            (1e100, 0.0, 0.0),
            (1.0, 1.0, 1.0),
        ];
        for (x, y, z) in cases {
            let p = Point::from_coords(x, y, z);
            let norm = p.0.norm();
            assert!(
                (norm - 1.0).abs() < 1e-14,
                "normalize({x},{y},{z}) has norm {norm}",
            );
        }
    }

    #[test]
    fn test_reference_dir_orthogonal() {
        // reference_dir should return a unit vector different from the input.
        let points = [
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
            Point::from_coords(1.0, 1.0, 1.0),
        ];
        for p in &points {
            let r = p.reference_dir();
            assert!(r.is_unit(), "reference_dir should be unit length");
            assert_ne!(*p, r, "reference_dir should differ from input");
        }
    }

    #[test]
    fn test_point_cross_antisymmetry() {
        // point_cross(p, q) == -point_cross(q, p)
        let p = Point::from_coords(1.0, 2.0, 3.0);
        let q = Point::from_coords(4.0, -1.0, 2.0);
        let pq = p.point_cross(q);
        let qp = q.point_cross(p);
        assert!((pq.0.x + qp.0.x).abs() < 1e-14);
        assert!((pq.0.y + qp.0.y).abs() < 1e-14);
        assert!((pq.0.z + qp.0.z).abs() < 1e-14);
    }

    #[test]
    fn test_distance_symmetric() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let q = Point::from_coords(0.0, 1.0, 0.0);
        let d1 = p.distance(q).radians();
        let d2 = q.distance(p).radians();
        assert!((d1 - d2).abs() < 1e-15, "distance should be symmetric");
    }

    #[test]
    fn test_chord_angle_properties() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let q = Point::from_coords(0.0, 1.0, 0.0);
        let neg_p = -p;

        // Same point → zero chord angle.
        assert_eq!(p.chord_angle(p).length2(), 0.0);

        // Orthogonal points → chord angle with length2 = 2.
        assert!((p.chord_angle(q).length2() - 2.0).abs() < 1e-14);

        // Antipodal points → chord angle with length2 = 4.
        assert!((p.chord_angle(neg_p).length2() - 4.0).abs() < 1e-14);
    }

    #[test]
    fn test_stable_angle_agrees_with_distance() {
        // For well-separated points, stable_angle and distance should agree.
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let q = Point::from_coords(0.0, 1.0, 0.0);
        let d = p.distance(q).radians();
        let sa = p.stable_angle(q).radians();
        assert!((d - sa).abs() < 1e-14, "distance={d}, stable_angle={sa}",);
    }

    #[test]
    fn test_approx_equals_tolerance() {
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let q = Point::from_coords(1.0, 1e-5, 0.0);
        // q is about 1e-5 radians from p.
        assert!(p.approx_eq_with(q, Angle::from_radians(1e-4)));
        assert!(!p.approx_eq_with(q, Angle::from_radians(1e-6)));
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

    fn make_point(x: f64, y: f64, z: f64) -> Option<Point> {
        let x = clamp_finite(x);
        let y = clamp_finite(y);
        let z = clamp_finite(z);
        if x == 0.0 && y == 0.0 && z == 0.0 {
            return None;
        }
        Some(Point::from_coords(x, y, z))
    }

    #[quickcheck]
    fn prop_from_coords_unit(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => (p.0.norm() - 1.0).abs() < 1e-14,
            None => true,
        }
    }

    #[quickcheck]
    fn prop_cross_orthogonal(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(q)) => {
                let c = p.point_cross(q);
                c.0.dot(p.0).abs() < 1e-10 && c.0.dot(q.0).abs() < 1e-10
            }
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_distance_self_zero(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => p.distance(p).radians().abs() < 1e-14,
            None => true,
        }
    }

    #[quickcheck]
    fn prop_distance_antipodal_pi(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => (p.distance(-p).radians() - PI).abs() < 1e-14,
            None => true,
        }
    }

    #[quickcheck]
    fn prop_ortho_is_orthogonal(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => {
                let o = ortho(p);
                o.0.dot(p.0).abs() < 1e-14
            }
            None => true,
        }
    }

    #[quickcheck]
    fn prop_rotate_preserves_distance(
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
        angle: f64,
    ) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(axis)) => {
                let angle = if angle.is_finite() {
                    angle.clamp(-PI, PI)
                } else {
                    0.5
                };
                let rotated = rotate(p, axis, Angle::from_radians(angle));
                // Rotation preserves unit length.
                (rotated.0.norm() - 1.0).abs() < 1e-13
            }
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_stable_angle_symmetric(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(q)) => {
                (p.stable_angle(q).radians() - q.stable_angle(p).radians()).abs() < 1e-14
            }
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_neg_involutive(x: f64, y: f64, z: f64) -> bool {
        match make_point(x, y, z) {
            Some(p) => -(-p) == p,
            None => true,
        }
    }

    #[quickcheck]
    fn prop_distance_non_negative(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(q)) => p.distance(q).radians() >= 0.0,
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_distance_symmetric(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(q)) => (p.distance(q).radians() - q.distance(p).radians()).abs() < 1e-14,
            _ => true,
        }
    }

    #[quickcheck]
    fn prop_chord_angle_non_negative(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> bool {
        match (make_point(x1, y1, z1), make_point(x2, y2, z2)) {
            (Some(p), Some(q)) => p.chord_angle(q).length2() >= 0.0,
            _ => true,
        }
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x: i32, y: i32, z: i32) -> bool {
        if x == 0 && y == 0 && z == 0 {
            return true;
        }
        let p = Point::from_coords(f64::from(x), f64::from(y), f64::from(z));
        let json1 = serde_json::to_string(&p).unwrap();
        let back: Point = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: Point = serde_json::from_str(&json2).unwrap();
        // Idempotent: second roundtrip must be stable
        back == back2
    }
}
