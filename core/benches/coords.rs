// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2coords_test.cc`
//! `BM_STtoIJ`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::coords;

// C++: BM_STtoIJ — convert 1024 ST values to IJ
#[library_benchmark]
fn st_to_ij_1024() -> i32 {
    let mut result = 0i32;
    for i in 0..1024_u32 {
        let s = f64::from(i) / 1024.0;
        result = coords::st_to_ij(black_box(s));
    }
    black_box(result)
}

// Single ST → IJ conversion
#[library_benchmark]
fn st_to_ij_single() -> i32 {
    black_box(coords::st_to_ij(black_box(0.5)))
}

library_benchmark_group!(
    name = coords_benchmarks;
    benchmarks =
        st_to_ij_1024,
        st_to_ij_single
);

main!(library_benchmark_groups = coords_benchmarks);
