// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Point vector properties P-PV-* (data-model §4.1):
//! - **P-PV-01** (FAST round trip) by composition: the encoder agreement `p_pv_01_enc__n<k>`,
//!   P-PV-02, and the model's round trip `p_pv_mrt__n<k>`;
//! - **P-PV-02:** the agreement on the uncompressed branch and the unknown tags, at T029's
//!   bound: the empty input and every 1-byte input except tag 1;
//! - **P-PV-03** (the compact branch) is a tool limit (T029) and has no harness here.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::codec::{PtVec, check_agreement, check_encoder, check_model_round_trip};

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

agreement!(PtVec, "P-PV-02 (exact input): the point vector decoder agrees with upstream on the \
                   uncompressed format, and rejects the unknown format tags 2 to 7.";
    p_pv_02__b0 = 0, 2 => |_b, accepted| {
        kani::cover!(!accepted, "the empty input rejected");
    },
    p_pv_02__b1 = 1, 9 => |b, accepted| {
        kani::cover!(accepted, "no points");
        kani::cover!(!accepted && b[0] & 7 >= 2, "an unknown tag rejected");
        kani::cover!(!accepted && b[0] & 7 == 0, "truncated");
    },
);

encoder!(PtVec, "P-PV-01 (round trip): every vector of points round-trips through the \
                 uncompressed (FAST) format, bit for bit.";
    p_pv_01_enc__n0 = 0, 1, 11,
    p_pv_01_enc__n1 = 1, 25, 26,
    p_pv_01_enc__n2 = 2, 49, 50,
);

model_round_trip!(PtVec, "P-PV-01 (round trip): every vector of points round-trips through the \
                          uncompressed (FAST) format, bit for bit.";
    p_pv_mrt__n0 = 0, 2, 11,
    p_pv_mrt__n1 = 1, 26, 27,
    p_pv_mrt__n2 = 2, 50, 51,
);
