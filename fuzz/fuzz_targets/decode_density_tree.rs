// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::density_tree::S2DensityTree;

// `S2DensityTree::init` parses a custom varint-header tree format from arbitrary
// bytes, and `decode` then walks the parsed tree into a `CellId -> weight` map.
// Neither path is exercised by the bounded `decoder_robustness.rs` suite, so
// this is an otherwise-uncovered untrusted-input surface. Fuzz both: the parse
// and the subsequent tree walk must never panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let mut tree = S2DensityTree::new();
    if tree.init(data).is_ok() {
        let _ = tree.decode();
    }
});
