// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Convex hull computation on the sphere.
//!
//! [`ConvexHullQuery`] builds the convex hull of any collection of points,
//! polylines, loops, and polygons, returning a single convex [`Loop`].
//!
//! The convex hull is the smallest convex region on the sphere containing
//! all of the input geometry. Uses Andrew's monotone chain algorithm,
//! a variant of the Graham scan, in O(n log n) time.
//!
//! Corresponds to C++ `s2convex_hull_query.h`, Go `s2/convex_hull_query.go`.

use crate::s2::edge_distances;
use crate::s2::point::ortho;
use crate::s2::predicates::{self, Direction};
use crate::s2::{Cap, LatLng, Loop, Point, Rect};

/// Builds the convex hull of points, polylines, loops, and polygons.
///
/// Call the `add_*` methods to add geometry, then call [`convex_hull`](Self::convex_hull) to
/// compute the result. The query state is not cleared between calls; you can
/// keep adding geometry and recompute.
///
/// Not safe for concurrent use.
///
/// # Examples
///
/// ```
/// use s2rst::s2::convex_hull_query::ConvexHullQuery;
/// use s2rst::s2::LatLng;
///
/// let mut q = ConvexHullQuery::new();
/// q.add_point(LatLng::from_degrees(0.0, 0.0).to_point());
/// q.add_point(LatLng::from_degrees(0.0, 10.0).to_point());
/// q.add_point(LatLng::from_degrees(10.0, 5.0).to_point());
///
/// let hull = q.convex_hull();
/// assert!(hull.num_vertices() >= 3);
/// assert!(hull.area() > 0.0);
/// ```
#[derive(Debug)]
pub struct ConvexHullQuery {
    bound: Rect,
    points: Vec<Point>,
}

impl ConvexHullQuery {
    /// Creates a new empty query.
    pub fn new() -> Self {
        ConvexHullQuery {
            bound: Rect::empty(),
            points: Vec::new(),
        }
    }

    /// Adds a single point to the input geometry.
    ///
    /// Points with a non-finite (NaN or infinite) coordinate are silently
    /// ignored: they are not valid locations on the sphere and would otherwise
    /// reach the exact-predicate path, where converting a non-finite value to
    /// `ExactFloat` panics. Dropping them leaves a defined hull of the
    /// remaining valid points.
    pub fn add_point(&mut self, p: Point) {
        if !is_finite(p) {
            return;
        }
        self.bound = self.bound.add_point(LatLng::from_point(p));
        self.points.push(p);
    }

    /// Adds a slice of points to the input geometry.
    pub fn add_points(&mut self, points: &[Point]) {
        for &p in points {
            self.add_point(p);
        }
    }

    /// Adds the vertices of a polyline. Matches C++ `AddPolyline`.
    pub fn add_polyline(&mut self, polyline: &crate::s2::polyline::Polyline) {
        self.add_points(polyline.vertices_vec());
    }

    /// Adds the vertices of a loop. Matches C++ `AddLoop`.
    pub fn add_loop(&mut self, loop_: &Loop) {
        self.add_points(loop_.vertices());
    }

    /// Adds all loop vertices of a polygon. Matches C++ `AddPolygon`.
    pub fn add_polygon(&mut self, polygon: &crate::s2::polygon::Polygon) {
        for i in 0..polygon.num_loops() {
            self.add_loop(polygon.loop_at(i));
        }
    }

    /// Returns a bounding cap for the input geometry.
    pub fn cap_bound(&self) -> Cap {
        self.bound.cap_bound()
    }

    /// Returns a [`Loop`] representing the convex hull of the input geometry.
    ///
    /// Returns an empty loop if there is no geometry, a full loop if the
    /// geometry spans more than half the sphere, or a small 3-vertex loop
    /// for 1 or 2 input points.
    pub fn convex_hull(&mut self) -> Loop {
        let c = self.cap_bound();
        if c.height() >= 1.0 {
            // Bounding cap is not convex (> hemisphere). Return the full loop.
            return Loop::full();
        }

        // Remove duplicates.
        self.points.sort_unstable_by(|a, b| {
            a.0.x
                .total_cmp(&b.0.x)
                .then(a.0.y.total_cmp(&b.0.y))
                .then(a.0.z.total_cmp(&b.0.z))
        });
        self.points.dedup();

        // Sort points in CCW order around an origin outside the convex hull.
        let origin = ortho(c.center());
        self.points.sort_unstable_by(|a, b| {
            let sign = predicates::robust_sign(origin, *a, *b);
            if sign == Direction::CounterClockwise {
                std::cmp::Ordering::Less
            } else if sign == Direction::Clockwise {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // Special cases for fewer than 3 points.
        match self.points.len() {
            0 => return Loop::empty(),
            1 => return single_point_loop(self.points[0]),
            2 => return single_edge_loop(self.points[0], self.points[1]),
            _ => {}
        }

        // Generate the lower and upper halves of the convex hull.
        let lower = monotone_chain(&self.points);

        let mut reversed = self.points.clone();
        reversed.reverse();
        let upper = monotone_chain(&reversed);

        // Combine chains, removing duplicate endpoints.
        let mut hull: Vec<Point> = lower[..lower.len() - 1].to_vec();
        hull.extend_from_slice(&upper[..upper.len() - 1]);

        Loop::new(hull)
    }
}

impl Default for ConvexHullQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns `true` if every coordinate of `p` is finite (no NaN or infinity).
///
/// `Point` wraps a public `Vector`, so a caller can construct a non-finite
/// point directly (e.g. `Point::from_coords(f64::INFINITY, 0.0, 0.0)`, whose
/// normalization yields NaN). Such points are filtered out before they reach
/// the exact geometric predicates, which panic on non-finite input.
fn is_finite(p: Point) -> bool {
    p.0.x.is_finite() && p.0.y.is_finite() && p.0.z.is_finite()
}

/// Andrew's monotone chain: selects the maximal subset of points such that
/// the edge chain makes only left (CCW) turns.
fn monotone_chain(points: &[Point]) -> Vec<Point> {
    let mut output: Vec<Point> = Vec::new();
    for &p in points {
        while output.len() >= 2 {
            let n = output.len();
            if predicates::robust_sign(output[n - 2], output[n - 1], p)
                == Direction::CounterClockwise
            {
                break;
            }
            output.pop();
        }
        output.push(p);
    }
    output
}

/// Constructs a 3-vertex polygon consisting of `p` and two nearby vertices.
fn single_point_loop(p: Point) -> Loop {
    const OFFSET: f64 = 1e-15;
    let d0 = ortho(p);
    let d1 = Point(p.0.cross(d0.0));
    let vertices = vec![
        p,
        Point((p.0 + d0.0 * OFFSET).normalize()),
        Point((p.0 + d1.0 * OFFSET).normalize()),
    ];
    Loop::new(vertices)
}

/// Constructs a loop consisting of two vertices and their midpoint.
fn single_edge_loop(a: Point, b: Point) -> Loop {
    // If the points are exactly antipodal, return the full loop.
    if (a.0 + b.0).norm2() == 0.0 {
        return Loop::full();
    }
    let mid = edge_distances::interpolate(0.5, a, b);
    let mut l = Loop::new(vec![a, b, mid]);
    l.normalize();
    l
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "convex_hull_query_tests.rs"]
mod convex_hull_query_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::region::Region;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty() {
        let mut q = ConvexHullQuery::new();
        let hull = q.convex_hull();
        assert!(hull.is_empty_loop());
    }

    #[test]
    fn test_single_point() {
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        let hull = q.convex_hull();
        assert_eq!(hull.num_vertices(), 3);
    }

    #[test]
    fn test_two_points() {
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 10.0));
        let hull = q.convex_hull();
        assert_eq!(hull.num_vertices(), 3);
    }

    #[test]
    fn test_triangle() {
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 10.0));
        q.add_point(p(10.0, 0.0));
        let hull = q.convex_hull();
        // Should return at least 3 vertices
        assert!(hull.num_vertices() >= 3);
        // Area should be positive
        assert!(hull.area() > 0.0);
    }

    #[test]
    fn test_square() {
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 10.0));
        q.add_point(p(10.0, 0.0));
        q.add_point(p(10.0, 10.0));
        let hull = q.convex_hull();
        assert!(hull.num_vertices() >= 3);
        assert!(hull.area() > 0.0);
    }

    #[test]
    fn test_duplicate_points() {
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 10.0));
        q.add_point(p(0.0, 10.0));
        let hull = q.convex_hull();
        assert_eq!(hull.num_vertices(), 3);
    }

    #[test]
    fn test_convex_hull_contains_points() {
        let mut q = ConvexHullQuery::new();
        let pts = vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 5.0)];
        for &pt in &pts {
            q.add_point(pt);
        }
        let hull = q.convex_hull();

        // Hull should contain a point well inside it
        let inside = p(3.0, 5.0);
        assert!(
            hull.contains_point(&inside),
            "convex hull should contain interior point"
        );
    }

    #[test]
    fn test_add_polyline() {
        use crate::s2::polyline::Polyline;
        let polyline = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 5.0)]);
        let mut q = ConvexHullQuery::new();
        q.add_polyline(&polyline);
        let hull = q.convex_hull();
        assert!(hull.num_vertices() >= 3);
        assert!(hull.area() > 0.0);
    }

    #[test]
    fn test_add_loop() {
        let loop_ = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let mut q = ConvexHullQuery::new();
        q.add_loop(&loop_);
        let hull = q.convex_hull();
        assert!(hull.num_vertices() >= 3);
        assert!(hull.area() > 0.0);
    }

    #[test]
    fn test_add_polygon() {
        use crate::s2::polygon::Polygon;
        let loop_ = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
        let polygon = Polygon::from_loops(vec![loop_]);
        let mut q = ConvexHullQuery::new();
        q.add_polygon(&polygon);
        let hull = q.convex_hull();
        assert!(hull.num_vertices() >= 3);
    }

    #[test]
    fn test_interior_point_removed() {
        // Add 4 points where one is inside the triangle formed by the other 3.
        let mut q = ConvexHullQuery::new();
        q.add_point(p(0.0, 0.0));
        q.add_point(p(0.0, 20.0));
        q.add_point(p(20.0, 0.0));
        q.add_point(p(5.0, 5.0)); // Inside the triangle
        let hull = q.convex_hull();
        // The hull should have exactly 3 vertices (the interior point removed).
        assert_eq!(hull.num_vertices(), 3);
    }
}
