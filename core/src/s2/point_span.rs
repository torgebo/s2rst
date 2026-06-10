// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! A borrowed view of loop vertices with wraparound indexing.
//!
//! Corresponds to C++ `s2point_span.h`. Plain `S2PointSpan` is just `&[Point]`
//! in Rust; [`PointLoopSpan`] is the loop-aware view where two full copies of
//! the vertex array are addressable.

use std::ops::{Deref, Index};

use crate::s2::point::Point;

/// A view over loop vertices where indices in `[n, 2n)` wrap to `[0, n)`.
///
/// Port of C++ `S2PointLoopSpan`: `span[i]` accepts `i` in `[0, 2*len())`,
/// mapping `i >= len()` to `i - len()`. Subtracting once is cheaper than the
/// `i % n` in inner loops, and lets edge code write `span[i + 1]` for the
/// edge leaving vertex `i` without a wrap branch at the call site.
///
/// Dereferences to `&[Point]`, so slice methods (`len`, `iter`, `first`, ...)
/// are available. `span[i]` always uses the wraparound rule; for plain range
/// indexing use the underlying slice via `&span.0[a..b]`.
#[derive(Clone, Copy, Debug)]
pub struct PointLoopSpan<'a>(pub &'a [Point]);

impl<'a> PointLoopSpan<'a> {
    /// Creates a loop view over the given vertices.
    pub fn new(vertices: &'a [Point]) -> Self {
        PointLoopSpan(vertices)
    }

    /// Returns vertex `i`, where `i` may be in `[0, 2*len())`; indices of
    /// `len()` and above wrap around to the start.
    ///
    /// # Panics
    /// In debug builds, panics if `i >= 2 * len()`.
    #[inline]
    pub fn vertex(&self, i: usize) -> Point {
        debug_assert!(i < 2 * self.0.len(), "index {i} out of wraparound range");
        let n = self.0.len();
        self.0[if i < n { i } else { i - n }]
    }
}

impl Index<usize> for PointLoopSpan<'_> {
    type Output = Point;

    #[inline]
    fn index(&self, i: usize) -> &Point {
        debug_assert!(i < 2 * self.0.len(), "index {i} out of wraparound range");
        let n = self.0.len();
        &self.0[if i < n { i } else { i - n }]
    }
}

impl Deref for PointLoopSpan<'_> {
    type Target = [Point];

    fn deref(&self) -> &[Point] {
        self.0
    }
}

impl<'a> From<&'a [Point]> for PointLoopSpan<'a> {
    fn from(vertices: &'a [Point]) -> Self {
        PointLoopSpan(vertices)
    }
}

impl<'a> From<&'a Vec<Point>> for PointLoopSpan<'a> {
    fn from(vertices: &'a Vec<Point>) -> Self {
        PointLoopSpan(vertices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r3::Vector;

    fn pts(n: usize) -> Vec<Point> {
        (0..n)
            .map(|i| Point(Vector::new(i as f64, 1.0, 0.0).normalize()))
            .collect()
    }

    #[test]
    fn test_wraparound_indexing() {
        let v = pts(4);
        let span = PointLoopSpan::new(&v);
        assert_eq!(span.len(), 4);
        for i in 0..4 {
            assert_eq!(span[i], v[i]);
            assert_eq!(span[i + 4], v[i], "index {} must wrap to {}", i + 4, i);
            assert_eq!(span.vertex(i + 4), v[i]);
        }
    }

    #[test]
    fn test_slice_methods_via_deref() {
        let v = pts(3);
        let span = PointLoopSpan::from(&v);
        assert!(!span.is_empty());
        assert_eq!(span.iter().count(), 3);
        assert_eq!(&span.0[1..3], &v[1..3]);
        assert_eq!(span.first(), Some(&v[0]));
    }

    #[test]
    fn test_edge_iteration_pattern() {
        // The intended idiom: span[i + 1] is the destination of edge i,
        // including the closing edge back to vertex 0.
        let v = pts(5);
        let span = PointLoopSpan::new(&v);
        let mut edges = Vec::new();
        for i in 0..span.len() {
            edges.push((span[i], span[i + 1]));
        }
        assert_eq!(edges.len(), 5);
        assert_eq!(edges[4], (v[4], v[0]));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "out of wraparound range")]
    fn test_out_of_range_panics_in_debug() {
        let v = pts(3);
        let span = PointLoopSpan::new(&v);
        let _ = span[6];
    }
}
