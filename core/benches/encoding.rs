// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

#![allow(missing_docs, clippy::exit, reason = "benchmarks")]
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use std::hint::black_box;

use s2rst::s1::Angle;
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::{LatLng, Loop, Polygon, Rect};

// ─── Rect encoding (Go: BenchmarkRectDecode) ───────────────────────────

#[inline(never)]
fn make_rect() -> Rect {
    Rect::from_center_size(
        LatLng::from_degrees(80.0, 170.0),
        LatLng::from_degrees(40.0, 60.0),
    )
}

#[library_benchmark]
fn rect_encode() -> Vec<u8> {
    let rect = make_rect();
    let mut buf = Vec::with_capacity(128);
    black_box(&rect).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn rect_decode() -> Rect {
    let rect = make_rect();
    let mut buf = Vec::new();
    rect.encode(&mut buf).unwrap();
    black_box(Rect::decode(&mut buf.as_slice()).unwrap())
}

// ─── Polygon encoding (C++: BM_S2Encoding, BM_S2Decoding) ─────────────

#[inline(never)]
fn make_polygon_n(n: usize) -> Polygon {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    Polygon::from_loops(vec![loop_])
}

fn encode_polygon(p: &Polygon) -> Vec<u8> {
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    buf
}

#[library_benchmark]
fn polygon_encode_64v() -> Vec<u8> {
    let p = make_polygon_n(64);
    let mut buf = Vec::with_capacity(2048);
    black_box(&p).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn polygon_decode_64v() -> Polygon {
    let buf = encode_polygon(&make_polygon_n(64));
    black_box(Polygon::decode(&mut buf.as_slice()).unwrap())
}

#[library_benchmark]
fn polygon_encode_256v() -> Vec<u8> {
    let p = make_polygon_n(256);
    let mut buf = Vec::with_capacity(8192);
    black_box(&p).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn polygon_decode_256v() -> Polygon {
    let buf = encode_polygon(&make_polygon_n(256));
    black_box(Polygon::decode(&mut buf.as_slice()).unwrap())
}

#[library_benchmark]
fn polygon_encode_1024v() -> Vec<u8> {
    let p = make_polygon_n(1024);
    let mut buf = Vec::with_capacity(32768);
    black_box(&p).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn polygon_decode_1024v() -> Polygon {
    let buf = encode_polygon(&make_polygon_n(1024));
    black_box(Polygon::decode(&mut buf.as_slice()).unwrap())
}

// ─── ShapeIndex encoding ───────────────────────────────────────────────

#[inline(never)]
fn make_index_n(n: usize) -> ShapeIndex {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    let polygon = Polygon::from_loops(vec![loop_]);
    let lax = LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();
    index
}

fn encode_index(idx: &ShapeIndex) -> Vec<u8> {
    let mut buf = Vec::new();
    idx.encode_to_writer(&mut buf).unwrap();
    buf
}

#[library_benchmark]
fn shape_index_encode_64() -> Vec<u8> {
    let idx = make_index_n(64);
    let mut buf = Vec::with_capacity(4096);
    black_box(&idx).encode_to_writer(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn shape_index_decode_64() -> ShapeIndex {
    let buf = encode_index(&make_index_n(64));
    black_box(ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap())
}

#[library_benchmark]
fn shape_index_encode_1024() -> Vec<u8> {
    let idx = make_index_n(1024);
    let mut buf = Vec::with_capacity(65536);
    black_box(&idx).encode_to_writer(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn shape_index_decode_1024() -> ShapeIndex {
    let buf = encode_index(&make_index_n(1024));
    black_box(ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap())
}

// ─── LaxPolygon encoding (C++: BM_DecodeS2LaxPolygonShape) ────────────

#[inline(never)]
fn make_lax_polygon_n(n: usize) -> LaxPolygon {
    let center = LatLng::from_degrees(0.0, 0.0).to_point();
    let loop_ = Loop::make_regular(center, Angle::from_degrees(1.0), n);
    let polygon = Polygon::from_loops(vec![loop_]);
    LaxPolygon::from_polygon_ref(&polygon)
}

fn encode_lax_polygon(lp: &LaxPolygon) -> Vec<u8> {
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    buf
}

#[library_benchmark]
fn lax_polygon_encode_64v() -> Vec<u8> {
    let lp = make_lax_polygon_n(64);
    let mut buf = Vec::with_capacity(2048);
    black_box(&lp).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn lax_polygon_decode_64v() -> LaxPolygon {
    let buf = encode_lax_polygon(&make_lax_polygon_n(64));
    black_box(LaxPolygon::decode(&mut buf.as_slice()).unwrap())
}

#[library_benchmark]
fn lax_polygon_encode_256v() -> Vec<u8> {
    let lp = make_lax_polygon_n(256);
    let mut buf = Vec::with_capacity(8192);
    black_box(&lp).encode(&mut buf).unwrap();
    black_box(buf)
}

#[library_benchmark]
fn lax_polygon_decode_256v() -> LaxPolygon {
    let buf = encode_lax_polygon(&make_lax_polygon_n(256));
    black_box(LaxPolygon::decode(&mut buf.as_slice()).unwrap())
}

library_benchmark_group!(
    name = encoding_benchmarks;
    benchmarks =
        rect_encode,
        rect_decode,
        polygon_encode_64v,
        polygon_decode_64v,
        polygon_encode_256v,
        polygon_decode_256v,
        polygon_encode_1024v,
        polygon_decode_1024v,
        shape_index_encode_64,
        shape_index_decode_64,
        shape_index_encode_1024,
        shape_index_decode_1024,
        lax_polygon_encode_64v,
        lax_polygon_decode_64v,
        lax_polygon_encode_256v,
        lax_polygon_decode_256v
);

main!(library_benchmark_groups = encoding_benchmarks);
