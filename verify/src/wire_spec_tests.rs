// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the reference description (Constitution III; spec 003 T009).

use super::*;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

#[test]
fn varint_every_length_round_trips() {
    for k in 0..10u32 {
        let x = if k == 0 { 0 } else { (1u64 << (7 * k)) | 1 };
        let b = enc_varint(x);
        assert_eq!(b.len(), (k + 1) as usize, "length of {x:#x}");
        assert_eq!(ref_varint(&b), Some((x, b.len())));
    }
    let max = enc_varint(u64::MAX);
    assert_eq!(max.len(), 10);
    assert_eq!(max[9], 1);
    assert_eq!(ref_varint(&max), Some((u64::MAX, 10)));
}

#[test]
fn varint_over_long_truncated_and_trailing() {
    assert_eq!(ref_varint(&[0x80, 0x00]), Some((0, 2)));
    assert_eq!(ref_varint(&[0xff, 0x80, 0x00]), Some((127, 3)));
    assert_eq!(ref_varint(&[]), None);
    assert_eq!(ref_varint(&[0x80]), None);
    assert_eq!(ref_varint(&[0x05, 0x99]), Some((5, 1)));
}

#[test]
fn varint_tenth_byte_rule() {
    let mut b = vec![0xff; 9];
    for (last, upstream) in [
        (0u8, true),
        (1, true),
        (2, false),
        (0x7f, false),
        (0x80, false),
    ] {
        b.push(last);
        assert_eq!(ref_varint(&b).is_some(), upstream, "tenth byte {last:#x}");
        // Since the D-003 fix the library agrees with upstream.
        assert_eq!(lib_varint(&b), ref_varint(&b), "lib, tenth byte {last:#x}");
        b.pop();
    }
    // An eleventh continuation byte is never a varint.
    b.extend_from_slice(&[0x80, 0x01]);
    assert_eq!(ref_varint(&b), None);
}

#[test]
fn fixed_width_and_byte_width() {
    assert_eq!(enc_fixed(0x0102_0304, 3), vec![0x04, 0x03, 0x02]);
    assert_eq!(
        ref_fixed(&[0x04, 0x03, 0x02, 0xff], 3),
        Some((0x02_0304, 3))
    );
    assert_eq!(ref_fixed(&[1, 2], 3), None);
    assert_eq!(ref_fixed(&[], 0), Some((0, 0)));
    for (v, w) in [
        (0u64, 1),
        (1, 1),
        (0xff, 1),
        (0x100, 2),
        (u64::MAX >> 8, 7),
        (u64::MAX, 8),
    ] {
        assert_eq!(byte_width(v), w, "{v:#x}");
    }
}

#[test]
fn uint_vector_width_is_minimal_and_round_trips() {
    for v in [
        vec![],
        vec![0],
        vec![0xff, 1],
        vec![0x1_0000, 3],
        vec![u64::MAX, 0],
    ] {
        let b = enc_uint_vector(&v, 8);
        let (h, hn) = ref_varint(&b).unwrap();
        let width = (h & 7) + 1;
        let mut one = 1u64;
        for &x in &v {
            one |= x;
        }
        assert_eq!(width as usize, byte_width(one));
        assert_eq!(h / 8, v.len() as u64);
        assert_eq!(b.len(), hn + v.len() * width as usize);
        assert_eq!(ref_uint_vector(&b, 8), Some((v.clone(), b.len())));
        // A trailing byte is left unread.
        let mut t = b.clone();
        t.push(9);
        assert_eq!(ref_uint_vector(&t, 8), Some((v.clone(), b.len())));
        // Every proper prefix is rejected, except a vector with no elements.
        for k in 0..b.len() {
            assert_eq!(ref_uint_vector(&b[..k], 8), None, "prefix {k} of {v:?}");
        }
    }
    let u32s = [0xdead_beef_u64, 7];
    let b = enc_uint_vector(&u32s, 4);
    assert_eq!(ref_uint_vector(&b, 4), Some((u32s.to_vec(), b.len())));
}

#[test]
fn uint_vector_count_limit_is_a_library_deviation() {
    // A header announcing LIB_MAX_COUNT + 1 one-byte elements, followed by too few bytes:
    // both reject, upstream because the bytes are missing, the library because of the limit.
    let over = enc_varint((LIB_MAX_COUNT + 1) * 8);
    assert_eq!(ref_uint_vector(&over, 8), None);
    assert_eq!(lib_uint_vector(&over, 8), None);
    assert!(dev_count(LIB_MAX_COUNT + 1, LIB_MAX_COUNT));
    assert!(!dev_count(LIB_MAX_COUNT, LIB_MAX_COUNT));
}

#[test]
fn cellid_vector_base_lengths_and_extended_shift() {
    // Leaf cells: the encoder may drop the odd shift to an even one (upstream's last step).
    let leaves = [0x1000_0000_0000_0001u64, 0x1000_0000_0000_0003];
    let b = enc_cellid_vector(&leaves);
    assert_eq!(ref_cellid_vector(&b), Some((leaves.to_vec(), b.len())));
    // A hand-built extended shift: code 31, byte 2, so shift 5 and base bit 4 set; one delta 1.
    let mut b = vec![0xf8, 2];
    b.extend_from_slice(&enc_uint_vector(&[1], 8));
    assert_eq!(ref_cellid_vector(&b), Some((vec![16 + (1 << 5)], b.len())));
    // Shift codes 29 and 30: shifts 1 and 3, with base bits 0 and 2.
    for (code, id) in [(29u8, 1u64 + (1 << 1)), (30, 4 + (1 << 3))] {
        let mut b = vec![code << 3];
        b.extend_from_slice(&enc_uint_vector(&[1], 8));
        assert_eq!(
            ref_cellid_vector(&b),
            Some((vec![id], b.len())),
            "code {code}"
        );
    }
    // Base length 0 with a large delta, and base length 7.
    for v in [
        vec![1u64 << 63],
        vec![0x0123_4567_89ab_cd01, 0x0123_4567_89ab_cd03],
    ] {
        let b = enc_cellid_vector(&v);
        assert_eq!(ref_cellid_vector(&b), Some((v.clone(), b.len())));
    }
    // An extended shift above 56 is rejected; 56 is accepted.
    assert_eq!(ref_cellid_vector(&[0xf8, 28, 0x00]), None);
    assert!(ref_cellid_vector(&[0xf8, 27, 0x00]).is_some());
    // Too short for upstream's two-byte check.
    assert_eq!(ref_cellid_vector(&[0x00]), None);
    assert_eq!(
        ref_cellid_vector(&enc_cellid_vector(&[])),
        Some((vec![], 2))
    );
}

#[test]
fn string_vector_lazy_offsets_rule() {
    let mut b = enc_uint_vector(&[5, 2, 6], 8);
    b.extend_from_slice(b"abcdef");
    let expected = vec![b"abcde".to_vec(), Vec::new(), b"cdef".to_vec()];
    assert_eq!(ref_string_vector(&b), Some((expected, b.len())));
    let mut b = enc_uint_vector(&[8, 6], 8);
    b.extend_from_slice(b"abcdef");
    assert_eq!(
        ref_string_vector(&b),
        Some((vec![Vec::new(), Vec::new()], b.len()))
    );
    // Data missing.
    let mut b = enc_uint_vector(&[3, 10], 8);
    b.extend_from_slice(b"abcdef");
    assert_eq!(ref_string_vector(&b), None);
    // Round trip with empty strings.
    let v = vec![Vec::new(), b"x".to_vec(), Vec::new()];
    let b = enc_string_vector(&v);
    assert_eq!(ref_string_vector(&b), Some((v, b.len())));
    assert!(dev_string_total(LIB_MAX_STRING_DATA + 1));
    assert!(!dev_string_total(LIB_MAX_STRING_DATA));
}

#[test]
fn point_vector_tags_and_round_trip() {
    let p = [bits(f64::NAN), bits(-0.0), bits(1e300)];
    let b = enc_point_vector_fast(&[p]);
    assert_eq!(b[0] & 7, 0);
    assert_eq!(
        ref_point_vector(&b),
        PointVectorAnswer::Points(vec![p], b.len())
    );
    for tag in 2..8u8 {
        assert_eq!(ref_point_vector(&[tag]), PointVectorAnswer::Reject);
    }
    assert_eq!(ref_point_vector(&[1, 0]), PointVectorAnswer::Compact);
    assert_eq!(ref_point_vector(&[]), PointVectorAnswer::Reject);
    // The library's point limit is a deviation; upstream's is i32::MAX.
    let over = enc_varint((LIB_MAX_POINTS + 1) << 3);
    assert_eq!(lib_point_vector(&over), PointVectorAnswer::Reject);
}

#[test]
fn fixed_size_codecs() {
    // CellId: upstream accepts any 64 bits; the library rejects invalid cells.
    let face3 = 0x7000_0000_0000_0000u64;
    assert_eq!(ref_cell_id(&enc_cell_id(face3)), Some((face3, 8)));
    assert_eq!(lib_cell_id(&enc_cell_id(face3)), Some((face3, 8)));
    assert_eq!(ref_cell_id(&enc_cell_id(0)), Some((0, 8)));
    assert_eq!(lib_cell_id(&enc_cell_id(0)), None);
    assert!(dev_valid_id(0) && dev_valid_id(7 << 61) && !dev_valid_id(face3));
    // Point: any bits, exactly 24 bytes consumed.
    let p = [bits(f64::NAN), bits(f64::INFINITY), 0];
    let mut b = enc_point(p);
    b.push(1);
    assert_eq!(ref_point(&b), Some((p, 24)));
    // Cap: upstream clamps NaN and values above 4 to 4; the library keeps NaN and rejects.
    let c = [bits(0.0), bits(0.0), bits(1.0), bits(f64::NAN)];
    assert_eq!(
        ref_cap(&enc_cap(c)).map(|(c, _)| f64::from_bits(c[3])),
        Some(4.0)
    );
    assert_eq!(lib_cap(&enc_cap(c)), None);
    let c5 = [bits(0.0), bits(0.0), bits(1.0), bits(5.0)];
    assert_eq!(
        lib_cap(&enc_cap(c5)).map(|(c, _)| f64::from_bits(c[3])),
        Some(4.0)
    );
    let not_unit = [bits(3.0), bits(4.0), bits(12.0), bits(1.0)];
    assert!(ref_cap(&enc_cap(not_unit)).is_some());
    assert_eq!(lib_cap(&enc_cap(not_unit)), None);
    let empty = [bits(1.0), bits(0.0), bits(0.0), bits(-1.0)];
    assert!(lib_cap(&enc_cap(empty)).is_some());
    // Spec 007 FR-017 (option B): upstream keeps a radius of −5, the library decodes −1.
    let minus5 = [bits(1.0), bits(0.0), bits(0.0), bits(-5.0)];
    assert_eq!(
        ref_cap(&enc_cap(minus5)).map(|(c, _)| f64::from_bits(c[3])),
        Some(-5.0)
    );
    assert_eq!(
        lib_cap(&enc_cap(minus5)).map(|(c, _)| f64::from_bits(c[3])),
        Some(-1.0)
    );
    // Rect: versions 0 and 1 accepted, 2 rejected, invalid bounds rejected.
    let r = [bits(-0.5), bits(0.5), bits(1.0), bits(2.0)];
    let mut b = enc_rect(r);
    assert_eq!(ref_rect(&b), Some((r, 33)));
    b[0] = 0;
    assert_eq!(ref_rect(&b), Some((r, 33)));
    b[0] = 2;
    assert_eq!(ref_rect(&b), None);
    let bad = [bits(0.0), bits(2.0), bits(0.0), bits(1.0)];
    assert_eq!(ref_rect(&enc_rect(bad)), None);
    let empty_rect = [
        bits(1.0),
        bits(0.0),
        bits(core::f64::consts::PI),
        bits(-core::f64::consts::PI),
    ];
    assert!(rect_valid(empty_rect));
    // Cell: only valid ids.
    assert_eq!(lib_cell(&enc_cell_id(face3)), Some((face3, 8)));
    assert_eq!(lib_cell(&enc_cell_id(0)), None);
}
