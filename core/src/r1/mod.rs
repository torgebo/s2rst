// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library — a derivative work of the
// upstream Apache-2.0 implementations (Copyright Google Inc.):
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java
// See LICENSE.

//! One-dimensional geometry on the real line (ℝ¹).
//!
//! This module provides [`Interval`], a closed interval `[lo, hi]` on the real
//! number line. Intervals support containment tests, intersection, union,
//! expansion, and distance computation. An empty interval is represented by
//! `lo > hi`.
//!
//! `r1::Interval` is used throughout the library to represent latitude ranges
//! and other one-dimensional bounds.

mod interval;

pub use interval::{Endpoint, Interval};
