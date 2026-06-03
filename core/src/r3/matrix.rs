// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A simple 3x3 matrix type for coordinate frame transformations.

use crate::r3::Vector;

/// A 3x3 matrix stored in row-major order.
///
/// Primarily used for coordinate frame transformations (`GetFrame`,
/// `ToFrame`, `FromFrame`).
///
/// Corresponds to C++ `Matrix3x3<double>` (aka `Matrix3x3_d`).
///
/// # Examples
///
/// ```
/// use s2rst::r3::{Matrix3x3, Vector};
///
/// // The identity matrix leaves vectors unchanged.
/// let id = Matrix3x3::identity();
/// let v = Vector::new(1.0, 2.0, 3.0);
/// assert_eq!(id * v, v);
///
/// // Build a matrix from column vectors and multiply.
/// let m = Matrix3x3::from_cols(
///     Vector::new(0.0, 1.0, 0.0),
///     Vector::new(-1.0, 0.0, 0.0),
///     Vector::new(0.0, 0.0, 1.0),
/// );
/// // This rotates 90° around Z: (1,0,0) → (0,1,0).
/// let rotated = m * Vector::new(1.0, 0.0, 0.0);
/// assert!((rotated.x - 0.0).abs() < 1e-15);
/// assert!((rotated.y - 1.0).abs() < 1e-15);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Matrix3x3 {
    m: [[f64; 3]; 3],
}

impl Matrix3x3 {
    /// Creates a matrix from explicit values (row-major order).
    pub fn new(
        m00: f64,
        m01: f64,
        m02: f64,
        m10: f64,
        m11: f64,
        m12: f64,
        m20: f64,
        m21: f64,
        m22: f64,
    ) -> Self {
        Matrix3x3 {
            m: [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]],
        }
    }

    /// Creates a matrix from column vectors.
    pub fn from_cols(c0: Vector, c1: Vector, c2: Vector) -> Self {
        Matrix3x3 {
            m: [[c0.x, c1.x, c2.x], [c0.y, c1.y, c2.y], [c0.z, c1.z, c2.z]],
        }
    }

    /// Returns column `i` as a vector.
    pub fn col(&self, i: usize) -> Vector {
        Vector {
            x: self.m[0][i],
            y: self.m[1][i],
            z: self.m[2][i],
        }
    }

    /// Sets column `i` to the given vector.
    pub fn set_col(&mut self, i: usize, v: Vector) {
        self.m[0][i] = v.x;
        self.m[1][i] = v.y;
        self.m[2][i] = v.z;
    }

    /// Returns row `i` as a vector.
    pub fn row(&self, i: usize) -> Vector {
        Vector {
            x: self.m[i][0],
            y: self.m[i][1],
            z: self.m[i][2],
        }
    }

    /// Returns the transpose of this matrix.
    pub fn transpose(&self) -> Self {
        Matrix3x3 {
            m: [
                [self.m[0][0], self.m[1][0], self.m[2][0]],
                [self.m[0][1], self.m[1][1], self.m[2][1]],
                [self.m[0][2], self.m[1][2], self.m[2][2]],
            ],
        }
    }

    /// Multiplies this matrix by a vector.
    pub fn mul_vec(&self, v: Vector) -> Vector {
        Vector {
            x: self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z,
            y: self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z,
            z: self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z,
        }
    }

    /// Returns the element at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.m[row][col]
    }

    /// Returns the identity matrix.
    pub fn identity() -> Self {
        Matrix3x3 {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }
}

impl std::ops::Mul<Vector> for Matrix3x3 {
    type Output = Vector;
    fn mul(self, v: Vector) -> Vector {
        self.mul_vec(v)
    }
}

impl std::ops::Mul<Vector> for &Matrix3x3 {
    type Output = Vector;
    fn mul(self, v: Vector) -> Vector {
        self.mul_vec(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity() {
        let m = Matrix3x3::identity();
        let v = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let result = m * v;
        assert_eq!(result.x, 1.0);
        assert_eq!(result.y, 2.0);
        assert_eq!(result.z, 3.0);
    }

    #[test]
    fn test_transpose() {
        let m = Matrix3x3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let t = m.transpose();
        assert_eq!(t.get(0, 0), 1.0);
        assert_eq!(t.get(0, 1), 4.0);
        assert_eq!(t.get(0, 2), 7.0);
        assert_eq!(t.get(1, 0), 2.0);
        assert_eq!(t.get(1, 1), 5.0);
    }

    #[test]
    fn test_from_cols() {
        let c0 = Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        };
        let c1 = Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let c2 = Vector {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        };
        let m = Matrix3x3::from_cols(c0, c1, c2);
        assert_eq!(m, Matrix3x3::identity());
    }

    #[test]
    fn test_col_roundtrip() {
        let v = Vector {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let mut m = Matrix3x3::default();
        m.set_col(1, v);
        let out = m.col(1);
        assert_eq!(out.x, 1.0);
        assert_eq!(out.y, 2.0);
        assert_eq!(out.z, 3.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let m = Matrix3x3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let json = serde_json::to_string(&m).unwrap();
        let back: Matrix3x3 = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }
}
