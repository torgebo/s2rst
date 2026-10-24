// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The library's string vector codec in `encoded_string_vector.rs`, `encode_string_vector` and
//! `decode_string_vector`, restated for Verus (spec 003 FR-010, FR-020, P-SV-01, P-SV-02).
//!
//! Proved for every input and every length:
//! - the encoder appends the end offsets as an integer vector, then the strings;
//! - the decoder returns exactly the spec decoder [`dec_svec`]'s answer for every byte
//!   sequence and position. That is sound acceptance, crash-freedom and termination, and
//!   the FR-020 rule: an entry whose range is out of order or out of the data is empty, and
//!   decoding continues from its end;
//! - decoding the encoding of any strings, up to the library's limits, returns them and stops
//!   right after the data, whatever follows ([`lemma_svec_round_trip`]).
//!
//! Permitted differences (`mirrors.toml`):
//! - owned `Vec<u8>` strings for `&[u8]`;
//! - a slice and a cursor for the reader. The data is a subslice of the input instead of
//!   `Read::take` + `read_to_end`, and a short input is `None` where the library returns
//!   `UnexpectedEof`;
//! - a `Vec<u8>` for the writer, `None` for an `io::Error`;
//! - `u64` offsets and positions for the library's `usize` casts.

use vstd::prelude::*;
use vstd::slice::{slice_subrange, slice_to_vec};

use crate::fixed_width::*;
use crate::uint_vector::*;

verus! {

/// The library's limit on the data bytes (`decode_string_vector`), a documented deviation
/// (FR-022): upstream has none.
pub const MAX_STRING_DATA: u64 = 500_000_000;

/// The strings' total length.
pub open spec fn total(v: Seq<Seq<u8>>) -> int
    decreases v.len(),
{
    if v.len() == 0 { 0 } else { total(v.drop_last()) + v.last().len() }
}

/// The strings, concatenated.
pub open spec fn concat(v: Seq<Seq<u8>>) -> Seq<u8>
    decreases v.len(),
{
    if v.len() == 0 { Seq::empty() } else { concat(v.drop_last()) + v.last() }
}

/// The cumulative end offsets (upstream `StringVectorEncoder::Encode`).
pub open spec fn ends(v: Seq<Seq<u8>>) -> Seq<u64>
    decreases v.len(),
{
    if v.len() == 0 { Seq::empty() } else { ends(v.drop_last()).push(total(v) as u64) }
}

/// Entry `i` of a decoded vector: `data[offsets[i-1]..offsets[i]]` when that range is in
/// order and inside the data, else empty (upstream v0.14's lazy rule, FR-020).
pub open spec fn entry(off: Seq<u64>, i: int, data: Seq<u8>) -> Seq<u8> {
    let prev: int = if i == 0 { 0 } else { off[i - 1] as int };
    if prev <= off[i] && off[i] <= data.len() { data.subrange(prev, off[i] as int) } else { Seq::empty() }
}

/// All the entries.
pub open spec fn entries(off: Seq<u64>, data: Seq<u8>) -> Seq<Seq<u8>> {
    Seq::new(off.len(), |i: int| entry(off, i, data))
}

/// The whole decoder.
pub open spec fn dec_svec(s: Seq<u8>, pos: int) -> Option<(Seq<Seq<u8>>, int)> {
    match dec_uvec(s, pos, 8) {
        Some((off, p)) => if off.len() == 0 {
            Some((Seq::empty(), p))
        } else {
            let t = off.last();
            if t > MAX_STRING_DATA || t > s.len() - p {
                None
            } else {
                Some((entries(off, s.subrange(p, p + t)), p + t))
            }
        },
        None => None,
    }
}

/// The byte views of a vector of strings.
pub open spec fn views(v: Seq<Vec<u8>>) -> Seq<Seq<u8>> {
    v.map_values(|x: Vec<u8>| x@)
}

pub proof fn lemma_total_nonneg(v: Seq<Seq<u8>>)
    ensures total(v) >= 0, concat(v).len() == total(v), ends(v).len() == v.len(),
    decreases v.len(),
{
    if v.len() > 0 {
        lemma_total_nonneg(v.drop_last());
    }
}

/// Every end offset is at most the total, and they do not decrease.
pub proof fn lemma_ends_bounded(v: Seq<Seq<u8>>)
    requires total(v) <= u64::MAX,
    ensures
        forall|i: int| 0 <= i < v.len() ==> #[trigger] ends(v)[i] as int <= total(v),
        forall|i: int| 0 <= i < v.len() ==> #[trigger] ends(v)[i] as int == total(v.take(i + 1)),
    decreases v.len(),
{
    lemma_total_nonneg(v);
    if v.len() > 0 {
        let u = v.drop_last();
        lemma_total_nonneg(u);
        lemma_ends_bounded(u);
        assert forall|i: int| 0 <= i < v.len() implies #[trigger] ends(v)[i] as int == total(v.take(i + 1)) by {
            if i < u.len() {
                assert(v.take(i + 1) =~= u.take(i + 1));
            } else {
                assert(v.take(i + 1) =~= v);
            }
        }
        assert forall|i: int| 0 <= i < v.len() implies #[trigger] ends(v)[i] as int <= total(v) by {
            if i < u.len() {
                assert(ends(v)[i] == ends(u)[i]);
            }
        }
    }
}

/// Entries do not change when more data follows, as long as every offset is inside the first
/// part.
pub proof fn lemma_entries_prefix(off: Seq<u64>, d1: Seq<u8>, d2: Seq<u8>)
    requires forall|i: int| 0 <= i < off.len() ==> off[i] as int <= d1.len(),
    ensures entries(off, d1 + d2) =~= entries(off, d1),
{
    assert forall|i: int| 0 <= i < off.len() implies entry(off, i, d1 + d2) == entry(off, i, d1) by {
        let prev: int = if i == 0 { 0 } else { off[i - 1] as int };
        if prev <= off[i] && off[i] <= d1.len() {
            assert((d1 + d2).subrange(prev, off[i] as int) =~= d1.subrange(prev, off[i] as int));
        }
    }
}

/// The entries of the encoding are the strings.
pub proof fn lemma_entries_of_encoding(v: Seq<Seq<u8>>)
    requires total(v) <= u64::MAX,
    ensures entries(ends(v), concat(v)) =~= v,
    decreases v.len(),
{
    lemma_total_nonneg(v);
    lemma_ends_bounded(v);
    if v.len() > 0 {
        let u = v.drop_last();
        lemma_total_nonneg(u);
        lemma_ends_bounded(u);
        lemma_entries_of_encoding(u);
        // The earlier entries do not see the last string.
        assert(ends(v) == ends(u).push(total(v) as u64));
        assert forall|i: int| 0 <= i < u.len() implies ends(u)[i] as int <= concat(u).len() by {}
        lemma_entries_prefix(ends(u), concat(u), v.last());
        assert forall|i: int| 0 <= i < u.len() implies entry(ends(v), i, concat(v)) == entry(ends(u), i, concat(v)) by {
            assert(ends(v)[i] == ends(u)[i]);
            if i > 0 { assert(ends(v)[i - 1] == ends(u)[i - 1]); }
        }
        // The last entry is the last string.
        let n = v.len() - 1;
        let prev: int = if n == 0 { 0 } else { ends(u)[n - 1] as int };
        assert(prev == total(u)) by {
            if n > 0 {
                assert(u.take(n) =~= u);
            }
        }
        assert(concat(v).subrange(total(u), total(v)) =~= v.last());
        assert(entry(ends(v), n, concat(v)) == v.last());
        assert(entries(ends(v), concat(v)) =~= entries(ends(u), concat(u)).push(v.last()));
        assert(u.push(v.last()) =~= v);
    }
}

/// Round trip with exact consumption (P-SV-01): for every vector of strings within the
/// library's limits, whatever precedes and follows.
pub proof fn lemma_svec_round_trip(prefix: Seq<u8>, v: Seq<Seq<u8>>, width: int, rest: Seq<u8>)
    requires
        1 <= width <= 8,
        v.len() <= MAX_DECODE_COUNT,
        total(v) <= MAX_STRING_DATA,
        forall|k: int| 0 <= k < v.len() ==> fits(ends(v)[k], width),
    ensures ({
        let enc = uvec(ends(v), width, 8) + concat(v);
        dec_svec(prefix + enc + rest, prefix.len() as int) == Some((v, (prefix.len() + enc.len()) as int))
    }),
{
    lemma_total_nonneg(v);
    lemma_ends_bounded(v);
    let u = uvec(ends(v), width, 8);
    let s = prefix + (u + concat(v)) + rest;
    assert(s =~= prefix + u + (concat(v) + rest));
    lemma_uvec_full_round_trip(prefix, ends(v), width, 8, concat(v) + rest);
    let p = (prefix.len() + u.len()) as int;
    if v.len() == 0 {
        assert(concat(v) =~= Seq::<u8>::empty());
    } else {
        let t = ends(v).last();
        assert(t as int == total(v));
        assert(s.subrange(p, p + t) =~= concat(v));
        lemma_entries_of_encoding(v);
    }
}

/// Restates `encode_string_vector`.
pub fn encode_string_vector(strings: &[Vec<u8>], w: &mut Vec<u8>) -> (width: usize)
    requires strings.len() <= 0x1000_0000_0000_0000, total(views(strings@)) <= 0x1000_0000_0000_0000,
    ensures
        1 <= width <= 8,
        forall|k: int| 0 <= k < strings.len() ==> fits(ends(views(strings@))[k], width as int),
        final(w)@ == old(w)@ + uvec(ends(views(strings@)), width as int, 8) + concat(views(strings@)),
{
    let ghost vs = views(strings@);
    let mut offset: u64 = 0;
    let mut offsets: Vec<u64> = Vec::new();
    let mut i: usize = 0;
    proof { lemma_total_nonneg(vs); }
    while i < strings.len()
        invariant
            i <= strings.len(), vs == views(strings@), total(vs) <= 0x1000_0000_0000_0000,
            offset as int == total(vs.take(i as int)),
            offsets@ =~= ends(vs.take(i as int)),
        decreases strings.len() - i,
    {
        proof {
            assert(vs.take((i + 1) as int).drop_last() =~= vs.take(i as int));
            assert(vs.take((i + 1) as int).last() == strings[i as int]@);
            lemma_total_prefix(vs, (i + 1) as int);
        }
        offset = offset + strings[i].len() as u64;
        offsets.push(offset);
        i += 1;
    }
    assert(vs.take(strings.len() as int) =~= vs);
    let width = encode_uint_vector_u64(&offsets, w);
    let ghost wo = w@;
    let mut i: usize = 0;
    while i < strings.len()
        invariant i <= strings.len(), vs == views(strings@), w@ =~= wo + concat(vs.take(i as int)),
        decreases strings.len() - i,
    {
        proof {
            assert(vs.take((i + 1) as int).drop_last() =~= vs.take(i as int));
            assert(vs.take((i + 1) as int).last() == strings[i as int]@);
        }
        w.extend_from_slice(strings[i].as_slice());
        i += 1;
    }
    assert(vs.take(strings.len() as int) =~= vs);
    width
}

/// A prefix's total is at most the whole total.
pub proof fn lemma_total_prefix(v: Seq<Seq<u8>>, k: int)
    requires 0 <= k <= v.len(),
    ensures 0 <= total(v.take(k)) <= total(v),
    decreases v.len(),
{
    lemma_total_nonneg(v);
    lemma_total_nonneg(v.take(k));
    if k < v.len() {
        assert(v.drop_last().take(k) =~= v.take(k));
        lemma_total_prefix(v.drop_last(), k);
    } else {
        assert(v.take(k) =~= v);
    }
}

/// Restates `decode_string_vector` (P-SV-02, FR-020): exactly the spec decoder's answer, for
/// every input.
pub fn decode_string_vector(buf: &[u8], pos: usize) -> (r: Option<(Vec<Vec<u8>>, usize)>)
    ensures
        r matches Some((_, p)) ==> pos < p <= buf.len(),
        r matches Some((v, p)) ==> dec_svec(buf@, pos as int) == Some((views(v@), p as int)),
        r is None ==> dec_svec(buf@, pos as int) is None,
{
    let (offsets, p) = match decode_uint_vector_u64(buf, pos) { Some(x) => x, None => return None };
    if offsets.len() == 0 {
        let out: Vec<Vec<u8>> = Vec::new();
        assert(views(out@) =~= Seq::<Seq<u8>>::empty());
        return Some((out, p));
    }
    let total_len: u64 = offsets[offsets.len() - 1];
    if total_len > MAX_STRING_DATA {
        return None;
    }
    if total_len > (buf.len() - p) as u64 {
        return None;
    }
    let end_all = p + total_len as usize;
    let data = slice_subrange(buf, p, end_all);
    let ghost off = offsets@;
    let mut result: Vec<Vec<u8>> = Vec::new();
    let mut start: u64 = 0;
    let mut i: usize = 0;
    while i < offsets.len()
        invariant
            i <= offsets.len(), off == offsets@, data@ == buf@.subrange(p as int, p + total_len),
            data@.len() == total_len,
            i == 0 ==> start == 0,
            i > 0 ==> start == off[i - 1],
            views(result@) =~= Seq::new(i as nat, |k: int| entry(off, k, data@)),
        decreases offsets.len() - i,
    {
        let end = offsets[i];
        let ghost r0 = views(result@);
        if start <= end && end <= total_len {
            let piece = slice_to_vec(slice_subrange(data, start as usize, end as usize));
            result.push(piece);
        } else {
            result.push(Vec::new());
        }
        proof {
            assert(entry(off, i as int, data@) == views(result@).last());
            assert(views(result@) =~= r0.push(entry(off, i as int, data@)));
        }
        start = end;
        i += 1;
    }
    proof {
        assert(entries(off, data@) =~= Seq::new(off.len(), |k: int| entry(off, k, data@)));
    }
    Some((result, end_all))
}

} // verus!
