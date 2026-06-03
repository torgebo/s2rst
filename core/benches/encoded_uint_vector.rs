// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `encoded_uint_vector_test.cc`
//! `BM_DecodeValue`, `BM_LowerBound`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::encoded_uint_vector::{
    decode_uint_vector_u32, decode_uint_vector_u64, encode_uint_vector_u32, encode_uint_vector_u64,
};

// C++: BM_DecodeValue<uint32_t> — encode then decode 1024 u32 values
#[library_benchmark]
fn encode_decode_u32_1024() -> Vec<u32> {
    let values: Vec<u32> = (0..1024).map(|i| i * 0xa5a5).collect();
    let mut buf = Vec::new();
    encode_uint_vector_u32(&values, &mut buf).unwrap();
    black_box(decode_uint_vector_u32(&mut buf.as_slice()).unwrap())
}

// C++: BM_DecodeValue<uint64_t> — encode then decode 1024 u64 values
#[library_benchmark]
fn encode_decode_u64_1024() -> Vec<u64> {
    let values: Vec<u64> = (0..1024).map(|i| i * 0xa5a5a5a5a5a5).collect();
    let mut buf = Vec::new();
    encode_uint_vector_u64(&values, &mut buf).unwrap();
    black_box(decode_uint_vector_u64(&mut buf.as_slice()).unwrap())
}

// Encode-only u32
#[library_benchmark]
fn encode_u32_1024() -> Vec<u8> {
    let values: Vec<u32> = (0..1024).map(|i| i * 0xa5a5).collect();
    let mut buf = Vec::with_capacity(4096);
    encode_uint_vector_u32(&values, &mut buf).unwrap();
    black_box(buf)
}

// Decode-only u32
#[library_benchmark]
fn decode_u32_1024() -> Vec<u32> {
    let values: Vec<u32> = (0..1024).map(|i| i * 0xa5a5).collect();
    let mut buf = Vec::new();
    encode_uint_vector_u32(&values, &mut buf).unwrap();
    black_box(decode_uint_vector_u32(&mut buf.as_slice()).unwrap())
}

// Large vector: 64K u32
#[library_benchmark]
fn encode_decode_u32_65536() -> Vec<u32> {
    let values: Vec<u32> = (0..65536).collect();
    let mut buf = Vec::new();
    encode_uint_vector_u32(&values, &mut buf).unwrap();
    black_box(decode_uint_vector_u32(&mut buf.as_slice()).unwrap())
}

library_benchmark_group!(
    name = encoded_uint_vector_benchmarks;
    benchmarks =
        encode_decode_u32_1024,
        encode_decode_u64_1024,
        encode_u32_1024,
        decode_u32_1024,
        encode_decode_u32_65536
);

main!(library_benchmark_groups = encoded_uint_vector_benchmarks);
