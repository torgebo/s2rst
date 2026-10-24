// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The E-form round trips, upstream's integer identities and the E-form constructors, proven by
//! enumerating every `i32` (spec 007 FR-008, research R12; P-AGEF-02, P-AGCNV-02 and P-AGCNV-01's
//! integer clauses).
//!
//! Each statement's domain is the integers, so checking every one of them is a proof, not a
//! sample, of the library's code compiled in release mode on this machine's IEEE-754
//! arithmetic (`TRUSTED_BASE.md`, spec 007). Kani cannot reach them: the E7 round trip timed
//! out at 12 minutes over every `i32` and over `0 ≤ n < 2²⁴` alike (research R0). The precedent
//! is spec 004's `uv_projection.rs`, which checks every one of `2³⁰ + 1` boundary values.
//!
//! The tests are `#[ignore]`d, because a debug build would take minutes:
//! `make verify-values P=angle` runs them with
//! `cargo test --release --test angle_enumeration -- --ignored` (about 70 s in all).

use s2rst::s1::Angle;

/// Counts the `n` in `range` where `holds(n)` fails, and the first one.
fn failures(
    range: std::ops::RangeInclusive<i32>,
    holds: impl Fn(i32) -> bool,
) -> (u64, Option<i32>) {
    let (mut count, mut first) = (0u64, None);
    for n in range {
        if !holds(n) {
            count += 1;
            first.get_or_insert(n);
        }
    }
    (count, first)
}

/// P-AGEF-02 (correct result), E5: `from_e5(n).e5() == n` for every `i32`.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agef_02_e5() {
    let r = failures(i32::MIN..=i32::MAX, |n| Angle::from_e5(n).e5() == n);
    assert_eq!(r, (0, None), "from_e5(n).e5() != n: (count, first)");
}

/// P-AGEF-02 (correct result), E6: `from_e6(n).e6() == n` for every `i32`.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agef_02_e6() {
    let r = failures(i32::MIN..=i32::MAX, |n| Angle::from_e6(n).e6() == n);
    assert_eq!(r, (0, None), "from_e6(n).e6() != n: (count, first)");
}

/// P-AGEF-02 (correct result), E7: `from_e7(n).e7() == n` for every `i32`.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agef_02_e7() {
    let r = failures(i32::MIN..=i32::MAX, |n| Angle::from_e7(n).e7() == n);
    assert_eq!(r, (0, None), "from_e7(n).e7() != n: (count, first)");
}

/// P-AGCNV-02 (correct result), `E6(n) == E7(10 n)` (`s1angle.h:52`) for every `n` whose
/// `10 n` is an `i32`.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agcnv_02_e6_e7() {
    let r = failures(i32::MIN / 10..=i32::MAX / 10, |n| {
        Angle::from_e6(n).radians().to_bits() == Angle::from_e7(10 * n).radians().to_bits()
    });
    assert_eq!(r, (0, None), "E6(n) != E7(10 n): (count, first)");
}

/// P-AGCNV-02 (correct result), `Degrees(n) == E6(10⁶ n)` and `Degrees(n) == E7(10⁷ n)`
/// (`s1angle.h:50–51`) for every `n` whose product is an `i32`.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agcnv_02_degrees() {
    let r6 = failures(i32::MIN / 1_000_000..=i32::MAX / 1_000_000, |n| {
        Angle::from_degrees(f64::from(n)).radians().to_bits()
            == Angle::from_e6(n * 1_000_000).radians().to_bits()
    });
    let r7 = failures(i32::MIN / 10_000_000..=i32::MAX / 10_000_000, |n| {
        Angle::from_degrees(f64::from(n)).radians().to_bits()
            == Angle::from_e7(n * 10_000_000).radians().to_bits()
    });
    assert_eq!(
        (r6, r7),
        ((0, None), (0, None)),
        "Degrees(n) != E6/E7: (count, first)"
    );
}

/// P-AGCNV-01 (exact output), the E-form constructors: `from_eK(n).radians()` is
/// `angle_spec::upstream_from_e(K, n)`, bit for bit, for every `i32` and K = 5, 6, 7.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agcnv_01_e_forms() {
    use s2rst_verify::angle_spec::upstream_from_e;
    let r = [
        failures(i32::MIN..=i32::MAX, |n| {
            Angle::from_e5(n).radians().to_bits() == upstream_from_e(5, n).to_bits()
        }),
        failures(i32::MIN..=i32::MAX, |n| {
            Angle::from_e6(n).radians().to_bits() == upstream_from_e(6, n).to_bits()
        }),
        failures(i32::MIN..=i32::MAX, |n| {
            Angle::from_e7(n).radians().to_bits() == upstream_from_e(7, n).to_bits()
        }),
    ];
    assert_eq!(
        r,
        [(0, None); 3],
        "from_eK(n) != upstream_from_e(K, n): (count, first)"
    );
}

/// P-AGCNV-01 (exact output), the unsigned constructors: `from_unsigned_eK(u)` is
/// `from_eK(u as i32)`, bit for bit, for every `u32` and K = 6, 7.
#[test]
#[ignore = "release mode: make verify-values P=angle"]
fn p_agcnv_01_unsigned() {
    let wrapped = |n: i32| u32::from_ne_bytes(n.to_ne_bytes());
    let r6 = failures(i32::MIN..=i32::MAX, |n| {
        Angle::from_unsigned_e6(wrapped(n)).radians().to_bits()
            == Angle::from_e6(n).radians().to_bits()
    });
    let r7 = failures(i32::MIN..=i32::MAX, |n| {
        Angle::from_unsigned_e7(wrapped(n)).radians().to_bits()
            == Angle::from_e7(n).radians().to_bits()
    });
    assert_eq!(
        (r6, r7),
        ((0, None), (0, None)),
        "from_unsigned_eK: (count, first)"
    );
}
