// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Encoding and decoding S2 objects to/from compact binary format.
//!
//! S2 types implement `S2Encode` / `S2Decode` for efficient serialisation.
//! `ShapeIndex` has its own `encode_to_writer` / `decode_from_reader`.
//!
//! Run with: `cargo run --example encoding_decoding`
#![allow(clippy::print_stdout, reason = "example binary")]

use s2rst::s1::Angle;
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::shape_index::ShapeIndex;
use s2rst::s2::text_format;
use s2rst::s2::{Cap, CellId, LatLng, Loop, Point, Polygon, Rect};

fn main() {
    // ── Point ──────────────────────────────────────────────────────────
    let p = LatLng::from_degrees(48.8566, 2.3522).to_point(); // Paris
    roundtrip("Point", &p);
    let decoded = decode::<Point>(&encode(&p));
    println!(
        "  Decoded: ({:.4}°, {:.4}°)\n",
        LatLng::from_point(decoded).lat.degrees(),
        LatLng::from_point(decoded).lng.degrees()
    );

    // ── CellId ─────────────────────────────────────────────────────────
    let cell_id = CellId::from_point(&p).parent_at_level(15);
    roundtrip("CellId", &cell_id);
    println!("  Token: {}\n", cell_id.to_token());

    // ── Cap ────────────────────────────────────────────────────────────
    let cap = Cap::from_center_angle(p, Angle::from_degrees(1.0));
    roundtrip("Cap", &cap);
    println!();

    // ── Rect ───────────────────────────────────────────────────────────
    let rect = Rect::from_center_size(
        LatLng::from_degrees(48.85, 2.35),
        LatLng::from_degrees(0.5, 1.0),
    );
    roundtrip("Rect", &rect);
    println!();

    // ── CellUnion ──────────────────────────────────────────────────────
    let coverer = s2rst::s2::region_coverer::RegionCoverer::new().max_cells(8);
    let covering = coverer.covering(&cap);
    roundtrip("CellUnion", &covering);
    println!("  {} cells\n", covering.num_cells());

    // ── Polyline ───────────────────────────────────────────────────────
    let polyline = text_format::make_polyline("48.86:2.35, 48.87:2.36, 48.88:2.34");
    roundtrip("Polyline", &polyline);
    println!("  {} vertices\n", polyline.num_vertices());

    // ── Loop ───────────────────────────────────────────────────────────
    let loop_ = Loop::make_regular(p, Angle::from_degrees(0.01), 8);
    roundtrip("Loop", &loop_);
    println!("  {} vertices\n", loop_.num_vertices());

    // ── Polygon ────────────────────────────────────────────────────────
    let polygon = Polygon::from_loops(vec![Loop::make_regular(p, Angle::from_degrees(1.0), 64)]);
    roundtrip("Polygon (64v)", &polygon);
    println!(
        "  {} loops, {} vertices\n",
        polygon.num_loops(),
        polygon
            .loops()
            .iter()
            .map(Loop::num_vertices)
            .sum::<usize>()
    );

    // ── ShapeIndex ─────────────────────────────────────────────────────
    println!("=== ShapeIndex encode/decode ===");
    let lax = LaxPolygon::from_polygon_ref(&polygon);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lax));
    index.build();

    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).expect("encode failed");
    println!("  Encoded: {} bytes", buf.len());

    let decoded_index = ShapeIndex::decode_from_reader(&mut buf.as_slice()).expect("decode failed");
    println!(
        "  Decoded: {} shape(s), {} edge(s)",
        decoded_index.num_shape_ids(),
        decoded_index.num_edges()
    );
    println!(
        "  Shapes match: {}",
        index.num_shape_ids() == decoded_index.num_shape_ids()
    );
    println!(
        "  Edges match:  {}",
        index.num_edges() == decoded_index.num_edges()
    );
}

fn encode<T: S2Encode>(val: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    val.encode(&mut buf).expect("encode failed");
    buf
}

fn decode<T: S2Decode>(buf: &[u8]) -> T {
    T::decode(&mut &buf[..]).expect("decode failed")
}

fn roundtrip<T: S2Encode + S2Decode>(label: &str, val: &T) -> usize {
    let buf = encode(val);
    let _decoded = decode::<T>(&buf);
    println!("  {label}: {} bytes", buf.len());
    buf.len()
}
