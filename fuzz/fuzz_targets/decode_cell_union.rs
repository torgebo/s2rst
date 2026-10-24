// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::CellUnion;
use s2rst::s2::encoding::{S2Decode, S2Encode};

// `CellUnion::decode` reads a version byte, a cell count, and that many raw
// 64-bit cell ids, kept exactly as encoded; it rejects input whose ids are not a
// valid union (invalid, unsorted, duplicated or overlapping; spec 002, FR-017).
// Beyond never panicking/aborting/hanging, a successful decode is a valid union,
// so re-encoding it and decoding again must yield an equal union.
fuzz_target!(|data: &[u8]| {
    let Ok(cu) = <CellUnion as S2Decode>::decode(&mut &data[..]) else {
        return;
    };
    let mut buf = Vec::new();
    cu.encode(&mut buf).expect("encoding to a Vec is infallible");
    let cu2 = <CellUnion as S2Decode>::decode(&mut &buf[..])
        .expect("re-decoding a self-encoded cell union must succeed");
    assert_eq!(cu, cu2, "CellUnion encode/decode is not idempotent");
});
