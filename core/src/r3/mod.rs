// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! Three-dimensional Euclidean geometry and exact arithmetic (ℝ³).
//!
//! This module provides the foundation for all point-based geometry in the
//! library:
//!
//! - [`Vector`] — a 3D vector with `f64` components. This is the underlying
//!   representation for [`s2::Point`](crate::s2::Point) (a unit-length vector
//!   on the sphere).
//! - [`Matrix3x3`] — a 3×3 matrix for coordinate frame transformations.
//! - [`PreciseVector`] — a 3D vector with arbitrary-precision
//!   [`ExactFloat`] components, used by the robust geometric predicates in
//!   [`s2::predicates`](crate::s2::predicates) when `f64` precision is
//!   insufficient.
//! - [`ExactFloat`] — an exact floating-point type (`mantissa × 2^exp` with
//!   a `BigInt` mantissa) supporting addition, subtraction, and
//!   multiplication without rounding. Division and square root are not
//!   provided because they are not needed for exact geometric predicates.

pub mod exact_float;
pub mod matrix;
mod precise_vector;
mod vector;

pub use exact_float::ExactFloat;
pub use matrix::Matrix3x3;
pub use precise_vector::PreciseVector;
pub use vector::{Axis, Vector};
