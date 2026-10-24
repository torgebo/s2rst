// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Integer vector properties P-UV-* and the varint header property P-VAR-01 (data-model §4.1).
//!
//! - **P-UV-01/02** (round trip, contract part 1) by composition: the encoder agreement
//!   `p_uv_0{1,2}_enc__n<k>`, the decoder agreement P-UV-03/04, and the model's round trip
//!   `p_uv_mrt_u{64,32}__n<k>` (research R4).
//! - **P-UV-03/04** (agreement, parts 2–4): `p_uv_0{3,4}__b0 … __b11`, every input of up to 11
//!   bytes. Every Kani check is on, so they also discharge the decoders' crash obligations.
//! - **P-VAR-01** is checked inside them: the reference reads the header with `lib_varint`,
//!   which is upstream's varint since the D-003 fix.
//! - **P-UV-05/06:** the width and the length of the encoder's output.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::encoded_uint_vector::{encode_uint_vector_u32, encode_uint_vector_u64};

use crate::cd_crash::continued;
#[cfg(feature = "slow")]
use crate::codec::check_agreement_on;
use crate::codec::{
    Codec, ENCODING_MAX, U32Vec, U64Vec, check_agreement, check_encoder, check_model_round_trip,
    same_bytes,
};
use crate::wire_domain::{any_u32s, any_u64s};
use crate::wire_spec::enc_varint;

/// One agreement member per `name = length => |input, accepted| covers`.
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

/// One member per `name = count, encoding bound, unwind` for the generic bodies.
macro_rules! encoder {
    ($codec:ty, $what:literal; $($name:ident = $n:literal, $e:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nFor every vector of exactly ", stringify!($n), " values, the library writes the canonical encoding (contract part 2, encoder half).")]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            check_encoder::<$codec, $n, $e>();
        }
    )* };
}

macro_rules! model_round_trip {
    ($codec:ty, $what:literal; $($name:ident = $n:literal, $e:literal, $unwind:literal),* $(,)?) => { $(
        #[doc = concat!($what, "\n\nThe reference description decodes its own encoding of every vector of exactly ", stringify!($n), " values, whatever follows it, consuming exactly the encoding (research R4, the third leg).")]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            check_model_round_trip::<$codec, $n, $e>();
        }
    )* };
}

agreement!(U64Vec, "P-UV-03 (exact input): the `u64` integer vector decoder accepts and returns \
                    what upstream does, and consumes the same bytes.";
    p_uv_03__b0 = 0, 2 => |_b, accepted| {
        kani::cover!(!accepted, "the empty input rejected");
    },
    p_uv_03__b1 = 1, 2 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
    },
    p_uv_03__b2 = 2, 3 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 8, "the longest vector, 1 one-byte elements");
    },
    p_uv_03__b3 = 3, 4 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 16, "the longest vector, 2 one-byte elements");
    },
    p_uv_03__b4 = 4, 5 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 24, "the longest vector, 3 one-byte elements");
    },
    p_uv_03__b5 = 5, 6 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 32, "the longest vector, 4 one-byte elements");
        kani::cover!(!accepted && continued(b, 4) && b[4] >= 2 && b[4] < 0x80, "a count over the limit (5-byte header)");
    },
    p_uv_03__b6 = 6, 7 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 40, "the longest vector, 5 one-byte elements");
    },
    p_uv_03__b7 = 7, 8 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 48, "the longest vector, 6 one-byte elements");
    },
    p_uv_03__b8 = 8, 9 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 56, "the longest vector, 7 one-byte elements");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_03__b9 = 9, 10 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 64, "the longest vector, 8 one-byte elements");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_03__b10 = 10, 11 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 72, "the longest vector, 9 one-byte elements");
        kani::cover!(!accepted && continued(b, 9) && b[9] > 1 && b[9] < 0x80, "a varint whose tenth byte is above 1 (P-VAR-01, D-003)");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_03__b11 = 11, 12 => |b, accepted| {
        kani::cover!(accepted && b[0] < 8, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 80, "the longest vector, 10 one-byte elements");
        kani::cover!(!accepted && continued(b, 4) && b[4] >= 2 && b[4] < 0x80, "a count over the limit (5-byte header)");
        kani::cover!(!accepted && continued(b, 9) && b[9] > 1 && b[9] < 0x80, "a varint whose tenth byte is above 1 (P-VAR-01, D-003)");
    },
);

agreement!(U32Vec, "P-UV-04 (exact input): the `u32` integer vector decoder accepts and returns \
                    what upstream does, and consumes the same bytes.";
    p_uv_04__b0 = 0, 2 => |_b, accepted| {
        kani::cover!(!accepted, "the empty input rejected");
    },
    p_uv_04__b1 = 1, 2 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
    },
    p_uv_04__b2 = 2, 3 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 4, "the longest vector, 1 one-byte elements");
    },
    p_uv_04__b3 = 3, 4 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 8, "the longest vector, 2 one-byte elements");
    },
    p_uv_04__b4 = 4, 5 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 12, "the longest vector, 3 one-byte elements");
        kani::cover!(!accepted && continued(b, 3) && b[3] >= 96 && b[3] < 0x80, "a count over the limit (4-byte header)");
    },
    p_uv_04__b5 = 5, 6 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 16, "the longest vector, 4 one-byte elements");
    },
    p_uv_04__b6 = 6, 7 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 20, "the longest vector, 5 one-byte elements");
    },
    p_uv_04__b7 = 7, 8 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 24, "the longest vector, 6 one-byte elements");
    },
    p_uv_04__b8 = 8, 9 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 28, "the longest vector, 7 one-byte elements");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_04__b9 = 9, 10 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 32, "the longest vector, 8 one-byte elements");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_04__b10 = 10, 11 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 36, "the longest vector, 9 one-byte elements");
        kani::cover!(!accepted && continued(b, 9) && b[9] > 1 && b[9] < 0x80, "a varint whose tenth byte is above 1 (P-VAR-01, D-003)");
    },
    #[cfg(feature = "slow")] // weekly: over the per-change leg's budget (T044-T048)
    p_uv_04__b11 = 11, 12 => |b, accepted| {
        kani::cover!(accepted && b[0] < 4, "an empty vector");
        kani::cover!(!accepted, "rejected");
        kani::cover!(accepted && b[0] == 40, "the longest vector, 10 one-byte elements");
        kani::cover!(!accepted && continued(b, 3) && b[3] >= 96 && b[3] < 0x80, "a count over the limit (4-byte header)");
        kani::cover!(!accepted && continued(b, 9) && b[9] > 1 && b[9] < 0x80, "a varint whose tenth byte is above 1 (P-VAR-01, D-003)");
    },
);

encoder!(U64Vec, "P-UV-01 (round trip): every vector of `u64` round-trips through the integer \
                  vector encoding.";
    p_uv_01_enc__n0 = 0, 1, 11,
    p_uv_01_enc__n1 = 1, 9, 11,
    p_uv_01_enc__n2 = 2, 17, 18,
);

encoder!(U32Vec, "P-UV-02 (round trip): every vector of `u32` round-trips through the integer \
                  vector encoding.";
    p_uv_02_enc__n0 = 0, 1, 11,
    p_uv_02_enc__n1 = 1, 5, 11,
    p_uv_02_enc__n2 = 2, 9, 11,
);

model_round_trip!(U64Vec, "P-UV-01 (round trip): every vector of `u64` round-trips through the \
                           integer vector encoding.";
    p_uv_mrt_u64__n0 = 0, 2, 11,
    p_uv_mrt_u64__n1 = 1, 10, 11,
    p_uv_mrt_u64__n2 = 2, 18, 19,
);

model_round_trip!(U32Vec, "P-UV-02 (round trip): every vector of `u32` round-trips through the \
                           integer vector encoding.";
    p_uv_mrt_u32__n0 = 0, 2, 11,
    p_uv_mrt_u32__n1 = 1, 6, 11,
    p_uv_mrt_u32__n2 = 2, 10, 11,
);

/// The width the header declares, the OR of the values with 1, and the encoding.
fn width_and_or(w: &[u8], mask: u8, or: u64) -> (usize, u64) {
    (usize::from(w[0] & mask) + 1, or | 1)
}

/// One P-UV-05 and P-UV-06 member per element type and count.
macro_rules! shape {
    ($($name:ident = $gen:ident, $enc:ident, $n:literal, $size:literal, $mask:literal),* $(,)?) => { $(
        #[doc = concat!("P-UV-05 (exact output) and P-UV-06 (exact output): the integer vector encoder stores each value in the fewest bytes that hold all of them, and writes only the header and those bytes.\n\n", "For `", stringify!($enc), "`: for every vector of exactly ", stringify!($n), " values, the element width is the minimum number of bytes that holds `1 | OR(v)`, and the encoded length is `|varint(header)| + n · width`.")]
        #[kani::proof]
        #[kani::unwind(11)]
        fn $name() {
            let v = $gen::<$n>();
            let mut w = Vec::with_capacity(ENCODING_MAX);
            assert!($enc(&v, &mut w).is_ok());
            let mut or = 0u64;
            for x in &v {
                or |= u64::from(*x);
            }
            let (width, or) = width_and_or(&w, $mask, or);
            assert!(width == 8 || or >> (8 * width) == 0, "the width does not hold the values");
            assert!(width == 1 || or >> (8 * (width - 1)) != 0, "a narrower width would hold the values");
            let header = enc_varint(($n as u64) * $size | (width as u64 - 1));
            assert!(w.len() == header.len() + $n * width, "the encoded length");
            kani::cover!(width == 1, "width 1");
            kani::cover!(width == $size, "the widest width");
        }
    )* };
}

/// P-UV-01 (round trip) and P-VAR-01 (exact input): the encoder leg of the `u64` vector round
/// trip with a varint header of two bytes.
///
/// On one concrete input: sixteen zeros, whose header is 128, the smallest value that takes two
/// varint bytes. The `__n<k>` members stop at 2 values, whose headers fit one byte. With any
/// symbolic value among the sixteen, the element width is symbolic and this ran out of memory at 8
/// GiB (T044), so the domain is this one vector: in effect a unit test, run with every check on.
#[kani::proof]
#[kani::unwind(19)]
fn p_uv_01_enc__n16_zeros() {
    let v = vec![0u64; 16];
    let mut w = Vec::with_capacity(ENCODING_MAX);
    U64Vec::encode(&v, &mut w);
    let want = U64Vec::reference_encode(&v);
    assert!(
        same_bytes::<18>(&w, &want),
        "the encoding differs from the reference"
    );
    kani::cover!(
        w.len() == 18 && w[0] == 0x80 && w[1] == 1,
        "the header 128, in two bytes"
    );
}

/// P-UV-03 (exact input) and P-VAR-01 (exact input): the `u64` vector decoder agrees with
/// upstream on inputs whose varint header takes two bytes.
///
/// On every 18-byte input whose header is `0x80 | 8k`, `0x01`, which declares `16 + k` one-byte
/// elements (`k < 16`), the library and the reference agree. Only `k = 0` fits in the input. The
/// `__b<L>` members stop at 11 bytes, too few for any two-byte header that decodes.
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(19)]
fn p_uv_03__h2_b18() {
    let mut b: [u8; 18] = kani::any();
    kani::assume(b[0] >= 0x80 && b[0] & 7 == 0);
    b[1] = 1;
    check_agreement_on::<U64Vec, 18>(&b, |b, accepted| {
        kani::cover!(
            accepted && b[0] == 0x80,
            "sixteen elements after a two-byte header"
        );
        kani::cover!(!accepted, "more elements declared than the input holds");
    });
}

shape!(
    p_uv_05_u64__n1 = any_u64s,
    encode_uint_vector_u64,
    1,
    8,
    7,
    p_uv_05_u64__n2 = any_u64s,
    encode_uint_vector_u64,
    2,
    8,
    7,
    p_uv_05_u32__n1 = any_u32s,
    encode_uint_vector_u32,
    1,
    4,
    3,
    p_uv_05_u32__n2 = any_u32s,
    encode_uint_vector_u32,
    2,
    4,
    3,
);
