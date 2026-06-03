// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2polyline_alignment_test.cc`
//! `BM_GetExactVertexAlignment`, `BM_GetExactVertexAlignmentCost`,
//! `BM_GetApproxVertexAlignment`, `BM_ComputeMedoid`, `BM_ComputeConsensus`*
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::polyline::Polyline;
use s2rst::s2::polyline_alignment::{
    ConsensusOptions, MedoidOptions, VertexAlignment, get_approx_vertex_alignment,
    get_consensus_polyline, get_exact_vertex_alignment, get_exact_vertex_alignment_cost,
    get_medoid_polyline,
};
use s2rst::s2::{LatLng, Loop};

// Create a polyline from a regular loop with `n` vertices, slightly perturbed.
#[inline(never)]
fn make_polyline(n: usize, offset: f64) -> Polyline {
    let center = LatLng::from_degrees(offset, offset * 1.3).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    Polyline::new((0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect())
}

#[inline(never)]
fn make_pair(n: usize) -> (Polyline, Polyline) {
    (make_polyline(n, 0.0), make_polyline(n, 0.001))
}

// C++: BM_GetExactVertexAlignmentCost (16 vertices)
#[library_benchmark]
fn exact_alignment_cost_16() -> f64 {
    let (a, b) = make_pair(16);
    black_box(get_exact_vertex_alignment_cost(&a, &b))
}

// C++: BM_GetExactVertexAlignmentCost (64 vertices)
#[library_benchmark]
fn exact_alignment_cost_64() -> f64 {
    let (a, b) = make_pair(64);
    black_box(get_exact_vertex_alignment_cost(&a, &b))
}

// C++: BM_GetExactVertexAlignment (16 vertices)
#[library_benchmark]
fn exact_alignment_16() -> VertexAlignment {
    let (a, b) = make_pair(16);
    black_box(get_exact_vertex_alignment(&a, &b))
}

// C++: BM_GetExactVertexAlignment (64 vertices)
#[library_benchmark]
fn exact_alignment_64() -> VertexAlignment {
    let (a, b) = make_pair(64);
    black_box(get_exact_vertex_alignment(&a, &b))
}

// C++: BM_GetApproxVertexAlignment (128 vertices, radius 4)
#[library_benchmark]
fn approx_alignment_128_r4() -> VertexAlignment {
    let (a, b) = make_pair(128);
    black_box(get_approx_vertex_alignment(&a, &b, 4))
}

// C++: BM_GetApproxVertexAlignment (128 vertices, radius 8)
#[library_benchmark]
fn approx_alignment_128_r8() -> VertexAlignment {
    let (a, b) = make_pair(128);
    black_box(get_approx_vertex_alignment(&a, &b, 8))
}

// C++: BM_GetApproxVertexAlignment (1024 vertices, radius 4)
#[library_benchmark]
fn approx_alignment_1024_r4() -> VertexAlignment {
    let (a, b) = make_pair(1024);
    black_box(get_approx_vertex_alignment(&a, &b, 4))
}

#[inline(never)]
fn make_polylines(count: usize, n: usize) -> Vec<Polyline> {
    (0..count)
        .map(|i| make_polyline(n, f64::from(i as i32) * 0.002))
        .collect()
}

// C++: BM_ComputeMedoid (10 polylines × 128 vertices)
#[library_benchmark]
fn compute_medoid_10x128() -> usize {
    let polylines = make_polylines(10, 128);
    let opts = MedoidOptions { approx: true };
    black_box(get_medoid_polyline(&polylines, &opts))
}

// C++: BM_ComputeConsensusUnseeded (5 polylines × 64 vertices)
#[library_benchmark]
fn compute_consensus_unseeded_5x64() -> Polyline {
    let polylines = make_polylines(5, 64);
    let opts = ConsensusOptions {
        approx: true,
        seed_medoid: false,
        iteration_cap: 5,
    };
    black_box(get_consensus_polyline(&polylines, &opts))
}

// C++: BM_ComputeConsensusSeeded (5 polylines × 64 vertices)
#[library_benchmark]
fn compute_consensus_seeded_5x64() -> Polyline {
    let polylines = make_polylines(5, 64);
    let opts = ConsensusOptions {
        approx: true,
        seed_medoid: true,
        iteration_cap: 5,
    };
    black_box(get_consensus_polyline(&polylines, &opts))
}

library_benchmark_group!(
    name = polyline_alignment_benchmarks;
    benchmarks =
        exact_alignment_cost_16,
        exact_alignment_cost_64,
        exact_alignment_16,
        exact_alignment_64,
        approx_alignment_128_r4,
        approx_alignment_128_r8,
        approx_alignment_1024_r4,
        compute_medoid_10x128,
        compute_consensus_unseeded_5x64,
        compute_consensus_seeded_5x64
);

main!(library_benchmark_groups = polyline_alignment_benchmarks);
