// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Sanity tests for the reference model (`s2rst_verify::spec`) against the committed C++
//! oracle vectors (research R3, R16). The Kani proofs show that the implementation agrees
//! with the model for all inputs; these tests show, by sampling, that the model agrees with
//! upstream C++. Together: C++ oracle ⇔ model (sampled) ⇔ implementation (proved).

use s2rst::s2::CellId;
use s2rst_verify::spec::{self, Decomposed};

const CSV: &str = include_str!("../../core/tests/data/cpp_cellid_vectors.csv");

/// Lines of a named `# SECTION` (stops at the next `#` line or a blank line).
fn section_lines(name: &str) -> Vec<&'static str> {
    let header = format!("# {name}");
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in CSV.lines() {
        if line == header {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with('#') || line.is_empty() {
                break;
            }
            lines.push(line);
        }
    }
    assert!(!lines.is_empty(), "section {name} is empty or missing");
    lines
}

fn fields(line: &str) -> Vec<&str> {
    line.split(',').map(str::trim).collect()
}

fn u64f(s: &str) -> u64 {
    s.parse().unwrap()
}

fn decomposed(id: u64) -> Decomposed {
    spec::decompose(id).unwrap_or_else(|| panic!("{id} is not spec::valid"))
}

/// `CELLID_CHILDREN` (`id,c0,c1,c2,c3`): `compose(child(d, k)) == ck`.
#[test]
fn children_match_cpp() {
    let rows = section_lines("CELLID_CHILDREN");
    for line in &rows {
        let f = fields(line);
        let d = decomposed(u64f(f[0]));
        for k in 0..4u8 {
            let want = u64f(f[1 + usize::from(k)]);
            assert_eq!(
                spec::compose(spec::child(d, k)),
                want,
                "child {k} of {}",
                f[0]
            );
            assert!(spec::valid(want));
        }
    }
    eprintln!("CELLID_CHILDREN: {} rows", rows.len());
}

/// `CELLID_RANGE` (`id,range_min,range_max`): the leaf interval of the model.
#[test]
fn range_matches_cpp() {
    let rows = section_lines("CELLID_RANGE");
    for line in &rows {
        let f = fields(line);
        let (lo, hi) = spec::range(decomposed(u64f(f[0])));
        assert_eq!(spec::leaf_id(lo), u64f(f[1]), "range_min of {}", f[0]);
        assert_eq!(spec::leaf_id(hi), u64f(f[2]), "range_max of {}", f[0]);
    }
    eprintln!("CELLID_RANGE: {} rows", rows.len());
}

/// `CELLID_CONTAINS` (`a,b,contains,intersects`, as 0/1): the prefix relation on paths.
#[test]
fn contains_matches_cpp() {
    let rows = section_lines("CELLID_CONTAINS");
    for line in &rows {
        let f = fields(line);
        let a = decomposed(u64f(f[0]));
        let b = decomposed(u64f(f[1]));
        assert_eq!(
            spec::contains(a, b),
            f[2] == "1",
            "contains({}, {})",
            f[0],
            f[1]
        );
        assert_eq!(
            spec::contains(a, b) || spec::contains(b, a),
            f[3] == "1",
            "intersects({}, {})",
            f[0],
            f[1]
        );
    }
    eprintln!("CELLID_CONTAINS: {} rows", rows.len());
}

/// `CELLID_FACE_IJ` (`id,face,i,j,orientation`): the one-level-at-a-time Hilbert walk.
#[test]
fn face_ij_matches_cpp() {
    let rows = section_lines("CELLID_FACE_IJ");
    for line in &rows {
        let f = fields(line);
        let (face, i, j, o) = spec::face_ij_orientation(decomposed(u64f(f[0])));
        let want: (u8, u32, u32, u8) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
            f[4].parse().unwrap(),
        );
        assert_eq!((face, i, j, o), want, "ToFaceIJOrientation of {}", f[0]);
        // The leaf at (i, j) lies inside the cell.
        let leaf = decomposed(spec::leaf_from_face_ij(face, i, j));
        assert!(
            spec::contains(decomposed(u64f(f[0])), leaf),
            "leaf of {}",
            f[0]
        );
    }
    eprintln!("CELLID_FACE_IJ: {} rows", rows.len());
}

/// Hand-written cases for the text model and the past-the-end value.
#[test]
fn hand_written_cases() {
    assert_eq!(
        spec::parse_token(b"+1"),
        spec::NONE,
        "C++ FromToken rejects a sign"
    );
    assert_eq!(spec::parse_token(b"X"), spec::NONE);
    assert_eq!(
        spec::parse_token(b""),
        spec::NONE,
        "C++ FromToken gives S2CellId(0)"
    );
    assert_eq!(spec::parse_token(b"1"), 1u64 << 60);
    // 2 = 0b10 has its lowest set bit at an odd position: invalid.
    let (s, n) = spec::debug_string(2);
    assert_eq!(&s[..n], b"Invalid: 0000000000000002");
    // 5 = 0b101 is the valid face-0 leaf whose last digit is 2.
    let (s, n) = spec::debug_string(5);
    assert_eq!(&s[..n], b"0/000000000000000000000000000002");
    assert_eq!(spec::end(0), CellId::end(0u8).0);
}

/// The model's own round trips, on a sweep of decompositions.
#[test]
fn model_round_trips() {
    for face in 0..6u8 {
        for level in 0..=30u8 {
            let n = spec::pow4(u32::from(level));
            for path in [0, n / 3, n / 2, n - 1] {
                let d = Decomposed { face, level, path };
                let x = spec::compose(d);
                assert_eq!(spec::decompose(x), Some(d));
                let (t, len) = spec::token(x);
                assert_eq!(spec::parse_token(&t[..len]), x);
                let (s, len) = spec::debug_string(x);
                assert_eq!(spec::parse_debug_string(&s[..len]), Some(x));
                assert_eq!(spec::from_position(level, spec::position(d)), x);
            }
        }
    }
}

/// Decodes the hex-encoded bytes of a text field (empty field → empty bytes).
fn hex_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// `CELLID_FROM_TOKEN` (`token_hex,id`): `spec::parse_token` is C++ `FromToken` (R16).
#[test]
fn parse_token_matches_cpp() {
    let rows = section_lines("CELLID_FROM_TOKEN");
    for line in &rows {
        let (hex, id) = line.split_once(',').unwrap();
        let bytes = hex_bytes(hex.trim());
        assert_eq!(
            spec::parse_token(&bytes),
            u64f(id.trim()),
            "token {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }
    eprintln!("CELLID_FROM_TOKEN: {} rows", rows.len());
}

/// `CELLID_FROM_DEBUG_STRING` (`string_hex,id`, 0 for None): `spec::parse_debug_string` is
/// C++ `FromDebugString` (R16).
#[test]
fn parse_debug_string_matches_cpp() {
    let rows = section_lines("CELLID_FROM_DEBUG_STRING");
    for line in &rows {
        let (hex, id) = line.split_once(',').unwrap();
        let bytes = hex_bytes(hex.trim());
        assert_eq!(
            spec::parse_debug_string(&bytes).unwrap_or(spec::NONE),
            u64f(id.trim()),
            "string {:?}",
            String::from_utf8_lossy(&bytes)
        );
    }
    eprintln!("CELLID_FROM_DEBUG_STRING: {} rows", rows.len());
}

/// `CELLID_TO_STRING` (`id,string`): `spec::debug_string` is C++ `ToString` (R16).
#[test]
fn debug_string_matches_cpp() {
    let rows = section_lines("CELLID_TO_STRING");
    for line in &rows {
        let (id, want) = line.split_once(',').unwrap();
        let (s, n) = spec::debug_string(u64f(id.trim()));
        assert_eq!(&s[..n], want.as_bytes(), "id {id}");
    }
    eprintln!("CELLID_TO_STRING: {} rows", rows.len());
}

/// `spec::valid_by_marker` is `spec::valid`: on every lowest-set-bit position and face, with
/// the bits above the marker all clear, all set and in a fixed pattern, and on the C++ vectors'
/// ids.
#[test]
fn valid_by_marker_is_valid() {
    let mut n = 0;
    for tz in 0..64u32 {
        for face in 0..8u64 {
            for above in [0u64, u64::MAX, 0x5a5a_5a5a_5a5a_5a5a] {
                let marker = 1u64 << tz;
                let high = (above << 1) << tz; // bits above the marker
                let x = (face << 61) | (high & !(7u64 << 61)) | marker;
                assert_eq!(spec::valid_by_marker(x), spec::valid(x), "{x:#x}");
                n += 1;
            }
        }
    }
    assert_eq!(spec::valid_by_marker(0), spec::valid(0));
    for line in section_lines("CELLID_CHILDREN") {
        let id = u64f(fields(line)[0]);
        assert!(spec::valid_by_marker(id) && spec::valid(id));
    }
    assert!(n > 1000);
}
