// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! The SC-008 reservation measurement (spec 003 FR-019, research R7): each K2 input (at most
//! 16 bytes, declaring the largest count its decoder accepts, with none of the data) is
//! rejected after the decoder has allocated at most 2 MiB in total.
//!
//! Contract part 5 (bounded reservation) is not observable by the bounded checker, so this
//! test, with `core/tests/decoder_robustness.rs::reservation_under_address_space_limit`, is
//! its evidence. One test only: the instrumented allocator counts every thread.

use std::alloc::System;

use s2rst::s2::encoded_s2cell_id_vector::decode_s2cell_id_vector;
use s2rst::s2::encoded_s2point_vector::decode_s2point_vector;
use s2rst::s2::encoded_string_vector::decode_string_vector;
use s2rst::s2::encoded_uint_vector::{decode_uint_vector_u32, decode_uint_vector_u64};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const LIMIT: usize = 2 * 1024 * 1024;

fn leb128(mut x: u64) -> Vec<u8> {
    let mut out = Vec::new();
    while x >= 0x80 {
        out.push((x & 0x7f) as u8 | 0x80);
        x >>= 7;
    }
    out.push(x as u8);
    out
}

#[test]
fn k2_inputs_reserve_at_most_two_mib() {
    let u64_header = leb128(50_000_000 * 8);
    let mut cell_ids = vec![0x00];
    cell_ids.extend_from_slice(&u64_header);
    // One end offset of 500,000,000 (the largest the library accepts: it rejects a data length
    // above it), and no data. It was 499,999,999, one below (code review of v0.4.0..74b9bc1).
    let strings = s2rst_verify::wire_spec::enc_uint_vector(&[500_000_000], 8);
    type Decode = fn(&[u8]) -> bool;
    let cases: [(&str, Vec<u8>, Decode); 5] = [
        ("uint_vector_u64", u64_header.clone(), |b| {
            decode_uint_vector_u64(&mut &b[..]).is_err()
        }),
        ("uint_vector_u32", leb128(50_000_000 * 4), |b| {
            decode_uint_vector_u32(&mut &b[..]).is_err()
        }),
        ("s2cell_id_vector", cell_ids, |b| {
            decode_s2cell_id_vector(&mut &b[..]).is_err()
        }),
        ("string_vector", strings, |b| {
            decode_string_vector(&mut &b[..]).is_err()
        }),
        ("s2point_vector", leb128(50_000_000 << 3), |b| {
            decode_s2point_vector(&mut &b[..]).is_err()
        }),
    ];
    for (name, input, decode) in cases {
        assert!(input.len() <= 16, "{name}: input too long");
        let region = Region::new(GLOBAL);
        let rejected = decode(&input);
        let change = region.change();
        let reserved =
            change.bytes_allocated + usize::try_from(change.bytes_reallocated.max(0)).unwrap();
        assert!(rejected, "{name}: a truncated input was accepted");
        assert!(
            reserved <= LIMIT,
            "{name}: reserved {reserved} bytes (limit {LIMIT})"
        );
    }
}
