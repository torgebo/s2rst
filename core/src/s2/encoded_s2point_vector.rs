// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Compact encoding of `S2Point` vectors.
//!
//! Provides two formats:
//! - **UNCOMPRESSED** (`CodingHint::Fast`): varint header + raw `S2Points`.
//! - **`CELL_IDS`** (`CodingHint::Compact`): Represents points as `S2CellId`
//!   centers at a chosen level, with exceptions for points that aren't exact
//!   cell centers. Uses a block-based delta encoding scheme.
//!
//! Corresponds to C++ `encoded_s2point_vector.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "bit-level encoding uses intentional i32->u32 casts for shift amounts"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "bit-level point encoding — bounded by cell level"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u32 -> i32 for coordinate deltas — bounded by cell level"
)]
use std::io::{self, Read, Write};

use crate::r3::Vector;
use crate::s2::Point;
use crate::s2::coords::{self, Face, MAX_CELL_LEVEL};
use crate::s2::encoded_string_vector::{self, StringVectorBuilder};
use crate::s2::encoded_uint_vector;
use crate::s2::encoding::write_uvarint;

// ─── Constants ──────────────────────────────────────────────────────────

/// Number of low-order bits of the `size_format` varint that store the format.
const ENCODING_FORMAT_BITS: u32 = 3;

/// Number of values per encoded block.
const BLOCK_SHIFT: u32 = 4;
const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;

/// Sentinel indicating a point encoded as an exception (raw `S2Point`).
const EXCEPTION: u64 = u64::MAX;

/// Minimum fraction of points that must be encodable as `S2CellIds` for the
/// `CELL_IDS` format to be worthwhile.
const MIN_ENCODABLE_FRACTION: f64 = 0.05;

// ─── Format tag ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Format {
    Uncompressed = 0,
    CellIds = 1,
}

// ─── Coding hint ────────────────────────────────────────────────────────

/// Controls the trade-off between encoding speed and encoded size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CodingHint {
    /// Optimize for encoding/decoding speed (UNCOMPRESSED format).
    #[default]
    Fast,
    /// Optimize for smaller encoded size (`CELL_IDS` format when beneficial).
    Compact,
}

/// Reads 3 little-endian f64s from a byte slice starting at `off`.
fn read_point_from_bytes(data: &[u8], off: usize) -> Point {
    // The caller ensures data[off..off+24] is valid.
    let x = f64::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
        data[off + 4],
        data[off + 5],
        data[off + 6],
        data[off + 7],
    ]);
    let y = f64::from_le_bytes([
        data[off + 8],
        data[off + 9],
        data[off + 10],
        data[off + 11],
        data[off + 12],
        data[off + 13],
        data[off + 14],
        data[off + 15],
    ]);
    let z = f64::from_le_bytes([
        data[off + 16],
        data[off + 17],
        data[off + 18],
        data[off + 19],
        data[off + 20],
        data[off + 21],
        data[off + 22],
        data[off + 23],
    ]);
    Point(Vector { x, y, z })
}

// ─── Public API ─────────────────────────────────────────────────────────

/// Encodes a vector of `S2Points` using the given hint.
///
/// # Errors
///
/// Returns an I/O error if the write fails.
pub fn encode_s2point_vector(
    points: &[Point],
    hint: CodingHint,
    w: &mut dyn Write,
) -> io::Result<()> {
    match hint {
        CodingHint::Fast => encode_fast(points, w),
        CodingHint::Compact => encode_compact(points, w),
    }
}

/// Decodes a vector of `S2Points` encoded by [`encode_s2point_vector`].
///
/// # Errors
///
/// Returns an error if the data is malformed or the read fails.
pub fn decode_s2point_vector(r: &mut dyn Read) -> io::Result<Vec<Point>> {
    // Peek at the first byte to determine the format.
    let mut first_byte = [0u8; 1];
    r.read_exact(&mut first_byte)?;
    let format = first_byte[0] & ((1 << ENCODING_FORMAT_BITS) - 1);
    match format {
        0 => {
            // UNCOMPRESSED: the first byte is the start of a varint.
            // Reconstruct the varint by reading the rest of it.
            let size_format = reconstruct_uvarint(first_byte[0], r)?;
            decode_uncompressed(size_format, r)
        }
        1 => {
            // CELL_IDS: the first two bytes are a raw 2-byte header.
            let mut second_byte = [0u8; 1];
            r.read_exact(&mut second_byte)?;
            decode_cell_ids_raw(first_byte[0], second_byte[0], r)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown S2PointVector format {format}"),
        )),
    }
}

/// Reconstructs a varint given its first byte has already been read.
fn reconstruct_uvarint(first: u8, r: &mut dyn Read) -> io::Result<u64> {
    let mut result = u64::from(first & 0x7F);
    if first < 0x80 {
        return Ok(result);
    }
    let mut shift = 7u32;
    loop {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let b = buf[0];
        result |= u64::from(b & 0x7F) << shift;
        if b < 0x80 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "varint overflow",
            ));
        }
    }
}

// ─── UNCOMPRESSED format ────────────────────────────────────────────────

fn encode_fast(points: &[Point], w: &mut dyn Write) -> io::Result<()> {
    let size_format =
        ((points.len() as u64) << ENCODING_FORMAT_BITS) | (Format::Uncompressed as u64);
    write_uvarint(w, size_format)?;
    for p in points {
        w.write_all(&p.0.x.to_le_bytes())?;
        w.write_all(&p.0.y.to_le_bytes())?;
        w.write_all(&p.0.z.to_le_bytes())?;
    }
    Ok(())
}

/// Maximum number of points to decode (safety limit).
const MAX_DECODE_POINTS: usize = 50_000_000;

fn decode_uncompressed(size_format: u64, r: &mut dyn Read) -> io::Result<Vec<Point>> {
    let count = (size_format >> ENCODING_FORMAT_BITS) as usize;
    if count > MAX_DECODE_POINTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "too many points",
        ));
    }
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let mut buf = [0u8; 24];
        r.read_exact(&mut buf)?;
        points.push(read_point_from_bytes(&buf, 0));
    }
    Ok(points)
}

// ─── CELL_IDS format ────────────────────────────────────────────────────

/// A point in (face, si, ti) representation with its cell level.
struct CellPoint {
    level: i8, // -1 if not a cell center
    face: u8,
    si: u32,
    ti: u32,
}

fn encode_compact(points: &[Point], w: &mut dyn Write) -> io::Result<()> {
    // 1. Convert points and choose best level.
    let mut cell_points = Vec::with_capacity(points.len());
    let level = choose_best_level(points, &mut cell_points);
    if level < 0 {
        return encode_fast(points, w);
    }

    // 2. Convert to 64-bit values.
    let (values, have_exceptions) = convert_cells_to_values(&cell_points, level);

    // 3. Choose base.
    let (base, base_bits) = choose_base(&values, level, have_exceptions);

    // 4. Encode header (2 bytes).
    let num_blocks = (values.len() + BLOCK_SIZE - 1) >> BLOCK_SHIFT;
    let base_bytes = base_bits / 8;
    let last_block_count = values.len() - BLOCK_SIZE * (num_blocks - 1);
    debug_assert!((1..=BLOCK_SIZE).contains(&last_block_count));
    debug_assert!(base_bytes <= 7);
    debug_assert!(level <= 30);

    let byte0: u8 = (Format::CellIds as u8)
        | (u8::from(have_exceptions) << 3)
        | (((last_block_count - 1) as u8) << 4);
    let byte1: u8 = (base_bytes as u8) | ((level as u8) << 3);
    w.write_all(&[byte0, byte1])?;

    // 5. Encode base.
    let base_shift = base_shift(level, base_bits as i32);
    encoded_uint_vector::encode_uint_with_length(w, base >> base_shift, base_bytes)?;

    // 6. Encode blocks.
    let mut blocks = StringVectorBuilder::new();
    for i in (0..values.len()).step_by(BLOCK_SIZE) {
        let block_size = BLOCK_SIZE.min(values.len() - i);
        let block_values = &values[i..i + block_size];
        let code = get_block_code(block_values, base, have_exceptions);

        let mut block = Vec::new();

        // Block header.
        let offset_bytes = code.offset_bits / 8;
        let delta_nibbles = code.delta_bits / 4;
        let overlap_nibbles = code.overlap_bits / 4;
        debug_assert!(offset_bytes <= 8);
        debug_assert!((1..=16).contains(&delta_nibbles));
        debug_assert!(overlap_nibbles <= 1);
        let header: u8 = ((offset_bytes - overlap_nibbles) as u8)
            | ((overlap_nibbles as u8) << 3)
            | (((delta_nibbles - 1) as u8) << 4);
        block.push(header);

        // Determine offset.
        let mut offset = u64::MAX;
        let mut num_exceptions = 0usize;
        for &v in block_values {
            if v == EXCEPTION {
                num_exceptions += 1;
            } else {
                debug_assert!(v >= base);
                offset = offset.min(v - base);
            }
        }
        if num_exceptions == block_size {
            offset = 0;
        }

        // Encode offset.
        let offset_shift = code.delta_bits - code.overlap_bits;
        let offset = offset & !bit_mask(offset_shift);
        if offset > 0 {
            encoded_uint_vector::encode_uint_with_length(
                &mut block,
                offset >> offset_shift,
                offset_bytes,
            )?;
        }

        // Encode deltas and collect exceptions.
        let delta_bytes = delta_nibbles.div_ceil(2);
        let mut exceptions: Vec<&Point> = Vec::new();
        for j in 0..block_size {
            let delta;
            if block_values[j] == EXCEPTION {
                delta = exceptions.len() as u64;
                exceptions.push(&points[i + j]);
            } else {
                debug_assert!(block_values[j] >= offset + base);
                let mut d = block_values[j] - (offset + base);
                if have_exceptions {
                    d += BLOCK_SIZE as u64;
                }
                delta = d;
            }
            debug_assert!(delta <= bit_mask(code.delta_bits));

            if (delta_nibbles & 1) != 0 && (j & 1) != 0 {
                // Combine with high-order 4 bits of previous delta.
                // Safe: j is odd and j>=1, so block has at least one delta byte.
                let Some(last) = block.pop() else {
                    unreachable!("block must have previous delta byte when j >= 1");
                };
                let combined = (delta << 4) | (u64::from(last) & 0xf);
                encoded_uint_vector::encode_uint_with_length(&mut block, combined, delta_bytes)?;
            } else {
                encoded_uint_vector::encode_uint_with_length(&mut block, delta, delta_bytes)?;
            }
        }

        // Append exceptions.
        for &p in &exceptions {
            block.extend_from_slice(&p.0.x.to_le_bytes());
            block.extend_from_slice(&p.0.y.to_le_bytes());
            block.extend_from_slice(&p.0.z.to_le_bytes());
        }

        blocks.add(block);
    }

    blocks.encode(w)?;
    Ok(())
}

fn decode_cell_ids_raw(byte0: u8, byte1: u8, r: &mut dyn Read) -> io::Result<Vec<Point>> {
    debug_assert_eq!(byte0 & 7, Format::CellIds as u8);
    let have_exceptions = (byte0 & 8) != 0;
    let last_block_count = ((byte0 >> 4) + 1) as usize;
    let base_bytes = (byte1 & 7) as usize;
    let level = i32::from(byte1 >> 3);
    if level > i32::from(MAX_CELL_LEVEL) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid level {level}"),
        ));
    }

    // Decode base.
    let raw_base = encoded_uint_vector::decode_uint_with_length(r, base_bytes)?;
    let base = raw_base << base_shift(level, (base_bytes * 8) as i32);

    // Decode blocks.
    let block_data = encoded_string_vector::decode_string_vector(r)?;
    let num_blocks = block_data.len();
    if num_blocks == 0 {
        return Ok(Vec::new());
    }
    let total_points = BLOCK_SIZE * (num_blocks - 1) + last_block_count;

    let mut points = Vec::with_capacity(total_points);

    for (bi, block) in block_data.iter().enumerate() {
        if block.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty block"));
        }
        let block_size = if bi == num_blocks - 1 {
            last_block_count
        } else {
            BLOCK_SIZE
        };

        let header = block[0];
        let overlap_nibbles = ((header >> 3) & 1) as usize;
        let offset_bytes = ((header & 7) as usize) + overlap_nibbles;
        let delta_nibbles = ((header >> 4) + 1) as usize;

        let mut ptr = 1usize;

        // Decode offset.
        let offset = if offset_bytes > 0 {
            let offset_shift = (delta_nibbles - overlap_nibbles) * 4;
            if offset_shift >= 64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "offset shift too large",
                ));
            }
            if ptr + offset_bytes > block.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "offset out of bounds",
                ));
            }
            let raw = encoded_uint_vector::get_uint_with_length(
                &block[ptr..ptr + offset_bytes],
                offset_bytes,
            );
            ptr += offset_bytes;
            raw << offset_shift
        } else {
            0u64
        };

        // Decode each delta.
        let delta_bytes = delta_nibbles.div_ceil(2);
        for j in 0..block_size {
            let delta_nibble_offset = j * delta_nibbles;
            let delta_byte_offset = delta_nibble_offset / 2;
            let delta_ptr = ptr + delta_byte_offset;
            if delta_ptr + delta_bytes > block.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "delta out of bounds",
                ));
            }
            let raw = encoded_uint_vector::get_uint_with_length(
                &block[delta_ptr..delta_ptr + delta_bytes],
                delta_bytes,
            );
            let mut delta = raw >> ((delta_nibble_offset & 1) * 4);
            delta &= bit_mask(delta_nibbles * 4);

            if have_exceptions && delta < BLOCK_SIZE as u64 {
                // Exception: raw S2Point stored at end of block.
                let exceptions_start = ptr + (block_size * delta_nibbles).div_ceil(2);
                let exc_offset = exceptions_start + (delta as usize) * 24;
                if exc_offset + 24 > block.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "exception out of bounds",
                    ));
                }
                points.push(read_point_from_bytes(block, exc_offset));
            } else {
                if have_exceptions {
                    delta -= BLOCK_SIZE as u64;
                }
                // Upstream C++ computes this in wrapping unsigned arithmetic.
                // Valid encodings never overflow; malformed input can, so wrap
                // explicitly — this both matches the reference output and keeps
                // the decoder panic-free on untrusted bytes (found by fuzzing).
                let value = base.wrapping_add(offset).wrapping_add(delta);
                points.push(value_to_point(value, level));
            }
        }
    }

    Ok(points)
}

// ─── Bit interleaving ───────────────────────────────────────────────────

/// Interleaves bit pairs of two u32 values into a u64.
fn interleave_bit_pairs(val0: u32, val1: u32) -> u64 {
    let mut v0 = u64::from(val0);
    let mut v1 = u64::from(val1);
    v0 = (v0 | (v0 << 16)) & 0x0000ffff0000ffff;
    v1 = (v1 | (v1 << 16)) & 0x0000ffff0000ffff;
    v0 = (v0 | (v0 << 8)) & 0x00ff00ff00ff00ff;
    v1 = (v1 | (v1 << 8)) & 0x00ff00ff00ff00ff;
    v0 = (v0 | (v0 << 4)) & 0x0f0f0f0f0f0f0f0f;
    v1 = (v1 | (v1 << 4)) & 0x0f0f0f0f0f0f0f0f;
    v0 = (v0 | (v0 << 2)) & 0x3333333333333333;
    v1 = (v1 | (v1 << 2)) & 0x3333333333333333;
    v0 | (v1 << 2)
}

/// Deinterleaves a u64 into two u32 bit-pair values.
fn deinterleave_bit_pairs(code: u64) -> (u32, u32) {
    let mut v0 = code;
    let mut v1 = code >> 2;
    v0 &= 0x3333333333333333;
    v0 |= v0 >> 2;
    v1 &= 0x3333333333333333;
    v1 |= v1 >> 2;
    v0 &= 0x0f0f0f0f0f0f0f0f;
    v0 |= v0 >> 4;
    v1 &= 0x0f0f0f0f0f0f0f0f;
    v1 |= v1 >> 4;
    v0 &= 0x00ff00ff00ff00ff;
    v0 |= v0 >> 8;
    v1 &= 0x00ff00ff00ff00ff;
    v1 |= v1 >> 8;
    v0 &= 0x0000ffff0000ffff;
    v0 |= v0 >> 16;
    v1 &= 0x0000ffff0000ffff;
    v1 |= v1 >> 16;
    (v0 as u32, v1 as u32)
}

// ─── Helpers ────────────────────────────────────────────────────────────

fn bit_mask(n: usize) -> u64 {
    if n == 0 {
        0
    } else if n >= 64 {
        u64::MAX
    } else {
        (1u64 << n) - 1
    }
}

fn max_bits_for_level(level: i32) -> usize {
    (2 * level + 3) as usize
}

fn base_shift(level: i32, base_bits: i32) -> usize {
    0i32.max(max_bits_for_level(level) as i32 - base_bits) as usize
}

/// Converts a 64-bit encoded value back to an `S2Point`.
fn value_to_point(value: u64, level: i32) -> Point {
    let (sj, tj) = deinterleave_bit_pairs(value);
    let shift = i32::from(MAX_CELL_LEVEL) - level;
    let si = (((sj << 1) | 1) << shift) & 0x7fffffff;
    let ti = (((tj << 1) | 1) << shift) & 0x7fffffff;
    let face_val = (((sj << shift) >> 30) | (((tj << (shift as u32 + 1)) >> 29) & 4)) as u8;
    // Clamp to valid face range [0, 5] to handle malformed data.
    let face = Face::from_u8(face_val.min(5));
    let u = coords::st_to_uv(coords::si_ti_to_st(si));
    let v = coords::st_to_uv(coords::si_ti_to_st(ti));
    Point(coords::face_uv_to_xyz(face, u, v).normalize())
}

/// Chooses the `S2CellId` level at which the most points can be represented.
/// Returns -1 if not enough points are encodable.
fn choose_best_level(points: &[Point], cell_points: &mut Vec<CellPoint>) -> i32 {
    cell_points.clear();
    cell_points.reserve(points.len());

    let mut level_counts = [0u32; MAX_CELL_LEVEL as usize + 1];
    for p in points {
        let (face, si, ti, opt_level) = coords::xyz_to_face_si_ti(&p.0);
        let level = match opt_level {
            Some(l) => {
                level_counts[l.as_usize()] += 1;
                l.as_u8() as i8
            }
            None => -1,
        };
        cell_points.push(CellPoint {
            level,
            face: face.as_u8(),
            si,
            ti,
        });
    }

    // Pick the first (lowest) level with the maximum count (matching C++).
    let mut best_level = 0;
    for level in 1..=MAX_CELL_LEVEL as usize {
        if level_counts[level] > level_counts[best_level] {
            best_level = level;
        }
    }

    if f64::from(level_counts[best_level]) <= MIN_ENCODABLE_FRACTION * points.len() as f64 {
        return -1;
    }
    best_level as i32
}

/// Converts cell points to 64-bit interleaved values at the given level.
fn convert_cells_to_values(cell_points: &[CellPoint], level: i32) -> (Vec<u64>, bool) {
    let mut values = Vec::with_capacity(cell_points.len());
    let mut have_exceptions = false;
    let shift = i32::from(MAX_CELL_LEVEL) - level;
    for cp in cell_points {
        if i32::from(cp.level) == level {
            let sj = (((u32::from(cp.face) & 3) << 30) | (cp.si >> 1)) >> shift as u32;
            let tj = (((u32::from(cp.face) & 4) << 29) | cp.ti) >> (shift as u32 + 1);
            let v = interleave_bit_pairs(sj, tj);
            debug_assert!(v <= bit_mask(max_bits_for_level(level)));
            values.push(v);
        } else {
            values.push(EXCEPTION);
            have_exceptions = true;
        }
    }
    (values, have_exceptions)
}

/// Chooses the global base value (shared bit prefix).
fn choose_base(values: &[u64], level: i32, have_exceptions: bool) -> (u64, usize) {
    let mut v_min = EXCEPTION;
    let mut v_max = 0u64;
    for &v in values {
        if v != EXCEPTION {
            v_min = v_min.min(v);
            v_max = v_max.max(v);
        }
    }
    if v_min == EXCEPTION {
        return (0, 0);
    }

    let min_delta_bits = if have_exceptions || values.len() == 1 {
        8
    } else {
        4
    };
    let xor_width = if v_min ^ v_max == 0 {
        0
    } else {
        64 - (v_min ^ v_max).leading_zeros() as i32
    };
    let excluded_bits = xor_width
        .max(min_delta_bits)
        .max(base_shift(level, 56) as i32);
    let base = v_min & !bit_mask(excluded_bits as usize);

    let base_bits = if base == 0 {
        0
    } else {
        let low_bit = base.trailing_zeros() as i32;
        ((max_bits_for_level(level) as i32 - low_bit + 7) & !7) as usize
    };

    // Round base to use all available base_bits.
    let base = v_min & !bit_mask(base_shift(level, base_bits as i32));
    (base, base_bits)
}

struct BlockCode {
    delta_bits: usize,
    offset_bits: usize,
    overlap_bits: usize,
}

fn can_encode(
    d_min: u64,
    d_max: u64,
    delta_bits: usize,
    overlap_bits: usize,
    have_exceptions: bool,
) -> bool {
    let d_min = d_min & !bit_mask(delta_bits - overlap_bits);
    let mut max_delta = bit_mask(delta_bits);
    if have_exceptions {
        if max_delta < BLOCK_SIZE as u64 {
            return false;
        }
        max_delta -= BLOCK_SIZE as u64;
    }
    d_min > u64::MAX - max_delta || d_min + max_delta >= d_max
}

fn get_block_code(values: &[u64], base: u64, have_exceptions: bool) -> BlockCode {
    let mut b_min = EXCEPTION;
    let mut b_max = 0u64;
    for &v in values {
        if v != EXCEPTION {
            b_min = b_min.min(v);
            b_max = b_max.max(v);
        }
    }
    if b_min == EXCEPTION {
        return BlockCode {
            delta_bits: 4,
            offset_bits: 0,
            overlap_bits: 0,
        };
    }
    b_min -= base;
    b_max -= base;

    let range = b_max - b_min;
    let range_bits = if range == 0 {
        1
    } else {
        64 - range.leading_zeros() as usize
    };
    let mut delta_bits = ((range_bits.max(1) - 1 + 3) & !3).max(4);
    let mut overlap_bits = 0;

    if !can_encode(b_min, b_max, delta_bits, 0, have_exceptions) {
        if can_encode(b_min, b_max, delta_bits, 4, have_exceptions) {
            overlap_bits = 4;
        } else {
            debug_assert!(delta_bits <= 60);
            delta_bits += 4;
            if !can_encode(b_min, b_max, delta_bits, 0, have_exceptions) {
                debug_assert!(can_encode(b_min, b_max, delta_bits, 4, have_exceptions));
                overlap_bits = 4;
            }
        }
    }

    // Special case: block size 1 with no exceptions → use 8-bit delta.
    if values.len() == 1 && !have_exceptions {
        debug_assert!(delta_bits == 4 && overlap_bits == 0);
        delta_bits = 8;
    }

    let max_delta = bit_mask(delta_bits)
        - if have_exceptions {
            BLOCK_SIZE as u64
        } else {
            0
        };
    let mut offset_bits = 0;
    if b_max > max_delta {
        let offset_shift = delta_bits - overlap_bits;
        let mask = bit_mask(offset_shift);
        let min_offset = (b_max - max_delta + mask) & !mask;
        debug_assert!(min_offset != 0);
        let raw_bits = 64 - min_offset.leading_zeros() as usize;
        offset_bits = ((raw_bits.saturating_sub(offset_shift) + 7) & !7).max(8);
        if offset_bits == 64 {
            overlap_bits = 4;
        }
    }

    BlockCode {
        delta_bits,
        offset_bits,
        overlap_bits,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::CellId;

    fn roundtrip(points: &[Point], hint: CodingHint, expected_bytes: Option<usize>) -> usize {
        let mut buf = Vec::new();
        encode_s2point_vector(points, hint, &mut buf).unwrap();
        if let Some(expected) = expected_bytes {
            assert_eq!(
                buf.len(),
                expected,
                "encoded size mismatch for {hint:?} with {} points",
                points.len()
            );
        }
        let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.len(), points.len(), "point count mismatch");
        for (i, (got, want)) in decoded.iter().zip(points).enumerate() {
            assert!(
                (got.0.x - want.0.x).abs() < 1e-15
                    && (got.0.y - want.0.y).abs() < 1e-15
                    && (got.0.z - want.0.z).abs() < 1e-15,
                "point {i} mismatch: got {got:?}, want {want:?}"
            );
        }
        buf.len()
    }

    /// Converts an encoded 64-bit value back to an `S2Point` (for test setup).
    fn encoded_value_to_point(value: u64, level: i32) -> Point {
        value_to_point(value, level)
    }

    #[test]
    fn test_empty() {
        roundtrip(&[], CodingHint::Fast, Some(1));
        roundtrip(&[], CodingHint::Compact, Some(1));
    }

    #[test]
    fn test_one_point_fast() {
        roundtrip(
            &[Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            })],
            CodingHint::Fast,
            Some(25),
        );
    }

    #[test]
    fn test_one_point_compact() {
        // header(2) + block_count(1) + block_offsets(1) + block_header(1) + delta(1) = 6
        roundtrip(
            &[Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            })],
            CodingHint::Compact,
            Some(6),
        );
    }

    #[test]
    fn test_cell_id_with_exception() {
        // One cell center + one non-cell-center.
        // header(2) + block_count(1) + block_offsets(1) + block_header(1)
        // + two deltas(2) + exception(24) = 31
        let cell_point = CellId::from_debug_string("1/23").unwrap().to_point();
        let exc = Point(Vector {
            x: 0.1,
            y: 0.2,
            z: 0.3,
        })
        .normalize();
        roundtrip(&[cell_point, exc], CodingHint::Compact, Some(31));
    }

    #[test]
    fn test_first_at_all_levels() {
        for level in 0..=MAX_CELL_LEVEL {
            let p = CellId::begin(level).to_point();
            roundtrip(&[p], CodingHint::Compact, Some(6));
        }
    }

    #[test]
    fn test_last_at_all_levels() {
        for level in 0..=MAX_CELL_LEVEL {
            let p = CellId::end(level).prev().to_point();
            let expected_size = 6 + (level as usize) / 4;
            roundtrip(&[p], CodingHint::Compact, Some(expected_size));
        }
    }

    #[test]
    fn test_last_two_points_at_all_levels() {
        for level in 0..=MAX_CELL_LEVEL {
            let id = CellId::end(level).prev();
            let expected_size = 6 + (level as usize + 2) / 4;
            roundtrip(
                &[id.to_point(), id.prev().to_point()],
                CodingHint::Compact,
                Some(expected_size),
            );
        }
    }

    #[test]
    fn test_many_duplicate_points_at_all_levels() {
        for level in 0..=MAX_CELL_LEVEL {
            let id = CellId::end(level).prev();
            let mut expected_size = 23 + (level as usize + 2) / 4;
            if level == 30 {
                expected_size += 1;
            }
            let points: Vec<Point> = vec![id.to_point(); 32];
            roundtrip(&points, CodingHint::Compact, Some(expected_size));
        }
    }

    #[test]
    fn test_no_overlap_or_extra_delta_bits_needed() {
        let level = 3;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0x72, level));
        points.push(encoded_value_to_point(0x74, level));
        points.push(encoded_value_to_point(0x75, level));
        points.push(encoded_value_to_point(0x7e, level));
        roundtrip(&points, CodingHint::Compact, Some(10 + BLOCK_SIZE / 2));
    }

    #[test]
    fn test_overlap_needed() {
        let level = 3;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0x78, level));
        points.push(encoded_value_to_point(0x7a, level));
        points.push(encoded_value_to_point(0x7c, level));
        points.push(encoded_value_to_point(0x84, level));
        roundtrip(&points, CodingHint::Compact, Some(10 + BLOCK_SIZE / 2));
    }

    #[test]
    fn test_extra_delta_bits_needed() {
        let level = 3;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0x08, level));
        points.push(encoded_value_to_point(0x4e, level));
        points.push(encoded_value_to_point(0x82, level));
        points.push(encoded_value_to_point(0x104, level));
        roundtrip(&points, CodingHint::Compact, Some(13 + BLOCK_SIZE / 2));
    }

    #[test]
    fn test_extra_delta_bits_and_overlap_needed() {
        let level = 5;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0xf08, level));
        points.push(encoded_value_to_point(0xf4e, level));
        points.push(encoded_value_to_point(0xf82, level));
        points.push(encoded_value_to_point(0x1004, level));
        roundtrip(&points, CodingHint::Compact, Some(14 + BLOCK_SIZE / 2));
    }

    #[test]
    fn test_sixty_four_bit_offset() {
        let level = MAX_CELL_LEVEL;
        let mut points: Vec<Point> = vec![CellId::begin(level).to_point(); BLOCK_SIZE];
        points.push(CellId::end(level).prev().to_point());
        points.push(CellId::end(level).prev().prev().to_point());
        roundtrip(&points, CodingHint::Compact, Some(16 + BLOCK_SIZE / 2));
    }

    #[test]
    fn test_all_exceptions_block() {
        let mut points: Vec<Point> =
            vec![encoded_value_to_point(0, i32::from(MAX_CELL_LEVEL)); BLOCK_SIZE];
        points.push(
            Point(Vector {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            })
            .normalize(),
        );
        points.push(
            Point(Vector {
                x: 0.3,
                y: 0.2,
                z: 0.1,
            })
            .normalize(),
        );
        roundtrip(&points, CodingHint::Compact, Some(72));
        roundtrip(&points, CodingHint::Fast, Some(434));
    }

    #[test]
    fn test_roundtrip_fast() {
        let level = 3;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0x78, level));
        points.push(encoded_value_to_point(0x7a, level));
        points.push(encoded_value_to_point(0x7c, level));
        points.push(encoded_value_to_point(0x84, level));

        let mut buf = Vec::new();
        encode_s2point_vector(&points, CodingHint::Fast, &mut buf).unwrap();
        let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, points);

        // Re-encode and verify identical.
        let mut buf2 = Vec::new();
        encode_s2point_vector(&decoded, CodingHint::Fast, &mut buf2).unwrap();
        assert_eq!(buf, buf2);
    }

    #[test]
    fn test_roundtrip_compact() {
        let level = 3;
        let mut points: Vec<Point> = vec![encoded_value_to_point(0, level); BLOCK_SIZE];
        points.push(encoded_value_to_point(0x78, level));
        points.push(encoded_value_to_point(0x7a, level));
        points.push(encoded_value_to_point(0x7c, level));
        points.push(encoded_value_to_point(0x84, level));

        let mut buf = Vec::new();
        encode_s2point_vector(&points, CodingHint::Compact, &mut buf).unwrap();
        let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, points);
    }

    #[test]
    fn test_one_point_with_exceptions_no_overlap() {
        let a = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let mut points = vec![
            Point(Vector {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
            .normalize(),
        ];
        for _ in 0..15 {
            points.push(a);
        }
        points.push(a); // second block
        roundtrip(&points, CodingHint::Compact, Some(48));
    }

    #[test]
    fn test_interleave_deinterleave() {
        for &(a, b) in &[
            (0u32, 0u32),
            (1, 0),
            (0, 1),
            (0xFFFF, 0xFFFF),
            (0x12345678, 0x9ABCDEF0),
        ] {
            let code = interleave_bit_pairs(a, b);
            let (a2, b2) = deinterleave_bit_pairs(code);
            assert_eq!(a, a2, "a mismatch for ({a:#x}, {b:#x})");
            assert_eq!(b, b2, "b mismatch for ({a:#x}, {b:#x})");
        }
    }

    #[test]
    fn test_points_at_multiple_levels() {
        // Two points at level 5 (face 1) should be encoded; others as exceptions.
        let points = vec![
            CellId::from_debug_string("2/11001310230102")
                .unwrap()
                .to_point(),
            CellId::from_debug_string("1/23322").unwrap().to_point(),
            CellId::from_debug_string("3/3").unwrap().to_point(),
            CellId::from_debug_string("1/23323").unwrap().to_point(),
            CellId::from_debug_string("2/12101023022012")
                .unwrap()
                .to_point(),
        ];
        roundtrip(&points, CodingHint::Compact, Some(83));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_coding_hint_roundtrip() {
        for h in [CodingHint::Fast, CodingHint::Compact] {
            let json = serde_json::to_string(&h).unwrap();
            let back: CodingHint = serde_json::from_str(&json).unwrap();
            assert_eq!(h, back);
        }
    }
}
