// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original property tests for [`super`], added by the formal verification of the decoding
//! layer (specs/003-formal-verify-decoders, research R13): round trips at sizes beyond the
//! proofs' bounds, and a 1,000,000-element vector (SC-008). Written for this crate, not
//! ported from upstream S2.

use quickcheck_macros::quickcheck;

use super::*;

fn round_trip_u64(v: &[u64]) -> bool {
    let mut buf = Vec::new();
    encode_uint_vector_u64(v, &mut buf).unwrap();
    let mut r = buf.as_slice();
    decode_uint_vector_u64(&mut r).is_ok_and(|d| d == v && r.is_empty())
}

#[quickcheck]
fn prop_uint_vector_u64_round_trips(v: Vec<u64>) -> bool {
    round_trip_u64(&v)
}

#[quickcheck]
fn prop_uint_vector_u32_round_trips(v: Vec<u32>) -> bool {
    let mut buf = Vec::new();
    encode_uint_vector_u32(&v, &mut buf).unwrap();
    let mut r = buf.as_slice();
    decode_uint_vector_u32(&mut r).is_ok_and(|d| d == v && r.is_empty())
}

/// P-UV-05 beyond the bounds: the header's width field is the minimal width of the largest
/// element (at least 1).
#[quickcheck]
fn prop_uint_vector_width_is_minimal(v: Vec<u64>) -> bool {
    let mut buf = Vec::new();
    encode_uint_vector_u64(&v, &mut buf).unwrap();
    let header = read_uvarint(&mut buf.as_slice()).unwrap();
    let width = (header & 7) + 1;
    let largest = v.iter().fold(1u64, |a, &x| a | x);
    width == u64::from(largest.ilog2() / 8 + 1) && header / 8 == v.len() as u64
}

/// SC-008: a 1,000,000-element vector still round-trips after FR-019 (grow on demand).
#[test]
fn million_element_vector_round_trips() {
    let v: Vec<u64> = (0..1_000_000u64)
        .map(|i| i.wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .collect();
    assert!(round_trip_u64(&v));
}
