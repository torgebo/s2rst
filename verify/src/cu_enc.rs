// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! P-UENC: binary encoding and decoding (FR-005, FR-017).
//!
//! `p_uenc_01__n<k>` and `p_uenc_04__b<L>` are shared (data-model §3): they also discharge
//! `C-CellUnion::encode` over `Seq(k)` and `C-CellUnion::decode` over every byte string of
//! exactly `L` bytes. The decoder is the one untrusted-input path (Constitution II).
//!
//! Upstream finding U-003: C++ `S2CellUnion::Decode` accepts ids that are not a valid union.
//! The library rejects them, a documented deviation (FR-017), and these proofs state the
//! library's behaviour.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{CellId, CellUnion};

use crate::union_domain::{any_seq, any_valid_union, impl_cell_range, out_ids_equal, with_bytes};
use crate::union_spec;

// The byte comparison is a `memcmp` loop over `9 + 8·n` bytes, so each member's unwind is
// that length plus 2.
macro_rules! p_uenc_01 {
    ($($name:ident = $n:literal, $unwind:literal => |$raw:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "P-UENC-01 (exact output): `encode` writes a version byte, the cell count, then each ",
            "id, as 8-byte little-endian numbers.\n\n",
            "`encode(U)` writes exactly the model's `encoding(U)` (version byte, ",
            "count, ids), `9 + 8·n` bytes, for every sequence of exactly ", stringify!($n),
            " ids (`Encode()`). Also discharges `C-CellUnion::encode` for `Seq(n)`."
        )]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            let (v, $raw) = any_seq::<$n>();
            let mut buf = Vec::new();
            let written = CellUnion::from_verbatim(v).encode(&mut buf);
            assert!(written.is_ok());
            assert_eq!(buf, union_spec::encoding(&$raw));
            assert_eq!(buf.len(), 9 + 8 * $n);
            $covers
        }
    )* };
}

p_uenc_01!(
    p_uenc_01__n0 = 0, 11 => |raw| { kani::cover!(raw.is_empty(), "the empty union"); },
    p_uenc_01__n1 = 1, 19 => |raw| {
        kani::cover!(CellId(raw[0]).is_valid(), "a valid id");
        kani::cover!(!CellId(raw[0]).is_valid(), "an invalid id");
    },
    p_uenc_01__n2 = 2, 27 => |raw| { kani::cover!(raw[0] > raw[1], "unsorted ids"); },
    p_uenc_01__n3 = 3, 35 => |raw| { kani::cover!(raw[0] == raw[2], "a repeated id"); },
);

// The decode covers below test id validity with the implementation's loop-free
// `CellId::is_valid`, not the model's 31-level `spec::decompose`: a cover only shows that an
// outcome is reachable, and the model's loop would force an unwind of 32 on the decoder.

/// The count field of a decoder input, if the input is long enough to hold one.
fn count_field(b: &[u8]) -> Option<u64> {
    let count: [u8; 8] = b.get(1..9)?.try_into().ok()?;
    Some(u64::from_le_bytes(count))
}

/// The `i`-th id of a decoder input, if present.
fn id_field(b: &[u8], i: usize) -> Option<u64> {
    let id: [u8; 8] = b.get(9 + 8 * i..17 + 8 * i)?.try_into().ok()?;
    Some(u64::from_le_bytes(id))
}

macro_rules! p_uenc_04 {
    ($unwind:literal; $($(#[$attr:meta])* $name:ident = $len:literal => |$b:ident, $r:ident| $covers:block),* $(,)?) => { $(
        $(#[$attr])*
        #[doc = concat!(
            "P-UENC-04 (no crash): decoding a `CellUnion` never panics or hangs, whatever the ",
            "bytes.\n\n",
            "`S2Decode for CellUnion` never panics or hangs on any input of exactly ",
            stringify!($len), " bytes (Constitution II). Also discharges ",
            "`C-CellUnion::decode` for `Bytes(", stringify!($len), ")`."
        )]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            with_bytes::<$len>(|$b| {
                let $r = CellUnion::decode(&mut &$b[..]);
                $covers
            });
        }
    )* };
}

// Room for one id: a one-cell union, or an error (truncated, an invalid id, ...).
p_uenc_04!(4;
    p_uenc_04__b17 = 17 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b18 = 18 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b19 = 19 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b20 = 20 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b21 = 21 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b22 = 22 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b23 = 23 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
    p_uenc_04__b24 = 24 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 1), "one cell");
        kani::cover!(
            r.is_err() && count_field(b) == Some(1) && id_field(b, 0).is_some_and(|x| !CellId(x).is_valid()),
            "rejected: an invalid id"
        );
    },
);

// Room for two ids: the first length that reaches FR-017's order check. It needs about
// 7.8 GB (92 s), over the 4 GB per-change ceiling, so it runs in the weekly slow tier with an
// 8 GB ceiling (`make verify-slow F=union`; research R15).
p_uenc_04!(5;
    #[cfg(feature = "slow")]
    p_uenc_04__b25 = 25 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|u| u.num_cells() == 2), "two cells");
        kani::cover!(
            r.is_err()
                && count_field(b) == Some(2)
                && id_field(b, 0).is_some_and(|x| CellId(x).is_valid())
                && id_field(b, 1).is_some_and(|x| CellId(x).is_valid()),
            "rejected: two valid ids that are not a valid union"
        );
    },
);

macro_rules! p_uenc_02 {
    ($($(#[$attr:meta])* $name:ident = $n:literal, $len:literal => |$ds:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!(
            "P-UENC-02 (round trip): decoding an encoded valid union gives the same ids, ",
            "normalised or not.\n\n",
            "`decode(encode(U)) = U` for every valid union of exactly ",
            stringify!($n), " cells, normalised or not (`Decode()`, `Encode()`; FR-017). ",
            "Proven as `decode(encoding(U)) = U` on the model's byte layout, built on the stack; ",
            "with P-UENC-01 (`encode(U) = encoding(U)`) that is the statement. Decoding bytes that ",
            "`encode` wrote into a `Vec` ran out of memory even for one cell."
        )]
        $(#[$attr])*
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            let (v, $ds) = any_valid_union::<$n>();
            let mut bytes = [0u8; $len];
            bytes[0] = union_spec::ENCODING_VERSION;
            bytes[1..9].copy_from_slice(&($n as u64).to_le_bytes());
            for i in 0..$n {
                bytes[9 + 8 * i..17 + 8 * i].copy_from_slice(&v[i].0.to_le_bytes());
            }
            let decoded = CellUnion::decode(&mut &bytes[..]);
            assert!(decoded.is_ok());
            if let Ok(u) = decoded {
                let ids = u.cell_ids();
                assert_eq!(ids.len(), $n);
                for i in 0..$n {
                    assert_eq!(ids[i], v[i]);
                }
            }
            $covers
        }
    )* };
}

p_uenc_02!(
    p_uenc_02__n0 = 0, 9 => |_ds| { kani::cover!(true, "the empty union"); },
    p_uenc_02__n1 = 1, 17 => |ds| { kani::cover!(ds[0].level == 0, "a face cell"); },
    p_uenc_02__n2 = 2, 25 => |ds| { kani::cover!(ds[0].face != ds[1].face, "two faces"); },
    p_uenc_02__n3 = 3, 33 => |ds| { kani::cover!(ds[2].level == 30, "a leaf"); },
    // Four cells (a sibling group) runs in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_uenc_02__n4 = 4, 41 => |ds| {
        kani::cover!(
            union_spec::sibling_group(ds[0], ds[1], ds[2], ds[3]),
            "four siblings, decoded verbatim"
        );
    },
);

/// P-UENC-03's assertion for an input that can hold at most `MAX` ids: the decoder and the
/// model accept the same inputs, with the same ids. The model reads id validity and ranges
/// through the implementation's `is_valid`, `range_min` and `range_max`, which 001 proved
/// equal to the model's (P-VAL-03, P-RANGE-01); `union_spec::decode`, its search-based
/// version, is the one checked against C++.
fn decode_agrees<const MAX: usize>(b: &[u8]) -> bool {
    let got = CellUnion::decode(&mut &b[..]);
    let want = union_spec::decode_with(b, impl_cell_range);
    match (got, want) {
        (Ok(u), Some(ids)) => {
            assert!(out_ids_equal::<MAX>(u.cell_ids(), &ids));
            true
        }
        (Err(_), None) => false,
        _ => panic!("the decoder and the model disagree"),
    }
}

macro_rules! p_uenc_03 {
    ($($(#[$attr:meta])* $name:ident = $len:literal, $max:literal => |$b:ident, $ok:ident| $covers:block),* $(,)?) => { $(
        $(#[$attr])*
        #[doc = concat!(
            "P-UENC-03 (exact input): `CellUnion::decode` accepts exactly the encodings of valid ",
            "unions, returns their ids unchanged, and rejects everything else.\n\n",
            "`decode(B)` agrees with the model's `decode(B)` for every input of ",
            "exactly ", stringify!($len), " bytes: it accepts exactly the encodings of valid ",
            "unions, returns their ids unchanged, and rejects everything else (`Decode()` with ",
            "the FR-017 deviation; U-003)."
        )]
        #[kani::proof]
        #[kani::unwind(5)]
        fn $name() {
            with_bytes::<$len>(|$b| {
                let $ok = decode_agrees::<$max>($b);
                $covers
            });
        }
    )* };
}

p_uenc_03!(
    p_uenc_03__b0 = 0, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b1 = 1, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b2 = 2, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b3 = 3, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b4 = 4, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b5 = 5, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b6 = 6, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b7 = 7, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b8 = 8, 0 => |_b, ok| { kani::cover!(!ok, "rejected"); },
    p_uenc_03__b9 = 9, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b10 = 10, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b11 = 11, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b12 = 12, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b13 = 13, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b14 = 14, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b15 = 15, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    p_uenc_03__b16 = 16, 0 => |_b, ok| {
        kani::cover!(ok, "accepted: the empty union");
        kani::cover!(!ok, "rejected");
    },
    // Lengths 17–25 (one or two cells) run in the weekly slow tier (T044).
    #[cfg(feature = "slow")]
    p_uenc_03__b17 = 17, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b18 = 18, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b19 = 19, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b20 = 20, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b21 = 21, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b22 = 22, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b23 = 23, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
    #[cfg(feature = "slow")]
    p_uenc_03__b24 = 24, 1 => |b, ok| {
        kani::cover!(ok, "accepted: one cell");
        kani::cover!(!ok && count_field(b) == Some(1), "rejected: one id");
    },
);

// P-UENC-03 stops at 24 bytes. At 25 bytes (two cells, the first length that reaches FR-017's
// order check) it ran out of memory at 8 GB, both as one harness and split by the count
// field (2026-10-04). The crash proof `p_uenc_04__b25` does reach 25 bytes, weekly; the
// order check's functional evidence is the FR-017 regression tests, `decode_matches_cpp` and
// the fuzz target.
