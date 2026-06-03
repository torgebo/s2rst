// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Encoded vector of variable-length byte strings.
//!
//! Uses [`super::encoded_uint_vector`] for offset storage. Each string is stored
//! contiguously, and offsets record the cumulative end position.
//!
//! Corresponds to C++ `encoded_string_vector.h`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "u64 -> usize for decoded string lengths"
)]
use std::io::{self, Read, Write};

use super::encoded_uint_vector::{decode_uint_vector_u64, encode_uint_vector_u64};

// ─── Encoding ───────────────────────────────────────────────────────────

/// Encodes a vector of byte slices in the `EncodedStringVector` format.
///
/// The format is:
///   `EncodedUintVector<u64>` of cumulative offsets (excluding the initial 0)
///   raw data bytes
///
/// # Errors
///
/// Returns an I/O error if the write fails.
pub fn encode_string_vector(strings: &[&[u8]], w: &mut dyn Write) -> io::Result<()> {
    // Build cumulative offsets (we skip the leading 0 per C++ convention).
    let mut offset = 0u64;
    let mut offsets = Vec::with_capacity(strings.len());
    for s in strings {
        offset += s.len() as u64;
        offsets.push(offset);
    }
    encode_uint_vector_u64(&offsets, w)?;
    for s in strings {
        w.write_all(s)?;
    }
    Ok(())
}

/// Decodes a vector of byte vectors from the `EncodedStringVector` format.
///
/// # Errors
///
/// Returns an error if the data is malformed or the read fails.
pub fn decode_string_vector(r: &mut dyn Read) -> io::Result<Vec<Vec<u8>>> {
    let offsets = decode_uint_vector_u64(r)?;
    if offsets.is_empty() {
        return Ok(Vec::new());
    }
    let total_len = *offsets.last().unwrap_or(&0);
    // Safety limit: don't allocate more than ~500MB for a string vector.
    if total_len > 500_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "string vector data too large",
        ));
    }
    let mut data = vec![0u8; total_len as usize];
    r.read_exact(&mut data)?;

    let mut result = Vec::with_capacity(offsets.len());
    let mut start = 0usize;
    for &end in &offsets {
        let end = end as usize;
        if start <= end && end <= data.len() {
            result.push(data[start..end].to_vec());
        } else {
            result.push(Vec::new());
        }
        start = end;
    }
    Ok(result)
}

/// Helper to build a string vector incrementally, encoding each "string"
/// (block) via a temporary buffer.
#[derive(Debug)]
pub struct StringVectorBuilder {
    blocks: Vec<Vec<u8>>,
}

impl StringVectorBuilder {
    pub(crate) fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Adds a pre-built block.
    pub(crate) fn add(&mut self, data: Vec<u8>) {
        self.blocks.push(data);
    }

    /// Encodes the accumulated blocks as an `EncodedStringVector`.
    pub(crate) fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        let refs: Vec<&[u8]> = self.blocks.iter().map(Vec::as_slice).collect();
        encode_string_vector(&refs, w)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let mut buf = Vec::new();
        encode_string_vector(&[], &mut buf).unwrap();
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_single_string() {
        let strings: Vec<&[u8]> = vec![b"hello"];
        let mut buf = Vec::new();
        encode_string_vector(&strings, &mut buf).unwrap();
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, vec![b"hello".to_vec()]);
    }

    #[test]
    fn test_multiple_strings() {
        let strings: Vec<&[u8]> = vec![b"abc", b"", b"de", b"fghij"];
        let mut buf = Vec::new();
        encode_string_vector(&strings, &mut buf).unwrap();
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(
            decoded,
            vec![
                b"abc".to_vec(),
                b"".to_vec(),
                b"de".to_vec(),
                b"fghij".to_vec(),
            ]
        );
    }

    #[test]
    fn test_builder() {
        let mut builder = StringVectorBuilder::new();
        builder.add(vec![1, 2, 3]);
        builder.add(vec![4, 5]);
        builder.add(vec![]);
        builder.add(vec![6]);

        let mut buf = Vec::new();
        builder.encode(&mut buf).unwrap();
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded, vec![vec![1, 2, 3], vec![4, 5], vec![], vec![6]]);
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ encoded_string_vector_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    /// Encodes, verifies byte size, decodes, and checks roundtrip.
    fn test_encoded_string_vector(input: &[&[u8]], expected_bytes: usize) {
        let mut buf = Vec::new();
        encode_string_vector(input, &mut buf).unwrap();
        assert_eq!(
            expected_bytes,
            buf.len(),
            "encoded byte size mismatch for {} strings",
            input.len()
        );
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(input.len(), decoded.len());
        for (i, (expected, actual)) in input.iter().zip(&decoded).enumerate() {
            assert_eq!(*expected, actual.as_slice(), "string {i} mismatch");
        }
    }

    #[test]
    fn test_empty_vector() {
        // C++ TEST(EncodedStringVectorTest, Empty)
        test_encoded_string_vector(&[], 1);
    }

    #[test]
    fn test_empty_string() {
        // C++ TEST(EncodedStringVectorTest, EmptyString)
        test_encoded_string_vector(&[b""], 2);
    }

    #[test]
    fn test_repeated_empty_strings() {
        // C++ TEST(EncodedStringVectorTest, RepeatedEmptyStrings)
        test_encoded_string_vector(&[b"", b"", b""], 4);
    }

    #[test]
    fn test_one_string() {
        // C++ TEST(EncodedStringVectorTest, OneString)
        test_encoded_string_vector(&[b"apples"], 8);
    }

    #[test]
    fn test_two_strings() {
        // C++ TEST(EncodedStringVectorTest, TwoStrings)
        test_encoded_string_vector(&[b"fuji", b"mutsu"], 12);
    }

    #[test]
    fn test_two_big_strings() {
        // C++ TEST(EncodedStringVectorTest, TwoBigStrings)
        let s1 = vec![b'x'; 10_000];
        let s2 = vec![b'y'; 100_000];
        test_encoded_string_vector(&[&s1, &s2], 110_007);
    }

    #[test]
    fn test_subscript_operator() {
        // C++ TEST(EncodedStringVectorTest, SubscriptOperator)
        let strings: Vec<&[u8]> = vec![b"pink lady", b"gala"];
        let mut buf = Vec::new();
        encode_string_vector(&strings, &mut buf).unwrap();
        let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(2, decoded.len());
        assert_eq!(b"pink lady", decoded[0].as_slice());
        assert_eq!(b"gala", decoded[1].as_slice());
    }

    #[test]
    fn test_reinitialize() {
        // C++ TEST(EncodedStringVectorTest, ReInitialize)
        // Encode three different inputs, decode each time.
        let input1: Vec<&[u8]> = vec![b"abcd", b"edfg"];
        let input2: Vec<&[u8]> = vec![b"hij", b"klm", b"nop", b"qrs"];
        let input3: Vec<&[u8]> = vec![b"tu"];

        let mut buf = Vec::new();
        encode_string_vector(&input1, &mut buf).unwrap();
        let d1 = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(2, d1.len());
        assert_eq!(b"abcd", d1[0].as_slice());

        buf.clear();
        encode_string_vector(&input2, &mut buf).unwrap();
        let d2 = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(4, d2.len());
        assert_eq!(b"hij", d2[0].as_slice());

        buf.clear();
        encode_string_vector(&input3, &mut buf).unwrap();
        let d3 = decode_string_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(1, d3.len());
        assert_eq!(b"tu", d3[0].as_slice());
    }
}
