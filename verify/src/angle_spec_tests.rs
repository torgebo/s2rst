// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the angle description (spec 007 T009), on the survey's ties and the
//! integer range's edges.

use super::*;

/// The radians whose degrees × 10⁶ (or × 10⁷) is exactly a half integer, from the survey probe
/// (`specs/007-formal-verify-chord-angle-angle/probe/cpp/survey_cpp.txt`), with C++'s results.
const TIES: [(u64, u32, i32); 6] = [
    (0x3e42_bd86_46c1_beed, 6, 0),  // 0.5
    (0x3e5c_1c49_6a22_9e64, 6, 2),  // 1.5
    (0x3e67_6ce7_d872_2ea9, 6, 2),  // 2.5
    (0x3e75_1537_0f99_f6cb, 6, 4),  // 4.5
    (0xbe0d_fc0a_0acf_97e2, 7, 0),  // −0.5
    (0xbe32_bd86_46c1_beed, 7, -2), // −2.5
];

#[test]
fn ties_go_to_even_as_cpp() {
    for (bits, k, want) in TIES {
        assert_eq!(upstream_e(k, f64::from_bits(bits)), want, "{bits:#x}");
    }
}

#[test]
fn round_half_even_around_ties() {
    assert_eq!(round_half_even(0.5), 0.0);
    assert_eq!(round_half_even(1.5), 2.0);
    assert_eq!(round_half_even(2.5), 2.0);
    assert_eq!(round_half_even(-2.5), -2.0);
    assert_eq!(round_half_even(-0.5), -0.0);
    assert_eq!(round_half_even(2.4999999999999996), 2.0);
    assert_eq!(round_half_even(2.5000000000000004), 3.0);
    assert_eq!(round_half_even(-3.5), -4.0);
    assert_eq!(round_half_even(1e300), 1e300);
}

#[test]
fn outside_the_range_saturates() {
    assert_eq!(upstream_e(7, 1e10), i32::MAX);
    assert_eq!(upstream_e(7, -1e10), i32::MIN);
    assert_eq!(upstream_e(5, f64::INFINITY), i32::MAX);
    assert_eq!(upstream_e(5, f64::NEG_INFINITY), i32::MIN);
    assert_eq!(upstream_e(5, f64::NAN), 0);
    // Just inside: 214.7483647° is the largest E7.
    assert_eq!(upstream_e(7, upstream_from_e(7, i32::MAX)), i32::MAX);
    assert_eq!(upstream_e(7, upstream_from_e(7, i32::MIN)), i32::MIN);
}

#[test]
fn conversions() {
    assert_eq!(upstream_from_degrees(180.0), PI);
    assert_eq!(upstream_degrees(PI), 180.0);
    assert_eq!(
        upstream_from_e(6, 45_123_000),
        upstream_from_degrees(1e-6 * 45_123_000.0)
    );
    for n in [-214_748_364, -1, 0, 1, 12_345_678, 214_748_364] {
        assert_eq!(
            upstream_from_e(6, n).to_bits(),
            upstream_from_e(7, 10 * n).to_bits(),
            "E6({n}) == E7({})",
            10 * n
        );
    }
}
