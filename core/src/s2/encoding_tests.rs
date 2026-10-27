// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for the binary codecs in [`super`], added by the formal verification of the
//! decoding layer (specs/003-formal-verify-decoders). Written for this crate, not ported
//! from upstream S2.

use super::*;
use crate::s2::Face;

/// R6 (FR-013): the `Rect` decoder's version error is formatted lazily, with today's text.
#[test]
fn rect_version_error_text_is_unchanged() {
    let mut bytes = vec![2u8];
    bytes.extend_from_slice(&[0u8; 32]);
    let err = Rect::decode(&mut bytes.as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert_eq!(err.to_string(), "unsupported encoding version 2");
}

/// The `VARINT_DECODE` rows of the C++ reference vectors (spec 003 FR-024), checked against
/// the crate-private `read_uvarint`: every row matches C++, including the ten-byte forms
/// whose tenth byte is above 1, which both reject (D-003, fixed).
#[test]
fn read_uvarint_matches_cpp() {
    const CSV: &str = include_str!("../../tests/data/cpp_coding_vectors.csv");
    let mut rows = 0;
    for (i, line) in CSV.lines().enumerate() {
        let f: Vec<&str> = line.split(',').collect();
        if f[0] != "VARINT_DECODE" {
            continue;
        }
        rows += 1;
        let input: Vec<u8> = (0..f[1].len())
            .step_by(2)
            .map(|k| u8::from_str_radix(&f[1][k..k + 2], 16).unwrap())
            .collect();
        let mut r = input.as_slice();
        let got = match read_uvarint(&mut r) {
            Err(_) => "reject".to_string(),
            Ok(v) => format!("ok|{}|1|{v:016x}", input.len() - r.len()),
        };
        assert_eq!(got, f[2], "line {}: input {}", i + 1, f[1]);
    }
    assert!(rows >= 40, "only {rows} VARINT_DECODE rows");
}

/// D-003 regression test (spec 003 EVALUATION.md, "Defects found"): upstream
/// `Varint::Parse64WithLimit` and `Parse64` reject a ten-byte varint whose tenth byte is
/// above 1 ("too long to be a varint64"); `read_uvarint` used to accept it and drop the bits
/// above 63. A tenth byte of 0 or 1 is still accepted.
#[test]
fn read_uvarint_rejects_a_tenth_byte_above_one() {
    for last in [0x02u8, 0x7f] {
        let mut b = vec![0xff; 9];
        b.push(last);
        assert!(
            read_uvarint(&mut b.as_slice()).is_err(),
            "tenth byte {last:#x}"
        );
    }
    let mut max = vec![0xff; 9];
    max.push(0x01);
    assert_eq!(read_uvarint(&mut max.as_slice()).unwrap(), u64::MAX);
}

/// D-004, kept as a documented deviation (spec 003 FR-022; the user's decision of
/// 2026-10-06): upstream `S2Cap::Decode` builds the radius with `S1ChordAngle::FromLength2`,
/// which is `std::min(4.0, length2)`, so a NaN `length2` decodes as a full cap. The library
/// keeps the NaN, so the cap is invalid and is rejected, like every other invalid cap.
#[test]
fn cap_with_nan_length2_is_rejected() {
    let mut b = Vec::new();
    for x in [0.0f64, 0.0, 1.0, f64::NAN] {
        b.extend_from_slice(&x.to_le_bytes());
    }
    let err = Cap::decode(&mut b.as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

/// D-004 with a NaN of another payload, and a negative NaN: still rejected (spec 007 FR-018:
/// `ChordAngle::from_length2(NaN)` is now `STRAIGHT`, as upstream, so the decoder rejects a NaN
/// radius before building the chord angle).
#[test]
fn cap_with_any_nan_length2_is_rejected() {
    for nan in [f64::from_bits(0x7ff0_0000_0000_0001), -f64::NAN] {
        let mut b = Vec::new();
        for x in [0.0f64, 0.0, 1.0, nan] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        let err = Cap::decode(&mut b.as_slice()).unwrap_err();
        assert_eq!(
            err.kind(),
            io::ErrorKind::InvalidData,
            "{:#x}",
            nan.to_bits()
        );
    }
}

/// Spec 007 FR-017 (option B): C++ keeps a negative radius as it is, so its bytes for an empty
/// cap of radius −5 decode here as the empty cap of radius `NEGATIVE` (−1); re-encoding gives
/// −1's bytes, not −5's. −∞ likewise.
#[test]
fn cap_with_a_negative_radius_decodes_as_negative() {
    for radius in [-5.0f64, -0.001, f64::NEG_INFINITY] {
        let mut b = Vec::new();
        for x in [0.0f64, 0.0, 1.0, radius] {
            b.extend_from_slice(&x.to_le_bytes());
        }
        let cap = Cap::decode(&mut b.as_slice()).unwrap();
        assert!(cap.is_empty(), "{radius}");
        assert_eq!(
            cap.chord_radius().length2().to_bits(),
            (-1.0f64).to_bits(),
            "{radius}"
        );
        let mut out = Vec::new();
        cap.encode(&mut out).unwrap();
        assert_eq!(&out[24..32], &(-1.0f64).to_le_bytes(), "{radius}");
    }
}

/// Code review of v0.4.0..74b9bc1, finding 1 (same shape as the vector decoders): a loop count
/// of 2³² + 1 is over the limit on every target. On wasm32 it was truncated to 1 before the
/// check, and the input decoded as a one-loop polygon.
#[test]
fn lax_polygon_loop_count_above_u32_max_is_rejected() {
    let mut b = vec![LAX_POLYGON_VERSION];
    write_uvarint(&mut b, (1u64 << 32) + 1).unwrap();
    let p = Point::from_coords(1.0, 0.0, 0.0);
    encoded_s2point_vector::encode_s2point_vector(&[p], CodingHint::Fast, &mut b).unwrap();
    let err = LaxPolygon::decode(&mut b.as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

/// Code review of v0.4.0..74b9bc1, finding 1 (same shape): a compressed polyline whose
/// off-center count is 2³² is rejected on every target. On wasm32 the count was truncated to 0
/// before the check, and the input decoded as the polyline.
#[test]
fn compressed_polyline_off_center_count_above_u32_max_is_rejected() {
    let vertices: Vec<Point> = [0u8, 1, 2]
        .iter()
        .map(|&f| {
            CellId::from_face(Face::from_u8(f))
                .child_begin_at_level(Level::new(10))
                .to_point()
        })
        .collect();
    let polyline = crate::s2::polyline::Polyline::new(vertices);
    let mut b = Vec::new();
    encode_polyline_compressed(&polyline, &mut b).unwrap();
    assert_eq!(b[0], POLYLINE_COMPRESSED_VERSION);
    // The last field is the off-center count, here 0 in one byte.
    assert_eq!(b.pop(), Some(0));
    let mut good = b.clone();
    good.push(0);
    assert!(crate::s2::polyline::Polyline::decode(&mut good.as_slice()).is_ok());
    write_uvarint(&mut b, 1u64 << 32).unwrap();
    let err = crate::s2::polyline::Polyline::decode(&mut b.as_slice()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}
