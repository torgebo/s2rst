// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Tested agreement between the restated codecs and the library (spec 003 FR-011;
//! contracts/deductive-conventions.md, "Links to the library").
//!
//! The Verus proofs hold for the restated copies in `src/`. These tests tie each copy to the
//! library function it mirrors:
//! - **decoders:** the library on `&mut &b[..]` and the restated decoder on `(&b, 0)` agree
//!   on acceptance, on the value and on the bytes consumed. The inputs are random byte
//!   strings up to 4 KiB, and library encodings of random values with a byte changed, cut
//!   short or followed by more bytes;
//! - **encoders:** both write the same bytes for random vectors up to 10,000 elements
//!   (strings up to 1 KiB);
//! - **the C++ vectors:** every row of `core/tests/data/cpp_coding_vectors.csv` for these
//!   codecs goes through both, and both must give the C++ answer.
//!
//! The library's `read_uvarint` and `write_uvarint` are crate-private. The restated varint is
//! linked to them through the `VARINT_DECODE` rows, which the library's own
//! `encoding_tests.rs` checks against the same C++ answers, and through the vector headers,
//! which both sides read and write with their varint.

use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use s2rst::s2::CellId;
use s2rst::s2::encoded_s2cell_id_vector as lib_cid;
use s2rst::s2::encoded_string_vector as lib_str;
use s2rst::s2::encoded_uint_vector as lib_uv;
use s2rst_deductive::{cellid_vector, fixed_width, string_vector, uint_vector, varint};

const CSV: &str = include_str!("../../../core/tests/data/cpp_coding_vectors.csv");

/// The decoders' common answer: `None` for a rejection, else the values and the bytes
/// consumed.
type Answer<T> = Option<(Vec<T>, usize)>;

fn lib_answer<T, E>(b: &[u8], decode: impl FnOnce(&mut &[u8]) -> Result<Vec<T>, E>) -> Answer<T> {
    let mut r = b;
    decode(&mut r).ok().map(|v| (v, b.len() - r.len()))
}

fn agree<T: PartialEq + std::fmt::Debug>(
    what: &str,
    b: &[u8],
    lib: Answer<T>,
    restated: Answer<T>,
) -> TestResult {
    if lib == restated {
        TestResult::passed()
    } else {
        TestResult::error(format!(
            "{what} on {} bytes {:02x?}: library {lib:?}, restated {restated:?}",
            b.len(),
            &b[..b.len().min(64)]
        ))
    }
}

fn quickcheck<A: Arbitrary + std::fmt::Debug>(prop: fn(A) -> TestResult) {
    QuickCheck::new().quickcheck(prop);
}

/// A random byte string up to 4 KiB.
#[derive(Clone, Debug)]
struct Bytes(Vec<u8>);

impl Arbitrary for Bytes {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = (usize::from(u16::arbitrary(g)) % 4097) >> (u8::arbitrary(g) % 13);
        Bytes((0..n).map(|_| u8::arbitrary(g)).collect())
    }
}

/// A change to a valid encoding: one byte flipped, the end cut off, bytes appended.
#[derive(Clone, Debug)]
struct Damage {
    flip: Option<(usize, u8)>,
    cut: Option<usize>,
    tail: Vec<u8>,
}

impl Arbitrary for Damage {
    fn arbitrary(g: &mut Gen) -> Self {
        let pick = u8::arbitrary(g) % 4;
        let tail_len = usize::from(u8::arbitrary(g) % 17);
        Damage {
            flip: (pick & 1 == 1).then(|| (usize::arbitrary(g), u8::arbitrary(g).max(1))),
            cut: (pick & 2 == 2).then(|| usize::arbitrary(g)),
            tail: (0..tail_len).map(|_| u8::arbitrary(g)).collect(),
        }
    }
}

impl Damage {
    fn apply(&self, mut b: Vec<u8>) -> Vec<u8> {
        if let Some((at, x)) = self.flip
            && !b.is_empty()
        {
            let at = at % b.len();
            b[at] ^= x;
        }
        if let Some(at) = self.cut {
            b.truncate(at % (b.len() + 1));
        }
        b.extend_from_slice(&self.tail);
        b
    }
}

/// A count up to 10,000, skewed towards small ones.
fn count(g: &mut Gen) -> usize {
    (usize::from(u16::arbitrary(g)) % 10_001) >> (u8::arbitrary(g) % 15)
}

/// A value of at most `w` bytes, sometimes fewer.
fn sized(g: &mut Gen, w: u32) -> u64 {
    let w = if bool::arbitrary(g) {
        w
    } else {
        u32::from(u8::arbitrary(g)) % (w + 1)
    };
    if w == 0 {
        0
    } else {
        u64::arbitrary(g) >> (64 - 8 * w)
    }
}

/// A vector of `u64`s whose widths vary from vector to vector.
#[derive(Clone, Debug)]
struct U64s(Vec<u64>);

impl Arbitrary for U64s {
    fn arbitrary(g: &mut Gen) -> Self {
        let w = u32::from(u8::arbitrary(g)) % 9;
        let n = count(g);
        U64s((0..n).map(|_| sized(g, w)).collect())
    }
}

#[derive(Clone, Debug)]
struct U32s(Vec<u32>);

impl Arbitrary for U32s {
    fn arbitrary(g: &mut Gen) -> Self {
        let w = u32::from(u8::arbitrary(g)) % 5;
        let n = count(g);
        U32s(
            (0..n)
                .map(|_| u32::try_from(sized(g, w)).unwrap())
                .collect(),
        )
    }
}

/// Cell identifiers the encoder can do something with: a shared prefix, a common level (or a
/// level per identifier), and sometimes raw bits.
#[derive(Clone, Debug)]
struct Ids(Vec<u64>);

impl Arbitrary for Ids {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = count(g);
        let level = u64::from(u8::arbitrary(g) % 31);
        let mixed = u8::arbitrary(g) % 4 == 0;
        let raw = u8::arbitrary(g) % 8 == 0;
        let prefix = u64::arbitrary(g);
        let spread = u32::from(u8::arbitrary(g)) % 9;
        let mask = if spread == 8 {
            u64::MAX
        } else {
            (1u64 << (8 * spread)) - 1
        };
        Ids((0..n)
            .map(|_| {
                if raw {
                    return u64::arbitrary(g);
                }
                let l = if mixed {
                    u64::from(u8::arbitrary(g) % 31)
                } else {
                    level
                };
                let lsb = 1u64 << (2 * (30 - l));
                let x = (prefix & !mask) | (u64::arbitrary(g) & mask);
                (x & !(2 * lsb - 1)) | lsb
            })
            .collect())
    }
}

/// Strings up to 1 KiB each.
#[derive(Clone, Debug)]
struct Strings(Vec<Vec<u8>>);

impl Arbitrary for Strings {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = count(g);
        Strings(
            (0..n)
                .map(|_| {
                    let len = (usize::from(u16::arbitrary(g)) % 1025) >> (u8::arbitrary(g) % 11);
                    (0..len).map(|_| u8::arbitrary(g)).collect()
                })
                .collect(),
        )
    }
}

// ---- the decoders, on random bytes --------------------------------------------------------

fn dec_u64(b: &[u8]) -> TestResult {
    agree(
        "decode_uint_vector_u64",
        b,
        lib_answer(b, |r| lib_uv::decode_uint_vector_u64(r)),
        uint_vector::decode_uint_vector_u64(b, 0),
    )
}

fn dec_u32(b: &[u8]) -> TestResult {
    agree(
        "decode_uint_vector_u32",
        b,
        lib_answer(b, |r| lib_uv::decode_uint_vector_u32(r)),
        uint_vector::decode_uint_vector_u32(b, 0),
    )
}

fn dec_ids(b: &[u8]) -> TestResult {
    let lib = lib_answer(b, |r| lib_cid::decode_s2cell_id_vector(r))
        .map(|(v, n)| (v.iter().map(|c| c.0).collect(), n));
    agree(
        "decode_s2cell_id_vector",
        b,
        lib,
        cellid_vector::decode_s2cell_id_vector(b, 0),
    )
}

fn dec_strings(b: &[u8]) -> TestResult {
    agree(
        "decode_string_vector",
        b,
        lib_answer(b, |r| lib_str::decode_string_vector(r)),
        string_vector::decode_string_vector(b, 0),
    )
}

#[test]
fn decoders_agree_on_random_bytes() {
    quickcheck(|b: Bytes| dec_u64(&b.0));
    quickcheck(|b: Bytes| dec_u32(&b.0));
    quickcheck(|b: Bytes| dec_ids(&b.0));
    quickcheck(|b: Bytes| dec_strings(&b.0));
}

#[test]
fn fixed_width_helpers_agree() {
    quickcheck(|(b, len): (Bytes, u8)| {
        let len = usize::from(len % 9);
        let mut r = b.0.as_slice();
        let lib = lib_uv::decode_uint_with_length(&mut r, len)
            .ok()
            .map(|x| (x, b.0.len() - r.len()));
        let restated = fixed_width::decode_uint_with_length(&b.0, 0, len);
        if lib != restated {
            return TestResult::error(format!(
                "decode_uint_with_length({len}): library {lib:?}, restated {restated:?}"
            ));
        }
        if b.0.len() >= len
            && lib_uv::get_uint_with_length(&b.0, len)
                != fixed_width::get_uint_with_length(&b.0, len)
        {
            return TestResult::error(format!("get_uint_with_length({len}) differs"));
        }
        TestResult::passed()
    });
    quickcheck(|(x, len): (u64, u8)| {
        let len = usize::from(len % 9);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        lib_uv::encode_uint_with_length(&mut a, x, len).unwrap();
        fixed_width::encode_uint_with_length(&mut b, x, len);
        TestResult::from_bool(a == b)
    });
}

// ---- the decoders, on damaged encodings ---------------------------------------------------

#[test]
fn decoders_agree_on_damaged_encodings() {
    quickcheck(|(v, d): (U64s, Damage)| {
        let mut w = Vec::new();
        lib_uv::encode_uint_vector_u64(&v.0, &mut w).unwrap();
        dec_u64(&d.apply(w))
    });
    quickcheck(|(v, d): (U32s, Damage)| {
        let mut w = Vec::new();
        lib_uv::encode_uint_vector_u32(&v.0, &mut w).unwrap();
        dec_u32(&d.apply(w))
    });
    quickcheck(|(v, d): (Ids, Damage)| {
        let ids: Vec<CellId> = v.0.iter().copied().map(CellId).collect();
        let mut w = Vec::new();
        lib_cid::encode_s2cell_id_vector(&ids, &mut w).unwrap();
        dec_ids(&d.apply(w))
    });
    quickcheck(|(v, d): (Strings, Damage)| {
        let refs: Vec<&[u8]> = v.0.iter().map(Vec::as_slice).collect();
        let mut w = Vec::new();
        lib_str::encode_string_vector(&refs, &mut w).unwrap();
        dec_strings(&d.apply(w))
    });
}

// ---- the encoders ---------------------------------------------------------------------------

fn same_bytes(what: &str, lib: &[u8], restated: &[u8]) -> TestResult {
    if lib == restated {
        TestResult::passed()
    } else {
        TestResult::error(format!(
            "{what}: library {:02x?}, restated {:02x?}",
            &lib[..lib.len().min(64)],
            &restated[..restated.len().min(64)]
        ))
    }
}

#[test]
fn encoders_write_the_same_bytes() {
    quickcheck(|v: U64s| {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        lib_uv::encode_uint_vector_u64(&v.0, &mut a).unwrap();
        let _ = uint_vector::encode_uint_vector_u64(&v.0, &mut b);
        same_bytes("encode_uint_vector_u64", &a, &b)
    });
    quickcheck(|v: U32s| {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        lib_uv::encode_uint_vector_u32(&v.0, &mut a).unwrap();
        let _ = uint_vector::encode_uint_vector_u32(&v.0, &mut b);
        same_bytes("encode_uint_vector_u32", &a, &b)
    });
    quickcheck(|v: Ids| {
        let ids: Vec<CellId> = v.0.iter().copied().map(CellId).collect();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        lib_cid::encode_s2cell_id_vector(&ids, &mut a).unwrap();
        cellid_vector::encode_s2cell_id_vector(&v.0, &mut b);
        same_bytes("encode_s2cell_id_vector", &a, &b)
    });
    quickcheck(|v: Strings| {
        let refs: Vec<&[u8]> = v.0.iter().map(Vec::as_slice).collect();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        lib_str::encode_string_vector(&refs, &mut a).unwrap();
        let _ = string_vector::encode_string_vector(&v.0, &mut b);
        same_bytes("encode_string_vector", &a, &b)
    });
}

/// The generators reach the sizes the contract names, and the damaged encodings exercise
/// acceptance as well as rejection, so the agreement above is not vacuous.
#[test]
fn the_inputs_cover_the_stated_ranges() {
    let g = &mut Gen::new(100);
    let mut longest = [0usize; 5];
    let mut accepted = [0usize; 4];
    for _ in 0..2_000 {
        longest[0] = longest[0].max(Bytes::arbitrary(g).0.len());
        let v = U64s::arbitrary(g).0;
        longest[1] = longest[1].max(v.len());
        let ids = Ids::arbitrary(g).0;
        longest[2] = longest[2].max(ids.len());
        let strings = Strings::arbitrary(g).0;
        longest[3] = longest[3].max(strings.len());
        longest[4] = longest[4].max(strings.iter().map(Vec::len).max().unwrap_or(0));
        let d = Damage::arbitrary(g);
        let mut w = Vec::new();
        lib_uv::encode_uint_vector_u64(&v, &mut w).unwrap();
        accepted[0] += usize::from(uint_vector::decode_uint_vector_u64(&d.apply(w), 0).is_some());
        let mut w = Vec::new();
        let cids: Vec<CellId> = ids.iter().copied().map(CellId).collect();
        lib_cid::encode_s2cell_id_vector(&cids, &mut w).unwrap();
        accepted[1] +=
            usize::from(cellid_vector::decode_s2cell_id_vector(&d.apply(w), 0).is_some());
        let mut w = Vec::new();
        let _ = string_vector::encode_string_vector(&strings, &mut w);
        accepted[2] += usize::from(string_vector::decode_string_vector(&d.apply(w), 0).is_some());
        accepted[3] +=
            usize::from(uint_vector::decode_uint_vector_u64(&Bytes::arbitrary(g).0, 0).is_some());
    }
    assert!(
        longest[0] > 4_000,
        "random bytes reach {} bytes",
        longest[0]
    );
    assert!(
        longest[1] > 9_000 && longest[2] > 9_000,
        "vectors reach {} and {} elements",
        longest[1],
        longest[2]
    );
    assert!(
        longest[3] > 9_000 && longest[4] > 1_000,
        "string vectors reach {} strings of {} bytes",
        longest[3],
        longest[4]
    );
    for (what, n) in [
        "u64 vector",
        "cell-id vector",
        "string vector",
        "random u64 vector",
    ]
    .iter()
    .zip(accepted)
    {
        assert!(
            (100..1_900).contains(&n),
            "{what}: {n} of 2,000 damaged inputs accepted"
        );
    }
}

// ---- the C++ vectors ------------------------------------------------------------------------

fn bytes(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn list(s: &str) -> Vec<&str> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(';').collect()
    }
}

/// An answer in the CSV's text form: `reject` or `ok|consumed|count|items`.
fn text<T>(a: &Answer<T>, f: impl Fn(&T) -> String) -> String {
    match a {
        None => "reject".into(),
        Some((v, n)) => format!(
            "ok|{n}|{}|{}",
            v.len(),
            v.iter().map(f).collect::<Vec<_>>().join(";")
        ),
    }
}

#[test]
fn every_cpp_row_goes_through_both() {
    let mut failures = Vec::new();
    let mut rows = 0;
    for (i, line) in CSV.lines().enumerate().skip(1) {
        let f: Vec<&str> = line.split(',').collect();
        let (kind, input, expected) = (f[0], f[1], f[2]);
        let mut check = |lib: Option<String>, restated: String| {
            rows += 1;
            if lib.as_ref().is_some_and(|l| *l != restated) || restated != expected {
                failures.push(format!(
                    "line {}: {kind} {input}: C++ {expected}, library {lib:?}, restated {restated}",
                    i + 1
                ));
            }
        };
        match kind {
            "VARINT_DECODE" => {
                let b = bytes(input);
                let a = varint::read_uvarint(&b, 0).map(|(x, n)| (vec![x], n));
                check(None, text(&a, |x| format!("{x:016x}")));
            }
            "UINT64_DECODE" => {
                let b = bytes(input);
                let f = |x: &u64| format!("{x:016x}");
                check(
                    Some(text(
                        &lib_answer(&b, |r| lib_uv::decode_uint_vector_u64(r)),
                        f,
                    )),
                    text(&uint_vector::decode_uint_vector_u64(&b, 0), f),
                );
            }
            "UINT32_DECODE" => {
                let b = bytes(input);
                let f = |x: &u32| format!("{x:08x}");
                check(
                    Some(text(
                        &lib_answer(&b, |r| lib_uv::decode_uint_vector_u32(r)),
                        f,
                    )),
                    text(&uint_vector::decode_uint_vector_u32(&b, 0), f),
                );
            }
            "CELLID_VECTOR_DECODE" => {
                let b = bytes(input);
                let lib = lib_answer(&b, |r| lib_cid::decode_s2cell_id_vector(r))
                    .map(|(v, n)| (v.iter().map(|c| c.0).collect::<Vec<_>>(), n));
                let f = |x: &u64| format!("{x:016x}");
                check(
                    Some(text(&lib, f)),
                    text(&cellid_vector::decode_s2cell_id_vector(&b, 0), f),
                );
            }
            "STRING_VECTOR_DECODE" => {
                let b = bytes(input);
                let f = |s: &Vec<u8>| hex(s);
                check(
                    Some(text(
                        &lib_answer(&b, |r| lib_str::decode_string_vector(r)),
                        f,
                    )),
                    text(&string_vector::decode_string_vector(&b, 0), f),
                );
            }
            "UINT64_ENCODE" => {
                let v: Vec<u64> = list(input)
                    .into_iter()
                    .map(|x| u64::from_str_radix(x, 16).unwrap())
                    .collect();
                let (mut a, mut b) = (Vec::new(), Vec::new());
                lib_uv::encode_uint_vector_u64(&v, &mut a).unwrap();
                let _ = uint_vector::encode_uint_vector_u64(&v, &mut b);
                check(Some(hex(&a)), hex(&b));
            }
            "UINT32_ENCODE" => {
                let v: Vec<u32> = list(input)
                    .into_iter()
                    .map(|x| u32::from_str_radix(x, 16).unwrap())
                    .collect();
                let (mut a, mut b) = (Vec::new(), Vec::new());
                lib_uv::encode_uint_vector_u32(&v, &mut a).unwrap();
                let _ = uint_vector::encode_uint_vector_u32(&v, &mut b);
                check(Some(hex(&a)), hex(&b));
            }
            "CELLID_VECTOR_ENCODE" => {
                let v: Vec<u64> = list(input)
                    .into_iter()
                    .map(|x| u64::from_str_radix(x, 16).unwrap())
                    .collect();
                let ids: Vec<CellId> = v.iter().copied().map(CellId).collect();
                let (mut a, mut b) = (Vec::new(), Vec::new());
                lib_cid::encode_s2cell_id_vector(&ids, &mut a).unwrap();
                cellid_vector::encode_s2cell_id_vector(&v, &mut b);
                check(Some(hex(&a)), hex(&b));
            }
            "STRING_VECTOR_ENCODE" => {
                let (n, items) = input.split_once('|').expect("count|items");
                let v: Vec<Vec<u8>> = if n == "0" {
                    Vec::new()
                } else {
                    items.split(';').map(bytes).collect()
                };
                let refs: Vec<&[u8]> = v.iter().map(Vec::as_slice).collect();
                let (mut a, mut b) = (Vec::new(), Vec::new());
                lib_str::encode_string_vector(&refs, &mut a).unwrap();
                let _ = string_vector::encode_string_vector(&v, &mut b);
                check(Some(hex(&a)), hex(&b));
            }
            _ => {}
        }
    }
    assert!(rows > 3_900, "only {rows} rows for the restated codecs");
    assert!(
        failures.is_empty(),
        "{} rows differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
