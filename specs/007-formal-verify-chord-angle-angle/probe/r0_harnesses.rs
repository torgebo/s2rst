// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Spec 007 planning probe (research R0): Kani harnesses that measure what the checker models
//! for `ChordAngle` and `Angle` (`asin`, `tan`, `sin_cos`, `round_ties_even`, `%`), the cost of
//! the order and neighbour laws on canonical chord angles, and drafts of the fixes FR-017,
//! FR-018, FR-021 and FR-022, before the library changes.
//!
//! They run from a scratch crate that depends on `s2rst`, through a copy of `run-verify.sh`
//! (4 GiB and 12 minutes per harness), one job at a time. Results go to research.md R0. Not
//! part of `verify/`.

use s2rst::s1::{Angle, ChordAngle};
use std::f64::consts::PI;

// ---- drafts of the fixes ----------------------------------------------------------------------

/// FR-017 and FR-018: every negative (−∞ included) is `NEGATIVE`; NaN and values above 4 are
/// `STRAIGHT`, as upstream's `std::min(4.0, x)`.
fn from_length2_draft(x: f64) -> ChordAngle {
    if x < 0.0 {
        ChordAngle::NEGATIVE
    } else if x.is_nan() {
        ChordAngle::STRAIGHT
    } else {
        ChordAngle::from_length2(x)
    }
}

/// FR-021: the IEEE remainder by the `f64` nearest 2π, through the exact `%`, with −π mapped
/// to π as upstream's `Normalize`.
fn normalized_draft(x: f64) -> f64 {
    let y = 2.0 * PI;
    let r = x % y;
    if r > PI {
        r - y
    } else if r <= -PI {
        r + y
    } else {
        r
    }
}

/// FR-022: ties to even, as C++ on x86-64; saturating outside the `i32` range.
fn e7_draft(a: Angle) -> i32 {
    (a.degrees() * 1e7).round_ties_even() as i32
}

// ---- domains ----------------------------------------------------------------------------------

fn is_canonical(l: f64) -> bool {
    l == -1.0 || (0.0 <= l && l <= 4.0) || l == f64::INFINITY
}

/// Every canonical chord angle: `NEGATIVE`, every length in [0, 4], `INFINITY`.
fn any_canonical() -> ChordAngle {
    if kani::any() {
        ChordAngle::INFINITY
    } else {
        let x: f64 = kani::any();
        kani::assume(x == -1.0 || (0.0 <= x && x <= 4.0));
        ChordAngle::from_length2(x)
    }
}

/// Every canonical chord angle that is not special: [0, 4].
fn any_length() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume(0.0 <= x && x <= 4.0);
    ChordAngle::from_length2(x)
}

// ---- 1. what the checker models ---------------------------------------------------------------

/// `to_angle` goes through `sqrt` and `asin`.
#[kani::proof]
fn r0_ca_to_angle_crash() {
    let c = any_canonical();
    let _ = c.to_angle();
}

/// `e7` of a chord angle goes through `to_angle`.
#[kani::proof]
fn r0_ca_e7_crash() {
    let c = any_canonical();
    let _ = c.e7();
}

/// `from_angle` goes through `sin`; its result is in [0, 4] for a non-negative finite angle.
#[kani::proof]
fn r0_ca_from_angle_range() {
    let r: f64 = kani::any();
    kani::assume(r.is_finite() && r >= 0.0);
    let l = ChordAngle::from_angle(Angle::from_radians(r)).length2();
    assert!((0.0..=4.0).contains(&l));
}

/// `Angle::tan` calls the C library's `tan`.
#[kani::proof]
fn r0_ag_tan_crash() {
    let a = Angle::from_radians(kani::any());
    let _ = a.tan();
}

/// `Angle::sin_cos`.
#[kani::proof]
fn r0_ag_sin_cos_crash() {
    let a = Angle::from_radians(kani::any());
    let _ = a.sin_cos();
}

/// `ChordAngle::tan` divides `sin` (through `sqrt`) by `cos`.
#[kani::proof]
fn r0_ca_tan_crash() {
    let c = any_length();
    let _ = c.tan();
}

/// `round_ties_even` against its definition through `floor`, for every value in the `i32`
/// range.
#[kani::proof]
fn r0_round_ties_even_model() {
    let v: f64 = kani::any();
    kani::assume(v > -2147483648.5 && v < 2147483647.5);
    let f = v.floor();
    let d = v - f;
    let even = f % 2.0 == 0.0;
    let want = if d < 0.5 || (d == 0.5 && even) { f } else { f + 1.0 };
    assert!(v.round_ties_even() == want);
}

/// The draft of `normalized` lands in (−π, π] for every finite angle.
#[kani::proof]
fn r0_ag_normalized_draft_range() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let n = normalized_draft(x);
    assert!(-PI < n && n <= PI);
}

/// The draft is idempotent, bit for bit.
#[kani::proof]
fn r0_ag_normalized_draft_idempotent() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let n = normalized_draft(x);
    assert!(normalized_draft(n).to_bits() == n.to_bits());
}

/// The library's `normalized` lands in (−π, π] for every finite angle. Expected to fail (K7).
#[kani::proof]
fn r0_ag_normalized_library_range() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite());
    let n = Angle::from_radians(x).normalized().radians();
    assert!(-PI < n && n <= PI);
}

// ---- 2. the E-forms ---------------------------------------------------------------------------

/// The library's E-forms are crash-free for every angle, NaN and the infinities included.
#[kani::proof]
fn r0_ag_eforms_crash() {
    let a = Angle::from_radians(kani::any());
    let _ = (a.e5(), a.e6(), a.e7());
}

/// The round trip `from_e7(n).e7() == n` for every `i32`, with the library's rounding. The
/// pilot did not finish it in 25 minutes.
#[kani::proof]
fn r0_ag_e7_round_trip_library() {
    let n: i32 = kani::any();
    assert!(Angle::from_e7(n).e7() == n);
}

/// The same with FR-022's draft.
#[kani::proof]
fn r0_ag_e7_round_trip_draft() {
    let n: i32 = kani::any();
    assert!(e7_draft(Angle::from_e7(n)) == n);
}

/// The same, split: non-negative `n` below 2²⁴, a quarter of the range in magnitude bits.
#[kani::proof]
fn r0_ag_e7_round_trip_draft_small() {
    let n: i32 = kani::any();
    kani::assume((0..1 << 24).contains(&n));
    assert!(e7_draft(Angle::from_e7(n)) == n);
}

/// Upstream's identity `E6(n) == E7(10 n)` (`s1angle.h:52`), for every `n` with `10 n` in range.
#[kani::proof]
fn r0_ag_e6_e7_identity() {
    let n: i32 = kani::any();
    kani::assume(n > i32::MIN / 10 && n < i32::MAX / 10);
    assert!(Angle::from_e6(n) == Angle::from_e7(10 * n));
}

// ---- 3. the chord angle's laws on canonical values --------------------------------------------

/// FR-017/FR-018's draft constructor returns a canonical value for every `f64`.
#[kani::proof]
fn r0_ca_from_length2_draft_canonical() {
    let x: f64 = kani::any();
    assert!(is_canonical(from_length2_draft(x).length2()));
}

/// The library's constructor does not (K1–K3). Expected to fail.
#[kani::proof]
fn r0_ca_from_length2_library_canonical() {
    let x: f64 = kani::any();
    assert!(is_canonical(ChordAngle::from_length2(x).length2()));
}

/// The order is a total order on canonical values: 006's generic laws (`verify/src/laws.rs`,
/// `ord::all`, private to that crate, restated here as they are).
#[kani::proof]
fn r0_ca_order_laws() {
    use std::cmp::Ordering;
    let (a, b, c) = (any_canonical(), any_canonical(), any_canonical());
    let cmp = |x: &ChordAngle, y: &ChordAngle| x.partial_cmp(y).unwrap();
    assert!(cmp(&a, &a) == Ordering::Equal);
    assert!(cmp(&b, &a) == cmp(&a, &b).reverse());
    let (ab, bc, ac) = (cmp(&a, &b), cmp(&b, &c), cmp(&a, &c));
    if ab != Ordering::Greater && bc != Ordering::Greater {
        assert!(ac != Ordering::Greater);
    }
    if ab == Ordering::Less && bc == Ordering::Less {
        assert!(ac == Ordering::Less);
    }
    assert!((cmp(&a, &b) == Ordering::Equal) == (a == b));
}

/// `successor` is the next canonical value: above `c`, canonical, and nothing canonical lies
/// strictly between.
#[kani::proof]
fn r0_ca_successor_next() {
    let c = any_canonical();
    kani::assume(c != ChordAngle::INFINITY);
    let s = c.successor();
    assert!(is_canonical(s.length2()));
    assert!(c < s);
    let d = any_canonical();
    assert!(!(c < d && d < s));
}

/// `predecessor` is the previous canonical value, likewise.
#[kani::proof]
fn r0_ca_predecessor_previous() {
    let c = any_canonical();
    kani::assume(c != ChordAngle::NEGATIVE);
    let p = c.predecessor();
    assert!(is_canonical(p.length2()));
    assert!(p < c);
    let d = any_canonical();
    assert!(!(p < d && d < c));
}

/// `predecessor(successor(c)) == c` below `INFINITY`.
#[kani::proof]
fn r0_ca_neighbours_inverse() {
    let c = any_canonical();
    kani::assume(c != ChordAngle::INFINITY);
    assert!(c.successor().predecessor() == c);
}

/// Addition of non-special values returns a value in [0, 4] (through `sqrt`).
#[kani::proof]
fn r0_ca_add_range() {
    let (a, b) = (any_length(), any_length());
    let l = (a + b).length2();
    assert!((0.0..=4.0).contains(&l));
}

/// Subtraction, likewise.
#[kani::proof]
fn r0_ca_sub_range() {
    let (a, b) = (any_length(), any_length());
    let l = (a - b).length2();
    assert!((0.0..=4.0).contains(&l));
}

/// `plus_error` with FR-018's NaN handling returns a canonical value for every error.
#[kani::proof]
fn r0_ca_plus_error_canonical() {
    let c = any_canonical();
    let e: f64 = kani::any();
    kani::assume(!e.is_nan());
    assert!(is_canonical(c.plus_error(e).length2()));
}

// ---- 4. isolating the checker's models (added after the first run, 2026-10-16) -----------------

/// Concrete `%`: `1 % 2 == 1`, `7 % 2 == 1`, `−7 % 2 == −1`, `5.5 % 2 == 1.5`.
#[kani::proof]
fn r0_frem_concrete() {
    let two: f64 = kani::any();
    kani::assume(two == 2.0);
    assert!(1.0 % two == 1.0);
    assert!(7.0 % two == 1.0);
    assert!(-7.0 % two == -1.0);
    assert!(5.5 % two == 1.5);
}

/// Concrete `round_ties_even`: 0.5 → 0, 1.5 → 2, 2.5 → 2, −2.5 → −2, 2.4 → 2.
#[kani::proof]
fn r0_round_ties_even_concrete() {
    let h: f64 = kani::any();
    kani::assume(h == 0.5);
    assert!(h.round_ties_even() == 0.0);
    assert!((h + 1.0).round_ties_even() == 2.0);
    assert!((h + 2.0).round_ties_even() == 2.0);
    assert!((-h - 2.0).round_ties_even() == -2.0);
    assert!((h + 1.9).round_ties_even() == 2.0);
}

/// `round_ties_even` against its definition, without `%`: evenness through `floor(f / 2)`.
#[kani::proof]
fn r0_round_ties_even_model_nofrem() {
    let v: f64 = kani::any();
    kani::assume(v > -2147483648.5 && v < 2147483647.5);
    let f = v.floor();
    let d = v - f;
    let even = (f / 2.0).floor() * 2.0 == f;
    let want = if d < 0.5 || (d == 0.5 && even) { f } else { f + 1.0 };
    assert!(v.round_ties_even() == want);
}

/// `%` against fmod's definition for a symbolic dividend and divisor 2π: the result has the
/// dividend's sign (or is zero), is smaller than 2π in magnitude, and is exact for
/// `|x| < 2π`.
#[kani::proof]
fn r0_frem_small_identity() {
    let x: f64 = kani::any();
    kani::assume(x.is_finite() && x.abs() < 2.0 * PI);
    assert!((x % (2.0 * PI)).to_bits() == x.to_bits());
}

/// Which value the checker's `%` gives for `1 % 2` and `−0 % 2π` (covers, not assertions).
#[kani::proof]
fn r0_frem_which() {
    let two: f64 = kani::any();
    kani::assume(two == 2.0);
    let r = 1.0 % two;
    kani::cover!(r == 1.0, "1 % 2 is 1 (fmod)");
    kani::cover!(r == -1.0, "1 % 2 is -1");
    kani::cover!(r == 0.0, "1 % 2 is 0");
    kani::cover!(r.is_nan(), "1 % 2 is NaN");
    kani::cover!(r != 1.0 && r != -1.0 && r != 0.0 && !r.is_nan(), "1 % 2 is something else");
    let z: f64 = kani::any();
    kani::assume(z == 0.0 && z.is_sign_negative());
    let s = z % (2.0 * PI);
    kani::cover!(s.to_bits() == (-0.0f64).to_bits(), "-0 % 2pi is -0");
    kani::cover!(s.to_bits() == 0.0f64.to_bits(), "-0 % 2pi is +0");
}
