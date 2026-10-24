// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The decoder contract (spec 003 FR-004; data-model §3; research R4).
//!
//! The contract is stated **once**, in the generic bodies below, and instantiated per codec
//! through the [`Codec`] trait:
//!
//! 1. **Round trip**, by composition (research R4): [`check_encoder`] (the library writes the
//!    canonical encoding), [`check_agreement`] (the library's decoder agrees with the
//!    reference decoder on every input), and [`check_model_round_trip`] (the reference
//!    decoder inverts the canonical encoder). A harness that runs the library's encoder and
//!    decoder together does not fit the checker for the vector formats (research R0).
//! 2. **Exact consumption**: [`check_encoder`] compares lengths, [`check_agreement`] compares
//!    the bytes consumed.
//! 3. **Sound acceptance** and 4. **Rejection**: [`check_agreement`] compares acceptance and
//!    values with the reference description, which carries the documented validity
//!    conditions and deviations (`wire_spec`'s `lib_*` functions).
//! 5. **Bounded reservation** is not observable by the checker; its evidence is
//!    `verify/tests/reservation.rs` and the FR-019 regression test (research R7).
//!
//! Collections are compared element by element over a concrete maximum, the harness's own
//! length, never with `==`, which unrolls `memcmp` (spec 003 tasks.md, fact 4). A decoder's
//! error is forgotten, not dropped ([`discard`](crate::wire_domain::discard)).

use std::io;

use s2rst::r3::Vector;
use s2rst::s1::ChordAngle;
use s2rst::s2::encoded_s2cell_id_vector::{decode_s2cell_id_vector, encode_s2cell_id_vector};
use s2rst::s2::encoded_s2point_vector::{CodingHint, decode_s2point_vector, encode_s2point_vector};
use s2rst::s2::encoded_string_vector::{decode_string_vector, encode_string_vector};
use s2rst::s2::encoded_uint_vector::{
    decode_uint_vector_u32, decode_uint_vector_u64, encode_uint_vector_u32, encode_uint_vector_u64,
};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{Cap, Cell, CellId, Point, Rect};
use s2rst::{r1, s1};

use crate::wire_domain::{
    any_cap, any_ids, any_point, any_points, any_strings, any_u32s, any_u64s, any_valid_cell_id,
    any_valid_rect, cap_bits, point_bits, rect_bits,
};
use crate::wire_spec::{
    LIB_MAX_COUNT, LIB_MAX_POINTS, PointBits, PointVectorAnswer, cap_fields, dev_string_total,
    enc_cap, enc_cell_id, enc_cellid_vector, enc_point, enc_point_vector_fast, enc_rect,
    enc_uint_vector, lib_cell, lib_cell_id, lib_cellid_vector, lib_point_vector, lib_string_vector,
    lib_uint_vector, put_string_vector, rect_valid, ref_point, ref_rect,
};

/// The writer's capacity in the encoder harnesses: above the longest canonical encoding any of
/// them produces (49 bytes, two uncompressed points), so the writer never reallocates.
pub const ENCODING_MAX: usize = 64;

/// One codec of the decoding layer, as data-model §3 lists them.
pub trait Codec {
    /// The decoded value.
    type Value;
    /// A symbolic value of the round-trip domain, with exactly `N` elements for a vector
    /// codec (`N` is ignored by the fixed-size codecs).
    fn any_value<const N: usize>() -> Self::Value;
    /// The documented round-trip domain (`encodable` in data-model §3).
    fn encodable(v: &Self::Value) -> bool;
    /// The library's encoder.
    fn encode(v: &Self::Value, w: &mut Vec<u8>);
    /// The library's decoder.
    fn decode(r: &mut &[u8]) -> io::Result<Self::Value>;
    /// The canonical encoding of the reference description.
    fn reference_encode(v: &Self::Value) -> Vec<u8>;
    /// The reference description's answer, with the documented deviations (`lib_*`).
    fn reference_decode(b: &[u8]) -> Option<(Self::Value, usize)>;
    /// Whether the reference description speaks about this input at all (the point
    /// vector's compact branch is not modelled; spec FR-002).
    fn modelled(b: &[u8]) -> bool {
        let _ = b;
        true
    }
    /// Bit-exact equality, element by element over at most `MAX` elements (and `MAX` bytes per
    /// string). A harness passes its own bound, so a comparison loop never needs a larger
    /// unwind than the harness's input (a fixed-size codec ignores `MAX`).
    fn same<const MAX: usize>(a: &Self::Value, b: &Self::Value) -> bool;
}

/// Byte-string equality over a concrete maximum.
pub fn same_bytes<const MAX: usize>(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() || a.len() > MAX {
        return false;
    }
    for i in 0..MAX {
        if i < a.len() && a[i] != b[i] {
            return false;
        }
    }
    true
}

/// Slice equality over at most `MAX` elements with a per-element comparison.
pub fn same_slices<const MAX: usize, T>(a: &[T], b: &[T], eq: impl Fn(&T, &T) -> bool) -> bool {
    if a.len() != b.len() || a.len() > MAX {
        return false;
    }
    for i in 0..MAX {
        if i < a.len() && !eq(&a[i], &b[i]) {
            return false;
        }
    }
    true
}

/// Contract part 2 (encoder half) and the first leg of part 1: for every value of the domain
/// with `N` elements, the library writes exactly the canonical encoding. `E` bounds the
/// encoding's length (the harness's unwind must exceed it).
pub fn check_encoder<C: Codec, const N: usize, const E: usize>() {
    let v = C::any_value::<N>();
    kani::assume(C::encodable(&v));
    let mut w = Vec::with_capacity(ENCODING_MAX);
    C::encode(&v, &mut w);
    let want = C::reference_encode(&v);
    assert!(
        same_bytes::<E>(&w, &want),
        "the encoding differs from the reference"
    );
}

/// Contract parts 2 (decoder half), 3 and 4, and the second leg of part 1: on every
/// `L`-byte input the reference speaks about, the library's decoder and the reference
/// decoder accept the same inputs, return the same value and consume the same bytes.
pub fn check_agreement<C: Codec, const L: usize>(cover: impl FnOnce(&[u8], bool)) {
    let b: [u8; L] = kani::any();
    check_agreement_on::<C, L>(&b, cover);
}

/// [`check_agreement`] on a given input, for the families whose domain is a constrained part
/// of `Bytes(L)` (the caller states the constraint).
pub fn check_agreement_on<C: Codec, const L: usize>(b: &[u8; L], cover: impl FnOnce(&[u8], bool)) {
    kani::assume(C::modelled(b));
    let mut r = &b[..];
    let lib = C::decode(&mut r);
    let consumed = L - r.len();
    let want = C::reference_decode(b);
    let accepted = lib.is_ok();
    match (lib, want) {
        (Ok(v), Some((w, n))) => {
            assert!(C::same::<L>(&v, &w), "a different value");
            assert!(consumed == n, "a different number of bytes consumed");
        }
        (Err(e), None) => core::mem::forget(e),
        (Ok(_), None) => panic!("the library accepts an input the reference rejects"),
        (Err(_), Some(_)) => panic!("the library rejects an input the reference accepts"),
    }
    cover(b, accepted);
}

/// The third leg of part 1: the reference decoder inverts the canonical encoder, consuming
/// exactly the encoding, whatever follows it: `ref_decode(enc(v) + rest) = (v, |enc(v)|)`
/// (research R4). The encoding is copied into an `E`-byte buffer whose remaining bytes are
/// arbitrary, so every loop of the decoder has a concrete bound. `E` must exceed the longest
/// encoding of `N` values, so at least one byte follows it.
pub fn check_model_round_trip<C: Codec, const N: usize, const E: usize>() {
    let v = C::any_value::<N>();
    kani::assume(C::encodable(&v));
    let enc = C::reference_encode(&v);
    let n = enc.len();
    assert!(n < E, "the encoding is longer than the harness's buffer");
    let mut buf: [u8; E] = kani::any();
    for i in 0..E {
        if i < n {
            buf[i] = enc[i];
        }
    }
    match C::reference_decode(&buf) {
        Some((w, k)) => {
            // `E` bounds the element count and each string's length alike.
            assert!(
                C::same::<E>(&v, &w),
                "the model does not invert its encoder"
            );
            assert!(k == n, "the model consumes a different number of bytes");
        }
        None => panic!("the model rejects its own encoding"),
    }
}

// ---- the ten codecs (data-model §3; T041) ---------------------------------------------------

/// `U32Vec`: `encode_uint_vector_u32` / `decode_uint_vector_u32`.
pub struct U32Vec;
/// `U64Vec`: `encode_uint_vector_u64` / `decode_uint_vector_u64`.
pub struct U64Vec;
/// `IdVec`: `encode_s2cell_id_vector` / `decode_s2cell_id_vector`.
pub struct IdVec;
/// `StrVec`: `encode_string_vector` / `decode_string_vector`, with strings of up to 2 bytes
/// on the encoder side (research R5).
pub struct StrVec;
/// `PtVec`: `encode_s2point_vector` with `CodingHint::Fast` / `decode_s2point_vector`.
pub struct PtVec;
/// `CellIdC`: `impl S2Encode/S2Decode for CellId`.
pub struct CellIdC;
/// `PointC`: `impl S2Encode/S2Decode for Point`.
pub struct PointC;
/// `CapC`: `impl S2Encode/S2Decode for Cap`.
pub struct CapC;
/// `RectC`: `impl S2Encode/S2Decode for Rect`.
pub struct RectC;
/// `CellC`: `impl S2Encode/S2Decode for Cell`. Its value is the cell's identifier: a `Cell`'s
/// other fields are computed from it by `Cell::from`, which both sides call.
pub struct CellC;

fn widen(v: &[u32]) -> Vec<u64> {
    let mut out = Vec::with_capacity(v.len());
    for x in v {
        out.push(u64::from(*x));
    }
    out
}

fn ids(v: &[CellId]) -> Vec<u64> {
    let mut out = Vec::with_capacity(v.len());
    for c in v {
        out.push(c.0);
    }
    out
}

/// Bit patterns compared element by element: `==` on arrays unrolls `memcmp` (fact 4).
fn same_bits<const K: usize>(a: [u64; K], b: [u64; K]) -> bool {
    let mut i = 0;
    while i < K {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

fn same_point(a: &Point, b: &Point) -> bool {
    same_bits(point_bits(a), point_bits(b))
}

fn point_of(p: PointBits) -> Point {
    Point(Vector {
        x: f64::from_bits(p[0]),
        y: f64::from_bits(p[1]),
        z: f64::from_bits(p[2]),
    })
}

impl Codec for U32Vec {
    type Value = Vec<u32>;
    fn any_value<const N: usize>() -> Vec<u32> {
        any_u32s::<N>()
    }
    fn encodable(v: &Vec<u32>) -> bool {
        v.len() as u64 <= LIB_MAX_COUNT
    }
    fn encode(v: &Vec<u32>, w: &mut Vec<u8>) {
        assert!(encode_uint_vector_u32(v, w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Vec<u32>> {
        decode_uint_vector_u32(r)
    }
    fn reference_encode(v: &Vec<u32>) -> Vec<u8> {
        enc_uint_vector(&widen(v), 4)
    }
    fn reference_decode(b: &[u8]) -> Option<(Vec<u32>, usize)> {
        let (v, n) = lib_uint_vector(b, 4)?;
        let mut out = Vec::new();
        for x in v {
            out.push(u32::try_from(x).ok()?);
        }
        Some((out, n))
    }
    fn same<const MAX: usize>(a: &Vec<u32>, b: &Vec<u32>) -> bool {
        same_slices::<MAX, _>(a, b, |x, y| x == y)
    }
}

impl Codec for U64Vec {
    type Value = Vec<u64>;
    fn any_value<const N: usize>() -> Vec<u64> {
        any_u64s::<N>()
    }
    fn encodable(v: &Vec<u64>) -> bool {
        v.len() as u64 <= LIB_MAX_COUNT
    }
    fn encode(v: &Vec<u64>, w: &mut Vec<u8>) {
        assert!(encode_uint_vector_u64(v, w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Vec<u64>> {
        decode_uint_vector_u64(r)
    }
    fn reference_encode(v: &Vec<u64>) -> Vec<u8> {
        enc_uint_vector(v, 8)
    }
    fn reference_decode(b: &[u8]) -> Option<(Vec<u64>, usize)> {
        lib_uint_vector(b, 8)
    }
    fn same<const MAX: usize>(a: &Vec<u64>, b: &Vec<u64>) -> bool {
        same_slices::<MAX, _>(a, b, |x, y| x == y)
    }
}

impl Codec for IdVec {
    type Value = Vec<CellId>;
    fn any_value<const N: usize>() -> Vec<CellId> {
        any_ids::<N>()
    }
    fn encodable(v: &Vec<CellId>) -> bool {
        v.len() as u64 <= LIB_MAX_COUNT
    }
    fn encode(v: &Vec<CellId>, w: &mut Vec<u8>) {
        assert!(encode_s2cell_id_vector(v, w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Vec<CellId>> {
        decode_s2cell_id_vector(r)
    }
    fn reference_encode(v: &Vec<CellId>) -> Vec<u8> {
        enc_cellid_vector(&ids(v))
    }
    fn reference_decode(b: &[u8]) -> Option<(Vec<CellId>, usize)> {
        let (v, n) = lib_cellid_vector(b)?;
        let mut out = Vec::new();
        for x in v {
            out.push(CellId(x));
        }
        Some((out, n))
    }
    fn same<const MAX: usize>(a: &Vec<CellId>, b: &Vec<CellId>) -> bool {
        same_slices::<MAX, _>(a, b, |x, y| x.0 == y.0)
    }
}

impl Codec for StrVec {
    type Value = Vec<Vec<u8>>;
    fn any_value<const N: usize>() -> Vec<Vec<u8>> {
        any_strings::<N, 2>()
    }
    fn encodable(v: &Vec<Vec<u8>>) -> bool {
        let mut total = 0u64;
        for s in v {
            total += s.len() as u64;
        }
        !dev_string_total(total)
    }
    fn encode(v: &Vec<Vec<u8>>, w: &mut Vec<u8>) {
        let mut refs: Vec<&[u8]> = Vec::with_capacity(v.len());
        for s in v {
            refs.push(s.as_slice());
        }
        assert!(encode_string_vector(&refs, w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Vec<Vec<u8>>> {
        decode_string_vector(r)
    }
    fn reference_encode(v: &Vec<Vec<u8>>) -> Vec<u8> {
        let mut out = Vec::with_capacity(ENCODING_MAX);
        put_string_vector(&mut out, v);
        out
    }
    fn reference_decode(b: &[u8]) -> Option<(Vec<Vec<u8>>, usize)> {
        lib_string_vector(b)
    }
    fn same<const MAX: usize>(a: &Vec<Vec<u8>>, b: &Vec<Vec<u8>>) -> bool {
        same_slices::<MAX, _>(a, b, |x, y| same_bytes::<MAX>(x, y))
    }
}

impl Codec for PtVec {
    type Value = Vec<Point>;
    fn any_value<const N: usize>() -> Vec<Point> {
        any_points::<N>()
    }
    fn encodable(v: &Vec<Point>) -> bool {
        v.len() as u64 <= LIB_MAX_POINTS
    }
    fn encode(v: &Vec<Point>, w: &mut Vec<u8>) {
        assert!(encode_s2point_vector(v, CodingHint::Fast, w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Vec<Point>> {
        decode_s2point_vector(r)
    }
    fn reference_encode(v: &Vec<Point>) -> Vec<u8> {
        let mut bits = Vec::with_capacity(v.len());
        for p in v {
            bits.push(point_bits(p));
        }
        enc_point_vector_fast(&bits)
    }
    fn reference_decode(b: &[u8]) -> Option<(Vec<Point>, usize)> {
        match lib_point_vector(b) {
            PointVectorAnswer::Points(v, n) => {
                let mut out = Vec::new();
                for p in v {
                    out.push(point_of(p));
                }
                Some((out, n))
            }
            PointVectorAnswer::Reject | PointVectorAnswer::Compact => None,
        }
    }
    /// The compact branch (tag 1) is not modelled (FR-002); P-PV-03 covers it crash-only.
    fn modelled(b: &[u8]) -> bool {
        b.is_empty() || b[0] & 7 != 1
    }
    fn same<const MAX: usize>(a: &Vec<Point>, b: &Vec<Point>) -> bool {
        same_slices::<MAX, _>(a, b, same_point)
    }
}

impl Codec for CellIdC {
    type Value = CellId;
    fn any_value<const N: usize>() -> CellId {
        any_valid_cell_id().0
    }
    /// The generator builds only valid ids; this states the domain with the library's own
    /// predicate, which FR-022 documents.
    fn encodable(v: &CellId) -> bool {
        v.is_valid()
    }
    fn encode(v: &CellId, w: &mut Vec<u8>) {
        assert!(v.encode(w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<CellId> {
        CellId::decode(r)
    }
    fn reference_encode(v: &CellId) -> Vec<u8> {
        enc_cell_id(v.0)
    }
    fn reference_decode(b: &[u8]) -> Option<(CellId, usize)> {
        lib_cell_id(b).map(|(x, n)| (CellId(x), n))
    }
    fn same<const MAX: usize>(a: &CellId, b: &CellId) -> bool {
        a.0 == b.0
    }
}

impl Codec for PointC {
    type Value = Point;
    fn any_value<const N: usize>() -> Point {
        any_point()
    }
    fn encodable(_: &Point) -> bool {
        true
    }
    fn encode(v: &Point, w: &mut Vec<u8>) {
        assert!(v.encode(w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Point> {
        Point::decode(r)
    }
    fn reference_encode(v: &Point) -> Vec<u8> {
        enc_point(point_bits(v))
    }
    fn reference_decode(b: &[u8]) -> Option<(Point, usize)> {
        ref_point(b).map(|(p, n)| (point_of(p), n))
    }
    fn same<const MAX: usize>(a: &Point, b: &Point) -> bool {
        same_point(a, b)
    }
}

/// `CapC` uses research R18's fallback: acceptance is stated relative to the library's own
/// `Cap::is_valid` (in the trusted base), because the restated float predicate
/// [`cap_valid`](crate::wire_spec::cap_valid), asserted against the library's, timed out
/// (T048: `p_fx_03` and `p_fx_08__b32`, `__b33` over 5 minutes). The fields, their order, the
/// clamp of `length2` and the consumption are still the reference's.
impl Codec for CapC {
    type Value = Cap;
    fn any_value<const N: usize>() -> Cap {
        let c = any_cap();
        kani::assume(c.is_valid());
        c
    }
    fn encodable(v: &Cap) -> bool {
        v.is_valid()
    }
    fn encode(v: &Cap, w: &mut Vec<u8>) {
        assert!(v.encode(w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Cap> {
        Cap::decode(r)
    }
    fn reference_encode(v: &Cap) -> Vec<u8> {
        enc_cap(cap_bits(v))
    }
    fn reference_decode(b: &[u8]) -> Option<(Cap, usize)> {
        let c = cap_fields(b)?;
        // The same fields, read with `from_le_bytes` as the library reads them, so the checker
        // sees one expression per field and the float predicate is not compared against a
        // differently built copy of itself.
        let f = |k: usize| {
            u64::from_le_bytes([
                b[8 * k],
                b[8 * k + 1],
                b[8 * k + 2],
                b[8 * k + 3],
                b[8 * k + 4],
                b[8 * k + 5],
                b[8 * k + 6],
                b[8 * k + 7],
            ])
        };
        assert!(
            same_bits(c, [f(0), f(1), f(2), f(3)]),
            "the reference's fields"
        );
        // D-004, the documented deviation (`wire_spec::lib_cap`): a NaN `length2` is rejected
        // where upstream decodes a full cap. Since spec 007 FR-018, `from_length2(NaN)` is
        // upstream's `STRAIGHT`, so the rejection must be stated here.
        if f64::from_bits(f(3)).is_nan() {
            return None;
        }
        let cap = Cap::from_center_chord_angle(
            point_of([f(0), f(1), f(2)]),
            ChordAngle::from_length2(f64::from_bits(f(3))),
        );
        if cap.is_valid() {
            Some((cap, 32))
        } else {
            None
        }
    }
    fn same<const MAX: usize>(a: &Cap, b: &Cap) -> bool {
        same_bits(cap_bits(a), cap_bits(b))
    }
}

impl Codec for RectC {
    type Value = Rect;
    fn any_value<const N: usize>() -> Rect {
        any_valid_rect()
    }
    fn encodable(v: &Rect) -> bool {
        rect_valid(rect_bits(v))
    }
    fn encode(v: &Rect, w: &mut Vec<u8>) {
        assert!(v.encode(w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<Rect> {
        Rect::decode(r)
    }
    fn reference_encode(v: &Rect) -> Vec<u8> {
        enc_rect(rect_bits(v))
    }
    fn reference_decode(b: &[u8]) -> Option<(Rect, usize)> {
        let (r, n) = ref_rect(b)?;
        Some((
            Rect {
                lat: r1::Interval {
                    lo: f64::from_bits(r[0]),
                    hi: f64::from_bits(r[1]),
                },
                lng: s1::Interval {
                    lo: f64::from_bits(r[2]),
                    hi: f64::from_bits(r[3]),
                },
            },
            n,
        ))
    }
    fn same<const MAX: usize>(a: &Rect, b: &Rect) -> bool {
        same_bits(rect_bits(a), rect_bits(b))
    }
}

impl Codec for CellC {
    type Value = CellId;
    fn any_value<const N: usize>() -> CellId {
        any_valid_cell_id().0
    }
    /// The generator builds only valid ids; this states the domain with the library's own
    /// predicate, which FR-022 documents.
    fn encodable(v: &CellId) -> bool {
        v.is_valid()
    }
    fn encode(v: &CellId, w: &mut Vec<u8>) {
        assert!(Cell::from(*v).encode(w).is_ok());
    }
    fn decode(r: &mut &[u8]) -> io::Result<CellId> {
        Cell::decode(r).map(|c| c.id())
    }
    fn reference_encode(v: &CellId) -> Vec<u8> {
        enc_cell_id(v.0)
    }
    fn reference_decode(b: &[u8]) -> Option<(CellId, usize)> {
        lib_cell(b).map(|(x, n)| (CellId(x), n))
    }
    fn same<const MAX: usize>(a: &CellId, b: &CellId) -> bool {
        a.0 == b.0
    }
}
