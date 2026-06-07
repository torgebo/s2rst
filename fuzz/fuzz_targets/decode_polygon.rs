// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::Polygon;
use s2rst::s2::encoding::S2Decode;

// `Polygon::decode` dispatches on a version byte to either the lossless format
// (per-loop vertex arrays) or the compressed format (snap level + derivative-
// coded points), with nested loop counts and depth fields. It must never
// panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = <Polygon as S2Decode>::decode(&mut &data[..]);
});
