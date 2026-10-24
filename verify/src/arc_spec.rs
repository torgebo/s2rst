// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a circular interval (spec 004 data-model §1.2): the set of points
//! on the circle that an interval denotes.
//!
//! Written from upstream's `S1Interval` header (`s1interval.h`, `v0.14.0-40`): an interval is a
//! closed arc of the unit circle from `lo` to `hi`, counter-clockwise. Its bounds lie in
//! [−π, π], and π and −π are one point. The full interval is `[−π, π]` and the empty one
//! `[π, −π]`, the only intervals in which −π may appear. An interval with `lo > hi` is
//! *inverted*: it wraps across the point ±π. Points are `f64` values in [−π, π], so an arc
//! denotes a set of floats.
//!
//! Membership is a property of float points. The relations between intervals are those of
//! real arcs, decided by the cyclic order of the bounds with exact float comparisons ([`ccw_le`]),
//! because a gap between two adjacent floats is still a gap (see [`contains`]).
//!
//! The description reads an [`Interval`]'s two public fields and never calls its methods
//! (FR-007). It also restates two pieces of upstream arithmetic on purpose:
//! [`upstream_length`], the statement of P-IMEA-01 (spec 004 plan, Complexity Tracking), and
//! [`upstream_expanded`], the statement of P-IEXP-03 (FR-023).

use s2rst::s1::Interval;
use std::f64::consts::PI;

/// A point argument's documented domain: `−π ≤ p ≤ π` (false for NaN).
#[must_use]
pub fn in_range(p: f64) -> bool {
    -PI <= p && p <= PI
}

/// The canonical form of a point: −π and π are one point, written π.
#[must_use]
pub fn canon(p: f64) -> f64 {
    if p == -PI { PI } else { p }
}

/// Interval validity (FR-005): both bounds in [−π, π], and −π only in the full interval
/// `[−π, π]` (as its low bound) or the empty interval `[π, −π]` (as its high bound).
#[must_use]
pub fn valid(lo: f64, hi: f64) -> bool {
    in_range(lo) && in_range(hi) && (lo != -PI || hi == PI) && (hi != -PI || lo == PI)
}

/// The empty interval, `[π, −π]`.
#[must_use]
pub fn empty(x: Interval) -> bool {
    x.lo == PI && x.hi == -PI
}

/// The full interval, `[−π, π]`.
#[must_use]
pub fn full(x: Interval) -> bool {
    x.lo == -PI && x.hi == PI
}

/// An inverted interval wraps across ±π: `lo > hi`. The empty interval is inverted.
#[must_use]
pub fn inverted(x: Interval) -> bool {
    x.lo > x.hi
}

/// Whether the closed arc `x` contains the point `p` (in range).
#[must_use]
pub fn member(x: Interval, p: f64) -> bool {
    let q = canon(p);
    if inverted(x) {
        (q >= x.lo || q <= x.hi) && !empty(x)
    } else {
        x.lo <= q && q <= x.hi
    }
}

/// Whether `p` lies in the interior of `x`: the open arc, and every point for the full
/// interval (whose interior is the whole circle).
#[must_use]
pub fn interior(x: Interval, p: f64) -> bool {
    if full(x) {
        return true;
    }
    let q = canon(p);
    if inverted(x) {
        q > x.lo || q < x.hi
    } else {
        x.lo < q && q < x.hi
    }
}

/// Whether `q` comes no later than `r` on the circle, walking counter-clockwise from `a`: the
/// exact real distances satisfy `L(a, q) ≤ L(a, r)`, with `L ∈ [0, 2π)`. Points at or after
/// `a` in float order come first, by value, then those before it, by value. The points are
/// taken in canonical form, and only float comparisons and boolean equality are used: no
/// distance is computed, and booleans are never ordered (Kani 0.68.0 mis-models `<` and `<=`
/// on symbolic `bool`s; spec 004 `values/EVALUATION.md`).
#[must_use]
pub fn ccw_le(a: f64, q: f64, r: f64) -> bool {
    let (a, q, r) = (canon(a), canon(q), canon(r));
    let (q_wraps, r_wraps) = (q < a, r < a);
    if q_wraps == r_wraps { q <= r } else { r_wraps }
}

/// Whether `q` comes strictly before `r`, counter-clockwise from `a` (as [`ccw_le`]).
#[must_use]
pub fn ccw_lt(a: f64, q: f64, r: f64) -> bool {
    let (a, q, r) = (canon(a), canon(q), canon(r));
    let (q_wraps, r_wraps) = (q < a, r < a);
    if q_wraps == r_wraps { q < r } else { r_wraps }
}

/// Whether the closed arc `x` contains the closed arc `y`, as arcs of the real circle.
///
/// Upstream's arcs are real arcs with float bounds. A float-free gap still counts:
/// `[next_up(−π), π]` contains every float, but not the full circle, because the reals strictly
/// between −π and `next_up(−π)` are outside it. So containment cannot be read off the float
/// points. For non-empty `y` and non-full `x`: walking counter-clockwise from `x.lo`, `y` must
/// start no later than it ends, and end no later than `x` does.
#[must_use]
pub fn contains(x: Interval, y: Interval) -> bool {
    if empty(y) || full(x) {
        return true;
    }
    if empty(x) || full(y) {
        return false;
    }
    ccw_le(x.lo, y.lo, y.hi) && ccw_le(x.lo, y.hi, x.hi)
}

/// Whether `y` lies in the interior of `x`: the open real arc, or the whole circle for the full
/// interval.
#[must_use]
pub fn interior_contains(x: Interval, y: Interval) -> bool {
    if empty(y) || full(x) {
        return true;
    }
    if empty(x) || full(y) {
        return false;
    }
    ccw_lt(x.lo, x.lo, y.lo) && ccw_le(x.lo, y.lo, y.hi) && ccw_lt(x.lo, y.hi, x.hi)
}

/// Whether two closed arcs meet: one contains a bound of the other.
#[must_use]
pub fn intersects(x: Interval, y: Interval) -> bool {
    [x.lo, x.hi, y.lo, y.hi]
        .iter()
        .any(|&p| member(x, p) && member(y, p))
}

/// Whether the closed arc `y` meets the interior of `x`. The interior of a single point is
/// empty, and that of the full interval is the whole circle. Otherwise `y` meets the open arc
/// `(x.lo, x.hi)` exactly when it starts inside it, or contains `x.lo` and runs on past it
/// (it is not the single point `x.lo`'s end).
#[must_use]
pub fn interior_intersects(x: Interval, y: Interval) -> bool {
    if empty(x) || empty(y) || (x.lo == x.hi) {
        return false;
    }
    if full(x) || full(y) {
        return true;
    }
    interior(x, y.lo) || (member(y, x.lo) && canon(y.hi) != canon(x.lo))
}

/// Upstream's `S1Interval::GetLength` (`s1interval.cc:41–47`), restated in its operation
/// order, as the statement of P-IMEA-01:
///
/// ```text
/// double length = hi() - lo();
/// if (length >= 0) return length;
/// length += 2 * M_PI;
/// return (length > 0) ? length : -1;   // Empty intervals have a negative length.
/// ```
#[must_use]
pub fn upstream_length(x: Interval) -> f64 {
    let length = x.hi - x.lo;
    if length >= 0.0 {
        return length;
    }
    let length = length + 2.0 * PI;
    if length > 0.0 { length } else { -1.0 }
}

/// Upstream's `S1Interval::Expanded` (`s1interval.cc:208–224`), restated in its operation
/// order, as the statement of P-IEXP-03 (FR-023):
///
/// ```text
/// if (margin >= 0) {
///   if (is_empty()) return *this;
///   if (GetLength() + 2 * margin + 2 * DBL_EPSILON >= 2 * M_PI) return Full();
/// } else {
///   if (is_full()) return *this;
///   if (GetLength() + 2 * margin - 2 * DBL_EPSILON <= 0) return Empty();
/// }
/// S1Interval result(remainder(lo() - margin, 2*M_PI), remainder(hi() + margin, 2*M_PI));
/// if (result.lo() <= -M_PI) result.set_lo(M_PI);
/// return result;
/// ```
///
/// Two pieces follow the library, not C++:
/// - the length is the one the library documents: upstream's, except the corner's true length
///   (FR-020), which the expansion follows by that requirement;
/// - `remainder` is the library's `x − round(x / y)·y`. It agrees with C's `remainder` on the
///   arguments that reach it, `|x| < 2π`: the length tests return early for every larger
///   margin, and the subtraction of `±2π` is exact there (Sterbenz). At the tie `x = ±π` the
///   two pick opposite ends of the seam, which the constructor makes one point.
///
/// The constructor's normalisation is restated as upstream writes it, on the original bounds.
#[must_use]
pub fn upstream_expanded(x: Interval, margin: f64) -> Interval {
    let length = if is_corner(x) {
        ULP_PI
    } else {
        upstream_length(x)
    };
    if margin >= 0.0 {
        if empty(x) {
            return x;
        }
        if length + 2.0 * margin + 2.0 * f64::EPSILON >= 2.0 * PI {
            return Interval { lo: -PI, hi: PI };
        }
    } else {
        if full(x) {
            return x;
        }
        if length + 2.0 * margin - 2.0 * f64::EPSILON <= 0.0 {
            return Interval { lo: PI, hi: -PI };
        }
    }
    let remainder = |a: f64, b: f64| a - (a / b).round() * b;
    let (lo, hi) = (
        remainder(x.lo - margin, 2.0 * PI),
        remainder(x.hi + margin, 2.0 * PI),
    );
    let mut result = Interval {
        lo: if lo == -PI && hi != PI { PI } else { lo },
        hi: if hi == -PI && lo != PI { PI } else { hi },
    };
    if result.lo <= -PI {
        result.lo = PI;
    }
    result
}

/// The K4 corner's high bound: the float just above −π.
#[must_use]
pub fn corner_hi() -> f64 {
    f64::from_bits((-PI).to_bits() - 1)
}

/// `DEV_SEAM_LENGTH` (FR-020): whether `x` is the one-ulp interval across the seam,
/// `[π, next_up(−π)]`, the only valid, non-empty interval whose length upstream's arithmetic
/// gets wrong (P-IMEA-03). The library deviates from upstream there and only there.
#[must_use]
pub fn is_corner(x: Interval) -> bool {
    x.lo == PI && x.hi.to_bits() == corner_hi().to_bits()
}

/// One ulp of π, `2^−51`: the corner's exact length.
pub const ULP_PI: f64 = f64::from_bits(0x3CC0_0000_0000_0000);

/// The distance between two finite floats in units in the last place: the number of floats
/// from one to the other, counting `−0.0` and `0.0` as adjacent.
#[must_use]
pub fn ulps_apart(a: f64, b: f64) -> u64 {
    // Map the bit patterns onto a monotone integer line: negatives below zero.
    fn ordered(x: f64) -> i64 {
        let bits = x.to_bits() as i64;
        if bits < 0 {
            -(bits & i64::MAX) - 1
        } else {
            bits
        }
    }
    ordered(a).abs_diff(ordered(b))
}

#[cfg(test)]
#[path = "arc_spec_tests.rs"]
mod arc_spec_tests;
