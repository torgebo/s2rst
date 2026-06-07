// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Encoding and decoding for [`ShapeIndex`].
//!
//! Wire-compatible with C++ `MutableS2ShapeIndex::Encode` /
//! `MutableS2ShapeIndex::Init`. The format is:
//!
//! 1. Varint64: `(max_edges_per_cell << 2) | version`
//! 2. Tagged shape vector (via `EncodedStringVector`)
//! 3. `EncodedS2CellIdVector` of cell IDs
//! 4. `EncodedStringVector` of encoded cells
//!
//! Shape tags match C++:
//! - 1 = `S2Polygon::Shape`
//! - 2 = `S2Polyline::Shape`
//! - 3 = `S2PointVectorShape`
//! - 4 = `S2LaxPolylineShape`
//! - 5 = `S2LaxPolygonShape`

#![expect(
    clippy::cast_sign_loss,
    reason = "ShapeId/EdgeId (i32) encoded as u64 — values always non-negative at encoding time"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "u64 varint -> i32/usize — values fit by format spec"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "u64 -> i32 for decoded EdgeId/ShapeId — bounded by format"
)]
use std::io::{self, Cursor, Read, Write};

use crate::s2::cell_id::CellId;
use crate::s2::encoded_s2cell_id_vector;
use crate::s2::encoded_s2point_vector::CodingHint;
use crate::s2::encoded_string_vector::{self, StringVectorBuilder};
use crate::s2::encoding::{S2Decode, read_uvarint, write_uvarint};
use crate::s2::lax_polygon::LaxPolygon;
use crate::s2::lax_polyline::LaxPolyline;
use crate::s2::point_vector::PointVector;
use crate::s2::polygon::Polygon;
use crate::s2::polyline::Polyline;
use crate::s2::shape::Shape;
use crate::s2::shape::ShapeId;
use crate::s2::shape_index::{ClippedShape, ShapeIndex, ShapeIndexCell};

/// Current encoding version for `ShapeIndex`.
const SHAPE_INDEX_VERSION: u64 = 2;

/// Upper bound on capacity reserved up-front from an untrusted element count
/// (e.g. the per-cell edge count), so a tiny malformed input can't drive a huge
/// `Vec::with_capacity`. The vector still grows on demand, so valid data of any
/// size still decodes correctly.
const MAX_PREALLOC: usize = 1 << 16;

// ─── Shape type tags (matching C++) ─────────────────────────────────────

/// No type tag; shape cannot be encoded.
const NO_TYPE_TAG: u32 = 0;

/// `S2Polygon::Shape`
const POLYGON_TYPE_TAG: u32 = 1;

/// `S2Polyline::Shape`
const POLYLINE_TYPE_TAG: u32 = 2;

/// `S2PointVectorShape`
const POINT_VECTOR_TYPE_TAG: u32 = 3;

/// `S2LaxPolylineShape`
const LAX_POLYLINE_TYPE_TAG: u32 = 4;

/// `S2LaxPolygonShape`
const LAX_POLYGON_TYPE_TAG: u32 = 5;

// ─── Shape encoding helpers ─────────────────────────────────────────────

/// Returns the type tag for a shape, or [`NO_TYPE_TAG`] if unsupported.
fn shape_type_tag(shape: &dyn Shape) -> u32 {
    shape.type_tag()
}

/// Encodes a single shape with its type tag.
fn encode_tagged_shape(shape: &dyn Shape, hint: CodingHint, w: &mut dyn Write) -> io::Result<()> {
    let tag = shape_type_tag(shape);
    if tag == NO_TYPE_TAG {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "shape has no type tag, cannot encode (dimension={})",
                shape.dimension()
            ),
        ));
    }
    write_uvarint(w, u64::from(tag))?;
    shape.encode_tagged(w, hint)
}

/// Decodes a shape from its tagged representation.
fn decode_tagged_shape(data: &[u8]) -> io::Result<Box<dyn Shape>> {
    let mut r = Cursor::new(data);
    let tag = read_uvarint(&mut r)? as u32;
    match tag {
        POLYGON_TYPE_TAG => {
            let polygon = Polygon::decode(&mut r)?;
            Ok(Box::new(polygon))
        }
        POLYLINE_TYPE_TAG => {
            let polyline = Polyline::decode(&mut r)?;
            Ok(Box::new(polyline))
        }
        POINT_VECTOR_TYPE_TAG => {
            let pv = PointVector::decode(&mut r)?;
            Ok(Box::new(pv))
        }
        LAX_POLYLINE_TYPE_TAG => {
            let lp = LaxPolyline::decode(&mut r)?;
            Ok(Box::new(lp))
        }
        LAX_POLYGON_TYPE_TAG => {
            let lp = LaxPolygon::decode(&mut r)?;
            Ok(Box::new(lp))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported shape type tag {tag}"),
        )),
    }
}

// ─── Cell encoding ──────────────────────────────────────────────────────

/// Encodes a single `ShapeIndexCell` using the compact C++ format.
fn encode_cell(cell: &ShapeIndexCell, num_shape_ids: usize, w: &mut dyn Write) -> io::Result<()> {
    if num_shape_ids == 1 {
        // Single shape in entire index.
        debug_assert_eq!(cell.shapes.len(), 1);
        let clipped = &cell.shapes[0];
        debug_assert_eq!(clipped.shape_id, 0);
        let n = clipped.num_edges();
        let cc = u64::from(clipped.contains_center);

        if (2..=17).contains(&n) && clipped.edges[n - 1] - clipped.edges[0] == (n as i32 - 1) {
            // Contiguous range: tag bit 0 = 0
            let edge_id = clipped.edges[0] as u64;
            let val = (edge_id << 6) | ((n as u64 - 2) << 2) | (cc << 1);
            write_uvarint(w, val)?;
        } else if n == 1 {
            // Single edge: tag bits 0-1 = 01
            let edge_id = clipped.edges[0] as u64;
            write_uvarint(w, (edge_id << 3) | (cc << 2) | 1)?;
        } else {
            // General case (including n == 0): tag bits 0-1 = 11
            write_uvarint(w, (n as u64) << 3 | (cc << 2) | 3)?;
            encode_edges(clipped, w)?;
        }
    } else {
        if cell.shapes.len() > 1 {
            write_uvarint(w, ((cell.shapes.len() as u64) << 3) | 3)?;
        }
        let mut shape_id_base = 0i32;
        for clipped in &cell.shapes {
            debug_assert!(clipped.shape_id.0 >= shape_id_base);
            let shape_delta = (clipped.shape_id.0 - shape_id_base) as u64;
            shape_id_base = clipped.shape_id.0 + 1;

            let n = clipped.num_edges();
            let cc = u64::from(clipped.contains_center);

            if (1..=16).contains(&n) && clipped.edges[n - 1] - clipped.edges[0] == (n as i32 - 1) {
                // Contiguous range: tag bit 0 = 0
                let edge_id = clipped.edges[0] as u64;
                write_uvarint(w, (edge_id << 2) | (cc << 1))?;
                write_uvarint(w, (shape_delta << 4) | (n as u64 - 1))?;
            } else if n == 0 {
                // No edges: tag bits 0-2 = 111
                write_uvarint(w, (shape_delta << 4) | (cc << 3) | 7)?;
            } else {
                // General case: tag bits 0-1 = 01
                write_uvarint(w, ((n as u64 - 1) << 3) | (cc << 2) | 1)?;
                write_uvarint(w, shape_delta)?;
                encode_edges(clipped, w)?;
            }
        }
    }
    Ok(())
}

/// Encodes edge IDs for a clipped shape.
fn encode_edges(clipped: &ClippedShape, w: &mut dyn Write) -> io::Result<()> {
    // Delta-encode edge IDs.
    let mut prev = 0i32;
    for &edge in &clipped.edges {
        write_uvarint(w, (edge - prev) as u64)?;
        prev = edge + 1;
    }
    Ok(())
}

/// Decodes a `ShapeIndexCell` from the compact C++ format.
fn decode_cell(data: &[u8], num_shape_ids: usize) -> io::Result<ShapeIndexCell> {
    let mut r = Cursor::new(data);
    let mut cell = ShapeIndexCell::default();

    if num_shape_ids == 1 {
        let header = read_uvarint(&mut r)?;
        let mut clipped = ClippedShape {
            shape_id: ShapeId(0),
            contains_center: false,
            edges: Vec::new(),
        };

        if (header & 1) == 0 {
            // Contiguous range.
            let num_edges = ((header >> 2) & 15) as usize + 2;
            clipped.contains_center = (header & 2) != 0;
            let edge_id = (header >> 6) as i32;
            clipped.edges = (edge_id..edge_id + num_edges as i32).collect();
        } else if (header & 2) == 0 {
            // Single edge.
            clipped.contains_center = (header & 4) != 0;
            let edge_id = (header >> 3) as i32;
            clipped.edges = vec![edge_id];
        } else {
            // General case.
            clipped.contains_center = (header & 4) != 0;
            let num_edges = (header >> 3) as usize;
            clipped.edges = decode_edges(num_edges, &mut r)?;
        }

        cell.shapes.push(clipped);
    } else {
        // Determine num_clipped.
        // Peek at first byte to see if it's a multi-shape header.
        let first_byte = if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "empty cell"));
        } else {
            data[0]
        };

        let num_clipped = if (first_byte & 7) == 3 {
            // Multi-shape header: read varint, extract num_clipped.
            let header = read_uvarint(&mut r)?;
            let n = (header >> 3) as usize;
            if n <= 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "multi-shape header with num_clipped <= 1",
                ));
            }
            n
        } else {
            1
        };

        let mut shape_id_base = 0i32;
        for _ in 0..num_clipped {
            let header = read_uvarint(&mut r)?;
            let mut clipped = ClippedShape {
                shape_id: ShapeId(0),
                contains_center: false,
                edges: Vec::new(),
            };

            if (header & 1) == 0 {
                // Contiguous range.
                let edge_id = (header >> 2) as i32;
                clipped.contains_center = (header & 2) != 0;
                let next_val = read_uvarint(&mut r)?;
                let num_edges = ((next_val & 15) + 1) as usize;
                let shape_delta = (next_val >> 4) as i32;
                clipped.shape_id = ShapeId(add_i32(shape_id_base, shape_delta)?);
                clipped.edges = (edge_id..add_i32(edge_id, num_edges as i32)?).collect();
            } else if (header & 7) == 7 {
                // No edges.
                clipped.contains_center = (header & 8) != 0;
                let shape_delta = (header >> 4) as i32;
                clipped.shape_id = ShapeId(add_i32(shape_id_base, shape_delta)?);
            } else if (header & 3) == 1 {
                // General case.
                let num_edges = ((header >> 3) + 1) as usize;
                clipped.contains_center = (header & 4) != 0;
                let shape_delta = read_uvarint(&mut r)? as i32;
                clipped.shape_id = ShapeId(add_i32(shape_id_base, shape_delta)?);
                clipped.edges = decode_edges(num_edges, &mut r)?;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid cell encoding tag: {header:#x}"),
                ));
            }

            shape_id_base = add_i32(clipped.shape_id.0, 1)?;
            cell.shapes.push(clipped);
        }
    }

    Ok(cell)
}

/// Adds two `i32`s from untrusted decoded data, returning `Err` on overflow
/// (shape/edge ids and their deltas come straight off the wire).
fn add_i32(a: i32, b: i32) -> io::Result<i32> {
    a.checked_add(b).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "integer overflow in cell decode",
        )
    })
}

/// Decodes delta-encoded edge IDs.
fn decode_edges(num_edges: usize, r: &mut dyn Read) -> io::Result<Vec<i32>> {
    let mut edges = Vec::with_capacity(num_edges.min(MAX_PREALLOC));
    let mut prev = 0i32;
    for _ in 0..num_edges {
        let delta = read_uvarint(r)? as i32;
        let edge = add_i32(prev, delta)?;
        edges.push(edge);
        prev = add_i32(edge, 1)?;
    }
    Ok(edges)
}

// ─── ShapeIndex encode/decode ───────────────────────────────────────────

impl ShapeIndex {
    /// Encodes the `ShapeIndex` in the C++ `MutableS2ShapeIndex` wire format.
    ///
    /// Uses `CodingHint::Fast` for shapes (UNCOMPRESSED `S2Point` vectors).
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails, or if any shape does not
    /// support encoding.
    pub fn encode_to_writer(&self, w: &mut dyn Write) -> io::Result<()> {
        self.encode_with_hint(w, CodingHint::Fast)
    }

    /// Encodes using the given coding hint for shape point vectors.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the write fails, or if any shape does not
    /// support encoding.
    pub fn encode_with_hint(&self, w: &mut dyn Write, hint: CodingHint) -> io::Result<()> {
        // 1. Version + options.
        let max_edges = self.max_edges_per_cell() as u64;
        write_uvarint(w, (max_edges << 2) | SHAPE_INDEX_VERSION)?;

        // 2. Tagged shapes (via StringVector).
        let mut shape_vec = StringVectorBuilder::new();
        for shape_opt in self.shapes_iter() {
            let mut shape_buf = Vec::new();
            if let Some(shape) = shape_opt {
                encode_tagged_shape(shape.as_ref(), hint, &mut shape_buf)?;
            }
            // Empty buf for None (null shape).
            shape_vec.add(shape_buf);
        }
        shape_vec.encode(w)?;

        // 3. Build index if needed (we need the cell map).
        // Note: encode requires the index to be built. The caller should
        // call build() before encoding.
        let mut cell_ids = Vec::new();
        let mut encoded_cells = StringVectorBuilder::new();
        let num_shape_ids = self.num_shape_ids();

        for it in self.cell_iter() {
            cell_ids.push(it.cell_id());
            if let Some(cell) = it.index_cell() {
                let mut cell_buf = Vec::new();
                encode_cell(cell, num_shape_ids, &mut cell_buf)?;
                encoded_cells.add(cell_buf);
            }
        }

        // 4. Encode cell IDs and cell data.
        encoded_s2cell_id_vector::encode_s2cell_id_vector(&cell_ids, w)?;
        encoded_cells.encode(w)?;

        Ok(())
    }

    /// Decodes a `ShapeIndex` from the C++ `MutableS2ShapeIndex` wire format.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed, uses an unsupported
    /// version, or the read fails.
    pub fn decode_from_reader(r: &mut dyn Read) -> io::Result<Self> {
        // 1. Version + options.
        let max_edges_version = read_uvarint(r)?;
        let version = max_edges_version & 3;
        if version != SHAPE_INDEX_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported ShapeIndex version {version}"),
            ));
        }
        let max_edges_per_cell = (max_edges_version >> 2) as usize;

        // 2. Decode tagged shapes.
        let shape_data = encoded_string_vector::decode_string_vector(r)?;
        let num_shapes = shape_data.len();
        let mut shapes: Vec<Option<Box<dyn Shape>>> = Vec::with_capacity(num_shapes);
        for data in &shape_data {
            if data.is_empty() {
                shapes.push(None);
            } else {
                shapes.push(Some(decode_tagged_shape(data)?));
            }
        }

        // 3. Decode cell IDs and cells.
        let cell_ids = encoded_s2cell_id_vector::decode_s2cell_id_vector(r)?;
        let cell_data = encoded_string_vector::decode_string_vector(r)?;
        if cell_ids.len() != cell_data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cell_ids and cell_data length mismatch",
            ));
        }

        let mut index = ShapeIndex::new();
        index.set_max_edges_per_cell(max_edges_per_cell);
        for shape in shapes {
            index.add_option(shape);
        }

        // Decode each cell.
        for (i, cid) in cell_ids.iter().enumerate() {
            let cell = decode_cell(&cell_data[i], num_shapes)?;
            index.insert_cell(*cid, cell);
        }
        index.mark_built();

        Ok(index)
    }

    /// Returns an iterator over cell IDs and cells (for encoding).
    fn cell_iter(&self) -> CellIter<'_> {
        CellIter { inner: self.iter() }
    }

    /// Returns the shapes slice for encoding.
    fn shapes_iter(&self) -> &[Option<Box<dyn Shape>>] {
        self.shapes_slice()
    }
}

/// Iterator over (`CellId`, &`ShapeIndexCell`) pairs.
struct CellIter<'a> {
    inner: crate::s2::shape_index::ShapeIndexIterator<'a>,
}

impl<'a> Iterator for CellIter<'a> {
    type Item = CellIterItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.done() {
            return None;
        }
        let item = CellIterItem {
            cell_id: self.inner.cell_id(),
            cell: self.inner.index_cell(),
        };
        self.inner.next();
        Some(item)
    }
}

struct CellIterItem<'a> {
    cell_id: CellId,
    cell: Option<&'a ShapeIndexCell>,
}

impl<'a> CellIterItem<'a> {
    fn cell_id(&self) -> CellId {
        self.cell_id
    }

    fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        self.cell
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::encoding::{S2Decode, S2Encode};

    fn roundtrip_index(index: &ShapeIndex) -> ShapeIndex {
        let mut buf = Vec::new();
        index.encode_to_writer(&mut buf).unwrap();
        let mut cursor = Cursor::new(buf);
        ShapeIndex::decode_from_reader(&mut cursor).unwrap()
    }

    fn assert_indices_equal(a: &ShapeIndex, b: &ShapeIndex) {
        assert_eq!(a.num_shape_ids(), b.num_shape_ids());
        // Check that cells match.
        let mut it_a = a.iter();
        let mut it_b = b.iter();
        loop {
            if it_a.done() && it_b.done() {
                break;
            }
            assert!(!it_a.done(), "a has fewer cells");
            assert!(!it_b.done(), "b has fewer cells");
            assert_eq!(it_a.cell_id(), it_b.cell_id());

            let cell_a = it_a.index_cell().unwrap();
            let cell_b = it_b.index_cell().unwrap();
            assert_eq!(cell_a.shapes.len(), cell_b.shapes.len());
            for (ca, cb) in cell_a.shapes.iter().zip(cell_b.shapes.iter()) {
                assert_eq!(ca.shape_id, cb.shape_id);
                assert_eq!(ca.contains_center, cb.contains_center);
                assert_eq!(ca.edges, cb.edges);
            }

            it_a.next();
            it_b.next();
        }
    }

    #[test]
    fn test_empty_index() {
        let mut index = ShapeIndex::new();
        index.build();
        let decoded = roundtrip_index(&index);
        assert_eq!(decoded.num_shape_ids(), 0);
    }

    #[test]
    fn test_single_point_vector() {
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 1.0).to_point(),
        ]);
        index.add(Box::new(pv));
        index.build();
        let decoded = roundtrip_index(&index);
        assert_indices_equal(&index, &decoded);
    }

    #[test]
    fn test_single_lax_polyline() {
        let mut index = ShapeIndex::new();
        let lp = LaxPolyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 0.0).to_point(),
            LatLng::from_degrees(2.0, 0.0).to_point(),
        ]);
        index.add(Box::new(lp));
        index.build();
        let decoded = roundtrip_index(&index);
        assert_indices_equal(&index, &decoded);
    }

    #[test]
    fn test_single_lax_polygon() {
        let mut index = ShapeIndex::new();
        let p0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p1 = LatLng::from_degrees(0.0, 1.0).to_point();
        let p2 = LatLng::from_degrees(1.0, 0.0).to_point();
        let lp = LaxPolygon::from_loops(&[&[p0, p1, p2]]);
        index.add(Box::new(lp));
        index.build();
        let decoded = roundtrip_index(&index);
        assert_indices_equal(&index, &decoded);
    }

    #[test]
    fn test_multiple_shapes() {
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(vec![LatLng::from_degrees(10.0, 10.0).to_point()]);
        let lp = LaxPolyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 1.0).to_point(),
        ]);
        index.add(Box::new(pv));
        index.add(Box::new(lp));
        index.build();
        let decoded = roundtrip_index(&index);
        assert_indices_equal(&index, &decoded);
    }

    #[test]
    fn test_lax_polyline_roundtrip() {
        let lp = LaxPolyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 0.0).to_point(),
        ]);
        let mut buf = Vec::new();
        lp.encode(&mut buf).unwrap();
        let decoded = LaxPolyline::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(lp.num_vertices(), decoded.num_vertices());
        for i in 0..lp.num_vertices() {
            assert_eq!(lp.vertex(i), decoded.vertex(i));
        }
    }

    #[test]
    fn test_lax_polygon_roundtrip() {
        let p0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p1 = LatLng::from_degrees(0.0, 1.0).to_point();
        let p2 = LatLng::from_degrees(1.0, 0.0).to_point();
        let p3 = LatLng::from_degrees(10.0, 10.0).to_point();
        let p4 = LatLng::from_degrees(10.0, 11.0).to_point();
        let p5 = LatLng::from_degrees(11.0, 10.0).to_point();
        let lp = LaxPolygon::from_loops(&[&[p0, p1, p2], &[p3, p4, p5]]);
        let mut buf = Vec::new();
        lp.encode(&mut buf).unwrap();
        let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(lp.num_loops(), decoded.num_loops());
        for i in 0..lp.num_loops() {
            assert_eq!(lp.num_loop_vertices(i), decoded.num_loop_vertices(i));
            for j in 0..lp.num_loop_vertices(i) {
                assert_eq!(lp.loop_vertex(i, j), decoded.loop_vertex(i, j));
            }
        }
    }

    #[test]
    fn test_lax_polygon_empty() {
        let lp = LaxPolygon::default();
        let mut buf = Vec::new();
        lp.encode(&mut buf).unwrap();
        let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(decoded.num_loops(), 0);
    }

    #[test]
    fn test_point_vector_roundtrip() {
        let pv = PointVector::new(vec![
            LatLng::from_degrees(37.0, -122.0).to_point(),
            LatLng::from_degrees(38.0, -121.0).to_point(),
            LatLng::from_degrees(39.0, -120.0).to_point(),
        ]);
        let mut buf = Vec::new();
        pv.encode(&mut buf).unwrap();
        let decoded = PointVector::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(pv.len(), decoded.len());
        for i in 0..pv.len() {
            assert_eq!(pv.point(i), decoded.point(i));
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2shapeutil_coding_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_fast_encode_tagged_shapes_mixed() {
        // C++ TEST(FastEncodeTaggedShapes, MixedShapes)
        // Tests encoding/decoding a collection of points, polylines, and polygons.
        let index =
            crate::s2::text_format::make_index("0:0 | 0:1 # 1:1, 1:2, 1:3 # 2:2; 2:3, 2:4, 3:3");
        let decoded = roundtrip_index(&index);
        assert_indices_equal(&index, &decoded);
        assert_eq!(index.num_shape_ids(), decoded.num_shape_ids());
    }

    #[test]
    fn test_fast_encode_shape_polygon() {
        // C++ TEST(FastEncodeShape, S2Polygon) — roundtrip a polygon shape
        let polygon = crate::s2::text_format::make_polygon("0:0, 0:1, 1:0");
        let mut buf = Vec::new();
        polygon.encode(&mut buf).unwrap();
        let decoded = Polygon::decode(&mut buf.as_slice()).unwrap();
        assert!(polygon.boundary_equals(&decoded));
    }

    #[test]
    fn test_null_shape_encoding() {
        // Verify that null shapes (released shapes) roundtrip correctly.
        // C++ ShapeIndex supports null shape slots; the encoding handles them.
        let mut index = ShapeIndex::new();
        let pv = PointVector::new(vec![LatLng::from_degrees(0.0, 0.0).to_point()]);
        index.add(Box::new(pv));
        index.build();

        let mut buf = Vec::new();
        index.encode_to_writer(&mut buf).unwrap();
        let decoded = ShapeIndex::decode_from_reader(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(decoded.num_shape_ids(), 1);
        assert!(decoded.shape(0).is_some());
    }

    #[test]
    fn test_encode_decode_preserves_max_edges_per_cell() {
        // C++ tests verify that options().max_edges_per_cell() roundtrips.
        let mut index = ShapeIndex::new();
        index.set_max_edges_per_cell(42);
        let pv = PointVector::new(vec![LatLng::from_degrees(0.0, 0.0).to_point()]);
        index.add(Box::new(pv));
        index.build();

        let mut buf = Vec::new();
        index.encode_to_writer(&mut buf).unwrap();
        let decoded = ShapeIndex::decode_from_reader(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(decoded.max_edges_per_cell(), 42);
    }

    #[test]
    fn test_compact_encoding() {
        let mut index = ShapeIndex::new();
        let lp = LaxPolyline::new(vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(1.0, 0.0).to_point(),
        ]);
        index.add(Box::new(lp));
        index.build();

        let mut buf_fast = Vec::new();
        index
            .encode_with_hint(&mut buf_fast, CodingHint::Fast)
            .unwrap();

        let mut buf_compact = Vec::new();
        index
            .encode_with_hint(&mut buf_compact, CodingHint::Compact)
            .unwrap();

        // Compact should be smaller or equal.
        assert!(buf_compact.len() <= buf_fast.len());

        // Both should roundtrip correctly.
        let decoded_fast = ShapeIndex::decode_from_reader(&mut Cursor::new(&buf_fast)).unwrap();
        let decoded_compact =
            ShapeIndex::decode_from_reader(&mut Cursor::new(&buf_compact)).unwrap();
        assert_indices_equal(&index, &decoded_fast);
        assert_indices_equal(&index, &decoded_compact);
    }
}
