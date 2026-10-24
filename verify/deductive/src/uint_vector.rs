// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The library's `EncodedUintVector` codecs in `encoded_uint_vector.rs`,
//! `encode_uint_vector_u64`, `decode_uint_vector_u64`, `encode_uint_vector_u32` and
//! `decode_uint_vector_u32`, restated for Verus (spec 003 FR-010, P-UV-01 … P-UV-06).
//!
//! Proved for every input and every length:
//! - each encoder appends exactly [`uvec`] of its values, with the minimal width
//!   ([`fits`] for every element, P-UV-05);
//! - each decoder returns exactly the spec decoder [`dec_uvec`]'s answer, for every byte
//!   sequence and position. That is sound acceptance, crash-freedom and termination;
//! - decoding `prefix + uvec(v) + rest` returns `v` and stops right after the encoding, for
//!   every vector up to the library's count limit ([`lemma_uvec_full_round_trip`]).
//!
//! The spec is one family, parameterised by the element size `sz` (4 or 8). The 32-bit
//! values are viewed as `u64`s.
//!
//! Permitted differences (`mirrors.toml`): a slice and a cursor for the reader, a `Vec<u8>` for
//! the writer, `None` for an `io::Error`, and indexed loops for `iter().fold`.

use vstd::prelude::*;

use crate::fixed_width::*;
use crate::varint::*;

verus! {

/// The library's maximum element count (`MAX_DECODE_COUNT`), a documented deviation
/// (FR-022): upstream has no limit.
pub const MAX_DECODE_COUNT: u64 = 50_000_000;

/// `1 | v[0] | … | v[n-1]`, the encoders' fold.
pub open spec fn or_fold(v: Seq<u64>) -> u64
    decreases v.len(),
{
    if v.len() == 0 { 1u64 } else { (or_fold(v.drop_last()) | v.last()) as u64 }
}

pub proof fn lemma_or_fold_covers(v: Seq<u64>, k: int)
    requires 0 <= k < v.len(),
    ensures (v[k] | or_fold(v)) == or_fold(v),
    decreases v.len(),
{
    let p = or_fold(v.drop_last());
    let x = v.last();
    if k == v.len() - 1 {
        assert((x | ((p | x) as u64)) == ((p | x) as u64)) by (bit_vector);
    } else {
        lemma_or_fold_covers(v.drop_last(), k);
        let y = v[k];
        assert(v.drop_last()[k] == y);
        assert((y | ((p | x) as u64)) == ((p | x) as u64)) by (bit_vector) requires (y | p) == p;
    }
}

/// The elements, `len` bytes each, front to back.
pub open spec fn elems(v: Seq<u64>, len: int) -> Seq<u8>
    decreases v.len(),
{
    if v.len() == 0 { Seq::empty() } else { le(v[0], len) + elems(v.skip(1), len) }
}

pub proof fn lemma_elems_push(v: Seq<u64>, x: u64, len: int)
    ensures elems(v.push(x), len) =~= elems(v, len) + le(x, len),
    decreases v.len(),
{
    if v.len() == 0 {
        assert(v.push(x).skip(1) =~= Seq::<u64>::empty());
        assert(v =~= Seq::<u64>::empty());
        assert(elems(v.push(x), len) == le(x, len) + elems(Seq::<u64>::empty(), len));
        assert(elems(v.push(x), len) =~= elems(v, len) + le(x, len));
    } else {
        assert(v.push(x).skip(1) =~= v.skip(1).push(x));
        assert(v.push(x)[0] == v[0]);
        lemma_elems_push(v.skip(1), x, len);
        assert(elems(v.push(x), len) == le(v[0], len) + elems(v.skip(1).push(x), len));
        assert(elems(v, len) == le(v[0], len) + elems(v.skip(1), len));
        assert(elems(v.push(x), len) =~= elems(v, len) + le(x, len));
    }
}

/// The header varint: `(n · sz) | (len − 1)` (upstream `EncodeUintVector`).
pub open spec fn header(n: int, len: int, sz: int) -> u64 {
    ((n * sz) as u64 | (len - 1) as u64) as u64
}

/// The `EncodedUintVector` encoding of `v` with element size `sz` and width `len`.
pub open spec fn uvec(v: Seq<u64>, len: int, sz: int) -> Seq<u8> {
    varint(header(v.len() as int, len, sz)) + elems(v, len)
}

/// The decoders' element loop: `count` elements of `len` bytes from `p`.
pub open spec fn dec_elems(s: Seq<u8>, p: int, count: nat, len: int) -> Option<(Seq<u64>, int)>
    decreases count,
{
    if count == 0 {
        Some((Seq::empty(), p))
    } else if p < 0 || len < 1 || p + len > s.len() {
        None
    } else {
        match dec_elems(s, p + len, (count - 1) as nat, len) {
            Some((t, q)) => Some((seq![le_value(s.subrange(p, p + len), len)] + t, q)),
            None => None,
        }
    }
}

/// A whole decoder, for element size `sz` (4 or 8).
pub open spec fn dec_uvec(s: Seq<u8>, pos: int, sz: u64) -> Option<(Seq<u64>, int)> {
    match dec(s, pos, 0) {
        Some((h, p)) => if h / sz > MAX_DECODE_COUNT { None } else {
            dec_elems(s, p, (h / sz) as nat, ((h & ((sz - 1) as u64)) + 1) as int)
        },
        None => None,
    }
}

pub proof fn lemma_dec_elems_split(s: Seq<u8>, p: int, count: nat, i: nat, len: int)
    requires i <= count,
    ensures dec_elems(s, p, count, len) == (match dec_elems(s, p, i, len) {
        Some((t, q)) => match dec_elems(s, q, (count - i) as nat, len) {
            Some((u, r)) => Some((t + u, r)),
            None => None::<(Seq<u64>, int)>,
        },
        None => None::<(Seq<u64>, int)>,
    }),
    decreases i,
{
    if i == 0 {
        match dec_elems(s, p, count, len) {
            Some((u, r)) => { assert(Seq::<u64>::empty() + u =~= u); }
            None => {}
        }
    } else if p < 0 || len < 1 || p + len > s.len() {
    } else {
        lemma_dec_elems_split(s, p + len, (count - 1) as nat, (i - 1) as nat, len);
        let x = le_value(s.subrange(p, p + len), len);
        match dec_elems(s, p + len, (i - 1) as nat, len) {
            Some((t, q)) => {
                match dec_elems(s, q, (count - i) as nat, len) {
                    Some((u, r)) => { assert(seq![x] + (t + u) =~= (seq![x] + t) + u); }
                    None => {}
                }
            }
            None => {}
        }
    }
}

/// Decoding `elems(v)` placed at `p` yields `v` and stops right after it.
pub proof fn lemma_dec_elems_round_trip(s: Seq<u8>, p: int, v: Seq<u64>, len: int)
    requires
        1 <= len <= 8, 0 <= p,
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len),
        p + elems(v, len).len() <= s.len(),
        s.subrange(p, p + elems(v, len).len()) == elems(v, len),
    ensures dec_elems(s, p, v.len(), len) == Some((v, p + elems(v, len).len())),
    decreases v.len(),
{
    if v.len() > 0 {
        let e = elems(v, len);
        let tl = v.skip(1);
        assert(e == le(v[0], len) + elems(tl, len));
        assert(le(v[0], len).len() == len);
        let sub = s.subrange(p, p + len);
        assert(sub =~= le(v[0], len)) by {
            assert(s.subrange(p, p + e.len()).subrange(0, len) =~= sub);
            assert(e.subrange(0, len) =~= le(v[0], len));
        }
        assert(sub.subrange(0, len) =~= sub);
        lemma_le_round_trip(sub, v[0], len);
        assert forall|k: int| 0 <= k < tl.len() implies fits(tl[k], len) by { assert(tl[k] == v[k + 1]); }
        assert(s.subrange(p + len, p + len + elems(tl, len).len()) =~= elems(tl, len)) by {
            assert(s.subrange(p, p + e.len()).subrange(len, e.len() as int) =~= elems(tl, len));
        }
        lemma_dec_elems_round_trip(s, p + len, tl, len);
        assert(seq![v[0]] + tl =~= v);
    } else {
        assert(elems(v, len) =~= Seq::<u8>::empty());
        assert(v =~= Seq::<u64>::empty());
    }
}

/// The header of `n ≤ MAX_DECODE_COUNT` elements of width `len ≤ sz` decodes back to `n`
/// and `len`.
pub proof fn lemma_header(n: u64, len: u64, sz: u64)
    requires n <= MAX_DECODE_COUNT, 1 <= len <= sz, sz == 4 || sz == 8,
    ensures
        header(n as int, len as int, sz as int) / sz == n,
        ((header(n as int, len as int, sz as int) & ((sz - 1) as u64)) + 1) == len,
{
    let l = (len - 1) as u64;
    if sz == 8 {
        assert(((((n * 8) as u64 | l) as u64) / 8) == n && (((((n * 8) as u64 | l) as u64) & 7) + 1) == l + 1)
            by (bit_vector) requires n <= 50_000_000, l < 8;
    } else {
        assert(((((n * 4) as u64 | l) as u64) / 4) == n && (((((n * 4) as u64 | l) as u64) & 3) + 1) == l + 1)
            by (bit_vector) requires n <= 50_000_000, l < 4;
    }
}

/// Round trip with exact consumption (P-UV-01, P-UV-02): for every vector of up to the
/// count limit whose elements fit `len ≤ sz` bytes, whatever precedes and follows.
pub proof fn lemma_uvec_full_round_trip(prefix: Seq<u8>, v: Seq<u64>, len: int, sz: u64, rest: Seq<u8>)
    requires
        1 <= len <= sz, sz == 4 || sz == 8,
        v.len() <= MAX_DECODE_COUNT,
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len),
    ensures dec_uvec(prefix + uvec(v, len, sz as int) + rest, prefix.len() as int, sz)
        == Some((v, (prefix.len() + uvec(v, len, sz as int).len()) as int)),
{
    let h = header(v.len() as int, len, sz as int);
    let s = prefix + uvec(v, len, sz as int) + rest;
    assert(s =~= prefix + varint(h) + (elems(v, len) + rest));
    lemma_round_trip(prefix, h, elems(v, len) + rest);
    lemma_header(v.len() as u64, len as u64, sz);
    let p = (prefix.len() + varint(h).len()) as int;
    assert(s.subrange(p, p + elems(v, len).len()) =~= elems(v, len));
    lemma_dec_elems_round_trip(s, p, v, len);
}

/// Restates `encode_uint_vector_u64` (P-UV-05, P-UV-06). The slice-length bound is Rust's
/// object-size limit (`v.len() * 8 <= isize::MAX`).
pub fn encode_uint_vector_u64(v: &[u64], w: &mut Vec<u8>) -> (len: usize)
    requires v.len() <= 0x1000_0000_0000_0000,
    ensures
        1 <= len <= 8,
        len == 1 || !fits(or_fold(v@), len - 1),
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len as int),
        final(w)@ == old(w)@ + uvec(v@, len as int, 8),
{
    let mut one_bits: u64 = 1;
    let mut i: usize = 0;
    while i < v.len()
        invariant i <= v.len(), one_bits == or_fold(v@.take(i as int)),
        decreases v.len() - i,
    {
        assert(v@.take((i + 1) as int).drop_last() =~= v@.take(i as int));
        one_bits = one_bits | v[i];
        i += 1;
    }
    assert(v@.take(v.len() as int) =~= v@);
    let len = byte_length_u64(one_bits);
    proof {
        assert forall|k: int| 0 <= k < v.len() implies fits(v[k], len as int) by {
            lemma_or_fold_covers(v@, k);
            let x = v[k];
            let o = one_bits;
            let l = len as u64;
            assert((8 * len) as u64 == 8 * l);
            assert(l >= 8 || (x >> (8 * l)) == 0) by (bit_vector)
                requires (x | o) == o, 1 <= l <= 8, l >= 8 || (o >> (8 * l)) == 0;
        }
    }
    let size_len: u64 = ((v.len() as u64) * 8) | ((len as u64) - 1);
    write_uvarint(w, size_len);
    let ghost wh = w@;
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(), len <= 8,
            w@ =~= wh + elems(v@.take(i as int), len as int),
        decreases v.len() - i,
    {
        encode_uint_with_length(w, v[i], len);
        proof {
            lemma_elems_push(v@.take(i as int), v[i as int], len as int);
            assert(v@.take((i + 1) as int) =~= v@.take(i as int).push(v[i as int]));
        }
        i += 1;
    }
    assert(v@.take(v.len() as int) =~= v@);
    len
}

/// Restates `decode_uint_vector_u64` (P-UV-03): exactly the spec decoder's answer, for every
/// input.
pub fn decode_uint_vector_u64(buf: &[u8], pos: usize) -> (r: Option<(Vec<u64>, usize)>)
    ensures
        r matches Some((_, p)) ==> pos < p <= buf.len(),
        r matches Some((v, p)) ==> dec_uvec(buf@, pos as int, 8) == Some((v@, p as int)),
        r is None ==> dec_uvec(buf@, pos as int, 8) is None,
{
    let (size_len, mut p) = match read_uvarint(buf, pos) { Some(x) => x, None => return None };
    let count: u64 = size_len / 8;
    if count > MAX_DECODE_COUNT {
        return None;
    }
    assert((size_len & 7) <= 7) by (bit_vector);
    let len: usize = ((size_len & 7) + 1) as usize;
    let ghost p0 = p as int;
    let mut result: Vec<u64> = Vec::new();
    let mut i: u64 = 0;
    proof { lemma_dec_elems_split(buf@, p0, count as nat, 0, len as int); }
    while i < count
        invariant
            pos < p <= buf.len(), 1 <= len <= 8, i <= count, result.len() == i,
            len == ((size_len & 7) + 1), count == size_len / 8,
            dec(buf@, pos as int, 0) == Some((size_len, p0)),
            dec_elems(buf@, p0, count as nat, len as int) == (match dec_elems(buf@, p as int, (count - i) as nat, len as int) {
                Some((u, r)) => Some((result@ + u, r)),
                None => None::<(Seq<u64>, int)>,
            }),
        decreases count - i,
    {
        let (x, np) = match decode_uint_with_length(buf, p, len) { Some(x) => x, None => return None };
        let ghost r0 = result@;
        result.push(x);
        proof {
            match dec_elems(buf@, (p + len) as int, (count - i - 1) as nat, len as int) {
                Some((u, r)) => { assert(r0 + (seq![x] + u) =~= result@ + u); }
                None => {}
            }
        }
        p = np;
        i = i + 1;
    }
    proof { assert(result@ + Seq::<u64>::empty() =~= result@); }
    Some((result, p))
}

/// The value of at most 4 little-endian bytes is below `2^32`.
pub proof fn lemma_le_value_small(s: Seq<u8>, k: int)
    requires 0 <= k <= 4, s.len() >= k,
    ensures le_value(s, k) < 0x1_0000_0000,
    decreases k,
{
    if k > 0 {
        lemma_le_value_small(s, k - 1);
        let m: u64 = (k - 1) as u64;
        let prev = le_value(s, k - 1);
        let b: u8 = s[k - 1];
        let cur = le_value(s, k);
        assert((8 * (k - 1)) as u64 == 8 * m);
        assert(cur == (prev | ((b as u64) << (8 * m))) as u64);
        // The bound grows by one byte per step: prev < 2^(8m) ≤ 2^24 and b << 8m < 2^(8m+8).
        assert(prev < (1u64 << (8 * m)) ==> cur < (1u64 << (8 * (m + 1)))) by (bit_vector)
            requires m < 4, cur == (prev | ((b as u64) << (8 * m))) as u64;
        lemma_le_value_bits(s, k - 1);
        assert(cur < 0x1_0000_0000) by (bit_vector)
            requires m < 4, prev < (1u64 << (8 * m)), cur == (prev | ((b as u64) << (8 * m))) as u64;
    }
}

/// The value of `k ≤ 7` little-endian bytes is below `2^(8k)`.
pub proof fn lemma_le_value_bits(s: Seq<u8>, k: int)
    requires 0 <= k <= 7, s.len() >= k,
    ensures le_value(s, k) < (1u64 << ((8 * k) as u64)),
    decreases k,
{
    if k > 0 {
        lemma_le_value_bits(s, k - 1);
        let m: u64 = (k - 1) as u64;
        let prev = le_value(s, k - 1);
        let b: u8 = s[k - 1];
        let cur = le_value(s, k);
        assert((8 * (k - 1)) as u64 == 8 * m);
        assert((8 * k) as u64 == 8 * (m + 1));
        assert(cur == (prev | ((b as u64) << (8 * m))) as u64);
        assert(cur < (1u64 << (8 * (m + 1)))) by (bit_vector)
            requires m < 7, prev < (1u64 << (8 * m)), cur == (prev | ((b as u64) << (8 * m))) as u64;
    } else {
        assert(0u64 < (1u64 << 0u64)) by (bit_vector);
    }
}

/// The 32-bit values viewed as `u64`s.
pub open spec fn widen(v: Seq<u32>) -> Seq<u64> {
    v.map_values(|x: u32| x as u64)
}

pub proof fn lemma_or_fold_u32(v: Seq<u32>, acc: u32)
    requires acc as u64 == or_fold(widen(v.drop_last())), v.len() > 0,
    ensures ((acc | v.last()) as u32) as u64 == or_fold(widen(v)),
{
    assert(widen(v).drop_last() =~= widen(v.drop_last()));
    assert(widen(v).last() == v.last() as u64);
    let a = acc;
    let x = v.last();
    assert(((a | x) as u32) as u64 == ((a as u64) | (x as u64)) as u64) by (bit_vector);
}

/// Restates `encode_uint_vector_u32`.
pub fn encode_uint_vector_u32(v: &[u32], w: &mut Vec<u8>) -> (len: usize)
    requires v.len() <= 0x1000_0000_0000_0000,
    ensures
        1 <= len <= 4,
        len == 1 || !fits(or_fold(widen(v@)), len - 1),
        forall|k: int| 0 <= k < v.len() ==> fits(v[k] as u64, len as int),
        final(w)@ == old(w)@ + uvec(widen(v@), len as int, 4),
{
    let mut one_bits: u32 = 1;
    let mut i: usize = 0;
    assert(widen(v@.take(0)) =~= Seq::<u64>::empty());
    while i < v.len()
        invariant i <= v.len(), one_bits as u64 == or_fold(widen(v@.take(i as int))),
        decreases v.len() - i,
    {
        proof {
            assert(v@.take((i + 1) as int).drop_last() =~= v@.take(i as int));
            assert(v@.take((i + 1) as int).last() == v[i as int]);
            lemma_or_fold_u32(v@.take((i + 1) as int), one_bits);
        }
        one_bits = one_bits | v[i];
        i += 1;
    }
    assert(v@.take(v.len() as int) =~= v@);
    let len = byte_length_u32(one_bits);
    proof {
        assert forall|k: int| 0 <= k < v.len() implies fits(v[k] as u64, len as int) by {
            lemma_or_fold_covers(widen(v@), k);
            assert(widen(v@)[k] == v[k] as u64);
            let x = v[k] as u64;
            let o = one_bits as u64;
            let l = len as u64;
            assert((8 * len) as u64 == 8 * l);
            assert(l >= 8 || (x >> (8 * l)) == 0) by (bit_vector)
                requires (x | o) == o, 1 <= l <= 8, l >= 8 || (o >> (8 * l)) == 0;
        }
    }
    let size_len: u64 = ((v.len() as u64) * 4) | ((len as u64) - 1);
    write_uvarint(w, size_len);
    let ghost wh = w@;
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(), len <= 4,
            w@ =~= wh + elems(widen(v@.take(i as int)), len as int),
        decreases v.len() - i,
    {
        encode_uint_with_length(w, v[i] as u64, len);
        proof {
            lemma_elems_push(widen(v@.take(i as int)), v[i as int] as u64, len as int);
            assert(widen(v@.take((i + 1) as int)) =~= widen(v@.take(i as int)).push(v[i as int] as u64));
        }
        i += 1;
    }
    assert(v@.take(v.len() as int) =~= v@);
    len
}

/// Restates `decode_uint_vector_u32` (P-UV-04): exactly the spec decoder's answer for element
/// size 4, viewed as `u64`s.
pub fn decode_uint_vector_u32(buf: &[u8], pos: usize) -> (r: Option<(Vec<u32>, usize)>)
    ensures
        r matches Some((_, p)) ==> pos < p <= buf.len(),
        r matches Some((v, p)) ==> dec_uvec(buf@, pos as int, 4) == Some((widen(v@), p as int)),
        r is None ==> dec_uvec(buf@, pos as int, 4) is None,
{
    let (size_len, mut p) = match read_uvarint(buf, pos) { Some(x) => x, None => return None };
    let count: u64 = size_len / 4;
    if count > MAX_DECODE_COUNT {
        return None;
    }
    assert((size_len & 3) <= 3) by (bit_vector);
    let len: usize = ((size_len & 3) + 1) as usize;
    let ghost p0 = p as int;
    let mut result: Vec<u32> = Vec::new();
    let mut i: u64 = 0;
    proof {
        lemma_dec_elems_split(buf@, p0, count as nat, 0, len as int);
        assert(widen(result@) =~= Seq::<u64>::empty());
    }
    while i < count
        invariant
            pos < p <= buf.len(), 1 <= len <= 4, i <= count, result.len() == i,
            len == ((size_len & 3) + 1), count == size_len / 4,
            dec(buf@, pos as int, 0) == Some((size_len, p0)),
            dec_elems(buf@, p0, count as nat, len as int) == (match dec_elems(buf@, p as int, (count - i) as nat, len as int) {
                Some((u, r)) => Some((widen(result@) + u, r)),
                None => None::<(Seq<u64>, int)>,
            }),
        decreases count - i,
    {
        let (x, np) = match decode_uint_with_length(buf, p, len) { Some(x) => x, None => return None };
        proof { lemma_le_value_small(buf@.subrange(p as int, p + len), len as int); }
        let ghost r0 = widen(result@);
        result.push(x as u32);
        proof {
            assert(widen(result@) =~= r0.push(x));
            match dec_elems(buf@, (p + len) as int, (count - i - 1) as nat, len as int) {
                Some((u, r)) => { assert(r0 + (seq![x] + u) =~= widen(result@) + u); }
                None => {}
            }
        }
        p = np;
        i = i + 1;
    }
    proof { assert(widen(result@) + Seq::<u64>::empty() =~= widen(result@)); }
    Some((result, p))
}

} // verus!
