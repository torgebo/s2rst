// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The xlarge tier: crash-freedom members that exceed the weekly budget (8 GiB, 30 minutes),
//! for a machine with 32 GB of memory (`make verify-xlarge`; behind the `xlarge` feature).
//!
//! These are the tool limits of spec 003 T029 (`decoding/EVALUATION.md`, "Exclusions and tool
//! limits"), and the property members that ran out of memory at 8 GiB:
//! - the string vector decoder from 1 byte;
//! - the point vector decoder from 2 bytes, both format branches;
//! - the cell-id vector's model round trip at two ids (P-CV-01, T045);
//! - the cell-id vector's encoder agreement at one and two ids (P-CV-03, T045);
//! - the bounded agreement of the restated `u64`, `u32` and string vector encoders at two values
//!   (FR-011, T058).
//!
//! Locally every member ran out of solver memory or time. A member that passes here moves its
//! entry's bound up: record the run (machine, ceiling, time) in "Bounds measured first", then
//! write the family's missing lengths up to the largest one that fits.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use std::io::ErrorKind;

use s2rst::s2::encoded_s2point_vector::decode_s2point_vector;
use s2rst::s2::encoded_string_vector::decode_string_vector;

use crate::cd_crash::{continued, is_kind};
use s2rst::s2::encoded_string_vector::encode_string_vector;
use s2rst::s2::encoded_uint_vector::{encode_uint_vector_u32, encode_uint_vector_u64};
use s2rst_deductive::{string_vector, uint_vector};

use crate::codec::{IdVec, check_encoder, check_model_round_trip, same_bytes};
use crate::wire_domain::{any_strings, any_u32s, any_u64s};
use crate::wire_domain::{discard, with_bytes};

/// One string vector member per `name = length => covers`, unwind `length + 2` (T029's
/// value: the offsets loop and the data read).
macro_rules! string_crash {
    ($($name:ident = $len:literal, $unwind:literal => |$b:ident, $r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!("C-encoded_string_vector::decode_string_vector: decoding never panics, overflows or loops on any input of exactly ", stringify!($len), " bytes (FR-005). Xlarge tier.")]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            with_bytes::<$len>(|$b| {
                let $r = decode_string_vector(&mut &$b[..]);
                let () = $covers;
                discard($r);
            });
        }
    )* };
}

string_crash!(
    c_decode_string_vector__b1 = 1, 3 => |_b, r| {
        kani::cover!(r.as_ref().is_ok_and(Vec::is_empty), "zero strings");
        kani::cover!(is_kind(&r, ErrorKind::UnexpectedEof), "the offsets truncated");
    },
    c_decode_string_vector__b2 = 2, 4 => |_b, r| {
        kani::cover!(r.as_ref().is_ok_and(|v| v.len() == 1 && v[0].is_empty()), "one empty string");
        kani::cover!(is_kind(&r, ErrorKind::UnexpectedEof), "the data truncated");
    },
    c_decode_string_vector__b3 = 3, 5 => |b, r| {
        kani::cover!(r.as_ref().is_ok_and(|v| v.len() == 1 && v[0].len() == 1), "one one-byte string");
        kani::cover!(
            r.as_ref().is_ok_and(|v| v.len() == 2 && v[0].is_empty()) && b[1] > b[2],
            "an out-of-order entry decoded as empty (FR-020)"
        );
    },
    c_decode_string_vector__b4 = 4, 6 => |_b, r| {
        kani::cover!(r.as_ref().is_ok_and(|v| v.len() == 2 && v[1].len() == 1), "two strings, the second one byte");
    },
);

/// A point-vector input whose first byte has the given format tag (`k` < 2) or a tag of 2–7
/// (`k` = 2).
fn with_point_tag<const L: usize>(tag: u8, f: impl FnOnce(&[u8])) {
    with_bytes::<L>(|b| {
        if L > 0 {
            if tag < 2 {
                kani::assume(b[0] & 7 == tag);
            } else {
                kani::assume(b[0] & 7 >= 2);
            }
        }
        f(b);
    });
}

/// One point vector member per `name = tag, length, unwind => covers`.
macro_rules! point_crash {
    ($($name:ident = $tag:literal, $len:literal, $unwind:literal => |$b:ident, $r:ident| $covers:block),* $(,)?) => { $(
        #[doc = concat!("C-encoded_s2point_vector::decode_s2point_vector: decoding never panics, overflows or loops on any input of exactly ", stringify!($len), " bytes whose format tag is ", stringify!($tag), " (FR-005). Xlarge tier.")]
        #[kani::proof]
        #[kani::unwind($unwind)]
        fn $name() {
            with_point_tag::<$len>($tag, |$b| {
                let $r = decode_s2point_vector(&mut &$b[..]);
                let () = $covers;
                discard($r);
            });
        }
    )* };
}

point_crash!(
    c_decode_s2point_vector__t0_b2 = 0, 2, 3 => |_b, r| {
        kani::cover!(r.as_ref().is_ok_and(Vec::is_empty), "no points, a byte left over");
        kani::cover!(is_kind(&r, ErrorKind::UnexpectedEof), "truncated");
    },
    c_decode_s2point_vector__t0_b5 = 0, 5, 6 => |b, r| {
        kani::cover!(
            is_kind(&r, ErrorKind::InvalidData) && continued(b, 4) && b[4] >= 2 && b[4] < 0x80,
            "a count over the limit (5-byte header)"
        );
    },
    c_decode_s2point_vector__t0_b11 = 0, 11, 12 => |b, r| {
        kani::cover!(
            is_kind(&r, ErrorKind::InvalidData) && continued(b, 9) && b[9] > 1 && b[9] < 0x80,
            "a varint whose tenth byte is above 1 (D-003)"
        );
        kani::cover!(is_kind(&r, ErrorKind::UnexpectedEof), "truncated");
    },
    c_decode_s2point_vector__t0_b25 = 0, 25, 26 => |_b, r| {
        kani::cover!(r.as_ref().is_ok_and(|v| v.len() == 1), "one point");
    },
    c_decode_s2point_vector__t1_b2 = 1, 2, 3 => |_b, r| {
        kani::cover!(r.is_err(), "rejected or truncated");
    },
    c_decode_s2point_vector__t1_b11 = 1, 11, 12 => |_b, r| {
        kani::cover!(r.is_err(), "rejected or truncated");
    },
);

/// P-CV-01 (round trip): every vector of cell ids, of any bits, round-trips through the cell-id
/// vector encoding.
///
/// The model's leg at two ids: the reference description decodes its own encoding of every pair
/// of identifiers, whatever follows it, consuming exactly the encoding. Out of memory at 4 GiB
/// and at 8 GiB (174.7 s, T045). Xlarge tier.
#[kani::proof]
#[kani::unwind(28)]
fn p_cv_mrt__n2() {
    check_model_round_trip::<IdVec, 2, 27>();
}

/// P-CV-03 (exact output): the cell-id vector encoder chooses the base, the shift and the delta
/// width as upstream's search does.
///
/// At one id: the library writes upstream's encoding, base and shift as upstream's search
/// chooses them. Out of memory at 8 GiB (75.0 s, T045). Xlarge tier.
#[kani::proof]
#[kani::unwind(19)]
fn p_cv_03__n1() {
    check_encoder::<IdVec, 1, 18>();
}

/// P-CV-03 (exact output): the cell-id vector encoder chooses the base, the shift and the delta
/// width as upstream's search does.
///
/// At two ids. Out of memory at 8 GiB (154.6 s, T045). Xlarge tier.
#[kani::proof]
#[kani::unwind(27)]
fn p_cv_03__n2() {
    check_encoder::<IdVec, 2, 26>();
}

/// Bounded agreement (model check), `encode_uint_vector_u32` at two values: the library and the
/// restated encoder write the same bytes (FR-011). Out of memory at 8 GiB (47.8 s, T058). Xlarge
/// tier.
#[kani::proof]
#[kani::unwind(11)]
fn a_uv32_enc__n2() {
    let v = any_u32s::<2>();
    let (mut a, mut b) = (Vec::with_capacity(64), Vec::with_capacity(64));
    assert!(encode_uint_vector_u32(&v, &mut a).is_ok());
    let _ = uint_vector::encode_uint_vector_u32(&v, &mut b);
    assert!(same_bytes::<9>(&a, &b), "the encodings differ");
    kani::cover!(!a.is_empty(), "written");
}

/// Bounded agreement (model check), `encode_string_vector` at two strings of up to 2 bytes
/// (FR-011). Out of memory at 4 and 8 GiB (T058). Xlarge tier.
#[kani::proof]
#[kani::unwind(11)]
fn a_sv_enc__n2() {
    let v = any_strings::<2, 2>();
    let mut refs: Vec<&[u8]> = Vec::with_capacity(2);
    for s in &v {
        refs.push(s.as_slice());
    }
    let (mut a, mut b) = (Vec::with_capacity(64), Vec::with_capacity(64));
    assert!(encode_string_vector(&refs, &mut a).is_ok());
    let _ = string_vector::encode_string_vector(&v, &mut b);
    assert!(same_bytes::<8>(&a, &b), "the encodings differ");
    kani::cover!(!a.is_empty(), "written");
}

/// Bounded agreement (model check), `encode_uint_vector_u64` at two values (FR-011). Out of memory
/// at 8 GiB (42.5 s, T058). Xlarge tier.
#[kani::proof]
#[kani::unwind(18)]
fn a_uv64_enc__n2() {
    let v = any_u64s::<2>();
    let (mut a, mut b) = (Vec::with_capacity(64), Vec::with_capacity(64));
    assert!(encode_uint_vector_u64(&v, &mut a).is_ok());
    let _ = uint_vector::encode_uint_vector_u64(&v, &mut b);
    assert!(same_bytes::<17>(&a, &b), "the encodings differ");
    kani::cover!(!a.is_empty(), "written");
}
