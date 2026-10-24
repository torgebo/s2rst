// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! String vector properties P-SV-* (data-model §4.1). The decoder is a tool limit (T029), so
//! P-SV-01 and P-SV-02 are not proven on the K track: their every-length proofs are the Verus
//! track's (`verify/deductive/src/string_vector.rs`). Here:
//! - **P-SV-03:** the encoder writes the canonical encoding, the offsets' integer vector and
//!   then the data, for strings of up to 2 bytes (research R5);
//! - the model's round trip `p_sv_mrt__n<k>`, the third leg of P-SV-01.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use crate::codec::{StrVec, check_encoder, check_model_round_trip};

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

encoder!(StrVec, "P-SV-03 (exact output): the string vector's encoding is the offsets' integer \
                  vector followed by the strings' bytes.";
    p_sv_03__n0 = 0, 1, 11,
    p_sv_03__n1 = 1, 4, 11,
    p_sv_03__n2 = 2, 7, 11,
);

model_round_trip!(StrVec, "P-SV-01 (round trip): every vector of byte strings round-trips through \
                           the string vector encoding.";
    p_sv_mrt__n0 = 0, 2, 11,
    p_sv_mrt__n1 = 1, 5, 11,
    #[cfg(feature = "slow")] // weekly: out of memory at 4 GiB; 124.2 s at 8 GiB (T046)
    p_sv_mrt__n2 = 2, 8, 11,
);
