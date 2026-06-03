// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `value_lexicon_test.cc`
//! `BM_AddInt64`, `BM_AddS2Point`, `BM_AddS2PointPairs`, `BM_FindS2Point`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::value_lexicon::ValueLexicon;

// C++: BM_AddInt64 — add 10_000 unique i64 values
#[library_benchmark]
fn add_int64_10k() -> u32 {
    let mut lex = ValueLexicon::<i64>::new();
    for i in 0..10_000_i64 {
        lex.add(i);
    }
    black_box(lex.add(0)) // find existing
}

// C++: BM_AddInt64 — add 100_000 unique i64 values
#[library_benchmark]
fn add_int64_100k() -> u32 {
    let mut lex = ValueLexicon::<i64>::new();
    for i in 0..100_000_i64 {
        lex.add(i);
    }
    black_box(lex.add(0))
}

// C++: BM_AddS2PointPairs — add 10_000 pairs (each added twice)
#[library_benchmark]
fn add_pairs_10k() -> u32 {
    let mut lex = ValueLexicon::<i64>::new();
    for i in 0..10_000_i64 {
        lex.add(i);
        lex.add(i); // duplicate
    }
    black_box(lex.add(0))
}

// C++: BM_FindS2Point — find all 10_000 existing entries
#[library_benchmark]
fn find_10k() -> u32 {
    let mut lex = ValueLexicon::<i64>::new();
    for i in 0..10_000_i64 {
        lex.add(i);
    }
    // Now re-add (= find) all of them.
    let mut last = 0u32;
    for i in 0..10_000_i64 {
        last = lex.add(i);
    }
    black_box(last)
}

library_benchmark_group!(
    name = value_lexicon_benchmarks;
    benchmarks =
        add_int64_10k,
        add_int64_100k,
        add_pairs_10k,
        find_10k
);

main!(library_benchmark_groups = value_lexicon_benchmarks);
