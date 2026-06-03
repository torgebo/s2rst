// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! One-dimensional geometry on the unit circle (S¹).
//!
//! This module provides types for representing angles and angular intervals:
//!
//! - [`Angle`] — a one-dimensional angle stored as `f64` radians. Supports
//!   construction from degrees, radians, and E5/E6/E7 integer encodings, as
//!   well as standard arithmetic operations.
//! - [`ChordAngle`] — an angle stored as the squared chord length through the
//!   sphere interior (`2 sin²(θ/2)`). This representation is significantly
//!   faster for computing and comparing distances since it avoids
//!   trigonometric functions. It can only represent angles in `[0, π]`.
//! - [`Interval`] — a closed interval on the unit circle. Unlike
//!   [`r1::Interval`](crate::r1::Interval), a circular interval can "wrap
//!   around" — its lower bound may be greater than its upper bound,
//!   representing an arc that crosses the point `(-1, 0)` (angle ±π).
//!
//! Use `Angle` for general angle manipulation and `ChordAngle` in
//! performance-sensitive distance computations. The two types convert freely
//! via [`ChordAngle::from_angle`] and [`ChordAngle::to_angle`].

mod angle;
mod chord_angle;
mod interval;

pub use angle::Angle;
pub use chord_angle::ChordAngle;
pub use interval::Interval;
