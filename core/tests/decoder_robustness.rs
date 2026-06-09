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

use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::density_tree::{S2DensityTree, TreeEncoder};
use s2rst::s2::encoded_s2cell_id_vector::{decode_s2cell_id_vector, encode_s2cell_id_vector};
use s2rst::s2::encoded_s2point_vector::{CodingHint, decode_s2point_vector, encode_s2point_vector};
use s2rst::s2::encoded_s2shape_index::EncodedS2ShapeIndex;
use s2rst::s2::encoded_string_vector::{decode_string_vector, encode_string_vector};
use s2rst::s2::encoded_uint_vector::{
    decode_uint_vector_u32, decode_uint_vector_u64, encode_uint_vector_u32, encode_uint_vector_u64,
};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::point_compression::{
    decode_points_compressed, encode_points_compressed, points_to_xyz_face_si_ti,
};
use s2rst::s2::point_vector::PointVector;
use s2rst::s2::polyline::Polyline;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{Cap, Cell, CellId, CellUnion, LatLng, Loop, Point, Polygon, Rect};

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

/// Like [`no_panic`] but for decoders that consume a `&[u8]` directly (e.g. the
/// shape-index initializer) rather than a `&mut dyn Read`.
fn no_panic_bytes<F>(label: &str, decode: F, input: &[u8])
where
    F: Fn(&[u8]),
{
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode(input)));
    assert!(
        res.is_ok(),
        "{label} panicked on {}-byte input: {input:02x?}",
        input.len()
    );
}

/// Build a small but structurally complete density tree (a face cell with two
/// descendants, so the ancestors-present invariant holds) and return its bytes.
fn valid_density_tree_bytes() -> Vec<u8> {
    let mut enc = TreeEncoder::new();
    let face = CellId::from_face(0u8);
    enc.put(face, 100);
    enc.put(face.children()[0], 60);
    enc.put(face.children()[0].children()[2], 25);
    let mut tree = S2DensityTree::new();
    enc.build(&mut tree);
    let mut bytes = Vec::new();
    tree.encode(&mut bytes);
    bytes
}

/// Drive the density-tree decode path exactly as the fuzz target does (`init`
/// then `decode`); neither may panic or hang on arbitrary bytes.
fn density_no_panic(input: &[u8]) {
    no_panic_bytes(
        "density_tree",
        |b| {
            let mut tree = S2DensityTree::new();
            if tree.init(b).is_ok() {
                let _unused = tree.decode();
            }
        },
        input,
    );
}

/// Drive the shape-index decode-*then-query* path exactly as
/// `fuzz/fuzz_targets/decode_s2shape_index.rs` does: decode, materialize every
/// shape's edges, then run a contains-point query at each cell center. None of
/// it may panic on arbitrary bytes. The other `s2shape_index` regression cases
/// only reach `init`; the query path additionally runs cell-id arithmetic
/// (`seek`/`range_min`/`range_max`/`center`) that a malformed cell-id vector can
/// drive into an overflow or out-of-range panic.
fn shape_index_query_no_panic(input: &[u8]) {
    no_panic_bytes(
        "s2shape_index_query",
        |b| {
            let mut index = EncodedS2ShapeIndex::new();
            if index.init(b).is_err() {
                return;
            }
            let idx = index.as_index();
            for id in 0..idx.num_shape_ids() {
                if let Some(shape) = idx.shape(id as i32) {
                    let n = shape.num_edges();
                    for e in 0..n.min(4096) {
                        let _unused = shape.edge(e);
                    }
                }
            }
            let mut query = ContainsPointQuery::new(idx, VertexModel::SemiOpen);
            let mut it = idx.iter();
            let mut budget = 4096usize;
            while !it.done() && budget > 0 {
                let _unused = query.contains(it.center());
                it.next();
                budget -= 1;
            }
        },
        input,
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

/// Regression cases for the higher-level decoders, all found by the `fuzz/`
/// cargo-fuzz layer and fixed by validating decoded values before constructing
/// geometry. Each must now decode to `Err` (never panic).
#[test]
fn regression_high_level_crashes() {
    // 1. `EncodedS2ShapeIndex::init` -> tagged `Polygon` decode -> an unvalidated
    //    `Rect` bound (raw f64) -> `expand_for_subregions` -> `S1Interval::new`
    //    assert. Fixed by validating the bound in `Rect::decode`.
    let shape_index_bound: &[u8] = &[
        0x2a, 0x08, 0x4a, 0x01, 0x01, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0,
        0x3f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x27, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x8a, 0x01, 0x00, 0x00, 0x27, 0xdc, 0xf7, 0xff, 0xff, 0xff, 0xfc,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xc9, 0x58, 0x00, 0x00, 0x00, 0x00, 0x00, 0xde, 0xa1, 0x3f, 0x08,
        0xe0, 0x10, 0x08, 0x01, 0x00,
    ];
    // 2. `EncodedS2ShapeIndex::init` -> per-cell decode -> i32 `shape_id`/`edge`
    //    add overflow. Fixed with checked arithmetic in `decode_cell`/`decode_edges`.
    let shape_index_overflow: &[u8] = &[
        0x2a, 0x80, 0x00, 0xf0, 0x12, 0x2c, 0xef, 0x11, 0x30, 0x10, 0x12, 0x10, 0x10, 0x2c, 0xef,
        0xe7, 0x11, 0x2a, 0x08, 0x4a, 0x02, 0x02, 0x04, 0x19, 0xff, 0x01, 0x00, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0x08, 0x12, 0x2c, 0xef, 0x11, 0x30, 0x10, 0x12, 0x10, 0x10,
        0x2c, 0xef, 0xe7, 0x11, 0x2a, 0x08, 0x4a, 0x02, 0x02, 0x04, 0x19, 0xff, 0x01, 0x00, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x08, 0xff, 0xff, 0x10, 0x10, 0xff, 0xff, 0x10,
        0x10,
    ];
    for input in [shape_index_bound, shape_index_overflow] {
        no_panic_bytes(
            "s2shape_index",
            |b| {
                let mut idx = EncodedS2ShapeIndex::new();
                let _unused = idx.init(b);
            },
            input,
        );
    }

    // 2b. `EncodedS2ShapeIndex::init` decodes a cell-id vector of `[0x1700…, 0,
    //     0, 0]` — unsorted, with invalid (id 0, `lsb() == 0`) entries. The decode
    //     itself was fine, but the *query* path then called `range_max` on id 0,
    //     underflowing `lsb() - 1` (cell_id.rs `range_max`) → panic. (A face-bits
    //     >= 6 id would likewise panic `center`'s `to_point` face-table index.)
    //     Fixed by rejecting invalid / non-increasing cell ids in
    //     `ShapeIndex::decode_from_reader`. Found by `decode_s2shape_index`.
    let shape_index_bad_cell_ids: &[u8] = &[
        0x2a, 0x19, 0x62, 0x00, 0xc4, 0x00, 0x28, 0x00, 0x7e, 0x1a, 0xbf, 0xc6, 0x3a, 0x8d, 0x16,
        0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0x4a, 0x84, 0x42, 0xc3, 0xf9,
        0xef, 0x3f, 0xce, 0x5b, 0x5a, 0x6f, 0xa6, 0xdd, 0xa1, 0x52, 0x00, 0xdd, 0x0b, 0x7e, 0x1a,
        0x3a, 0xc6, 0x3f, 0xe0, 0x20, 0x17, 0x00, 0x00, 0x00, 0x20, 0x04, 0x06, 0x0d, 0x14, 0x09,
        0x02, 0x00, 0x00, 0x00, 0x14, 0x09, 0x02, 0x00, 0x00, 0x00, 0x00, 0x3d, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x2a,
    ];
    shape_index_query_no_panic(shape_index_bad_cell_ids);

    // 2c. `EncodedS2ShapeIndex::init` decodes valid cells whose clipped-shape
    //     *edge ids* exceed the owning shape's edge count. The decode was fine,
    //     but the query then called `shape.edge(edge_id)` (contains_point_query.rs)
    //     with an out-of-range id → out-of-bounds panic in `LaxPolygon::edge`.
    //     Fixed by validating each cell's clipped edge ids against the shape's
    //     edge count in `ShapeIndex::decode_from_reader`. Found by
    //     `decode_s2shape_index`.
    let shape_index_bad_edge_ids: &[u8] = &[
        0x2a, 0x19, 0x62, 0x00, 0xc4, 0x00, 0x28, 0x01, 0x04, 0x20, 0xd4, 0x4a, 0x84, 0x42, 0xc3,
        0xf9, 0xef, 0x3f, 0xce, 0x5b, 0x5a, 0x6f, 0xa6, 0xdd, 0xa1, 0x3f, 0x1e, 0xdd, 0x89, 0x2b,
        0x0b, 0xdf, 0x91, 0x3f, 0x0a, 0xf7, 0xcb, 0x5e, 0xd8, 0xe0, 0xef, 0x3f, 0x0d, 0xed, 0x98,
        0x8b, 0x4b, 0xd5, 0xb1, 0x3f, 0x0e, 0xc9, 0xef, 0x48, 0xc7, 0xcb, 0xaa, 0x3f, 0x25, 0x33,
        0x97, 0x00, 0x1f, 0xb4, 0xef, 0x3f, 0xf3, 0x9d, 0xe7, 0x42, 0x4f, 0xa8, 0xba, 0x3f, 0x02,
        0x81, 0xc2, 0xb8, 0xd6, 0x4f, 0xb6, 0x3f, 0x4f, 0x37, 0x9f, 0xef, 0xce, 0x73, 0xef, 0x3f,
        0x62, 0xa4, 0x4b, 0x74, 0x6e, 0xae, 0xc1, 0x3f, 0xd3, 0x62, 0x4f, 0x4c, 0xd4, 0x32, 0xbf,
        0xbf, 0x03, 0x20, 0xd4, 0x4a, 0x84, 0x42, 0xc3, 0xf9, 0xef, 0x3f, 0xce, 0x5b, 0x5a, 0x6f,
        0xa6, 0xdd, 0xa1, 0x3f, 0x1e, 0xdd, 0x89, 0x2b, 0x0b, 0xdf, 0x91, 0x3f, 0x5e, 0x0a, 0xd8,
        0xcb, 0xf7, 0xe0, 0xef, 0x3f, 0x0d, 0xed, 0x98, 0x8b, 0x4b, 0xd5, 0xb1, 0x3f, 0x0e, 0xc9,
        0xef, 0x48, 0xc7, 0xcb, 0xaa, 0x3f, 0x25, 0x33, 0x97, 0x00, 0x1f, 0xb4, 0xef, 0x3f, 0xf3,
        0x9f, 0xe7, 0x42, 0x4f, 0xa8, 0xba, 0x3f, 0x02, 0x81, 0xc2, 0xb8, 0xd6, 0x4f, 0xb6, 0x3f,
        0x4f, 0x37, 0x9f, 0xef, 0xce, 0x73, 0xef, 0x3f, 0x62, 0xa4, 0x4b, 0x74, 0x6e, 0xae, 0xc1,
        0x3f, 0xd3, 0x62, 0x4f, 0x4c, 0xd4, 0x36, 0xbf, 0xbf, 0x05, 0x01, 0x01, 0x20, 0x8d, 0x16,
        0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0xa8, 0xe3, 0xc5, 0xbf, 0x89,
        0x73, 0x0b, 0x7e, 0x1a, 0x3a, 0xc6, 0xbf, 0x8d, 0x16, 0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f,
        0xf4, 0x0b, 0x8a, 0x74, 0xa8, 0xe3, 0xc5, 0x3f, 0x89, 0x73, 0x0b, 0x7e, 0x1a, 0x3a, 0xc6,
        0xbf, 0x8d, 0x16, 0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f, 0xf4, 0x16, 0x29, 0x21, 0xfb, 0x08,
        0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0xa8, 0xe3, 0xc5, 0x3f, 0x89, 0x73, 0x0b, 0x7e, 0x1a,
        0x3a, 0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0xa8, 0xe3, 0xc5, 0xbf, 0x89, 0x73, 0x0b, 0x7e,
        0x1a, 0x3a, 0xc6, 0x3f, 0xe0, 0x20, 0x04, 0x0c, 0x14, 0x1c, 0x20, 0x04, 0x06, 0x0d, 0x14,
        0x02, 0x00, 0x09, 0x08, 0x02, 0x21, 0x1b, 0x00, 0x02, 0x00, 0x02, 0x04, 0x05, 0x1b, 0x08,
        0x00, 0x2c, 0x00, 0x00, 0x01,
    ];
    shape_index_query_no_panic(shape_index_bad_edge_ids);

    // 2d. Like 2c, but a clipped shape's *shape id* (also delta-decoded) is
    //     negative. `index.shape(shape_id)` calls `ShapeId::as_usize`, which
    //     asserts non-negative (shape.rs) → panic, before any edge lookup. Fixed
    //     by range-checking the shape id in `ShapeIndex::decode_from_reader`.
    //     Found by `decode_s2shape_index`.
    let shape_index_bad_shape_id: &[u8] = &[
        0x2a, 0x19, 0x62, 0x00, 0xc4, 0x00, 0x28, 0x00, 0x03, 0x00, 0x00, 0x51, 0x51, 0x51, 0x51,
        0x51, 0x51, 0x41, 0x00, 0x05, 0x00, 0x7e, 0x1a, 0xbf, 0x23, 0x3a, 0x8d, 0x16, 0x29, 0x21,
        0x5e, 0x5e, 0x5e, 0x5e, 0x5e, 0x3b, 0x24, 0x5e, 0x5e, 0x5e, 0x5e, 0x5e, 0x5e, 0x5e, 0x5e,
        0x5e, 0x5e, 0x5e, 0x14, 0x1c, 0x20, 0x04, 0x06, 0x0d, 0x14, 0x09, 0xe5, 0x02, 0x00, 0x02,
        0x08, 0x21, 0x1b, 0xff, 0xff, 0x92, 0x92, 0xff, 0xff, 0x2a, 0x19, 0x62, 0x00, 0xc4, 0x00,
        0x28, 0x01, 0x04, 0x20, 0xd4, 0x4a, 0x84, 0x42, 0xc3, 0xf9, 0xef, 0x3f, 0xce, 0x5b, 0x5a,
        0x6f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67,
        0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67,
        0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x67, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x6e, 0x4f,
        0x3e, 0xc0, 0x2c, 0x62, 0x4f, 0x4c, 0xd4, 0x32, 0xbf, 0xbf, 0x03, 0x20, 0xd4, 0x4a, 0x84,
        0x42, 0xc3, 0xf9, 0xef, 0x3f, 0xce, 0x5b, 0x5a, 0x6f, 0xa6, 0xdd, 0xa1, 0x3f, 0x1e, 0xdd,
        0x89, 0x2b, 0x0b, 0xdf, 0x91, 0x3f, 0x0a, 0xf7, 0xcb, 0x5e, 0xd8, 0xe0, 0xef, 0x3f, 0x0d,
        0xed, 0x98, 0x8b, 0x4b, 0xd5, 0x8d, 0x16, 0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f, 0xf4, 0x0b,
        0x8a, 0x74, 0xa8, 0xe3, 0xc5, 0x3f, 0x89, 0x73, 0x0b, 0x7e, 0x1a, 0x3a, 0xc6, 0xbf, 0x8d,
        0x16, 0x29, 0x21, 0xfb, 0x08, 0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0x88, 0xe3, 0xc5, 0x3f,
        0x89, 0x73, 0x0b, 0x7e, 0x1a, 0x3a, 0xc6, 0x0f, 0x00, 0x16, 0x29, 0x2a, 0xff, 0x00, 0x10,
        0x06, 0x00, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x92, 0x9b,
        0x93, 0x93, 0x93, 0x93, 0x93, 0x19, 0x62, 0x00, 0xc4, 0x9d, 0x28, 0x01, 0x04, 0x02, 0x23,
        0x30, 0x30, 0x30, 0x30, 0x30, 0xd5, 0xd5, 0xd5, 0xd5, 0xd5, 0xd5, 0x30, 0x30, 0x30, 0xfb,
        0x08, 0xef, 0x3f, 0xf4, 0x0b, 0x8a, 0x74, 0xaa, 0x3f, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x67, 0xc4, 0x00, 0x28, 0x00, 0x8d, 0x16, 0x29, 0x21, 0x5e, 0x5e,
        0x5e, 0x5e, 0x84, 0x42, 0x93, 0xd6, 0x4f, 0x04, 0x00, 0xc3,
    ];
    shape_index_query_no_panic(shape_index_bad_shape_id);

    // 3. `Polygon::decode` (compressed) -> `decode_loop_compressed` ->
    //    `from_decoded_compressed` -> `init_bound` with a NaN off-center point ->
    //    `S1Interval::from_point_pair` assert. Fixed by validating off-center
    //    points in `decode_points_compressed`.
    let polygon_compressed_nan: &[u8] = &[
        0x04, 0x03, 0x22, 0x0b, 0xc8, 0x05, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10, 0x00, 0x01, 0x05, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x04, 0x03, 0x22, 0x80, 0x00,
        0x00, 0x00, 0x00,
    ];
    no_panic(
        "polygon",
        |r| <Polygon as S2Decode>::decode(r),
        polygon_compressed_nan,
    );

    // 4. `Polygon::decode` (compressed) -> `decode_loop_compressed` -> a
    //    degenerate loop (duplicate vertices) -> `ordered_ccw` assert during the
    //    index build. Fixed by validating the loop in `from_decoded[_compressed]`.
    let polygon_compressed_degenerate: &[u8] = &[
        0x04, 0x04, 0x02, 0x0a, 0x59, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x08, 0x04, 0x04, 0x0a, 0xdd, 0x01, 0x00, 0x00, 0x40, 0x00, 0x04, 0x00, 0x00,
        0x00,
    ];
    no_panic(
        "polygon",
        |r| <Polygon as S2Decode>::decode(r),
        polygon_compressed_degenerate,
    );
}

// ── higher-level decoders ────────────────────────────────────────────────────

/// `decode_points_compressed` is the arithmetic-heavy compressed point codec. It
/// takes a level and a point count alongside the reader, so it can't join the
/// uniform `for_each_decoder` sweep — fuzz it directly. The count is *capped*
/// (mirroring `fuzz/fuzz_targets/decode_points_compressed.rs`) because this
/// function, unlike the higher-level decoders, does not bound `num_points`
/// itself; its callers do, before the up-front `Vec::with_capacity(num_points)`.
#[test]
fn compressed_points_robustness() {
    let mut r = rng(0xE5);
    let level = 20u8; // any valid S2 cell level (0..=30)

    for _ in 0..100 {
        // Valid data round-trips by count (compression may snap to cell centers).
        let n = r.gen_range(1..32);
        let points = rand_points(&mut r, n);
        let xyz = points_to_xyz_face_si_ti(&points);
        let mut buf = Vec::new();
        encode_points_compressed(&mut buf, &xyz, level).unwrap();
        let decoded = decode_points_compressed(&mut buf.as_slice(), level, points.len()).unwrap();
        assert_eq!(decoded.len(), points.len());

        // Every truncation of that valid encoding must not panic.
        for cut in 0..=buf.len() {
            no_panic(
                "compressed_points",
                |rr| decode_points_compressed(rr, level, points.len()),
                &buf[..cut],
            );
        }
    }

    // Short random inputs, with the level and a capped count derived from the
    // input prefix exactly as the fuzz target does.
    for _ in 0..20_000 {
        let len = r.gen_range(0..=64);
        let bytes: Vec<u8> = (0..len).map(|_| r.r#gen()).collect();
        if bytes.len() < 3 {
            continue;
        }
        let lvl: u8 = bytes[0] % 31; // 0..=30
        let num_points = (u16::from_le_bytes([bytes[1], bytes[2]]) % 4096) as usize;
        no_panic(
            "compressed_points",
            |rr| decode_points_compressed(rr, lvl, num_points),
            &bytes[3..],
        );
    }
}

/// Valid encodings of the higher-level decoders (`Polygon`, shape index) must
/// round-trip, and every truncation of a valid encoding must decode without
/// panicking (EOF mid-field → `Err`, never a panic).
///
/// Arbitrary / mutated-byte fuzzing of these two is intentionally left to the
/// nightly `fuzz/` lane, which runs under `-rss_limit_mb`. A crafted count in
/// these formats can drive a very large allocation — and, for shape-index edge
/// counts, an unbounded one — which an in-process test with no memory cap cannot
/// absorb. Truncating a *valid* encoding never inflates a count field, so it
/// stays safe here.
#[test]
fn high_level_decoders_truncation() {
    // Polygon: empty, full, and a small square shell (exercises the loop and
    // vertex reading paths under truncation).
    let square = Polygon::from_loops(vec![Loop::new(vec![
        LatLng::from_degrees(-10.0, -10.0).to_point(),
        LatLng::from_degrees(-10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, -10.0).to_point(),
    ])]);
    for poly in [Polygon::empty(), Polygon::full(), square] {
        let mut buf = Vec::new();
        poly.encode(&mut buf).unwrap();
        let decoded = <Polygon as S2Decode>::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.num_loops(), poly.num_loops());
        for cut in 0..=buf.len() {
            no_panic("polygon", |r| <Polygon as S2Decode>::decode(r), &buf[..cut]);
        }
    }

    // Shape index: an empty index round-trips and every truncation is clean.
    let mut index = ShapeIndex::new();
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let mut enc = EncodedS2ShapeIndex::new();
    enc.init(&buf).unwrap();
    assert_eq!(enc.num_shape_ids(), 0);
    for cut in 0..=buf.len() {
        no_panic_bytes(
            "s2shape_index",
            |b| {
                let mut idx = EncodedS2ShapeIndex::new();
                let _unused = idx.init(b);
            },
            &buf[..cut],
        );
    }
}

/// `S2DensityTree::{init, decode}` parses a custom varint-tree format and walks
/// it into a `CellId -> weight` map. Like the shape index it consumes `&[u8]`
/// directly, so it can't join the uniform sweep. After the offset-overflow and
/// traversal-budget fixes, arbitrary bytes are fully bounded (offsets are
/// range-checked against the buffer, and each byte offset is decoded at most
/// once), so the whole mutation/random sweep runs safely in-process.
#[test]
fn density_tree_robustness() {
    // Valid data round-trips: init succeeds and decode recovers all 3 cells.
    let valid = valid_density_tree_bytes();
    let mut tree = S2DensityTree::new();
    tree.init(&valid).unwrap();
    assert_eq!(tree.decode().unwrap().len(), 3);

    // Every truncation of the valid encoding must not panic.
    for cut in 0..=valid.len() {
        density_no_panic(&valid[..cut]);
    }

    // Mutated valid encodings (flips, insertions, deletions) must not panic/hang.
    let mut r = rng(0xF6);
    for _ in 0..4000 {
        let mut bytes = valid.clone();
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
        density_no_panic(&bytes);
    }

    // Short random inputs — most bounce off the 14-byte magic at `init` ...
    for _ in 0..20_000 {
        let len = r.gen_range(0..=48);
        let bytes: Vec<u8> = (0..len).map(|_| r.r#gen()).collect();
        density_no_panic(&bytes);
    }

    // ... so also fuzz inputs that DO carry the magic, reaching the tree walk
    // where the overflow/aliasing bugs lived (the fuzz target needs this seed
    // too: without the magic prefix libFuzzer never gets past `init`).
    for _ in 0..20_000 {
        let len = r.gen_range(0..=48);
        let mut bytes = b"S2DensityTree0".to_vec();
        bytes.extend((0..len).map(|_| r.r#gen::<u8>()));
        density_no_panic(&bytes);
    }
}

/// Regression cases for `S2DensityTree::{init, decode}`, found via
/// `fuzz/fuzz_targets/decode_density_tree.rs`:
///   A — `i64` overflow summing face offsets in `decode_header` (panicked in
///       `init`);
///   B — the same overflow on child offsets in `DensityCell::decode` (panicked
///       in `decode`);
///   C — aliased child offsets (delta 0) drove a 2^level re-decode → timeout/OOM.
/// Fixed with `add_i64` / offset range checks and a visited-offset guard; each
/// must now resolve to `Err`, never panic or hang.
#[test]
fn regression_density_tree_crashes() {
    const MAGIC: &[u8] = b"S2DensityTree0";
    // varint(i64::MAX) = eight 0xFF continuation bytes then 0x7F.
    const VARINT_I64_MAX: &[u8] = &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f];

    // A: face_mask=0x03, face-0 length = i64::MAX -> faces[1] = pos + i64::MAX.
    let mut a = MAGIC.to_vec();
    a.push(0x03);
    a.extend_from_slice(VARINT_I64_MAX);
    let mut ta = S2DensityTree::new();
    assert!(
        ta.init(&a).is_err(),
        "A: header offset overflow must be rejected, not panic"
    );

    // B: one face; its root cell has child_mask=0x03 and a child delta = i64::MAX.
    let mut b = MAGIC.to_vec();
    b.extend_from_slice(&[0x01, 0x03]);
    b.extend_from_slice(VARINT_I64_MAX);
    let mut tb = S2DensityTree::new();
    assert!(tb.init(&b).is_ok(), "B: header is valid");
    assert!(
        tb.decode().is_err(),
        "B: child offset overflow must be rejected, not panic"
    );

    // C: a chain of [0x03,0x00] cells (both children share the next offset) plus
    // a childless terminator; without the visited-offset guard this re-decodes
    // the chain 2^level times.
    let mut c = MAGIC.to_vec();
    c.push(0x01);
    for _ in 0..25 {
        c.extend_from_slice(&[0x03, 0x00]);
    }
    c.push(0x00);
    let mut tc = S2DensityTree::new();
    assert!(tc.init(&c).is_ok(), "C: header is valid");
    assert!(
        tc.decode().is_err(),
        "C: aliased offsets must be rejected, not hang"
    );
}

// ── comprehensive panic survey across EVERY decoder ──────────────────────────

/// Throw a diverse corpus (valid encodings of every type, their truncations and
/// mutations, plus random and adversarial bytes) at EVERY decoder and assert
/// none panics.
///
/// With the allocation caps in `encoding`/`shape_index_encoding`/
/// `point_compression`, arbitrary bytes can no longer drive a large up-front
/// allocation, so this whole sweep runs safely in-process. The test build keeps
/// `debug_assert!`s enabled, so any decoded value that violates a geometry
/// invariant surfaces as a (caught) panic and fails this test, naming the
/// offending decoder and the exact bytes.
#[test]
fn no_decoder_panics_on_any_input() {
    fn enc<T: S2Encode>(x: &T) -> Vec<u8> {
        let mut b = Vec::new();
        x.encode(&mut b).unwrap();
        b
    }

    let mut r = rng(0x5A);

    // A known-valid CCW square loop (same coordinates as the geo-interop tests).
    let square: Vec<Point> = [(-10.0, -10.0), (-10.0, 10.0), (10.0, 10.0), (10.0, -10.0)]
        .iter()
        .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
        .collect();
    let pts = rand_points(&mut r, 6);
    let cells = rand_cell_ids(&mut r, 6);

    // A rich shape index: one of each tagged shape type.
    let mut index = ShapeIndex::new();
    index.add(Box::new(LaxPolyline::new(pts.clone())));
    index.add(Box::new(PointVector::new(pts.clone())));
    index.add(Box::new(LaxPolygon::from_loops(&[&square])));
    index.build();
    let mut si_bytes = Vec::new();
    index.encode_to_writer(&mut si_bytes).unwrap();

    // Valid encodings of every decodable type.
    let mut seeds: Vec<Vec<u8>> = vec![
        enc(&rand_point(&mut r)),
        enc(&Cap::from_point(rand_point(&mut r))),
        enc(&Rect::from_point_pair(
            LatLng::from_degrees(-1.0, -2.0),
            LatLng::from_degrees(3.0, 4.0),
        )),
        enc(&CellId::from_point(&rand_point(&mut r))),
        enc(&CellUnion::from_cell_ids(cells.clone())),
        enc(&Cell::from(CellId::from_point(&rand_point(&mut r)))),
        enc(&Polyline::new(pts.clone())),
        enc(&Loop::new(square.clone())),
        enc(&Polygon::from_loops(vec![Loop::new(square.clone())])),
        enc(&LaxPolyline::new(pts.clone())),
        enc(&LaxPolygon::from_loops(&[&square])),
        enc(&PointVector::new(pts.clone())),
        si_bytes,
        enc_u32_vec(&[1, 2, 3, 4]),
        enc_u64_vec(&[1, 2, 3, 4]),
        enc_string_vec(&[vec![1, 2], vec![3]]),
        enc_cell_id_vec(&cells),
        enc_point_vec(&pts, CodingHint::Fast),
        enc_point_vec(&pts, CodingHint::Compact),
    ];
    // Compressed-points seed: 3-byte (level, count) prefix the decoder closure
    // strips, then the encoded payload.
    {
        let xyz = points_to_xyz_face_si_ti(&pts);
        let count = (pts.len() as u16).to_le_bytes();
        let mut b = vec![20u8, count[0], count[1]];
        encode_points_compressed(&mut b, &xyz, 20u8).unwrap();
        seeds.push(b);
    }

    // Corpus: seeds, every truncation, mutations, random, and adversarial bytes.
    let mut corpus: Vec<Vec<u8>> = Vec::new();
    for s in &seeds {
        corpus.push(s.clone());
        for cut in 0..s.len() {
            corpus.push(s[..cut].to_vec());
        }
    }
    for _ in 0..2000 {
        let base = &seeds[r.gen_range(0..seeds.len())];
        let mut b = base.clone();
        for _ in 0..r.gen_range(1..=6) {
            if b.is_empty() {
                b.push(r.r#gen());
                continue;
            }
            match r.gen_range(0..4) {
                0 => {
                    let i = r.gen_range(0..b.len());
                    b[i] ^= 1 << r.gen_range(0..8);
                }
                1 => {
                    let i = r.gen_range(0..b.len());
                    b[i] = r.r#gen();
                }
                2 => {
                    let i = r.gen_range(0..=b.len());
                    b.insert(i, r.r#gen());
                }
                _ => {
                    let i = r.gen_range(0..b.len());
                    b.remove(i);
                }
            }
        }
        corpus.push(b);
    }
    for _ in 0..10_000 {
        let n = r.gen_range(0..=64);
        corpus.push((0..n).map(|_| r.r#gen()).collect());
    }
    corpus.push(vec![]);
    for b in 0u16..=255 {
        corpus.push(vec![b as u8]);
    }
    for n in [1usize, 2, 4, 8, 10, 16, 32, 64] {
        corpus.push(vec![0xFF; n]);
    }

    // Every decoder, wrapped so it consumes a `&[u8]` and may panic.
    type Decoder = Box<dyn Fn(&[u8])>;
    let decoders: Vec<(&str, Decoder)> = vec![
        (
            "uint_vector_u32",
            Box::new(|b| drop(decode_uint_vector_u32(&mut &b[..]))),
        ),
        (
            "uint_vector_u64",
            Box::new(|b| drop(decode_uint_vector_u64(&mut &b[..]))),
        ),
        (
            "string_vector",
            Box::new(|b| drop(decode_string_vector(&mut &b[..]))),
        ),
        (
            "s2cell_id_vector",
            Box::new(|b| drop(decode_s2cell_id_vector(&mut &b[..]))),
        ),
        (
            "s2point_vector",
            Box::new(|b| drop(decode_s2point_vector(&mut &b[..]))),
        ),
        (
            "point",
            Box::new(|b| drop(<Point as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "cap",
            Box::new(|b| drop(<Cap as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "rect",
            Box::new(|b| drop(<Rect as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "cellid",
            Box::new(|b| drop(<CellId as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "cellunion",
            Box::new(|b| drop(<CellUnion as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "cell",
            Box::new(|b| drop(<Cell as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "polyline",
            Box::new(|b| drop(<Polyline as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "loop",
            Box::new(|b| drop(<Loop as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "polygon",
            Box::new(|b| drop(<Polygon as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "lax_polyline",
            Box::new(|b| drop(<LaxPolyline as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "lax_polygon",
            Box::new(|b| drop(<LaxPolygon as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "point_vector",
            Box::new(|b| drop(<PointVector as S2Decode>::decode(&mut &b[..]))),
        ),
        (
            "s2shape_index",
            Box::new(|b| {
                let mut idx = EncodedS2ShapeIndex::new();
                drop(idx.init(b));
            }),
        ),
        (
            "points_compressed",
            Box::new(|b| {
                if b.len() >= 3 {
                    let lvl = b[0] % 31;
                    let n = (u16::from_le_bytes([b[1], b[2]]) % 4096) as usize;
                    drop(decode_points_compressed(&mut &b[3..], lvl, n));
                }
            }),
        ),
    ];

    // Run the survey: record the first panicking input per decoder, along with
    // where it panicked (the hook captures the location into a thread-local;
    // silencing stderr so a clean run isn't buried in backtraces).
    thread_local! {
        static LAST_PANIC: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    }
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map_or_else(String::new, |l| format!("{}:{}", l.file(), l.line()));
        LAST_PANIC.with(|c| *c.borrow_mut() = loc);
    }));
    let mut failures: Vec<(&str, String, Vec<u8>)> = Vec::new();
    for (name, decode) in &decoders {
        for input in &corpus {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode(input)));
            if res.is_err() {
                let loc = LAST_PANIC.with(|c| c.borrow().clone());
                failures.push((name, loc, input.clone()));
                break;
            }
        }
    }
    std::panic::set_hook(prev_hook);

    assert!(
        failures.is_empty(),
        "decoders panicked on crafted input:\n{}",
        failures
            .iter()
            .map(|(n, loc, b)| format!("  {n} @ {loc}: {b:02x?}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
