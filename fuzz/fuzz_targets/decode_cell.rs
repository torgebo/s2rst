// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::Cell;
use s2rst::s2::encoding::{S2Decode, S2Encode};

// `Cell::decode` reads a packed cell id and reconstructs the cell's geometry,
// validating the id against the S2 face/level invariants. Beyond never
// panicking/aborting/hanging, a successfully decoded cell must re-encode and
// decode back to an equal cell — the id survives the round-trip intact.
fuzz_target!(|data: &[u8]| {
    let Ok(cell) = <Cell as S2Decode>::decode(&mut &data[..]) else {
        return;
    };
    let mut buf = Vec::new();
    cell.encode(&mut buf).expect("encoding to a Vec is infallible");
    let cell2 = <Cell as S2Decode>::decode(&mut &buf[..])
        .expect("re-decoding a self-encoded cell must succeed");
    assert_eq!(cell, cell2, "Cell encode/decode is not idempotent");
});
