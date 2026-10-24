// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The library's cell-identifier vector codec in `encoded_s2cell_id_vector.rs`,
//! `encode_s2cell_id_vector`, `encode_base_shift`, `decode_s2cell_id_vector` and
//! `decode_base_shift`, restated for Verus (spec 003 FR-010, P-CV-01, P-CV-02).
//!
//! Proved for every input and every length:
//! - the decoder returns exactly the spec decoder [`dec_idvec`]'s answer for every byte
//!   sequence and position: sound acceptance, crash-freedom and termination;
//! - decoding the encoder's output, whatever follows it, returns the identifiers and stops
//!   right after the encoding, for every sequence of 64-bit identifiers up to the library's
//!   count limit. That is the round trip with exact consumption, P-CV-01.
//!
//! Permitted differences (`mirrors.toml`):
//! - identifiers as raw `u64` (the library's `CellId(pub u64)`);
//! - a slice and a cursor for the reader, a `Vec<u8>` for the writer, `None` for an
//!   `io::Error`;
//! - `u32` instead of `i32` for the shift, shift code and delta MSB (every value is
//!   non-negative);
//! - `if` expressions for `min`, `max`, and `<<` for `wrapping_shl`, since every shift is
//!   below 64;
//! - an indexed loop for the deltas' `iter().map().collect()`.

use vstd::prelude::*;

use crate::fixed_width::*;
use crate::uint_vector::*;
use crate::varint::*;

verus! {

/// `max(1, len)`, the base-length rule of upstream's `EncodeBaseShift` and `Init`.
pub open spec fn len1(len: int) -> int {
    if len < 1 { 1 } else { len }
}

/// The base and shift of a header at `pos`, and the position after it, as upstream
/// `EncodedS2CellIdVector::Init` reads them (`encoded_s2cell_id_vector.cc:153–178`).
pub open spec fn dec_base_shift(s: Seq<u8>, pos: int) -> Option<(u64, u64, int)> {
    if pos < 0 || pos >= s.len() {
        None
    } else {
        let byte0 = s[pos];
        let base_len = (byte0 & 7) as int;
        let sc0 = (byte0 >> 3) as int;
        let p1 = pos + 1;
        if sc0 == 31 && p1 >= s.len() {
            None
        } else {
            let sc: int = if sc0 == 31 { 29 + s[p1] } else { sc0 };
            let p: int = if sc0 == 31 { p1 + 1 } else { p1 };
            if sc > 56 || base_len > s.len() - p {
                None
            } else {
                let raw = le_value(s.subrange(p, p + base_len), base_len);
                let base0: u64 = (raw << ((64 - 8 * len1(base_len)) as u64)) as u64;
                if sc >= 29 {
                    let sh: u64 = (2 * (sc - 29) + 1) as u64;
                    Some(((base0 | (1u64 << ((sh - 1) as u64))) as u64, sh, p + base_len))
                } else {
                    Some((base0, (2 * sc) as u64, p + base_len))
                }
            }
        }
    }
}

/// Identifier `i` from its delta: `base + (delta << shift)`, wrapping.
pub open spec fn recon(base: u64, shift: u64, d: u64) -> u64 {
    let x: int = base as int + ((d << shift) as u64) as int;
    if x > u64::MAX as int { (x - 0x1_0000_0000_0000_0000) as u64 } else { x as u64 }
}

/// The decoded identifiers from a delta sequence.
pub open spec fn recon_all(base: u64, shift: u64, d: Seq<u64>) -> Seq<u64> {
    d.map_values(|x: u64| recon(base, shift, x))
}

/// The whole decoder.
pub open spec fn dec_idvec(s: Seq<u8>, pos: int) -> Option<(Seq<u64>, int)> {
    match dec_base_shift(s, pos) {
        Some((b, sh, p)) => match dec_uvec(s, p, 8) {
            Some((d, q)) => Some((recon_all(b, sh, d), q)),
            None => None,
        },
        None => None,
    }
}

/// Restates `decode_base_shift`.
pub fn decode_base_shift(buf: &[u8], pos: usize) -> (r: Option<(u64, u64, usize)>)
    ensures
        r matches Some((b, sh, p)) ==> dec_base_shift(buf@, pos as int) == Some((b, sh, p as int)) && pos < p <= buf.len() && sh <= 56,
        r is None ==> dec_base_shift(buf@, pos as int) is None,
{
    if pos >= buf.len() {
        return None;
    }
    let code_plus_len = buf[pos];
    proof {
        assert(code_plus_len >> 3 <= 31) by (bit_vector);
        assert((code_plus_len & 7) <= 7) by (bit_vector);
    }
    let mut p = pos + 1;
    let mut shift_code: u64 = (code_plus_len >> 3) as u64;
    let base_len: usize = (code_plus_len & 7) as usize;
    if shift_code == 31 {
        if p >= buf.len() {
            return None;
        }
        shift_code = 29 + buf[p] as u64;
        p = p + 1;
        if shift_code > 56 {
            return None;
        }
    }
    let (raw_base, np) = match decode_uint_with_length(buf, p, base_len) { Some(x) => x, None => return None };
    let m: u64 = if base_len < 1 { 1 } else { base_len as u64 };
    let mut base: u64 = raw_base << (64 - 8 * m);
    let shift: u64 = if shift_code >= 29 {
        let s = 2 * (shift_code - 29) + 1;
        base = base | (1u64 << (s - 1));
        s
    } else {
        2 * shift_code
    };
    Some((base, shift, np))
}

/// Restates `decode_s2cell_id_vector` (P-CV-02): exactly the spec decoder's answer, for every
/// input.
pub fn decode_s2cell_id_vector(buf: &[u8], pos: usize) -> (r: Option<(Vec<u64>, usize)>)
    ensures
        r matches Some((_, p)) ==> pos < p <= buf.len(),
        r matches Some((v, p)) ==> dec_idvec(buf@, pos as int) == Some((v@, p as int)),
        r is None ==> dec_idvec(buf@, pos as int) is None,
{
    let (base, shift, p) = match decode_base_shift(buf, pos) { Some(x) => x, None => return None };
    let (deltas, q) = match decode_uint_vector_u64(buf, p) { Some(x) => x, None => return None };
    let mut result: Vec<u64> = Vec::new();
    let mut i: usize = 0;
    while i < deltas.len()
        invariant
            i <= deltas.len(), shift <= 56,
            result@ =~= recon_all(base, shift, deltas@.take(i as int)),
        decreases deltas.len() - i,
    {
        let d = deltas[i];
        let id = base.wrapping_add(d << shift);
        assert(id == recon(base, shift, d));
        let ghost r0 = result@;
        result.push(id);
        proof {
            assert(deltas@.take((i + 1) as int) =~= deltas@.take(i as int).push(d));
            assert(recon_all(base, shift, deltas@.take((i + 1) as int))
                =~= recon_all(base, shift, deltas@.take(i as int)).push(recon(base, shift, d)));
        }
        i += 1;
    }
    assert(deltas@.take(deltas.len() as int) =~= deltas@);
    Some((result, q))
}

// ---- the encoder ----------------------------------------------------------------------------

/// The low `n` bits.
pub open spec fn mask(n: u64) -> u64 {
    ((1u64 << n) - 1) as u64
}

/// The bit pattern below `shift` that every identifier shares, and that the decoder puts back
/// into the base: none for an even shift, bit `shift − 1` for an odd one.
pub open spec fn odd_bit(shift: u64) -> u64 {
    if shift % 2 == 1 { 1u64 << ((shift - 1) as u64) } else { 0u64 }
}

/// The shift code of upstream `EncodeBaseShift` (`encoded_s2cell_id_vector.cc:38–60`).
pub open spec fn shift_code(shift: u64) -> u64 {
    let c: u64 = if shift % 2 == 0 { shift / 2 } else { (shift / 2 + 29) as u64 };
    if c < 31 { c } else { 31 }
}

/// The header bytes `encode_base_shift` writes.
pub open spec fn base_shift_bytes(shift: u64, base: u64, base_len: int) -> Seq<u8> {
    let c = shift_code(shift);
    let byte0: u8 = #[verifier::truncate] (((c << 3u64) | (base_len as u64)) as u8);
    let ext: Seq<u8> = if c == 31 { seq![#[verifier::truncate] ((shift / 2) as u8)] } else { Seq::empty() };
    seq![byte0] + ext + le((base >> ((64 - 8 * len1(base_len)) as u64)) as u64, base_len)
}

/// Restates `encode_base_shift`.
pub fn encode_base_shift(w: &mut Vec<u8>, shift: u64, base: u64, base_len: usize)
    requires shift <= 56, base_len <= 7,
    ensures final(w)@ == old(w)@ + base_shift_bytes(shift, base, base_len as int),
{
    let ghost w0 = w@;
    proof {
        assert((shift & 1 == 0) == (shift % 2 == 0) && (shift >> 1) == shift / 2 && shift / 2 <= 28) by (bit_vector)
            requires shift <= 56;
    }
    let code: u64 = if shift & 1 == 0 { shift >> 1 } else { (shift >> 1) + 29 };
    let code: u64 = if code < 31 { code } else { 31 };
    assert(code == shift_code(shift));
    let bl = base_len as u64;
    proof {
        assert(((code << 3) | bl) <= 255) by (bit_vector) requires code <= 31, bl <= 7;
    }
    let byte0: u8 = ((code << 3) | bl) as u8;
    let ghost w1 = w@;
    w.push(byte0);
    if code == 31 {
        w.push((shift >> 1) as u8);
    }
    let ghost w2 = w@;
    let m: u64 = if base_len < 1 { 1 } else { base_len as u64 };
    let base_bytes = base >> (64 - 8 * m);
    encode_uint_with_length(w, base_bytes, base_len);
    proof {
        assert(len1(base_len as int) == m as int);
        assert((64 - 8 * len1(base_len as int)) as u64 == 64 - 8 * m);
        assert(w@ =~= w0 + base_shift_bytes(shift, base, base_len as int));
    }
}

/// The base and the pattern the encoder chose rebuild every identifier: `B | P` plus the
/// delta shifted back is `id`, with no wrap.
pub proof fn lemma_recon_bits(id: u64, b: u64, p: u64, s: u64)
    requires
        s <= 56,
        p == 0 || (s >= 1 && p == (1u64 << ((s - 1) as u64))),
        (id & mask(s)) == p,
        ((b & mask(s)) & !p) == 0,
        b <= id,
    ensures
        (b | p) <= id,
        ((((id - b) as u64) >> s) << s) as u64 == (id - (b | p)) as u64,
{
    assert((b | p) <= id && ((((id - b) as u64) >> s) << s) as u64 == ((id - (b | p)) as u64)) by (bit_vector)
        requires
            s <= 56,
            p == 0 || (s >= 1 && p == (1u64 << ((s - 1) as u64))),
            (id & (((1u64 << s) - 1) as u64)) == p,
            ((b & (((1u64 << s) - 1) as u64)) & !p) == 0,
            b <= id;
}

/// The decoder reads back what `encode_base_shift` wrote: the base with the odd bit, the
/// shift, and the header's length.
pub proof fn lemma_base_shift_round_trip(prefix: Seq<u8>, shift: u64, base: u64, base_len: int, rest: Seq<u8>)
    requires
        shift <= 56, 0 <= base_len <= 7,
        base_len == 0 ==> base == 0,
        base_len >= 1 ==> (base & mask((64 - 8 * base_len) as u64)) == 0,
    ensures ({
        let h = base_shift_bytes(shift, base, base_len);
        dec_base_shift(prefix + h + rest, prefix.len() as int)
            == Some(((base | odd_bit(shift)) as u64, shift, (prefix.len() + h.len()) as int))
    }),
{
    let h = base_shift_bytes(shift, base, base_len);
    let s = prefix + h + rest;
    let pos = prefix.len() as int;
    let c = shift_code(shift);
    let bl = base_len as u64;
    let byte0: u8 = #[verifier::truncate] (((c << 3u64) | bl) as u8);
    let k: u64 = (64 - 8 * len1(base_len)) as u64;
    let bb: u64 = (base >> k) as u64;
    let tail = le(bb, base_len);
    let ext: Seq<u8> = if c == 31 { seq![#[verifier::truncate] ((shift / 2) as u8)] } else { Seq::empty() };
    let hl: int = 1 + ext.len() as int;
    assert(h == seq![byte0] + ext + tail);
    assert(h.len() == hl + base_len);
    assert(c <= 31);
    assert((byte0 & 7) as u64 == bl && (byte0 >> 3) as u64 == c) by (bit_vector)
        requires c <= 31, bl <= 7, byte0 == (((c << 3u64) | bl) as u8);
    // Positions inside s.
    assert(s[pos] == h[0]);
    assert(h[0] == byte0);
    let p = pos + hl;
    assert(s.subrange(p, p + base_len) =~= tail) by {
        assert forall|j: int| 0 <= j < base_len implies s[p + j] == tail[j] by {
            assert(s[p + j] == h[hl + j]);
            assert(h[hl + j] == tail[j]);
        }
    }
    assert(s.subrange(p, p + base_len).subrange(0, base_len) =~= tail);
    assert(s.len() >= p + base_len);
    // The base bytes read back.
    let raw = le_value(s.subrange(p, p + base_len), base_len);
    if base_len >= 1 {
        assert(k == (64 - 8 * bl) as u64);
        assert(fits(bb, base_len)) by {
            assert(((bb >> ((8 * bl) as u64)) as u64) == 0) by (bit_vector)
                requires bl >= 1, bl <= 7, bb == (base >> ((64 - 8 * bl) as u64)) as u64;
            assert((8 * base_len) as u64 == (8 * bl) as u64);
        }
        lemma_le_round_trip(s.subrange(p, p + base_len), bb, base_len);
        assert(raw == bb);
        assert(((bb << k) as u64) == base) by (bit_vector)
            requires bb == (base >> k) as u64, k <= 56, k >= 8, (base & (((1u64 << k) - 1) as u64)) == 0;
    } else {
        assert(raw == 0);
        assert(k == 56);
        assert(((0u64 << 56u64) as u64) == 0) by (bit_vector);
    }
    assert(((raw << k) as u64) == base);
    if c == 31 {
        let e: u8 = #[verifier::truncate] ((shift / 2) as u8);
        assert(s[pos + 1] == h[1]);
        assert(h[1] == e);
        assert(shift % 2 == 1 && shift / 2 + 29 >= 31 && shift / 2 <= 28);
        assert(e as u64 == shift / 2);
        assert(29 + e <= 56);
        assert((2 * ((29 + e) - 29) + 1) as u64 == shift);
    } else if c >= 29 {
        assert(shift % 2 == 1);
        assert((2 * (c - 29) + 1) as u64 == shift);
    } else {
        assert(shift % 2 == 0 && 2 * c == shift);
        assert((base | 0u64) == base) by (bit_vector);
        assert(odd_bit(shift) == 0u64);
    }
}

/// The whole round trip, given the facts the encoder establishes about its choice of base and
/// shift.
pub proof fn lemma_idvec_round_trip(
    prefix: Seq<u8>, v: Seq<u64>, base: u64, base_len: int, shift: u64,
    deltas: Seq<u64>, width: int, rest: Seq<u8>,
)
    requires
        shift <= 56, 0 <= base_len <= 7, 1 <= width <= 8,
        base_len == 0 ==> base == 0,
        base_len >= 1 ==> (base & mask((64 - 8 * base_len) as u64)) == 0,
        v.len() <= MAX_DECODE_COUNT,
        deltas.len() == v.len(),
        forall|k: int| 0 <= k < v.len() ==> deltas[k] == ((v[k] - base) as u64) >> shift,
        forall|k: int| 0 <= k < v.len() ==> base <= v[k],
        forall|k: int| 0 <= k < v.len() ==> (v[k] & mask(shift)) == odd_bit(shift),
        ((base & mask(shift)) & !odd_bit(shift)) == 0,
        forall|k: int| 0 <= k < deltas.len() ==> fits(deltas[k], width),
    ensures ({
        let enc = base_shift_bytes(shift, base, base_len) + uvec(deltas, width, 8);
        dec_idvec(prefix + enc + rest, prefix.len() as int) == Some((v, (prefix.len() + enc.len()) as int))
    }),
{
    let h = base_shift_bytes(shift, base, base_len);
    let u = uvec(deltas, width, 8);
    let s = prefix + (h + u) + rest;
    assert(s =~= prefix + h + (u + rest));
    lemma_base_shift_round_trip(prefix, shift, base, base_len, u + rest);
    assert(s =~= (prefix + h) + u + rest);
    lemma_uvec_full_round_trip(prefix + h, deltas, width, 8, rest);
    let b2 = (base | odd_bit(shift)) as u64;
    assert forall|k: int| 0 <= k < v.len() implies recon(b2, shift, deltas[k]) == v[k] by {
        let p = odd_bit(shift);
        assert(p == 0 || (shift >= 1 && p == (1u64 << ((shift - 1) as u64))));
        lemma_recon_bits(v[k], base, p, shift);
    }
    assert(recon_all(b2, shift, deltas) =~= v);
}

/// Restates `encode_s2cell_id_vector` (P-CV-01): for every sequence of identifiers up to the
/// count limit, the decoder reads back exactly `v` from the bytes written, whatever follows.
pub fn encode_s2cell_id_vector(v: &[u64], w: &mut Vec<u8>)
    requires v.len() <= 0x1000_0000_0000_0000,
    ensures
        v.len() <= MAX_DECODE_COUNT ==> forall|rest: Seq<u8>|
            #[trigger] dec_idvec(final(w)@ + rest, old(w)@.len() as int) == Some((v@, final(w)@.len() as int)),
{
    broadcast use vstd::std_specs::bits::axiom_u64_trailing_zeros, vstd::std_specs::bits::axiom_u64_leading_zeros;
    let ghost w0 = w@;
    let ghost n = v.len() as int;
    let mut v_or: u64 = 0;
    let mut v_and: u64 = !0u64;
    let mut v_min: u64 = !0u64;
    let mut v_max: u64 = 0;
    let ghost mut kmin: int = 0;
    assert(!0u64 == 0xffff_ffff_ffff_ffffu64) by (bit_vector);
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(), n == v.len(),
            forall|k: int| 0 <= k < i ==> (#[trigger] v[k] | v_or) == v_or,
            forall|k: int| 0 <= k < i ==> (#[trigger] v[k] & v_and) == v_and,
            forall|k: int| 0 <= k < i ==> v_min <= #[trigger] v[k] <= v_max,
            i > 0 ==> 0 <= kmin < i && v[kmin] == v_min,
            i == 0 ==> v_or == 0 && v_and == !0u64 && v_min == !0u64 && v_max == 0,
            !0u64 == 0xffff_ffff_ffff_ffffu64,
        decreases v.len() - i,
    {
        let id = v[i];
        let ghost (o0, a0) = (v_or, v_and);
        proof {
            assert forall|k: int| 0 <= k < i implies (#[trigger] v[k] | ((o0 | id) as u64)) == ((o0 | id) as u64) by {
                let x = v[k];
                assert((x | o0) == o0 ==> (x | ((o0 | id) as u64)) == ((o0 | id) as u64)) by (bit_vector);
            }
            assert((id | ((o0 | id) as u64)) == ((o0 | id) as u64)) by (bit_vector);
            assert forall|k: int| 0 <= k < i implies (#[trigger] v[k] & ((a0 & id) as u64)) == ((a0 & id) as u64) by {
                let x = v[k];
                assert((x & a0) == a0 ==> (x & ((a0 & id) as u64)) == ((a0 & id) as u64)) by (bit_vector);
            }
            assert((id & ((a0 & id) as u64)) == ((a0 & id) as u64)) by (bit_vector);
        }
        v_or = v_or | id;
        v_and = v_and & id;
        if id < v_min {
            v_min = id;
            proof { kmin = i as int; }
        } else if i == 0 {
            proof { kmin = 0; }
        }
        if id > v_max {
            v_max = id;
        }
        i += 1;
    }
    let mut e_base: u64 = 0;
    let mut e_base_len: usize = 0;
    let mut e_shift: u64 = 0;
    let mut e_max_delta_msb: u64 = 0;
    if v_or > 0 {
        let tz: u64 = v_or.trailing_zeros() as u64;
        let e0: u64 = (if tz < 56 { tz } else { 56 }) & !1u64;
        proof {
            assert(e0 <= 56 && e0 <= tz && e0 % 2 == 0) by (bit_vector)
                requires e0 == ((if tz < 56 { tz } else { 56 }) & !1u64) as u64;
            assert(tz < 64);
            assert((v_or << ((64 - tz) as u64)) == 0);
            assert((v_or & mask(e0)) == 0) by (bit_vector)
                requires tz < 64, e0 <= tz, (v_or << ((64 - tz) as u64)) == 0;
            assert forall|k: int| 0 <= k < n implies (#[trigger] v[k] & mask(e0)) == 0 by {
                let x = v[k];
                assert((x | v_or) == v_or);
                assert((x & mask(e0)) == 0) by (bit_vector)
                    requires (x | v_or) == v_or, (v_or & mask(e0)) == 0;
            }
        }
        e_shift = e0;
        if v_and & (1u64 << e_shift) != 0 {
            e_shift = e_shift + 1;
            proof {
                assert forall|k: int| 0 <= k < n implies (#[trigger] v[k] & mask(e_shift)) == odd_bit(e_shift) by {
                    let x = v[k];
                    assert((x & v_and) == v_and);
                    assert((x & mask(e0)) == 0);
                    assert((x & mask((e0 + 1) as u64)) == (1u64 << e0)) by (bit_vector)
                        requires e0 <= 56, (x & v_and) == v_and, (v_and & (1u64 << e0)) != 0, (x & (((1u64 << e0) - 1) as u64)) == 0;
                    assert(e_shift % 2 == 1 && ((e_shift - 1) as u64) == e0);
                }
            }
        } else {
            proof {
                assert forall|k: int| 0 <= k < n implies (#[trigger] v[k] & mask(e_shift)) == odd_bit(e_shift) by {
                    assert((v[k] & mask(e0)) == 0);
                }
            }
        }
        let ghost e1 = e_shift;
        let mut e_bytes: u64 = u64::MAX;
        let mut len: usize = 0;
        assert((v_min & !(u64::MAX >> 0u64)) == 0) by (bit_vector);
        assert(n > 0) by {
            if n == 0 { assert(v_or == 0); }
        }
        while len < 8
            invariant
                len <= 8, e_shift == e1, e1 == e0 || e1 == e0 + 1, e0 <= 56, e0 % 2 == 0,
                n == v.len(), 0 < n <= 0x1000_0000_0000_0000,
                0 <= kmin < n, v[kmin] == v_min, v_min <= v_max,
                forall|k: int| 0 <= k < n ==> v_min <= #[trigger] v[k],
                forall|k: int| 0 <= k < n ==> (#[trigger] v[k] & mask(e1)) == odd_bit(e1),
                e_base_len <= 7, e_max_delta_msb <= 63,
                e1 == 57 ==> e_max_delta_msb <= 6,
                e_base == (v_min & !(u64::MAX >> ((8 * e_base_len) as u64))) as u64,
            decreases 8 - len,
        {
            let t_base = v_min & !(u64::MAX >> (8 * len as u64));
            proof {
                assert(t_base <= v_min) by (bit_vector) requires t_base == (v_min & !(u64::MAX >> ((8 * len) as u64))) as u64;
            }
            let x = (v_max - t_base) >> e_shift;
            let lz = x.leading_zeros() as u64;
            proof {
                assert(lz <= 64);
                if e_shift == 57 {
                    assert(x < 128) by (bit_vector) requires x == ((v_max - t_base) as u64) >> 57u64;
                    assert(lz >= 57) by (bit_vector)
                        requires x < 128, lz <= 64, lz < 64 ==> (x >> ((63 - lz) as u64)) & 1u64 != 0u64;
                }
            }
            let wdt = 64 - lz;
            let t_max_delta_msb: u64 = (if wdt > 1 { wdt } else { 1 }) - 1;
            proof {
                assert(t_max_delta_msb <= 63);
                assert((t_max_delta_msb >> 3) <= 7) by (bit_vector) requires t_max_delta_msb <= 63;
                assert((v.len() as u64) * ((t_max_delta_msb >> 3) + 1) <= 0x8000_0000_0000_0000) by (nonlinear_arith)
                    requires (v.len() as u64) <= 0x1000_0000_0000_0000, (t_max_delta_msb >> 3) <= 7;
            }
            let t_bytes = len as u64 + (v.len() as u64) * ((t_max_delta_msb >> 3) + 1);
            if t_bytes < e_bytes {
                e_base = t_base;
                e_base_len = len;
                e_max_delta_msb = t_max_delta_msb;
                e_bytes = t_bytes;
            }
            len += 1;
        }
        proof {
            assert((e_shift & 1) != 0 ==> e_shift >= 1) by (bit_vector);
            assert(e_max_delta_msb <= 6 ==> (e_max_delta_msb & 7) != 7) by (bit_vector);
            assert((e_shift & 1 != 0) == (e_shift % 2 == 1)) by (bit_vector);
        }
        if (e_shift & 1) != 0 && (e_max_delta_msb & 7) != 7 {
            e_shift = e_shift - 1;
            proof {
                // Back to the even shift e0: nothing below it.
                assert forall|k: int| 0 <= k < n implies (#[trigger] v[k] & mask(e_shift)) == odd_bit(e_shift) by {
                    let x = v[k];
                    assert((x & mask(e1)) == odd_bit(e1));
                    assert(e1 == e0 + 1 && e_shift == e0);
                    assert((x & mask(e0)) == 0) by (bit_vector)
                        requires e0 <= 56, (x & mask((e0 + 1) as u64)) == (1u64 << e0);
                }
            }
        }
        proof {
            assert(e_shift <= 56);
            // The base's low bits are a subset of v_min's, which follow the pattern.
            let t = e_shift;
            let b = e_base;
            let L = e_base_len as u64;
            assert((v_min & mask(t)) == odd_bit(t));
            assert(((b & mask(t)) & !odd_bit(t)) == 0) by (bit_vector)
                requires b == (v_min & !(u64::MAX >> ((8 * L) as u64))) as u64, (v_min & mask(t)) == odd_bit(t);
            assert(b <= v_min) by (bit_vector) requires b == (v_min & !(u64::MAX >> ((8 * L) as u64))) as u64;
            assert(L == 0 ==> b == 0) by (bit_vector) requires b == (v_min & !(u64::MAX >> ((8 * L) as u64))) as u64;
            assert((L >= 1 && L <= 7) ==> (b & mask((64 - 8 * L) as u64)) == 0) by (bit_vector)
                requires b == (v_min & !(u64::MAX >> ((8 * L) as u64))) as u64;
        }
    } else {
        proof {
            assert forall|k: int| 0 <= k < n implies (#[trigger] v[k] & mask(0)) == odd_bit(0) && v[k] == 0 by {
                let x = v[k];
                assert((x | v_or) == v_or);
                assert(x == 0 && (x & mask(0)) == 0) by (bit_vector) requires (x | v_or) == v_or, v_or == 0;
            }
            assert(((0u64 & mask(0)) & !odd_bit(0)) == 0) by (bit_vector);
        }
    }
    let ghost shift = e_shift;
    let ghost base = e_base;
    let ghost base_len = e_base_len as int;
    proof {
        assert(shift <= 56 && base_len <= 7);
        assert(forall|k: int| 0 <= k < n ==> base <= #[trigger] v[k]);
        assert(forall|k: int| 0 <= k < n ==> (#[trigger] v[k] & mask(shift)) == odd_bit(shift));
        assert(((base & mask(shift)) & !odd_bit(shift)) == 0);
        assert(base_len == 0 ==> base == 0);
        assert(base_len >= 1 ==> (base & mask((64 - 8 * base_len) as u64)) == 0);
    }
    encode_base_shift(w, e_shift, e_base, e_base_len);
    let ghost wh = w@;
    let mut deltas: Vec<u64> = Vec::new();
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(), deltas.len() == i, e_shift == shift, e_base == base, shift <= 56, n == v.len(),
            forall|k: int| 0 <= k < n ==> base <= #[trigger] v[k],
            forall|k: int| 0 <= k < i ==> deltas[k] == ((v[k] - base) as u64) >> shift,
        decreases v.len() - i,
    {
        deltas.push((v[i] - e_base) >> e_shift);
        i += 1;
    }
    let width = encode_uint_vector_u64(&deltas, w);
    proof {
        let enc = base_shift_bytes(shift, base, base_len) + uvec(deltas@, width as int, 8);
        assert(w@ =~= w0 + enc);
        if v.len() <= MAX_DECODE_COUNT {
            assert forall|rest: Seq<u8>|
                #[trigger] dec_idvec(w@ + rest, w0.len() as int) == Some((v@, w@.len() as int)) by {
                lemma_idvec_round_trip(w0, v@, base, base_len, shift, deltas@, width as int, rest);
                assert(w@ + rest =~= w0 + enc + rest);
            }
        }
    }
}

} // verus!
