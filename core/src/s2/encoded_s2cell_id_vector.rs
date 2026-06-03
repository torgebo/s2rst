// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Compact encoding of `S2CellId` vectors.
//!
//! Uses delta encoding with a shared base and shift. Particularly efficient
//! when all cell IDs are at the same level or are nearby on the Hilbert curve.
//!
//! Corresponds to C++ `encoded_s2cell_id_vector.h/cc`.

#![expect(
    clippy::cast_sign_loss,
    reason = "cell ID encoding uses i32 deltas cast to u64"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "cell ID delta encoding — values bounded by format"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64 -> i64 for cell ID delta decoding — bounded by format"
)]
use std::io::{self, Read, Write};

use crate::s2::cell_id::CellId;
use crate::s2::encoded_uint_vector;

// ─── Encode ─────────────────────────────────────────────────────────────

/// Encodes a vector of `S2CellId`s compactly.
///
/// # Errors
///
/// Returns an I/O error if the write fails.
pub fn encode_s2cell_id_vector(v: &[CellId], w: &mut dyn Write) -> io::Result<()> {
    let mut v_or: u64 = 0;
    let mut v_and: u64 = !0;
    let mut v_min: u64 = !0;
    let mut v_max: u64 = 0;
    for &cid in v {
        let id = cid.0;
        v_or |= id;
        v_and &= id;
        v_min = v_min.min(id);
        v_max = v_max.max(id);
    }

    let mut e_base: u64 = 0;
    let mut e_base_len: usize = 0;
    let mut e_shift: i32 = 0;
    let mut e_max_delta_msb: i32 = 0;

    if v_or > 0 {
        // Only allow even shifts, unless all values share the same low bit.
        e_shift = (v_or.trailing_zeros() as i32).min(56) & !1;
        if v_and & (1u64 << e_shift) != 0 {
            e_shift += 1; // All S2CellIds at the same level.
        }

        let mut e_bytes = u64::MAX;
        for len in 0..8usize {
            let t_base = v_min & !(u64::MAX >> (8 * len));
            let t_max_delta_msb = if v.is_empty() {
                0
            } else {
                (64 - ((v_max - t_base) >> e_shift).leading_zeros() as i32).max(1) - 1
            };
            let t_bytes = len as u64 + v.len() as u64 * ((t_max_delta_msb as u64 >> 3) + 1);
            if t_bytes < e_bytes {
                e_base = t_base;
                e_base_len = len;
                e_max_delta_msb = t_max_delta_msb;
                e_bytes = t_bytes;
            }
        }
        // Odd shift costs 1 extra byte; check if even shift is equally good.
        if (e_shift & 1) != 0 && (e_max_delta_msb & 7) != 7 {
            e_shift -= 1;
        }
    }
    debug_assert!(e_base_len <= 7);
    debug_assert!(e_shift <= 56);

    encode_base_shift(w, e_shift, e_base, e_base_len)?;

    // Encode deltas.
    let deltas: Vec<u64> = v.iter().map(|cid| (cid.0 - e_base) >> e_shift).collect();
    encoded_uint_vector::encode_uint_vector_u64(&deltas, w)?;
    Ok(())
}

fn encode_base_shift(w: &mut dyn Write, shift: i32, base: u64, base_len: usize) -> io::Result<()> {
    // shift_code is 5 bits:
    //   values 0..28 → even shifts 0..56
    //   values 29,30 → odd shifts 1,3
    //   value 31     → odd shift >= 5 (next byte encodes shift/2)
    let shift_code = if shift & 1 == 0 {
        shift >> 1
    } else {
        (shift >> 1) + 29
    };
    let shift_code = shift_code.min(31);

    let byte0: u8 = ((shift_code as u8) << 3) | (base_len as u8);
    w.write_all(&[byte0])?;
    if shift_code == 31 {
        w.write_all(&[(shift >> 1) as u8])?;
    }

    // Encode the base_len most-significant bytes of base.
    let base_bytes = base >> (64 - 8 * base_len.max(1));
    encoded_uint_vector::encode_uint_with_length(w, base_bytes, base_len)?;
    Ok(())
}

// ─── Decode ─────────────────────────────────────────────────────────────

/// Decodes a vector of `S2CellId`s encoded by [`encode_s2cell_id_vector`].
///
/// # Errors
///
/// Returns an error if the data is malformed or the read fails.
pub fn decode_s2cell_id_vector(r: &mut dyn Read) -> io::Result<Vec<CellId>> {
    let (base, shift) = decode_base_shift(r)?;
    let deltas = encoded_uint_vector::decode_uint_vector_u64(r)?;

    let mut result = Vec::with_capacity(deltas.len());
    for d in deltas {
        result.push(CellId(base.wrapping_add(d.wrapping_shl(shift))));
    }
    Ok(result)
}

fn decode_base_shift(r: &mut dyn Read) -> io::Result<(u64, u32)> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    let code_plus_len = buf[0];
    let mut shift_code = i32::from(code_plus_len >> 3);
    let base_len = (code_plus_len & 7) as usize;

    if shift_code == 31 {
        r.read_exact(&mut buf)?;
        shift_code = 29 + i32::from(buf[0]);
        if shift_code > 56 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid shift in S2CellIdVector",
            ));
        }
    }

    let raw_base = encoded_uint_vector::decode_uint_with_length(r, base_len)?;
    let mut base = raw_base << (64 - 8 * base_len.max(1));

    let shift = if shift_code >= 29 {
        let s = 2 * (shift_code - 29) + 1;
        base |= 1u64 << (s - 1);
        s as u32
    } else {
        (2 * shift_code) as u32
    };

    Ok((base, shift))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes `cell_ids`, verifies encoded size equals `expected_bytes`,
    /// then decodes and verifies the result matches the input.
    fn test_encoded(cell_ids: &[CellId], expected_bytes: usize) {
        let mut buf = Vec::new();
        encode_s2cell_id_vector(cell_ids, &mut buf).unwrap();
        assert_eq!(
            buf.len(),
            expected_bytes,
            "encoded size mismatch for {} cell IDs",
            cell_ids.len()
        );
        let decoded = decode_s2cell_id_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.len(), cell_ids.len());
        for (i, (got, want)) in decoded.iter().zip(cell_ids).enumerate() {
            assert_eq!(got.0, want.0, "cell id {i} mismatch");
        }
    }

    /// Like `test_encoded` but takes raw u64 values.
    fn test_encoded_raw(raw_ids: &[u64], expected_bytes: usize) {
        let ids: Vec<CellId> = raw_ids.iter().map(|&id| CellId(id)).collect();
        test_encoded(&ids, expected_bytes);
    }

    fn roundtrip(cell_ids: &[CellId]) -> Vec<CellId> {
        let mut buf = Vec::new();
        encode_s2cell_id_vector(cell_ids, &mut buf).unwrap();
        let decoded = decode_s2cell_id_vector(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.len(), cell_ids.len());
        for (i, (got, want)) in decoded.iter().zip(cell_ids).enumerate() {
            assert_eq!(got.0, want.0, "cell id {i} mismatch");
        }
        decoded
    }

    // ─── Original tests ──────────────────────────────────────────────

    #[test]
    fn test_empty() {
        test_encoded(&[], 2);
    }

    #[test]
    fn test_single() {
        let id = CellId::from_debug_string("3/012").unwrap();
        roundtrip(&[id]);
    }

    #[test]
    fn test_same_level() {
        let ids: Vec<CellId> = (0..6).map(CellId::from_face).collect();
        roundtrip(&ids);
    }

    #[test]
    fn test_mixed_levels() {
        let ids = vec![
            CellId::from_debug_string("0/").unwrap(),
            CellId::from_debug_string("1/01").unwrap(),
            CellId::from_debug_string("2/0123").unwrap(),
        ];
        roundtrip(&ids);
    }

    #[test]
    fn test_leaf_cells() {
        let id = CellId::from_face(0);
        let ids = vec![
            id.child_begin_at_level(30),
            id.child_begin_at_level(30).next(),
            id.child_end_at_level(30).prev(),
        ];
        roundtrip(&ids);
    }

    #[test]
    fn test_all_faces() {
        let ids: Vec<CellId> = (0..6).map(CellId::from_face).collect();
        let mut buf = Vec::new();
        encode_s2cell_id_vector(&ids, &mut buf).unwrap();
        assert!(buf.len() <= 10, "encoded size {} too large", buf.len());
        roundtrip(&ids);
    }

    // ─── C++ tests: basic cell types ─────────────────────────────────

    /// C++ TEST(EncodedS2CellIdVector, None)
    #[test]
    fn test_none() {
        test_encoded(&[CellId::none()], 3);
    }

    /// C++ TEST(EncodedS2CellIdVector, `NoneNone`)
    #[test]
    fn test_none_none() {
        test_encoded(&[CellId::none(), CellId::none()], 4);
    }

    /// C++ TEST(EncodedS2CellIdVector, Sentinel)
    #[test]
    fn test_sentinel() {
        test_encoded(&[CellId::sentinel()], 10);
    }

    /// C++ TEST(EncodedS2CellIdVector, `MaximumShiftCell`)
    /// Tests encoding of a single cell at level 2 (maximum shift = 56).
    #[test]
    fn test_maximum_shift_cell() {
        let id = CellId::from_debug_string("0/00").unwrap();
        test_encoded(&[id], 3);
    }

    /// C++ TEST(EncodedS2CellIdVector, `SentinelSentinel`)
    #[test]
    fn test_sentinel_sentinel() {
        test_encoded(&[CellId::sentinel(), CellId::sentinel()], 11);
    }

    /// C++ TEST(EncodedS2CellIdVector, `NoneSentinelNone`)
    #[test]
    fn test_none_sentinel_none() {
        test_encoded(&[CellId::none(), CellId::sentinel(), CellId::none()], 26);
    }

    // ─── C++ tests: encoding properties ──────────────────────────────

    /// C++ TEST(EncodedS2CellIdVector, `InvalidCells`)
    /// Tests that cells with an invalid LSB can be encoded.
    #[test]
    fn test_invalid_cells() {
        test_encoded_raw(&[0x6, 0xe, 0x7e], 5);
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteLeafCells`)
    /// If all cells are leaf cells, the low bit is not encoded and
    /// this uses the standard 1-byte header.
    #[test]
    fn test_one_byte_leaf_cells() {
        test_encoded_raw(&[0x3, 0x7, 0x177], 5);
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteLevel29Cells`)
    /// If all cells are at level 29, the low bit is not encoded.
    #[test]
    fn test_one_byte_level_29_cells() {
        test_encoded_raw(&[0xc, 0x1c, 0x47c], 5);
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteLevel28Cells`)
    /// If all cells are at level 28, the low bit is not encoded,
    /// using the extended 2-byte header.
    #[test]
    fn test_one_byte_level_28_cells() {
        test_encoded_raw(&[0x30, 0x70, 0x1770], 6);
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteMixedCellLevels`)
    /// Cells at mixed levels can be encoded in one byte.
    #[test]
    fn test_one_byte_mixed_cell_levels() {
        test_encoded_raw(&[0x300, 0x1c00, 0x7000, 0xff00], 6);
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteMixedCellLevelsWithPrefix`)
    /// Cells at mixed levels sharing a multi-byte prefix.
    #[test]
    fn test_one_byte_mixed_cell_levels_with_prefix() {
        test_encoded_raw(
            &[
                0x1234567800000300,
                0x1234567800001c00,
                0x1234567800007000,
                0x123456780000ff00,
            ],
            10,
        );
    }

    /// C++ TEST(EncodedS2CellIdVector, `OneByteRangeWithBaseValue`)
    /// Tests cells encodable in one byte by choosing a base value
    /// whose bit range overlaps the delta values.
    #[test]
    fn test_one_byte_range_with_base_value() {
        test_encoded_raw(
            &[
                0x00ffff0000000000,
                0x0100fc0000000000,
                0x0100500000000000,
                0x0100330000000000,
            ],
            9,
        );
    }

    // ─── C++ tests: shift range validation ───────────────────────────

    /// C++ TEST(EncodedS2CellIdVector, `MaxShiftRange`)
    /// Verify that the maximum supported shift (56) can be decoded.
    #[test]
    fn test_max_shift_range() {
        let bytes: Vec<u8> = vec![
            (31 << 3) + 1, // shift_code=31 means extended header; count=1
            27,            // 27+29 = 56, the maximum supported shift
            1,
            0, // encoded cell ID (not important)
        ];
        let result = decode_s2cell_id_vector(&mut bytes.as_slice());
        assert!(result.is_ok(), "max shift should decode successfully");
    }

    /// C++ TEST(EncodedS2CellIdVector, `ShiftOutOfRange`)
    /// Verify that a shift > 56 is rejected.
    #[test]
    fn test_shift_out_of_range() {
        let bytes: Vec<u8> = vec![
            (31 << 3) + 1, // shift_code=31 means extended header; count=1
            28,            // 28+29 = 57, exceeds maximum shift of 56
            1,
            0, // encoded cell ID (not important)
        ];
        let result = decode_s2cell_id_vector(&mut bytes.as_slice());
        assert!(result.is_err(), "shift > 56 should be rejected");
    }

    // ─── C++ tests: structured cell collections ──────────────────────

    /// C++ TEST(EncodedS2CellIdVector, `SixFaceCells`)
    #[test]
    fn test_six_face_cells() {
        let ids: Vec<CellId> = (0..6).map(CellId::from_face).collect();
        test_encoded(&ids, 8);
    }

    /// C++ TEST(EncodedS2CellIdVector, `FourLevel10Children`)
    #[test]
    fn test_four_level_10_children() {
        let parent = CellId::from_debug_string("3/012301230").unwrap();
        let mut ids = Vec::new();
        let mut id = parent.child_begin();
        let end = parent.child_end();
        while id != end {
            ids.push(id);
            id = id.next();
        }
        assert_eq!(ids.len(), 4);
        test_encoded(&ids, 8);
    }

    /// C++ TEST(EncodedS2CellIdVector, `CoveringCells`)
    /// Tests encoding of a realistic covering (97 cells at mixed levels).
    #[test]
    fn test_covering_cells() {
        let raw_ids: Vec<u64> = vec![
            0x414a617f00000000,
            0x414a61c000000000,
            0x414a624000000000,
            0x414a63c000000000,
            0x414a647000000000,
            0x414a64c000000000,
            0x414a653000000000,
            0x414a704000000000,
            0x414a70c000000000,
            0x414a714000000000,
            0x414a71b000000000,
            0x414a7a7c00000000,
            0x414a7ac000000000,
            0x414a8a4000000000,
            0x414a8bc000000000,
            0x414a8c4000000000,
            0x414a8d7000000000,
            0x414a8dc000000000,
            0x414a914000000000,
            0x414a91c000000000,
            0x414a924000000000,
            0x414a942c00000000,
            0x414a95c000000000,
            0x414a96c000000000,
            0x414ab0c000000000,
            0x414ab14000000000,
            0x414ab34000000000,
            0x414ab3c000000000,
            0x414ab44000000000,
            0x414ab4c000000000,
            0x414ab6c000000000,
            0x414ab74000000000,
            0x414ab8c000000000,
            0x414ab94000000000,
            0x414aba1000000000,
            0x414aba3000000000,
            0x414abbc000000000,
            0x414abe4000000000,
            0x414abec000000000,
            0x414abf4000000000,
            0x46b5454000000000,
            0x46b545c000000000,
            0x46b5464000000000,
            0x46b547c000000000,
            0x46b5487000000000,
            0x46b548c000000000,
            0x46b5494000000000,
            0x46b54a5400000000,
            0x46b54ac000000000,
            0x46b54b4000000000,
            0x46b54bc000000000,
            0x46b54c7000000000,
            0x46b54c8004000000,
            0x46b54ec000000000,
            0x46b55ad400000000,
            0x46b55b4000000000,
            0x46b55bc000000000,
            0x46b55c4000000000,
            0x46b55c8100000000,
            0x46b55dc000000000,
            0x46b55e4000000000,
            0x46b5604000000000,
            0x46b560c000000000,
            0x46b561c000000000,
            0x46ca424000000000,
            0x46ca42c000000000,
            0x46ca43c000000000,
            0x46ca444000000000,
            0x46ca45c000000000,
            0x46ca467000000000,
            0x46ca469000000000,
            0x46ca5fc000000000,
            0x46ca604000000000,
            0x46ca60c000000000,
            0x46ca674000000000,
            0x46ca679000000000,
            0x46ca67f000000000,
            0x46ca684000000000,
            0x46ca855000000000,
            0x46ca8c4000000000,
            0x46ca8cc000000000,
            0x46ca8e5400000000,
            0x46ca8ec000000000,
            0x46ca8f0100000000,
            0x46ca8fc000000000,
            0x46ca900400000000,
            0x46ca98c000000000,
            0x46ca994000000000,
            0x46ca99c000000000,
            0x46ca9a4000000000,
            0x46ca9ac000000000,
            0x46ca9bd500000000,
            0x46ca9e4000000000,
            0x46ca9ec000000000,
            0x46caf34000000000,
            0x46caf4c000000000,
            0x46caf54000000000,
        ];
        assert_eq!(raw_ids.len(), 97);
        test_encoded_raw(&raw_ids, 488);
    }

    // ─── C++ tests: malformed input ──────────────────────────────────

    /// C++ TEST(EncodedS2CellIdVector, `EncodedS2CellIdVectorInitNeverCrashesRegression`)
    /// Tests that malformed encoded data doesn't crash the decoder.
    #[test]
    fn test_init_never_crashes_regression() {
        let bytes: Vec<u8> = vec![32, 135, 128, 128, 128, 48, 39, 132, 143, 84];
        let result = decode_s2cell_id_vector(&mut bytes.as_slice());
        // Either decodes successfully or returns an error — must not panic.
        drop(result);
    }
}
