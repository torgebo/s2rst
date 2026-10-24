// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Variable-width encoding of unsigned integer vectors.
//!
//! Encodes a vector of unsigned integers using a fixed number of bytes per
//! value, where the byte count is the minimum needed for the largest value.
//!
//! Corresponds to C++ `encoded_uint_vector.h`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "u64 -> usize for decoded vector lengths"
)]
use std::io::{self, Read, Write};

use crate::s2::encoding::{read_uvarint, write_uvarint};

// ─── Encoding ───────────────────────────────────────────────────────────

/// Writes `value` in little-endian format using exactly `length` bytes: its low `length`
/// bytes, as upstream `EncodeUintWithLength`.
///
/// # Errors
///
/// Returns an I/O error if the write fails.
///
/// # Panics
///
/// Panics if `length > 8`. Upstream states the same precondition as a debug-only check
/// (`ABSL_DCHECK`, `encoded_uint_vector.h:145`).
pub fn encode_uint_with_length(w: &mut dyn Write, value: u64, length: usize) -> io::Result<()> {
    let bytes = value.to_le_bytes();
    w.write_all(&bytes[..length])
}

/// Reads a little-endian unsigned integer of `length` bytes from the start of `data`.
///
/// # Panics
///
/// Panics if `length > 8` or `data.len() < length`. Upstream `GetUintWithLength` checks the
/// length only in debug builds (`ABSL_DCHECK`, `encoded_uint_vector.h:159`) and cannot check
/// the input's size, since it takes a pointer.
pub fn get_uint_with_length(data: &[u8], length: usize) -> u64 {
    let mut buf = [0u8; 8];
    buf[..length].copy_from_slice(&data[..length]);
    u64::from_le_bytes(buf)
}

/// Reads and consumes a little-endian unsigned integer of `length` bytes. A `length` of 0
/// reads nothing and returns 0.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] if fewer than `length` bytes are left, and any
/// other error of the reader as it is.
///
/// # Panics
///
/// Panics if `length > 8`, the precondition of [`encode_uint_with_length`].
pub fn decode_uint_with_length(r: &mut dyn Read, length: usize) -> io::Result<u64> {
    if length == 0 {
        return Ok(0);
    }
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf[..length])?;
    Ok(u64::from_le_bytes(buf))
}

/// Encodes a vector of `u32` values in the `EncodedUintVector` format.
///
/// The encoding is:
///   `varint64`: `(v.len() * sizeof(T)) | (len - 1)`
///   array of `v.len()` elements, `len` bytes each
///
/// where `len` is the minimum bytes needed for the max value (at least 1).
///
/// # Errors
///
/// Returns an I/O error if the write fails.
pub fn encode_uint_vector_u32(v: &[u32], w: &mut dyn Write) -> io::Result<()> {
    let one_bits: u32 = v.iter().fold(1u32, |acc, &x| acc | x);
    let len = byte_length_u32(one_bits);
    let size_len = (v.len() as u64 * 4) | (len as u64 - 1);
    write_uvarint(w, size_len)?;
    for &x in v {
        encode_uint_with_length(w, u64::from(x), len)?;
    }
    Ok(())
}

/// Encodes a vector of `u64` values in the `EncodedUintVector` format.
///
/// # Errors
///
/// Returns an I/O error if the write fails.
pub fn encode_uint_vector_u64(v: &[u64], w: &mut dyn Write) -> io::Result<()> {
    let one_bits: u64 = v.iter().fold(1u64, |acc, &x| acc | x);
    let len = byte_length_u64(one_bits);
    let size_len = (v.len() as u64 * 8) | (len as u64 - 1);
    write_uvarint(w, size_len)?;
    for &x in v {
        encode_uint_with_length(w, x, len)?;
    }
    Ok(())
}

/// Maximum number of elements to decode in a uint vector (safety limit).
const MAX_DECODE_COUNT: usize = 50_000_000;

/// Decodes a vector of `u32` values from the `EncodedUintVector` format.
///
/// # Errors
///
/// - [`io::ErrorKind::UnexpectedEof`]: the input ends inside the header or an element.
/// - [`io::ErrorKind::InvalidData`]: the header varint's tenth byte is above 1 (more than 64
///   bits; upstream rejects it too), or the header declares more than 50,000,000 elements.
///   The count limit is a deliberate deviation: upstream has none, and rejects a count only
///   when its bytes are missing. Here a short input declaring a huge count is rejected
///   before anything is reserved for it.
///
/// Any other error of the reader is returned as it is. Nothing else is validated: every
/// width of 1–4 bytes and every value is accepted, as upstream.
pub fn decode_uint_vector_u32(r: &mut dyn Read) -> io::Result<Vec<u32>> {
    let size_len = read_uvarint(r)?;
    let count = (size_len / 4) as usize; // sizeof(u32) == 4
    if count > MAX_DECODE_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "uint vector too large",
        ));
    }
    let len = ((size_len & 3) + 1) as usize;
    // Grow on demand: never reserve from a decoded count (spec 003 FR-019; a short input
    // declaring a huge count must not reserve memory for it). Not even a capped reservation:
    // any reservation sized from decoded data keeps the decoder out of the Kani proofs' reach
    // (research R7). The regrowth costs within 3 % in the benchmarks (decoding/EVALUATION.md).
    let mut result = Vec::new();
    for _ in 0..count {
        result.push(decode_uint_with_length(r, len)? as u32);
    }
    Ok(result)
}

/// Decodes a vector of `u64` values from the `EncodedUintVector` format.
///
/// # Errors
///
/// As [`decode_uint_vector_u32`]: [`io::ErrorKind::UnexpectedEof`] for a truncated input,
/// and [`io::ErrorKind::InvalidData`] for a header varint whose tenth byte is above 1 or a
/// count above 50,000,000 elements (the deliberate deviation described there). Every width
/// of 1–8 bytes and every value is accepted, as upstream.
pub fn decode_uint_vector_u64(r: &mut dyn Read) -> io::Result<Vec<u64>> {
    let size_len = read_uvarint(r)?;
    let count = (size_len / 8) as usize; // sizeof(u64) == 8
    if count > MAX_DECODE_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "uint vector too large",
        ));
    }
    let len = ((size_len & 7) + 1) as usize;
    // Grow on demand: never reserve from a decoded count (spec 003 FR-019; a short input
    // declaring a huge count must not reserve memory for it). Not even a capped reservation:
    // any reservation sized from decoded data keeps the decoder out of the Kani proofs' reach
    // (research R7). The regrowth costs within 3 % in the benchmarks (decoding/EVALUATION.md).
    let mut result = Vec::new();
    for _ in 0..count {
        result.push(decode_uint_with_length(r, len)?);
    }
    Ok(result)
}

// ─── Helpers ────────────────────────────────────────────────────────────

/// Minimum bytes to represent a u32 (at least 1).
fn byte_length_u32(v: u32) -> usize {
    if v == 0 {
        1
    } else {
        (32 - v.leading_zeros() as usize).div_ceil(8)
    }
}

/// Minimum bytes to represent a u64 (at least 1).
fn byte_length_u64(v: u64) -> usize {
    if v == 0 {
        1
    } else {
        (64 - v.leading_zeros() as usize).div_ceil(8)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_u64(input: &[u64], expected_bytes: usize) {
        let mut buf = Vec::new();
        encode_uint_vector_u64(input, &mut buf).unwrap();
        assert_eq!(buf.len(), expected_bytes, "encoded size for {input:?}");
        let decoded = decode_uint_vector_u64(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, input);
    }

    fn roundtrip_u32(input: &[u32], expected_bytes: usize) {
        let mut buf = Vec::new();
        encode_uint_vector_u32(input, &mut buf).unwrap();
        assert_eq!(buf.len(), expected_bytes, "encoded size for {input:?}");
        let decoded = decode_uint_vector_u32(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, input);
    }

    #[test]
    fn test_empty() {
        // varint64 of 0 = 1 byte
        roundtrip_u32(&[], 1);
    }

    #[test]
    fn test_zero() {
        // varint64((1 * 8) | 0) = varint64(8) = 1 byte, plus 1 byte value
        roundtrip_u64(&[0], 2);
    }

    #[test]
    fn test_repeated_zeros_u16_equiv() {
        // C++ uses u16, Rust uses u32. For u32: varint64((3 * 4) | 0) = 1 byte, + 3 bytes
        roundtrip_u32(&[0, 0, 0], 4);
    }

    #[test]
    fn test_max_int() {
        // varint64((1 * 8) | 7) = varint64(15) = 1 byte, + 8 bytes
        roundtrip_u64(&[u64::MAX], 9);
    }

    #[test]
    fn test_one_byte() {
        // All values fit in 1 byte. varint64((4*8)|0) = 1 byte, + 4 bytes
        roundtrip_u64(&[0, 255, 1, 254], 5);
    }

    #[test]
    fn test_two_bytes() {
        // Max value is 256 → needs 2 bytes. varint64((4*8)|1) = 1 byte, + 8 bytes
        roundtrip_u64(&[0, 255, 256, 254], 9);
    }

    #[test]
    fn test_three_bytes() {
        // Max value is 0xffffff → 3 bytes. varint64((4*8)|2) = 1 byte, + 12 bytes
        roundtrip_u64(&[0xffffff, 0x0102, 0, 0x050403], 13);
    }

    #[test]
    fn test_eight_bytes() {
        // Max is u64::MAX → 8 bytes. varint64((3*8)|7) = 1 byte, + 24 bytes
        roundtrip_u64(&[u64::MAX, 0, 0x0102030405060708], 25);
    }

    #[test]
    fn test_roundtrip_encoding() {
        let values: Vec<u64> = vec![10, 20, 30, 40];
        let mut buf = Vec::new();
        encode_uint_vector_u64(&values, &mut buf).unwrap();
        let decoded = decode_uint_vector_u64(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, values);

        // Re-encode and verify identical bytes.
        let mut buf2 = Vec::new();
        encode_uint_vector_u64(&decoded, &mut buf2).unwrap();
        assert_eq!(buf, buf2);
    }

    #[test]
    fn test_encode_uint_with_length() {
        let mut buf = Vec::new();
        encode_uint_with_length(&mut buf, 0x1234, 2).unwrap();
        assert_eq!(buf, vec![0x34, 0x12]);

        buf.clear();
        encode_uint_with_length(&mut buf, 0, 0).unwrap();
        assert!(buf.is_empty());

        buf.clear();
        encode_uint_with_length(&mut buf, 0xABCDEF, 3).unwrap();
        assert_eq!(buf, vec![0xEF, 0xCD, 0xAB]);
    }

    #[test]
    fn test_get_uint_with_length() {
        assert_eq!(get_uint_with_length(&[0x34, 0x12], 2), 0x1234);
        assert_eq!(get_uint_with_length(&[0xEF, 0xCD, 0xAB], 3), 0xABCDEF);
        assert_eq!(get_uint_with_length(&[], 0), 0);
    }
}

#[cfg(test)]
#[path = "encoded_uint_vector_tests.rs"]
mod encoded_uint_vector_tests;
