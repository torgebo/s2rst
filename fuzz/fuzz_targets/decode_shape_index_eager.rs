// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::shape_index::ShapeIndex;

// `ShapeIndex::decode_from_reader` is the `MutableS2ShapeIndex` wire-format
// decoder. The Rust port decodes eagerly — materializing every shape and cell
// up front — rather than lazily as C++'s `EncodedS2ShapeIndex` does;
// `EncodedS2ShapeIndex::init` is in fact a thin wrapper that calls straight into
// this same routine. This target is therefore the minimal *decode-only* surface
// for that routine, while `decode_s2shape_index` reuses the decode and then
// queries the result. It must never panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let _ = ShapeIndex::decode_from_reader(&mut &data[..]);
});
