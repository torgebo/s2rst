// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Differential tests for the decoding layer against the C++ reference implementation
//! (google/s2geometry), spec 003 FR-024 and Principle I.
//!
//! The vectors in `data/cpp_coding_vectors.csv` come from `tools/refgen-cpp/generate_coding`,
//! with the C++ S2 library as an independent oracle. They cover:
//! - the integer, cell-id, string and point vectors;
//! - the `CellId`, `Point`, `Cap`, `Rect` and `Cell` codecs;
//! - every prefix of every encoding;
//! - the survey's K2, K3 and K4 inputs.
//!
//! Every row must match C++, except the rows marked `deviation:reject`, where the library
//! documents that it rejects (FR-022): invalid `CellId`s and `Cap`s, and a NaN cap `length2`,
//! which C++ decodes as a full cap (D-004); and the rows marked
//! `deviation:negative-canonical`, where C++ keeps a cap's negative `length2` and the library
//! decodes it as −1 (spec 007 FR-017, option B).
//!
//! The `VARINT_DECODE` rows are checked inside the crate, by
//! `src/s2/encoding_tests.rs`, because `read_uvarint` is not public.

use s2rst::r1;
use s2rst::r3::Vector;
use s2rst::s1::{self, ChordAngle};
use s2rst::s2::encoded_s2cell_id_vector::{decode_s2cell_id_vector, encode_s2cell_id_vector};
use s2rst::s2::encoded_s2point_vector::{CodingHint, decode_s2point_vector, encode_s2point_vector};
use s2rst::s2::encoded_string_vector::{decode_string_vector, encode_string_vector};
use s2rst::s2::encoded_uint_vector::{
    decode_uint_vector_u32, decode_uint_vector_u64, encode_uint_vector_u32, encode_uint_vector_u64,
};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{Cap, Cell, CellId, Point, Rect};

const CSV: &str = include_str!("data/cpp_coding_vectors.csv");

struct Row {
    line: usize,
    kind: &'static str,
    input: &'static str,
    expected: &'static str,
    note: &'static str,
}

fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for (i, line) in CSV.lines().enumerate().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        assert_eq!(f.len(), 4, "line {}: {line:?}", i + 1);
        out.push(Row {
            line: i + 1,
            kind: f[0],
            input: f[1],
            expected: f[2],
            note: f[3],
        });
    }
    out
}

fn bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut out, x| {
        let _ = write!(out, "{x:02x}");
        out
    })
}

fn hex64(s: &str) -> u64 {
    u64::from_str_radix(s, 16).unwrap_or_else(|e| panic!("bad hex {s:?}: {e}"))
}

fn list(s: &str) -> Vec<&str> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(';').collect()
    }
}

fn bits_text(b: &[u64]) -> String {
    b.iter()
        .map(|x| format!("{x:016x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn bits_of(s: &str) -> Vec<f64> {
    s.split(':').map(|x| f64::from_bits(hex64(x))).collect()
}

fn point_bits(p: &Point) -> [u64; 3] {
    [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
}

fn point_of(s: &str) -> Point {
    let c = bits_of(s);
    Point(Vector {
        x: c[0],
        y: c[1],
        z: c[2],
    })
}

/// The decoder's answer, in the CSV's text form: `reject` or `ok|consumed|count|items`.
fn answer<T, E>(
    input: &[u8],
    decode: impl FnOnce(&mut &[u8]) -> Result<Vec<T>, E>,
    f: impl Fn(&T) -> String,
) -> String {
    let mut r = input;
    match decode(&mut r) {
        Err(_) => "reject".into(),
        Ok(v) => {
            let items: Vec<String> = v.iter().map(f).collect();
            format!(
                "ok|{}|{}|{}",
                input.len() - r.len(),
                items.len(),
                items.join(";")
            )
        }
    }
}

fn one<T: S2Decode>(input: &[u8], f: impl Fn(&T) -> String) -> String {
    answer(input, |r| T::decode(r).map(|v| vec![v]), f)
}

fn decode_row(r: &Row) -> String {
    let b = bytes(r.input);
    match r.kind {
        "UINT32_DECODE" => answer(&b, |r| decode_uint_vector_u32(r), |v| format!("{v:08x}")),
        "UINT64_DECODE" => answer(&b, |r| decode_uint_vector_u64(r), |v| format!("{v:016x}")),
        "CELLID_VECTOR_DECODE" => answer(
            &b,
            |r| decode_s2cell_id_vector(r),
            |c| format!("{:016x}", c.0),
        ),
        "STRING_VECTOR_DECODE" => answer(&b, |r| decode_string_vector(r), |s| hex(s)),
        "POINT_VECTOR_DECODE" => answer(
            &b,
            |r| decode_s2point_vector(r),
            |p| bits_text(&point_bits(p)),
        ),
        "CELLID_DECODE" => one::<CellId>(&b, |c| format!("{:016x}", c.0)),
        "POINT_DECODE" => one::<Point>(&b, |p| bits_text(&point_bits(p))),
        "CAP_DECODE" => one::<Cap>(&b, |c| {
            let p = point_bits(&c.center());
            bits_text(&[p[0], p[1], p[2], c.chord_radius().length2().to_bits()])
        }),
        "RECT_DECODE" => one::<Rect>(&b, |r| {
            bits_text(&[
                r.lat.lo.to_bits(),
                r.lat.hi.to_bits(),
                r.lng.lo.to_bits(),
                r.lng.hi.to_bits(),
            ])
        }),
        "CELL_DECODE" => one::<Cell>(&b, |c| format!("{:016x}", c.id().0)),
        k => panic!("line {}: unknown decode kind {k}", r.line),
    }
}

fn encode_row(r: &Row) -> String {
    let mut w = Vec::new();
    match r.kind {
        "UINT32_ENCODE" => {
            let v: Vec<u32> = list(r.input)
                .into_iter()
                .map(|x| u32::from_str_radix(x, 16).unwrap())
                .collect();
            encode_uint_vector_u32(&v, &mut w).unwrap();
        }
        "UINT64_ENCODE" => {
            let v: Vec<u64> = list(r.input).into_iter().map(hex64).collect();
            encode_uint_vector_u64(&v, &mut w).unwrap();
        }
        "CELLID_VECTOR_ENCODE" => {
            let v: Vec<CellId> = list(r.input)
                .into_iter()
                .map(|x| CellId(hex64(x)))
                .collect();
            encode_s2cell_id_vector(&v, &mut w).unwrap();
        }
        "STRING_VECTOR_ENCODE" => {
            let (count, items) = r.input.split_once('|').expect("count|items");
            let count: usize = count.parse().unwrap();
            let v: Vec<Vec<u8>> = if count == 0 {
                Vec::new()
            } else {
                items.split(';').map(bytes).collect()
            };
            let refs: Vec<&[u8]> = v.iter().map(Vec::as_slice).collect();
            encode_string_vector(&refs, &mut w).unwrap();
        }
        "POINT_VECTOR_ENCODE" => {
            let v: Vec<Point> = list(r.input).into_iter().map(point_of).collect();
            encode_s2point_vector(&v, CodingHint::Fast, &mut w).unwrap();
        }
        "CELLID_ENCODE" => CellId(hex64(r.input)).encode(&mut w).unwrap(),
        "POINT_ENCODE" => point_of(r.input).encode(&mut w).unwrap(),
        "CAP_ENCODE" => {
            let c = bits_of(r.input);
            Cap::from_center_chord_angle(
                Point(Vector {
                    x: c[0],
                    y: c[1],
                    z: c[2],
                }),
                ChordAngle::from_length2(c[3]),
            )
            .encode(&mut w)
            .unwrap();
        }
        "RECT_ENCODE" => {
            let c = bits_of(r.input);
            Rect {
                lat: r1::Interval { lo: c[0], hi: c[1] },
                lng: s1::Interval { lo: c[2], hi: c[3] },
            }
            .encode(&mut w)
            .unwrap();
        }
        "CELL_ENCODE" => Cell::from(CellId(hex64(r.input))).encode(&mut w).unwrap(),
        k => panic!("line {}: unknown encode kind {k}", r.line),
    }
    hex(&w)
}

#[test]
fn every_encoder_matches_cpp() {
    let mut failures = Vec::new();
    let mut n = 0;
    for r in rows().iter().filter(|r| r.kind.ends_with("_ENCODE")) {
        n += 1;
        let got = encode_row(r);
        if got != r.expected {
            failures.push(format!(
                "line {} {} {}: C++ {} but library {}",
                r.line, r.kind, r.input, r.expected, got
            ));
        }
    }
    assert!(n > 200, "only {n} encode rows");
    assert!(
        failures.is_empty(),
        "{} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn every_decoder_matches_cpp_or_its_documented_deviation() {
    let mut failures = Vec::new();
    let mut deviations = 0;
    let mut n = 0;
    for r in rows()
        .iter()
        .filter(|r| r.kind.ends_with("_DECODE") && r.kind != "VARINT_DECODE")
    {
        n += 1;
        let want = match r.note {
            "" => r.expected.to_string(),
            "deviation:reject" => {
                deviations += 1;
                "reject".to_string()
            }
            // Spec 007 FR-017 (option B): C++ keeps the negative length², the library's
            // `ChordAngle::from_length2` makes it −1.
            "deviation:negative-canonical" => {
                deviations += 1;
                let (cap, _) = r.expected.rsplit_once(':').expect("a cap's length²");
                format!("{cap}:{:016x}", (-1.0f64).to_bits())
            }
            other => panic!("line {}: unknown note {other:?}", r.line),
        };
        let got = decode_row(r);
        if got != want {
            failures.push(format!(
                "line {} {} {}: documented {} but library {}",
                r.line, r.kind, r.input, want, got
            ));
        }
    }
    assert!(n > 4000, "only {n} decode rows");
    assert!(
        failures.is_empty(),
        "{} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // invalid CellIds and Caps (K4) and D-004's NaN cap radius
    assert!(
        deviations >= 7,
        "the deviation rows are missing ({deviations})"
    );
}

/// K2 and K3 rows in particular: the survey's inputs are present and behave as documented.
#[test]
fn survey_inputs_are_covered() {
    let rows = rows();
    let find = |kind: &str, input: &str| rows.iter().find(|r| r.kind == kind && r.input == input);
    // K3: offsets [5, 2, 6] over "abcdef" decode as ["abcde", "", "cdef"], as upstream v0.14.
    let k3 = find("STRING_VECTOR_DECODE", "18050206616263646566").expect("the K3 row");
    assert_eq!(k3.expected, "ok|10|3|6162636465;;63646566");
    assert_eq!(decode_row(k3), k3.expected);
    // K2: the largest count with no data is rejected by both.
    let k2 = find(
        "UINT64_DECODE",
        &hex(&{
            let mut v = Vec::new();
            let mut x: u64 = 50_000_000 * 8;
            while x >= 0x80 {
                v.push((x & 0x7f) as u8 | 0x80);
                x >>= 7;
            }
            v.push(x as u8);
            v
        }),
    )
    .expect("the K2 row");
    assert_eq!(k2.expected, "reject");
    assert_eq!(decode_row(k2), "reject");
}
