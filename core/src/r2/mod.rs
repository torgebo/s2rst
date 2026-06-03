// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! Two-dimensional Euclidean geometry (ℝ²).
//!
//! This module provides basic types for working in the Euclidean plane:
//!
//! - [`Point`] — a 2D point (or vector) with `x` and `y` components,
//!   supporting arithmetic, dot product, and cross product.
//! - [`Rect`] — a closed axis-aligned rectangle, stored as a pair of
//!   [`r1::Interval`](crate::r1::Interval)s.
//!
//! These types are primarily used internally to represent cell bounds in
//! `(u, v)` cube-space coordinates during S2 cell operations.

mod point;
mod rect;

pub use point::{Axis, Point};
pub use rect::Rect;
