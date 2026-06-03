// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! A closed interval on the unit circle (1-dimensional sphere).
//!
//! An `Interval` represents a closed interval on the unit circle. It can
//! represent the empty interval, the full interval, and zero-length intervals.
//!
//! Points are represented by the angle they make with the positive x-axis in
//! the range \[-π, π\]. The lower bound may be greater than the upper bound,
//! in which case the interval is "inverted" (it passes through the point
//! (-1, 0)).
//!
//! The point (-1, 0) has two representations: π and -π. Internally, -π is
//! normalized to π, except for the full interval \[-π, π\] and the empty
//! interval \[π, -π\].

use std::f64::consts::PI;
use std::fmt;

/// A closed interval on the unit circle.
///
/// This type is `Copy` and intended to be passed by value.
///
/// # Examples
///
/// ```
/// use std::f64::consts::PI;
/// use s2rst::s1::Interval;
///
/// // A simple interval from 0 to π/2 (first quadrant).
/// let i = Interval::new(0.0, PI / 2.0);
/// assert!(!i.is_empty());
/// assert!(i.contains(PI / 4.0));
/// assert!(!i.contains(PI));
///
/// // An inverted interval that wraps through -π/π.
/// let wrap = Interval::new(3.0, -3.0);
/// assert!(wrap.is_inverted());
/// assert!(wrap.contains(PI));  // π is between 3.0 and -3.0 the "long way"
///
/// // The full circle.
/// let full = Interval::full();
/// assert!(full.is_full());
/// assert_eq!(full.length(), 2.0 * PI);
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
    /// Creates a new interval from endpoints. Both endpoints must be in
    /// \[-π, π\]. The value -π is normalized to π (except for full/empty).
    pub fn new(lo: f64, hi: f64) -> Self {
        // Note: both checks use the *original* lo/hi values (matching C++
        // where the parameter names shadow nothing).
        let new_lo = if lo == -PI && hi != PI { PI } else { lo };
        let new_hi = if hi == -PI && lo != PI { PI } else { hi };
        debug_assert!(is_valid_pair(new_lo, new_hi));
        Interval {
            lo: new_lo,
            hi: new_hi,
        }
    }

    /// Internal constructor that skips -π normalization. Both arguments
    /// must already be in the correct range.
    fn new_checked(lo: f64, hi: f64) -> Self {
        debug_assert!(is_valid_pair(lo, hi));
        Interval { lo, hi }
    }

    /// Returns the empty interval \[π, -π\].
    pub fn empty() -> Self {
        Interval { lo: PI, hi: -PI }
    }

    /// Returns the full interval \[-π, π\].
    pub fn full() -> Self {
        Interval { lo: -PI, hi: PI }
    }

    /// Returns an interval containing a single point.
    pub fn from_point(p: f64) -> Self {
        let p = if p == -PI { PI } else { p };
        Interval::new_checked(p, p)
    }

    /// Returns the minimal interval containing the two given points.
    /// Both arguments must be in \[-π, π\].
    pub fn from_point_pair(p1: f64, p2: f64) -> Self {
        debug_assert!(p1.abs() <= PI);
        debug_assert!(p2.abs() <= PI);
        let p1 = if p1 == -PI { PI } else { p1 };
        let p2 = if p2 == -PI { PI } else { p2 };
        if positive_distance(p1, p2) <= PI {
            Interval::new_checked(p1, p2)
        } else {
            Interval::new_checked(p2, p1)
        }
    }

    /// Reports whether the interval is valid.
    pub fn is_valid(self) -> bool {
        is_valid_pair(self.lo, self.hi)
    }

    /// Reports whether the interval is full (contains all points).
    pub fn is_full(self) -> bool {
        self.lo == -PI && self.hi == PI
    }

    /// Reports whether the interval is empty (contains no points).
    pub fn is_empty(self) -> bool {
        self.lo == PI && self.hi == -PI
    }

    /// Reports whether the interval is inverted (lo > hi).
    /// This is true for empty intervals.
    pub fn is_inverted(self) -> bool {
        self.lo > self.hi
    }

    /// Returns the midpoint of the interval. Undefined for full and empty
    /// intervals.
    pub fn center(self) -> f64 {
        let center = 0.5 * (self.lo + self.hi);
        if !self.is_inverted() {
            return center;
        }
        if center <= 0.0 {
            center + PI
        } else {
            center - PI
        }
    }

    /// Returns the length of the interval. The length of an empty interval
    /// is negative.
    pub fn length(self) -> f64 {
        let length = self.hi - self.lo;
        if length >= 0.0 {
            return length;
        }
        let length = length + 2.0 * PI;
        if length > 0.0 { length } else { -1.0 }
    }

    /// Returns the complement of the interior of the interval.
    ///
    /// An interval and its complement have the same boundary but do not share
    /// any interior values. The complement of a singleton interval is full.
    pub fn complement(self) -> Interval {
        if self.lo == self.hi {
            return Interval::full(); // Singleton.
        }
        Interval::new_checked(self.hi, self.lo) // Handles empty and full.
    }

    /// Returns the midpoint of the complement of the interval.
    ///
    /// For full and empty intervals, the result is arbitrary. For a singleton
    /// interval, the result is its antipodal point on S1.
    pub fn complement_center(self) -> f64 {
        if self.lo != self.hi {
            return self.complement().center();
        }
        // Singleton.
        if self.hi <= 0.0 {
            self.hi + PI
        } else {
            self.hi - PI
        }
    }

    /// Reports whether the interval contains the point `p`.
    /// Assumes `p` is in \[-π, π\].
    pub fn contains(self, p: f64) -> bool {
        debug_assert!(p.abs() <= PI);
        let p = if p == -PI { PI } else { p };
        self.fast_contains(p)
    }

    /// Reports whether the interior of the interval contains the point `p`.
    pub fn interior_contains(self, p: f64) -> bool {
        debug_assert!(p.abs() <= PI);
        let p = if p == -PI { PI } else { p };
        if self.is_inverted() {
            p > self.lo || p < self.hi
        } else {
            (p > self.lo && p < self.hi) || self.is_full()
        }
    }

    /// Reports whether this interval contains the interval `y`.
    pub fn contains_interval(self, y: Interval) -> bool {
        if self.is_inverted() {
            if y.is_inverted() {
                return y.lo >= self.lo && y.hi <= self.hi;
            }
            return (y.lo >= self.lo || y.hi <= self.hi) && !self.is_empty();
        }
        if y.is_inverted() {
            return self.is_full() || y.is_empty();
        }
        y.lo >= self.lo && y.hi <= self.hi
    }

    /// Reports whether the interior of this interval contains the entire
    /// interval `y`.
    pub fn interior_contains_interval(self, y: Interval) -> bool {
        if self.is_inverted() {
            if !y.is_inverted() {
                return y.lo > self.lo || y.hi < self.hi;
            }
            return (y.lo > self.lo && y.hi < self.hi) || y.is_empty();
        }
        if y.is_inverted() {
            return self.is_full() || y.is_empty();
        }
        (y.lo > self.lo && y.hi < self.hi) || self.is_full()
    }

    /// Reports whether this interval intersects the given interval.
    pub fn intersects(self, y: Interval) -> bool {
        if self.is_empty() || y.is_empty() {
            return false;
        }
        if self.is_inverted() {
            return y.is_inverted() || y.lo <= self.hi || y.hi >= self.lo;
        }
        if y.is_inverted() {
            return y.lo <= self.hi || y.hi >= self.lo;
        }
        y.lo <= self.hi && y.hi >= self.lo
    }

    /// Reports whether the interior of this interval intersects any point of
    /// the interval `y` (including its boundary).
    pub fn interior_intersects(self, y: Interval) -> bool {
        if self.is_empty() || y.is_empty() || self.lo == self.hi {
            return false;
        }
        if self.is_inverted() {
            return y.is_inverted() || y.lo < self.hi || y.hi > self.lo;
        }
        if y.is_inverted() {
            return y.lo < self.hi || y.hi > self.lo;
        }
        (y.lo < self.hi && y.hi > self.lo) || self.is_full()
    }

    /// Returns the Hausdorff distance to the given interval `y`, measured
    /// along S1.
    pub fn directed_hausdorff_distance(self, y: Interval) -> f64 {
        if y.contains_interval(self) {
            return 0.0; // includes the case self is empty
        }
        if y.is_empty() {
            return PI; // maximum possible distance on S1
        }
        let y_complement_center = y.complement_center();
        if self.contains(y_complement_center) {
            return positive_distance(y.hi, y_complement_center);
        }
        // The Hausdorff distance is realized by either two hi endpoints or two
        // lo endpoints, whichever is farther apart.
        let hi_hi = if Interval::new_checked(y.hi, y_complement_center).contains(self.hi) {
            positive_distance(y.hi, self.hi)
        } else {
            0.0
        };
        let lo_lo = if Interval::new_checked(y_complement_center, y.lo).contains(self.lo) {
            positive_distance(self.lo, y.lo)
        } else {
            0.0
        };
        debug_assert!(hi_hi > 0.0 || lo_lo > 0.0);
        hi_hi.max(lo_lo)
    }

    /// Returns the interval expanded to contain the given point `p`.
    pub fn add_point(self, p: f64) -> Interval {
        debug_assert!(p.abs() <= PI);
        let p = if p == -PI { PI } else { p };
        if self.fast_contains(p) {
            return self;
        }
        if self.is_empty() {
            return Interval::from_point(p);
        }
        if positive_distance(p, self.lo) < positive_distance(self.hi, p) {
            Interval::new_checked(p, self.hi)
        } else {
            Interval::new_checked(self.lo, p)
        }
    }

    /// Returns the closest point in the interval to the given point `p`.
    /// The interval must be non-empty.
    pub fn project(self, p: f64) -> f64 {
        debug_assert!(!self.is_empty());
        debug_assert!(p.abs() <= PI);
        let p = if p == -PI { PI } else { p };
        if self.fast_contains(p) {
            return p;
        }
        let dlo = positive_distance(p, self.lo);
        let dhi = positive_distance(self.hi, p);
        if dlo < dhi { self.lo } else { self.hi }
    }

    /// Returns an interval expanded on each side by `margin`.
    ///
    /// If `margin` is negative, the interval is shrunk instead. The resulting
    /// interval may be empty or full. Any expansion (positive or negative) of
    /// a full interval remains full, and any expansion of an empty interval
    /// remains empty.
    pub fn expanded(self, margin: f64) -> Interval {
        if margin >= 0.0 {
            if self.is_empty() {
                return self;
            }
            // Check whether this interval will be full after expansion, allowing
            // for a 1-bit rounding error when computing each endpoint.
            if self.length() + 2.0 * margin + 2.0 * f64::EPSILON >= 2.0 * PI {
                return Interval::full();
            }
        } else {
            if self.is_full() {
                return self;
            }
            // Check whether this interval will be empty after expansion, allowing
            // for a 1-bit rounding error when computing each endpoint.
            if self.length() + 2.0 * margin - 2.0 * f64::EPSILON <= 0.0 {
                return Interval::empty();
            }
        }
        let mut result = Interval::new(
            f64::rem_euclid_workaround(self.lo - margin, 2.0 * PI),
            f64::rem_euclid_workaround(self.hi + margin, 2.0 * PI),
        );
        if result.lo <= -PI {
            result.lo = PI;
        }
        result
    }

    /// Returns the smallest interval that contains both this interval and `y`.
    pub fn union(self, y: Interval) -> Interval {
        if y.is_empty() {
            return self;
        }
        if self.fast_contains(y.lo) {
            if self.fast_contains(y.hi) {
                if self.contains_interval(y) {
                    return self;
                }
                return Interval::full();
            }
            return Interval::new_checked(self.lo, y.hi);
        }
        if self.fast_contains(y.hi) {
            return Interval::new_checked(y.lo, self.hi);
        }
        // This interval contains neither endpoint of y.
        if self.is_empty() || y.fast_contains(self.lo) {
            return y;
        }
        // Check which pair of endpoints are closer together.
        let dlo = positive_distance(y.hi, self.lo);
        let dhi = positive_distance(self.hi, y.lo);
        if dlo < dhi {
            Interval::new_checked(y.lo, self.hi)
        } else {
            Interval::new_checked(self.lo, y.hi)
        }
    }

    /// Returns the smallest interval that contains the intersection of this
    /// interval with `y`. The region of intersection may consist of two
    /// disjoint subintervals.
    pub fn intersection(self, y: Interval) -> Interval {
        if y.is_empty() {
            return Interval::empty();
        }
        if self.fast_contains(y.lo) {
            if self.fast_contains(y.hi) {
                // Either this interval contains y, or the intersection is two
                // disjoint pieces. Return the shorter of the two originals.
                if y.length() < self.length() {
                    return y;
                }
                return self;
            }
            return Interval::new_checked(y.lo, self.hi);
        }
        if self.fast_contains(y.hi) {
            return Interval::new_checked(self.lo, y.hi);
        }
        // This interval contains neither endpoint of y.
        if y.fast_contains(self.lo) {
            return self;
        }
        debug_assert!(!self.intersects(y));
        Interval::empty()
    }

    /// Reports whether this interval can be transformed into `y` by moving
    /// each endpoint by at most `max_error` (without crossing).
    ///
    /// Empty and full intervals are considered to start at an arbitrary point,
    /// so any interval with length <= 2*`max_error` matches empty, and any
    /// interval with length >= 2π - 2*`max_error` matches full.
    pub fn approx_eq_with(self, y: Interval, max_error: f64) -> bool {
        if self.is_empty() {
            return y.length() <= 2.0 * max_error;
        }
        if y.is_empty() {
            return self.length() <= 2.0 * max_error;
        }
        if self.is_full() {
            return y.length() >= 2.0 * (PI - max_error);
        }
        if y.is_full() {
            return self.length() >= 2.0 * (PI - max_error);
        }
        (ieee_remainder(y.lo - self.lo, 2.0 * PI)).abs() <= max_error
            && (ieee_remainder(y.hi - self.hi, 2.0 * PI)).abs() <= max_error
            && (self.length() - y.length()).abs() <= 2.0 * max_error
    }

    /// Like [`approx_eq_with`](Interval::approx_eq_with) with a default
    /// `max_error` of 1e-15.
    pub fn approx_eq(self, y: Interval) -> bool {
        self.approx_eq_with(y, 1e-15)
    }

    /// Provides indexed access: 0 → lo, 1 → hi.
    ///
    /// # Panics
    /// Panics if `i` is not 0 or 1.
    pub fn bound(self, i: usize) -> f64 {
        [self.lo, self.hi][i]
    }

    /// Reports whether the interval contains `p`. Assumes `p` is already
    /// normalized (i.e., not -π).
    fn fast_contains(self, p: f64) -> bool {
        if self.is_inverted() {
            (p >= self.lo || p <= self.hi) && !self.is_empty()
        } else {
            p >= self.lo && p <= self.hi
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
        self.lo == other.lo && self.hi == other.hi
    }
}

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lo, self.hi)
    }
}

// --- Helper functions ---

fn is_valid_pair(lo: f64, hi: f64) -> bool {
    lo.abs() <= PI && hi.abs() <= PI && !(lo == -PI && hi != PI) && !(hi == -PI && lo != PI)
}

/// Compute the distance from `a` to `b` in \[0, 2π).
/// Numerically stable (does not lose precision for very small positive
/// distances).
fn positive_distance(a: f64, b: f64) -> f64 {
    let d = b - a;
    if d >= 0.0 {
        return d;
    }
    (b + PI) - (a - PI)
}

/// IEEE 754 remainder (same as C `remainder()`).
/// Rust's `f64::rem_euclid` is NOT the same thing; this is `x - round(x/y)*y`.
fn ieee_remainder(x: f64, y: f64) -> f64 {
    // Rust doesn't expose C's `remainder()` in stable std, but we can use
    // the formula directly. For our use case (small values near 0 mod 2π),
    // this is fine.
    let q = (x / y).round();
    x - q * y
}

// Extension trait for the remainder workaround used in `expanded()`.
trait RemEuclid {
    fn rem_euclid_workaround(x: f64, y: f64) -> f64;
}

impl RemEuclid for f64 {
    /// IEEE 754 remainder, matching C's `remainder()`.
    fn rem_euclid_workaround(x: f64, y: f64) -> f64 {
        ieee_remainder(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn interval_is_send_sync() {
        is_send_sync::<Interval>();
    }

    // Standard test intervals, matching the C++ test fixture.
    fn empty() -> Interval {
        Interval::empty()
    }
    fn full() -> Interval {
        Interval::full()
    }

    // Single-point intervals
    fn zero() -> Interval {
        Interval::new(0.0, 0.0)
    }
    fn pi2() -> Interval {
        Interval::new(FRAC_PI_2, FRAC_PI_2)
    }
    fn pi() -> Interval {
        Interval::new(PI, PI)
    }
    fn mipi() -> Interval {
        Interval::new(-PI, -PI)
    } // normalized to [π, π]
    fn mipi2() -> Interval {
        Interval::new(-FRAC_PI_2, -FRAC_PI_2)
    }

    // Single quadrants
    fn quad1() -> Interval {
        Interval::new(0.0, FRAC_PI_2)
    }
    fn quad2() -> Interval {
        Interval::new(FRAC_PI_2, -PI)
    }
    fn quad3() -> Interval {
        Interval::new(PI, -FRAC_PI_2)
    }
    fn quad4() -> Interval {
        Interval::new(-FRAC_PI_2, 0.0)
    }

    // Quadrant pairs
    fn quad12() -> Interval {
        Interval::new(0.0, -PI)
    }
    fn quad23() -> Interval {
        Interval::new(FRAC_PI_2, -FRAC_PI_2)
    }
    fn quad34() -> Interval {
        Interval::new(-PI, 0.0)
    }
    // Quadrant triples
    fn quad123() -> Interval {
        Interval::new(0.0, -FRAC_PI_2)
    }
    fn quad234() -> Interval {
        Interval::new(FRAC_PI_2, 0.0)
    }
    fn quad341() -> Interval {
        Interval::new(PI, FRAC_PI_2)
    }
    fn quad412() -> Interval {
        Interval::new(-FRAC_PI_2, -PI)
    }

    // Small intervals around quadrant midpoints, offset slightly CCW
    fn mid12() -> Interval {
        Interval::new(FRAC_PI_2 - 0.01, FRAC_PI_2 + 0.02)
    }
    fn mid23() -> Interval {
        Interval::new(PI - 0.01, -PI + 0.02)
    }
    fn mid34() -> Interval {
        Interval::new(-FRAC_PI_2 - 0.01, -FRAC_PI_2 + 0.02)
    }
    fn mid41() -> Interval {
        Interval::new(-0.01, 0.02)
    }

    #[test]
    fn test_constructors_and_accessors() {
        assert_eq!(quad12().lo, 0.0);
        assert_eq!(quad12().hi, PI);
        assert_eq!(quad34().bound(0), PI);
        assert_eq!(quad34().bound(1), 0.0);
        assert_eq!(pi().lo, PI);
        assert_eq!(pi().hi, PI);

        // Check that [-π, -π] is normalized to [π, π].
        assert_eq!(mipi().lo, PI);
        assert_eq!(mipi().hi, PI);
        assert_eq!(quad23().lo, FRAC_PI_2);
        assert_eq!(quad23().hi, -FRAC_PI_2);

        // Default is empty.
        let default_empty: Interval = Interval::default();
        assert!(default_empty.is_valid());
        assert!(default_empty.is_empty());
        assert_eq!(empty().lo, default_empty.lo);
        assert_eq!(empty().hi, default_empty.hi);
    }

    #[test]
    fn test_simple_predicates() {
        assert!(zero().is_valid() && !zero().is_empty() && !zero().is_full());
        assert!(empty().is_valid() && empty().is_empty() && !empty().is_full());
        assert!(empty().is_inverted());
        assert!(full().is_valid() && !full().is_empty() && full().is_full());
        assert!(!quad12().is_empty() && !quad12().is_full() && !quad12().is_inverted());
        assert!(!quad23().is_empty() && !quad23().is_full() && quad23().is_inverted());
        assert!(pi().is_valid() && !pi().is_empty() && !pi().is_inverted());
        assert!(mipi().is_valid() && !mipi().is_empty() && !mipi().is_inverted());
    }

    #[test]
    fn test_almost_empty_or_full() {
        let almost_pi = PI - 2.0 * f64::EPSILON;
        assert!(!Interval::new(-almost_pi, PI).is_full());
        assert!(!Interval::new(-PI, almost_pi).is_full());
        assert!(!Interval::new(PI, -almost_pi).is_empty());
        assert!(!Interval::new(almost_pi, -PI).is_empty());
    }

    #[test]
    fn test_center() {
        assert_eq!(quad12().center(), FRAC_PI_2);
        assert!((Interval::new(3.1, 2.9).center() - (3.0 - PI)).abs() < 1e-15);
        assert!((Interval::new(-2.9, -3.1).center() - (PI - 3.0)).abs() < 1e-15);
        assert!((Interval::new(2.1, -2.1).center() - PI).abs() < 1e-15);
        assert_eq!(pi().center(), PI);
        assert_eq!(mipi().center(), PI);
        assert_eq!(quad23().center().abs(), PI);
        assert!((quad123().center() - 0.75 * PI).abs() < 1e-15);
    }

    #[test]
    fn test_length() {
        assert_eq!(quad12().length(), PI);
        assert_eq!(pi().length(), 0.0);
        assert_eq!(mipi().length(), 0.0);
        assert!((quad123().length() - 1.5 * PI).abs() < 1e-15);
        assert_eq!(quad23().length().abs(), PI);
        assert_eq!(full().length(), 2.0 * PI);
        assert!(empty().length() < 0.0);
    }

    #[test]
    fn test_complement() {
        assert!(empty().complement().is_full());
        assert!(full().complement().is_empty());
        assert!(pi().complement().is_full());
        assert!(mipi().complement().is_full());
        assert!(zero().complement().is_full());
        assert!(quad12().complement().approx_eq(quad34()));
        assert!(quad34().complement().approx_eq(quad12()));
        assert!(quad123().complement().approx_eq(quad4()));
    }

    #[test]
    fn test_contains_point() {
        assert!(!empty().contains(0.0));
        assert!(!empty().contains(PI));
        assert!(!empty().contains(-PI));
        assert!(!empty().interior_contains(PI));
        assert!(!empty().interior_contains(-PI));

        assert!(full().contains(0.0));
        assert!(full().contains(PI));
        assert!(full().contains(-PI));
        assert!(full().interior_contains(PI));
        assert!(full().interior_contains(-PI));

        assert!(quad12().contains(0.0));
        assert!(quad12().contains(PI));
        assert!(quad12().contains(-PI));
        assert!(quad12().interior_contains(FRAC_PI_2));
        assert!(!quad12().interior_contains(0.0));
        assert!(!quad12().interior_contains(PI));
        assert!(!quad12().interior_contains(-PI));

        assert!(quad23().contains(FRAC_PI_2));
        assert!(quad23().contains(-FRAC_PI_2));
        assert!(quad23().contains(PI));
        assert!(quad23().contains(-PI));
        assert!(!quad23().contains(0.0));
        assert!(!quad23().interior_contains(FRAC_PI_2));
        assert!(!quad23().interior_contains(-FRAC_PI_2));
        assert!(quad23().interior_contains(PI));
        assert!(quad23().interior_contains(-PI));
        assert!(!quad23().interior_contains(0.0));

        assert!(pi().contains(PI));
        assert!(pi().contains(-PI));
        assert!(!pi().contains(0.0));
        assert!(!pi().interior_contains(PI));
        assert!(!pi().interior_contains(-PI));

        assert!(mipi().contains(PI));
        assert!(mipi().contains(-PI));
        assert!(!mipi().contains(0.0));
        assert!(!mipi().interior_contains(PI));
        assert!(!mipi().interior_contains(-PI));

        assert!(zero().contains(0.0));
        assert!(!zero().interior_contains(0.0));
    }

    /// Test helper matching C++ `TestIntervalOps`.
    fn test_interval_ops(
        x: Interval,
        y: Interval,
        expected: &str,
        expected_union: Interval,
        expected_intersection: Interval,
    ) {
        let exp: Vec<bool> = expected.chars().map(|c| c == 'T').collect();
        assert_eq!(x.contains_interval(y), exp[0], "{x}.contains_interval({y})");
        assert_eq!(
            x.interior_contains_interval(y),
            exp[1],
            "{x}.interior_contains_interval({y})"
        );
        assert_eq!(x.intersects(y), exp[2], "{x}.intersects({y})");
        assert_eq!(
            x.interior_intersects(y),
            exp[3],
            "{x}.interior_intersects({y})"
        );

        let u = x.union(y);
        assert_eq!(u.lo, expected_union.lo, "{x}.union({y}).lo");
        assert_eq!(u.hi, expected_union.hi, "{x}.union({y}).hi");

        let i = x.intersection(y);
        assert_eq!(i.lo, expected_intersection.lo, "{x}.intersection({y}).lo");
        assert_eq!(i.hi, expected_intersection.hi, "{x}.intersection({y}).hi");

        assert_eq!(x.contains_interval(y), x.union(y) == x);
        assert_eq!(x.intersects(y), !x.intersection(y).is_empty());

        if y.lo == y.hi {
            let r = x.add_point(y.lo);
            assert_eq!(r.lo, expected_union.lo, "add_point union.lo");
            assert_eq!(r.hi, expected_union.hi, "add_point union.hi");
        }
    }

    #[test]
    fn test_interval_ops_cases() {
        let e = empty();
        let f = full();

        test_interval_ops(e, e, "TTFF", e, e);
        test_interval_ops(e, f, "FFFF", f, e);
        test_interval_ops(e, zero(), "FFFF", zero(), e);
        test_interval_ops(e, pi(), "FFFF", pi(), e);
        test_interval_ops(e, mipi(), "FFFF", mipi(), e);

        test_interval_ops(f, e, "TTFF", f, e);
        test_interval_ops(f, f, "TTTT", f, f);
        test_interval_ops(f, zero(), "TTTT", f, zero());
        test_interval_ops(f, pi(), "TTTT", f, pi());
        test_interval_ops(f, mipi(), "TTTT", f, mipi());
        test_interval_ops(f, quad12(), "TTTT", f, quad12());
        test_interval_ops(f, quad23(), "TTTT", f, quad23());

        test_interval_ops(zero(), e, "TTFF", zero(), e);
        test_interval_ops(zero(), f, "FFTF", f, zero());
        test_interval_ops(zero(), zero(), "TFTF", zero(), zero());
        test_interval_ops(zero(), pi(), "FFFF", Interval::new(0.0, PI), e);
        test_interval_ops(zero(), pi2(), "FFFF", quad1(), e);
        test_interval_ops(zero(), mipi(), "FFFF", quad12(), e);
        test_interval_ops(zero(), mipi2(), "FFFF", quad4(), e);
        test_interval_ops(zero(), quad12(), "FFTF", quad12(), zero());
        test_interval_ops(zero(), quad23(), "FFFF", quad123(), e);

        test_interval_ops(pi2(), e, "TTFF", pi2(), e);
        test_interval_ops(pi2(), f, "FFTF", f, pi2());
        test_interval_ops(pi2(), zero(), "FFFF", quad1(), e);
        test_interval_ops(pi2(), pi(), "FFFF", Interval::new(FRAC_PI_2, PI), e);
        test_interval_ops(pi2(), pi2(), "TFTF", pi2(), pi2());
        test_interval_ops(pi2(), mipi(), "FFFF", quad2(), e);
        test_interval_ops(pi2(), mipi2(), "FFFF", quad23(), e);
        test_interval_ops(pi2(), quad12(), "FFTF", quad12(), pi2());
        test_interval_ops(pi2(), quad23(), "FFTF", quad23(), pi2());

        test_interval_ops(pi(), e, "TTFF", pi(), e);
        test_interval_ops(pi(), f, "FFTF", f, pi());
        test_interval_ops(pi(), zero(), "FFFF", Interval::new(PI, 0.0), e);
        test_interval_ops(pi(), pi(), "TFTF", pi(), pi());
        test_interval_ops(pi(), pi2(), "FFFF", Interval::new(FRAC_PI_2, PI), e);
        test_interval_ops(pi(), mipi(), "TFTF", pi(), pi());
        test_interval_ops(pi(), mipi2(), "FFFF", quad3(), e);
        test_interval_ops(pi(), quad12(), "FFTF", Interval::new(0.0, PI), pi());
        test_interval_ops(pi(), quad23(), "FFTF", quad23(), pi());

        test_interval_ops(mipi(), e, "TTFF", mipi(), e);
        test_interval_ops(mipi(), f, "FFTF", f, mipi());
        test_interval_ops(mipi(), zero(), "FFFF", quad34(), e);
        test_interval_ops(mipi(), pi(), "TFTF", mipi(), mipi());
        test_interval_ops(mipi(), pi2(), "FFFF", quad2(), e);
        test_interval_ops(mipi(), mipi(), "TFTF", mipi(), mipi());
        test_interval_ops(mipi(), mipi2(), "FFFF", Interval::new(-PI, -FRAC_PI_2), e);
        test_interval_ops(mipi(), quad12(), "FFTF", quad12(), mipi());
        test_interval_ops(mipi(), quad23(), "FFTF", quad23(), mipi());

        test_interval_ops(quad12(), e, "TTFF", quad12(), e);
        test_interval_ops(quad12(), f, "FFTT", f, quad12());
        test_interval_ops(quad12(), zero(), "TFTF", quad12(), zero());
        test_interval_ops(quad12(), pi(), "TFTF", quad12(), pi());
        test_interval_ops(quad12(), mipi(), "TFTF", quad12(), mipi());
        test_interval_ops(quad12(), quad12(), "TFTT", quad12(), quad12());
        test_interval_ops(quad12(), quad23(), "FFTT", quad123(), quad2());
        test_interval_ops(quad12(), quad34(), "FFTF", f, quad12());

        test_interval_ops(quad23(), e, "TTFF", quad23(), e);
        test_interval_ops(quad23(), f, "FFTT", f, quad23());
        test_interval_ops(quad23(), zero(), "FFFF", quad234(), e);
        test_interval_ops(quad23(), pi(), "TTTT", quad23(), pi());
        test_interval_ops(quad23(), mipi(), "TTTT", quad23(), mipi());
        test_interval_ops(quad23(), quad12(), "FFTT", quad123(), quad2());
        test_interval_ops(quad23(), quad23(), "TFTT", quad23(), quad23());
        test_interval_ops(
            quad23(),
            quad34(),
            "FFTT",
            quad234(),
            Interval::new(-PI, -FRAC_PI_2),
        );

        test_interval_ops(
            quad1(),
            quad23(),
            "FFTF",
            quad123(),
            Interval::new(FRAC_PI_2, FRAC_PI_2),
        );
        test_interval_ops(quad2(), quad3(), "FFTF", quad23(), mipi());
        test_interval_ops(quad3(), quad2(), "FFTF", quad23(), pi());
        test_interval_ops(quad2(), pi(), "TFTF", quad2(), pi());
        test_interval_ops(quad2(), mipi(), "TFTF", quad2(), mipi());
        test_interval_ops(quad3(), pi(), "TFTF", quad3(), pi());
        test_interval_ops(quad3(), mipi(), "TFTF", quad3(), mipi());

        test_interval_ops(quad12(), mid12(), "TTTT", quad12(), mid12());
        test_interval_ops(mid12(), quad12(), "FFTT", quad12(), mid12());

        let quad12eps = Interval::new_checked(quad12().lo, mid23().hi);
        let quad2hi = Interval::new_checked(mid23().lo, quad12().hi);
        test_interval_ops(quad12(), mid23(), "FFTT", quad12eps, quad2hi);
        test_interval_ops(mid23(), quad12(), "FFTT", quad12eps, quad2hi);

        let quad412eps = Interval::new_checked(mid34().lo, quad12().hi);
        test_interval_ops(quad12(), mid34(), "FFFF", quad412eps, e);
        test_interval_ops(mid34(), quad12(), "FFFF", quad412eps, e);

        let quadeps12 = Interval::new_checked(mid41().lo, quad12().hi);
        let quad1lo = Interval::new_checked(quad12().lo, mid41().hi);
        test_interval_ops(quad12(), mid41(), "FFTT", quadeps12, quad1lo);
        test_interval_ops(mid41(), quad12(), "FFTT", quadeps12, quad1lo);

        let quad2lo = Interval::new_checked(quad23().lo, mid12().hi);
        let quad3hi = Interval::new_checked(mid34().lo, quad23().hi);
        let quadeps23 = Interval::new_checked(mid12().lo, quad23().hi);
        let quad23eps = Interval::new_checked(quad23().lo, mid34().hi);
        let quadeps123 = Interval::new_checked(mid41().lo, quad23().hi);
        test_interval_ops(quad23(), mid12(), "FFTT", quadeps23, quad2lo);
        test_interval_ops(mid12(), quad23(), "FFTT", quadeps23, quad2lo);
        test_interval_ops(quad23(), mid23(), "TTTT", quad23(), mid23());
        test_interval_ops(mid23(), quad23(), "FFTT", quad23(), mid23());
        test_interval_ops(quad23(), mid34(), "FFTT", quad23eps, quad3hi);
        test_interval_ops(mid34(), quad23(), "FFTT", quad23eps, quad3hi);
        test_interval_ops(quad23(), mid41(), "FFFF", quadeps123, e);
        test_interval_ops(mid41(), quad23(), "FFFF", quadeps123, e);
    }

    #[test]
    fn test_add_point() {
        let mut r;
        r = empty().add_point(0.0);
        assert_eq!(r, zero());
        r = empty().add_point(PI);
        assert_eq!(r, pi());
        r = empty().add_point(-PI);
        assert_eq!(r, mipi());
        r = empty().add_point(PI).add_point(-PI);
        assert_eq!(r, pi());
        r = empty().add_point(-PI).add_point(PI);
        assert_eq!(r, mipi());
        r = empty().add_point(mid12().lo).add_point(mid12().hi);
        assert_eq!(r, mid12());
        r = empty().add_point(mid23().lo).add_point(mid23().hi);
        assert_eq!(r, mid23());
        r = quad1().add_point(-0.9 * PI).add_point(-FRAC_PI_2);
        assert_eq!(r, quad123());
        r = full().add_point(0.0);
        assert!(r.is_full());
        r = full().add_point(PI);
        assert!(r.is_full());
        r = full().add_point(-PI);
        assert!(r.is_full());
    }

    #[test]
    fn test_project() {
        let r = Interval::new(-PI, -PI);
        assert_eq!(r.project(-PI), PI);
        assert_eq!(r.project(0.0), PI);

        let r = Interval::new(0.0, PI);
        assert_eq!(r.project(0.1), 0.1);
        assert_eq!(r.project(-FRAC_PI_2 + 1e-15), 0.0);
        assert_eq!(r.project(-FRAC_PI_2 - 1e-15), PI);

        let r = Interval::new(PI - 0.1, -PI + 0.1);
        assert_eq!(r.project(PI), PI);
        assert_eq!(r.project(1e-15), PI - 0.1);
        assert_eq!(r.project(-1e-15), -PI + 0.1);

        assert_eq!(Interval::full().project(0.0), 0.0);
        assert_eq!(Interval::full().project(PI), PI);
        assert_eq!(Interval::full().project(-PI), PI);
    }

    #[test]
    fn test_from_point_pair() {
        assert_eq!(Interval::from_point_pair(-PI, PI), pi());
        assert_eq!(Interval::from_point_pair(PI, -PI), pi());
        assert_eq!(Interval::from_point_pair(mid34().hi, mid34().lo), mid34());
        assert_eq!(Interval::from_point_pair(mid23().lo, mid23().hi), mid23());
    }

    #[test]
    fn test_expanded() {
        assert_eq!(empty().expanded(1.0), empty());
        assert_eq!(full().expanded(1.0), full());
        assert_eq!(zero().expanded(1.0), Interval::new(-1.0, 1.0));
        assert_eq!(mipi().expanded(0.01), Interval::new(PI - 0.01, -PI + 0.01));
        assert_eq!(pi().expanded(27.0), full());
        assert_eq!(pi().expanded(FRAC_PI_2), quad23());
        assert_eq!(pi2().expanded(FRAC_PI_2), quad12());
        assert_eq!(mipi2().expanded(FRAC_PI_2), quad34());

        assert_eq!(empty().expanded(-1.0), empty());
        assert_eq!(full().expanded(-1.0), full());
        assert_eq!(quad123().expanded(-27.0), empty());
        assert_eq!(quad234().expanded(-27.0), empty());
        assert_eq!(quad123().expanded(-FRAC_PI_2), quad2());
        assert_eq!(quad341().expanded(-FRAC_PI_2), quad4());
        assert_eq!(quad412().expanded(-FRAC_PI_2), quad1());
    }

    #[test]
    fn test_approx_equals() {
        let lo = 4.0 * f64::EPSILON;
        let hi = 6.0 * f64::EPSILON;

        // Empty intervals.
        assert!(empty().approx_eq(empty()));
        assert!(zero().approx_eq(empty()));
        assert!(empty().approx_eq(zero()));
        assert!(pi().approx_eq(empty()));
        assert!(empty().approx_eq(pi()));
        assert!(mipi().approx_eq(empty()));
        assert!(empty().approx_eq(mipi()));
        assert!(!empty().approx_eq(full()));
        assert!(empty().approx_eq(Interval::new(1.0, 1.0 + 2.0 * lo)));
        assert!(!empty().approx_eq(Interval::new(1.0, 1.0 + 2.0 * hi)));
        assert!(Interval::new(PI - lo, -PI + lo).approx_eq(empty()));

        // Full intervals.
        assert!(full().approx_eq(full()));
        assert!(!full().approx_eq(empty()));
        assert!(!full().approx_eq(zero()));
        assert!(!full().approx_eq(pi()));
        assert!(full().approx_eq(Interval::new(lo, -lo)));
        assert!(!full().approx_eq(Interval::new(2.0 * hi, 0.0)));
        assert!(Interval::new(-PI + lo, PI - lo).approx_eq(full()));
        assert!(!Interval::new(-PI, PI - 2.0 * hi).approx_eq(full()));

        // Singleton intervals.
        assert!(pi().approx_eq(pi()));
        assert!(mipi().approx_eq(pi()));
        assert!(pi().approx_eq(Interval::new(PI - lo, PI - lo)));
        assert!(!pi().approx_eq(Interval::new(PI - hi, PI - hi)));
        assert!(pi().approx_eq(Interval::new(PI - lo, -PI + lo)));
        assert!(!pi().approx_eq(Interval::new(PI - hi, -PI)));
        assert!(!zero().approx_eq(pi()));
        assert!(pi().union(mid12()).union(zero()).approx_eq(quad12()));
        assert!(quad2().intersection(quad3()).approx_eq(pi()));
        assert!(quad3().intersection(quad2()).approx_eq(pi()));

        // Intervals whose endpoints are in opposite order (inverted).
        assert!(!Interval::new(0.0, lo).approx_eq(Interval::new(lo, 0.0)));
        assert!(
            !Interval::new(PI - 0.5 * lo, -PI + 0.5 * lo)
                .approx_eq(Interval::new(-PI + 0.5 * lo, PI - 0.5 * lo))
        );

        // Other intervals.
        assert!(Interval::new(1.0 - lo, 2.0 + lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(Interval::new(1.0 + lo, 2.0 - lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(Interval::new(2.0 - lo, 1.0 + lo).approx_eq(Interval::new(2.0, 1.0)));
        assert!(Interval::new(2.0 + lo, 1.0 - lo).approx_eq(Interval::new(2.0, 1.0)));
        assert!(!Interval::new(1.0 - hi, 2.0 + lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 + hi, 2.0 - lo).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(2.0 - hi, 1.0 + lo).approx_eq(Interval::new(2.0, 1.0)));
        assert!(!Interval::new(2.0 + hi, 1.0 - lo).approx_eq(Interval::new(2.0, 1.0)));
        assert!(!Interval::new(1.0 - lo, 2.0 + hi).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(1.0 + lo, 2.0 - hi).approx_eq(Interval::new(1.0, 2.0)));
        assert!(!Interval::new(2.0 - lo, 1.0 + hi).approx_eq(Interval::new(2.0, 1.0)));
        assert!(!Interval::new(2.0 + lo, 1.0 - hi).approx_eq(Interval::new(2.0, 1.0)));
    }

    #[test]
    fn test_operator_equals() {
        assert_eq!(empty(), empty());
        assert_eq!(full(), full());
        assert_ne!(full(), empty());
    }

    #[test]
    fn test_directed_hausdorff_distance() {
        assert!((empty().directed_hausdorff_distance(empty())).abs() < 1e-6);
        assert!((empty().directed_hausdorff_distance(mid12())).abs() < 1e-6);
        assert!((mid12().directed_hausdorff_distance(empty()) - PI).abs() < 1e-6);

        assert_eq!(quad12().directed_hausdorff_distance(quad123()), 0.0);

        let inv = Interval::new(3.0, -3.0); // complement center is 0
        assert!((Interval::new(-0.1, 0.2).directed_hausdorff_distance(inv) - 3.0).abs() < 1e-6);
        assert!(
            (Interval::new(0.1, 0.2).directed_hausdorff_distance(inv) - (3.0 - 0.1)).abs() < 1e-6
        );
        assert!(
            (Interval::new(-0.2, -0.1).directed_hausdorff_distance(inv) - (3.0 - 0.1)).abs() < 1e-6
        );
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck_macros::quickcheck;
    use std::f64::consts::PI;

    /// Clamp to a valid S1 angle in (-π, π], with -π normalized to π.
    fn normalize(x: f64) -> f64 {
        if !x.is_finite() {
            return 0.0;
        }
        // Clamp to a range where ieee_remainder is numerically stable.
        // For very large values, the remainder is unreliable.
        let x = x.clamp(-1e8, 1e8);
        let mut x = ieee_remainder(x, 2.0 * PI);
        // ieee_remainder can produce values very slightly outside [-PI, PI].
        x = x.clamp(-PI, PI);
        // Avoid producing exactly -PI (normalized to PI).
        if x == -PI {
            x = PI;
        }
        x
    }

    /// Build a valid non-empty S1 interval from two arbitrary floats.
    fn make_interval(a: f64, b: f64) -> Interval {
        let a = normalize(a);
        let b = normalize(b);
        Interval::new(a, b)
    }

    #[quickcheck]
    fn prop_complement_complement(a: f64, b: f64) -> bool {
        let i = make_interval(a, b);
        // complement(complement(i)) ≈ i (except singletons become full → empty)
        if i.lo == i.hi {
            // Singleton: complement is full, complement(full) is empty.
            return i.complement().complement().is_empty();
        }
        let cc = i.complement().complement();
        cc.approx_eq_with(i, 1e-14)
    }

    #[quickcheck]
    fn prop_union_contains_both(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let u = a.union(b);
        u.contains_interval(a) && u.contains_interval(b)
    }

    #[quickcheck]
    fn prop_full_contains_any_point(p: f64) -> bool {
        let p = normalize(p);
        Interval::full().contains(p)
    }

    #[quickcheck]
    fn prop_empty_contains_nothing(p: f64) -> bool {
        let p = normalize(p);
        !Interval::empty().contains(p)
    }

    #[quickcheck]
    fn prop_length_non_negative_for_non_empty(a: f64, b: f64) -> bool {
        let i = make_interval(a, b);
        if i.is_empty() {
            i.length() < 0.0
        } else {
            i.length() >= 0.0
        }
    }

    #[quickcheck]
    fn prop_full_is_full() -> bool {
        Interval::full().is_full() && !Interval::full().is_empty()
    }

    #[quickcheck]
    fn prop_empty_is_empty() -> bool {
        Interval::empty().is_empty() && !Interval::empty().is_full()
    }

    #[quickcheck]
    fn prop_add_point_contains(a1: f64, a2: f64, p: f64) -> bool {
        let i = make_interval(a1, a2);
        let p = normalize(p);
        i.add_point(p).contains(p)
    }

    #[quickcheck]
    fn prop_intersection_empty_iff_no_intersects(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let i = a.intersection(b);
        // For S1 intervals, intersection may return a superset of the true
        // intersection (when the result is two disjoint arcs). But the
        // empty/non-empty invariant holds.
        a.intersects(b) != i.is_empty()
    }

    #[quickcheck]
    fn prop_union_is_commutative(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        let ab = a.union(b);
        let ba = b.union(a);
        // When a point is equidistant from both endpoints, union may pick
        // different (but equally valid) intervals depending on argument order.
        // Both results must contain both inputs and have the same length.
        ab.contains_interval(a)
            && ab.contains_interval(b)
            && ba.contains_interval(a)
            && ba.contains_interval(b)
            && (ab.length() - ba.length()).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_expanded_contains_original(a1: f64, a2: f64, margin: f64) -> bool {
        let a = make_interval(a1, a2);
        let margin = if margin.is_finite() {
            margin.abs().min(1.0)
        } else {
            0.1
        };
        let expanded = a.expanded(margin);
        if a.is_empty() {
            return expanded.is_empty();
        }
        expanded.contains_interval(a)
    }

    #[quickcheck]
    fn prop_contains_implies_union_unchanged(a1: f64, a2: f64, b1: f64, b2: f64) -> bool {
        let a = make_interval(a1, a2);
        let b = make_interval(b1, b2);
        // If a contains b, then union(a, b) == a.
        if a.contains_interval(b) {
            a.union(b) == a
        } else {
            true
        }
    }

    #[quickcheck]
    fn prop_project_in_interval(a1: f64, a2: f64, p: f64) -> bool {
        let a = make_interval(a1, a2);
        if a.is_empty() {
            return true;
        }
        let p = normalize(p);
        a.contains(a.project(p))
    }

    #[quickcheck]
    fn prop_complement_length(a: f64, b: f64) -> bool {
        let i = make_interval(a, b);
        if i.lo == i.hi {
            // Singleton → complement is full (length 2π).
            return (i.complement().length() - 2.0 * PI).abs() < 1e-14;
        }
        if i.is_full() || i.is_empty() {
            return true;
        }
        // length(i) + length(complement(i)) ≈ 2π
        (i.length() + i.complement().length() - 2.0 * PI).abs() < 1e-14
    }

    #[quickcheck]
    fn prop_from_point_contains(p: f64) -> bool {
        let p = normalize(p);
        let i = Interval::from_point(p);
        i.contains(p) && !i.is_empty() && i.lo == i.hi
    }

    #[cfg(feature = "serde")]
    #[quickcheck]
    fn prop_serde_roundtrip(a: i32, b: i32) -> bool {
        let i = make_interval(f64::from(a), f64::from(b));
        let json1 = serde_json::to_string(&i).unwrap();
        let back: Interval = serde_json::from_str(&json1).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        let back2: Interval = serde_json::from_str(&json2).unwrap();
        back == back2
    }
}
