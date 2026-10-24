// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Planning probe for spec 003 (research R0): the library's varint and 64-bit integer-vector
//! codecs, restated for Verus with their round trips proved for every length. Evidence for
//! the plan, and the starting draft for `verify/deductive/src/{varint,uint_vector}.rs`.
//!
//! ```text
//! verus --rlimit 60 --triggers-mode silent specs/003-formal-verify-decoders/probe/uint_vector.rs
//! ```

use vstd::prelude::*;

verus! {

// ---- varint (`encoding::write_uvarint`, `encoding::read_uvarint`) ---------------------------

/// The upstream varint encoding of `x`: 7 bits per byte, low bits first, the high bit set on
/// every byte but the last.
pub open spec fn varint(x: u64) -> Seq<u8>
    decreases x,
{
    if x < 128 {
        seq![x as u8]
    } else {
        seq![(x % 128 + 128) as u8] + varint(x / 128)
    }
}

/// `write_uvarint`, restated over a `Vec<u8>` sink.
pub fn write_uvarint(w: &mut Vec<u8>, x0: u64)
    ensures final(w)@ == old(w)@ + varint(x0),
{
    let ghost w0 = w@;
    let mut x = x0;
    while x >= 0x80
        invariant w@ + varint(x) == w0 + varint(x0),
        decreases x,
    {
        let b: u8 = (x as u8) | 0x80;
        assert(b == (x % 128 + 128) as u8) by (bit_vector)
            requires x >= 0x80, b == (x as u8) | 0x80u8;
        let ghost wb = w@;
        w.push(b);
        let nx = x >> 7;
        assert(nx == x / 128) by (bit_vector) requires nx == x >> 7;
        assert(varint(x) == seq![b] + varint(nx));
        assert(w@ + varint(nx) =~= wb + varint(x));
        x = nx;
    }
    let ghost wb = w@;
    w.push(x as u8);
    assert(w@ =~= wb + varint(x));
}

/// The library's decoder as a spec function over a byte sequence: the value contributed from
/// position `i` on, already shifted by `shift`, and the position after the last byte read.
pub open spec fn dec(s: Seq<u8>, i: int, shift: u64) -> Option<(u64, int)>
    decreases s.len() - i,
{
    if i < 0 || i >= s.len() || shift >= 64 {
        None
    } else {
        let b = s[i];
        let part: u64 = ((b & 0x7f) as u64) << shift;
        if b < 0x80 {
            Some((part, i + 1))
        } else if shift + 7 >= 64 {
            None
        } else {
            match dec(s, i + 1, (shift + 7) as u64) {
                Some((v, j)) => Some(((part | v) as u64, j)),
                None => None,
            }
        }
    }
}

/// `read_uvarint`, restated over a slice and a cursor. Returns the value and the new cursor.
pub fn read_uvarint(buf: &[u8], pos: usize) -> (r: Option<(u64, usize)>)
    ensures
        r matches Some((v, p)) ==> dec(buf@, pos as int, 0) == Some((v, p as int)) && pos < p <= buf.len(),
        r is None ==> dec(buf@, pos as int, 0) is None,
{
    let mut result: u64 = 0;
    let mut shift: u64 = 0;
    let mut i: usize = pos;
    proof {
        assert forall|v: u64| ((0u64 | v) as u64) == v by {
            assert(((0u64 | v) as u64) == v) by (bit_vector);
        }
    }
    loop
        invariant
            pos <= i,
            shift < 64,
            shift == 7 * (i - pos),
            dec(buf@, pos as int, 0) == (match dec(buf@, i as int, shift) {
                Some((v, j)) => Some(((result | v) as u64, j)),
                None => None::<(u64, int)>,
            }),
        decreases buf.len() - i,
    {
        if i >= buf.len() {
            return None;
        }
        let b = buf[i];
        let part: u64 = ((b & 0x7f) as u64) << shift;
        if b < 0x80 {
            assert(((result | part) as u64) == (result | part));
            return Some((result | part, i + 1));
        }
        if shift + 7 >= 64 {
            return None;
        }
        proof {
            let r0 = result;
            assert forall|v: u64| ((r0 | ((part | v) as u64)) as u64) == (((r0 | part) as u64 | v) as u64) by {
                assert(((r0 | ((part | v) as u64)) as u64) == (((r0 | part) as u64 | v) as u64)) by (bit_vector);
            }
        }
        result = result | part;
        shift = shift + 7;
        i = i + 1;
    }
}

/// Decoding the encoding of `y`, placed at position `i` with the given shift, reads exactly
/// its bytes and yields `y << shift`, provided `y << shift` loses no bits.
pub proof fn lemma_dec_varint(s: Seq<u8>, i: int, y: u64, shift: u64)
    requires
        0 <= i,
        i + varint(y).len() <= s.len(),
        s.subrange(i, i + varint(y).len()) == varint(y),
        shift < 64,
        shift == 0 || (y >> ((64 - shift) as u64)) == 0,
    ensures
        dec(s, i, shift) == Some(((y << shift) as u64, i + varint(y).len())),
    decreases y,
{
    let b = s[i];
    assert(b == varint(y)[0]);
    if y < 128 {
        assert(b == y as u8);
        assert((((b & 0x7f) as u64) << shift) == ((y << shift) as u64)) by (bit_vector)
            requires y < 128, b == y as u8;
    } else {
        let ny = y / 128;
        assert(b == (y % 128 + 128) as u8);
        assert(b >= 0x80) by (bit_vector) requires b == (y % 128 + 128) as u8, y >= 128;
        assert(shift + 7 < 64) by (bit_vector)
            requires y >= 128, shift < 64, shift == 0 || (y >> ((64 - shift) as u64)) == 0;
        assert(varint(y) == seq![b] + varint(ny));
        assert(s.subrange(i + 1, i + 1 + varint(ny).len()) =~= varint(ny)) by {
            assert(s.subrange(i, i + varint(y).len()).subrange(1, varint(y).len() as int) =~= varint(ny));
        }
        assert((ny >> ((64 - (shift + 7)) as u64)) == 0) by (bit_vector)
            requires ny == y / 128, shift + 7 < 64, shift == 0 || (y >> ((64 - shift) as u64)) == 0;
        lemma_dec_varint(s, i + 1, ny, (shift + 7) as u64);
        assert((((((b & 0x7f) as u64) << shift) as u64 | ((ny << ((shift + 7) as u64)) as u64)) as u64)
            == ((y << shift) as u64)) by (bit_vector)
            requires b == (y % 128 + 128) as u8, ny == y / 128, shift + 7 < 64;
    }
}

/// Varint round trip with exact consumption: whatever precedes and follows the encoding.
pub proof fn lemma_round_trip(prefix: Seq<u8>, x: u64, rest: Seq<u8>)
    ensures dec(prefix + varint(x) + rest, prefix.len() as int, 0)
        == Some((x, (prefix.len() + varint(x).len()) as int)),
{
    let s = prefix + varint(x) + rest;
    assert(s.subrange(prefix.len() as int, (prefix.len() + varint(x).len()) as int) =~= varint(x));
    lemma_dec_varint(s, prefix.len() as int, x, 0);
    assert((x << 0u64) == x) by (bit_vector);
}

// ---- fixed-width little-endian elements -------------------------------------------------

pub open spec fn byte_at(x: u64, j: int) -> u8 {
    #[verifier::truncate] (((x >> ((8 * j) as u64)) & 0xff) as u8)
}

/// The low `len` little-endian bytes of `x`.
pub open spec fn le(x: u64, len: int) -> Seq<u8> {
    Seq::new(len as nat, |j: int| byte_at(x, j))
}

/// The value of the first `len` little-endian bytes of `s`.
pub open spec fn le_value(s: Seq<u8>, len: int) -> u64
    decreases len,
{
    if len <= 0 { 0u64 } else {
        (le_value(s, len - 1) | ((s[len - 1] as u64) << ((8 * (len - 1)) as u64))) as u64
    }
}

pub open spec fn fits(x: u64, len: int) -> bool {
    len >= 8 || (x >> ((8 * len) as u64)) == 0
}

proof fn lemma_le_value_of_le(s: Seq<u8>, x: u64, len: int, k: int)
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

proof fn lemma_le_round_trip(s: Seq<u8>, x: u64, len: int)
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

/// `encode_uint_with_length`, restated over a `Vec<u8>` sink.
fn put_le(w: &mut Vec<u8>, x: u64, len: usize)
    requires len <= 8,
    ensures final(w)@ == old(w)@ + le(x, len as int),
{
    let ghost w0 = w@;
    let mut j: usize = 0;
    while j < len
        invariant j <= len <= 8, w@ =~= w0 + le(x, j as int),
        decreases len - j,
    {
        let b: u8 = ((x >> ((8 * j) as u64)) & 0xff) as u8;
        w.push(b);
        assert(w@ =~= w0 + le(x, (j + 1) as int));
        j += 1;
    }
}

/// `decode_uint_with_length` over a slice: `len` bytes at `p`.
fn get_le(buf: &[u8], p: usize, len: usize) -> (r: u64)
    requires len <= 8, p + len <= buf.len(),
    ensures r == le_value(buf@.subrange(p as int, p + len), len as int),
{
    let ghost s = buf@.subrange(p as int, p + len);
    let mut acc: u64 = 0;
    let mut j: usize = 0;
    while j < len
        invariant j <= len <= 8, p + len <= buf.len(), s == buf@.subrange(p as int, p + len),
            acc == le_value(s, j as int),
        decreases len - j,
    {
        assert(s[j as int] == buf[(p + j) as int]);
        acc = acc | ((buf[p + j] as u64) << ((8 * j) as u64));
        j += 1;
    }
    acc
}

// ---- the integer vector -------------------------------------------------------------------

/// `1 | v[0] | … | v[n-1]`, the library's fold.
pub open spec fn or_fold(v: Seq<u64>) -> u64
    decreases v.len(),
{
    if v.len() == 0 { 1u64 } else { (or_fold(v.drop_last()) | v.last()) as u64 }
}

proof fn lemma_or_fold_covers(v: Seq<u64>, k: int)
    requires 0 <= k < v.len(),
    ensures (v[k] | or_fold(v)) == or_fold(v),
    decreases v.len(),
{
    let o = or_fold(v);
    let p = or_fold(v.drop_last());
    let x = v.last();
    assert(o == (p | x) as u64);
    if k == v.len() - 1 {
        assert((x | ((p | x) as u64)) == ((p | x) as u64)) by (bit_vector);
    } else {
        lemma_or_fold_covers(v.drop_last(), k);
        let y = v[k];
        assert(v.drop_last()[k] == y);
        assert((y | ((p | x) as u64)) == ((p | x) as u64)) by (bit_vector) requires (y | p) == p;
    }
}

/// `byte_length_u64`, restated with comparisons (the library uses `leading_zeros` and
/// `div_ceil`; the agreement checks cover the difference).
fn byte_length(v: u64) -> (len: usize)
    ensures 1 <= len <= 8, fits(v, len as int),
{
    let len: usize = if v < 0x100 { 1 } else if v < 0x1_0000 { 2 } else if v < 0x100_0000 { 3 }
        else if v < 0x1_0000_0000 { 4 } else if v < 0x100_0000_0000 { 5 }
        else if v < 0x1_0000_0000_0000 { 6 } else if v < 0x100_0000_0000_0000 { 7 } else { 8 };
    let l = len as u64;
    assert(l >= 8 || (v >> (8 * l)) == 0) by (bit_vector)
        requires
            (v < 0x100 ==> l == 1), (0x100 <= v < 0x1_0000 ==> l == 2),
            (0x1_0000 <= v < 0x100_0000 ==> l == 3), (0x100_0000 <= v < 0x1_0000_0000 ==> l == 4),
            (0x1_0000_0000 <= v < 0x100_0000_0000 ==> l == 5),
            (0x100_0000_0000 <= v < 0x1_0000_0000_0000 ==> l == 6),
            (0x1_0000_0000_0000 <= v < 0x100_0000_0000_0000 ==> l == 7),
            (0x100_0000_0000_0000 <= v ==> l == 8);
    assert((8 * len) as u64 == 8 * l);
    len
}

/// The elements, `len` bytes each, front to back.
pub open spec fn elems(v: Seq<u64>, len: int) -> Seq<u8>
    decreases v.len(),
{
    if v.len() == 0 { Seq::empty() } else { le(v[0], len) + elems(v.skip(1), len) }
}

proof fn lemma_elems_push(v: Seq<u64>, x: u64, len: int)
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

/// The library's maximum element count.
pub const MAX_DECODE_COUNT: u64 = 50_000_000;

pub open spec fn header(n: int, len: int) -> u64 {
    ((n * 8) as u64 | (len - 1) as u64) as u64
}

/// The `EncodedUintVector<uint64>` encoding of `v` with element width `len`.
pub open spec fn uvec(v: Seq<u64>, len: int) -> Seq<u8> {
    varint(header(v.len() as int, len)) + elems(v, len)
}

/// `encode_uint_vector_u64`, restated. The slice-length bound is Rust's object-size limit
/// (`v.len() * 8 <= isize::MAX`).
pub fn encode_uint_vector_u64(v: &[u64], w: &mut Vec<u8>) -> (len: usize)
    requires v.len() <= 0x1000_0000_0000_0000,
    ensures
        1 <= len <= 8,
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len as int),
        final(w)@ == old(w)@ + uvec(v@, len as int),
{
    let ghost w0 = w@;
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
    let len = byte_length(one_bits);
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
        put_le(w, v[i], len);
        proof {
            lemma_elems_push(v@.take(i as int), v[i as int], len as int);
            assert(v@.take((i + 1) as int) =~= v@.take(i as int).push(v[i as int]));
        }
        i += 1;
    }
    assert(v@.take(v.len() as int) =~= v@);
    len
}

/// The decoder's element loop as a spec function: `count` elements of `len` bytes from `p`.
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

/// The whole decoder as a spec function.
pub open spec fn dec_uvec(s: Seq<u8>, pos: int) -> Option<(Seq<u64>, int)> {
    match dec(s, pos, 0) {
        Some((h, p)) => if h / 8 > MAX_DECODE_COUNT { None } else {
            dec_elems(s, p, (h / 8) as nat, ((h & 7) + 1) as int)
        },
        None => None,
    }
}

proof fn lemma_dec_elems_split(s: Seq<u8>, p: int, count: nat, i: nat, len: int)
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
proof fn lemma_dec_elems_round_trip(s: Seq<u8>, p: int, v: Seq<u64>, len: int)
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

/// `decode_uint_vector_u64`, restated over a slice and a cursor.
pub fn decode_uint_vector_u64(buf: &[u8], pos: usize) -> (r: Option<(Vec<u64>, usize)>)
    ensures
        r matches Some((_, p)) ==> pos < p <= buf.len(),
        r matches Some((v, p)) ==> dec_uvec(buf@, pos as int) == Some((v@, p as int)),
        r is None ==> dec_uvec(buf@, pos as int) is None,
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
        if len > buf.len() - p {
            return None;
        }
        let x = get_le(buf, p, len);
        let ghost r0 = result@;
        result.push(x);
        proof {
            match dec_elems(buf@, (p + len) as int, (count - i - 1) as nat, len as int) {
                Some((u, r)) => { assert(r0 + (seq![x] + u) =~= result@ + u); }
                None => {}
            }
        }
        p = p + len;
        i = i + 1;
    }
    proof {
        assert(result@ + Seq::<u64>::empty() =~= result@);
    }
    Some((result, p))
}

/// The header round trip: the varint is read back, and it yields the count and the width.
pub proof fn lemma_uvec_round_trip(prefix: Seq<u8>, v: Seq<u64>, len: int, rest: Seq<u8>)
    requires
        1 <= len <= 8,
        v.len() <= MAX_DECODE_COUNT,
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len),
    ensures ({
        let s = prefix + uvec(v, len) + rest;
        let h = header(v.len() as int, len);
        &&& dec(s, prefix.len() as int, 0) == Some((h, (prefix.len() + varint(h).len()) as int))
        &&& h / 8 == v.len()
        &&& ((h & 7) + 1) as int == len
    }),
{
    let h = header(v.len() as int, len);
    let s = prefix + uvec(v, len) + rest;
    assert(s =~= prefix + varint(h) + (elems(v, len) + rest));
    lemma_round_trip(prefix, h, elems(v, len) + rest);
    let n = v.len() as u64;
    let l = (len - 1) as u64;
    assert(h == ((n * 8) as u64 | l) as u64);
    assert(((((n * 8) as u64 | l) as u64) / 8) == n && (((((n * 8) as u64 | l) as u64) & 7) + 1) == l + 1)
        by (bit_vector) requires n <= 50_000_000, l < 8;
}

/// Round trip with exact consumption for every vector up to the count limit, whatever
/// precedes and follows the encoding.
pub proof fn lemma_uvec_full_round_trip(prefix: Seq<u8>, v: Seq<u64>, len: int, rest: Seq<u8>)
    requires
        1 <= len <= 8,
        v.len() <= MAX_DECODE_COUNT,
        forall|k: int| 0 <= k < v.len() ==> fits(v[k], len),
    ensures dec_uvec(prefix + uvec(v, len) + rest, prefix.len() as int)
        == Some((v, (prefix.len() + uvec(v, len).len()) as int)),
{
    let s = prefix + uvec(v, len) + rest;
    let h = header(v.len() as int, len);
    lemma_uvec_round_trip(prefix, v, len, rest);
    let p = (prefix.len() + varint(h).len()) as int;
    assert(s.subrange(p, p + elems(v, len).len()) =~= elems(v, len));
    lemma_dec_elems_round_trip(s, p, v, len);
}

fn main() {}

} // verus!
