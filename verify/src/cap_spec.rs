// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of a spherical cap (spec 008 data-model §1; research R3).
//!
//! Written from upstream's `s2cap.{h,cc}` and `s2metrics.h` (`v0.14.0-40`). A cap is a centre on
//! the unit sphere and a radius, a chord angle: it is the set of points whose squared chord
//! distance from the centre is at most the radius's squared length. The empty cap has the
//! radius `Negative()` (−1) and contains nothing; the full cap has the radius `Straight()` (4)
//! and contains every point. Upstream's own membership test is that comparison
//! (`S1ChordAngle(center, p) <= radius`, `s2cap.cc` `Contains(S2Point)`), so the description
//! recomputes the squared chord itself, as upstream's `S1ChordAngle(a, b)` defines it, and
//! compares.
//!
//! The description reads a cap through `center()` and `chord_radius()` only; it never calls the
//! operations it describes (FR-007). Booleans are combined with `&&`, `||` and `==` only (spec 004
//! research R13); no `%` on floats (spec 007 research R0).
//!
//! The last section restates upstream's arithmetic (`upstream_*`). It is not the meaning: it
//! says what upstream computes, so that a property can state "bit for bit upstream's", and it is
//! checked against the C++ vectors (`tests/values_spec_vs_cpp.rs`).

use s2rst::r3::Vector;
use s2rst::s2::{Cap, Point};
use std::f64::consts::{PI, SQRT_2};

/// Upstream's `S1ChordAngle(a, b)`: the squared length of the chord from `a` to `b`, at most 4
/// (`s1chord_angle.h`, the two-point constructor).
pub fn chord2(a: Point, b: Point) -> f64 {
    let d = a.0 - b.0;
    (d.x * d.x + d.y * d.y + d.z * d.z).min(4.0)
}

/// The radius's squared length.
pub fn radius2(cap: Cap) -> f64 {
    cap.chord_radius().length2()
}

/// Upstream's `is_empty`: a negative radius.
pub fn empty(cap: Cap) -> bool {
    radius2(cap) < 0.0
}

/// Upstream's `is_full`: the radius `Straight()`, length² 4.
pub fn full(cap: Cap) -> bool {
    radius2(cap) == 4.0
}

/// Upstream's `is_valid`: a unit-length centre (`S2::IsUnitLength`, the library's
/// `Point::is_unit`, spec 006) and a radius of length² at most 4.
pub fn valid(cap: Cap) -> bool {
    cap.center().is_unit() && radius2(cap) <= 4.0
}

/// The cap contains `p`: the full cap every point, the empty cap none, otherwise the squared
/// chord from the centre is at most the radius's.
pub fn member(cap: Cap, p: Point) -> bool {
    full(cap) || (!empty(cap) && chord2(cap.center(), p) <= radius2(cap))
}

/// The cap's interior contains `p`: the full cap every point, otherwise the squared chord is
/// below the radius's (so a cap of radius 0 and the empty cap have no interior).
pub fn interior_member(cap: Cap, p: Point) -> bool {
    full(cap) || chord2(cap.center(), p) < radius2(cap)
}

// ---- restatements of upstream's arithmetic -------------------------------------------------------

/// `S2Cap::height()`: `0.5 · length²`.
pub fn upstream_height(l: f64) -> f64 {
    0.5 * l
}

/// `S2Cap::GetArea()`: `2π · max(0, height)`, with C++'s `std::max` (0 for NaN).
pub fn upstream_area(l: f64) -> f64 {
    let h = upstream_height(l);
    2.0 * PI * if 0.0 < h { h } else { 0.0 }
}

/// `S2Cap::GetCentroid()`: the origin for an empty cap; otherwise `(r · area) · centre` with
/// `r = 1 − 0.5 · height`.
pub fn upstream_centroid(center: Point, l: f64) -> Point {
    if l < 0.0 {
        return Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    }
    let r = 1.0 - 0.5 * upstream_height(l);
    let s = r * upstream_area(l);
    Point(Vector {
        x: center.0.x * s,
        y: center.0.y * s,
        z: center.0.z * s,
    })
}

/// Upstream's `S1ChordAngle::FromLength2(x)`: `std::min(4.0, x)`, so a value above 4 and NaN
/// give 4 and a negative value stays as it is. The library maps every negative to −1 instead
/// (spec 007's option B, a documented deviation); a property comparing the library with these
/// restatements states that difference.
fn from_length2(x: f64) -> f64 {
    if x < 4.0 { x } else { 4.0 }
}

/// `S2Cap::FromCenterHeight`: the radius `FromLength2(2 h)`.
pub fn upstream_from_height(h: f64) -> f64 {
    from_length2(2.0 * h)
}

/// `S2Cap::FromCenterArea`: the radius `FromLength2(area / π)`.
pub fn upstream_from_area(a: f64) -> f64 {
    from_length2(a / PI)
}

/// `S2Cap::Complement()`'s radius for a cap neither empty nor full: `FromLength2(4 − l)`.
pub fn upstream_complement_length2(l: f64) -> f64 {
    from_length2(4.0 - l)
}

/// `S2Cap::AddCap`'s round-up: `(2 ε + kRelativeSumError) · d²`, `kRelativeSumError = 2.02 ε`
/// (`s1chord_angle.h:129`).
pub fn upstream_add_cap_margin(d2: f64) -> f64 {
    (2.0 * f64::EPSILON + 2.02 * f64::EPSILON) * d2
}

/// `ilogb(x)` for a positive `x`: the exponent of its leading bit, exact (subnormals included);
/// `i32::MAX` for +∞, `i32::MIN` for 0.
fn ilogb(x: f64) -> i32 {
    if x == 0.0 {
        return i32::MIN;
    }
    if x.is_infinite() {
        return i32::MAX;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp == 0 {
        // Subnormal: the leading bit of the 52-bit mantissa.
        let m = bits & ((1u64 << 52) - 1);
        -1023 - (m.leading_zeros() as i32 - 12)
    } else {
        exp - 1023
    }
}

/// `S2Cap::GetCellUnionBound`'s level: `kMinWidth.GetLevelForMinValue(r) − 1`, where
/// `GetLevelForMinValue(r)` is `ilogb(deriv / r)` clamped to [0, 30], and 30 for a radius that is
/// not positive (`s2metrics.h:186–200`); `kMinWidth`'s derivative is `2√2 / 3` (the quadratic
/// projection). A result below 0 means the six faces. Upstream does not treat the empty cap
/// apart: its `GetRadius()` is −1 radian, so the level is 29 and `GetCellUnionBound` returns
/// four cells. The library does the same since spec 008's K8 (it returned none).
pub fn upstream_level(r: f64) -> i32 {
    let level = if r > 0.0 {
        ilogb((2.0 * SQRT_2 / 3.0) / r).clamp(0, 30)
    } else {
        30
    };
    level - 1
}

/// A copy of the library's private `reduce_longitude` (`core/src/s2/cap.rs`, spec 008 FR-020),
/// so that P-CPBND-05 can state its exactness: `cap_spec_tests` fails if the two bodies differ
/// (a drift guard, as spec 003's for its restated codecs). It is the implementation's code, not
/// the description: the description of the reduction is upstream's `remainder(x, 2π)`, which the
/// library's tests check the function against exactly (`core/src/s2/cap_tests.rs`).
pub fn reduce_longitude_copy(x: f64) -> f64 {
    let two_pi = 2.0 * PI;
    if x > PI {
        x - two_pi
    } else if x < -PI {
        x + two_pi
    } else {
        x
    }
}

#[cfg(test)]
#[path = "cap_spec_tests.rs"]
mod cap_spec_tests;
