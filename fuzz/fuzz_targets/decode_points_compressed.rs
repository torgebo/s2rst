// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::point_compression::decode_points_compressed;

// `decode_points_compressed` is the arithmetic-heavy compressed point codec:
// derivative-coded (pi, qi) deltas, run-length-encoded face indices, and exact
// off-center overrides. It is the codec family behind the one decode crash found
// so far (an overflow in the s2point_vector path). It must never
// panic/abort/hang on arbitrary bytes.
//
// Two of its three inputs are not part of the byte stream, so we derive them
// from the input prefix:
//   - `level`: a cell level in 0..=30. `Level::from(u8)` panics outside that
//     range, so we clamp with `% 31`.
//   - `num_points`: unlike the higher-level decoders, this function does NOT
//     bound the count itself — its callers cap it against MAX_ENCODED_VERTICES
//     before the up-front `Vec::with_capacity(num_points)`. We mirror that here
//     with a small cap so the harness can't trigger an allocation blowup that no
//     real caller can reach; the codec arithmetic is what we want to fuzz.
fuzz_target!(|data: &[u8]| {
    // Need 1 byte for the level and 2 for the count before the payload.
    if data.len() < 3 {
        return;
    }
    let level: u8 = data[0] % 31; // 0..=30 (a valid S2 cell level)
    let num_points = (u16::from_le_bytes([data[1], data[2]]) % 4096) as usize;
    let _ = decode_points_compressed(&mut &data[3..], level, num_points);
});
