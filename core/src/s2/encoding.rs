// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Binary encoding and decoding for S2 types.
//!
//! Wire-compatible with the C++ S2 library. Uses little-endian byte order.
//!
//! Corresponds to C++ `Encode`/`Decode` methods on each type.

#![expect(
    clippy::cast_sign_loss,
    reason = "depth/level (i32) encoded as u32/u64 — always non-negative for valid geometries"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "depth/level/count <-> u8/u32/u64 for serialization — bounded by format"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for depth/level counts — bounded by geometry limits"
)]
use std::io::{self, Read, Write};

use crate::r1;
use crate::s1::{self, ChordAngle};
use crate::s2::region::Region;
use crate::s2::{Cap, Cell, CellId, CellUnion, Loop, Point, Polygon, Rect};

/// Lossless encoding version used for most types (C++ `kCurrentLosslessEncodingVersionNumber`).
const ENCODING_VERSION: u8 = 1;

/// Maximum vertices in a single shape for decode safety.
const MAX_ENCODED_VERTICES: u32 = 50_000_000;

/// Maximum loops in a polygon for decode safety.
const MAX_ENCODED_LOOPS: u32 = 10_000_000;

/// Maximum cells in a `CellUnion` for decode safety.
const MAX_ENCODED_CELLS: u64 = 1_000_000;

// ─── Traits ─────────────────────────────────────────────────────────────

/// Binary encoding for S2 types.
pub trait S2Encode {
    /// Encodes this value to the writer in the S2 binary format.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails.
    fn encode(&self, w: &mut dyn Write) -> io::Result<()>;
}

/// Binary decoding for S2 types.
pub trait S2Decode: Sized {
    /// Decodes a value from the reader in the S2 binary format.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or the read fails.
    fn decode(r: &mut dyn Read) -> io::Result<Self>;
}

// ─── Helpers ────────────────────────────────────────────────────────────

fn write_u32(w: &mut dyn Write, v: u32) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_u64(w: &mut dyn Write, v: u64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn write_f64(w: &mut dyn Write, v: f64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn read_u8(r: &mut dyn Read) -> io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(r: &mut dyn Read) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_u64(r: &mut dyn Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn read_f64(r: &mut dyn Read) -> io::Result<f64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

pub(crate) fn write_uvarint(w: &mut dyn Write, mut x: u64) -> io::Result<()> {
    while x >= 0x80 {
        w.write_all(&[(x as u8) | 0x80])?;
        x >>= 7;
    }
    w.write_all(&[x as u8])
}

pub(crate) fn read_uvarint(r: &mut dyn Read) -> io::Result<u64> {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    loop {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let b = buf[0];
        result |= u64::from(b & 0x7F) << shift;
        if b < 0x80 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "varint overflow",
            ));
        }
    }
}

// ─── CellId ─────────────────────────────────────────────────────────────

impl S2Encode for CellId {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        write_u64(w, self.0)
    }
}

impl S2Decode for CellId {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        Ok(CellId(read_u64(r)?))
    }
}

// ─── Point ──────────────────────────────────────────────────────────────
// C++ S2Point is encoded as 3 raw doubles (24 bytes), no version byte.

impl S2Encode for Point {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        write_f64(w, self.0.x)?;
        write_f64(w, self.0.y)?;
        write_f64(w, self.0.z)
    }
}

impl S2Decode for Point {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let x = read_f64(r)?;
        let y = read_f64(r)?;
        let z = read_f64(r)?;
        Ok(Point(crate::r3::Vector { x, y, z }))
    }
}

// ─── Cap ────────────────────────────────────────────────────────────────
// Note: Cap has NO version byte in Go/C++.

impl S2Encode for Cap {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        let center = self.center();
        write_f64(w, center.0.x)?;
        write_f64(w, center.0.y)?;
        write_f64(w, center.0.z)?;
        write_f64(w, self.chord_radius().length2())
    }
}

impl S2Decode for Cap {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let x = read_f64(r)?;
        let y = read_f64(r)?;
        let z = read_f64(r)?;
        let radius = read_f64(r)?;
        let center = Point(crate::r3::Vector { x, y, z });
        Ok(Cap::from_center_chord_angle(
            center,
            ChordAngle::from_length2(radius),
        ))
    }
}

// ─── Rect (S2LatLngRect) ───────────────────────────────────────────────

impl S2Encode for Rect {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&[ENCODING_VERSION])?;
        write_f64(w, self.lat.lo)?;
        write_f64(w, self.lat.hi)?;
        write_f64(w, self.lng.lo)?;
        write_f64(w, self.lng.hi)
    }
}

impl S2Decode for Rect {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        if version > ENCODING_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported encoding version {version}"),
            ));
        }
        let lat_lo = read_f64(r)?;
        let lat_hi = read_f64(r)?;
        let lng_lo = read_f64(r)?;
        let lng_hi = read_f64(r)?;
        Ok(Rect {
            lat: r1::Interval {
                lo: lat_lo,
                hi: lat_hi,
            },
            lng: s1::Interval {
                lo: lng_lo,
                hi: lng_hi,
            },
        })
    }
}

// ─── CellUnion ──────────────────────────────────────────────────────────

impl S2Encode for CellUnion {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&[ENCODING_VERSION])?;
        write_u64(w, self.len() as u64)?;
        for &id in self {
            id.encode(w)?;
        }
        Ok(())
    }
}

impl S2Decode for CellUnion {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        if version > ENCODING_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported encoding version {version}"),
            ));
        }
        let n = read_u64(r)?;
        if n > MAX_ENCODED_CELLS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("too many cells ({n}; max is {MAX_ENCODED_CELLS})"),
            ));
        }
        let mut ids = Vec::with_capacity(n as usize);
        for _ in 0..n {
            ids.push(CellId::decode(r)?);
        }
        Ok(CellUnion::from_cell_ids(ids))
    }
}

// ─── Polyline ───────────────────────────────────────────────────────────

/// Polyline lossless encoding version (C++ `kCurrentLosslessEncodingVersionNumber`).
const POLYLINE_LOSSLESS_VERSION: u8 = 1;
/// Polyline compressed encoding version (C++ `kCurrentCompressedEncodingVersionNumber`).
const POLYLINE_COMPRESSED_VERSION: u8 = 2;

impl S2Encode for crate::s2::polyline::Polyline {
    /// Encodes in lossless format (matching C++ default `CodingHint::FAST`).
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&[POLYLINE_LOSSLESS_VERSION])?;
        write_u32(w, self.len() as u32)?;
        for v in self.iter() {
            write_f64(w, v.0.x)?;
            write_f64(w, v.0.y)?;
            write_f64(w, v.0.z)?;
        }
        Ok(())
    }
}

impl S2Decode for crate::s2::polyline::Polyline {
    /// Decodes both lossless (version 1) and compressed (version 2) formats.
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        match version {
            POLYLINE_LOSSLESS_VERSION => decode_polyline_lossless(r),
            POLYLINE_COMPRESSED_VERSION => decode_polyline_compressed(r),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported polyline encoding version {version}"),
            )),
        }
    }
}

fn decode_polyline_lossless(r: &mut dyn Read) -> io::Result<crate::s2::polyline::Polyline> {
    let n = read_u32(r)?;
    if n > MAX_ENCODED_VERTICES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("too many vertices ({n}; max is {MAX_ENCODED_VERTICES})"),
        ));
    }
    let mut vertices = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let x = read_f64(r)?;
        let y = read_f64(r)?;
        let z = read_f64(r)?;
        vertices.push(Point(crate::r3::Vector { x, y, z }));
    }
    Ok(crate::s2::polyline::Polyline::new(vertices))
}

fn decode_polyline_compressed(r: &mut dyn Read) -> io::Result<crate::s2::polyline::Polyline> {
    let snap_level_raw = read_u8(r)?;
    let snap_level = Level::try_new(snap_level_raw).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid snap level {snap_level_raw}"),
        )
    })?;
    let num_vertices = read_uvarint(r)? as u32;
    if num_vertices == 0 {
        return Ok(crate::s2::polyline::Polyline::new(vec![]));
    }
    if num_vertices > MAX_ENCODED_VERTICES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("too many vertices ({num_vertices}; max is {MAX_ENCODED_VERTICES})"),
        ));
    }
    let points = decode_points_compressed(r, snap_level, num_vertices as usize)?;
    Ok(crate::s2::polyline::Polyline::new(points))
}

/// Encodes a polyline in compressed format (version 2).
///
/// This matches C++ `S2Polyline::Encode` with `CodingHint::COMPACT`.
#[cfg(test)]
pub(crate) fn encode_polyline_compressed(
    polyline: &crate::s2::polyline::Polyline,
    w: &mut dyn Write,
) -> io::Result<()> {
    if polyline.is_empty() {
        return encode_polyline_compressed_inner(polyline, w, &[], Level::MAX);
    }

    let vertices: Vec<Point> = polyline.iter().copied().collect();
    let all_vertices = points_to_xyz_face_si_ti(&vertices);

    // Build histogram to find best snap level.
    // Index 0 = not a cell center, index 1..=31 = level 0..=30.
    let mut histogram = [0i32; MAX_CELL_LEVEL as usize + 2];
    for v in &all_vertices {
        let idx = v.cell_level.map_or(0, |l| l.as_usize() + 1);
        histogram[idx] += 1;
    }

    let (snap_level_idx, &num_snapped) = histogram[1..]
        .iter()
        .enumerate()
        .max_by_key(|&(_, &count)| count)
        .unwrap_or((0, &0));
    let snap_level = Level::new(snap_level_idx as u8);

    // Estimate sizes to decide compressed vs lossless.
    let exact_point_size = 24 + 2;
    let num_unsnapped = polyline.len() as i32 - num_snapped;
    let compressed_size = 4 * polyline.len() as i32 + exact_point_size * num_unsnapped;
    let lossless_size = 24 * polyline.len() as i32;

    if compressed_size < lossless_size {
        encode_polyline_compressed_inner(polyline, w, &all_vertices, snap_level)
    } else {
        polyline.encode(w)
    }
}

#[cfg(test)]
fn encode_polyline_compressed_inner(
    polyline: &crate::s2::polyline::Polyline,
    w: &mut dyn Write,
    all_vertices: &[S2XYZFaceSiTi],
    snap_level: Level,
) -> io::Result<()> {
    w.write_all(&[POLYLINE_COMPRESSED_VERSION])?;
    w.write_all(&[snap_level.as_u8()])?;
    write_uvarint(w, polyline.len() as u64)?;
    if !polyline.is_empty() {
        encode_points_compressed(w, all_vertices, snap_level)?;
    }
    Ok(())
}

// ─── Loop ───────────────────────────────────────────────────────────────

impl S2Encode for Loop {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        w.write_all(&[ENCODING_VERSION])?;
        write_u32(w, self.num_vertices() as u32)?;
        for v in self.vertices() {
            write_f64(w, v.0.x)?;
            write_f64(w, v.0.y)?;
            write_f64(w, v.0.z)?;
        }
        // C++ uses put8(origin_inside_), put32(depth_).
        w.write_all(&[u8::from(self.contains_origin())])?;
        write_u32(w, self.depth() as u32)?;
        // Encode the bound.
        self.bound().encode(w)
    }
}

impl S2Decode for Loop {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        if version != ENCODING_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported loop encoding version {version}"),
            ));
        }
        let n = read_u32(r)?;
        if n > MAX_ENCODED_VERTICES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("too many vertices ({n}; max is {MAX_ENCODED_VERTICES})"),
            ));
        }
        let mut vertices = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let x = read_f64(r)?;
            let y = read_f64(r)?;
            let z = read_f64(r)?;
            vertices.push(Point(crate::r3::Vector { x, y, z }));
        }
        let origin_inside = read_u8(r)? != 0;
        let depth = read_u32(r)? as i32;
        let bound = Rect::decode(r)?;
        Ok(Loop::from_decoded(vertices, origin_inside, depth, bound))
    }
}

// ─── Compressed Loop ────────────────────────────────────────────────────

use crate::s2::coords::{Level, MAX_CELL_LEVEL};
use crate::s2::point_compression::{
    S2XYZFaceSiTi, decode_points_compressed, encode_points_compressed, points_to_xyz_face_si_ti,
};

/// Compressed encoding version (matches C++ kCurrentCompressedEncodingVersionNumber).
const COMPRESSED_ENCODING_VERSION: u8 = 4;

/// Bit flags for compressed loop properties.
const ORIGIN_INSIDE_FLAG: u32 = 1;
const BOUND_ENCODED_FLAG: u32 = 2;

/// Loops with at least this many vertices include the bound in compressed encoding.
const MIN_VERTICES_FOR_BOUND: usize = 64;

/// Encodes a loop in compressed format (no version byte — nested inside polygon).
#[cfg(test)]
pub(crate) fn encode_loop_compressed(
    l: &Loop,
    w: &mut dyn Write,
    snap_level: Level,
) -> io::Result<()> {
    let vertices = points_to_xyz_face_si_ti(l.vertices());
    write_uvarint(w, l.num_vertices() as u64)?;
    encode_points_compressed(w, &vertices, snap_level)?;

    let mut properties: u32 = 0;
    if l.contains_origin() {
        properties |= ORIGIN_INSIDE_FLAG;
    }
    let encode_bound = l.num_vertices() >= MIN_VERTICES_FOR_BOUND;
    if encode_bound {
        properties |= BOUND_ENCODED_FLAG;
    }
    write_uvarint(w, u64::from(properties))?;
    write_uvarint(w, l.depth() as u64)?;
    if encode_bound {
        l.bound().encode(w)?;
    }
    Ok(())
}

/// Decodes a loop from compressed format.
pub(crate) fn decode_loop_compressed(r: &mut dyn Read, snap_level: Level) -> io::Result<Loop> {
    let num_vertices = read_uvarint(r)? as u32;
    if num_vertices == 0 || num_vertices > MAX_ENCODED_VERTICES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid compressed vertex count {num_vertices}"),
        ));
    }
    let points = decode_points_compressed(r, snap_level, num_vertices as usize)?;

    let properties = read_uvarint(r)? as u32;
    let origin_inside = (properties & ORIGIN_INSIDE_FLAG) != 0;
    let bound_encoded = (properties & BOUND_ENCODED_FLAG) != 0;

    let depth = read_uvarint(r)? as i32;

    let bound = if bound_encoded {
        Some(Rect::decode(r)?)
    } else {
        None
    };

    Ok(Loop::from_decoded_compressed(
        points,
        origin_inside,
        depth,
        bound,
    ))
}

// ─── Polygon ────────────────────────────────────────────────────────────

fn encode_polygon_lossless(p: &Polygon, w: &mut dyn Write) -> io::Result<()> {
    w.write_all(&[ENCODING_VERSION])?;
    // C++ uses put8(true) for legacy owns_loops, put8(has_holes).
    w.write_all(&[1u8])?; // legacy owns_loops, always true
    w.write_all(&[u8::from(p.has_holes())])?;
    write_u32(w, p.num_loops() as u32)?;
    for l in p.loops() {
        l.encode(w)?;
    }
    p.rect_bound().encode(w)
}

fn decode_polygon_lossless(r: &mut dyn Read) -> io::Result<Polygon> {
    let _owns_loops = read_u8(r)?;
    let has_holes = read_u8(r)? != 0;
    let nloops = read_u32(r)?;
    if nloops > MAX_ENCODED_LOOPS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("too many loops ({nloops}; max is {MAX_ENCODED_LOOPS})"),
        ));
    }
    let mut loops = Vec::with_capacity(nloops as usize);
    for _ in 0..nloops {
        loops.push(Loop::decode(r)?);
    }
    let bound = Rect::decode(r)?;
    Ok(Polygon::from_decoded_loops(loops, has_holes, bound))
}

fn encode_polygon_compressed(
    p: &Polygon,
    w: &mut dyn Write,
    all_vertices: &[S2XYZFaceSiTi],
    snap_level: Level,
) -> io::Result<()> {
    w.write_all(&[COMPRESSED_ENCODING_VERSION])?;
    w.write_all(&[snap_level.as_u8()])?;
    write_uvarint(w, p.num_loops() as u64)?;

    let mut offset = 0;
    for l in p.loops() {
        let n = l.num_vertices();
        encode_loop_compressed_with_vertices(l, w, snap_level, &all_vertices[offset..offset + n])?;
        offset += n;
    }
    Ok(())
}

/// Like `encode_loop_compressed` but uses pre-computed `S2XYZFaceSiTi` vertices.
fn encode_loop_compressed_with_vertices(
    l: &Loop,
    w: &mut dyn Write,
    snap_level: Level,
    vertices: &[S2XYZFaceSiTi],
) -> io::Result<()> {
    write_uvarint(w, l.num_vertices() as u64)?;
    encode_points_compressed(w, vertices, snap_level)?;

    let mut properties: u32 = 0;
    if l.contains_origin() {
        properties |= ORIGIN_INSIDE_FLAG;
    }
    if l.num_vertices() >= MIN_VERTICES_FOR_BOUND {
        properties |= BOUND_ENCODED_FLAG;
    }
    write_uvarint(w, u64::from(properties))?;
    write_uvarint(w, l.depth() as u64)?;
    if l.num_vertices() >= MIN_VERTICES_FOR_BOUND {
        l.bound().encode(w)?;
    }
    Ok(())
}

fn decode_polygon_compressed(r: &mut dyn Read) -> io::Result<Polygon> {
    let snap_level_raw = read_u8(r)?;
    let snap_level = Level::try_new(snap_level_raw).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid snap level {snap_level_raw}"),
        )
    })?;
    let nloops = read_uvarint(r)? as u32;
    if nloops > MAX_ENCODED_LOOPS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("too many loops ({nloops}; max is {MAX_ENCODED_LOOPS})"),
        ));
    }
    let mut loops = Vec::with_capacity(nloops as usize);
    for _ in 0..nloops {
        let l = decode_loop_compressed(r, snap_level)?;
        if !l.is_empty_loop() && l.num_vertices() > 0 {
            loops.push(l);
        }
    }
    Ok(Polygon::from_loops(loops))
}

impl S2Encode for Polygon {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        if self.num_vertices() == 0 {
            return encode_polygon_compressed(self, w, &[], Level::MAX);
        }

        // Convert all vertices to S2XYZFaceSiTi.
        let mut all_vertices = Vec::with_capacity(self.num_vertices());
        for l in self.loops() {
            all_vertices.extend(points_to_xyz_face_si_ti(l.vertices()));
        }

        // Build histogram of cell levels.
        // Index 0 = not a cell center, index 1..=31 = level 0..=30.
        let mut histogram = [0i32; MAX_CELL_LEVEL as usize + 2];
        for v in &all_vertices {
            let idx = v.cell_level.map_or(0, |l| l.as_usize() + 1);
            histogram[idx] += 1;
        }

        // Find snap_level with the most vertices (skip histogram[0] = unsnapped).
        let (snap_level_idx, &num_snapped) = histogram[1..]
            .iter()
            .enumerate()
            .max_by_key(|&(_, &count)| count)
            .unwrap_or((0, &0));
        let snap_level = Level::new(snap_level_idx as u8);

        // Estimate sizes.
        let exact_point_size = 24 + 2; // sizeof(S2Point) + varint overhead
        let num_unsnapped = self.num_vertices() as i32 - num_snapped;
        let compressed_size = 4 * self.num_vertices() as i32 + exact_point_size * num_unsnapped;
        let lossless_size = 24 * self.num_vertices() as i32;

        if compressed_size < lossless_size {
            encode_polygon_compressed(self, w, &all_vertices, snap_level)
        } else {
            encode_polygon_lossless(self, w)
        }
    }
}

impl S2Decode for Polygon {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        match version {
            ENCODING_VERSION => decode_polygon_lossless(r),
            COMPRESSED_ENCODING_VERSION => decode_polygon_compressed(r),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported polygon encoding version {version}"),
            )),
        }
    }
}

// ─── LaxPolyline ────────────────────────────────────────────────────────

use crate::s2::encoded_s2point_vector::{self, CodingHint};
use crate::s2::lax_polyline::LaxPolyline;

impl S2Encode for LaxPolyline {
    /// Encodes using `CodingHint::Fast` (UNCOMPRESSED format) matching
    /// the C++ default (`CodingHint::FAST`).
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        encoded_s2point_vector::encode_s2point_vector(self.vertices(), CodingHint::Fast, w)
    }
}

impl LaxPolyline {
    /// Encodes with the given coding hint.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails.
    pub fn encode_with_hint(&self, w: &mut dyn Write, hint: CodingHint) -> io::Result<()> {
        encoded_s2point_vector::encode_s2point_vector(self.vertices(), hint, w)
    }
}

impl S2Decode for LaxPolyline {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let points = encoded_s2point_vector::decode_s2point_vector(r)?;
        Ok(LaxPolyline::new(points))
    }
}

// ─── LaxPolygon ─────────────────────────────────────────────────────────

use crate::s2::lax_polygon::LaxPolygon;

/// Encoding version for `LaxPolygon` (matches C++ `kCurrentEncodingVersionNumber`).
const LAX_POLYGON_VERSION: u8 = 1;

impl S2Encode for LaxPolygon {
    /// Encodes using `CodingHint::Fast` matching C++ default.
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        self.encode_with_hint(w, CodingHint::Fast)
    }
}

impl LaxPolygon {
    /// Encodes with the given coding hint.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails.
    pub fn encode_with_hint(&self, w: &mut dyn Write, hint: CodingHint) -> io::Result<()> {
        w.write_all(&[LAX_POLYGON_VERSION])?;
        write_uvarint(w, self.num_loops() as u64)?;

        let all_vertices = self.all_vertices();
        encoded_s2point_vector::encode_s2point_vector(all_vertices, hint, w)?;

        if self.num_loops() > 1 {
            // Encode cumulative vertex counts as a uint32 vector.
            let mut loop_starts: Vec<u32> = Vec::with_capacity(self.num_loops() + 1);
            let mut offset = 0u32;
            for i in 0..self.num_loops() {
                loop_starts.push(offset);
                offset += self.num_loop_vertices(i) as u32;
            }
            loop_starts.push(offset);
            crate::s2::encoded_uint_vector::encode_uint_vector_u32(&loop_starts, w)?;
        }
        Ok(())
    }
}

impl S2Decode for LaxPolygon {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let version = read_u8(r)?;
        if version != LAX_POLYGON_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported LaxPolygon version {version}"),
            ));
        }

        let num_loops = read_uvarint(r)? as usize;
        if num_loops > MAX_ENCODED_LOOPS as usize {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too many loops in LaxPolygon",
            ));
        }

        let vertices = encoded_s2point_vector::decode_s2point_vector(r)?;

        if num_loops == 0 {
            Ok(LaxPolygon::default())
        } else if num_loops == 1 {
            Ok(LaxPolygon::from_loops(&[&vertices]))
        } else {
            let loop_starts = crate::s2::encoded_uint_vector::decode_uint_vector_u32(r)?;
            if loop_starts.len() != num_loops + 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "loop_starts length {} != num_loops + 1 = {}",
                        loop_starts.len(),
                        num_loops + 1
                    ),
                ));
            }
            // Split vertices into loops according to loop_starts.
            let mut loops: Vec<Vec<Point>> = Vec::with_capacity(num_loops);
            for i in 0..num_loops {
                let start = loop_starts[i] as usize;
                let end = loop_starts[i + 1] as usize;
                if end > vertices.len() || start > end {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid loop_starts offsets",
                    ));
                }
                loops.push(vertices[start..end].to_vec());
            }
            let loop_refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
            Ok(LaxPolygon::from_loops(&loop_refs))
        }
    }
}

// ─── PointVector ────────────────────────────────────────────────────────

use crate::s2::point_vector::PointVector;

impl S2Encode for PointVector {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        encoded_s2point_vector::encode_s2point_vector(self.points(), CodingHint::Fast, w)
    }
}

impl PointVector {
    /// Encodes with the given coding hint.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails.
    pub fn encode_with_hint(&self, w: &mut dyn Write, hint: CodingHint) -> io::Result<()> {
        encoded_s2point_vector::encode_s2point_vector(self.points(), hint, w)
    }
}

impl S2Decode for PointVector {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let points = encoded_s2point_vector::decode_s2point_vector(r)?;
        Ok(PointVector::new(points))
    }
}

// ─── Cell ───────────────────────────────────────────────────────────────

impl S2Encode for Cell {
    fn encode(&self, w: &mut dyn Write) -> io::Result<()> {
        self.id().encode(w)
    }
}

impl S2Decode for Cell {
    fn decode(r: &mut dyn Read) -> io::Result<Self> {
        let id = CellId::decode(r)?;
        Ok(Cell::from(id))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn roundtrip_encode_decode<T: S2Encode + S2Decode + std::fmt::Debug>(val: &T) -> T {
        let mut buf = Vec::new();
        val.encode(&mut buf).expect("encode failed");
        T::decode(&mut buf.as_slice()).expect("decode failed")
    }

    #[test]
    fn test_cell_id_roundtrip() {
        let id = CellId::from_face(3);
        let back = roundtrip_encode_decode(&id);
        assert_eq!(id, back);

        // Leaf cell
        let leaf = CellId::from_point(&LatLng::from_degrees(45.0, 90.0).to_point());
        let back = roundtrip_encode_decode(&leaf);
        assert_eq!(leaf, back);
    }

    #[test]
    fn test_point_roundtrip() {
        let p = LatLng::from_degrees(37.7749, -122.4194).to_point();
        let back = roundtrip_encode_decode(&p);
        assert_eq!(p, back);
    }

    #[test]
    fn test_cap_roundtrip() {
        let cap = Cap::from_center_angle(
            LatLng::from_degrees(0.0, 0.0).to_point(),
            s1::Angle::from_degrees(5.0),
        );
        let back = roundtrip_encode_decode(&cap);
        assert_eq!(cap.center(), back.center());
        assert_eq!(cap.chord_radius(), back.chord_radius());
    }

    #[test]
    fn test_cap_empty_roundtrip() {
        let cap = Cap::empty();
        let back = roundtrip_encode_decode(&cap);
        assert!(back.is_empty());
    }

    #[test]
    fn test_cap_full_roundtrip() {
        let cap = Cap::full();
        let back = roundtrip_encode_decode(&cap);
        assert!(back.is_full());
    }

    #[test]
    fn test_rect_roundtrip() {
        let rect = Rect::empty()
            .add_point(LatLng::from_degrees(10.0, 20.0))
            .add_point(LatLng::from_degrees(30.0, 40.0));
        let back = roundtrip_encode_decode(&rect);
        assert_eq!(rect, back);
    }

    #[test]
    fn test_cell_union_roundtrip() {
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0), CellId::from_face(3)]);
        let back = roundtrip_encode_decode(&cu);
        assert_eq!(cu.len(), back.len());
        for i in 0..cu.len() {
            assert_eq!(cu[i], back[i]);
        }
    }

    #[test]
    fn test_polyline_roundtrip() {
        let pl = crate::s2::polyline::Polyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 1.0).to_point(),
            LatLng::from_degrees(2.0, 0.0).to_point(),
        ]);
        let back = roundtrip_encode_decode(&pl);
        assert_eq!(pl.len(), back.len());
    }

    #[test]
    fn test_loop_roundtrip() {
        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let back = roundtrip_encode_decode(&l);
        assert_eq!(l.num_vertices(), back.num_vertices());
        assert_eq!(l.depth(), back.depth());
        assert_eq!(l.contains_origin(), back.contains_origin());
    }

    #[test]
    fn test_loop_empty_roundtrip() {
        let l = Loop::empty();
        let back = roundtrip_encode_decode(&l);
        assert!(back.is_empty_loop());
    }

    #[test]
    fn test_loop_full_roundtrip() {
        let l = Loop::full();
        let back = roundtrip_encode_decode(&l);
        assert!(back.is_full_loop());
    }

    #[test]
    fn test_polygon_roundtrip() {
        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let p = Polygon::from_loops(vec![l]);
        let back = roundtrip_encode_decode(&p);
        assert_eq!(p.num_loops(), back.num_loops());
        assert_eq!(p.has_holes(), back.has_holes());
    }

    #[test]
    fn test_polygon_empty_roundtrip() {
        let p = Polygon::empty();
        let back = roundtrip_encode_decode(&p);
        assert!(back.is_empty_polygon());
    }

    #[test]
    fn test_cell_roundtrip() {
        let cell = Cell::from(CellId::from_point(
            &LatLng::from_degrees(45.0, 90.0).to_point(),
        ));
        let back = roundtrip_encode_decode(&cell);
        assert_eq!(cell.id(), back.id());
    }

    #[test]
    fn test_invalid_version() {
        // Point decode needs 24 bytes (3 doubles); too-short input fails.
        let buf = vec![99u8, 0, 0, 0];
        let result = Point::decode(&mut buf.as_slice());
        assert!(result.is_err());

        // Rect decode checks version byte.
        let buf = vec![
            99u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        let result = Rect::decode(&mut buf.as_slice());
        assert!(result.is_err());

        // Polyline decode checks version byte.
        let buf = vec![99u8, 0, 0, 0, 0];
        let result = crate::s2::polyline::Polyline::decode(&mut buf.as_slice());
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_decode_polyline() {
        let pl = crate::s2::polyline::Polyline::new(vec![
            LatLng::from_degrees(10.0, 20.0).to_point(),
            LatLng::from_degrees(30.0, 40.0).to_point(),
            LatLng::from_degrees(50.0, 60.0).to_point(),
            LatLng::from_degrees(70.0, 80.0).to_point(),
        ]);

        let mut buf = Vec::new();
        pl.encode(&mut buf).expect("encode failed");
        let back =
            crate::s2::polyline::Polyline::decode(&mut buf.as_slice()).expect("decode failed");

        assert_eq!(pl.len(), back.len(), "vertex count mismatch");
        for i in 0..pl.len() {
            assert_eq!(
                pl[i], back[i],
                "vertex {i} mismatch: {:?} vs {:?}",
                pl[i], back[i]
            );
        }
    }

    #[test]
    fn test_encode_decode_polygon_with_holes() {
        use crate::s2::Loop;

        // Outer loop: a large triangle.
        let outer = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 20.0).to_point(),
            LatLng::from_degrees(20.0, 10.0).to_point(),
        ]);
        // Hole: a smaller triangle inside the outer loop.
        let hole = Loop::new(vec![
            LatLng::from_degrees(5.0, 8.0).to_point(),
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(5.0, 12.0).to_point(),
        ]);

        let polygon = Polygon::from_loops(vec![outer, hole]);
        assert_eq!(polygon.num_loops(), 2);
        assert!(polygon.has_holes());

        let mut buf = Vec::new();
        polygon.encode(&mut buf).expect("encode failed");
        let back = Polygon::decode(&mut buf.as_slice()).expect("decode failed");

        assert_eq!(polygon.num_loops(), back.num_loops(), "loop count mismatch");
        assert_eq!(polygon.has_holes(), back.has_holes(), "has_holes mismatch");
        // Areas should match closely.
        let area_diff = (polygon.area() - back.area()).abs();
        assert!(
            area_diff < 1e-10,
            "area mismatch: {} vs {}, diff = {}",
            polygon.area(),
            back.area(),
            area_diff
        );
    }

    // ─── Compressed encoding tests ──────────────────────────────────────

    #[test]
    fn test_loop_compressed_roundtrip() {
        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let snap_level = Level::MAX;
        let mut buf = Vec::new();
        encode_loop_compressed(&l, &mut buf, snap_level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), snap_level).unwrap();
        assert_eq!(l.num_vertices(), back.num_vertices());
        assert_eq!(l.contains_origin(), back.contains_origin());
        assert_eq!(l.depth(), back.depth());
    }

    #[test]
    fn test_loop_compressed_empty_full() {
        let snap_level = Level::MAX;

        let empty = Loop::empty();
        let mut buf = Vec::new();
        encode_loop_compressed(&empty, &mut buf, snap_level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), snap_level).unwrap();
        assert_eq!(empty.num_vertices(), back.num_vertices());
        assert_eq!(empty.contains_origin(), back.contains_origin());

        let full = Loop::full();
        buf.clear();
        encode_loop_compressed(&full, &mut buf, snap_level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), snap_level).unwrap();
        assert_eq!(full.num_vertices(), back.num_vertices());
        assert_eq!(full.contains_origin(), back.contains_origin());
    }

    #[test]
    fn test_loop_compressed_with_bound() {
        // Create a loop with >= 64 vertices so the bound gets encoded.
        let mut vertices = Vec::new();
        let n = 100;
        for i in 0..n {
            let angle = 2.0 * std::f64::consts::PI * f64::from(i) / f64::from(n);
            let lat = 0.5 * angle.sin();
            let lng = 0.5 * angle.cos();
            vertices.push(LatLng::from_degrees(lat, lng).to_point());
        }
        let l = Loop::new(vertices);
        assert!(l.num_vertices() >= 64);

        let snap_level = Level::MAX;
        let mut buf = Vec::new();
        encode_loop_compressed(&l, &mut buf, snap_level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), snap_level).unwrap();
        assert_eq!(l.num_vertices(), back.num_vertices());
        assert_eq!(l.contains_origin(), back.contains_origin());
        assert_eq!(l.depth(), back.depth());
    }

    #[test]
    fn test_polygon_compressed_roundtrip() {
        // Use cell-center vertices so the auto-selector picks compressed.
        let level = Level::new(10);
        let base = CellId::from_face(0).child_begin_at_level(level);
        let mut cells = Vec::new();
        let mut id = base;
        for _ in 0..20 {
            cells.push(id.to_point());
            id = id.next();
        }
        let outer = Loop::new(vec![cells[0], cells[5], cells[10]]);
        let polygon = Polygon::from_loops(vec![outer]);

        // Force compressed encoding explicitly.
        let all_verts = points_to_xyz_face_si_ti(polygon.loops()[0].vertices());
        let mut buf = Vec::new();
        encode_polygon_compressed(&polygon, &mut buf, &all_verts, level).unwrap();
        assert_eq!(buf[0], COMPRESSED_ENCODING_VERSION);

        let back = Polygon::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(polygon.num_loops(), back.num_loops());
        let area_diff = (polygon.area() - back.area()).abs();
        assert!(area_diff < 1e-10, "area mismatch: diff = {area_diff}");
    }

    #[test]
    fn test_polygon_full_roundtrip() {
        let p = Polygon::full();
        let mut buf = Vec::new();
        p.encode(&mut buf).expect("encode full polygon");
        let back = Polygon::decode(&mut buf.as_slice()).expect("decode full polygon");
        assert!(back.is_full_polygon(), "decoded polygon should be full");
        assert_eq!(p.num_loops(), back.num_loops());
    }

    #[test]
    fn test_polygon_two_loops_roundtrip() {
        // Two non-overlapping triangles.
        let l1 = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 5.0).to_point(),
            LatLng::from_degrees(5.0, 0.0).to_point(),
        ]);
        let l2 = Loop::new(vec![
            LatLng::from_degrees(20.0, 20.0).to_point(),
            LatLng::from_degrees(20.0, 25.0).to_point(),
            LatLng::from_degrees(25.0, 20.0).to_point(),
        ]);
        let p = Polygon::from_loops(vec![l1, l2]);
        assert_eq!(p.num_loops(), 2);

        let mut buf = Vec::new();
        p.encode(&mut buf).expect("encode 2-loop polygon");
        let back = Polygon::decode(&mut buf.as_slice()).expect("decode 2-loop polygon");
        assert_eq!(p.num_loops(), back.num_loops());
        assert_eq!(p.num_vertices(), back.num_vertices());
    }

    #[test]
    fn test_cell_encode_decode_multiple() {
        // Encode/decode cells at various levels.
        for face in 0..6u8 {
            let cell = Cell::from(CellId::from_face(face));
            let back = roundtrip_encode_decode(&cell);
            assert_eq!(cell.id(), back.id());
        }
        // A leaf cell.
        let leaf = Cell::from(CellId::from_point(
            &LatLng::from_degrees(37.0, -122.0).to_point(),
        ));
        let back = roundtrip_encode_decode(&leaf);
        assert_eq!(leaf.id(), back.id());
        assert_eq!(leaf.level(), back.level());
    }

    #[test]
    fn test_polygon_decode_both_versions() {
        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let polygon = Polygon::from_loops(vec![l]);

        // Encode as lossless (version 1).
        let mut lossless_buf = Vec::new();
        encode_polygon_lossless(&polygon, &mut lossless_buf).unwrap();
        assert_eq!(lossless_buf[0], ENCODING_VERSION);
        let back1 = Polygon::decode(&mut lossless_buf.as_slice()).unwrap();
        assert_eq!(polygon.num_loops(), back1.num_loops());

        // Encode as compressed (version 4).
        let vertices = points_to_xyz_face_si_ti(polygon.loops()[0].vertices());
        let mut compressed_buf = Vec::new();
        encode_polygon_compressed(&polygon, &mut compressed_buf, &vertices, Level::MAX).unwrap();
        assert_eq!(compressed_buf[0], COMPRESSED_ENCODING_VERSION);
        let back4 = Polygon::decode(&mut compressed_buf.as_slice()).unwrap();
        assert_eq!(polygon.num_loops(), back4.num_loops());
    }
    // ─── C++ wire-format compatibility tests ─────────────────────────────

    #[test]
    fn test_point_wire_format() {
        // C++ S2Point: 3 raw doubles (24 bytes), no version byte.
        let p = Point(crate::r3::Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let mut buf = Vec::new();
        p.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), 24, "S2Point should be exactly 24 bytes");
        // First 8 bytes should be f64 1.0 in little-endian.
        assert_eq!(&buf[0..8], &1.0f64.to_le_bytes());
        assert_eq!(&buf[8..16], &0.0f64.to_le_bytes());
        assert_eq!(&buf[16..24], &0.0f64.to_le_bytes());
    }

    #[test]
    fn test_cap_wire_format() {
        // C++ S2Cap: 4 raw doubles (32 bytes), no version byte.
        let cap = Cap::from_center_chord_angle(
            Point(crate::r3::Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            }),
            ChordAngle::ZERO,
        );
        let mut buf = Vec::new();
        cap.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), 32, "S2Cap should be exactly 32 bytes");
        assert_eq!(&buf[0..8], &1.0f64.to_le_bytes());
        assert_eq!(&buf[8..16], &0.0f64.to_le_bytes());
        assert_eq!(&buf[16..24], &0.0f64.to_le_bytes());
        assert_eq!(&buf[24..32], &0.0f64.to_le_bytes()); // radius = 0
    }

    #[test]
    fn test_cell_id_wire_format() {
        // C++ S2CellId: raw u64 (8 bytes).
        let id = CellId::from_face(3);
        let mut buf = Vec::new();
        id.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), 8, "S2CellId should be exactly 8 bytes");
    }

    #[test]
    fn test_rect_wire_format() {
        // C++ S2LatLngRect: version(1) + 4 doubles(32) = 33 bytes.
        let rect = Rect::empty();
        let mut buf = Vec::new();
        rect.encode(&mut buf).unwrap();
        assert_eq!(buf.len(), 33, "S2LatLngRect should be exactly 33 bytes");
        assert_eq!(buf[0], 1, "version should be 1");
    }

    #[test]
    fn test_loop_wire_format() {
        // C++ S2Loop: version(1) + num_verts(4) + verts(24*N) + origin(1)
        //             + depth(4) + bound(33) bytes.
        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let mut buf = Vec::new();
        l.encode(&mut buf).unwrap();
        // 1 + 4 + 24*3 + 1 + 4 + 33 = 115
        assert_eq!(buf.len(), 115, "S2Loop(3 verts) should be 115 bytes");
        assert_eq!(buf[0], 1, "version");
        assert_eq!(
            u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
            3,
            "num_vertices"
        );
    }

    #[test]
    fn test_polyline_wire_format() {
        // C++ S2Polyline lossless: version(1) + num_verts(4) + verts(24*N).
        let pl = crate::s2::polyline::Polyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 1.0).to_point(),
        ]);
        let mut buf = Vec::new();
        pl.encode(&mut buf).unwrap();
        // 1 + 4 + 24*2 = 53
        assert_eq!(buf.len(), 53, "S2Polyline(2 verts) should be 53 bytes");
        assert_eq!(buf[0], POLYLINE_LOSSLESS_VERSION, "version");
        assert_eq!(
            u32::from_le_bytes([buf[1], buf[2], buf[3], buf[4]]),
            2,
            "num_vertices"
        );
    }

    #[test]
    fn test_cell_union_wire_format() {
        // C++ S2CellUnion: version(1) + count(8) + cell_ids(8*N).
        let cu = CellUnion::from_cell_ids(vec![CellId::from_face(0), CellId::from_face(3)]);
        let mut buf = Vec::new();
        cu.encode(&mut buf).unwrap();
        // 1 + 8 + 8*2 = 25
        assert_eq!(buf.len(), 25, "S2CellUnion(2 cells) should be 25 bytes");
        assert_eq!(buf[0], 1, "version");
        assert_eq!(
            u64::from_le_bytes([
                buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8]
            ]),
            2,
            "num_cells"
        );
    }

    #[test]
    fn test_polyline_compressed_roundtrip() {
        let pl = crate::s2::polyline::Polyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 1.0).to_point(),
            LatLng::from_degrees(2.0, 0.0).to_point(),
        ]);
        let mut buf = Vec::new();
        encode_polyline_compressed(&pl, &mut buf).unwrap();
        let back = crate::s2::polyline::Polyline::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(pl.len(), back.len());
    }

    #[test]
    fn test_polyline_compressed_empty_roundtrip() {
        let pl = crate::s2::polyline::Polyline::new(vec![]);
        let mut buf = Vec::new();
        encode_polyline_compressed(&pl, &mut buf).unwrap();
        let back = crate::s2::polyline::Polyline::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(0, back.len());
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use crate::s2::LatLng;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn prop_uvarint_roundtrip(val: u64) -> bool {
        let mut buf = Vec::new();
        write_uvarint(&mut buf, val).unwrap();
        let back = read_uvarint(&mut buf.as_slice()).unwrap();
        back == val
    }

    #[quickcheck]
    fn prop_uvarint_size(val: u64) -> bool {
        // Varint encoding uses ceil(bits/7) bytes.
        let mut buf = Vec::new();
        write_uvarint(&mut buf, val).unwrap();
        let expected_bytes = if val == 0 {
            1
        } else {
            (64 - val.leading_zeros() as usize).div_ceil(7)
        };
        buf.len() == expected_bytes
    }

    #[quickcheck]
    fn prop_uvarint_small_values_compact(val: u8) -> bool {
        // Values < 128 should encode in exactly 1 byte.
        let mut buf = Vec::new();
        write_uvarint(&mut buf, u64::from(val)).unwrap();
        if val < 128 {
            buf.len() == 1 && buf[0] == val
        } else {
            buf.len() == 2
        }
    }

    #[quickcheck]
    fn prop_loop_compressed_preserves_vertex_count(
        lat1: i16,
        lng1: i16,
        lat2: i16,
        lng2: i16,
    ) -> bool {
        // Use i16 to avoid NaN/Infinity issues.
        let p0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p1 = LatLng::from_degrees(f64::from(lat1) / 200.0, f64::from(lng1) / 200.0).to_point();
        let p2 = LatLng::from_degrees(f64::from(lat2) / 200.0, f64::from(lng2) / 200.0).to_point();

        // Need non-degenerate triangle.
        if p0 == p1 || p0 == p2 || p1 == p2 {
            return true;
        }
        let cross = (p1.0 - p0.0).cross(p2.0 - p0.0).norm();
        if cross < 1e-10 {
            return true; // near-degenerate
        }

        let l = Loop::new(vec![p0, p1, p2]);
        let snap_level = Level::MAX;
        let mut buf = Vec::new();
        encode_loop_compressed(&l, &mut buf, snap_level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), snap_level).unwrap();
        l.num_vertices() == back.num_vertices()
            && l.contains_origin() == back.contains_origin()
            && l.depth() == back.depth()
    }

    #[quickcheck]
    fn prop_loop_compressed_at_various_snap_levels(level_raw: u8) -> bool {
        // Test loop compressed encoding at every valid snap level.
        let level = Level::new(level_raw % 31); // 0..30

        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 0.0).to_point(),
        ]);
        let mut buf = Vec::new();
        encode_loop_compressed(&l, &mut buf, level).unwrap();
        let back = decode_loop_compressed(&mut buf.as_slice(), level).unwrap();
        l.num_vertices() == back.num_vertices() && l.contains_origin() == back.contains_origin()
    }

    #[quickcheck]
    fn prop_polygon_encode_decode_roundtrip(lat: i16, lng: i16) -> bool {
        // Build a simple triangle polygon from varying coordinates.
        let p0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p1 = LatLng::from_degrees(0.0, 10.0).to_point();
        // Vary the third vertex.
        let lat_f = (f64::from(lat) / 3000.0).clamp(0.5, 89.0);
        let lng_f = (f64::from(lng) / 3000.0).clamp(0.5, 9.5);
        let p2 = LatLng::from_degrees(lat_f, lng_f).to_point();

        let polygon = Polygon::from_loops(vec![Loop::new(vec![p0, p1, p2])]);
        if polygon.num_loops() == 0 {
            return true; // degenerate
        }

        let mut buf = Vec::new();
        polygon.encode(&mut buf).unwrap();
        let back = Polygon::decode(&mut buf.as_slice()).unwrap();

        polygon.num_loops() == back.num_loops() && (polygon.area() - back.area()).abs() < 1e-8
    }

    #[quickcheck]
    fn prop_polygon_lossless_compressed_agree(lat: i16, lng: i16) -> bool {
        // Both lossless and compressed should produce decodable results
        // that agree on basic properties.
        let lat_f = (f64::from(lat) / 3000.0).clamp(1.0, 80.0);
        let lng_f = (f64::from(lng) / 3000.0).clamp(1.0, 170.0);

        let l = Loop::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, lng_f).to_point(),
            LatLng::from_degrees(lat_f, lng_f / 2.0).to_point(),
        ]);
        let polygon = Polygon::from_loops(vec![l]);
        if polygon.num_loops() == 0 {
            return true;
        }

        // Lossless.
        let mut lossless_buf = Vec::new();
        encode_polygon_lossless(&polygon, &mut lossless_buf).unwrap();
        let back_lossless = Polygon::decode(&mut lossless_buf.as_slice()).unwrap();

        // Compressed at max level.
        let mut all_verts = Vec::new();
        for loop_ in polygon.loops() {
            all_verts.extend(points_to_xyz_face_si_ti(loop_.vertices()));
        }
        let mut compressed_buf = Vec::new();
        encode_polygon_compressed(&polygon, &mut compressed_buf, &all_verts, Level::MAX).unwrap();
        let back_compressed = Polygon::decode(&mut compressed_buf.as_slice()).unwrap();

        back_lossless.num_loops() == back_compressed.num_loops()
            && (back_lossless.area() - back_compressed.area()).abs() < 1e-8
    }

    #[quickcheck]
    fn prop_polygon_auto_selector_always_decodable(raw: u64) -> bool {
        // The auto-selecting Polygon::encode should always produce
        // something that Polygon::decode can read back.
        let face = (raw % 6) as u8;
        let level = ((raw >> 3) % 25 + 5) as u8;
        let pos = raw >> 8;
        let base = CellId::from_face_pos_level(face, pos, level);

        let mut points = Vec::new();
        let mut id = base;
        for _ in 0..3 {
            points.push(id.to_point());
            id = id.next();
        }

        let polygon = Polygon::from_loops(vec![Loop::new(points)]);
        if polygon.num_loops() == 0 {
            return true;
        }

        let mut buf = Vec::new();
        polygon.encode(&mut buf).unwrap();
        let back = Polygon::decode(&mut buf.as_slice()).unwrap();
        polygon.num_loops() == back.num_loops()
    }
}
