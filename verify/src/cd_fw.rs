// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Fixed-width integer properties P-FW-* (data-model §4.1), whole-domain and symbolic in
//! `len ≤ 8`, the helpers' documented precondition. Every Kani check is on, so they also
//! discharge the helpers' crash obligations (INVENTORY rows 1–3).
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use std::io::ErrorKind;

use s2rst::s2::encoded_uint_vector::{
    decode_uint_with_length, encode_uint_with_length, get_uint_with_length,
};

use crate::wire_domain::{discard, with_bytes};
use crate::wire_spec::ref_fixed;

/// Byte `j` of `x`, little-endian.
fn byte(x: u64, j: usize) -> u8 {
    (x >> (8 * j)) as u8
}

/// P-FW-01 (exact output): `encode_uint_with_length(v, len)` writes the low `len` bytes of `v`,
/// little-endian.
///
/// `encode_uint_with_length(v, len)` writes exactly `len` bytes, the low `len` bytes of `v`,
/// little-endian; for every `v` and every `len ≤ 8`.
#[kani::proof]
#[kani::unwind(9)]
fn p_fw_01() {
    let v: u64 = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 8);
    let mut w = Vec::with_capacity(8);
    assert!(encode_uint_with_length(&mut w, v, len).is_ok());
    assert!(w.len() == len, "the number of bytes written");
    for j in 0..8 {
        if j < len {
            assert!(w[j] == byte(v, j), "a byte differs");
        }
    }
    kani::cover!(len == 0, "zero bytes");
    kani::cover!(len == 8, "eight bytes");
}

/// One P-FW-02 member per data length `k ≤ 8`, for any `len ≤ k`.
macro_rules! p_fw_02 {
    ($($name:ident = $k:literal),* $(,)?) => { $(
        #[doc = concat!("P-FW-02 (exact input): `get_uint_with_length(d, len)` reads the first `len` bytes as a little-endian number.\n\n", "`get_uint_with_length(d, len)` is the little-endian value of `d[..len]`, for data of exactly ", stringify!($k), " bytes and every `len` up to it.")]
        #[kani::proof]
        #[kani::unwind(9)]
        fn $name() {
            let d: [u8; $k] = kani::any();
            let len: usize = kani::any();
            kani::assume(len <= $k);
            let got = get_uint_with_length(&d, len);
            let want = ref_fixed(&d, len).map(|(v, _)| v);
            assert!(want == Some(got), "a different value");
            kani::cover!(len == 0, "zero bytes");
            kani::cover!(len == $k, "the whole slice");
        }
    )* };
}

p_fw_02!(
    p_fw_02__d0 = 0,
    p_fw_02__d1 = 1,
    p_fw_02__d2 = 2,
    p_fw_02__d3 = 3,
    p_fw_02__d4 = 4,
    p_fw_02__d5 = 5,
    p_fw_02__d6 = 6,
    p_fw_02__d7 = 7,
    p_fw_02__d8 = 8,
);

/// One P-FW-03 member per input length, for any `len ≤ 8`. The covers see the result `r`,
/// `len` and what is left of the input.
macro_rules! p_fw_03 {
    ($($name:ident = $len:literal => |$r:ident, $n:ident, $input:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!("P-FW-03 (exact input): `decode_uint_with_length` returns that number and consumes exactly `len` bytes, or fails on a shorter input.\n\n", "On every input of exactly ", stringify!($len), " bytes and every `len ≤ 8`, `decode_uint_with_length` returns the value `get_uint_with_length` would and consumes exactly `len` bytes, or reports `UnexpectedEof` when fewer are left.")]
        #[kani::proof]
        #[kani::unwind(10)]
        fn $name() {
            let $n: usize = kani::any();
            kani::assume($n <= 8);
            with_bytes::<$len>(|b| {
                let mut $input = b;
                let $r = decode_uint_with_length(&mut $input, $n);
                let consumed = $len - $input.len();
                match (&$r, ref_fixed(b, $n)) {
                    (Ok(v), Some((w, k))) => {
                        assert!(*v == w, "a different value");
                        assert!(consumed == k, "a different number of bytes consumed");
                    }
                    (Err(e), None) => assert!(e.kind() == ErrorKind::UnexpectedEof, "not UnexpectedEof"),
                    _ => panic!("acceptance differs"),
                }
                let () = $covers;
                discard($r);
            });
        }
    )* };
}

p_fw_03!(
    p_fw_03__b0 = 0 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
    },
    p_fw_03__b1 = 1 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 1, "the whole input");
    },
    p_fw_03__b2 = 2 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 2, "the whole input");
    },
    p_fw_03__b3 = 3 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 3, "the whole input");
    },
    p_fw_03__b4 = 4 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 4, "the whole input");
    },
    p_fw_03__b5 = 5 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 5, "the whole input");
    },
    p_fw_03__b6 = 6 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 6, "the whole input");
    },
    p_fw_03__b7 = 7 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_err(), "too short");
        kani::cover!(r.is_ok() && len == 7, "the whole input");
    },
    p_fw_03__b8 = 8 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_ok() && len == 8, "the whole input");
        kani::cover!(r.is_ok() && len == 8 && input.len() == 0, "eight bytes read");
    },
    p_fw_03__b9 = 9 => |r, len, input| {
        kani::cover!(r.is_ok() && len == 0, "zero bytes");
        kani::cover!(r.is_ok() && len == 8 && input.len() == 1, "eight bytes read");
    },
);

/// P-FW-04 (round trip): writing a number in `len` bytes and reading it back gives the number,
/// whenever it fits.
///
/// Writing then reading returns `v`, for every `len ≤ 8` and every `v` that fits in `len` bytes,
/// through both readers.
#[kani::proof]
#[kani::unwind(10)]
fn p_fw_04() {
    let v: u64 = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 8);
    kani::assume(len == 8 || v >> (8 * len) == 0);
    let mut w = Vec::with_capacity(8);
    assert!(encode_uint_with_length(&mut w, v, len).is_ok());
    assert!(
        get_uint_with_length(&w, len) == v,
        "get_uint_with_length does not read it back"
    );
    let mut input = &w[..];
    let r = decode_uint_with_length(&mut input, len);
    assert!(
        r.as_ref().is_ok_and(|x| *x == v),
        "decode_uint_with_length does not read it back"
    );
    assert!(input.is_empty(), "bytes left over");
    kani::cover!(len == 0, "zero bytes, so v = 0");
    kani::cover!(len == 8 && v >> 56 != 0, "eight bytes");
    discard(r);
}
