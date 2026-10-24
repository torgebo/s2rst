// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The reference description of the in-scope wire formats (spec 003 FR-007; data-model §1;
//! research R3).
//!
//! It has one **reference decoder** `ref_<fmt>` per format, returning the decoded value and
//! the number of bytes consumed, or `None` where upstream rejects the input, and one
//! **canonical encoder** `enc_<fmt>`, the bytes upstream's encoder writes.
//!
//! - Every function is written from upstream C++ at `v0.14.0-40` (`/home/t/sources/s2geometry`,
//!   the pinned oracle plus one example commit). The source is cited in each doc comment.
//!   Nothing here calls `s2rst`'s coding modules (FR-007).
//! - The library's **documented deviations** (FR-022) are separate predicates, `dev_*`. The
//!   `lib_*` functions apply them on top of the upstream meaning. Each is "upstream's answer,
//!   minus the documented deviations", which is what the agreement properties check the
//!   library against.
//! - Loops over slices are indexed, never iterator adapters, so CBMC sees their trip counts
//!   (memory note "Kani collection-harness lessons").
//!
//! The file is plain Rust, compiled outside Kani too, and sanity-tested against
//! `core/tests/data/cpp_coding_vectors.csv` by `tests/wire_spec_vs_cpp.rs`.

#![allow(
    clippy::needless_range_loop,
    reason = "indexed loops keep CBMC's trip counts exact (research R0)"
)]

use crate::spec;

/// The library's element-count limit for integer vectors (`MAX_DECODE_COUNT` in
/// `encoded_uint_vector.rs`). Upstream has none (FR-022).
pub const LIB_MAX_COUNT: u64 = 50_000_000;
/// The library's point-count limit for point vectors (`MAX_DECODE_POINTS`). Upstream's is
/// `i32::MAX` (`encoded_s2point_vector.cc:251`).
pub const LIB_MAX_POINTS: u64 = 50_000_000;
/// The library's limit on a string vector's data bytes. Upstream has none (FR-022).
pub const LIB_MAX_STRING_DATA: u64 = 500_000_000;
/// Upstream's point-count limit (`encoded_s2point_vector.cc:251`).
pub const UPSTREAM_MAX_POINTS: u64 = i32::MAX as u64;

// ---- varint ---------------------------------------------------------------------------------

/// Upstream `Decoder::get_varint64` (`util/coding/coder.h:619`, `Varint::Parse64WithLimit`
/// and `Parse64`, `util/coding/varint.cc:162–262`): little-endian groups of 7 bits, the high
/// bit marking a continuation, at most 10 bytes, and a tenth byte of at most 1 (bits beyond
/// 64 are "too long to be a varint64"). Over-long forms (`80 00` for 0) are accepted.
pub fn ref_varint(b: &[u8]) -> Option<(u64, usize)> {
    varint_upstream(b)
}

/// The library's `read_uvarint`: upstream's answer exactly. (Until the D-003 fix the library
/// accepted a tenth byte from 2 to 127; research R1, EVALUATION.md "Defects found".)
pub fn lib_varint(b: &[u8]) -> Option<(u64, usize)> {
    ref_varint(b)
}

fn varint_upstream(b: &[u8]) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    for i in 0..10 {
        if i >= b.len() {
            return None;
        }
        let byte = b[i];
        if i == 9 {
            // Upstream: `if (b < 2) goto done; return nullptr;`.
            if byte < 2 {
                return Some((value | (u64::from(byte & 1) << 63), 10));
            }
            return None;
        }
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte < 0x80 {
            return Some((value, i + 1));
        }
    }
    None
}

/// Upstream `Encoder::put_varint64`: the shortest form.
pub fn enc_varint(x: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10);
    put_varint(&mut out, x);
    out
}

// The encoders append to one buffer whose capacity comes from concrete counts, never from
// values: under the checker, a temporary `Vec` per field, or a capacity sized from data, is
// the memory wall of research R0 (finding 2). The bytes are the same either way.

/// Appends [`enc_varint`]`(x)`, as one slice (one growth check, as the library's writes).
fn put_varint(out: &mut Vec<u8>, mut x: u64) {
    let mut buf = [0u8; 10];
    let mut n = 0;
    while x >= 0x80 {
        buf[n] = (x & 0x7f) as u8 | 0x80;
        x >>= 7;
        n += 1;
    }
    buf[n] = x as u8;
    out.extend_from_slice(&buf[..=n]);
}

/// Appends [`enc_fixed`]`(v, len)`, as one slice.
fn put_fixed(out: &mut Vec<u8>, v: u64, len: usize) {
    let mut buf = [0u8; 8];
    for i in 0..8 {
        buf[i] = (v >> (8 * i)) as u8;
    }
    out.extend_from_slice(&buf[..len]);
}

// ---- fixed width ----------------------------------------------------------------------------

/// Upstream `DecodeUintWithLength` (`encoded_uint_vector.h:180–200`): `len` little-endian
/// bytes, rejected if fewer are available. REQUIRES `len <= 8`.
pub fn ref_fixed(b: &[u8], len: usize) -> Option<(u64, usize)> {
    if b.len() < len {
        return None;
    }
    // Straight-line, without a loop: a loop here would set every agreement harness's unwind to
    // at least 9, and so unroll the library's own loops that far even on 2-byte inputs.
    let byte = |i: usize| {
        if i < len {
            u64::from(b[i]) << (8 * i)
        } else {
            0
        }
    };
    let value = byte(0) | byte(1) | byte(2) | byte(3) | byte(4) | byte(5) | byte(6) | byte(7);
    Some((value, len))
}

/// Upstream `EncodeUintWithLength` (`encoded_uint_vector.h:139–153`): the low `len` bytes of
/// `v`, little-endian. REQUIRES `len <= 8`.
pub fn enc_fixed(v: u64, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    put_fixed(&mut out, v, len);
    out
}

/// The minimal number of bytes for `v`, at least 1: upstream's
/// `((bit_width(one_bits) - 1) >> 3) + 1` (`encoded_uint_vector.h:216`).
pub fn byte_width(v: u64) -> usize {
    let bits = 64 - v.leading_zeros() as usize;
    if bits == 0 { 1 } else { ((bits - 1) >> 3) + 1 }
}

// ---- integer vectors ---------------------------------------------------------------------------

/// Upstream `EncodedUintVector<T>::Init` + `Decode` (`encoded_uint_vector.h:229–310`) for
/// `size_of_t` 4 or 8: a varint `h`, then `h / size_of_t` elements of `(h mod size_of_t) + 1`
/// bytes each, rejected if `size > max / size_of_t` or if the bytes are not all there.
pub fn ref_uint_vector(b: &[u8], size_of_t: u64) -> Option<(Vec<u64>, usize)> {
    uint_vector_with(b, size_of_t, ref_varint, None)
}

/// The library's `decode_uint_vector_u32`/`_u64` as documented: upstream's answer, minus
/// `dev_count` (more than [`LIB_MAX_COUNT`] elements rejected) and with
/// [`lib_varint`]'s header.
pub fn lib_uint_vector(b: &[u8], size_of_t: u64) -> Option<(Vec<u64>, usize)> {
    uint_vector_with(b, size_of_t, lib_varint, Some(LIB_MAX_COUNT))
}

/// FR-022: the library rejects a count above its limit, upstream does not.
pub fn dev_count(count: u64, limit: u64) -> bool {
    count > limit
}

fn uint_vector_with(
    b: &[u8],
    size_of_t: u64,
    varint: fn(&[u8]) -> Option<(u64, usize)>,
    limit: Option<u64>,
) -> Option<(Vec<u64>, usize)> {
    let (h, p) = varint(b)?;
    let count = h / size_of_t;
    let len = ((h & (size_of_t - 1)) + 1) as usize;
    if let Some(limit) = limit
        && dev_count(count, limit)
    {
        return None;
    }
    if count > u64::MAX / size_of_t {
        return None;
    }
    let bytes = u128::from(count) * len as u128;
    if ((b.len() - p) as u128) < bytes {
        return None;
    }
    let count = count as usize;
    let mut out = Vec::new();
    for i in 0..count {
        let off = p + i * len;
        let (x, _) = ref_fixed(&b[off..], len)?;
        out.push(x);
    }
    Some((out, p + count * len))
}

/// Upstream `EncodeUintVector<T>` (`encoded_uint_vector.h:204–226`).
pub fn enc_uint_vector(v: &[u64], size_of_t: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + 8 * v.len());
    put_uint_vector(&mut out, v, size_of_t);
    out
}

/// Appends [`enc_uint_vector`]`(v, size_of_t)`.
fn put_uint_vector(out: &mut Vec<u8>, v: &[u64], size_of_t: u64) {
    let mut one_bits: u64 = 1;
    for i in 0..v.len() {
        one_bits |= v[i];
    }
    let len = byte_width(one_bits);
    put_varint(out, (v.len() as u64 * size_of_t) | (len as u64 - 1));
    for i in 0..v.len() {
        put_fixed(out, v[i], len);
    }
}

// ---- cell-identifier vector --------------------------------------------------------------------

/// Upstream `EncodedS2CellIdVector::Init` + `Decode` (`encoded_s2cell_id_vector.cc:153–190`):
/// a byte `shift_code << 3 | base_len`; for `shift_code == 31` an extended byte `s` with
/// `29 + s <= 56`; `base_len` bytes of the base's top bytes; a 64-bit integer vector of
/// deltas. Value `i` is `base + (delta_i << shift)`, wrapping. Identifiers are not validated.
pub fn ref_cellid_vector(b: &[u8]) -> Option<(Vec<u64>, usize)> {
    cellid_vector_with(b, ref_uint_vector)
}

/// The library's `decode_s2cell_id_vector` as documented: upstream's answer, with the
/// library's integer vector for the deltas ([`lib_uint_vector`]).
pub fn lib_cellid_vector(b: &[u8]) -> Option<(Vec<u64>, usize)> {
    cellid_vector_with(b, lib_uint_vector)
}

fn cellid_vector_with(b: &[u8], deltas: UintVectorDecoder) -> Option<(Vec<u64>, usize)> {
    if b.len() < 2 {
        return None;
    }
    let code_plus_len = b[0];
    let mut p = 1;
    let mut shift_code = u32::from(code_plus_len >> 3);
    if shift_code == 31 {
        shift_code = 29 + u32::from(b[1]);
        p = 2;
        if shift_code > 56 {
            return None;
        }
    }
    let base_len = usize::from(code_plus_len & 7);
    let (raw, n) = ref_fixed(&b[p..], base_len)?;
    p += n;
    let mut base = raw << (64 - 8 * base_len.max(1));
    let shift = if shift_code >= 29 {
        let s = 2 * (shift_code - 29) + 1;
        base |= 1u64 << (s - 1);
        s
    } else {
        2 * shift_code
    };
    let (d, n) = deltas(&b[p..], 8)?;
    let mut out = Vec::new();
    for i in 0..d.len() {
        out.push(base.wrapping_add(d[i] << shift));
    }
    Some((out, p + n))
}

/// Upstream `EncodeS2CellIdVector` and `EncodeBaseShift` (`encoded_s2cell_id_vector.cc:38–151`):
/// the even shift from the trailing zeros of the OR (odd when every id has the same low bit),
/// the base length 0–7 that minimises the size, then the odd-to-even shift adjustment.
pub fn enc_cellid_vector(v: &[u64]) -> Vec<u8> {
    let (mut v_or, mut v_and, mut v_min, mut v_max) = (0u64, !0u64, !0u64, 0u64);
    for i in 0..v.len() {
        v_or |= v[i];
        v_and &= v[i];
        v_min = v_min.min(v[i]);
        v_max = v_max.max(v[i]);
    }
    let (mut e_base, mut e_base_len, mut e_shift, mut e_max_delta_msb) = (0u64, 0usize, 0u32, 0u32);
    if v_or > 0 {
        e_shift = 56.min(v_or.trailing_zeros() & !1);
        if v_and & (1u64 << e_shift) != 0 {
            e_shift += 1;
        }
        let mut e_bytes = u64::MAX;
        for len in 0..8usize {
            let t_base = v_min & !(u64::MAX.checked_shr(8 * len as u32).unwrap_or(0));
            let w = 64 - ((v_max - t_base) >> e_shift).leading_zeros();
            let t_max_delta_msb = w.max(1) - 1;
            let t_bytes = len as u64 + v.len() as u64 * (u64::from(t_max_delta_msb >> 3) + 1);
            if t_bytes < e_bytes {
                e_base = t_base;
                e_base_len = len;
                e_max_delta_msb = t_max_delta_msb;
                e_bytes = t_bytes;
            }
        }
        if e_shift & 1 != 0 && e_max_delta_msb & 7 != 7 {
            e_shift -= 1;
        }
    }
    // EncodeBaseShift.
    let mut shift_code = e_shift >> 1;
    if e_shift & 1 != 0 {
        shift_code = 31.min(shift_code + 29);
    }
    let mut out = Vec::with_capacity(2 + 7 + 10 + 8 * v.len());
    out.push(((shift_code << 3) as u8) | e_base_len as u8);
    if shift_code == 31 {
        out.push((e_shift >> 1) as u8);
    }
    let base_bytes = e_base >> (64 - 8 * e_base_len.max(1));
    put_fixed(&mut out, base_bytes, e_base_len);
    let mut deltas = Vec::with_capacity(v.len());
    for i in 0..v.len() {
        deltas.push((v[i] - e_base) >> e_shift);
    }
    put_uint_vector(&mut out, &deltas, 8);
    out
}

// ---- string vector ---------------------------------------------------------------------------

/// Upstream `EncodedStringVector::Init` + `Decode` at v0.14 (`encoded_string_vector.cc:57–84`,
/// the lazy check of commit `95b1e15`): a 64-bit integer vector of end offsets, then
/// `offsets.last()` data bytes (rejected if not all there). Entry `i` is
/// `data[offsets[i-1]..offsets[i]]` when that range is in order and inside the data, else
/// empty, and decoding continues from `offsets[i]`.
pub fn ref_string_vector(b: &[u8]) -> Option<(Vec<Vec<u8>>, usize)> {
    string_vector_with(b, ref_uint_vector, None)
}

/// The library's `decode_string_vector` as documented (FR-020, FR-022): upstream's answer,
/// minus `dev_string_total` and with [`lib_uint_vector`]'s offsets.
pub fn lib_string_vector(b: &[u8]) -> Option<(Vec<Vec<u8>>, usize)> {
    string_vector_with(b, lib_uint_vector, Some(LIB_MAX_STRING_DATA))
}

/// FR-022: the library rejects more than [`LIB_MAX_STRING_DATA`] data bytes.
pub fn dev_string_total(total: u64) -> bool {
    total > LIB_MAX_STRING_DATA
}

fn string_vector_with(
    b: &[u8],
    offsets: UintVectorDecoder,
    limit: Option<u64>,
) -> Option<(Vec<Vec<u8>>, usize)> {
    let (off, p) = offsets(b, 8)?;
    if off.is_empty() {
        return Some((Vec::new(), p));
    }
    let total = off[off.len() - 1];
    if limit.is_some() && dev_string_total(total) {
        return None;
    }
    if ((b.len() - p) as u64) < total {
        return None;
    }
    let data = &b[p..p + total as usize];
    let mut out = Vec::new();
    let mut start: u64 = 0;
    for i in 0..off.len() {
        let limit = off[i];
        if start <= limit && limit <= total {
            // Byte by byte: `to_vec` of a slice of symbolic length is a memory wall under the
            // checker (research R0).
            let mut entry = Vec::new();
            for j in start as usize..limit as usize {
                entry.push(data[j]);
            }
            out.push(entry);
        } else {
            out.push(Vec::new());
        }
        start = limit;
    }
    Some((out, p + total as usize))
}

/// Upstream `StringVectorEncoder::Encode` (`encoded_string_vector.cc:30–55`): the cumulative end
/// offsets as a 64-bit integer vector, then the strings.
pub fn enc_string_vector(v: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    put_string_vector(&mut out, v);
    out
}

/// Appends [`enc_string_vector`]`(v)`. The data's length depends on the strings, so the
/// caller chooses the buffer: the harnesses pass one of concrete capacity (research R0).
pub fn put_string_vector(out: &mut Vec<u8>, v: &[Vec<u8>]) {
    let mut offsets = Vec::with_capacity(v.len());
    let mut end: u64 = 0;
    for i in 0..v.len() {
        end += v[i].len() as u64;
        offsets.push(end);
    }
    put_uint_vector(out, &offsets, 8);
    for i in 0..v.len() {
        out.extend_from_slice(&v[i]);
    }
}

// ---- point vector -----------------------------------------------------------------------------

/// A decoder of 64-bit integer vectors: [`ref_uint_vector`] or [`lib_uint_vector`].
type UintVectorDecoder = fn(&[u8], u64) -> Option<(Vec<u64>, usize)>;

/// A point as the bit patterns of its three coordinates.
pub type PointBits = [u64; 3];

/// The answer of [`ref_point_vector`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PointVectorAnswer {
    /// Decoded points and bytes consumed.
    Points(Vec<PointBits>, usize),
    /// Rejected.
    Reject,
    /// The `CELL_IDS` format (tag 1). Its values come from floating-point projections of cell
    /// centres, which this description does not model (spec FR-002).
    Compact,
}

/// Upstream `EncodedS2PointVector::Init` (`encoded_s2point_vector.cc:121–258`) for every
/// first byte: tag 0 is `UNCOMPRESSED` (a varint `h`, then `h >> 3` points of 24 bytes,
/// rejected above `i32::MAX` points or if the bytes are not all there), tag 1 is `CELL_IDS`,
/// tags 2–7 are rejected.
pub fn ref_point_vector(b: &[u8]) -> PointVectorAnswer {
    point_vector_with(b, ref_varint, UPSTREAM_MAX_POINTS)
}

/// The library's `decode_s2point_vector` as documented: upstream's answer, with
/// [`LIB_MAX_POINTS`] instead of upstream's limit and [`lib_varint`]'s header.
pub fn lib_point_vector(b: &[u8]) -> PointVectorAnswer {
    point_vector_with(b, lib_varint, LIB_MAX_POINTS)
}

fn point_vector_with(
    b: &[u8],
    varint: fn(&[u8]) -> Option<(u64, usize)>,
    limit: u64,
) -> PointVectorAnswer {
    if b.is_empty() {
        return PointVectorAnswer::Reject;
    }
    match b[0] & 7 {
        0 => {}
        1 => return PointVectorAnswer::Compact,
        _ => return PointVectorAnswer::Reject,
    }
    let Some((h, p)) = varint(b) else {
        return PointVectorAnswer::Reject;
    };
    let count = h >> 3;
    if dev_count(count, limit) {
        return PointVectorAnswer::Reject;
    }
    if ((b.len() - p) as u64) < count * 24 {
        return PointVectorAnswer::Reject;
    }
    let mut out = Vec::new();
    for i in 0..count as usize {
        out.push(ref_point_bits(&b[p + 24 * i..]));
    }
    PointVectorAnswer::Points(out, p + 24 * count as usize)
}

/// Upstream `EncodeS2PointVector(…, FAST)` (`encoded_s2point_vector.cc`, `EncodeS2PointVectorFast`):
/// a varint `n << 3`, then each point as three little-endian doubles.
pub fn enc_point_vector_fast(v: &[PointBits]) -> Vec<u8> {
    let mut out = Vec::with_capacity(10 + 24 * v.len());
    put_varint(&mut out, (v.len() as u64) << 3);
    for i in 0..v.len() {
        for k in 0..3 {
            put_fixed(&mut out, v[i][k], 8);
        }
    }
    out
}

// ---- fixed-size codecs -------------------------------------------------------------------------

/// Upstream `S2CellId::Decode` (`s2cell_id.cc:261–265`): 8 little-endian bytes, any value.
pub fn ref_cell_id(b: &[u8]) -> Option<(u64, usize)> {
    ref_fixed(b, 8)
}

/// The library's `CellId` decoder as documented (FR-022): upstream's answer, minus
/// `dev_valid_id`.
pub fn lib_cell_id(b: &[u8]) -> Option<(u64, usize)> {
    ref_cell_id(b).filter(|&(id, _)| !dev_valid_id(id))
}

/// FR-022: the library rejects an identifier that is not a valid cell (001's model, `spec::valid`).
pub fn dev_valid_id(id: u64) -> bool {
    !spec::valid(id)
}

/// Upstream `S2CellId::Encode`.
pub fn enc_cell_id(id: u64) -> Vec<u8> {
    enc_fixed(id, 8)
}

/// Three little-endian doubles, as upstream's lossless encoders and the FAST point vector lay
/// out a point; no validation.
pub fn ref_point(b: &[u8]) -> Option<(PointBits, usize)> {
    if b.len() < 24 {
        None
    } else {
        Some((ref_point_bits(b), 24))
    }
}

fn ref_point_bits(b: &[u8]) -> PointBits {
    let mut out = [0u64; 3];
    for k in 0..3 {
        out[k] = ref_fixed(&b[8 * k..], 8).map_or(0, |(v, _)| v);
    }
    out
}

/// The canonical point bytes.
pub fn enc_point(p: PointBits) -> Vec<u8> {
    let mut out = Vec::with_capacity(24);
    for k in 0..3 {
        put_fixed(&mut out, p[k], 8);
    }
    out
}

/// A cap as the bit patterns of its centre's coordinates and its `length2`.
pub type CapBits = [u64; 4];

/// Upstream `S2Cap::Decode` in a release build (`s2cap.cc:340–353`): 32 bytes, the centre and
/// `S1ChordAngle::FromLength2(length2)`, which is `std::min(4.0, length2)`
/// (`s1chord_angle.h:365–367`). `std::min(4.0, NaN)` is 4.0. Not validated without
/// `--s2debug`.
pub fn ref_cap(b: &[u8]) -> Option<(CapBits, usize)> {
    let raw = ref_cap_raw(b)?;
    let l = f64::from_bits(raw[3]);
    let clamped = if l < 4.0 { l } else { 4.0 };
    Some(([raw[0], raw[1], raw[2], clamped.to_bits()], 32))
}

/// The library's `Cap` decoder as documented (FR-022): a NaN `length2` is rejected, then the
/// centre and `ChordAngle::from_length2(length2)` (values above 4 become 4, negative values
/// −1), rejected unless [`cap_valid`]. Two deliberate deviations from [`ref_cap`]: a NaN
/// `length2` is rejected where upstream decodes a full cap (D-004, the user's decision of
/// 2026-10-06), and a negative `length2` other than −1 decodes as −1 where upstream keeps it
/// (spec 007 FR-017, option B, the user's decision of 2026-10-16).
pub fn lib_cap(b: &[u8]) -> Option<(CapBits, usize)> {
    let raw = ref_cap_raw(b)?;
    let l = f64::from_bits(raw[3]);
    if l.is_nan() {
        return None;
    }
    let len2 = if l < 0.0 {
        -1.0
    } else if l > 4.0 {
        4.0
    } else {
        l
    };
    let cap = [raw[0], raw[1], raw[2], len2.to_bits()];
    if dev_valid_cap(cap) {
        None
    } else {
        Some((cap, 32))
    }
}

/// The four fields of a cap encoding as they are, before any clamp or validity check: the
/// centre's coordinates and `length2`. (`CapC`'s agreement uses them with research R18's
/// fallback.)
pub fn cap_fields(b: &[u8]) -> Option<CapBits> {
    ref_cap_raw(b)
}

fn ref_cap_raw(b: &[u8]) -> Option<CapBits> {
    if b.len() < 32 {
        return None;
    }
    let mut out = [0u64; 4];
    for k in 0..4 {
        out[k] = ref_fixed(&b[8 * k..], 8).map_or(0, |(v, _)| v);
    }
    Some(out)
}

/// Upstream `S2Cap::is_valid` (`s2cap.cc`): `IsUnitLength(center) && length2 <= 4`, with
/// `IsUnitLength` = `fabs(Norm2() - 1) <= 5 * DBL_EPSILON` (`s2pointutil.cc:32–39`). `Norm2` is
/// evaluated as the library's `Vector::dot` does on a target without FMA:
/// `x * x + y * y + z * z`, left to right.
pub fn cap_valid(c: CapBits) -> bool {
    let (x, y, z) = (
        f64::from_bits(c[0]),
        f64::from_bits(c[1]),
        f64::from_bits(c[2]),
    );
    let norm2 = x * x + y * y + z * z;
    (norm2 - 1.0).abs() <= 5.0 * f64::EPSILON && f64::from_bits(c[3]) <= 4.0
}

/// FR-022: the library rejects a cap that is not [`cap_valid`].
pub fn dev_valid_cap(c: CapBits) -> bool {
    !cap_valid(c)
}

/// The canonical cap bytes.
pub fn enc_cap(c: CapBits) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    for k in 0..4 {
        put_fixed(&mut out, c[k], 8);
    }
    out
}

/// A rectangle as the bit patterns of `lat.lo`, `lat.hi`, `lng.lo`, `lng.hi`.
pub type RectBits = [u64; 4];

/// Upstream `S2LatLngRect::Decode` (`s2latlng_rect.cc:372–392`): a version byte of at most 1,
/// four doubles, rejected unless [`rect_valid`].
pub fn ref_rect(b: &[u8]) -> Option<(RectBits, usize)> {
    if b.len() < 33 || b[0] > 1 {
        return None;
    }
    let mut r = [0u64; 4];
    for k in 0..4 {
        r[k] = ref_fixed(&b[1 + 8 * k..], 8).map_or(0, |(v, _)| v);
    }
    if rect_valid(r) { Some((r, 33)) } else { None }
}

/// Upstream `S2LatLngRect::is_valid` (`s2latlng_rect.h:417–423`) with `S1Interval::is_valid`
/// (`s1interval.h:252–256`) and the two `is_empty`s.
pub fn rect_valid(r: RectBits) -> bool {
    use core::f64::consts::{FRAC_PI_2, PI};
    let [lat_lo, lat_hi, lng_lo, lng_hi] = r.map(f64::from_bits);
    let lng_valid = lng_lo.abs() <= PI
        && lng_hi.abs() <= PI
        && !(lng_lo == -PI && lng_hi != PI)
        && !(lng_hi == -PI && lng_lo != PI);
    let lat_empty = lat_lo > lat_hi;
    let lng_empty = lng_lo == PI && lng_hi == -PI;
    lat_lo.abs() <= FRAC_PI_2 && lat_hi.abs() <= FRAC_PI_2 && lng_valid && lat_empty == lng_empty
}

/// Upstream `S2LatLngRect::Encode`: version 1, then the four doubles.
pub fn enc_rect(r: RectBits) -> Vec<u8> {
    let mut out = Vec::with_capacity(33);
    out.push(1u8);
    for k in 0..4 {
        put_fixed(&mut out, r[k], 8);
    }
    out
}

/// Upstream `S2Cell::Decode` (`s2cell.cc:326–332`) on a valid identifier. Upstream's answer on
/// an invalid one is undefined behaviour (candidate U-004), so this description has none:
/// it returns the library's documented rejection (FR-022).
pub fn lib_cell(b: &[u8]) -> Option<(u64, usize)> {
    lib_cell_id(b)
}

#[cfg(test)]
#[path = "wire_spec_tests.rs"]
mod wire_spec_tests;
