// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Encoding properties P-ENC-* (FR-005): face and leaf coordinates, and the text forms.
//!
//! The parser families `p_enc_06__b<L>`, `p_enc_09__b<L>` and `p_enc_10__b<L>` cover every
//! byte string of exactly `L` bytes (`domain::with_text`, research R6). They are the
//! catalogue properties P-ENC-06/09/10 **and** discharge the crash obligations C-from_token,
//! C-from_debug_string and C-FromStr: they assume nothing beyond the crash domain, and Kani
//! checks every panic and overflow in them (data-model §3). Texts longer than 17 (token) or
//! 33 (debug string) bytes are the input-size remainder (FR-007): rejected by the length
//! check before any loop, exercised by the FR-017 tests and fuzz target.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::CellId;

use crate::domain::with_text;
use crate::spec;

/// P-ENC-06 harnesses for token lengths that can be accepted (1–16 bytes).
macro_rules! token_accepting {
    ($($name:ident = $len:literal),* $(,)?) => { $(
        /// P-ENC-06 (exact input): `from_token` accepts exactly 1 to 16 hex digits and returns the
        /// id they spell; anything else gives `none()`.
        ///
        /// `from_token(s).id() == spec::parse_token(s.as_bytes())`: none unless 1–16 hex digits
        /// (FR-015). Also discharges C-from_token for this length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $name() {
            with_text::<$len>(|s| {
                let got = CellId::from_token(s).id();
                assert_eq!(got, spec::parse_token(s.as_bytes()));
                kani::cover!(got != 0, "accepted");
                kani::cover!(got == 0, "rejected");
            });
        }
    )* };
}

/// P-ENC-06 harnesses for token lengths that are always rejected (0 and 17 bytes).
macro_rules! token_rejecting {
    ($($name:ident = $len:literal),* $(,)?) => { $(
        /// P-ENC-06 (exact input): `from_token` accepts exactly 1 to 16 hex digits and returns the
        /// id they spell; anything else gives `none()`.
        ///
        /// `from_token(s).id() == spec::parse_token(s.as_bytes())`: none unless 1–16 hex digits
        /// (FR-015). Also discharges C-from_token for this length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $name() {
            with_text::<$len>(|s| {
                let got = CellId::from_token(s).id();
                assert_eq!(got, spec::parse_token(s.as_bytes()));
                kani::cover!(got == 0, "rejected");
            });
        }
    )* };
}

token_rejecting!(p_enc_06__b0 = 0, p_enc_06__b17 = 17);
token_accepting!(
    p_enc_06__b1 = 1,
    p_enc_06__b2 = 2,
    p_enc_06__b3 = 3,
    p_enc_06__b4 = 4,
    p_enc_06__b5 = 5,
    p_enc_06__b6 = 6,
    p_enc_06__b7 = 7,
    p_enc_06__b8 = 8,
    p_enc_06__b9 = 9,
    p_enc_06__b10 = 10,
    p_enc_06__b11 = 11,
    p_enc_06__b12 = 12,
    p_enc_06__b13 = 13,
    p_enc_06__b14 = 14,
    p_enc_06__b15 = 15,
    p_enc_06__b16 = 16,
);

/// P-ENC-09 and P-ENC-10 harnesses for debug-string lengths that can be accepted (2–32).
macro_rules! debug_accepting {
    ($($p9:ident, $p10:ident = $len:literal),* $(,)?) => { $(
        /// P-ENC-09 (exact input): `from_debug_string` accepts exactly a face digit, `/` and up to
        /// 30 digits `0`–`3`, and returns that cell.
        ///
        /// `from_debug_string(s).map(CellId::id) == spec::parse_debug_string(s.as_bytes())`. Also
        /// discharges C-from_debug_string for this length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $p9() {
            with_text::<$len>(|s| {
                let got = CellId::from_debug_string(s).map(CellId::id);
                assert_eq!(got, spec::parse_debug_string(s.as_bytes()));
                kani::cover!(got.is_some(), "accepted");
                kani::cover!(got.is_none(), "rejected");
            });
        }

        /// P-ENC-10 (exact input): `str::parse::<CellId>` accepts and returns exactly what
        /// `from_debug_string` does.
        ///
        /// `s.parse::<CellId>().map(CellId::id) ==
        /// spec::parse_debug_string(s.as_bytes()).ok_or(())`. Also discharges C-FromStr for this
        /// length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $p10() {
            with_text::<$len>(|s| {
                let got = s.parse::<CellId>().map(CellId::id);
                assert_eq!(got, spec::parse_debug_string(s.as_bytes()).ok_or(()));
                kani::cover!(got.is_ok(), "accepted");
                kani::cover!(got.is_err(), "rejected");
            });
        }
    )* };
}

/// P-ENC-09 and P-ENC-10 harnesses for debug-string lengths that are always rejected
/// (0, 1 and 33 bytes).
macro_rules! debug_rejecting {
    ($($p9:ident, $p10:ident = $len:literal),* $(,)?) => { $(
        /// P-ENC-09 (exact input): `from_debug_string` accepts exactly a face digit, `/` and up to
        /// 30 digits `0`–`3`, and returns that cell.
        ///
        /// `from_debug_string(s).map(CellId::id) == spec::parse_debug_string(s.as_bytes())`. Also
        /// discharges C-from_debug_string for this length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $p9() {
            with_text::<$len>(|s| {
                let got = CellId::from_debug_string(s).map(CellId::id);
                assert_eq!(got, spec::parse_debug_string(s.as_bytes()));
                kani::cover!(got.is_none(), "rejected");
            });
        }

        /// P-ENC-10 (exact input): `str::parse::<CellId>` accepts and returns exactly what
        /// `from_debug_string` does.
        ///
        /// `s.parse::<CellId>().map(CellId::id) ==
        /// spec::parse_debug_string(s.as_bytes()).ok_or(())`. Also discharges C-FromStr for this
        /// length.
        #[kani::proof]
        #[kani::unwind(40)]
        fn $p10() {
            with_text::<$len>(|s| {
                let got = s.parse::<CellId>().map(CellId::id);
                assert_eq!(got, spec::parse_debug_string(s.as_bytes()).ok_or(()));
                kani::cover!(got.is_err(), "rejected");
            });
        }
    )* };
}

debug_rejecting!(
    p_enc_09__b0,
    p_enc_10__b0 = 0,
    p_enc_09__b1,
    p_enc_10__b1 = 1,
    p_enc_09__b33,
    p_enc_10__b33 = 33,
);
debug_accepting!(
    p_enc_09__b2,
    p_enc_10__b2 = 2,
    p_enc_09__b3,
    p_enc_10__b3 = 3,
    p_enc_09__b4,
    p_enc_10__b4 = 4,
    p_enc_09__b5,
    p_enc_10__b5 = 5,
    p_enc_09__b6,
    p_enc_10__b6 = 6,
    p_enc_09__b7,
    p_enc_10__b7 = 7,
    p_enc_09__b8,
    p_enc_10__b8 = 8,
    p_enc_09__b9,
    p_enc_10__b9 = 9,
    p_enc_09__b10,
    p_enc_10__b10 = 10,
    p_enc_09__b11,
    p_enc_10__b11 = 11,
    p_enc_09__b12,
    p_enc_10__b12 = 12,
    p_enc_09__b13,
    p_enc_10__b13 = 13,
    p_enc_09__b14,
    p_enc_10__b14 = 14,
    p_enc_09__b15,
    p_enc_10__b15 = 15,
    p_enc_09__b16,
    p_enc_10__b16 = 16,
    p_enc_09__b17,
    p_enc_10__b17 = 17,
    p_enc_09__b18,
    p_enc_10__b18 = 18,
    p_enc_09__b19,
    p_enc_10__b19 = 19,
    p_enc_09__b20,
    p_enc_10__b20 = 20,
    p_enc_09__b21,
    p_enc_10__b21 = 21,
    p_enc_09__b22,
    p_enc_10__b22 = 22,
    p_enc_09__b23,
    p_enc_10__b23 = 23,
    p_enc_09__b24,
    p_enc_10__b24 = 24,
    p_enc_09__b25,
    p_enc_10__b25 = 25,
    p_enc_09__b26,
    p_enc_10__b26 = 26,
    p_enc_09__b27,
    p_enc_10__b27 = 27,
    p_enc_09__b28,
    p_enc_10__b28 = 28,
    p_enc_09__b29,
    p_enc_10__b29 = 29,
    p_enc_09__b30,
    p_enc_10__b30 = 30,
    p_enc_09__b31,
    p_enc_10__b31 = 31,
    p_enc_09__b32,
    p_enc_10__b32 = 32,
);

/// P-ENC-01 (correct result): `to_face_ij_orientation()` gives the face, the leaf coordinates
/// `(i, j)` and the curve orientation that upstream's tables give.
///
/// `to_face_ij_orientation() == spec::face_ij_orientation(d)`.
#[kani::proof]
#[kani::unwind(31)]
fn p_enc_01() {
    let (c, d) = crate::domain::any_valid();
    let (f, i, j, o) = c.to_face_ij_orientation();
    let (mf, mi, mj, mo) = spec::face_ij_orientation(d);
    assert_eq!((f.as_u8(), i, j, o), (mf, mi as i32, mj as i32, mo));
    kani::cover!(d.level == 0, "face");
    kani::cover!(mo == 3, "swapped and inverted");
}

/// P-ENC-02 (correct result): `from_face_ij(f, i, j)` is the leaf at coordinates `(i, j)` on face
/// `f`.
///
/// `from_face_ij(f, i, j).id() == spec::leaf_from_face_ij(f, i, j)`, for `i, j` in `[0, 2^30)`.
#[kani::proof]
#[kani::unwind(31)]
fn p_enc_02() {
    let f = crate::domain::any_face();
    let i: u32 = kani::any();
    let j: u32 = kani::any();
    kani::assume(i < 1 << 30 && j < 1 << 30);
    let c = s2rst::s2::from_face_ij(s2rst::s2::Face::from_u8(f), i as i32, j as i32);
    assert_eq!(c.id(), spec::leaf_from_face_ij(f, i, j));
    kani::cover!(i == (1 << 30) - 1 && j == 0, "corner");
}

/// P-ENC-03 (round trip): converting a cell to face and leaf coordinates and back, then going up to
/// the cell's level, gives the cell.
///
/// With `(f, i, j, _) = to_face_ij_orientation()`:
/// `from_face_ij(f, i, j).parent_at_level(level()) == c`.
#[kani::proof]
#[kani::unwind(9)]
fn p_enc_03() {
    let (c, d) = crate::domain::any_valid();
    let (f, i, j, _) = c.to_face_ij_orientation();
    assert_eq!(
        s2rst::s2::from_face_ij(f, i, j).parent_at_level(c.level()),
        c
    );
    kani::cover!(d.level == 30, "leaf");
    kani::cover!(d.level == 0, "face");
}

/// P-ENC-04 (correct result): `get_center_si_ti()` is the cell's centre, in units of half a leaf.
///
/// `get_center_si_ti() == spec::center_si_ti(d)`.
#[kani::proof]
#[kani::unwind(31)]
fn p_enc_04() {
    let (c, d) = crate::domain::any_valid();
    let (f, si, ti) = c.get_center_si_ti();
    assert_eq!((f.as_u8(), si, ti), spec::center_si_ti(d));
    kani::cover!(d.level == 30, "leaf");
    kani::cover!(d.level == 29, "level 29 (the low-bit special case)");
}
