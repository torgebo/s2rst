// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The reference description (`s2rst_verify::wire_spec`) against every row of the committed
//! C++ oracle vectors (spec 003 FR-007, FR-024, P-PAR-01; data-model §10).
//!
//! - **Upstream:** every `ref_*` decoder and `enc_*` encoder gives C++'s answer on every row.
//! - **Library:** every `lib_*` decoder gives the library's documented answer on every row:
//!   - C++'s answer, where the row's note is empty;
//!   - `reject`, where it is `deviation:reject`.
//!
//! The Kani proofs show the implementation agrees with the `lib_*` description within their
//! bounds; this test shows the description agrees with C++. Together: C++ ⇔ description ⇔
//! implementation.

use s2rst_verify::wire_spec::{self, PointVectorAnswer};

const CSV: &str = include_str!("../../core/tests/data/cpp_coding_vectors.csv");

struct Row<'a> {
    line: usize,
    kind: &'a str,
    input: &'a str,
    expected: &'a str,
    note: &'a str,
}

fn rows() -> Vec<Row<'static>> {
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
    assert!(
        out.len() > 4000,
        "the CSV looks truncated ({} rows)",
        out.len()
    );
    out
}

fn bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    let mut out = String::new();
    for x in b {
        out.push_str(&format!("{x:02x}"));
    }
    out
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

fn ok(consumed: usize, items: &[String]) -> String {
    format!("ok|{consumed}|{}|{}", items.len(), items.join(";"))
}

fn answer<T>(r: Option<(Vec<T>, usize)>, f: impl Fn(&T) -> String) -> String {
    match r {
        None => "reject".into(),
        Some((v, n)) => ok(n, &v.iter().map(f).collect::<Vec<_>>()),
    }
}

fn one<T>(r: Option<(T, usize)>, f: impl Fn(&T) -> String) -> String {
    match r {
        None => "reject".into(),
        Some((v, n)) => ok(n, &[f(&v)]),
    }
}

fn bits_text(b: &[u64]) -> String {
    b.iter()
        .map(|x| format!("{x:016x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn bits_of<const N: usize>(s: &str) -> [u64; N] {
    let v: Vec<u64> = s.split(':').map(hex64).collect();
    v.try_into()
        .unwrap_or_else(|_| panic!("expected {N} coordinates in {s:?}"))
}

fn point_answer(a: PointVectorAnswer) -> String {
    match a {
        PointVectorAnswer::Reject => "reject".into(),
        PointVectorAnswer::Compact => "compact".into(),
        PointVectorAnswer::Points(v, n) => {
            ok(n, &v.iter().map(|p| bits_text(p)).collect::<Vec<_>>())
        }
    }
}

/// The upstream answer of the reference description for one row.
fn upstream(r: &Row) -> String {
    if r.kind.ends_with("_ENCODE") {
        return encode(r);
    }
    let b = bytes(r.input);
    match r.kind {
        "VARINT_DECODE" => one(wire_spec::ref_varint(&b), |v| format!("{v:016x}")),
        "UINT32_DECODE" => answer(wire_spec::ref_uint_vector(&b, 4), |v| format!("{v:08x}")),
        "UINT64_DECODE" => answer(wire_spec::ref_uint_vector(&b, 8), |v| format!("{v:016x}")),
        "CELLID_VECTOR_DECODE" => answer(wire_spec::ref_cellid_vector(&b), |v| format!("{v:016x}")),
        "STRING_VECTOR_DECODE" => answer(wire_spec::ref_string_vector(&b), |s| hex(s)),
        "POINT_VECTOR_DECODE" => point_answer(wire_spec::ref_point_vector(&b)),
        "CELLID_DECODE" => one(wire_spec::ref_cell_id(&b), |v| format!("{v:016x}")),
        "POINT_DECODE" => one(wire_spec::ref_point(&b), |p| bits_text(p)),
        "CAP_DECODE" => one(wire_spec::ref_cap(&b), |c| bits_text(c)),
        "RECT_DECODE" => one(wire_spec::ref_rect(&b), |r| bits_text(r)),
        // Upstream S2Cell::Decode on an invalid id is undefined; the generator emits valid ids
        // only, where upstream and the library agree.
        "CELL_DECODE" => one(wire_spec::lib_cell(&b), |v| format!("{v:016x}")),
        k => panic!("line {}: unknown kind {k}", r.line),
    }
}

/// The canonical encoding of an ENCODE row's input.
fn encode(r: &Row) -> String {
    let out = match r.kind {
        "UINT32_ENCODE" => {
            let v: Vec<u64> = list(r.input).into_iter().map(hex64).collect();
            wire_spec::enc_uint_vector(&v, 4)
        }
        "UINT64_ENCODE" => {
            let v: Vec<u64> = list(r.input).into_iter().map(hex64).collect();
            wire_spec::enc_uint_vector(&v, 8)
        }
        "CELLID_VECTOR_ENCODE" => {
            let v: Vec<u64> = list(r.input).into_iter().map(hex64).collect();
            wire_spec::enc_cellid_vector(&v)
        }
        "STRING_VECTOR_ENCODE" => {
            let (count, items) = r.input.split_once('|').expect("count|items");
            let count: usize = count.parse().unwrap();
            let v: Vec<Vec<u8>> = if count == 0 {
                Vec::new()
            } else {
                items.split(';').map(bytes).collect()
            };
            assert_eq!(v.len(), count, "line {}", r.line);
            wire_spec::enc_string_vector(&v)
        }
        "POINT_VECTOR_ENCODE" => {
            let v: Vec<[u64; 3]> = list(r.input).into_iter().map(bits_of::<3>).collect();
            wire_spec::enc_point_vector_fast(&v)
        }
        "CELLID_ENCODE" | "CELL_ENCODE" => wire_spec::enc_cell_id(hex64(r.input)),
        "POINT_ENCODE" => wire_spec::enc_point(bits_of::<3>(r.input)),
        "CAP_ENCODE" => wire_spec::enc_cap(bits_of::<4>(r.input)),
        "RECT_ENCODE" => wire_spec::enc_rect(bits_of::<4>(r.input)),
        k => panic!("line {}: unknown kind {k}", r.line),
    };
    hex(&out)
}

/// The library's documented answer for one DECODE row, or `None` for an ENCODE row.
fn library(r: &Row) -> Option<String> {
    if r.kind.ends_with("_ENCODE") {
        return None;
    }
    let b = bytes(r.input);
    Some(match r.kind {
        "VARINT_DECODE" => one(wire_spec::lib_varint(&b), |v| format!("{v:016x}")),
        "UINT32_DECODE" => answer(wire_spec::lib_uint_vector(&b, 4), |v| format!("{v:08x}")),
        "UINT64_DECODE" => answer(wire_spec::lib_uint_vector(&b, 8), |v| format!("{v:016x}")),
        "CELLID_VECTOR_DECODE" => answer(wire_spec::lib_cellid_vector(&b), |v| format!("{v:016x}")),
        "STRING_VECTOR_DECODE" => answer(wire_spec::lib_string_vector(&b), |s| hex(s)),
        "POINT_VECTOR_DECODE" => point_answer(wire_spec::lib_point_vector(&b)),
        "CELLID_DECODE" => one(wire_spec::lib_cell_id(&b), |v| format!("{v:016x}")),
        "POINT_DECODE" => one(wire_spec::ref_point(&b), |p| bits_text(p)),
        "CAP_DECODE" => one(wire_spec::lib_cap(&b), |c| bits_text(c)),
        "RECT_DECODE" => one(wire_spec::ref_rect(&b), |r| bits_text(r)),
        "CELL_DECODE" => one(wire_spec::lib_cell(&b), |v| format!("{v:016x}")),
        k => panic!("line {}: unknown kind {k}", r.line),
    })
}

#[test]
fn reference_description_agrees_with_cpp_on_every_row() {
    let mut failures = Vec::new();
    for r in rows() {
        let got = upstream(&r);
        if got != r.expected {
            failures.push(format!(
                "line {} {} {}: C++ {} but description {}",
                r.line, r.kind, r.input, r.expected, got
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn library_description_gives_the_documented_answer_on_every_row() {
    let mut failures = Vec::new();
    let mut deviations = 0;
    for r in rows() {
        let Some(got) = library(&r) else { continue };
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
            n => panic!("line {}: unknown note {n:?}", r.line),
        };
        if got != want {
            failures.push(format!(
                "line {} {} {}: documented {} but description {}",
                r.line, r.kind, r.input, want, got
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // The K4 rows (invalid CellIds and Caps) and D-004's NaN cap radius.
    assert!(
        deviations >= 7,
        "expected the deviation rows, found {deviations}"
    );
}
