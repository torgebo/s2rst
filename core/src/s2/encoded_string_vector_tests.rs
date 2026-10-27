// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original property tests for [`super`], added by the formal verification of the decoding
//! layer (specs/003-formal-verify-decoders, research R13 and FR-020). Written for this
//! crate, not ported from upstream S2.

use quickcheck_macros::quickcheck;

use super::*;
use crate::s2::encoded_uint_vector::encode_uint_vector_u64;

#[quickcheck]
fn prop_string_vector_round_trips(v: Vec<Vec<u8>>) -> bool {
    let refs: Vec<&[u8]> = v.iter().map(Vec::as_slice).collect();
    let mut buf = Vec::new();
    encode_string_vector(&refs, &mut buf).unwrap();
    let mut r = buf.as_slice();
    decode_string_vector(&mut r).is_ok_and(|d| d == v && r.is_empty())
}

/// FR-020 (K3): for any offsets over enough data, entry `i` is `data[prev..end]` when
/// `prev <= end <= data.len()`, else empty, where `prev` is the previous offset (0 first) —
/// upstream's lazy rule since v0.14 (commit `95b1e15`).
#[quickcheck]
fn prop_string_vector_corrupt_offsets_decode_as_empty(offsets: Vec<u8>, data: Vec<u8>) -> bool {
    let offsets: Vec<u64> = offsets.into_iter().map(u64::from).collect();
    let total = offsets.last().copied().unwrap_or(0) as usize;
    let mut data = data;
    data.resize(total, 0x5a);
    let mut buf = Vec::new();
    encode_uint_vector_u64(&offsets, &mut buf).unwrap();
    buf.extend_from_slice(&data);
    let Ok(decoded) = decode_string_vector(&mut buf.as_slice()) else {
        return false;
    };
    let mut prev = 0usize;
    for (i, &end) in offsets.iter().enumerate() {
        let end = end as usize;
        let want: &[u8] = if prev <= end && end <= data.len() {
            &data[prev..end]
        } else {
            &[]
        };
        if decoded[i] != want {
            return false;
        }
        prev = end;
    }
    decoded.len() == offsets.len()
}

/// SC-008: 1,000,000 strings still round-trip after FR-019.
#[test]
fn million_string_vector_round_trips() {
    let v: Vec<Vec<u8>> = (0..1_000_000u32)
        .map(|i| i.to_le_bytes()[..(i % 5) as usize].to_vec())
        .collect();
    let refs: Vec<&[u8]> = v.iter().map(Vec::as_slice).collect();
    let mut buf = Vec::new();
    encode_string_vector(&refs, &mut buf).unwrap();
    assert_eq!(decode_string_vector(&mut buf.as_slice()).unwrap(), v);
}

/// Code review of v0.4.0..74b9bc1, finding 1: an end offset past the data decodes as an empty
/// string on every target. On wasm32 the offset 2³² + 1 was truncated to 1 before the
/// comparison, and the entries decoded as `a` and `b`. `wasm/tests/web.rs` runs it on wasm32.
#[test]
fn offset_above_u32_max_is_past_the_data() {
    let mut buf = Vec::new();
    encode_uint_vector_u64(&[(1u64 << 32) + 1, 2], &mut buf).unwrap();
    buf.extend_from_slice(b"ab");
    let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
    assert_eq!(decoded, vec![Vec::<u8>::new(), Vec::new()]);
}
