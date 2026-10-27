// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original property tests for [`super`], added by the formal verification of the decoding
//! layer (specs/003-formal-verify-decoders, research R13). Written for this crate, not
//! ported from upstream S2.

use quickcheck_macros::quickcheck;

use super::*;
use crate::s2::Face;
use crate::s2::Level;

fn round_trip(ids: &[CellId]) -> bool {
    let mut buf = Vec::new();
    encode_s2cell_id_vector(ids, &mut buf).unwrap();
    let mut r = buf.as_slice();
    decode_s2cell_id_vector(&mut r).is_ok_and(|d| d == ids && r.is_empty())
}

/// P-CV-01 beyond the bounds: any 64-bit identifiers, valid cells or not.
#[quickcheck]
fn prop_cell_id_vector_round_trips_any_bits(raw: Vec<u64>) -> bool {
    let ids: Vec<CellId> = raw.into_iter().map(CellId).collect();
    round_trip(&ids)
}

/// Valid cells at one level, which reach the odd shifts and the extended shift code.
#[quickcheck]
fn prop_cell_id_vector_round_trips_same_level(raw: Vec<u64>, level: u8) -> bool {
    let level = level % 31;
    let ids: Vec<CellId> = raw
        .into_iter()
        .map(|x| CellId::from_face_pos_level(Face::from_u8(x as u8 % 6), x >> 3, Level::new(level)))
        .collect();
    round_trip(&ids)
}

/// SC-008: 1,000,000 ids still round-trip after FR-019.
#[test]
fn million_id_vector_round_trips() {
    let ids: Vec<CellId> = (0..1_000_000u64)
        .map(|i| CellId(i.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1))
        .collect();
    assert!(round_trip(&ids));
}
