// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A closed axis-aligned rectangle in the (x, y) plane.
//!
//! Corresponds to C++ `R2Rect`.

use std::fmt;

use crate::r1;
use crate::r2::Point;

/// A closed axis-aligned rectangle in the (x, y) plane.
///
/// The rectangle is stored as two `r1::Interval`s, one for each axis.
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::r2::{Point, Rect};
///
/// // Create a rectangle from corner points.
/// let r = Rect::from_points(Point::new(1.0, 2.0), Point::new(4.0, 6.0));
/// assert_eq!(r.size(), Point::new(3.0, 4.0));
/// assert_eq!(r.center(), Point::new(2.5, 4.0));
///
/// // Containment test.
/// assert!(r.contains_point(Point::new(2.0, 3.0)));
/// assert!(!r.contains_point(Point::new(5.0, 3.0)));
///
/// // Empty rectangle.
/// assert!(Rect::empty().is_empty());
/// ```
#[must_use]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    /// The x-interval [`x_lo`, `x_hi`].
    pub x: r1::Interval,
    /// The y-interval [`y_lo`, `y_hi`].
    pub y: r1::Interval,
}

impl Rect {
    /// Creates a rectangle from x and y intervals.
    /// Both must be either both empty or both non-empty.
    pub fn new(x: r1::Interval, y: r1::Interval) -> Self {
        debug_assert!(x.is_empty() == y.is_empty());
        Rect { x, y }
    }

    /// Creates a rectangle from lower-left and upper-right points.
    pub fn from_points(lo: Point, hi: Point) -> Self {
        let r = Rect {
            x: r1::Interval::new(lo.x, hi.x),
            y: r1::Interval::new(lo.y, hi.y),
        };
        debug_assert!(r.is_valid());
        r
    }

    /// Returns the canonical empty rectangle.
    pub fn empty() -> Self {
        Rect {
            x: r1::Interval::empty(),
            y: r1::Interval::empty(),
        }
    }

    /// Creates a rectangle containing a single point.
    pub fn from_point(p: Point) -> Self {
        Rect::from_points(p, p)
    }

    /// Creates the minimal bounding rectangle containing two points.
    pub fn from_point_pair(p1: Point, p2: Point) -> Self {
        Rect {
            x: r1::Interval::from_point_pair(p1.x, p2.x),
            y: r1::Interval::from_point_pair(p1.y, p2.y),
        }
    }

    /// Creates a rectangle from a center point and size in each dimension.
    /// Both components of size should be non-negative.
    pub fn from_center_size(center: Point, size: Point) -> Self {
        Rect {
            x: r1::Interval::new(center.x - 0.5 * size.x, center.x + 0.5 * size.x),
            y: r1::Interval::new(center.y - 0.5 * size.y, center.y + 0.5 * size.y),
        }
    }

    /// Returns the lower-left corner.
    #[inline]
    pub fn lo(self) -> Point {
        Point::new(self.x.lo, self.y.lo)
    }

    /// Returns the upper-right corner.
    #[inline]
    pub fn hi(self) -> Point {
        Point::new(self.x.hi, self.y.hi)
    }

    /// Reports whether the rectangle is valid (both intervals are either
    /// both empty or both non-empty).
    pub fn is_valid(self) -> bool {
        self.x.is_empty() == self.y.is_empty()
    }

    /// Reports whether the rectangle is empty.
    pub fn is_empty(self) -> bool {
        self.x.is_empty()
    }

    /// Returns the k-th vertex of the rectangle (k = 0,1,2,3) in CCW order.
    /// Vertex 0 is in the lower-left corner.
    pub fn vertex(self, k: i32) -> Point {
        use crate::r1::Endpoint;
        // Twiddle bits to return points in CCW order (lower left, lower right,
        // upper right, upper left).
        let k = k.rem_euclid(4) as usize;
        let j = ((k >> 1) & 1) != 0;
        let i = (usize::from(j) ^ (k & 1)) != 0;
        self.vertex_ij(Endpoint::from(i), Endpoint::from(j))
    }

    /// Returns the vertex in direction `i` along x (Lo=left, Hi=right) and
    /// direction `j` along y (Lo=down, Hi=up).
    pub fn vertex_ij(self, i: r1::Endpoint, j: r1::Endpoint) -> Point {
        Point::new(self.x.bound(i), self.y.bound(j))
    }

    /// Returns all four vertices in CCW order: lower-left, lower-right,
    /// upper-right, upper-left.
    #[inline]
    pub fn vertices(self) -> [Point; 4] {
        [
            Point::new(self.x.lo, self.y.lo),
            Point::new(self.x.hi, self.y.lo),
            Point::new(self.x.hi, self.y.hi),
            Point::new(self.x.lo, self.y.hi),
        ]
    }

    /// Returns the center of the rectangle.
    pub fn center(self) -> Point {
        Point::new(self.x.center(), self.y.center())
    }

    /// Returns the width and height as a Point. Empty rectangles have
    /// negative width and height.
    pub fn size(self) -> Point {
        Point::new(self.x.length(), self.y.length())
    }

    /// Reports whether the rectangle contains the point `p`.
    pub fn contains_point(self, p: Point) -> bool {
        self.x.contains(p.x) && self.y.contains(p.y)
    }

    /// Reports whether the interior of the rectangle contains the point `p`.
    pub fn interior_contains_point(self, p: Point) -> bool {
        self.x.interior_contains(p.x) && self.y.interior_contains(p.y)
    }

    /// Reports whether the rectangle contains the given other rectangle.
    pub fn contains(self, other: Rect) -> bool {
        self.x.contains_interval(other.x) && self.y.contains_interval(other.y)
    }

    /// Reports whether the interior of this rectangle contains all points
    /// of the given other rectangle (including its boundary).
    pub fn interior_contains(self, other: Rect) -> bool {
        self.x.interior_contains_interval(other.x) && self.y.interior_contains_interval(other.y)
    }

    /// Reports whether the two rectangles have any points in common.
    pub fn intersects(self, other: Rect) -> bool {
        self.x.intersects(other.x) && self.y.intersects(other.y)
    }

    /// Reports whether the interior of this rectangle intersects any point
    /// (including the boundary) of the given other rectangle.
    pub fn interior_intersects(self, other: Rect) -> bool {
        self.x.interior_intersects(other.x) && self.y.interior_intersects(other.y)
    }

    /// Returns the rectangle expanded to include the given point.
    pub fn add_point(self, p: Point) -> Rect {
        Rect {
            x: self.x.add_point(p.x),
            y: self.y.add_point(p.y),
        }
    }

    /// Returns the rectangle expanded to include the given other rectangle.
    pub fn add_rect(self, other: Rect) -> Rect {
        Rect {
            x: self.x.add_interval(other.x),
            y: self.y.add_interval(other.y),
        }
    }

    /// Returns the closest point in the rectangle to `p`.
    /// The rectangle must be non-empty.
    pub fn project(self, p: Point) -> Point {
        Point::new(self.x.project(p.x), self.y.project(p.y))
    }

    /// Returns a rectangle expanded on each side by `margin.x` in x and
    /// `margin.y` in y. Negative margins shrink the rectangle. Any expansion
    /// of an empty rectangle remains empty.
    pub fn expanded(self, margin: Point) -> Rect {
        let xx = self.x.expanded(margin.x);
        let yy = self.y.expanded(margin.y);
        if xx.is_empty() || yy.is_empty() {
            return Rect::empty();
        }
        Rect { x: xx, y: yy }
    }

    /// Returns a rectangle expanded by `margin` on all four sides.
    pub fn expanded_by_margin(self, margin: f64) -> Rect {
        self.expanded(Point::new(margin, margin))
    }

    /// Returns the smallest rectangle containing the union of this rectangle
    /// and `other`.
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            x: self.x.union(other.x),
            y: self.y.union(other.y),
        }
    }

    /// Returns the smallest rectangle containing the intersection of this
    /// rectangle and `other`.
    pub fn intersection(self, other: Rect) -> Rect {
        let xx = self.x.intersection(other.x);
        let yy = self.y.intersection(other.y);
        if xx.is_empty() || yy.is_empty() {
            return Rect::empty();
        }
        Rect { x: xx, y: yy }
    }

    /// Reports whether the x- and y-intervals of the two rectangles are
    /// the same up to `max_error`.
    pub fn approx_eq_with(self, other: Rect, max_error: f64) -> bool {
        self.x.approx_eq_with(other.x, max_error) && self.y.approx_eq_with(other.y, max_error)
    }

    /// Like [`approx_eq_with`](Rect::approx_eq_with) with a default
    /// `max_error` of 1e-15.
    pub fn approx_eq(self, other: Rect) -> bool {
        self.approx_eq_with(other, 1e-15)
    }
}

impl Default for Rect {
    /// The default rectangle is empty.
    fn default() -> Self {
        Rect::empty()
    }
}

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl fmt::Display for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Lo{}, Hi{}]", self.lo(), self.hi())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn rect_is_send_sync() {
        is_send_sync::<Rect>();
    }

    fn sw() -> Point {
        Point::new(0.0, 0.25)
    }
    fn ne() -> Point {
        Point::new(0.5, 0.75)
    }
    fn rect() -> Rect {
        Rect::from_points(sw(), ne())
    }
    fn rect_mid() -> Rect {
        Rect::from_points(Point::new(0.25, 0.5), Point::new(0.25, 0.5))
    }
    fn rect_sw() -> Rect {
        Rect::from_point(sw())
    }
    fn rect_ne() -> Rect {
        Rect::from_point(ne())
    }

    /// Test helper matching C++ `TestIntervalOps`.
    fn test_interval_ops(
        x: Rect,
        y: Rect,
        expected: &str,
        expected_union: Rect,
        expected_intersection: Rect,
    ) {
        let exp: Vec<bool> = expected.chars().map(|c| c == 'T').collect();
        assert_eq!(x.contains(y), exp[0], "{x}.contains({y})");
        assert_eq!(x.interior_contains(y), exp[1], "{x}.interior_contains({y})");
        assert_eq!(x.intersects(y), exp[2], "{x}.intersects({y})");
        assert_eq!(
            x.interior_intersects(y),
            exp[3],
            "{x}.interior_intersects({y})"
        );

        assert_eq!(x.union(y) == x, x.contains(y));
        assert_eq!(!x.intersection(y).is_empty(), x.intersects(y));

        assert_eq!(x.union(y), expected_union, "union");
        assert_eq!(x.intersection(y), expected_intersection, "intersection");

        // add_rect should produce the same as union.
        assert_eq!(x.add_rect(y), expected_union, "add_rect");

        // If y is a single point, add_point should also produce the union.
        if y.size() == Point::new(0.0, 0.0) {
            assert_eq!(x.add_point(y.lo()), expected_union, "add_point");
        }
    }

    #[test]
    fn test_empty_rectangles() {
        let e = Rect::empty();
        assert!(e.is_valid());
        assert!(e.is_empty());
        assert_eq!(e, e);
    }

    #[test]
    fn test_constructors_and_accessors() {
        let r = Rect::from_points(Point::new(0.1, 0.0), Point::new(0.25, 1.0));
        assert_eq!(r.x.lo, 0.1);
        assert_eq!(r.x.hi, 0.25);
        assert_eq!(r.y.lo, 0.0);
        assert_eq!(r.y.hi, 1.0);

        assert_eq!(r.x, r1::Interval::new(0.1, 0.25));
        assert_eq!(r.y, r1::Interval::new(0.0, 1.0));

        assert_eq!(r, r);
        assert_ne!(r, Rect::empty());

        let r2: Rect = Rect::default();
        assert!(r2.is_empty());
        assert_eq!(r2, Rect::empty());
    }

    #[test]
    fn test_from_center_size() {
        assert!(
            Rect::from_center_size(Point::new(0.3, 0.5), Point::new(0.2, 0.4)).approx_eq(
                Rect::from_points(Point::new(0.2, 0.3), Point::new(0.4, 0.7))
            )
        );
        assert!(
            Rect::from_center_size(Point::new(1.0, 0.1), Point::new(0.0, 2.0)).approx_eq(
                Rect::from_points(Point::new(1.0, -0.9), Point::new(1.0, 1.1))
            )
        );
    }

    #[test]
    fn test_from_point() {
        let d1 = Rect::from_points(Point::new(0.1, 0.0), Point::new(0.25, 1.0));
        assert_eq!(
            Rect::from_points(d1.lo(), d1.lo()),
            Rect::from_point(d1.lo())
        );
        assert_eq!(
            Rect::from_points(Point::new(0.15, 0.3), Point::new(0.35, 0.9)),
            Rect::from_point_pair(Point::new(0.15, 0.9), Point::new(0.35, 0.3))
        );
        assert_eq!(
            Rect::from_points(Point::new(0.12, 0.0), Point::new(0.83, 0.5)),
            Rect::from_point_pair(Point::new(0.83, 0.0), Point::new(0.12, 0.5))
        );
    }

    #[test]
    fn test_simple_predicates() {
        let r1 = Rect::from_points(sw(), ne());
        assert_eq!(r1.center(), Point::new(0.25, 0.5));
        assert_eq!(r1.vertex(0), Point::new(0.0, 0.25));
        assert_eq!(r1.vertex(1), Point::new(0.5, 0.25));
        assert_eq!(r1.vertex(2), Point::new(0.5, 0.75));
        assert_eq!(r1.vertex(3), Point::new(0.0, 0.75));
        assert!(r1.contains_point(Point::new(0.2, 0.4)));
        assert!(!r1.contains_point(Point::new(0.2, 0.8)));
        assert!(!r1.contains_point(Point::new(-0.1, 0.4)));
        assert!(!r1.contains_point(Point::new(0.6, 0.1)));
        assert!(r1.contains_point(sw()));
        assert!(r1.contains_point(ne()));
        assert!(!r1.interior_contains_point(sw()));
        assert!(!r1.interior_contains_point(ne()));

        // Verify vertices are in CCW order.
        for k in 0..4 {
            let a = r1.vertex(k - 1);
            let b = r1.vertex(k);
            let c = r1.vertex(k + 1);
            assert!((b - a).ortho().dot(c - a) > 0.0);
        }
    }

    #[test]
    fn test_interval_operations() {
        let e = Rect::empty();
        let r = rect();

        test_interval_ops(r, rect_mid(), "TTTT", r, rect_mid());
        test_interval_ops(r, rect_sw(), "TFTF", r, rect_sw());
        test_interval_ops(r, rect_ne(), "TFTF", r, rect_ne());

        test_interval_ops(
            r,
            Rect::from_points(Point::new(0.45, 0.1), Point::new(0.75, 0.3)),
            "FFTT",
            Rect::from_points(Point::new(0.0, 0.1), Point::new(0.75, 0.75)),
            Rect::from_points(Point::new(0.45, 0.25), Point::new(0.5, 0.3)),
        );
        test_interval_ops(
            r,
            Rect::from_points(Point::new(0.5, 0.1), Point::new(0.7, 0.3)),
            "FFTF",
            Rect::from_points(Point::new(0.0, 0.1), Point::new(0.7, 0.75)),
            Rect::from_points(Point::new(0.5, 0.25), Point::new(0.5, 0.3)),
        );
        test_interval_ops(
            r,
            Rect::from_points(Point::new(0.45, 0.1), Point::new(0.7, 0.25)),
            "FFTF",
            Rect::from_points(Point::new(0.0, 0.1), Point::new(0.7, 0.75)),
            Rect::from_points(Point::new(0.45, 0.25), Point::new(0.5, 0.25)),
        );

        test_interval_ops(
            Rect::from_points(Point::new(0.1, 0.2), Point::new(0.1, 0.3)),
            Rect::from_points(Point::new(0.15, 0.7), Point::new(0.2, 0.8)),
            "FFFF",
            Rect::from_points(Point::new(0.1, 0.2), Point::new(0.2, 0.8)),
            e,
        );

        // Overlap in x but not y, and vice versa.
        test_interval_ops(
            Rect::from_points(Point::new(0.1, 0.2), Point::new(0.4, 0.5)),
            Rect::from_points(Point::new(0.0, 0.0), Point::new(0.2, 0.1)),
            "FFFF",
            Rect::from_points(Point::new(0.0, 0.0), Point::new(0.4, 0.5)),
            e,
        );
        test_interval_ops(
            Rect::from_points(Point::new(0.0, 0.0), Point::new(0.1, 0.3)),
            Rect::from_points(Point::new(0.2, 0.1), Point::new(0.3, 0.4)),
            "FFFF",
            Rect::from_points(Point::new(0.0, 0.0), Point::new(0.3, 0.4)),
            e,
        );
    }

    #[test]
    fn test_add_point() {
        let mut r = Rect::empty();
        r = r.add_point(Point::new(0.0, 0.25));
        r = r.add_point(Point::new(0.5, 0.25));
        r = r.add_point(Point::new(0.0, 0.75));
        r = r.add_point(Point::new(0.1, 0.4));
        assert_eq!(r, rect());
    }

    #[test]
    fn test_project() {
        let r = Rect::new(r1::Interval::new(0.0, 0.5), r1::Interval::new(0.25, 0.75));
        assert_eq!(r.project(Point::new(-0.01, 0.24)), Point::new(0.0, 0.25));
        assert_eq!(r.project(Point::new(-5.0, 0.48)), Point::new(0.0, 0.48));
        assert_eq!(r.project(Point::new(-5.0, 2.48)), Point::new(0.0, 0.75));
        assert_eq!(r.project(Point::new(0.19, 2.48)), Point::new(0.19, 0.75));
        assert_eq!(r.project(Point::new(6.19, 2.48)), Point::new(0.5, 0.75));
        assert_eq!(r.project(Point::new(6.19, 0.53)), Point::new(0.5, 0.53));
        assert_eq!(r.project(Point::new(6.19, -2.53)), Point::new(0.5, 0.25));
        assert_eq!(r.project(Point::new(0.33, -2.53)), Point::new(0.33, 0.25));
        assert_eq!(r.project(Point::new(0.33, 0.37)), Point::new(0.33, 0.37));
    }

    #[test]
    fn test_expanded() {
        // Empty rectangles stay empty.
        assert!(Rect::empty().expanded(Point::new(0.1, 0.3)).is_empty());
        assert!(Rect::empty().expanded(Point::new(-0.1, -0.3)).is_empty());

        // Normal expansions.
        assert!(
            Rect::from_points(Point::new(0.2, 0.4), Point::new(0.3, 0.7))
                .expanded(Point::new(0.1, 0.3))
                .approx_eq(Rect::from_points(
                    Point::new(0.1, 0.1),
                    Point::new(0.4, 1.0)
                ))
        );

        // Shrinking that empties one axis empties the whole rect.
        assert!(
            Rect::from_points(Point::new(0.2, 0.4), Point::new(0.3, 0.7))
                .expanded(Point::new(-0.1, 0.3))
                .is_empty()
        );
        assert!(
            Rect::from_points(Point::new(0.2, 0.4), Point::new(0.3, 0.7))
                .expanded(Point::new(0.1, -0.2))
                .is_empty()
        );

        // Partial shrink.
        assert!(
            Rect::from_points(Point::new(0.2, 0.4), Point::new(0.3, 0.7))
                .expanded(Point::new(0.1, -0.1))
                .approx_eq(Rect::from_points(
                    Point::new(0.1, 0.5),
                    Point::new(0.4, 0.6)
                ))
        );

        // Uniform margin.
        assert!(
            Rect::from_points(Point::new(0.2, 0.4), Point::new(0.3, 0.7))
                .expanded_by_margin(0.1)
                .approx_eq(Rect::from_points(
                    Point::new(0.1, 0.3),
                    Point::new(0.4, 0.8)
                ))
        );
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    fn finite(x: f64) -> f64 {
        if x.is_finite() { x } else { 0.0 }
    }

    fn make_point(x: f64, y: f64) -> Point {
        Point::new(finite(x), finite(y))
    }

    fn make_rect(x1: f64, x2: f64, y1: f64, y2: f64) -> Rect {
        let x1 = finite(x1);
        let x2 = finite(x2);
        let y1 = finite(y1);
        let y2 = finite(y2);
        let (x_lo, x_hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (y_lo, y_hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Rect::from_points(Point::new(x_lo, y_lo), Point::new(x_hi, y_hi))
    }

    #[quickcheck]
    fn prop_from_point_contains(x: f64, y: f64) -> bool {
        let p = make_point(x, y);
        Rect::from_point(p).contains_point(p)
    }

    #[quickcheck]
    fn prop_expanded_contains(x1: f64, x2: f64, y1: f64, y2: f64, margin: f64) -> bool {
        let r = make_rect(x1, x2, y1, y2);
        let margin = finite(margin).abs().min(1e15);
        r.expanded_by_margin(margin).contains(r)
    }

    #[quickcheck]
    fn prop_area_non_negative(x1: f64, x2: f64, y1: f64, y2: f64) -> bool {
        let r = make_rect(x1, x2, y1, y2);
        let s = r.size();
        s.x >= 0.0 && s.y >= 0.0
    }

    #[quickcheck]
    fn prop_union_contains_both(
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        a1: f64,
        a2: f64,
        b1: f64,
        b2: f64,
    ) -> bool {
        let a = make_rect(x1, x2, y1, y2);
        let b = make_rect(a1, a2, b1, b2);
        let u = a.union(b);
        u.contains(a) && u.contains(b)
    }

    #[quickcheck]
    fn prop_add_point_contains(x1: f64, x2: f64, y1: f64, y2: f64, px: f64, py: f64) -> bool {
        let r = make_rect(x1, x2, y1, y2);
        let p = make_point(px, py);
        r.add_point(p).contains_point(p)
    }

    #[quickcheck]
    fn prop_intersection_subset_of_both(
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        a1: f64,
        a2: f64,
        b1: f64,
        b2: f64,
    ) -> bool {
        let a = make_rect(x1, x2, y1, y2);
        let b = make_rect(a1, a2, b1, b2);
        let i = a.intersection(b);
        a.contains(i) && b.contains(i)
    }

    #[quickcheck]
    fn prop_union_is_commutative(
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        a1: f64,
        a2: f64,
        b1: f64,
        b2: f64,
    ) -> bool {
        let a = make_rect(x1, x2, y1, y2);
        let b = make_rect(a1, a2, b1, b2);
        a.union(b) == b.union(a)
    }

    #[quickcheck]
    fn prop_project_in_rect(x1: f64, x2: f64, y1: f64, y2: f64, px: f64, py: f64) -> bool {
        let r = make_rect(x1, x2, y1, y2);
        let p = make_point(px, py);
        r.contains_point(r.project(p))
    }

    #[quickcheck]
    fn prop_center_in_rect(x1: f64, x2: f64, y1: f64, y2: f64) -> bool {
        // Clamp to avoid overflow in center computation.
        let x1 = finite(x1).clamp(-1e150, 1e150);
        let x2 = finite(x2).clamp(-1e150, 1e150);
        let y1 = finite(y1).clamp(-1e150, 1e150);
        let y2 = finite(y2).clamp(-1e150, 1e150);
        let (x_lo, x_hi) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let (y_lo, y_hi) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        let r = Rect::from_points(Point::new(x_lo, y_lo), Point::new(x_hi, y_hi));
        r.contains_point(r.center())
    }

    #[quickcheck]
    fn prop_intersection_is_commutative(
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        a1: f64,
        a2: f64,
        b1: f64,
        b2: f64,
    ) -> bool {
        let a = make_rect(x1, x2, y1, y2);
        let b = make_rect(a1, a2, b1, b2);
        a.intersection(b) == b.intersection(a)
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(x1: i32, x2: i32, y1: i32, y2: i32) -> bool {
        let r = make_rect(f64::from(x1), f64::from(x2), f64::from(y1), f64::from(y2));
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        serde_json::to_string(&back).unwrap() == json
    }
}
