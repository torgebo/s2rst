// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::polyline::Polyline;
use s2rst::s2::encoding::S2Decode;

// `Polyline::decode` reads a version byte, a vertex count, and the per-vertex
// point array. It must never panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = <Polyline as S2Decode>::decode(&mut &data[..]);
});
