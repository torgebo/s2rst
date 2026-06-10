// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::Loop;
use s2rst::s2::encoding::S2Decode;

// `Loop::decode` reads a version byte, a vertex count, and the per-vertex point
// array, plus the loop's origin/depth bookkeeping — the building block nested
// inside polygon decoding. It must never panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = <Loop as S2Decode>::decode(&mut &data[..]);
});
