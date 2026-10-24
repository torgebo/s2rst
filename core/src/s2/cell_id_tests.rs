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
    assert_eq!(CellId::from_face(3u8).to_debug_string(), "3/");
}
