// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Crash-freedom harnesses for the decoding layer (spec 003 FR-005; `decoding/INVENTORY.md`)
//! that no property harness subsumes.
//!
//! A harness runs one operation with every Kani check on: a panic, an arithmetic overflow,
//! an out-of-bounds access or an unwinding failure fails it. Covers show that each outcome a
//! member can reach is reachable. Most crash obligations are discharged by the property
//! harnesses, which run the same operations over the same domains with every check on
//! (T043–T048): the fixed-width helpers by `cd_fw`, the decoders by the agreement families of
//! `cd_uv`, `cd_cv`, `cd_pv` and `cd_fx`, the vector encoders by the encoder agreements. What
//! is left here:
//! - the five fixed-size encoders, over every value (the round trips cover valid values only);
//! - the `Cap` decoder at its full length (32 and 33 bytes): its agreement there, P-FX-08,
//!   runs only weekly (R18), and these two cost 4 s;
//! - the point vector decoder on every input of up to 1 byte, tag 1 included (the agreement
//!   does not model the compact branch, which is a tool limit from 2 bytes, T029);
//! - the cell-id vector encoder at one and two ids (weekly): its agreement, P-CV-03, fits only
//!   for zero ids.
#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use std::io::ErrorKind;

#[cfg(feature = "slow")]
use s2rst::s2::encoded_s2cell_id_vector::encode_s2cell_id_vector;
use s2rst::s2::encoded_s2point_vector::decode_s2point_vector;
use s2rst::s2::encoded_uint_vector::{
    decode_uint_with_length, encode_uint_with_length, get_uint_with_length,
};
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{Cap, Cell, CellId};

#[cfg(feature = "slow")]
use crate::wire_domain::any_ids;
use crate::wire_domain::{any_cap, any_point, any_rect, any_valid_cell_id, discard, with_bytes};

/// The first `k` bytes all have the continuation bit set.
pub(crate) fn continued(b: &[u8], k: usize) -> bool {
    let mut i = 0;
    while i < k {
        if b[i] & 0x80 == 0 {
            return false;
        }
        i += 1;
    }
    true
}

pub(crate) fn is_kind<T>(r: &std::io::Result<T>, kind: ErrorKind) -> bool {
    r.as_ref().is_err_and(|e| e.kind() == kind)
}

// ---- the point vector decoder, up to 1 byte (T036) ----------------------------------------

/// C-encoded_s2point_vector::decode_s2point_vector: decoding never panics, overflows or loops
/// on the empty input (FR-005).
#[kani::proof]
#[kani::unwind(2)]
fn c_decode_s2point_vector__b0() {
    with_bytes::<0>(|b| {
        let r = decode_s2point_vector(&mut &b[..]);
        kani::cover!(r.is_err(), "the empty input rejected");
        discard(r);
    });
}

/// C-encoded_s2point_vector::decode_s2point_vector: decoding never panics, overflows or loops
/// on any 1-byte input, every format tag (FR-005). Tags 2–7 are rejected after the first byte,
/// so this covers every longer input with those tags.
#[kani::proof]
#[kani::unwind(2)]
fn c_decode_s2point_vector__b1() {
    with_bytes::<1>(|b| {
        let r = decode_s2point_vector(&mut &b[..]);
        kani::cover!(r.as_ref().is_ok_and(Vec::is_empty), "no points (tag 0)");
        kani::cover!(
            is_kind(&r, ErrorKind::UnexpectedEof) && b[0] & 7 == 0,
            "truncated (tag 0)"
        );
        kani::cover!(r.is_err() && b[0] & 7 == 1, "tag 1 truncated");
        kani::cover!(r.is_err() && b[0] & 7 >= 2, "an unknown tag rejected");
        discard(r);
    });
}

// ---- the Cap decoder at full length (T031) -------------------------------------------------

/// C-impl S2Decode for Cap: decoding never panics or overflows on any input of exactly 32
/// bytes (FR-005). P-FX-08 proves the agreement here weekly.
#[kani::proof]
#[kani::unwind(2)]
fn c_decode_cap__b32() {
    with_bytes::<32>(|b| {
        let r = <Cap as S2Decode>::decode(&mut &b[..]);
        kani::cover!(r.is_ok(), "a cap");
        kani::cover!(r.is_err(), "an invalid cap rejected");
        discard(r);
    });
}

/// C-impl S2Decode for Cap: as `__b32`, with one byte left unread after the encoding.
#[kani::proof]
#[kani::unwind(2)]
fn c_decode_cap__b33() {
    with_bytes::<33>(|b| {
        let mut input = b;
        let r = <Cap as S2Decode>::decode(&mut input);
        kani::cover!(r.is_ok() && input.len() == 1, "one byte left unread");
        discard(r);
    });
}

// ---- the fixed-size encoders (T032) --------------------------------------------------------

/// C-impl S2Encode for CellId: every 64-bit value.
#[kani::proof]
fn c_encode_cell_id() {
    let mut w = Vec::with_capacity(40);
    assert!(CellId(kani::any()).encode(&mut w).is_ok());
    kani::cover!(w.len() == 8, "8 bytes written");
}

/// C-impl S2Encode for Point: every bit pattern.
#[kani::proof]
fn c_encode_point() {
    let mut w = Vec::with_capacity(40);
    assert!(any_point().encode(&mut w).is_ok());
    kani::cover!(w.len() == 24, "24 bytes written");
}

/// C-impl S2Encode for Cap: `Cap::from_center_chord_angle` of any bits.
#[kani::proof]
fn c_encode_cap() {
    let mut w = Vec::with_capacity(40);
    assert!(any_cap().encode(&mut w).is_ok());
    kani::cover!(w.len() == 32, "32 bytes written");
}

/// C-impl S2Encode for Rect: any four `f64` in the public fields.
#[kani::proof]
fn c_encode_rect() {
    let mut w = Vec::with_capacity(40);
    assert!(any_rect().encode(&mut w).is_ok());
    kani::cover!(w.len() == 33, "33 bytes written");
}

/// C-impl S2Encode for Cell: every cell (a `Cell` exists only for a valid id).
#[kani::proof]
#[kani::unwind(9)]
fn c_encode_cell() {
    let (id, _) = any_valid_cell_id();
    let mut w = Vec::with_capacity(40);
    assert!(Cell::from(id).encode(&mut w).is_ok());
    kani::cover!(w.len() == 8, "8 bytes written");
}

// ---- the cell-id vector encoder, weekly (T037) --------------------------------------------
/// C-encoded_s2cell_id_vector::encode_s2cell_id_vector: never panics or fails for any 1 value(s).
///
/// Weekly tier: out of memory at 4 GiB; 187.9 s at 8 GiB (T040).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
fn c_encode_s2cell_id_vector__n1() {
    let v = any_ids::<1>();
    let mut w = Vec::with_capacity(64);
    assert!(encode_s2cell_id_vector(&v, &mut w).is_ok());
    kani::cover!(w[0] & 7 == 0, "base length 0");
    kani::cover!(w[0] >> 3 == 31, "the extended shift code");
    kani::cover!(w[0] >> 3 < 31 && w[0] >> 3 >= 29, "an odd shift of 1 or 3");
}
/// C-encoded_s2cell_id_vector::encode_s2cell_id_vector: never panics or fails for any 2 value(s).
///
/// Weekly tier: out of memory at 4 GiB; 235.0 s at 8 GiB (T040).
#[cfg(feature = "slow")]
#[kani::proof]
#[kani::unwind(11)]
fn c_encode_s2cell_id_vector__n2() {
    let v = any_ids::<2>();
    let mut w = Vec::with_capacity(64);
    assert!(encode_s2cell_id_vector(&v, &mut w).is_ok());
    kani::cover!(w[0] & 7 == 0, "base length 0");
    kani::cover!(w[0] >> 3 == 31, "the extended shift code");
    kani::cover!(w[0] >> 3 < 31 && w[0] >> 3 >= 29, "an odd shift of 1 or 3");
    kani::cover!(w[0] & 7 == 7, "base length 7");
}

// ---------------------------------------------------------------------------------------------
// Outside the domain: the fixed-width helpers' documented panics (gap 1; `crash.rs` explains
// the `n_*` convention). Each panics in every build, so there is no `r_*` counterpart.
// ---------------------------------------------------------------------------------------------

/// N-encode_uint_with_length, N-decode_uint_with_length: each panics for every length above 8,
/// in every build, whatever the value or the input (the slice of the 8-byte buffer comes before
/// any write or read).
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(10)]
fn n_uint_with_length() {
    let len: usize = kani::any();
    kani::assume(len > 8);
    if kani::any::<bool>() {
        let mut w = Vec::with_capacity(8);
        discard(encode_uint_with_length(&mut w, kani::any(), len));
    } else {
        with_bytes::<9>(|d| {
            let mut r = d;
            discard(decode_uint_with_length(&mut r, len));
        });
    }
    kani::cover!(true, "returns");
}

/// N-get_uint_with_length: panics for every length above 8 and for every input shorter than the
/// length, in every build (inputs of up to 9 bytes).
#[kani::proof]
#[kani::should_panic]
#[kani::unwind(10)]
fn n_get_uint_with_length() {
    let d: [u8; 9] = kani::any();
    let k: usize = kani::any();
    kani::assume(k <= 9);
    let len: usize = kani::any();
    kani::assume(len > 8 || k < len);
    let _ = get_uint_with_length(&d[..k], len);
    kani::cover!(true, "returns");
}
