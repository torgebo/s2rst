// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Benchmarks ported from C++: `s2point_compression_test.cc`
//! `BM_S2EncodePointsCompressed`, `BM_S2DecodePointsCompressed`
#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::coords::Level;
use s2rst::s2::point_compression::{
    decode_points_compressed, encode_points_compressed, points_to_xyz_face_si_ti,
};
use s2rst::s2::{Loop, Point};

#[inline(never)]
fn make_points(n: usize, level: Level) -> (Vec<Point>, Level) {
    let center = Point::from_coords(1.0, 0.0, 0.0);
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    let points: Vec<Point> = (0..loop_.num_vertices()).map(|i| loop_.vertex(i)).collect();
    (points, level)
}

// C++: BM_S2EncodePointsCompressed (64 points, level 14)
#[library_benchmark]
fn encode_compressed_64() -> Vec<u8> {
    let (points, level) = make_points(64, Level::new(14));
    let xyzfst = points_to_xyz_face_si_ti(&points);
    let mut buf = Vec::with_capacity(2048);
    encode_points_compressed(&mut buf, &xyzfst, level).unwrap();
    black_box(buf)
}

// C++: BM_S2DecodePointsCompressed (64 points, level 14)
#[library_benchmark]
fn decode_compressed_64() -> Vec<Point> {
    let (points, level) = make_points(64, Level::new(14));
    let xyzfst = points_to_xyz_face_si_ti(&points);
    let mut buf = Vec::new();
    encode_points_compressed(&mut buf, &xyzfst, level).unwrap();
    black_box(decode_points_compressed(&mut buf.as_slice(), level, points.len()).unwrap())
}

// C++: BM_S2EncodePointsCompressed (1024 points, level 14)
#[library_benchmark]
fn encode_compressed_1024() -> Vec<u8> {
    let (points, level) = make_points(1024, Level::new(14));
    let xyzfst = points_to_xyz_face_si_ti(&points);
    let mut buf = Vec::with_capacity(32768);
    encode_points_compressed(&mut buf, &xyzfst, level).unwrap();
    black_box(buf)
}

// C++: BM_S2DecodePointsCompressed (1024 points, level 14)
#[library_benchmark]
fn decode_compressed_1024() -> Vec<Point> {
    let (points, level) = make_points(1024, Level::new(14));
    let xyzfst = points_to_xyz_face_si_ti(&points);
    let mut buf = Vec::new();
    encode_points_compressed(&mut buf, &xyzfst, level).unwrap();
    black_box(decode_points_compressed(&mut buf.as_slice(), level, points.len()).unwrap())
}

// points_to_xyz_face_si_ti conversion (64 points)
#[library_benchmark]
fn points_to_xyzfst_64() {
    let (points, _) = make_points(64, Level::new(14));
    drop(black_box(points_to_xyz_face_si_ti(&points)));
}

// points_to_xyz_face_si_ti conversion (1024 points)
#[library_benchmark]
fn points_to_xyzfst_1024() {
    let (points, _) = make_points(1024, Level::new(14));
    drop(black_box(points_to_xyz_face_si_ti(&points)));
}

library_benchmark_group!(
    name = point_compression_benchmarks;
    benchmarks =
        encode_compressed_64,
        decode_compressed_64,
        encode_compressed_1024,
        decode_compressed_1024,
        points_to_xyzfst_64,
        points_to_xyzfst_1024
);

main!(library_benchmark_groups = point_compression_benchmarks);
