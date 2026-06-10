// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::Cap;
use s2rst::s2::encoding::{S2Decode, S2Encode};

// `Cap::decode` reads four raw `f64`s (center xyz + radius length²) with no
// version byte and validates the result against the `Cap` invariants. It is the
// only `S2Decode` impl not reachable from another target. Beyond the
// never-panic/abort/hang invariant, this target asserts encode/decode
// idempotence: a cap that decoded successfully is valid, so re-encoding it must
// produce bytes that decode again and re-encode identically. A mismatch would
// expose encode/decode disagreement or a value silently changing (e.g. integer
// wrap) on the way through — the class of bug that hides in `--release`.
fuzz_target!(|data: &[u8]| {
    let Ok(cap) = <Cap as S2Decode>::decode(&mut &data[..]) else {
        return;
    };
    let mut b1 = Vec::new();
    cap.encode(&mut b1).expect("encoding a Cap to a Vec is infallible");
    let cap2 = <Cap as S2Decode>::decode(&mut &b1[..])
        .expect("re-decoding self-encoded Cap bytes must succeed");
    let mut b2 = Vec::new();
    cap2.encode(&mut b2)
        .expect("encoding a Cap to a Vec is infallible");
    assert_eq!(b1, b2, "Cap encode/decode is not idempotent");
});
