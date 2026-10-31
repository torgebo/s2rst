// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::CellId`] parity fixes found during the formal
//! verification pilot (specs/001-formal-verify-base-type). Written for this crate, not
//! ported from upstream S2.

use super::*;

/// FR-015: a token containing any character that is not a hex digit decodes to `none`.
///
/// C++ `S2CellId::FromToken` accepts only `0-9`, `a-f` and `A-F`, character by character, and
/// Go `CellIDFromToken` uses `strconv.ParseUint`, which rejects a sign. `u64::from_str_radix`
/// alone accepts a leading `+`, which made `"+1"` decode to the valid cell `0/00`.
#[test]
fn from_token_rejects_non_hex_characters() {
    for token in ["+1", "+fffffffffffffff", "+", "-1", "+0"] {
        assert_eq!(CellId::from_token(token), CellId::none(), "token {token:?}");
    }
}

/// FR-016: an invalid id prints as `Invalid: ` followed by 16 zero-padded lowercase hex
/// digits, like C++ `S2CellId::ToString` (`absl::Hex(id(), absl::kZeroPad16)`). `Display`
/// writes the same text. Valid ids are unaffected.
#[test]
fn invalid_debug_string_is_zero_padded() {
    // 2 = 0b10: the lowest set bit is at an odd position, so the id is invalid.
    assert_eq!(CellId(2).to_debug_string(), "Invalid: 0000000000000002");
    assert_eq!(
        CellId::none().to_debug_string(),
        "Invalid: 0000000000000000"
    );
    assert_eq!(
        CellId::sentinel().to_debug_string(),
        "Invalid: ffffffffffffffff"
    );
    assert_eq!(CellId(2).to_string(), CellId(2).to_debug_string());
    assert_eq!(CellId::from_face(Face::F3).to_debug_string(), "3/");
}

/// Spec 011 K4 (FR-019): `from_point` panics in debug builds for every point with two or more
/// infinite coordinates (the face check's `∞ · 0`), and returns for one.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "does not belong to face")]
fn from_point_panics_for_two_infinities() {
    let _ = CellId::from_point(&Point(crate::r3::Vector {
        x: 0.0,
        y: f64::INFINITY,
        z: f64::INFINITY,
    }));
}

#[test]
fn from_point_returns_a_leaf_for_one_infinity() {
    let id = CellId::from_point(&Point(crate::r3::Vector {
        x: 0.0,
        y: f64::INFINITY,
        z: 1.0,
    }));
    assert!(id.is_valid() && id.is_leaf());
}
