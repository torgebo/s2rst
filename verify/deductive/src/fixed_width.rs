// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The library's fixed-width little-endian helpers in `encoded_uint_vector.rs`,
//! `encode_uint_with_length`, `get_uint_with_length`, `decode_uint_with_length`,
//! `byte_length_u32` and `byte_length_u64`, restated for Verus (spec 003 FR-010,
//! P-FW-01 … P-FW-04, P-UV-05).
//!
//! Permitted differences (`mirrors.toml`): a slice and a cursor for `&mut dyn Read`, a
//! `Vec<u8>` for `&mut dyn Write`, `None` for an `io::Error`, explicit byte loops for
//! `to_le_bytes`/`from_le_bytes`, and comparisons for `leading_zeros` and `div_ceil`, which vstd
//! does not specify. The agreement checks cover each difference.

use vstd::prelude::*;

verus! {

/// Byte `j` of `x`, little-endian.
pub open spec fn byte_at(x: u64, j: int) -> u8 {
    #[verifier::truncate] (((x >> ((8 * j) as u64)) & 0xff) as u8)
}

/// The low `len` little-endian bytes of `x` (upstream `EncodeUintWithLength`).
pub open spec fn le(x: u64, len: int) -> Seq<u8> {
    Seq::new(len as nat, |j: int| byte_at(x, j))
}

/// The value of the first `len` little-endian bytes of `s` (upstream `GetUintWithLength`).
pub open spec fn le_value(s: Seq<u8>, len: int) -> u64
    decreases len,
{
    if len <= 0 { 0u64 } else {
        (le_value(s, len - 1) | ((s[len - 1] as u64) << ((8 * (len - 1)) as u64))) as u64
    }
}

/// `x` fits in `len` bytes.
pub open spec fn fits(x: u64, len: int) -> bool {
    len >= 8 || (x >> ((8 * len) as u64)) == 0
}

pub proof fn lemma_le_value_of_le(s: Seq<u8>, x: u64, len: int, k: int)
    requires 0 <= k <= len <= 8, s.len() >= len, forall|j: int| 0 <= j < len ==> s[j] == byte_at(x, j),
    ensures le_value(s, k) == (if k >= 8 { x } else { x & (((1u64 << ((8 * k) as u64)) - 1) as u64) }),
    decreases k,
{
    if k > 0 {
        lemma_le_value_of_le(s, x, len, k - 1);
        let m: u64 = (k - 1) as u64;
        let b: u8 = s[k - 1];
        let prev = le_value(s, k - 1);
        let cur = le_value(s, k);
        assert((8 * (k - 1)) as u64 == 8 * m);
        assert(b == #[verifier::truncate] (((x >> (8 * m)) & 0xff) as u8));
        assert(cur == (prev | ((b as u64) << (8 * m))) as u64);
        assert(cur == (if m + 1 >= 8 { x } else { x & (((1u64 << (8 * (m + 1))) - 1) as u64) }))
            by (bit_vector)
            requires
                m < 8,
                b == ((x >> (8 * m)) & 0xff) as u8,
                prev == (if m >= 8 { x } else { x & (((1u64 << (8 * m)) - 1) as u64) }),
                cur == (prev | ((b as u64) << (8 * m))) as u64;
        assert((8 * k) as u64 == 8 * (m + 1));
    } else {
        assert(x & (((1u64 << 0u64) - 1) as u64) == 0) by (bit_vector);
    }
}

/// P-FW-04: reading back `len` written bytes returns `x`, for every `x` that fits.
pub proof fn lemma_le_round_trip(s: Seq<u8>, x: u64, len: int)
    requires 1 <= len <= 8, fits(x, len), s.len() >= len, s.subrange(0, len) == le(x, len),
    ensures le_value(s, len) == x,
{
    assert forall|j: int| 0 <= j < len implies s[j] == byte_at(x, j) by {
        assert(s.subrange(0, len)[j] == s[j]);
    }
    lemma_le_value_of_le(s, x, len, len);
    let l = len as u64;
    assert((if l >= 8 { x } else { x & (((1u64 << (8 * l)) - 1) as u64) }) == x) by (bit_vector)
        requires 1 <= l <= 8, l >= 8 || (x >> (8 * l)) == 0;
    assert((8 * len) as u64 == 8 * l);
}

/// Restates `encode_uint_with_length` (P-FW-01). REQUIRES `len <= 8`, the library's
/// documented panic condition.
pub fn encode_uint_with_length(w: &mut Vec<u8>, value: u64, len: usize)
    requires len <= 8,
    ensures final(w)@ == old(w)@ + le(value, len as int),
{
    let ghost w0 = w@;
    let mut j: usize = 0;
    while j < len
        invariant j <= len <= 8, w@ =~= w0 + le(value, j as int),
        decreases len - j,
    {
        let b: u8 = ((value >> ((8 * j) as u64)) & 0xff) as u8;
        w.push(b);
        assert(w@ =~= w0 + le(value, (j + 1) as int));
        j += 1;
    }
}

/// Restates `get_uint_with_length` (P-FW-02). REQUIRES `len <= 8` and `data.len() >= len`.
pub fn get_uint_with_length(data: &[u8], len: usize) -> (r: u64)
    requires len <= 8, data.len() >= len,
    ensures r == le_value(data@, len as int),
{
    let mut acc: u64 = 0;
    let mut j: usize = 0;
    while j < len
        invariant j <= len <= 8, len <= data.len(), acc == le_value(data@, j as int),
        decreases len - j,
    {
        acc = acc | ((data[j] as u64) << ((8 * j) as u64));
        j += 1;
    }
    acc
}

/// Restates `decode_uint_with_length` (P-FW-03): `len` bytes at `pos`, or `None` if fewer
/// are left (the library's `UnexpectedEof`). REQUIRES `len <= 8`.
pub fn decode_uint_with_length(buf: &[u8], pos: usize, len: usize) -> (r: Option<(u64, usize)>)
    requires len <= 8, pos <= buf.len(),
    ensures
        len <= buf.len() - pos ==> r == Some((le_value(buf@.subrange(pos as int, pos + len), len as int), (pos + len) as usize)),
        len > buf.len() - pos ==> r is None,
{
    if len == 0 {
        return Some((0, pos));
    }
    if len > buf.len() - pos {
        return None;
    }
    let ghost s = buf@.subrange(pos as int, pos + len);
    let mut acc: u64 = 0;
    let mut j: usize = 0;
    while j < len
        invariant j <= len <= 8, pos + len <= buf.len(), s == buf@.subrange(pos as int, pos + len),
            acc == le_value(s, j as int),
        decreases len - j,
    {
        assert(s[j as int] == buf[(pos + j) as int]);
        acc = acc | ((buf[pos + j] as u64) << ((8 * j) as u64));
        j += 1;
    }
    Some((acc, pos + len))
}

/// Restates `byte_length_u64` (P-UV-05): the minimal number of bytes that holds `v`, at
/// least 1.
pub fn byte_length_u64(v: u64) -> (len: usize)
    ensures 1 <= len <= 8, fits(v, len as int), len > 1 ==> !fits(v, len - 1),
{
    let len: usize = if v < 0x100 { 1 } else if v < 0x1_0000 { 2 } else if v < 0x100_0000 { 3 }
        else if v < 0x1_0000_0000 { 4 } else if v < 0x100_0000_0000 { 5 }
        else if v < 0x1_0000_0000_0000 { 6 } else if v < 0x100_0000_0000_0000 { 7 } else { 8 };
    let l = len as u64;
    let m: u64 = if l > 1 { l - 1 } else { 0 };
    assert((l >= 8 || (v >> (8 * l)) == 0) && (l > 1 ==> (v >> (8 * m)) != 0)) by (bit_vector)
        requires
            l > 1 ==> m + 1 == l,
            (v < 0x100 ==> l == 1), (0x100 <= v < 0x1_0000 ==> l == 2),
            (0x1_0000 <= v < 0x100_0000 ==> l == 3), (0x100_0000 <= v < 0x1_0000_0000 ==> l == 4),
            (0x1_0000_0000 <= v < 0x100_0000_0000 ==> l == 5),
            (0x100_0000_0000 <= v < 0x1_0000_0000_0000 ==> l == 6),
            (0x1_0000_0000_0000 <= v < 0x100_0000_0000_0000 ==> l == 7),
            (0x100_0000_0000_0000 <= v ==> l == 8);
    assert((8 * len) as u64 == 8 * l);
    assert(len > 1 ==> (8 * (len - 1)) as u64 == 8 * m);
    len
}

/// Restates `byte_length_u32`: the minimal number of bytes that holds `v`, at least 1.
pub fn byte_length_u32(v: u32) -> (len: usize)
    ensures 1 <= len <= 4, fits(v as u64, len as int), len > 1 ==> !fits(v as u64, len - 1),
{
    let len: usize = if v < 0x100 { 1 } else if v < 0x1_0000 { 2 } else if v < 0x100_0000 { 3 } else { 4 };
    let l = len as u64;
    let x = v as u64;
    let m: u64 = if l > 1 { l - 1 } else { 0 };
    assert((l >= 8 || (x >> (8 * l)) == 0) && (l > 1 ==> (x >> (8 * m)) != 0)) by (bit_vector)
        requires
            l > 1 ==> m + 1 == l,
            x < 0x1_0000_0000,
            (x < 0x100 ==> l == 1), (0x100 <= x < 0x1_0000 ==> l == 2),
            (0x1_0000 <= x < 0x100_0000 ==> l == 3), (0x100_0000 <= x ==> l == 4);
    assert((8 * len) as u64 == 8 * l);
    assert(len > 1 ==> (8 * (len - 1)) as u64 == 8 * m);
    len
}

} // verus!
