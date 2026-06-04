// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Robustness / fuzz-style tests for the byte decoders.
//!
//! The decoders accept untrusted input (`&mut dyn Read`) and return
//! `io::Result`, so the contract is: for ANY input they must return `Ok`/`Err`
//! and never panic, hang, or allocate unboundedly — and valid input must
//! round-trip through encode→decode.
//!
//! Strategy: round-trip valid data, then attack each decoder with (a) every
//! truncation of a valid encoding, (b) mutations of valid encodings (bit/byte
//! flips, insertions, deletions — these stay near the valid manifold), and
//! (c) short fully-random inputs. Mutation/random inputs are kept short so a
//! decoded length field cannot drive a multi-gigabyte allocation during the run.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use s2rst::s2::encoded_s2cell_id_vector::{decode_s2cell_id_vector, encode_s2cell_id_vector};
use s2rst::s2::encoded_s2point_vector::{CodingHint, decode_s2point_vector, encode_s2point_vector};
use s2rst::s2::encoded_string_vector::{decode_string_vector, encode_string_vector};
use s2rst::s2::encoded_uint_vector::{
    decode_uint_vector_u32, decode_uint_vector_u64, encode_uint_vector_u32, encode_uint_vector_u64,
};
use s2rst::s2::{CellId, LatLng, Point};

// ── input generators ───────────────────────────────────────────────────────

fn rng(seed: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

fn rand_point(r: &mut ChaCha8Rng) -> Point {
    let lat = r.gen_range(-90.0..90.0);
    let lng = r.gen_range(-180.0..180.0);
    LatLng::from_degrees(lat, lng).to_point()
}

fn rand_points(r: &mut ChaCha8Rng, n: usize) -> Vec<Point> {
    (0..n).map(|_| rand_point(r)).collect()
}

fn rand_cell_ids(r: &mut ChaCha8Rng, n: usize) -> Vec<CellId> {
    (0..n)
        .map(|_| {
            let leaf = CellId::from_point(&rand_point(r));
            // Vary the level so the delta encoding sees a range of cells.
            let level: u8 = r.gen_range(0..=30);
            leaf.parent_at_level(level)
        })
        .collect()
}

// ── encode helpers (valid bytes for each format) ────────────────────────────

fn enc_u32_vec(v: &[u32]) -> Vec<u8> {
    let mut b = Vec::new();
    encode_uint_vector_u32(v, &mut b).unwrap();
    b
}
fn enc_u64_vec(v: &[u64]) -> Vec<u8> {
    let mut b = Vec::new();
    encode_uint_vector_u64(v, &mut b).unwrap();
    b
}
fn enc_string_vec(v: &[Vec<u8>]) -> Vec<u8> {
    let refs: Vec<&[u8]> = v.iter().map(Vec::as_slice).collect();
    let mut b = Vec::new();
    encode_string_vector(&refs, &mut b).unwrap();
    b
}
fn enc_cell_id_vec(v: &[CellId]) -> Vec<u8> {
    let mut b = Vec::new();
    encode_s2cell_id_vector(v, &mut b).unwrap();
    b
}
fn enc_point_vec(v: &[Point], hint: CodingHint) -> Vec<u8> {
    let mut b = Vec::new();
    encode_s2point_vector(v, hint, &mut b).unwrap();
    b
}

// ── the core invariant: a decoder must never panic ──────────────────────────

/// Run `decode` on `input`; fail the test (with the offending bytes) if it
/// panics. Returning `Ok` or `Err` is equally acceptable — only a panic, abort,
/// or hang is a bug.
fn no_panic<T, F>(label: &str, decode: F, input: &[u8])
where
    F: Fn(&mut &[u8]) -> std::io::Result<T>,
{
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut r = input;
        let _unused = decode(&mut r);
    }));
    assert!(
        res.is_ok(),
        "{label} panicked on {}-byte input: {input:02x?}",
        input.len()
    );
}

/// The set of decoders, each wrapped so it reads from a `&mut &[u8]`.
fn for_each_decoder(mut f: impl FnMut(&str, &dyn Fn(&[u8]))) {
    f("uint_vector_u32", &|b| {
        no_panic("uint_vector_u32", |r| decode_uint_vector_u32(r), b);
    });
    f("uint_vector_u64", &|b| {
        no_panic("uint_vector_u64", |r| decode_uint_vector_u64(r), b);
    });
    f("string_vector", &|b| {
        no_panic("string_vector", |r| decode_string_vector(r), b);
    });
    f("s2cell_id_vector", &|b| {
        no_panic("s2cell_id_vector", |r| decode_s2cell_id_vector(r), b);
    });
    f("s2point_vector", &|b| {
        no_panic("s2point_vector", |r| decode_s2point_vector(r), b);
    });
}

/// A representative valid encoding for each decoder, used as the seed for
/// truncation and mutation attacks.
fn valid_encodings(r: &mut ChaCha8Rng) -> Vec<(&'static str, Vec<u8>)> {
    let u32s: Vec<u32> = (0..16).map(|_| r.r#gen()).collect();
    let u64s: Vec<u64> = (0..16).map(|_| r.r#gen()).collect();
    let strings: Vec<Vec<u8>> = (0..6)
        .map(|_| {
            let n = r.gen_range(0..12);
            (0..n).map(|_| r.r#gen()).collect()
        })
        .collect();
    let cells = rand_cell_ids(r, 16);
    let points = rand_points(r, 16);
    vec![
        ("uint_vector_u32", enc_u32_vec(&u32s)),
        ("uint_vector_u64", enc_u64_vec(&u64s)),
        ("string_vector", enc_string_vec(&strings)),
        ("s2cell_id_vector", enc_cell_id_vec(&cells)),
        (
            "s2point_vector_fast",
            enc_point_vec(&points, CodingHint::Fast),
        ),
        (
            "s2point_vector_compact",
            enc_point_vec(&points, CodingHint::Compact),
        ),
    ]
}

fn decode_by_label(label: &str, b: &[u8]) {
    // Map the seed label to every decoder so each valid blob is also thrown at
    // the *other* decoders (cross-format confusion is a classic crash source).
    let _ = label;
    for_each_decoder(|_, run| run(b));
}

// ── tests ───────────────────────────────────────────────────────────────────

/// Valid data must round-trip through encode→decode unchanged.
#[test]
fn round_trip_valid() {
    let mut r = rng(0xA1);
    for _ in 0..200 {
        let n = r.gen_range(0..32);

        let u32s: Vec<u32> = (0..n).map(|_| r.r#gen()).collect();
        assert_eq!(
            decode_uint_vector_u32(&mut enc_u32_vec(&u32s).as_slice()).unwrap(),
            u32s
        );

        let u64s: Vec<u64> = (0..n).map(|_| r.r#gen()).collect();
        assert_eq!(
            decode_uint_vector_u64(&mut enc_u64_vec(&u64s).as_slice()).unwrap(),
            u64s
        );

        let strings: Vec<Vec<u8>> = (0..n)
            .map(|_| {
                let m = r.gen_range(0..16);
                (0..m).map(|_| r.r#gen()).collect()
            })
            .collect();
        assert_eq!(
            decode_string_vector(&mut enc_string_vec(&strings).as_slice()).unwrap(),
            strings
        );

        let cells = rand_cell_ids(&mut r, n);
        assert_eq!(
            decode_s2cell_id_vector(&mut enc_cell_id_vec(&cells).as_slice()).unwrap(),
            cells
        );

        let points = rand_points(&mut r, n);
        // Fast (UNCOMPRESSED) stores exact f64 components -> exact round-trip.
        let decoded_fast =
            decode_s2point_vector(&mut enc_point_vec(&points, CodingHint::Fast).as_slice())
                .unwrap();
        assert_eq!(decoded_fast, points);
        // Compact may snap arbitrary points to cell centers; require the count.
        let decoded_compact =
            decode_s2point_vector(&mut enc_point_vec(&points, CodingHint::Compact).as_slice())
                .unwrap();
        assert_eq!(decoded_compact.len(), points.len());
    }
}

/// Every prefix of a valid encoding must decode without panicking (EOF in the
/// middle of any field must surface as `Err`, never a slice/parse panic).
#[test]
fn truncation_resilience() {
    let mut r = rng(0xB2);
    for (_, bytes) in valid_encodings(&mut r) {
        for cut in 0..=bytes.len() {
            decode_by_label("", &bytes[..cut]);
        }
    }
}

/// Mutated valid encodings (flips, insertions, deletions) must not panic.
#[test]
fn mutation_fuzz() {
    let mut r = rng(0xC3);
    for _ in 0..4000 {
        let seeds = valid_encodings(&mut r);
        let (_, base) = &seeds[r.gen_range(0..seeds.len())];
        let mut bytes = base.clone();
        // Apply 1–6 random mutations.
        for _ in 0..r.gen_range(1..=6) {
            if bytes.is_empty() {
                bytes.push(r.r#gen());
                continue;
            }
            match r.gen_range(0..4) {
                0 => {
                    let i = r.gen_range(0..bytes.len());
                    bytes[i] ^= 1 << r.gen_range(0..8);
                }
                1 => {
                    let i = r.gen_range(0..bytes.len());
                    bytes[i] = r.r#gen();
                }
                2 => {
                    let i = r.gen_range(0..=bytes.len());
                    bytes.insert(i, r.r#gen());
                }
                _ => {
                    let i = r.gen_range(0..bytes.len());
                    bytes.remove(i);
                }
            }
        }
        decode_by_label("", &bytes);
    }
}

/// Short fully-random byte strings must not panic any decoder.
#[test]
fn random_bytes_fuzz() {
    let mut r = rng(0xD4);
    for _ in 0..20_000 {
        let len = r.gen_range(0..=48);
        let bytes: Vec<u8> = (0..len).map(|_| r.r#gen()).collect();
        decode_by_label("", &bytes);
    }
}

/// Hand-crafted adversarial inputs: empty, single bytes, all-ones varints, and
/// a length field that overstates the data (kept in a safe band so even a naive
/// pre-allocation stays small). Each must return `Err`/`Ok`, never panic.
#[test]
fn adversarial_inputs() {
    // Empty and every single byte.
    decode_by_label("", &[]);
    for b in 0u16..=255 {
        decode_by_label("", &[b as u8]);
    }
    // Long runs of 0xFF (maximal varints) of several lengths.
    for n in [1usize, 2, 4, 8, 10, 16, 32] {
        decode_by_label("", &vec![0xFFu8; n]);
    }
    // A length prefix claiming ~100k elements with no payload: a robust decoder
    // returns Err on EOF rather than trusting the count. 100k is large enough to
    // be wrong, small enough that a naive pre-allocation is harmless.
    let mut overstated = Vec::new();
    encode_uint_vector_u64(&(0..100_000u64).collect::<Vec<u64>>(), &mut overstated).unwrap();
    let header_only = &overstated[..overstated.len().min(4)];
    decode_by_label("", header_only);
}

/// Regression cases: exact inputs that previously crashed a decoder, kept here
/// so the bug stays fixed even without the `fuzz/` cargo-fuzz layer.
#[test]
fn regression_fuzz_crashes() {
    // `decode_s2point_vector`, CELL_IDS format: `base + offset + delta`
    // overflowed u64 (panic in debug, silent wrap in release). Fixed by computing
    // the cell value with wrapping arithmetic, matching upstream C++. Found by
    // `fuzz/fuzz_targets/decode_s2point_vector.rs`.
    let s2point_overflow: &[u8] = &[
        0x99, 0x60, 0x68, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d,
        0x0d, 0x6f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x0d, 0x3c, 0x0d, 0x0d,
        0x7a, 0x0d, 0x0d, 0x0d, 0x28, 0xe0, 0xe0, 0xdb, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d, 0x0d,
        0x0d, 0x29, 0x29, 0x29, 0xb9, 0x00, 0x00, 0x07, 0x20,
    ];
    decode_by_label("", s2point_overflow);
}
