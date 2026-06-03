// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Efficient batch edge crossing tester.
//!
//! [`EdgeCrosser`] allows edges to be efficiently tested for intersection
//! with a given fixed edge AB. It is especially efficient when testing
//! against an edge chain connecting vertices v0, v1, v2, ...
//!
//! Corresponds to Go `s2/edge_crosser.go`, C++ `s2edge_crosser.h`.

use crate::s2::Point;
use crate::s2::edge_crossings::{Crossing, vertex_crossing};
use crate::s2::predicates::{self, Direction};

/// Allows edges to be efficiently tested for intersection with a fixed edge AB.
///
/// Caches intermediate results to avoid redundant computations when testing
/// against edge chains.
///
/// # Examples
///
/// ```
/// use s2rst::s2::edge_crosser::EdgeCrosser;
/// use s2rst::s2::edge_crossings::Crossing;
/// use s2rst::s2::Point;
///
/// // Two edges that cross: A-B goes from (+x,0,+z) to (-x,0,+z),
/// // C-D goes from (0,+y,+z) to (0,-y,+z).
/// let a = Point::from_coords(1.0, 0.0, 1.0);
/// let b = Point::from_coords(-1.0, 0.0, 1.0);
/// let c = Point::from_coords(0.0, 1.0, 1.0);
/// let d = Point::from_coords(0.0, -1.0, 1.0);
///
/// let mut crosser = EdgeCrosser::new(a, b);
/// assert_eq!(crosser.crossing_sign(c, d), Crossing::Cross);
///
/// // Test an edge chain: restart and use chain_crossing_sign.
/// // Both e and f are on the same side of A-B, so e->f does not cross.
/// let e = Point::from_coords(0.0, -0.5, 1.0);
/// let f = Point::from_coords(0.0, -0.3, 1.0);
/// crosser.restart_at(e);
/// let sign = crosser.chain_crossing_sign(f);
/// assert_ne!(sign, Crossing::Cross);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeCrosser {
    pub(crate) a: Point,
    pub(crate) b: Point,
    a_xb: Point,
    /// Outward-facing tangent at A (perpendicular to AB, in the plane of A).
    a_tangent: Point,
    /// Outward-facing tangent at B.
    b_tangent: Point,
    /// Previous vertex in the chain.
    c: Point,
    /// Orientation of triangle ACB.
    acb: Direction,
}

impl EdgeCrosser {
    /// Creates a new `EdgeCrosser` for fixed edge AB.
    #[inline]
    pub fn new(a: Point, b: Point) -> Self {
        let norm = a.point_cross(b);
        EdgeCrosser {
            a,
            b,
            a_xb: Point(a.0.cross(b.0)),
            a_tangent: Point(a.0.cross(norm.0)),
            b_tangent: Point(norm.0.cross(b.0)),
            c: Point::default(),
            acb: Direction::Indeterminate,
        }
    }

    /// Creates a new `EdgeCrosser` for fixed edge AB with initial chain vertex C.
    pub fn with_start(a: Point, b: Point, c: Point) -> Self {
        let mut e = Self::new(a, b);
        e.restart_at(c);
        e
    }

    /// Sets the current point of the edge crosser to C.
    /// Call this when the chain "jumps" to a new place.
    #[inline]
    pub fn restart_at(&mut self, c: Point) {
        self.c = c;
        self.acb = -triage_sign_precomputed(self.a_xb, self.c);
    }

    /// Reports whether edge AB intersects edge CD.
    ///
    /// If any two vertices from different edges are the same, returns `MaybeCross`.
    /// If either edge is degenerate, returns `DoNotCross` or `MaybeCross`.
    #[inline]
    pub fn crossing_sign(&mut self, c: Point, d: Point) -> Crossing {
        if c != self.c {
            self.restart_at(c);
        }
        self.chain_crossing_sign(d)
    }

    /// Like `crossing_sign`, but uses the last vertex passed to a crossing
    /// method (or `restart_at`) as the first vertex of the current edge.
    #[inline]
    pub fn chain_crossing_sign(&mut self, d: Point) -> Crossing {
        let bda = triage_sign_precomputed(self.a_xb, d);
        if self.acb == -bda && bda != Direction::Indeterminate {
            // Most common case: triangles have opposite orientations.
            self.c = d;
            self.acb = -bda;
            return Crossing::DoNotCross;
        }
        self.crossing_sign_slow(d, bda)
    }

    /// Like `crossing_sign`, but also handles vertex crossings.
    #[inline]
    pub fn edge_or_vertex_crossing(&mut self, c: Point, d: Point) -> bool {
        if c != self.c {
            self.restart_at(c);
        }
        self.edge_or_vertex_chain_crossing(d)
    }

    /// Like `edge_or_vertex_crossing`, but for edge chains.
    #[inline]
    pub fn edge_or_vertex_chain_crossing(&mut self, d: Point) -> bool {
        let c = self.c;
        match self.chain_crossing_sign(d) {
            Crossing::DoNotCross => false,
            Crossing::Cross => true,
            Crossing::MaybeCross => vertex_crossing(self.a, self.b, c, d),
        }
    }

    /// Like `edge_or_vertex_crossing`, but returns a sign: +1 for a
    /// crossing where both edges go in the same direction (both outgoing or
    /// both incoming at the shared vertex), -1 for opposite directions, 0 for
    /// no crossing.
    #[inline]
    pub fn signed_edge_or_vertex_crossing(&mut self, c: Point, d: Point) -> i32 {
        if c != self.c {
            self.restart_at(c);
        }
        self.signed_edge_or_vertex_chain_crossing(d)
    }

    /// Like `signed_edge_or_vertex_crossing`, but for edge chains.
    #[inline]
    pub fn signed_edge_or_vertex_chain_crossing(&mut self, d: Point) -> i32 {
        let c = self.c;
        let crossing = self.chain_crossing_sign(d);
        match crossing {
            Crossing::DoNotCross => 0,
            Crossing::Cross => self.last_interior_crossing_sign(),
            Crossing::MaybeCross => {
                super::edge_crossings::signed_vertex_crossing(self.a, self.b, c, d)
            }
        }
    }

    /// Returns the sign of the last interior crossing detected by
    /// `chain_crossing_sign`. The sign is +1 if the crossing is in the
    /// same direction as AB, -1 if opposite.
    #[inline]
    fn last_interior_crossing_sign(&self) -> i32 {
        match self.acb {
            Direction::CounterClockwise => 1,
            Direction::Clockwise => -1,
            Direction::Indeterminate => 0,
        }
    }

    /// Slow path for `chain_crossing_sign`.
    ///
    /// Note: tried `#[cold]` here in Phase 3, regressed `bench_crossing_sign`
    /// and `bench_edge_crosser_chain` by ~1% because those bench inputs
    /// hit this path (the "slow" path is actually the common path when
    /// edges might cross). Left without an inline hint so LLVM is free to
    /// inline based on size/call-site heuristics.
    fn crossing_sign_slow(&mut self, d: Point, mut bda: Direction) -> Crossing {
        // Save d as the next c, and bda orientation, on exit.
        let result = self.crossing_sign_slow_inner(d, &mut bda);
        self.c = d;
        self.acb = -bda;
        result
    }

    fn crossing_sign_slow_inner(&mut self, d: Point, bda: &mut Direction) -> Crossing {
        // Use outward-facing tangents at A and B to reject non-intersecting edges
        // cheaply (without calling expensive_sign).
        // Go uses (1.5 + 1/sqrt(3)) * dblEpsilon.
        let max_error = (1.5 + 1.0 / predicates::SQRT3) * predicates::DBL_EPSILON;

        if (self.c.0.dot(self.a_tangent.0) > max_error && d.0.dot(self.a_tangent.0) > max_error)
            || (self.c.0.dot(self.b_tangent.0) > max_error && d.0.dot(self.b_tangent.0) > max_error)
        {
            return Crossing::DoNotCross;
        }

        // Eliminate cases where two vertices from different edges are equal.
        if self.a == self.c || self.a == d || self.b == self.c || self.b == d {
            return Crossing::MaybeCross;
        }

        // Eliminate degenerate edges.
        if self.a == self.b || self.c == d {
            return Crossing::DoNotCross;
        }

        // Time for the big guns.
        if self.acb == Direction::Indeterminate {
            self.acb = -predicates::expensive_sign(self.a, self.b, self.c);
        }
        if *bda == Direction::Indeterminate {
            *bda = predicates::expensive_sign(self.a, self.b, d);
        }

        if *bda != self.acb {
            return Crossing::DoNotCross;
        }

        let cbd = -predicates::robust_sign(self.c, d, self.b);
        if cbd != self.acb {
            return Crossing::DoNotCross;
        }
        let dac = predicates::robust_sign(self.c, d, self.a);
        if dac != self.acb {
            return Crossing::DoNotCross;
        }

        Crossing::Cross
    }
}

/// Fast path orientation test (same as `predicates::triage_sign` but
/// avoids module visibility issues).
#[inline]
/// Fast-path orientation test using a precomputed cross product `a_xb = a × b`.
/// This avoids recomputing the cross product for every edge in a chain.
fn triage_sign_precomputed(a_xb: Point, c: Point) -> Direction {
    let det = a_xb.0.dot(c.0);
    if det > predicates::MAX_DETERMINANT_ERROR {
        Direction::CounterClockwise
    } else if det < -predicates::MAX_DETERMINANT_ERROR {
        Direction::Clockwise
    } else {
        Direction::Indeterminate
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn edge_crosser_is_send_sync() {
        is_send_sync::<EdgeCrosser>();
    }

    #[test]
    fn test_crossing_sign() {
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        assert_eq!(crosser.crossing_sign(c, d), Crossing::Cross);
    }

    #[test]
    fn test_no_crossing() {
        // Two edges on the same hemisphere that don't cross.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let c = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        assert_ne!(crosser.crossing_sign(c, d), Crossing::Cross);
    }

    #[test]
    fn test_chain_crossing() {
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        crosser.restart_at(c);
        assert_eq!(crosser.chain_crossing_sign(d), Crossing::Cross);
    }

    #[test]
    fn test_edge_or_vertex_crossing() {
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        assert!(crosser.edge_or_vertex_crossing(c, d));
    }

    #[test]
    fn test_shared_vertex() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);

        let mut crosser = EdgeCrosser::new(a, b);
        assert_eq!(crosser.crossing_sign(a, b), Crossing::MaybeCross);
    }

    #[test]
    fn test_signed_edge_or_vertex_crossing_interior() {
        // Two edges that cross at an interior point.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        let sign = crosser.signed_edge_or_vertex_crossing(c, d);
        assert!(sign == 1 || sign == -1, "expected ±1, got {sign}");
    }

    #[test]
    fn test_signed_edge_or_vertex_crossing_no_cross() {
        // Two edges that don't cross.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let c = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        assert_eq!(crosser.signed_edge_or_vertex_crossing(c, d), 0);
    }

    #[test]
    fn test_signed_edge_or_vertex_crossing_same_edge() {
        // Same edge: vertex crossing with same direction → +1.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);

        let mut crosser = EdgeCrosser::new(a, b);
        assert_eq!(crosser.signed_edge_or_vertex_crossing(a, b), 1);
    }

    // --- Comprehensive 12-case crossing test (ported from C++ Crossings test) ---

    /// Helper: test a single crossing configuration and verify `crossing_sign`
    /// and `signed_edge_or_vertex_crossing`.
    fn test_crossing(
        a: Point,
        b: Point,
        c: Point,
        d: Point,
        expected_crossing: Crossing,
        expected_signed: i32,
    ) {
        let mut crosser = EdgeCrosser::new(a, b);
        assert_eq!(
            crosser.crossing_sign(c, d),
            expected_crossing,
            "crossing_sign({a:?},{b:?} vs {c:?},{d:?})"
        );
        let mut crosser2 = EdgeCrosser::new(a, b);
        assert_eq!(
            crosser2.signed_edge_or_vertex_crossing(c, d),
            expected_signed,
            "signed_edge_or_vertex_crossing({a:?},{b:?} vs {c:?},{d:?})"
        );
    }

    /// Helper: test all permutations of a crossing configuration.
    fn test_crossings(
        a: Point,
        b: Point,
        c: Point,
        d: Point,
        crossing_sign: Crossing,
        signed_crossing: i32,
    ) {
        let a = a.normalize();
        let b = b.normalize();
        let c = c.normalize();
        let d = d.normalize();

        test_crossing(a, b, c, d, crossing_sign, signed_crossing);
        test_crossing(b, a, c, d, crossing_sign, -signed_crossing);
        test_crossing(a, b, d, c, crossing_sign, -signed_crossing);
        test_crossing(b, a, d, c, crossing_sign, signed_crossing);
        // Same edge: should be vertex crossing with sign +1.
        test_crossing(a, b, a, b, Crossing::MaybeCross, 1);
        if crossing_sign == Crossing::MaybeCross {
            // For vertex crossings, if AB crosses CD then CD does not cross AB.
            test_crossing(c, d, a, b, crossing_sign, 0);
        } else {
            test_crossing(c, d, a, b, crossing_sign, -signed_crossing);
        }
    }

    #[test]
    fn test_crossings_comprehensive() {
        use crate::r3::Vector;
        let p = |x: f64, y: f64, z: f64| Point(Vector::new(x, y, z));

        // 1. Two regular edges that cross.
        test_crossings(
            p(1.0, 2.0, 1.0),
            p(1.0, -3.0, 0.5),
            p(1.0, -0.5, -3.0),
            p(0.1, 0.5, 3.0),
            Crossing::Cross,
            1,
        );

        // 2. Two regular edges that intersect antipodal points.
        test_crossings(
            p(1.0, 2.0, 1.0),
            p(1.0, -3.0, 0.5),
            p(-1.0, 0.5, 3.0),
            p(-0.1, -0.5, -3.0),
            Crossing::DoNotCross,
            0,
        );

        // 3. Two edges on the same great circle that start at antipodal points.
        test_crossings(
            p(0.0, 0.0, -1.0),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(0.0, 1.0, 1.0),
            Crossing::DoNotCross,
            0,
        );

        // 4. Two edges that cross where one vertex is origin().
        test_crossings(
            p(1.0, 0.0, 0.0),
            Point::origin(),
            p(1.0, -0.1, 1.0),
            p(1.0, 1.0, -0.1),
            Crossing::Cross,
            1,
        );

        // 5. Two edges intersecting antipodal points, one vertex is origin().
        test_crossings(
            p(1.0, 0.0, 0.0),
            Point::origin(),
            p(-1.0, 0.1, -1.0),
            p(-1.0, -1.0, 0.1),
            Crossing::DoNotCross,
            0,
        );

        // 6. Two edges that share an endpoint.
        test_crossings(
            p(7.0, -2.0, 3.0),
            p(2.0, 3.0, 4.0),
            p(2.0, 3.0, 4.0),
            p(-1.0, 2.0, 5.0),
            Crossing::MaybeCross,
            -1,
        );

        // 7. Two edges that barely cross near the middle of one edge.
        test_crossings(
            p(1.0, 1.0, 1.0),
            p(1.0, f64::from_bits(f64::to_bits(1.0) - 1), -1.0),
            p(11.0, -12.0, -1.0),
            p(10.0, 10.0, 1.0),
            Crossing::Cross,
            -1,
        );

        // 8. Edges separated by ~1e-15.
        test_crossings(
            p(1.0, 1.0, 1.0),
            p(1.0, f64::from_bits(f64::to_bits(1.0) + 1), -1.0),
            p(1.0, -1.0, 0.0),
            p(1.0, 1.0, 0.0),
            Crossing::DoNotCross,
            0,
        );

        // 9. Barely crossing near ends, floating-point underflow territory.
        test_crossings(
            p(0.0, 0.0, 1.0),
            p(2.0, -1e-323, 1.0),
            p(1.0, -1.0, 1.0),
            p(1e-323, 0.0, 1.0),
            Crossing::Cross,
            -1,
        );

        // 10. Separated by ~1e-640.
        test_crossings(
            p(0.0, 0.0, 1.0),
            p(2.0, 1e-323, 1.0),
            p(1.0, -1.0, 1.0),
            p(1e-323, 0.0, 1.0),
            Crossing::DoNotCross,
            0,
        );

        // 11. Barely crossing near middle, requires >2000 bits of precision.
        test_crossings(
            p(1.0, -1e-323, -1e-323),
            p(1e-323, 1.0, 1e-323),
            p(1.0, -1.0, 1e-323),
            p(1.0, 1.0, 0.0),
            Crossing::Cross,
            1,
        );

        // 12. Separated by ~1e-640, high precision needed.
        test_crossings(
            p(1.0, 1e-323, -1e-323),
            p(-1e-323, 1.0, 1e-323),
            p(1.0, -1.0, 1e-323),
            p(1.0, 1.0, 0.0),
            Crossing::DoNotCross,
            0,
        );
    }

    #[test]
    fn test_signed_edge_or_vertex_chain_crossing() {
        // Test chain version — interior crossing.
        let a = Point::from_coords(1.0, 0.0, 1.0).normalize();
        let b = Point::from_coords(-1.0, 0.0, 1.0).normalize();
        let c = Point::from_coords(0.0, 1.0, 1.0).normalize();
        let d = Point::from_coords(0.0, -1.0, 1.0).normalize();

        let mut crosser = EdgeCrosser::new(a, b);
        crosser.restart_at(c);
        let sign = crosser.signed_edge_or_vertex_chain_crossing(d);
        assert!(sign == 1 || sign == -1, "expected ±1, got {sign}");
    }
}
