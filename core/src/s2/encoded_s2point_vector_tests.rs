// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for [`super`], added by the formal verification of the decoding layer
//! (specs/003-formal-verify-decoders). Written for this crate, not ported from upstream S2.

use super::*;

/// R6 (FR-013): the unknown-format error is formatted lazily, with today's text.
#[test]
fn unknown_format_error_text_is_unchanged() {
    for tag in 2u8..8 {
        let err = decode_s2point_vector(&mut [tag].as_slice()).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            err.to_string(),
            format!("unknown S2PointVector format {tag}")
        );
    }
}

/// R6 (FR-013): the compact format's invalid-level error keeps its text.
#[test]
fn invalid_level_error_text_is_unchanged() {
    // Tag 1 (CELL_IDS), then a second byte with level 31 in its top five bits.
    let err = decode_s2point_vector(&mut [1u8, 31 << 3].as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert_eq!(err.to_string(), "invalid level 31");
}

fn fast_round_trip(points: &[Point]) -> bool {
    let mut buf = Vec::new();
    encode_s2point_vector(points, CodingHint::Fast, &mut buf).unwrap();
    let mut r = buf.as_slice();
    let Ok(d) = decode_s2point_vector(&mut r) else {
        return false;
    };
    r.is_empty()
        && d.len() == points.len()
        && d.iter().zip(points).all(|(a, b)| {
            a.0.x.to_bits() == b.0.x.to_bits()
                && a.0.y.to_bits() == b.0.y.to_bits()
                && a.0.z.to_bits() == b.0.z.to_bits()
        })
}

/// P-PV-01 beyond the bounds: the FAST format round-trips every bit pattern (NaN, infinities,
/// non-unit vectors), bit for bit.
#[quickcheck_macros::quickcheck]
fn prop_fast_point_vector_round_trips_any_bits(raw: Vec<(u64, u64, u64)>) -> bool {
    let points: Vec<Point> = raw
        .into_iter()
        .map(|(x, y, z)| {
            Point(Vector {
                x: f64::from_bits(x),
                y: f64::from_bits(y),
                z: f64::from_bits(z),
            })
        })
        .collect();
    fast_round_trip(&points)
}

/// SC-008: 1,000,000 points still round-trip after FR-019.
#[test]
fn million_point_vector_round_trips() {
    let points: Vec<Point> = (0..1_000_000u64)
        .map(|i| {
            let f = i as f64;
            Point(Vector {
                x: f,
                y: -f,
                z: 0.5,
            })
        })
        .collect();
    assert!(fast_round_trip(&points));
}

/// D-003 regression test for the point vector's own varint reader: a tenth byte above 1 is
/// rejected, as upstream does; 0 and 1 are accepted.
#[test]
fn reconstruct_uvarint_rejects_a_tenth_byte_above_one() {
    let rest = |last: u8| {
        let mut b = vec![0xff; 8];
        b.push(last);
        b
    };
    assert!(reconstruct_uvarint(0xff, &mut rest(0x02).as_slice()).is_err());
    assert!(reconstruct_uvarint(0xff, &mut rest(0x7f).as_slice()).is_err());
    assert_eq!(
        reconstruct_uvarint(0xff, &mut rest(0x01).as_slice()).unwrap(),
        u64::MAX
    );
}

/// A header varint cut short, `80 80` and then the end of the input, is rejected. The mutation
/// measurement found no test for it (`verify/decoding/MUTATION.md`): a reader that ended a
/// varint at the continuation byte 0x80 decoded this as zero points.
#[test]
fn header_varint_cut_short_is_rejected() {
    let err = decode_s2point_vector(&mut [0x80u8, 0x80].as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
}
