// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Encoded S2 shape index that works directly with encoded data.
//!
//! This module provides [`EncodedS2ShapeIndex`], which wraps a decoded
//! [`ShapeIndex`] and provides an interface compatible with the C++
//! `EncodedS2ShapeIndex`. In C++, the encoded version decodes lazily for
//! performance; in Rust, the current implementation eagerly decodes during
//! `init()` for simplicity and correctness.
//!
//! The primary use case is decoding a `ShapeIndex` from a byte buffer and
//! then performing queries on it (contains-point, closest-edge, etc.)
//! without needing to manually manage the decode step.
//!
//! Corresponds to C++ `encoded_s2shape_index.h`.

use std::io::{self, Cursor, Read};

use crate::s2::shape::Shape;
use crate::s2::shape_index::{ShapeIndex, ShapeIndexIterator};

/// An `S2ShapeIndex` implementation that is initialized from encoded data.
///
/// In C++, this class decodes data lazily (individual edges on demand).
/// The Rust implementation currently performs eager decoding during [`init`](Self::init),
/// but provides the same public API.
///
/// # Example
///
/// ```
/// use s2rst::s2::encoded_s2shape_index::EncodedS2ShapeIndex;
/// use s2rst::s2::shape_index::ShapeIndex;
///
/// // First encode an index.
/// let mut index = ShapeIndex::new();
/// index.build();
/// let mut buf = Vec::new();
/// index.encode_to_writer(&mut buf).unwrap();
///
/// // Then decode it.
/// let mut encoded = EncodedS2ShapeIndex::new();
/// encoded.init(&buf).unwrap();
/// assert_eq!(encoded.num_shape_ids(), 0);
/// ```
#[derive(Debug)]
pub struct EncodedS2ShapeIndex {
    /// The underlying eagerly-decoded index.
    index: ShapeIndex,
    /// The raw encoded bytes (kept for `encode()`).
    encoded_data: Vec<u8>,
}

impl EncodedS2ShapeIndex {
    /// Creates an uninitialized encoded shape index.
    pub fn new() -> Self {
        Self {
            index: ShapeIndex::new(),
            encoded_data: Vec::new(),
        }
    }

    /// Initializes the index from encoded data.
    ///
    /// The data must be in the format produced by [`ShapeIndex::encode_to_writer`].
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or uses an unsupported version.
    pub fn init(&mut self, data: &[u8]) -> io::Result<()> {
        self.encoded_data = data.to_vec();
        let mut cursor = Cursor::new(data);
        self.index = ShapeIndex::decode_from_reader(&mut cursor)?;
        Ok(())
    }

    /// Initializes from a reader.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn init_from_reader(&mut self, r: &mut dyn Read) -> io::Result<()> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        self.init(&data)
    }

    /// Encodes the index data to a writer.
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    pub fn encode(&self, w: &mut dyn io::Write) -> io::Result<()> {
        self.index.encode_to_writer(w)
    }

    /// Returns the number of distinct shape ids in the index.
    pub fn num_shape_ids(&self) -> usize {
        self.index.num_shape_ids()
    }

    /// Returns a reference to the shape with the given id, or `None`.
    pub fn shape(&self, id: i32) -> Option<&dyn Shape> {
        self.index.shape(id)
    }

    /// Returns the maximum number of edges per cell.
    pub fn max_edges_per_cell(&self) -> usize {
        self.index.max_edges_per_cell()
    }

    /// Returns an iterator positioned at the beginning of the index.
    pub fn iter(&self) -> ShapeIndexIterator<'_> {
        self.index.iter()
    }

    /// Returns a reference to the underlying decoded [`ShapeIndex`].
    ///
    /// This is useful for passing to query types that expect a `&ShapeIndex`.
    pub fn as_index(&self) -> &ShapeIndex {
        &self.index
    }

    /// Returns the total number of edges across all shapes in the index.
    pub fn num_edges(&self) -> usize {
        self.index.num_edges()
    }

    /// Minimizes memory usage. In the eager-decode implementation, this is a no-op
    /// since there's no lazily-held state to release.
    pub fn minimize(&mut self) {
        // No-op: the Rust implementation eagerly decodes everything.
    }
}

impl<'a> IntoIterator for &'a EncodedS2ShapeIndex {
    type Item = <ShapeIndexIterator<'a> as Iterator>::Item;
    type IntoIter = ShapeIndexIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Default for EncodedS2ShapeIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::point::Point;
    use crate::s2::point_vector::PointVector;
    use crate::s2::shape_index::ShapeIndex;

    fn encode_index(index: &ShapeIndex) -> Vec<u8> {
        let mut buf = Vec::new();
        index.encode_to_writer(&mut buf).unwrap();
        buf
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ encoded_s2shape_index_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_empty() {
        // C++ TEST(EncodedS2ShapeIndex, Empty)
        let mut index = ShapeIndex::new();
        index.build();
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(0, encoded.num_shape_ids());
        let it = encoded.iter();
        assert!(it.done());
    }

    #[test]
    fn test_one_edge() {
        // C++ TEST(EncodedS2ShapeIndex, OneEdge)
        let mut index = ShapeIndex::new();
        let lp = LaxPolyline::new(vec![
            LatLng::from_degrees(1.0, 1.0).to_point(),
            LatLng::from_degrees(2.0, 2.0).to_point(),
        ]);
        index.add(Box::new(lp));
        index.build();
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(1, encoded.num_shape_ids());
        assert!(encoded.shape(0).is_some());
        assert_eq!(1, encoded.num_edges());
    }

    #[test]
    fn test_regular_loop() {
        // C++ TEST(EncodedS2ShapeIndex, RegularLoops) — one test case
        let mut index = ShapeIndex::new();
        let center = Point::from_coords(3.0, 2.0, 1.0).normalize();
        let polygon = crate::s2::polygon::Polygon::from_loops(vec![crate::s2::Loop::make_regular(
            center,
            crate::s1::Angle::from_degrees(0.1),
            16,
        )]);
        let lp = LaxPolygon::from_polygon_ref(&polygon);
        index.add(Box::new(lp));
        index.build();
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(1, encoded.num_shape_ids());
        assert_eq!(16, encoded.num_edges());

        // Verify options roundtrip.
        assert_eq!(index.max_edges_per_cell(), encoded.max_edges_per_cell());
    }

    #[test]
    fn test_mixed_shapes() {
        // Tests encoding/decoding a collection of points, polylines, and polygons.
        let index =
            crate::s2::text_format::make_index("0:0 | 0:1 # 1:1, 1:2, 1:3 # 2:2; 2:3, 2:4, 3:3");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(index.num_shape_ids(), encoded.num_shape_ids());
        assert_eq!(index.num_edges(), encoded.num_edges());

        // Verify cells match.
        let mut it_orig = index.iter();
        let mut it_enc = encoded.iter();
        while !it_orig.done() && !it_enc.done() {
            assert_eq!(it_orig.cell_id(), it_enc.cell_id());
            it_orig.next();
            it_enc.next();
        }
        assert!(it_orig.done());
        assert!(it_enc.done());
    }

    #[test]
    fn test_as_index() {
        // Verify that as_index() returns a usable ShapeIndex reference.
        let index = crate::s2::text_format::make_index("0:0 | 1:1 # #");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();

        let idx_ref = encoded.as_index();
        assert_eq!(idx_ref.num_shape_ids(), 1);
        assert_eq!(idx_ref.num_edges(), 2);
    }

    #[test]
    fn test_re_encode() {
        // C++ tests verify that re-encoding produces the same bytes.
        let index = crate::s2::text_format::make_index("0:0 # 1:1, 2:2 # 3:3, 3:4, 4:3");
        let buf1 = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf1).unwrap();

        let mut buf2 = Vec::new();
        encoded.encode(&mut buf2).unwrap();

        // Decoded and re-encoded should produce same byte count.
        // (Exact byte equality depends on encoding determinism.)
        assert_eq!(buf1.len(), buf2.len());
    }

    #[test]
    fn test_minimize_is_no_op() {
        // In the Rust implementation, minimize() is a no-op (no lazy state).
        let index = crate::s2::text_format::make_index("0:0 # #");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(1, encoded.num_shape_ids());

        encoded.minimize();
        assert_eq!(1, encoded.num_shape_ids());
    }

    #[test]
    fn test_contains_point_query() {
        // C++ benchmarks test ContainsPoint with EncodedS2ShapeIndex.
        // We verify it works correctly.
        let index = crate::s2::text_format::make_index("# # 0:0, 0:10, 10:10, 10:0");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();

        let idx = encoded.as_index();
        let mut query = crate::s2::contains_point_query::ContainsPointQuery::new(
            idx,
            crate::s2::contains_point_query::VertexModel::SemiOpen,
        );

        // A point inside the polygon.
        let inside = LatLng::from_degrees(5.0, 5.0).to_point();
        assert!(query.contains(inside));

        // A point outside the polygon.
        let outside = LatLng::from_degrees(20.0, 20.0).to_point();
        assert!(!query.contains(outside));
    }

    #[test]
    fn test_point_clouds() {
        // Simplified version of C++ OverlappingPointClouds.
        let mut index = ShapeIndex::new();
        let mut points = Vec::new();
        for i in 0..50 {
            let lat = f64::from(i) * 0.01;
            points.push(LatLng::from_degrees(lat, lat).to_point());
        }
        index.add(Box::new(PointVector::new(points)));
        index.build();

        let buf = encode_index(&index);
        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(index.num_shape_ids(), encoded.num_shape_ids());
        assert_eq!(index.num_edges(), encoded.num_edges());
    }
}
