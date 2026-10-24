// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Cell-identifier vector properties P-CV-* (data-model §4.1):
//! - **P-CV-01** (round trip) by composition: P-CV-03's encoder agreement, P-CV-02, and the
//!   model's round trip `p_cv_mrt__n<k>` (research R4);
//! - **P-CV-02:** the agreement, every input of up to 11 bytes, including the extended shift
//!   code and the rejection of shifts above 56. It also discharges the decoder's crash
//!   obligation;
//! - **P-CV-03:** the encoder writes upstream's encoding (`enc_cellid_vector`, upstream's
//!   search for the base length and the shift).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::codec::{IdVec, check_agreement, check_encoder, check_model_round_trip};

/// One agreement member per `name = length, unwind => |input, accepted| covers`.
macro_rules! agreement {
    ($codec:ty, $what:literal; $($(#[$attr:meta])* $name:ident = $len:literal, $unwind:literal => |$b:ident, $acc:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nOn every input of exactly ", stringify!($len), " bytes, the library's decoder and the reference description agree on acceptance, value and bytes consumed (contract parts 2–4).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            check_agreement::<$codec, $len>(|$b, $acc| $covers);
        }
    )* };
}

/// One member per `name = count, encoding bound, unwind`.
macro_rules! encoder {
    ($codec:ty, $what:literal; $($(#[$attr:meta])* $name:ident = $n:literal, $e:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nFor every sequence of exactly ", stringify!($n), " values, the library writes the canonical encoding (contract part 2, encoder half).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            check_encoder::<$codec, $n, $e>();
        }
    )* };
}

macro_rules! model_round_trip {
    ($codec:ty, $what:literal; $($(#[$attr:meta])* $name:ident = $n:literal, $e:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nThe reference description decodes its own encoding of every sequence of exactly ", stringify!($n), " values, whatever follows it, consuming exactly the encoding (research R4, the third leg).")]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            check_model_round_trip::<$codec, $n, $e>();
        }
    )* };
}

agreement!(IdVec, "P-CV-02 (exact input): the cell-id vector decoder accepts and returns what \
                   upstream does, including the extended shift code, and rejects shifts above 56.";
    p_cv_02__b0 = 0, 2 => |_b, accepted| {
        kani::cover!(!accepted, "too short");
    },
    p_cv_02__b1 = 1, 2 => |_b, accepted| {
        kani::cover!(!accepted, "too short");
    },
    p_cv_02__b2 = 2, 3 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
    },
    p_cv_02__b3 = 3, 4 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
        kani::cover!(!accepted && b[0] >> 3 == 31 && b[1] > 27, "an extended shift above 56 rejected");
    },
    p_cv_02__b4 = 4, 5 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
        kani::cover!(accepted && b[0] >> 3 == 31, "the extended shift code");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_cv_02__b5 = 5, 6 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_cv_02__b6 = 6, 7 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_cv_02__b7 = 7, 8 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes per change (T045)
    p_cv_02__b8 = 8, 9 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes per change (T045)
    p_cv_02__b9 = 9, 10 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
        kani::cover!(accepted && b[0] & 7 == 7, "base length 7");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes per change (T045)
    p_cv_02__b10 = 10, 11 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
        kani::cover!(accepted && b[0] & 7 == 7, "base length 7");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes per change (T045)
    p_cv_02__b11 = 11, 12 => |b, accepted| {
        kani::cover!(accepted && b[0] & 7 == 0, "base length 0");
        kani::cover!(!accepted, "rejected or truncated");
        kani::cover!(accepted && b[0] & 7 == 7, "base length 7");
    },
);

encoder!(IdVec, "P-CV-03 (exact output): the cell-id vector encoder chooses the base, the shift \
                 and the delta width as upstream's search does.";
    p_cv_03__n0 = 0, 2, 11,
);
// `__n1` and `__n2` run out of memory at 8 GiB (T045): they are `cd_xlarge::p_cv_03__n1`, `__n2`.

model_round_trip!(IdVec, "P-CV-01 (round trip): every vector of cell ids, of any bits, \
                          round-trips through the cell-id vector encoding.";
    p_cv_mrt__n0 = 0, 3, 11,
    #[cfg(feature = "slow")] // weekly: out of memory at 4 GiB; 240.9 s at 8 GiB (T045)
    p_cv_mrt__n1 = 1, 19, 20,
);
// `__n2` runs out of memory at 8 GiB too: it is `cd_xlarge::p_cv_mrt__n2`.
