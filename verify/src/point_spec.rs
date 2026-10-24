// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Reference description of the point (spec 006 data-model §1.2): its order, its
//! classifications and its power-of-two scaling, read from a triple of `f64` coordinates.
//!
//! Written from upstream's `util/math/vector.h`, `s2pointutil.{h,cc}` and
//! `s2edge_crossings.cc` (`v0.14.0-40`). A point is a triple `(x, y, z)`; the type has no
//! validity rule, and "unit length" is upstream's test, restated below. The order is C++'s
//! `operator<` on the coordinates; the geometric domain is the box `[−1, 1]³`, which holds every
//! unit point (research R4).
//!
//! The description reads a [`Point`]'s public coordinates and never calls `Point`'s or
//! `Vector`'s methods (FR-007). Booleans are combined with `&&`, `||` and `==` only (spec 004
//! research R13); orders are [`Ordering`] values.
//!
//! The last section restates three pieces of upstream's arithmetic (`upstream_*`). They are
//! not the meaning: they say what upstream computes, so that a property can state "bit for bit
//! upstream's" and the C++ vectors can check the restatement (`verify/tests/values_spec_vs_cpp.rs`).

use s2rst::r3::Vector;
use s2rst::s2::Point;
use std::cmp::Ordering;

/// `2⁻²⁴²`, upstream's normalizability threshold (`ldexp(1, -242)`, `s2edge_crossings.cc:287`).
pub const TWO_M242: f64 = f64::from_bits((1023 - 242) << 52);

/// Upstream's `S2::Origin()` coordinates, written as upstream writes them
/// (`s2pointutil.h:114–115`). Rust and C++ both round a decimal literal to the nearest `f64`,
/// so these are upstream's bit patterns.
#[expect(
    clippy::excessive_precision,
    reason = "upstream's literals, digit for digit (s2pointutil.h:114–115)"
)]
pub const ORIGIN: [f64; 3] = [
    -0.0099994664350250197,
    0.0025924542609324121,
    0.99994664350250195,
];

/// The coordinates as an array, in upstream's order.
fn coords(p: Point) -> [f64; 3] {
    [p.0.x, p.0.y, p.0.z]
}

/// A point from three coordinates, as a struct literal.
pub fn point(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

/// C++'s `operator<` on vectors: `std::lexicographical_compare` over `(x, y, z)` with `<`
/// (`util/math/vector.h:110–115`). The first coordinate where `a_i < b_i` or `b_i < a_i`
/// decides; a pair that is neither (equal, or a NaN on either side) decides nothing, and the
/// comparison moves on. So NaN makes it intransitive (spec K8).
#[must_use]
pub fn lex_less(a: Point, b: Point) -> bool {
    let (a, b) = (coords(a), coords(b));
    for i in 0..3 {
        if a[i] < b[i] {
            return true;
        }
        if b[i] < a[i] {
            return false;
        }
    }
    false
}

/// The three-way order that [`lex_less`] induces: `Less` if `a < b`, `Greater` if `b < a`,
/// `Equal` otherwise.
#[must_use]
pub fn order(a: Point, b: Point) -> Ordering {
    if lex_less(a, b) {
        Ordering::Less
    } else if lex_less(b, a) {
        Ordering::Greater
    } else {
        Ordering::Equal
    }
}

/// Equality as values: IEEE `==` per coordinate, so `0` and `−0` are the same and NaN is never
/// the same as anything. The orthogonal directions' laws hold in this sense and not bit for bit
/// (spec 006 research R0, finding 2).
#[must_use]
pub fn same(a: Point, b: Point) -> bool {
    a.0.x == b.0.x && a.0.y == b.0.y && a.0.z == b.0.z
}

/// The largest absolute coordinate, for a triple without NaN.
#[must_use]
pub fn max_abs(p: Point) -> f64 {
    let [x, y, z] = coords(p).map(f64::abs);
    let m = if x >= y { x } else { y };
    if m >= z { m } else { z }
}

/// Whether the point is large enough that a cross product with it loses no precision to
/// underflow: its largest absolute coordinate is at least `2⁻²⁴²` (`IsNormalizable`,
/// `s2edge_crossings.cc:276–288`). For a triple without NaN.
#[must_use]
pub fn normalizable(p: Point) -> bool {
    max_abs(p) >= TWO_M242
}

/// Whether every coordinate is finite and the squared length `x·x + y·y + z·z`, summed in
/// upstream's `Norm2` order (`util/math/vector.h:315`), is finite too: the domain where
/// normalising never divides by an infinity (research R0, finding 1).
#[must_use]
pub fn normable(p: Point) -> bool {
    let [x, y, z] = coords(p);
    x.is_finite() && y.is_finite() && z.is_finite() && (x * x + y * y + z * z).is_finite()
}

/// Whether every coordinate is in `[−1, 1]`: the box that holds every unit point, the domain of
/// the geometric operations (research R4). False for NaN.
#[must_use]
pub fn in_box(p: Point) -> bool {
    coords(p).iter().all(|c| c.abs() <= 1.0)
}

/// Whether every coordinate is zero, of either sign.
#[must_use]
pub fn zero(p: Point) -> bool {
    p.0.x == 0.0 && p.0.y == 0.0 && p.0.z == 0.0
}

/// `ilogb(v)` for a non-zero finite `v`: the exponent `e` with `2^e ≤ |v| < 2^(e+1)`, read from
/// IEEE-754's encoding, subnormals included (`exponent(2⁻¹⁰⁷⁴) = −1074`).
#[must_use]
pub fn exponent(v: f64) -> i32 {
    let bits = v.abs().to_bits();
    let biased = i32::try_from(bits >> 52).expect("an 11-bit exponent");
    if biased == 0 {
        // A subnormal: the value is `mantissa · 2⁻¹⁰⁷⁴`, so the leading bit sets the exponent.
        let top = 63 - i32::try_from(bits.leading_zeros()).expect("at most 64");
        top - 1074
    } else {
        biased - 1023
    }
}

/// `2^k` as an `f64`, exactly: normal for `k` in `[−1022, 1023]`, subnormal down to `−1074`,
/// zero below that and infinity above (as C's `ldexp(1, k)`).
#[must_use]
pub fn pow2(k: i32) -> f64 {
    if k > 1023 {
        f64::INFINITY
    } else if k >= -1022 {
        f64::from_bits(u64::try_from(1023 + k).expect("a biased exponent") << 52)
    } else if k >= -1074 {
        f64::from_bits(1 << (k + 1074))
    } else {
        0.0
    }
}

/// `p · 2^k`, computed in two steps of `2^(k/2)` and `2^(k − k/2)`. For `|k| ≤ 2044` both
/// factors are normal (at most `2^1022`), and for the scaling's `k` (at most 1074, so each
/// factor at most `2^537`) each step is exact: it scales up without overflow.
pub fn scaled_by(p: Point, k: i32) -> Point {
    let (f1, f2) = (pow2(k / 2), pow2(k - k / 2));
    let [x, y, z] = coords(p);
    point(x * f1 * f2, y * f1 * f2, z * f1 * f2)
}

// ---- Restatements of upstream's arithmetic (FR-007) -------------------------------------
//
// What upstream computes, operation for operation, so that a property can say "bit for bit
// upstream's". Each is checked against the C++ vectors' `S2POINT_*` rows by
// `verify/tests/values_spec_vs_cpp.rs`.

/// Upstream's `Norm2`: `((0 + x·x) + y·y) + z·z` (`util/math/vector.h:182, 315`). Adding the
/// first product to `+0` changes nothing, since a square is never `−0`.
#[must_use]
pub fn upstream_norm2(p: Point) -> f64 {
    let [x, y, z] = coords(p);
    x * x + y * y + z * z
}

/// `S2::IsUnitLength`: `fabs(p.Norm2() − 1) <= 5 * DBL_EPSILON` (`s2pointutil.cc:32–40`).
#[must_use]
pub fn upstream_is_unit(p: Point) -> bool {
    (upstream_norm2(p) - 1.0).abs() <= 5.0 * f64::EPSILON
}

/// `Vector3::Normalize`: `n = Norm(); if (n != 0) n = 1 / n; return p * n`
/// (`util/math/vector.h:191–198`). Where the norm is zero, upstream multiplies by zero, so each
/// zero keeps the sign of its coordinate; the library, as Go, returns `(+0, +0, +0)` there
/// (spec 006 K12).
pub fn upstream_normalize(p: Point) -> Point {
    let mut n = upstream_norm2(p).sqrt();
    if n != 0.0 {
        n = 1.0 / n;
    }
    let [x, y, z] = coords(p);
    point(x * n, y * n, z * n)
}

/// `EnsureNormalizable`'s scaling: `ldexp(2, −1 − ilogb(p_max)) * p`
/// (`s2edge_crossings.cc:294–311`), for a non-zero triple without NaN. The factor is
/// `2^(−ilogb(p_max))`; it overflows to infinity for `p_max < 2⁻¹⁰²³`, and then the result has an
/// infinite coordinate and a NaN for each zero one (spec K6, U-012).
pub fn upstream_scaled(p: Point) -> Point {
    let factor = 2.0 * pow2(-1 - exponent(max_abs(p)));
    let [x, y, z] = coords(p);
    point(factor * x, factor * y, factor * z)
}

/// `S2::Ortho` (`s2pointutil.cc:48–59`): the cross product of `a` with
/// `(0.012, 0.0053, 0.00457)`, whose coordinate before `a`'s largest one (cyclically) is set to
/// 1, normalised. The largest coordinate is `LargestAbsComponent`'s (`util/math/vector.h`),
/// ties going to the later axis. Normalised by [`upstream_normalize`], so a zero cross product
/// keeps its zeros' signs (K12).
pub fn upstream_ortho(a: Point) -> Point {
    let [x, y, z] = coords(a).map(f64::abs);
    let k = if x > y {
        if x > z { 0 } else { 2 }
    } else if y > z {
        1
    } else {
        2
    };
    let mut t = [0.012, 0.0053, 0.00457];
    t[(k + 2) % 3] = 1.0;
    let [ax, ay, az] = coords(a);
    upstream_normalize(point(
        ay * t[2] - az * t[1],
        az * t[0] - ax * t[2],
        ax * t[1] - ay * t[0],
    ))
}

/// Go's `PointCross` before its zero test: `(p + op) × (op − p)`, twice the cross product,
/// with `r3.Vector.Cross`'s products and differences (`s2/point.go:85`, `r3/vector.go:76–81`).
/// Where it is exactly the zero vector, `PointCross` returns an orthogonal direction instead.
pub fn go_cross_kernel(p: Point, q: Point) -> Point {
    let [px, py, pz] = coords(p);
    let [qx, qy, qz] = coords(q);
    let (sx, sy, sz) = (px + qx, py + qy, pz + qz);
    let (dx, dy, dz) = (qx - px, qy - py, qz - pz);
    point(sy * dz - sz * dy, sz * dx - sx * dz, sx * dy - sy * dx)
}

#[cfg(test)]
#[path = "point_spec_tests.rs"]
mod point_spec_tests;
