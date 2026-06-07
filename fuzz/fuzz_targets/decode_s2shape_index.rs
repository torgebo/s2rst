// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::encoded_s2shape_index::EncodedS2ShapeIndex;

// `EncodedS2ShapeIndex::init` decodes a whole `MutableS2ShapeIndex` wire image:
// a version/options varint, a tag-dispatched vector of shapes (lax polyline /
// lax polygon / point vector), a cell-id vector, a cell-data string vector with
// a length cross-check, and per-cell edge lists. It nests every lower-level
// decoder, so it is the richest decode surface in the crate. It must never
// panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let mut index = EncodedS2ShapeIndex::new();
    let _ = index.init(data);
});
