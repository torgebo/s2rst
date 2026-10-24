// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::Interval`] fixes found during the formal
//! verification of the value types (specs/004-formal-verify-cell-interval). Written for this
//! crate, not ported from upstream S2.

use super::*;
use crate::r1;
use crate::s2::Rect;

/// The one-ulp interval across the seam, `[π, next_up(−π)]` (spec 004 K4).
fn corner() -> Interval {
    Interval::from_point_pair(PI, (-PI).next_up())
}

/// One ulp of π, `2^−51`: the corner's exact length.
const ULP_PI: f64 = 4.440_892_098_500_626e-16;

/// FR-020: the corner's length is its true length, one ulp of π.
///
/// Upstream's `S1Interval::GetLength` (`s1interval.cc:41–47`) computes `hi − lo`, which rounds
/// to exactly `−2π` here, adds `2π` and returns −1, the empty interval's length (U-007).
#[test]
fn seam_corner_length_is_one_ulp() {
    let x = corner();
    assert!(x.is_valid() && !x.is_empty() && x.is_inverted());
    assert_eq!(x.lo, PI);
    assert_eq!(x.length().to_bits(), ULP_PI.to_bits());
    assert_eq!(ULP_PI.to_bits(), 0x3CC0_0000_0000_0000);
}

/// FR-020: expanding the corner by 3.2 on each side covers the circle. With the length −1, the
/// full test failed and the result was a short arc around 0, on the other side of the circle.
#[test]
fn seam_corner_expands_to_full() {
    assert!(corner().expanded(3.2).is_full());
}

/// FR-020: at zero error the corner is not approximately the empty interval. With the length
/// −1 it was (`−1 ≤ 2 · 0`).
#[test]
fn seam_corner_is_not_approximately_empty() {
    assert!(!Interval::empty().approx_eq_with(corner(), 0.0));
    assert!(!corner().approx_eq_with(Interval::empty(), 0.0));
}

/// FR-020: a rectangle with the corner as its longitude range has a positive area and size.
/// With the length −1 its area was −0.0998 for latitudes [0, 0.1].
#[test]
fn seam_corner_rect_has_positive_area() {
    let rect = Rect::new(r1::Interval::new(0.0, 0.1), corner());
    assert!(rect.area() > 0.0, "area {}", rect.area());
    assert_eq!(rect.size().lng.radians().to_bits(), ULP_PI.to_bits());
}

/// FR-020 is narrow: the neighbouring two-ulp interval keeps upstream's arithmetic, bit for
/// bit (P-IMEA-01 proves this for every valid interval other than the corner).
#[test]
fn seam_neighbour_keeps_upstream_length() {
    let x = Interval::new(PI, (-PI).next_up().next_up());
    let upstream = (x.hi - x.lo) + 2.0 * PI;
    assert_eq!(x.length().to_bits(), upstream.to_bits());
    assert_eq!(x.length(), 2.0 * ULP_PI);
}

/// D-005: the directed Hausdorff distance to a short interval near 0 does not trip the debug
/// check in `new_checked`.
///
/// The complement's centre of `[0, 1e−17]` rounds to exactly −π. Upstream builds its helper
/// arcs with the normalising constructor `S1Interval(lo, hi)` (`s1interval.cc:155–158`), which
/// turns that −π into π; the port used `new_checked`, which does not, and failed its validity
/// check in debug builds. Release builds returned the right distance.
#[test]
fn directed_hausdorff_distance_to_a_short_interval_near_zero() {
    let x = Interval::new(1.0, 2.0);
    let y = Interval::new(0.0, 1e-17);
    assert_eq!(y.complement_center(), -PI);
    assert_eq!(x.directed_hausdorff_distance(y), 2.0);
}
