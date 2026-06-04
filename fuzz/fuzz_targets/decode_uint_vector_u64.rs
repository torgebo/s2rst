// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::encoded_uint_vector::decode_uint_vector_u64;

// The decoder must never panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = decode_uint_vector_u64(&mut &data[..]);
});
