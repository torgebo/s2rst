// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Wedge containment and intersection predicates.
//!
//! Given an edge chain `(x0, x1, x2)`, the *wedge* at `x1` is the region to
//! the left of the edges — more precisely, the set of all rays from `x1x0`
//! (inclusive) to `x1x2` (exclusive) in the clockwise direction.
//!
//! Corresponds to C++ `s2wedge_relations.h`, Go `s2/wedge_relations.go`.

use crate::s2::Point;
use crate::s2::predicates;

/// The possible relation between two wedges A and B.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WedgeRel {
    /// A and B are equal.
    #[default]
    Equal,
    /// A is a strict superset of B.
    ProperlyContains,
    /// A is a strict subset of B.
    IsProperlyContained,
    /// A−B, B−A, and A∩B are all non-empty.
    ProperlyOverlaps,
    /// A and B are disjoint.
    IsDisjoint,
}

/// Reports the relation between two non-empty wedges A=(a0, ab1, a2) and
/// B=(b0, ab1, b2) that share vertex `ab1`.
pub fn wedge_relation(a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> WedgeRel {
    // There are 6 possible edge orderings at a shared vertex (all
    // circular, i.e. abcd == bcda):
    //
    //  (1) a2 b2 b0 a0: A contains B
    //  (2) a2 a0 b0 b2: B contains A
    //  (3) a2 a0 b2 b0: A and B are disjoint
    //  (4) a2 b0 a0 b2: A and B intersect in one wedge
    //  (5) a2 b2 a0 b0: A and B intersect in one wedge
    //  (6) a2 b0 b2 a0: A and B intersect in two wedges
    //
    // We do not distinguish between 4, 5, and 6.
    if a0 == b0 && a2 == b2 {
        return WedgeRel::Equal;
    }

    // Cases 1, 2, 5, and 6.
    if predicates::ordered_ccw(a0, a2, b2, ab1) {
        // The cases with this vertex ordering are 1, 5, and 6.
        if predicates::ordered_ccw(b2, b0, a0, ab1) {
            return WedgeRel::ProperlyContains;
        }
        // We are in case 5 or 6, or case 2 if a2 == b2.
        if a2 == b2 {
            return WedgeRel::IsProperlyContained;
        }
        return WedgeRel::ProperlyOverlaps;
    }

    // We are in case 2, 3, or 4.
    if predicates::ordered_ccw(a0, b0, b2, ab1) {
        return WedgeRel::IsProperlyContained;
    }
    if predicates::ordered_ccw(a0, b0, a2, ab1) {
        return WedgeRel::IsDisjoint;
    }
    WedgeRel::ProperlyOverlaps
}

/// Reports whether non-empty wedge A=(a0, ab1, a2) contains B=(b0, ab1, b2).
///
/// Equivalent to `wedge_relation == ProperlyContains || Equal`.
pub fn wedge_contains(a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> bool {
    // For A to contain B the CCW edge order around ab1 must be a2 b2 b0 a0.
    predicates::ordered_ccw(a2, b2, b0, ab1) && predicates::ordered_ccw(b0, a0, a2, ab1)
}

/// Reports whether non-empty wedge A=(a0, ab1, a2) intersects B=(b0, ab1, b2).
///
/// Equivalent to `wedge_relation != IsDisjoint`, but faster.
pub fn wedge_intersects(a0: Point, ab1: Point, a2: Point, b0: Point, b2: Point) -> bool {
    // For A not to intersect B the CCW edge order around ab1 must be
    // a0 b2 b0 a2. Note that we use negations (!ordered_ccw) to get correct
    // results when two vertices are the same.
    !predicates::ordered_ccw(a0, b2, b0, ab1) || !predicates::ordered_ccw(b0, a2, a0, ab1)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point::from_coords(x, y, z).normalize()
    }

    // At ab1 = (0,0,1), the CCW order (viewed from outside) is:
    //   +x (0°) → +y (90°) → −x (180°) → −y (270°) → +x (360°)
    //
    // A wedge (a0, ab1, a2) sweeps clockwise from a0 to a2.
    // CW from +x to −y is the short 90° arc through the 4th quadrant.
    // CW from +x to +y is the long 270° arc through −y, −x.

    #[test]
    fn test_wedge_equal() {
        let ab1 = p(0.0, 0.0, 1.0);
        let a0 = p(1.0, 0.0, 0.0);
        let a2 = p(0.0, 1.0, 0.0);
        assert_eq!(wedge_relation(a0, ab1, a2, a0, a2), WedgeRel::Equal,);
        assert!(wedge_contains(a0, ab1, a2, a0, a2));
        assert!(wedge_intersects(a0, ab1, a2, a0, a2));
    }

    #[test]
    fn test_wedge_properly_contains() {
        // A is a large 270° wedge CW from +x to +y (through −y, −x).
        // B is a small 90° wedge CW from −y to −x, entirely inside A.
        let ab1 = p(0.0, 0.0, 1.0);
        let a0 = p(1.0, 0.0, 0.0);
        let a2 = p(0.0, 1.0, 0.0);
        let b0 = p(0.0, -1.0, 0.0);
        let b2 = p(-1.0, 0.0, 0.0);
        assert_eq!(
            wedge_relation(a0, ab1, a2, b0, b2),
            WedgeRel::ProperlyContains,
        );
        assert!(wedge_contains(a0, ab1, a2, b0, b2));
        assert!(wedge_intersects(a0, ab1, a2, b0, b2));
    }

    #[test]
    fn test_wedge_is_properly_contained() {
        // Swap A and B from ProperlyContains.
        let ab1 = p(0.0, 0.0, 1.0);
        let a0 = p(0.0, -1.0, 0.0);
        let a2 = p(-1.0, 0.0, 0.0);
        let b0 = p(1.0, 0.0, 0.0);
        let b2 = p(0.0, 1.0, 0.0);
        assert_eq!(
            wedge_relation(a0, ab1, a2, b0, b2),
            WedgeRel::IsProperlyContained,
        );
        assert!(!wedge_contains(a0, ab1, a2, b0, b2));
        assert!(wedge_intersects(a0, ab1, a2, b0, b2));
    }

    #[test]
    fn test_wedge_disjoint() {
        // A: 90° CW from +x to −y (4th quadrant).
        // B: 90° CW from −x to +y (2nd quadrant). Disjoint.
        let ab1 = p(0.0, 0.0, 1.0);
        let a0 = p(1.0, 0.0, 0.0);
        let a2 = p(0.0, -1.0, 0.0);
        let b0 = p(-1.0, 0.0, 0.0);
        let b2 = p(0.0, 1.0, 0.0);
        assert_eq!(wedge_relation(a0, ab1, a2, b0, b2), WedgeRel::IsDisjoint,);
        assert!(!wedge_contains(a0, ab1, a2, b0, b2));
        assert!(!wedge_intersects(a0, ab1, a2, b0, b2));
    }

    #[test]
    fn test_wedge_properly_overlaps() {
        // A: 180° CW from +x to −x (through −y, lower half).
        // B: 180° CW from +y to −y (through −x, left half).
        // These overlap in the 3rd quadrant but each has parts outside the other.
        let ab1 = p(0.0, 0.0, 1.0);
        let a0 = p(1.0, 0.0, 0.0);
        let a2 = p(-1.0, 0.0, 0.0);
        let b0 = p(0.0, 1.0, 0.0);
        let b2 = p(0.0, -1.0, 0.0);
        assert_eq!(
            wedge_relation(a0, ab1, a2, b0, b2),
            WedgeRel::ProperlyOverlaps,
        );
        assert!(!wedge_contains(a0, ab1, a2, b0, b2));
        assert!(wedge_intersects(a0, ab1, a2, b0, b2));
    }

    // ─── Port of C++ TestWedge cases ─────────────────────────────────

    fn test_wedge_case(
        a0: Point,
        ab1: Point,
        a2: Point,
        b0: Point,
        b2: Point,
        contains: bool,
        intersects: bool,
        expected: WedgeRel,
    ) {
        let a0 = a0.normalize();
        let ab1 = ab1.normalize();
        let a2 = a2.normalize();
        let b0 = b0.normalize();
        let b2 = b2.normalize();
        assert_eq!(
            wedge_contains(a0, ab1, a2, b0, b2),
            contains,
            "wedge_contains mismatch"
        );
        assert_eq!(
            wedge_intersects(a0, ab1, a2, b0, b2),
            intersects,
            "wedge_intersects mismatch"
        );
        assert_eq!(
            wedge_relation(a0, ab1, a2, b0, b2),
            expected,
            "wedge_relation mismatch"
        );
    }

    #[test]
    fn test_wedge_cases_from_cpp() {
        // C++: S2WedgeRelations::Wedges — all 11 cases
        let o = Point::from_coords(0.0, 0.0, 1.0);

        // Intersection in one wedge.
        test_wedge_case(
            Point::from_coords(-1.0, 0.0, 10.0),
            o,
            Point::from_coords(1.0, 2.0, 10.0),
            Point::from_coords(0.0, 1.0, 10.0),
            Point::from_coords(1.0, -2.0, 10.0),
            false,
            true,
            WedgeRel::ProperlyOverlaps,
        );
        // Intersection in two wedges.
        test_wedge_case(
            Point::from_coords(-1.0, -1.0, 10.0),
            o,
            Point::from_coords(1.0, -1.0, 10.0),
            Point::from_coords(1.0, 0.0, 10.0),
            Point::from_coords(-1.0, 1.0, 10.0),
            false,
            true,
            WedgeRel::ProperlyOverlaps,
        );
        // Normal containment.
        test_wedge_case(
            Point::from_coords(-1.0, -1.0, 10.0),
            o,
            Point::from_coords(1.0, -1.0, 10.0),
            Point::from_coords(-1.0, 0.0, 10.0),
            Point::from_coords(1.0, 0.0, 10.0),
            true,
            true,
            WedgeRel::ProperlyContains,
        );
        // Containment with equality on one side.
        test_wedge_case(
            Point::from_coords(2.0, 1.0, 10.0),
            o,
            Point::from_coords(-1.0, -1.0, 10.0),
            Point::from_coords(2.0, 1.0, 10.0),
            Point::from_coords(1.0, -5.0, 10.0),
            true,
            true,
            WedgeRel::ProperlyContains,
        );
        // Containment with equality on the other side.
        test_wedge_case(
            Point::from_coords(2.0, 1.0, 10.0),
            o,
            Point::from_coords(-1.0, -1.0, 10.0),
            Point::from_coords(1.0, -2.0, 10.0),
            Point::from_coords(-1.0, -1.0, 10.0),
            true,
            true,
            WedgeRel::ProperlyContains,
        );
        // Containment with equality on both sides (== Equal).
        test_wedge_case(
            Point::from_coords(-2.0, 3.0, 10.0),
            o,
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(-2.0, 3.0, 10.0),
            Point::from_coords(4.0, -5.0, 10.0),
            true,
            true,
            WedgeRel::Equal,
        );
        // Disjoint with equality on one side.
        test_wedge_case(
            Point::from_coords(-2.0, 3.0, 10.0),
            o,
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(-2.0, -3.0, 10.0),
            false,
            false,
            WedgeRel::IsDisjoint,
        );
        // Disjoint with equality on the other side.
        test_wedge_case(
            Point::from_coords(-2.0, 3.0, 10.0),
            o,
            Point::from_coords(0.0, 5.0, 10.0),
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(-2.0, 3.0, 10.0),
            false,
            false,
            WedgeRel::IsDisjoint,
        );
        // Disjoint with equality on both sides.
        test_wedge_case(
            Point::from_coords(-2.0, 3.0, 10.0),
            o,
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(4.0, -5.0, 10.0),
            Point::from_coords(-2.0, 3.0, 10.0),
            false,
            false,
            WedgeRel::IsDisjoint,
        );
        // B contains A with equality on one side.
        test_wedge_case(
            Point::from_coords(2.0, 1.0, 10.0),
            o,
            Point::from_coords(1.0, -5.0, 10.0),
            Point::from_coords(2.0, 1.0, 10.0),
            Point::from_coords(-1.0, -1.0, 10.0),
            false,
            true,
            WedgeRel::IsProperlyContained,
        );
        // B contains A with equality on the other side.
        test_wedge_case(
            Point::from_coords(2.0, 1.0, 10.0),
            o,
            Point::from_coords(1.0, -5.0, 10.0),
            Point::from_coords(-2.0, 1.0, 10.0),
            Point::from_coords(1.0, -5.0, 10.0),
            false,
            true,
            WedgeRel::IsProperlyContained,
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_wedge_rel_roundtrip() {
        for w in [
            WedgeRel::Equal,
            WedgeRel::ProperlyContains,
            WedgeRel::IsProperlyContained,
            WedgeRel::ProperlyOverlaps,
            WedgeRel::IsDisjoint,
        ] {
            let json = serde_json::to_string(&w).unwrap();
            let back: WedgeRel = serde_json::from_str(&json).unwrap();
            assert_eq!(w, back);
        }
    }
}
