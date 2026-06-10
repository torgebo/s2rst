// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::encoding::S2Decode;

// `LaxPolyline::decode` reads a version byte and a packed point vector (the same
// encoded-point machinery as `decode_s2point_vector`). It must never
// panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = <LaxPolyline as S2Decode>::decode(&mut &data[..]);
});
