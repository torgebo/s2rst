// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A closed interval on the real line.
//!
//! Represents an interval [lo, hi]. Zero-length intervals (lo == hi) represent
//! single points. If lo > hi, the interval is empty.

use std::fmt;
use std::ops::Not;

/// Selects the lower or upper endpoint of an interval.
///
/// # Examples
///
/// ```
/// use s2rst::r1::Endpoint;
///
/// let lo = Endpoint::Lo;
/// let hi = Endpoint::Hi;
///
/// // Negation flips the endpoint.
/// assert_eq!(!lo, Endpoint::Hi);
/// assert_eq!(!hi, Endpoint::Lo);
///
/// // Conversion from bool.
/// assert_eq!(Endpoint::from(false), Endpoint::Lo);
/// assert_eq!(Endpoint::from(true), Endpoint::Hi);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Endpoint {
    /// The lower bound.
    #[default]
    Lo = 0,
    /// The upper bound.
    Hi = 1,
}

impl Not for Endpoint {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        match self {
            Endpoint::Lo => Endpoint::Hi,
            Endpoint::Hi => Endpoint::Lo,
        }
    }
}

impl From<bool> for Endpoint {
    /// `false` → `Lo`, `true` → `Hi`.
    #[inline]
    fn from(b: bool) -> Self {
        if b { Endpoint::Hi } else { Endpoint::Lo }
    }
}

/// A closed, bounded interval on the real line.
///
/// An `Interval` can represent the empty interval (containing no points)
/// and zero-length intervals (containing a single point).
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use s2rst::r1::Interval;
///
/// // Create an interval [2, 5].
/// let i = Interval::new(2.0, 5.0);
/// assert_eq!(i.length(), 3.0);
/// assert_eq!(i.center(), 3.5);
///
/// // Containment tests.
/// assert!(i.contains(3.0));
/// assert!(!i.contains(6.0));
///
/// // Intersection of [2, 5] and [3, 7] is [3, 5].
/// let j = Interval::new(3.0, 7.0);
/// let inter = i.intersection(j);
/// assert_eq!(inter.lo, 3.0);
/// assert_eq!(inter.hi, 5.0);
///
/// // Empty interval.
/// assert!(Interval::empty().is_empty());
/// ```
#[must_use]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Interval {
    /// The low bound of the interval.
    pub lo: f64,
    /// The high bound of the interval.
    pub hi: f64,
}

impl Interval {
    /// Creates a new interval [lo, hi]. If lo > hi, the interval is empty.
    pub fn new(lo: f64, hi: f64) -> Self {
        Interval { lo, hi }
    }

    /// Returns an empty interval.
    pub fn empty() -> Self {
        Interval { lo: 1.0, hi: 0.0 }
    }

    /// Returns an interval containing a single point.
    pub fn from_point(p: f64) -> Self {
        Interval { lo: p, hi: p }
    }

    /// Returns the minimal interval containing the two given points.
    pub fn from_point_pair(p1: f64, p2: f64) -> Self {
        if p1 <= p2 {
            Interval { lo: p1, hi: p2 }
        } else {
            Interval { lo: p2, hi: p1 }
        }
    }

    /// Reports whether the interval is empty (contains no points).
    pub fn is_empty(self) -> bool {
        self.lo > self.hi
    }

    /// Returns the center of the interval. Undefined for empty intervals.
    pub fn center(self) -> f64 {
        0.5 * (self.lo + self.hi)
    }

    /// Returns the length of the interval. The length of an empty interval
    /// is negative.
    pub fn length(self) -> f64 {
        self.hi - self.lo
    }

    /// Reports whether the interval contains the point `p`.
    pub fn contains(self, p: f64) -> bool {
        p >= self.lo && p <= self.hi
    }

    /// Reports whether the interior of the interval contains the point `p`.
    pub fn interior_contains(self, p: f64) -> bool {
        p > self.lo && p < self.hi
    }

    /// Reports whether this interval contains the interval `y`.
    pub fn contains_interval(self, y: Interval) -> bool {
        if y.is_empty() {
            return true;
        }
        y.lo >= self.lo && y.hi <= self.hi
    }

    /// Reports whether the interior of this interval contains the entire
    /// interval `y` (including its boundary).
    pub fn interior_contains_interval(self, y: Interval) -> bool {
        if y.is_empty() {
            return true;
        }
        y.lo > self.lo && y.hi < self.hi
    }

    /// Reports whether this interval intersects the given interval
    /// (i.e., they have any points in common).
    pub fn intersects(self, y: Interval) -> bool {
        if self.lo <= y.lo {
            y.lo <= self.hi && y.lo <= y.hi
        } else {
            self.lo <= y.hi && self.lo <= self.hi
        }
    }

    /// Reports whether the interior of this interval intersects any point
    /// of the given interval (including its boundary).
    pub fn interior_intersects(self, y: Interval) -> bool {
        y.lo < self.hi && self.lo < y.hi && self.lo < self.hi && y.lo <= y.hi
    }

    /// Returns the Hausdorff distance to the given interval `y`.
    ///
    /// For two intervals x and y, this distance is defined as:
    ///   h(x, y) = max_{p in x} min_{q in y} d(p, q)
    pub fn directed_hausdorff_distance(self, y: Interval) -> f64 {
        if self.is_empty() {
            return 0.0;
        }
        if y.is_empty() {
            return f64::INFINITY;
        }
        0.0_f64.max((self.hi - y.hi).max(y.lo - self.lo))
    }

    /// Returns the interval expanded to contain the given point `p`.
    pub fn add_point(self, p: f64) -> Interval {
        if self.is_empty() {
            Interval::from_point(p)
        } else if p < self.lo {
            Interval { lo: p, hi: self.hi }
        } else if p > self.hi {
            Interval { lo: self.lo, hi: p }
        } else {
            self
        }
    }

    /// Returns the interval expanded to contain the given interval `y`.
    pub fn add_interval(self, y: Interval) -> Interval {
        if y.is_empty() {
            return self;
        }
        if self.is_empty() {
            return y;
        }
        Interval {
            lo: self.lo.min(y.lo),
            hi: self.hi.max(y.hi),
        }
    }

    /// Returns the closest point in the interval to the given point `p`.
    ///
    /// The interval must be non-empty.
    pub fn project(self, p: f64) -> f64 {
        debug_assert!(!self.is_empty());
        p.clamp(self.lo, self.hi)
    }

    /// Returns an interval that has been expanded on each side by `margin`.
    ///
    /// If `margin` is negative, the interval is shrunk instead. The resulting
    /// interval may be empty. Any expansion of an empty interval remains empty.
    pub fn expanded(self, margin: f64) -> Interval {
        if self.is_empty() {
            return self;
        }
        Interval {
            lo: self.lo - margin,
            hi: self.hi + margin,
        }
    }

    /// Returns the smallest interval that contains this interval and `y`.
    pub fn union(self, y: Interval) -> Interval {
        if self.is_empty() {
            return y;
        }
        if y.is_empty() {
            return self;
        }
        Interval {
            lo: self.lo.min(y.lo),
            hi: self.hi.max(y.hi),
        }
    }

    /// Returns the intersection of this interval with `y`.
    ///
    /// Empty intervals do not need to be special-cased.
    pub fn intersection(self, y: Interval) -> Interval {
        Interval {
            lo: self.lo.max(y.lo),
            hi: self.hi.min(y.hi),
        }
    }

    /// Reports whether this interval can be transformed into `y` by moving
    /// each endpoint by at most `max_error`.
    ///
    /// The empty interval is considered to be positioned arbitrarily on the
    /// real line, thus any interval with length <= 2*`max_error` matches the
    /// empty interval.
    pub fn approx_eq_with(self, y: Interval, max_error: f64) -> bool {
        if self.is_empty() {
            return y.length() <= 2.0 * max_error;
        }
        if y.is_empty() {
            return self.length() <= 2.0 * max_error;
        }
        (y.lo - self.lo).abs() <= max_error && (y.hi - self.hi).abs() <= max_error
    }

    /// Like [`approx_eq_with`](Interval::approx_eq_with) with a default
    /// `max_error` of 1e-15.
    pub fn approx_eq(self, y: Interval) -> bool {
        self.approx_eq_with(y, 1e-15)
    }

    /// Returns the lower or upper endpoint of the interval.
    pub fn bound(self, i: Endpoint) -> f64 {
        match i {
            Endpoint::Lo => self.lo,
            Endpoint::Hi => self.hi,
        }
    }
}

impl Default for Interval {
    /// The default interval is empty.
    fn default() -> Self {
        Interval::empty()
    }
}

impl PartialEq for Interval {
    fn eq(&self, other: &Self) -> bool {
        (self.lo == other.lo && self.hi == other.hi) || (self.is_empty() && other.is_empty())
    }
}

impl From<(f64, f64)> for Interval {
    fn from((lo, hi): (f64, f64)) -> Self {
        Interval { lo, hi }
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lo, self.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn interval_is_send_sync() {
        is_send_sync::<Interval>();
    }

    const UNIT: Interval = Interval { lo: 0.0, hi: 1.0 };
    const NEGUNIT: Interval = Interval { lo: -1.0, hi: 0.0 };
    const HALF: Interval = Interval { lo: 0.5, hi: 0.5 };

    fn empty() -> Interval {
        Interval::empty()
    }

    /// Test helper: checks `Contains`, `InteriorContains`, `Intersects`,
    /// `InteriorIntersects` against expected results encoded as "T"/"F" chars.
    fn test_interval_ops(x: Interval, y: Interval, expected: &str) {
        let expected: Vec<bool> = expected.chars().map(|c| c == 'T').collect();
        assert_eq!(
            x.contains_interval(y),
            expected[0],
            "{x}.contains_interval({y})"
        );
        assert_eq!(
            x.interior_contains_interval(y),
            expected[1],
            "{x}.interior_contains_interval({y})"
        );
        assert_eq!(x.intersects(y), expected[2], "{x}.intersects({y})");
        assert_eq!(
            x.interior_intersects(y),
            expected[3],
            "{x}.interior_intersects({y})"
        );

        // Invariant: x.contains(y) ⟺ x.union(y) == x
        assert_eq!(x.contains_interval(y), x.union(y) == x);
        // Invariant: x.intersects(y) ⟺ !x.intersection(y).is_empty()
        assert_eq!(x.intersects(y), !x.intersection(y).is_empty());

        // Invariant: add_interval produces the same result as union
        assert_eq!(x.union(y), x.add_interval(y));
    }

    #[test]
    fn test_constructors_and_accessors() {
        assert_eq!(UNIT.lo, 0.0);
        assert_eq!(UNIT.hi, 1.0);
        assert_eq!(NEGUNIT.bound(Endpoint::Lo), -1.0);
        assert_eq!(NEGUNIT.bound(Endpoint::Hi), 0.0);

        let ten = Interval::new(0.0, 10.0);
        assert_eq!(ten, Interval::new(0.0, 10.0));

        let ten2 = Interval::new(-10.0, 10.0);
        assert_eq!(ten2.bound(Endpoint::Lo), -10.0);
        assert_eq!(ten2.bound(Endpoint::Hi), 10.0);
    }

    #[test]
    fn test_is_empty() {
        assert!(!UNIT.is_empty());
        assert!(!HALF.is_empty());
        assert!(empty().is_empty());
    }

    #[test]
    fn test_equality() {
        assert_eq!(empty(), empty());
        assert_eq!(UNIT, UNIT);
        assert_ne!(UNIT, empty());
        assert_ne!(Interval::new(1.0, 2.0), Interval::new(1.0, 3.0));
    }

    #[test]
    fn test_default_is_empty() {
        let default_empty: Interval = Interval::default();
        assert!(default_empty.is_empty());
        assert_eq!(empty().lo, default_empty.lo);
        assert_eq!(empty().hi, default_empty.hi);
    }

    #[test]
    fn test_center_and_length() {
        assert_eq!(UNIT.center(), 0.5);
        assert_eq!(HALF.center(), 0.5);
        assert_eq!(NEGUNIT.length(), 1.0);
        assert_eq!(HALF.length(), 0.0);
        assert!(empty().length() < 0.0);
    }

    #[test]
    fn test_contains_point() {
        assert!(UNIT.contains(0.5));
        assert!(UNIT.interior_contains(0.5));
        assert!(UNIT.contains(0.0));
        assert!(!UNIT.interior_contains(0.0));
        assert!(UNIT.contains(1.0));
        assert!(!UNIT.interior_contains(1.0));
    }

    #[test]
    fn test_interval_ops_cases() {
        test_interval_ops(empty(), empty(), "TTFF");
        test_interval_ops(empty(), UNIT, "FFFF");
        test_interval_ops(UNIT, HALF, "TTTT");
        test_interval_ops(UNIT, UNIT, "TFTT");
        test_interval_ops(UNIT, empty(), "TTFF");
        test_interval_ops(UNIT, NEGUNIT, "FFTF");
        test_interval_ops(UNIT, Interval::new(0.0, 0.5), "TFTT");
        test_interval_ops(HALF, Interval::new(0.0, 0.5), "FFTF");
    }

    #[test]
    fn test_add_point() {
        let r = empty();
        let r = r.add_point(5.0);
        assert_eq!(r.lo, 5.0);
        assert_eq!(r.hi, 5.0);
        let r = r.add_point(-1.0);
        assert_eq!(r.lo, -1.0);
        assert_eq!(r.hi, 5.0);
        let r = r.add_point(0.0);
        assert_eq!(r.lo, -1.0);
        assert_eq!(r.hi, 5.0);
    }

    #[test]
    fn test_project() {
        assert_eq!(Interval::new(0.1, 0.4).project(0.3), 0.3);
        assert_eq!(Interval::new(0.1, 0.4).project(-7.0), 0.1);
        assert_eq!(Interval::new(0.1, 0.4).project(0.6), 0.4);
    }

    #[test]
    fn test_from_point_pair() {
        assert_eq!(Interval::from_point_pair(4.0, 4.0), Interval::new(4.0, 4.0));
        assert_eq!(
            Interval::from_point_pair(-1.0, -2.0),
            Interval::new(-2.0, -1.0)
        );
        assert_eq!(
            Interval::from_point_pair(-5.0, 3.0),
            Interval::new(-5.0, 3.0)
        );
    }

    #[test]
    fn test_expanded() {
        assert_eq!(empty().expanded(0.45), empty());
        assert_eq!(UNIT.expanded(0.5), Interval::new(-0.5, 1.5));
        assert_eq!(UNIT.expanded(-0.5), Interval::new(0.5, 0.5));
        assert!(UNIT.expanded(-0.51).is_empty());
        assert!(UNIT.expanded(-0.51).expanded(0.51).is_empty());
    }

    #[test]
    fn test_union() {
        assert_eq!(
            Interval::new(99.0, 100.0).union(empty()),
            Interval::new(99.0, 100.0)
        );
        assert_eq!(
            empty().union(Interval::new(99.0, 100.0)),
            Interval::new(99.0, 100.0)
        );
        assert!(
            Interval::new(5.0, 3.0)
                .union(Interval::new(0.0, -2.0))
                .is_empty()
        );
        assert!(
            Interval::new(0.0, -2.0)
                .union(Interval::new(5.0, 3.0))
                .is_empty()
        );
        assert_eq!(UNIT.union(UNIT), UNIT);
        assert_eq!(UNIT.union(NEGUNIT), Interval::new(-1.0, 1.0));
        assert_eq!(NEGUNIT.union(UNIT), Interval::new(-1.0, 1.0));
        assert_eq!(HALF.union(UNIT), UNIT);
    }

    #[test]
    fn test_intersection() {
        assert_eq!(UNIT.intersection(HALF), HALF);
        assert_eq!(UNIT.intersection(NEGUNIT), Interval::new(0.0, 0.0));
        assert!(NEGUNIT.intersection(HALF).is_empty());
        assert!(UNIT.intersection(empty()).is_empty());
        assert!(empty().intersection(UNIT).is_empty());
    }

    #[test]
    fn test_directed_hausdorff_distance() {
        assert_eq!(empty().directed_hausdorff_distance(UNIT), 0.0);
        assert_eq!(UNIT.directed_hausdorff_distance(empty()), f64::INFINITY);
        assert_eq!(
            Interval::new(0.0, 1.0).directed_hausdorff_distance(Interval::new(0.0, 2.0)),
            0.0
        );
        assert_eq!(
            Interval::new(0.0, 2.0).directed_hausdorff_distance(Interval::new(0.0, 1.0)),
            1.0
        );
    }

    #[test]
    fn test_approx_equals() {
        // Choose two values such that shifting by lo is within tolerance
        // but shifting by hi is not.
        let lo = 4.0 * f64::EPSILON; // < max_error default (1e-15)
        let hi = 6.0 * f64::EPSILON; // > max_error default (1e-15)

        // Empty intervals.
        assert!(empty().approx_eq(empty()));
        assert!(Interval::new(0.0, 0.0).approx_eq(empty()));
        assert!(empty().approx_eq(Interval::new(0.0, 0.0)));
        assert!(Interval::new(1.0, 1.0).approx_eq(empty()));
        assert!(empty().approx_eq(Interval::new(1.0, 1.0)));
        assert!(!empty().approx_eq(Interval::new(0.0, 1.0)));
        assert!(empty().approx_eq(Interval::new(1.0, 1.0 + 2.0 * lo)));
        assert!(!empty().approx_eq(Interval::new(1.0, 1.0 + 2.0 * hi)));

        // Singleton intervals.
        assert!(Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0, 1.0)));
        assert!(Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0 - lo, 1.0 - lo)));
        assert!(Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0 + lo, 1.0 + lo)));
        assert!(!Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0 - hi, 1.0)));
        assert!(!Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0, 1.0 + hi)));
        assert!(Interval::new(1.0, 1.0).approx_eq(Interval::new(1.0 - lo, 1.0 + lo)));
        assert!(!Interval::new(0.0, 0.0).approx_eq(Interval::new(1.0, 1.0)));

        // Other intervals.
        assert!(Interval::new(1.0 - lo, 2.0 + lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(Interval::new(1.0 + lo, 2.0 - lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 - hi, 2.0 + lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 + hi, 2.0 - lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 - lo, 2.0 + hi).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 + lo, 2.0 - hi).approx_eq(Interval::new(1.0, 2.0)));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Interval::new(2.0, 4.5)), "[2, 4.5]");
    }

    #[test]
    fn test_from_tuple() {
        let i: Interval = (1.0, 2.0).into();
        assert_eq!(i, Interval::new(1.0, 2.0));
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    /// Clamp a float to a reasonable finite range for testing.
    fn finite(x: f64) -> f64 {
        if x.is_finite() { x } else { 0.0 }
    }

    fn make_interval(a: f64, b: f64) -> Interval {
        let a = finite(a);
        let b = finite(b);
        if a <= b {
            Interval::new(a, b)
        } else {
            Interval::new(b, a)
        }
    }

    #[quickcheck]
    fn prop_union_contains_both(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let u = a.union(b);
        u.contains_interval(a) && u.contains_interval(b)
    }

    #[quickcheck]
    fn prop_intersection_subset_of_both(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let i = a.intersection(b);
        a.contains_interval(i) && b.contains_interval(i)
    }

    #[quickcheck]
    fn prop_expanded_contains_original(a1: f64, a2: f64, margin: f64) -> bool {
        let a = make_interval(a1, a2);
        let margin = finite(margin).abs().min(1e15); // keep margin reasonable
        a.expanded(margin).contains_interval(a)
    }

    #[quickcheck]
    fn prop_empty_is_empty() -> bool {
        let e = Interval::empty();
        e.is_empty() && !e.contains(0.0) && !e.contains(1.0) && !e.contains(-1.0)
    }

    #[quickcheck]
    fn prop_from_point_singleton(p: f64) -> bool {
        let p = finite(p);
        let i = Interval::from_point(p);
        i.lo == p && i.hi == p && i.contains(p) && !i.is_empty()
    }

    #[quickcheck]
    fn prop_add_point_contains(a1: f64, a2: f64, p: f64) -> bool {
        let a = make_interval(a1, a2);
        let p = finite(p);
        a.add_point(p).contains(p)
    }

    #[quickcheck]
    fn prop_from_point_pair_ordered(a: f64, b: f64) -> bool {
        let a = finite(a);
        let b = finite(b);
        let i = Interval::from_point_pair(a, b);
        i.lo <= i.hi && i.contains(a) && i.contains(b)
    }

    #[quickcheck]
    fn prop_project_in_interval(a1: f64, a2: f64, p: f64) -> bool {
        let a = make_interval(a1, a2);
        let p = finite(p);
        let proj = a.project(p);
        a.contains(proj)
    }

    #[quickcheck]
    fn prop_union_is_commutative(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        a.union(b) == b.union(a)
    }

    #[quickcheck]
    fn prop_intersection_is_commutative(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        a.intersection(b) == b.intersection(a)
    }

    #[quickcheck]
    fn prop_length_non_negative(a1: f64, a2: f64) -> bool {
        let a = make_interval(a1, a2);
        if a.is_empty() {
            a.length() < 0.0
        } else {
            a.length() >= 0.0
        }
    }

    #[quickcheck]
    fn prop_contains_endpoints(a1: f64, a2: f64) -> bool {
        let a = make_interval(a1, a2);
        if a.is_empty() {
            return true;
        }
        a.contains(a.lo) && a.contains(a.hi)
    }

    #[quickcheck]
    fn prop_center_in_interval(a1: f64, a2: f64) -> bool {
        // Clamp to avoid overflow in center computation (lo + hi).
        let a1 = finite(a1).clamp(-1e150, 1e150);
        let a2 = finite(a2).clamp(-1e150, 1e150);
        let a = if a1 <= a2 {
            Interval::new(a1, a2)
        } else {
            Interval::new(a2, a1)
        };
        if a.is_empty() {
            return true;
        }
        a.contains(a.center())
    }

    #[quickcheck]
    fn prop_add_interval_contains_both(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let r = a.add_interval(b);
        r.contains_interval(a) && r.contains_interval(b)
    }

    #[quickcheck]
    fn prop_directed_hausdorff_non_negative(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        a.directed_hausdorff_distance(b) >= 0.0
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(a: i32, b: i32) -> bool {
        let i = make_interval(f64::from(a), f64::from(b));
        let json = serde_json::to_string(&i).unwrap();
        let back: Interval = serde_json::from_str(&json).unwrap();
        serde_json::to_string(&back).unwrap() == json
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_endpoint_roundtrip() {
        for ep in [Endpoint::Lo, Endpoint::Hi] {
            let json = serde_json::to_string(&ep).unwrap();
            let back: Endpoint = serde_json::from_str(&json).unwrap();
            assert_eq!(ep, back);
        }
    }
}
