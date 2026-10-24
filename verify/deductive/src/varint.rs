// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The library's varint codec, `encoding::write_uvarint` and `encoding::read_uvarint`,
//! restated for Verus (spec 003 FR-010; contracts/deductive-conventions.md).
//!
//! Proved for every input: the encoder appends exactly [`varint`]; the decoder returns exactly
//! what the spec decoder [`dec`] returns for every byte sequence and position (so it
//! terminates and never overflows or indexes out of bounds); and decoding the encoding of any
//! `u64`, whatever precedes and follows it, returns the value and stops right after it
//! ([`lemma_round_trip`]).
//!
//! Permitted differences (`mirrors.toml`): a slice and a cursor for `&mut dyn Read`, a
//! `Vec<u8>` for `&mut dyn Write`, `None` for an `io::Error`.

use vstd::prelude::*;

verus! {

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

/// Restates `write_uvarint`.
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
/// position `i` on, already shifted by `shift`, and the position after the last byte read. A
/// tenth byte (shift 63) above 1 is rejected, as upstream does (the D-003 fix).
pub open spec fn dec(s: Seq<u8>, i: int, shift: u64) -> Option<(u64, int)>
    decreases s.len() - i,
{
    if i < 0 || i >= s.len() || shift >= 64 {
        None
    } else if shift == 63 && s[i] > 1 {
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

/// Restates `read_uvarint`, over a slice and a cursor; returns the value and the new cursor.
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
        if shift == 63 && b > 1 {
            return None;
        }
        let part: u64 = ((b & 0x7f) as u64) << shift;
        if b < 0x80 {
            assert(((result | part) as u64) == (result | part));
            return Some((result | part, i + 1));
        }
        // The library has no check here: shift is a multiple of 7 below 64, and not 63 (a byte
        // of 0x80 or more at 63 was rejected above), so it is at most 56.
        assert(shift + 7 < 64);
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
        // At shift 63 the value is 0 or 1, so the tenth-byte check passes.
        assert(shift == 63 ==> b <= 1) by (bit_vector)
            requires y < 128, b == y as u8, shift < 64, shift == 0 || (y >> ((64 - shift) as u64)) == 0;
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

/// Round trip with exact consumption, for every `u64`, whatever precedes and follows.
pub proof fn lemma_round_trip(prefix: Seq<u8>, x: u64, rest: Seq<u8>)
    ensures dec(prefix + varint(x) + rest, prefix.len() as int, 0)
        == Some((x, (prefix.len() + varint(x).len()) as int)),
{
    let s = prefix + varint(x) + rest;
    assert(s.subrange(prefix.len() as int, (prefix.len() + varint(x).len()) as int) =~= varint(x));
    lemma_dec_varint(s, prefix.len() as int, x, 0);
    assert((x << 0u64) == x) by (bit_vector);
}

} // verus!
