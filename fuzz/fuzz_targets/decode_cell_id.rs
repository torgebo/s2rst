// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::CellId;
use s2rst::s2::encoding::{S2Decode, S2Encode};

// The binary `CellId` decoder (spec 003 FR-023): 8 little-endian bytes, rejected unless a valid cell (a documented deviation from C++, which accepts any 64 bits). Beyond the
// never-panic/abort/hang invariant, a value that decoded must re-encode to bytes that decode
// again and re-encode identically (compared as bytes, so NaN payloads count too).
fuzz_target!(|data: &[u8]| {
    let Ok(v) = <CellId as S2Decode>::decode(&mut &data[..]) else {
        return;
    };
    let mut b1 = Vec::new();
    v.encode(&mut b1).expect("encoding to a Vec is infallible");
    let v2 = <CellId as S2Decode>::decode(&mut &b1[..]).expect("re-decoding self-encoded bytes must succeed");
    let mut b2 = Vec::new();
    v2.encode(&mut b2).expect("encoding to a Vec is infallible");
    assert_eq!(b1, b2, "CellId encode/decode is not idempotent");
});
