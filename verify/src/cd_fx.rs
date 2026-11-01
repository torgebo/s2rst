// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Fixed-size codec properties P-FX-* (data-model §4.1): the round trips P-FX-01 … 05, run
//! directly (encode a valid value, decode it, compare), which fits for fixed sizes (research
//! R0), and the agreements P-FX-06 … 10 over every input of up to the encoded length plus one
//! byte. With exact consumption that covers every longer input (FR-008). Every Kani check is
//! on, so the agreements also discharge the decoders' crash obligations.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

#[cfg(feature = "slow")]
use crate::codec::check_model_round_trip;
use crate::codec::{
    CapC, CellC, CellIdC, Codec, LatLngC, PointC, PointRegionC, RectC, check_agreement,
    check_encoder,
};

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

/// The direct round trip: a valid value's encoding is exactly `K` bytes, and decoding it with
/// one more byte after it returns the value and leaves that byte unread.
fn round_trip<C: Codec, const K: usize>() {
    let v = C::any_value::<1>();
    kani::assume(C::encodable(&v));
    let mut w = Vec::with_capacity(K + 1);
    C::encode(&v, &mut w);
    assert!(w.len() == K, "the encoded length");
    let extra: u8 = kani::any();
    w.push(extra);
    let mut input = &w[..];
    match C::decode(&mut input) {
        Ok(x) => assert!(C::same::<1>(&v, &x), "a different value"),
        Err(e) => {
            core::mem::forget(e);
            panic!("the decoder rejects a valid value's encoding");
        }
    }
    assert!(input.len() == 1, "the following byte was consumed");
}

/// P-FX-01 (round trip): every valid `CellId` round-trips through its 8-byte encoding.
///
/// Every valid `CellId` round-trips.
#[kani::proof]
#[kani::unwind(9)]
fn p_fx_01() {
    round_trip::<CellIdC, 8>();
}

/// P-FX-02 (round trip): every `Point` round-trips through its encoding, bit for bit.
///
/// Every `Point`, any bit pattern, round-trips.
#[kani::proof]
#[kani::unwind(9)]
fn p_fx_02() {
    round_trip::<PointC, 24>();
}

/// P-FX-03 (round trip): every valid `Cap` round-trips through its encoding.
///
/// Encoder half: for every valid `Cap`, the library writes the canonical encoding.
/// With P-FX-08 (`__b33`) and `p_fx_03_mrt`, this proves the round trip by composition
/// (research R4): the direct round trip, which sends the float validity check through the
/// library's writer and reader in one harness, timed out at 5 minutes (T048).
#[kani::proof]
#[kani::unwind(33)]
fn p_fx_03_enc() {
    check_encoder::<CapC, 1, 32>();
}

/// P-FX-03 (round trip): every valid `Cap` round-trips through its encoding.
///
/// The model's leg: the reference description decodes its own encoding of every valid `Cap`,
/// whatever follows it, consuming exactly 32 bytes.
///
/// Weekly tier: over 5 minutes at 4 GiB; 1,552.8 s at 8 GiB (T048).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(34)]
fn p_fx_03_mrt() {
    check_model_round_trip::<CapC, 1, 33>();
}

/// P-FX-04 (round trip): every valid `Rect` round-trips through its encoding.
///
/// Every valid `Rect` round-trips.
#[kani::proof]
#[kani::unwind(9)]
fn p_fx_04() {
    round_trip::<RectC, 33>();
}

/// P-FX-05 (round trip): every `Cell` round-trips through its encoding.
///
/// Every `Cell` round-trips.
#[kani::proof]
#[kani::unwind(9)]
fn p_fx_05() {
    round_trip::<CellC, 8>();
}

agreement!(CellIdC, "P-FX-06 (exact input): the `CellId` decoder reads 8 bytes and rejects ids \
                     that are not valid cells (a stricter check than upstream's).";
    p_fx_06__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_06__b8 = 8, 32 => |_b, accepted| {
        kani::cover!(accepted, "a valid id");
        kani::cover!(!accepted, "an invalid id rejected");
    },
    p_fx_06__b9 = 9, 32 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);

agreement!(PointC, "P-FX-07 (exact input): the `Point` decoder reads three 8-byte floats, any \
                    bits.";
    p_fx_07__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b8 = 8, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b9 = 9, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b10 = 10, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b11 = 11, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b12 = 12, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b13 = 13, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b14 = 14, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b15 = 15, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b16 = 16, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b17 = 17, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b18 = 18, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b19 = 19, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b20 = 20, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b21 = 21, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b22 = 22, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b23 = 23, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_07__b24 = 24, 5 => |_b, accepted| {
        kani::cover!(accepted, "every 24 bytes are a point");
    },
    p_fx_07__b25 = 25, 5 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);

// Upstream `S2Cap::Decode` accepts invalid caps in release builds and aborts in debug builds
// (U-006, tasks.md T077); the library rejects them, and the reference states that.
agreement!(CapC, "P-FX-08 (exact input): the `Cap` decoder reads a centre and a radius, and \
                  rejects caps that are not valid (a stricter check than upstream's).";
    p_fx_08__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b8 = 8, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b9 = 9, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b10 = 10, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b11 = 11, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b12 = 12, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b13 = 13, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b14 = 14, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b15 = 15, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b16 = 16, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b17 = 17, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b18 = 18, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b19 = 19, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b20 = 20, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b21 = 21, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b22 = 22, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b23 = 23, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b24 = 24, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b25 = 25, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b26 = 26, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b27 = 27, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b28 = 28, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b29 = 29, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b30 = 30, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_08__b31 = 31, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes at 4 GiB; 1,172.0 s at 8 GiB (T048)
    p_fx_08__b32 = 32, 5 => |_b, accepted| {
        kani::cover!(accepted, "a valid cap");
        kani::cover!(!accepted, "an invalid cap rejected");
    },
    #[cfg(feature = "slow")] // weekly: over 5 minutes at 4 GiB; 1,440.3 s at 8 GiB (T048)
    p_fx_08__b33 = 33, 5 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);

agreement!(RectC, "P-FX-09 (exact input): the `Rect` decoder reads a version byte and four \
                   bounds, and rejects rectangles that are not valid, as upstream does.";
    p_fx_09__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b8 = 8, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b9 = 9, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b10 = 10, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b11 = 11, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b12 = 12, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b13 = 13, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b14 = 14, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b15 = 15, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b16 = 16, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b17 = 17, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b18 = 18, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b19 = 19, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b20 = 20, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b21 = 21, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b22 = 22, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b23 = 23, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b24 = 24, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b25 = 25, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b26 = 26, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b27 = 27, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b28 = 28, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b29 = 29, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b30 = 30, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b31 = 31, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b32 = 32, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_09__b33 = 33, 5 => |b, accepted| {
        kani::cover!(accepted, "a valid rectangle");
        kani::cover!(!accepted && b[0] >= 2, "version 2 or above rejected");
        kani::cover!(!accepted && b[0] <= 1, "invalid bounds rejected");
    },
    p_fx_09__b34 = 34, 5 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);

agreement!(CellC, "P-FX-10 (exact input): the `Cell` decoder reads a cell id and rejects ids that \
                   are not valid cells.";
    p_fx_10__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_10__b8 = 8, 32 => |_b, accepted| {
        kani::cover!(accepted, "a cell");
        kani::cover!(!accepted, "an invalid id rejected (U-004)");
    },
    p_fx_10__b9 = 9, 32 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);

// ---- spec 014: `PointRegion` and `LatLng` --------------------------------------------------------

/// P-FX-11 (round trip): every unit-length `PointRegion` round-trips through its 25-byte
/// encoding, bit for bit, leaving a following byte unread.
///
/// Every point that `Point::is_unit` accepts. Under `pure_is_unit` (spec 008, L-CP-04, -07): the
/// encoder's domain and the decoder's check read one answer per point (research R1).
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn p_fx_11() {
    crate::stubs::reset();
    round_trip::<PointRegionC, 25>();
}

/// P-FX-11 (round trip), the encoder: for every unit-length `PointRegion`, the library writes
/// upstream's 25 bytes (version 1, then the point).
#[kani::proof]
#[kani::unwind(26)]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn p_fx_11_enc() {
    crate::stubs::reset();
    check_encoder::<PointRegionC, 1, 25>();
}

/// P-FX-12 (round trip): every `LatLng`, any bit patterns, round-trips through its 16-byte
/// encoding, bit for bit, leaving a following byte unread.
///
/// Every pair of `f64`.
#[kani::proof]
#[kani::unwind(9)]
fn p_fx_12() {
    round_trip::<LatLngC, 16>();
}

/// P-FX-12 (round trip), the encoder: for every `LatLng`, the library writes upstream's 16 bytes
/// (the latitude, then the longitude).
#[kani::proof]
#[kani::unwind(17)]
fn p_fx_12_enc() {
    check_encoder::<LatLngC, 1, 16>();
}

agreement!(PointRegionC, "P-FX-13 (exact input): the `PointRegion` decoder reads a version byte \
                          of at most 1 and three 8-byte floats, and rejects a point that is not \
                          unit length (relative to `Point::is_unit`, under `pure_is_unit`).";
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b0 = 0, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b1 = 1, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b2 = 2, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b3 = 3, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b4 = 4, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b5 = 5, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b6 = 6, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b7 = 7, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b8 = 8, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b9 = 9, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b10 = 10, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b11 = 11, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b12 = 12, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b13 = 13, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b14 = 14, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b15 = 15, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b16 = 16, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b17 = 17, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b18 = 18, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b19 = 19, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b20 = 20, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b21 = 21, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b22 = 22, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b23 = 23, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b24 = 24, 5 => |b, accepted| {
        let _ = b;
        kani::cover!(!accepted, "truncated");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b25 = 25, 11 => |b, accepted| {
        kani::cover!(accepted && b[0] == 0, "version 0 accepted");
        kani::cover!(accepted && b[0] == 1, "version 1 accepted");
        kani::cover!(!accepted && b[0] >= 2, "version 2 or above rejected");
        kani::cover!(!accepted && b[0] <= 1, "a point off unit length rejected");
    },
    #[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
    p_fx_13__b26 = 26, 11 => |b, accepted| {
        let _ = b;
        kani::cover!(accepted, "one byte left unread");
    },
);

agreement!(LatLngC, "P-FX-14 (exact input): the `LatLng` decoder reads two 8-byte floats, any bits.";
    p_fx_14__b0 = 0, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b1 = 1, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b2 = 2, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b3 = 3, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b4 = 4, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b5 = 5, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b6 = 6, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b7 = 7, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b8 = 8, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b9 = 9, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b10 = 10, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b11 = 11, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b12 = 12, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b13 = 13, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b14 = 14, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b15 = 15, 5 => |_b, accepted| {
        kani::cover!(!accepted, "truncated");
    },
    p_fx_14__b16 = 16, 5 => |_b, accepted| {
        kani::cover!(accepted, "accepted");
    },
    p_fx_14__b17 = 17, 5 => |_b, accepted| {
        kani::cover!(accepted, "one byte left unread");
    },
);
