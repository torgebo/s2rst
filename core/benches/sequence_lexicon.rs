// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `sequence_lexicon_test.cc`
//! `BM_AddInt32Sequence`, `BM_FindInt32Sequence`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s2::sequence_lexicon::SequenceLexicon;

// C++: BM_AddInt32Sequence (size=1, 10_000 sequences)
#[library_benchmark]
fn add_seq_size1_10k() -> u32 {
    let mut lex = SequenceLexicon::<i32>::new();
    for i in 0..10_000_i32 {
        lex.add(&[i]);
    }
    black_box(lex.add(&[0]))
}

// C++: BM_AddInt32Sequence (size=2, 10_000 sequences)
#[library_benchmark]
fn add_seq_size2_10k() -> u32 {
    let mut lex = SequenceLexicon::<i32>::new();
    for i in 0..10_000_i32 {
        lex.add(&[i, i + 1]);
    }
    black_box(lex.add(&[0, 1]))
}

// C++: BM_AddInt32Sequence (size=10, 10_000 sequences)
#[library_benchmark]
fn add_seq_size10_10k() -> u32 {
    let mut lex = SequenceLexicon::<i32>::new();
    for i in 0..10_000_i32 {
        let seq: Vec<i32> = (0..10).map(|j| i + j).collect();
        lex.add(&seq);
    }
    black_box(lex.add(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]))
}

// C++: BM_FindInt32Sequence (size=1, find all 10_000)
#[library_benchmark]
fn find_seq_size1_10k() -> u32 {
    let mut lex = SequenceLexicon::<i32>::new();
    for i in 0..10_000_i32 {
        lex.add(&[i]);
    }
    let mut last = 0u32;
    for i in 0..10_000_i32 {
        last = lex.add(&[i]);
    }
    black_box(last)
}

// C++: BM_FindInt32Sequence (size=10, find all 10_000)
#[library_benchmark]
fn find_seq_size10_10k() -> u32 {
    let mut lex = SequenceLexicon::<i32>::new();
    for i in 0..10_000_i32 {
        let seq: Vec<i32> = (0..10).map(|j| i + j).collect();
        lex.add(&seq);
    }
    let mut last = 0u32;
    for i in 0..10_000_i32 {
        let seq: Vec<i32> = (0..10).map(|j| i + j).collect();
        last = lex.add(&seq);
    }
    black_box(last)
}

library_benchmark_group!(
    name = sequence_lexicon_benchmarks;
    benchmarks =
        add_seq_size1_10k,
        add_seq_size2_10k,
        add_seq_size10_10k,
        find_seq_size1_10k,
        find_seq_size10_10k
);

main!(library_benchmark_groups = sequence_lexicon_benchmarks);
