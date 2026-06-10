// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::encoded_uint_vector::{decode_uint_vector_u32, encode_uint_vector_u32};

// The decoder must never panic/abort/hang on arbitrary bytes. It must also
// round-trip: re-encoding the decoded values and decoding again must reproduce
// them exactly. A mismatch would mean a width/length computation in the
// (de)serializer dropped or wrapped a value — a corruption that is silent in a
// `--release` build (no overflow checks) but is caught here.
fuzz_target!(|data: &[u8]| {
    let Ok(v) = decode_uint_vector_u32(&mut &data[..]) else {
        return;
    };
    let mut buf = Vec::new();
    encode_uint_vector_u32(&v, &mut buf).expect("encoding to a Vec is infallible");
    let v2 = decode_uint_vector_u32(&mut &buf[..])
        .expect("re-decoding self-encoded bytes must succeed");
    assert_eq!(v, v2, "u32 vector encode/decode is not idempotent");
});
