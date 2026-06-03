// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Edge distance functions: distance to edges, projection, interpolation.
//!
//! Corresponds to Go `s2/edge_distances.go`, C++ `s2edge_distances.cc`.

use crate::s1::{Angle, ChordAngle};
use crate::s2::Point;
use crate::s2::edge_crossings::{self, Crossing};
use crate::s2::predicates;

/// Returns the distance of point X from line segment AB.
///
/// The points are expected to be normalized. The result is very accurate for
/// small distances but may have some numerical error if the distance is large
/// (approximately π/2 or greater). The case A == B is handled correctly.
#[inline]
pub fn distance_from_segment(x: Point, a: Point, b: Point) -> Angle {
    let (min_dist, _) = update_min_distance_impl(x, a, b, ChordAngle::ZERO, true);
    min_dist.to_angle()
}

/// Reports whether the distance from X to edge AB is less than `limit`.
///
/// For less-than-or-equal, specify `limit.successor()`.
/// This is faster than `distance_from_segment`. If comparing against a fixed
/// `Angle`, convert it to `ChordAngle` once and reuse.
#[inline]
pub fn is_distance_less(x: Point, a: Point, b: Point, limit: ChordAngle) -> bool {
    let (_, less) = update_min_distance(x, a, b, limit);
    less
}

/// Checks if the distance from X to edge AB is less than `min_dist`.
///
/// If so, returns the updated value and `true`. The case A == B is handled
/// correctly. Use this when computing many distances and keeping track of the
/// minimum.
#[inline]
pub fn update_min_distance(
    x: Point,
    a: Point,
    b: Point,
    min_dist: ChordAngle,
) -> (ChordAngle, bool) {
    update_min_distance_impl(x, a, b, min_dist, false)
}

/// Checks if the distance from X to edge AB is greater than `max_dist`.
///
/// If so, returns the updated value and `true`. The case A == B is handled
/// correctly.
#[inline]
pub fn update_max_distance(
    x: Point,
    a: Point,
    b: Point,
    max_dist: ChordAngle,
) -> (ChordAngle, bool) {
    let ca = x.chord_angle(a);
    let cb = x.chord_angle(b);
    let mut dist = if ca > cb { ca } else { cb };
    if dist > ChordAngle::RIGHT {
        let (d, _) = update_min_distance_impl(-x, a, b, dist, true);
        dist = ChordAngle::STRAIGHT - d;
    }
    if max_dist < dist {
        (dist, true)
    } else {
        (max_dist, false)
    }
}

/// Reports whether the minimum distance from X to edge AB is attained at an
/// interior point of AB (not an endpoint), and that distance is less than
/// `limit`.
///
/// For less-than-or-equal, specify `limit.successor()`.
#[inline]
pub fn is_interior_distance_less(x: Point, a: Point, b: Point, limit: ChordAngle) -> bool {
    let (_, less) = update_min_interior_distance(x, a, b, limit);
    less
}

/// Reports whether the minimum distance from X to AB is attained at an
/// interior point of AB, and that distance is less than `min_dist`.
///
/// If so, returns the updated value and `true`.
#[inline]
pub fn update_min_interior_distance(
    x: Point,
    a: Point,
    b: Point,
    min_dist: ChordAngle,
) -> (ChordAngle, bool) {
    interior_dist(x, a, b, min_dist, false)
}

/// Returns the point along edge AB that is closest to point X.
///
/// The fractional distance of this point along AB can be obtained using
/// `distance_fraction`. All points must be unit length.
#[inline]
pub fn project(x: Point, a: Point, b: Point) -> Point {
    let a_xb = a.point_cross(b);
    // Find the closest point to X along the great circle through AB.
    let p = x.0 - a_xb.0 * (x.0.dot(a_xb.0) / a_xb.0.norm2());

    // If this point is on the edge AB, then it's the closest point.
    if predicates::sign(a_xb, a, Point(p)) && predicates::sign(Point(p), b, a_xb) {
        return Point(p.normalize());
    }

    // Otherwise, the closest point is either A or B.
    if (x.0 - a.0).norm2() <= (x.0 - b.0).norm2() {
        a
    } else {
        b
    }
}

/// Returns the distance ratio of point X along edge AB.
///
/// If X is on the line segment AB, this is the fraction `t` such that
/// `X == interpolate(t, a, b)`. Requires A and B to be distinct.
pub fn distance_fraction(x: Point, a: Point, b: Point) -> f64 {
    debug_assert!(a != b);
    let d0 = x.0.angle(a.0);
    let d1 = x.0.angle(b.0);
    d0 / (d0 + d1)
}

/// Returns the point along line segment AB whose distance from A is the
/// given fraction `t` of the distance AB.
///
/// Does NOT require `t` to be between 0 and 1. Distances are measured on
/// the sphere surface.
#[inline]
pub fn interpolate(t: f64, a: Point, b: Point) -> Point {
    if t == 0.0 {
        return a;
    }
    if t == 1.0 {
        return b;
    }
    let ab = Angle::from_radians(a.0.angle(b.0));
    interpolate_at_distance(Angle::from_radians(t * ab.radians()), a, b)
}

/// Returns the point along line segment AB whose distance from A is the
/// angle `ax`.
#[inline]
pub fn interpolate_at_distance(ax: Angle, a: Point, b: Point) -> Point {
    let a_rad = ax.radians();

    // Use PointCross to compute the tangent vector at A towards B. The
    // result is always perpendicular to A, even if A=B or A=-B.
    let normal = a.point_cross(b);
    let tangent = normal.0.cross(a.0);

    // Compute the appropriate linear combination of A and "tangent".
    Point((a.0 * a_rad.cos() + tangent * (a_rad.sin() / tangent.norm())).normalize())
}

/// Returns the maximum error in the result of `update_min_distance`
/// (and associated functions), assuming normalized input points.
///
/// The error can be added or subtracted from a `ChordAngle` using
/// `plus_error`.
pub fn min_update_distance_max_error(dist: ChordAngle) -> f64 {
    dist.max_point_error()
        .max(min_update_interior_distance_max_error(dist))
}

/// Returns the maximum error in the result of `update_min_interior_distance`,
/// assuming normalized input points.
pub fn min_update_interior_distance_max_error(dist: ChordAngle) -> f64 {
    if dist >= ChordAngle::RIGHT {
        return 0.0;
    }

    let b = 1.0_f64.min(0.5 * dist.length2());
    let a = (b * (2.0 - b)).sqrt();
    ((2.5 + 2.0 * predicates::SQRT3 + 8.5 * a) * a
        + (2.0 + 2.0 * predicates::SQRT3 / 3.0 + 6.5 * (1.0 - b)) * b
        + (23.0 + 16.0 / predicates::SQRT3) * predicates::DBL_EPSILON)
        * predicates::DBL_EPSILON
}

/// Returns the maximum error in the result of [`update_min_distance`],
/// assuming normalized input points.
///
/// This accounts for both the interior case and the endpoint case.
/// Corresponds to C++ `S2::GetUpdateMinDistanceMaxError`.
pub fn update_min_distance_max_error(dist: ChordAngle) -> f64 {
    min_update_interior_distance_max_error(dist).max(dist.max_point_error())
}

/// Computes the minimum distance between edges a0a1 and b0b1.
///
/// If the edges cross, the distance is zero. The cases a0 == a1 and
/// b0 == b1 are handled correctly.
#[inline]
pub(crate) fn update_edge_pair_min_distance(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
    mut min_dist: ChordAngle,
) -> (ChordAngle, bool) {
    if min_dist == ChordAngle::ZERO {
        return (ChordAngle::ZERO, false);
    }
    if edge_crossings::crossing_sign(a0, a1, b0, b1) != Crossing::DoNotCross {
        return (ChordAngle::ZERO, true);
    }

    let mut updated = false;
    let (d, ok1) = update_min_distance(a0, b0, b1, min_dist);
    min_dist = d;
    updated |= ok1;
    let (d, ok2) = update_min_distance(a1, b0, b1, min_dist);
    min_dist = d;
    updated |= ok2;
    let (d, ok3) = update_min_distance(b0, a0, a1, min_dist);
    min_dist = d;
    updated |= ok3;
    let (d, ok4) = update_min_distance(b1, a0, a1, min_dist);
    min_dist = d;
    updated |= ok4;
    (min_dist, updated)
}

/// Computes the maximum distance between edges a0a1 and b0b1.
///
/// If one edge crosses the antipodal reflection of the other, the distance
/// is π.
#[inline]
pub(crate) fn update_edge_pair_max_distance(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
    mut max_dist: ChordAngle,
) -> (ChordAngle, bool) {
    if max_dist == ChordAngle::STRAIGHT {
        return (ChordAngle::STRAIGHT, false);
    }
    if edge_crossings::crossing_sign(a0, a1, -b0, -b1) != Crossing::DoNotCross {
        return (ChordAngle::STRAIGHT, true);
    }

    let mut updated = false;
    let (d, ok1) = update_max_distance(a0, b0, b1, max_dist);
    max_dist = d;
    updated |= ok1;
    let (d, ok2) = update_max_distance(a1, b0, b1, max_dist);
    max_dist = d;
    updated |= ok2;
    let (d, ok3) = update_max_distance(b0, a0, a1, max_dist);
    max_dist = d;
    updated |= ok3;
    let (d, ok4) = update_max_distance(b1, a0, a1, max_dist);
    max_dist = d;
    updated |= ok4;
    (max_dist, updated)
}

/// Returns the pair of points `(a, b)` that achieves the minimum distance
/// between edges a0a1 and b0b1.
///
/// `a` is on a0a1 and `b` is on b0b1. If the edges intersect, both are
/// equal to the intersection point. Handles a0 == a1 and b0 == b1.
#[inline]
pub fn edge_pair_closest_points(a0: Point, a1: Point, b0: Point, b1: Point) -> (Point, Point) {
    if edge_crossings::crossing_sign(a0, a1, b0, b1) == Crossing::Cross {
        let x = edge_crossings::intersection(a0, a1, b0, b1);
        return (x, x);
    }

    let (mut min_dist, _) = update_min_distance_impl(a0, b0, b1, ChordAngle::ZERO, true);
    let mut closest_vertex = 0;
    if let (d, true) = update_min_distance(a1, b0, b1, min_dist) {
        min_dist = d;
        closest_vertex = 1;
    }
    if let (d, true) = update_min_distance(b0, a0, a1, min_dist) {
        min_dist = d;
        closest_vertex = 2;
    }
    if let (_, true) = update_min_distance(b1, a0, a1, min_dist) {
        closest_vertex = 3;
    }
    match closest_vertex {
        0 => (a0, project(a0, b0, b1)),
        1 => (a1, project(a1, b0, b1)),
        2 => (project(b0, a0, a1), b0),
        3 => (project(b1, a0, a1), b1),
        _ => unreachable!(),
    }
}

/// Returns the point at distance `r` from A along the line AB.
///
/// The line AB has a well-defined direction even when A and B are antipodal
/// or nearly so. If A == B then an arbitrary direction is chosen.
pub fn point_on_line(a: Point, b: Point, r: Angle) -> Point {
    let dir = Point(a.point_cross(b).0.cross(a.0).normalize());
    point_on_ray(a, dir, r)
}

/// Returns a point to the left of edge AB at distance `r` from A,
/// orthogonal to the edge.
pub fn point_to_left(a: Point, b: Point, r: Angle) -> Point {
    point_on_ray(a, Point(a.point_cross(b).0.normalize()), r)
}

/// Returns a point to the right of edge AB at distance `r` from A,
/// orthogonal to the edge.
pub fn point_to_right(a: Point, b: Point, r: Angle) -> Point {
    point_on_ray(a, Point(b.point_cross(a).0.normalize()), r)
}

/// Returns the point at distance `r` along a ray with the given origin and
/// direction.
///
/// `dir` must be perpendicular to `origin` and both should be normalized.
pub fn point_on_ray(origin: Point, dir: Point, r: Angle) -> Point {
    Point((origin.0 * r.cos() + dir.0 * r.sin()).normalize())
}

/// Like `point_on_ray` but takes a `ChordAngle` for the distance.
///
/// Faster than converting to Angle for small distances, but cannot accurately
/// represent distances near 180 degrees.
pub fn point_on_ray_chord(origin: Point, dir: Point, r: ChordAngle) -> Point {
    // Use the trig methods of ChordAngle directly to avoid conversion overhead.
    Point((origin.0 * r.cos() + dir.0 * r.sin()).normalize())
}

/// Returns a point perpendicular to `a` toward `b`, on the left of AB.
/// The result is a unit-length direction vector orthogonal to `a`.
pub fn get_point_to_left(a: Point, b: Point) -> Point {
    Point(a.0.cross(b.0).normalize())
}

/// Returns a point perpendicular to `a` away from `b`, on the right of AB.
/// The result is a unit-length direction vector orthogonal to `a`.
pub fn get_point_to_right(a: Point, b: Point) -> Point {
    get_point_to_left(b, a)
}

// --- Private helpers ---

/// Core implementation of `update_min_distance`.
fn update_min_distance_impl(
    x: Point,
    a: Point,
    b: Point,
    min_dist: ChordAngle,
    always_update: bool,
) -> (ChordAngle, bool) {
    if let (d, true) = interior_dist(x, a, b, min_dist, always_update) {
        return (d, true);
    }

    // Otherwise the minimum distance is to one of the endpoints.
    let xa2 = (x.0 - a.0).norm2();
    let xb2 = (x.0 - b.0).norm2();
    let dist = ChordAngle::from_length2(xa2.min(xb2));
    if !always_update && dist >= min_dist {
        return (min_dist, false);
    }
    (dist, true)
}

/// Returns the shortest distance from point X to edge AB, assuming the
/// closest point is interior to AB.
///
/// If the closest point is not interior, returns `(min_dist, false)`.
fn interior_dist(
    x: Point,
    a: Point,
    b: Point,
    min_dist: ChordAngle,
    always_update: bool,
) -> (ChordAngle, bool) {
    let xa2 = (x.0 - a.0).norm2();
    let xb2 = (x.0 - b.0).norm2();

    // Check whether we might be in the interior case using the planar
    // triangle law of cosines: |XA^2 - XB^2| < AB^2.
    let ab2 = (a.0 - b.0).norm2();
    let max_error = 4.75 * predicates::DBL_EPSILON * (xa2 + xb2 + ab2)
        + 8.0 * predicates::DBL_EPSILON * predicates::DBL_EPSILON;
    if (xa2 - xb2).abs() >= ab2 + max_error {
        return (min_dist, false);
    }

    // The minimum distance might be to a point on the edge interior.
    let c = a.point_cross(b);
    let c2 = c.0.norm2();
    let x_dot_c = x.0.dot(c.0);
    let x_dot_c2 = x_dot_c * x_dot_c;
    if !always_update && x_dot_c2 > c2 * min_dist.length2() {
        return (min_dist, false);
    }

    // Exact test for the interior case.
    let cx = c.0.cross(x.0);
    if (a.0 - x.0).dot(cx) >= 0.0 || (b.0 - x.0).dot(cx) <= 0.0 {
        return (min_dist, false);
    }

    // Compute the squared chord length XR^2 = XQ^2 + QR^2.
    let qr = 1.0 - (cx.norm2() / c2).sqrt();
    let dist = ChordAngle::from_length2((x_dot_c2 / c2) + (qr * qr));

    if !always_update && dist >= min_dist {
        return (min_dist, false);
    }

    (dist, true)
}

/// Reports whether every point on edge B=b0b1 is within `tolerance` of
/// Returns true if the minimum distance between edge A=a0a1 and edge B=b0b1
/// is less than `distance`. Returns true if the distance is zero (edges cross
/// or share an endpoint) and `distance` is non-zero.
///
/// Corresponds to C++ `S2::IsEdgePairDistanceLess`.
pub fn is_edge_pair_distance_less(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
    distance: ChordAngle,
) -> bool {
    use super::edge_crossings;
    // If the edges cross or share an endpoint, the minimum distance is zero.
    if edge_crossings::crossing_sign(a0, a1, b0, b1) != Crossing::DoNotCross {
        return distance != ChordAngle::ZERO;
    }
    // Otherwise the minimum distance is achieved at an endpoint.
    is_distance_less(a0, b0, b1, distance)
        || is_distance_less(a1, b0, b1, distance)
        || is_distance_less(b0, a0, a1, distance)
        || is_distance_less(b1, a0, a1, distance)
}

/// some point on edge A=a0a1.
///
/// Tolerance must be > 0 and < PI/2.
/// Corresponds to C++ `S2::IsEdgeBNearEdgeA`.
pub fn is_edge_b_near_edge_a(a0: Point, a1: Point, b0: Point, b1: Point, tolerance: Angle) -> bool {
    debug_assert!(tolerance.radians() > 0.0);
    debug_assert!(tolerance.radians() < std::f64::consts::FRAC_PI_2);

    use predicates::Direction;

    // Compute the normal to the plane containing edge A.
    let mut a_ortho = a0.point_cross(a1).0.normalize();

    // Project b0 and b1 onto edge A.
    let a_nearest_b0 = project(b0, a0, a1);
    let a_nearest_b1 = project(b1, a0, a1);

    // If a_nearest_b0 and a_nearest_b1 have opposite orientation from a0, a1,
    // flip a_ortho.
    if predicates::robust_sign(Point(a_ortho), a_nearest_b0, a_nearest_b1) == Direction::Clockwise {
        a_ortho = a_ortho * -1.0;
    }

    // Check if both endpoints of B are within tolerance of A.
    let b0_distance = Angle::from_radians(b0.0.angle(a_nearest_b0.0));
    let b1_distance = Angle::from_radians(b1.0.angle(a_nearest_b1.0));
    if b0_distance > tolerance || b1_distance > tolerance {
        return false;
    }

    // Check the angle between the great circle planes.
    let b_ortho = b0.point_cross(b1).0.normalize();
    let planar_angle = Angle::from_radians(a_ortho.angle(b_ortho));
    if planar_angle <= tolerance {
        return true;
    }

    // When planar_angle >= Pi/2, use the special case logic.
    if planar_angle >= Angle::from_radians(std::f64::consts::FRAC_PI_2) {
        return (Angle::from_radians(b0.0.angle(a0.0)) < Angle::from_radians(b0.0.angle(a1.0)))
            == (Angle::from_radians(b1.0.angle(a0.0)) < Angle::from_radians(b1.0.angle(a1.0)));
    }

    // Otherwise check if either of the two "furthest" points on circ(B)
    // from circ(A) lies on edge B. If so, B is not near A.
    // Note: the first cross product doesn't need RobustCrossProd since args are perpendicular.
    let furthest = Point(
        b_ortho
            .cross(a0.point_cross(a1).0.normalize().cross(b_ortho))
            .normalize(),
    );
    let furthest_inv = -furthest;

    // A point p lies on B if b_ortho → b0 → p → b1 → b_ortho never turns right.
    !((predicates::robust_sign(Point(b_ortho), b0, furthest) == Direction::CounterClockwise
        && predicates::robust_sign(furthest, b1, Point(b_ortho)) == Direction::CounterClockwise)
        || (predicates::robust_sign(Point(b_ortho), b0, furthest_inv)
            == Direction::CounterClockwise
            && predicates::robust_sign(furthest_inv, b1, Point(b_ortho))
                == Direction::CounterClockwise))
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn test_distance_from_segment_basic() {
        // Distance from (0,0,1) to edge from (1,0,0) to (0,1,0) should be π/2.
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let dist = distance_from_segment(x, a, b);
        assert!(
            float64_near(dist.radians(), PI / 2.0, 1e-14),
            "distance = {}, want π/2 = {}",
            dist.radians(),
            PI / 2.0,
        );
    }

    #[test]
    fn test_distance_from_segment_on_edge() {
        // Distance from a point on the edge should be ~0.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let mid = Point::from_coords(1.0, 1.0, 0.0);
        let dist = distance_from_segment(mid, a, b);
        assert!(
            dist.radians() < 1e-14,
            "distance from midpoint = {}, want ~0",
            dist.radians(),
        );
    }

    #[test]
    fn test_distance_from_segment_degenerate() {
        // A == B: distance should be the point-to-point distance.
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let dist = distance_from_segment(x, a, a);
        assert!(
            float64_near(dist.radians(), PI / 2.0, 1e-14),
            "degenerate distance = {}, want π/2",
            dist.radians(),
        );
    }

    #[test]
    fn test_is_distance_less() {
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // Distance is π/2, so it should be less than π but not less than π/4.
        assert!(is_distance_less(x, a, b, ChordAngle::STRAIGHT));
        assert!(!is_distance_less(
            x,
            a,
            b,
            ChordAngle::from_radians(PI / 4.0)
        ));
    }

    #[test]
    fn test_update_min_distance() {
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);

        // With STRAIGHT as min_dist, should update.
        let (d, ok) = update_min_distance(x, a, b, ChordAngle::STRAIGHT);
        assert!(ok);
        assert!(
            float64_near(d.to_angle().radians(), PI / 2.0, 1e-14),
            "updated dist = {}, want π/2",
            d.to_angle().radians(),
        );

        // With ZERO as min_dist, should not update.
        let (_, ok) = update_min_distance(x, a, b, ChordAngle::ZERO);
        assert!(!ok);
    }

    #[test]
    fn test_update_max_distance() {
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);

        // With ZERO as max_dist, should update.
        let (_, ok) = update_max_distance(x, a, b, ChordAngle::ZERO);
        assert!(ok);
    }

    #[test]
    fn test_project() {
        // Project (0,0,1) onto edge from (1,0,0) to (0,1,0).
        // The closest point should be an endpoint (since the edge is in the
        // xy-plane and x is at the north pole).
        let x = Point::from_coords(0.0, 0.0, 1.0);
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let p = project(x, a, b);
        // Should be one of the endpoints.
        assert!(p == a || p == b, "projected point should be an endpoint");

        // Project a point that is close to the interior of the edge.
        let mid = Point::from_coords(1.0, 1.0, 0.01);
        let p = project(mid, a, b);
        // Should be close to (1/√2, 1/√2, 0).
        let expected = Point::from_coords(1.0, 1.0, 0.0);
        assert!(
            p.approx_eq_angle(expected, Angle::from_radians(0.01)),
            "project({mid}) on AB = {p}, want ≈ {expected}",
        );
    }

    #[test]
    fn test_distance_fraction() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // Fraction at a should be 0.
        assert!(
            float64_near(distance_fraction(a, a, b), 0.0, 1e-14),
            "fraction at A = {}, want 0",
            distance_fraction(a, a, b),
        );
        // Fraction at b should be 1.
        assert!(
            float64_near(distance_fraction(b, a, b), 1.0, 1e-14),
            "fraction at B = {}, want 1",
            distance_fraction(b, a, b),
        );
    }

    #[test]
    fn test_interpolate() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // t=0 should be a, t=1 should be b.
        assert_eq!(interpolate(0.0, a, b), a);
        assert_eq!(interpolate(1.0, a, b), b);

        // t=0.5 should be the midpoint.
        let mid = interpolate(0.5, a, b);
        let expected = Point::from_coords(1.0, 1.0, 0.0);
        assert!(
            mid.approx_eq_angle(expected, Angle::from_radians(1e-14)),
            "midpoint = {mid}, want ≈ {expected}",
        );
    }

    #[test]
    fn test_interpolate_at_distance() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // At distance 0 from A should give A.
        let p0 = interpolate_at_distance(Angle::ZERO, a, b);
        assert!(
            p0.approx_eq(a),
            "interpolate_at_distance(0, a, b) = {p0}, want ≈ {a}",
        );
    }

    #[test]
    fn test_edge_pair_closest_points_crossing() {
        // Two crossing edges.
        let a0 = Point::from_coords(1.0, 0.0, 1.0);
        let a1 = Point::from_coords(-1.0, 0.0, 1.0);
        let b0 = Point::from_coords(0.0, 1.0, 1.0);
        let b1 = Point::from_coords(0.0, -1.0, 1.0);
        let (pa, pb) = edge_pair_closest_points(a0, a1, b0, b1);
        // For crossing edges, both points should be the same (intersection).
        assert!(
            pa.approx_eq_angle(pb, Angle::from_radians(1e-10)),
            "crossing edges: pa = {pa}, pb = {pb}, should be same",
        );
    }

    #[test]
    fn test_edge_pair_min_distance_crossing() {
        let a0 = Point::from_coords(1.0, 0.0, 1.0);
        let a1 = Point::from_coords(-1.0, 0.0, 1.0);
        let b0 = Point::from_coords(0.0, 1.0, 1.0);
        let b1 = Point::from_coords(0.0, -1.0, 1.0);
        let (d, ok) = update_edge_pair_min_distance(a0, a1, b0, b1, ChordAngle::STRAIGHT);
        assert!(ok);
        assert_eq!(d, ChordAngle::ZERO);
    }

    #[test]
    fn test_point_on_line() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // At distance 0 from A should give A.
        let p = point_on_line(a, b, Angle::ZERO);
        assert!(p.approx_eq(a), "point_on_line(a, b, 0) = {p}, want ≈ {a}",);
    }

    #[test]
    fn test_point_on_ray() {
        let origin = Point::from_coords(1.0, 0.0, 0.0);
        let dir = Point::from_coords(0.0, 1.0, 0.0);
        // At distance π/2 should give dir.
        let p = point_on_ray(origin, dir, Angle::from_radians(PI / 2.0));
        assert!(
            p.approx_eq_angle(dir, Angle::from_radians(1e-14)),
            "point_on_ray at π/2 = {p}, want ≈ {dir}",
        );
    }

    #[test]
    fn test_min_update_distance_max_error() {
        // Should return a non-negative error for any valid chord angle.
        assert!(min_update_distance_max_error(ChordAngle::ZERO) >= 0.0);
        assert!(min_update_distance_max_error(ChordAngle::RIGHT) >= 0.0);
        assert!(min_update_distance_max_error(ChordAngle::STRAIGHT) >= 0.0);
    }

    #[test]
    fn test_min_update_interior_distance_max_error() {
        // For angles >= RIGHT, error should be 0.
        assert_eq!(
            min_update_interior_distance_max_error(ChordAngle::RIGHT),
            0.0
        );
        assert_eq!(
            min_update_interior_distance_max_error(ChordAngle::STRAIGHT),
            0.0
        );
        // For small angles, should be positive.
        assert!(min_update_interior_distance_max_error(ChordAngle::from_radians(0.1)) > 0.0);
    }

    #[test]
    fn test_update_min_interior_distance() {
        // For a point exactly on the interior of an edge, the interior
        // distance should be approximately zero.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        // A point very close to the midpoint, slightly off the great circle.
        let mid = Point::from_coords(1.0, 1.0, 0.001);
        let (d, ok) = update_min_interior_distance(mid, a, b, ChordAngle::STRAIGHT);
        assert!(ok, "should find interior distance for near-midpoint");
        assert!(
            d.to_angle().radians() < 0.01,
            "interior distance = {}, want < 0.01",
            d.to_angle().radians(),
        );
    }

    #[test]
    fn test_point_to_left_right() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let r = Angle::from_radians(0.1);
        let left = point_to_left(a, b, r);
        let right = point_to_right(a, b, r);
        // Left and right should be on opposite sides.
        assert!(
            left.distance(right).radians() > 0.0,
            "left and right should be distinct",
        );
        // Both should be about distance r from a.
        assert!(
            float64_near(left.distance(a).radians(), 0.1, 0.01),
            "left distance from a = {}, want ≈ 0.1",
            left.distance(a).radians(),
        );
        assert!(
            float64_near(right.distance(a).radians(), 0.1, 0.01),
            "right distance from a = {}, want ≈ 0.1",
            right.distance(a).radians(),
        );
    }

    #[test]
    fn test_edge_pair_max_distance() {
        use crate::s2::LatLng;

        // Two edges that are nearly antipodal: one near the north pole,
        // one near the south pole.
        let a0 = LatLng::from_degrees(88.0, 0.0).to_point();
        let a1 = LatLng::from_degrees(89.0, 10.0).to_point();
        let b0 = LatLng::from_degrees(-88.0, 180.0).to_point();
        let b1 = LatLng::from_degrees(-89.0, 190.0).to_point();

        let (dist, updated) = update_edge_pair_max_distance(a0, a1, b0, b1, ChordAngle::ZERO);

        assert!(updated, "should have updated max distance");
        // The max distance between near-antipodal edges should be close to PI.
        let angle = dist.to_angle().radians();
        assert!(
            angle > PI - 0.1,
            "max distance = {angle}, expected close to PI = {PI}"
        );

        // Verify that STRAIGHT is a fixed point (already at maximum).
        let (dist2, updated2) = update_edge_pair_max_distance(a0, a1, b0, b1, ChordAngle::STRAIGHT);
        assert!(
            !updated2,
            "should not update when max_dist is already STRAIGHT"
        );
        assert_eq!(dist2, ChordAngle::STRAIGHT);
    }

    // --- UpdateMinInteriorDistanceRejectionTestIsConservative (from C++) ---

    #[test]
    fn test_update_min_interior_distance_rejection_conservative() {
        use crate::r3::Vector;
        // Three regression cases from C++ where the rejection test
        // must not incorrectly skip updates.
        {
            let x = Point(Vector::new(
                1.0,
                -4.6547732744037044e-11,
                -5.6374428459823598e-89,
            ));
            let a = Point(Vector::new(1.0, -8.9031850507928352e-11, 0.0));
            let b = Point(Vector::new(
                -0.99999999999996347,
                2.7030110029169596e-07,
                1.555092348806121e-99,
            ));
            let min_dist = ChordAngle::from_length2(6.3897233584120815e-26);
            let (_, updated) = update_min_interior_distance(x, a, b, min_dist);
            assert!(updated, "case 1: should update interior distance");
        }
        {
            let x = Point(Vector::new(1.0, -4.7617930898495072e-13, 0.0));
            let a = Point(Vector::new(-1.0, -1.6065916409055676e-10, 0.0));
            let b = Point(Vector::new(1.0, 0.0, 9.9964883247706732e-35));
            let min_dist = ChordAngle::from_length2(6.3897233584120815e-26);
            let (_, updated) = update_min_interior_distance(x, a, b, min_dist);
            assert!(updated, "case 2: should update interior distance");
        }
        {
            let x = Point(Vector::new(1.0, 0.0, 0.0));
            let a = Point(Vector::new(1.0, -8.4965026896454536e-11, 0.0));
            let b = Point(Vector::new(
                -0.99999999999966138,
                8.2297529603339328e-07,
                9.6070344113320997e-21,
            ));
            let min_dist = ChordAngle::from_length2(6.3897233584120815e-26);
            let (_, updated) = update_min_interior_distance(x, a, b, min_dist);
            assert!(updated, "case 3: should update interior distance");
        }
    }

    // --- EdgeBNearEdgeA (from C++) ---

    fn is_edge_b_near_a(a_str: &str, b_str: &str, max_error_degrees: f64) -> bool {
        use crate::s2::text_format::make_polyline;
        let a = make_polyline(a_str);
        let b = make_polyline(b_str);
        is_edge_b_near_edge_a(
            a.vertex(0),
            a.vertex(1),
            b.vertex(0),
            b.vertex(1),
            Angle::from_degrees(max_error_degrees),
        )
    }

    #[test]
    fn test_edge_b_near_edge_a() {
        // Same edge (both directions).
        assert!(is_edge_b_near_a("5:5, 10:-5", "5:5, 10:-5", 1e-6));
        assert!(is_edge_b_near_a("5:5, 10:-5", "10:-5, 5:5", 1e-6));

        // Short edge is near long edge.
        assert!(is_edge_b_near_a("10:0, -10:0", "2:1, -2:1", 1.0));
        // But NOT the reverse.
        assert!(!is_edge_b_near_a("2:1, -2:1", "10:0, -10:0", 1.0));

        // Short perpendicular edge too far away.
        assert!(!is_edge_b_near_a("10:0, -10:0", "0:1.5, 0:-1.5", 1.0));
        // But within larger tolerance.
        assert!(is_edge_b_near_a("10:0, -10:0", "0:1.5, 0:-1.5", 2.0));

        // Polar edges.
        assert!(!is_edge_b_near_a("89:1, -89:1", "89:2, -89:2", 0.5));
        assert!(is_edge_b_near_a("89:1, -89:1", "89:2, -89:2", 1.5));
        assert!(is_edge_b_near_a("89:1, -89:1", "-89:2, 89:2", 1.5));

        // Long nearly-antipodal edges.
        assert!(!is_edge_b_near_a("0:-100, 0:100", "5:-80, -5:80", 70.0));
        assert!(!is_edge_b_near_a("0:-100, 0:100", "1:-35, 10:35", 70.0));
        assert!(!is_edge_b_near_a("0:-100, 0:100", "5:80, -5:-80", 70.0));

        // Nearly-antipodal parallel edges.
        assert!(!is_edge_b_near_a(
            "0:-179.75, 0:-0.25",
            "0:179.75, 0:0.25",
            1.0
        ));

        // Edge B near perpendicular bisector of A.
        assert!(is_edge_b_near_a("40:0, -5:0", "39:0.975, -1:0.975", 1.0));
        assert!(is_edge_b_near_a("10:0, -10:0", "-.4:0.975, 0.4:0.975", 1.0));

        // Edge B extending beyond endpoint of A.
        assert!(is_edge_b_near_a("0:0, 1:0", "0.9:0, 1.1:0", 0.25));
        assert!(is_edge_b_near_a("0:0, 1:0", "1.1:0, 1.2:0", 0.25));
        assert!(is_edge_b_near_a("0:0, 1:0", "1.2:0, 1.1:0", 0.25));
    }

    // --- Interpolation edge cases ---

    #[test]
    fn test_interpolate_midpoint() {
        // Midpoint of two perpendicular unit vectors.
        let i = Point::from_coords(1.0, 0.0, 0.0);
        let j = Point::from_coords(0.0, 1.0, 0.0);
        let mid = interpolate(0.5, i, j);
        // Should be on the great circle, equidistant from i and j.
        let expected = Point::from_coords(1.0, 1.0, 0.0).normalize();
        assert!(
            mid.approx_eq(expected),
            "interpolate(0.5, i, j) should be ~(1,1,0).normalize()"
        );
    }

    #[test]
    fn test_interpolate_same_point() {
        let p = Point::from_coords(0.1, 1e-30, 0.3).normalize();
        assert_eq!(interpolate(0.0, p, p), p);
        assert_eq!(interpolate(0.5, p, p), p);
        assert_eq!(interpolate(1.0, p, p), p);
    }

    #[test]
    fn test_is_edge_pair_distance_less_crossing() {
        // Two crossing edges: distance is 0.
        let a0 = Point::from_coords(1.0, 0.0, 0.1);
        let a1 = Point::from_coords(1.0, 0.0, -0.1);
        let b0 = Point::from_coords(1.0, -0.1, 0.0);
        let b1 = Point::from_coords(1.0, 0.1, 0.0);
        // Distance is 0, so should be less than any positive limit.
        assert!(is_edge_pair_distance_less(
            a0,
            a1,
            b0,
            b1,
            ChordAngle::from_degrees(1.0)
        ));
        // But not less than 0.
        assert!(!is_edge_pair_distance_less(
            a0,
            a1,
            b0,
            b1,
            ChordAngle::ZERO
        ));
    }

    #[test]
    fn test_is_edge_pair_distance_less_far() {
        // Two far-apart edges on opposite sides of the sphere.
        let a0 = Point::from_coords(1.0, 0.0, 0.0);
        let a1 = Point::from_coords(1.0, 0.1, 0.0);
        let b0 = Point::from_coords(-1.0, 0.0, 0.0);
        let b1 = Point::from_coords(-1.0, 0.0, 0.1);
        // Should not be less than 90 degrees.
        assert!(!is_edge_pair_distance_less(
            a0,
            a1,
            b0,
            b1,
            ChordAngle::from_degrees(90.0)
        ));
        // But should be less than 180 degrees.
        assert!(is_edge_pair_distance_less(
            a0,
            a1,
            b0,
            b1,
            ChordAngle::STRAIGHT
        ));
    }
}
