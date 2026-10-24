// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of the chord angle (spec 007 data-model §1.1): the canonical values,
//! their order and their neighbours, read from a squared chord length.
//!
//! Written from upstream's `s1chord_angle.{h,cc}` (`v0.14.0-40`). A chord angle is the squared
//! length of the chord between two points of the unit sphere, so a real distance lies in
//! [0, 4]; upstream adds two sentinels outside that range, `Negative()` (−1, below every
//! distance) and `Infinity()` (+∞, above every distance). These are the canonical values. Since
//! spec 007's option B (FR-017) the library holds no others: every negative input becomes −1.
//!
//! The description works on the `f64` a chord angle reports through `length2()`; it never calls
//! the operations it describes (FR-007). The neighbours are computed from bit patterns, not with
//! `f64::next_up` or the library's own stepping (research R3). Booleans are combined with `&&`,
//! `||` and `==` only (spec 004 research R13); no `%` on floats (research R0, Finding 1).
//!
//! The last section restates upstream's arithmetic (`upstream_*`). It is not the meaning: it
//! says what upstream computes, so that a property can state "bit for bit upstream's" and the
//! C++ vectors can check the restatement (`verify/tests/values_spec_vs_cpp.rs`).

/// `Negative()`'s squared length.
pub const NEGATIVE: f64 = -1.0;
/// `Zero()`'s squared length.
pub const ZERO: f64 = 0.0;
/// `Right()`'s squared length, a quarter circle.
pub const RIGHT: f64 = 2.0;
/// `Straight()`'s squared length, antipodal points.
pub const STRAIGHT: f64 = 4.0;
/// `Infinity()`'s squared length.
pub const INFINITY: f64 = f64::INFINITY;

/// The smallest positive `f64`, the canonical value just above zero.
const TINY: f64 = f64::from_bits(1);

/// A canonical value: −1, a length in [0, 4] (either zero), or +∞.
#[must_use]
pub fn canonical(l: f64) -> bool {
    l == NEGATIVE || (ZERO..=STRAIGHT).contains(&l) || l == INFINITY
}

/// One of the two sentinels, `Negative()` or `Infinity()`.
#[must_use]
pub fn special(l: f64) -> bool {
    l == NEGATIVE || l == INFINITY
}

/// The order of the canonical values: the order of the reals (−1 below every length, +∞ above
/// every length, the two zeros equal).
#[must_use]
pub fn less(a: f64, b: f64) -> bool {
    a < b
}

/// The least canonical value above `l` (`l` itself at the top): −1 → +0; ±0 → the smallest
/// subnormal; a length below 4 → the next double up; 4 → +∞; +∞ → +∞.
#[must_use]
pub fn next(l: f64) -> f64 {
    if l == NEGATIVE {
        ZERO
    } else if l == ZERO {
        TINY
    } else if l == STRAIGHT || l == INFINITY {
        INFINITY
    } else {
        // 0 < l < 4: a positive finite double; the next pattern is the next double up.
        f64::from_bits(l.to_bits() + 1)
    }
}

/// The greatest canonical value below `l` (`l` itself at the bottom): +∞ → 4; a length above
/// the smallest subnormal → the next double down; the smallest subnormal → +0; ±0 → −1;
/// −1 → −1.
#[must_use]
pub fn prev(l: f64) -> f64 {
    if l == INFINITY {
        STRAIGHT
    } else if l == ZERO || l == NEGATIVE {
        NEGATIVE
    } else {
        // 0 < l ≤ 4: the previous pattern is the next double down (+0 below the smallest).
        f64::from_bits(l.to_bits() - 1)
    }
}

// ---- Restatements of upstream's arithmetic (FR-007) ---------------------------------------
//
// C++'s `std::min(a, b)` is `(b < a) ? b : a` and `std::max(a, b)` is `(a < b) ? b : a`; with a
// NaN operand both return `a`. The restatements spell them out, so that NaN and signed zeros
// come out as upstream's.

/// C++'s `std::min(a, b)`.
fn cpp_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// C++'s `std::max(a, b)`.
fn cpp_max(a: f64, b: f64) -> f64 {
    if a < b { b } else { a }
}

/// `S1ChordAngle::FromLength2(x)`, `S1ChordAngle(std::min(4.0, x))` (`s1chord_angle.h:365–367`):
/// above 4, +∞ and NaN give 4. **With the library's deviation** (option B, FR-017): a negative
/// `x`, −∞ included, gives −1, where upstream keeps `x`.
#[must_use]
pub fn upstream_from_length2(x: f64) -> f64 {
    if x < 0.0 { NEGATIVE } else { cpp_min(4.0, x) }
}

/// `S1ChordAngle::PlusError(e)` (`s1chord_angle.h`): a special value unchanged, otherwise
/// `std::max(0.0, std::min(4.0, length2 + e))`. A NaN sum gives 4.
#[must_use]
pub fn upstream_plus_error(l: f64, e: f64) -> f64 {
    if special(l) {
        l
    } else {
        cpp_max(0.0, cpp_min(4.0, l + e))
    }
}

/// `sin2(a)`, `a.length2() * (1 - 0.25 * a.length2())` (`s1chord_angle.cc:138`).
#[must_use]
pub fn upstream_sin2(l: f64) -> f64 {
    l * (1.0 - 0.25 * l)
}

/// `cos(a)`, `1 - 0.5 * a.length2()` (`s1chord_angle.cc:148`).
#[must_use]
pub fn upstream_cos(l: f64) -> f64 {
    1.0 - 0.5 * l
}

/// `GetS2PointConstructorMaxError()`, `4.5 * DBL_EPSILON * length2_ + 16 * DBL_EPSILON *
/// DBL_EPSILON` (`s1chord_angle.cc:65`).
#[must_use]
pub fn upstream_max_point_error(l: f64) -> f64 {
    4.5 * f64::EPSILON * l + 16.0 * f64::EPSILON * f64::EPSILON
}

/// `GetS1AngleConstructorMaxError()`, `1.5 * DBL_EPSILON * length2_` (`s1chord_angle.cc:72`,
/// at `v0.14.0-40`; older releases had `DBL_EPSILON * length2_`).
#[must_use]
pub fn upstream_max_angle_error(l: f64) -> f64 {
    1.5 * f64::EPSILON * l
}

#[cfg(test)]
#[path = "chord_spec_tests.rs"]
mod chord_spec_tests;
