// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A [`ShapeIndex`](crate::s2::shape_index::ShapeIndex)-compatible index
//! served directly from encoded bytes.
//!
//! Corresponds to C++ `encoded_s2shape_index.h`, including its lazy design:
//! `init` parses only the three section headers (tagged shapes, cell ids,
//! cell contents — all lazy views); shapes and cells are decoded on first
//! access and cached in per-slot [`OnceLock`]s, so concurrent readers are
//! safe and initialization cost is independent of the dataset size. Lax
//! polylines/polygons stay encoded even after access: the shape table hands
//! out [`EncodedLaxPolyline`](crate::s2::lax_polyline::EncodedLaxPolyline) /
//! [`EncodedLaxPolygon`](crate::s2::lax_polygon::EncodedLaxPolygon) views
//! over the same buffer.
//!
//! Malformed *lazily-decoded* payloads (a corrupt shape or cell record in a
//! structurally valid index) degrade gracefully: the slot reads as absent
//! (`shape()` returns `None`, the cell has no contents) and queries treat it
//! like removed geometry — never a panic. Validate untrusted bytes with
//! [`ShapeIndex::decode_from_reader`](crate::s2::shape_index::ShapeIndex::decode_from_reader)
//! when full validation is required.

use std::io::{self, Read};
use std::sync::{Arc, OnceLock};

use crate::s2::cell_id::CellId;
use crate::s2::cell_iterator::CellIterator;
use crate::s2::encoded_s2cell_id_vector::EncodedS2CellIdVector;
use crate::s2::encoded_string_vector::EncodedStringVector;
use crate::s2::encoding::read_uvarint;
use crate::s2::shape::{Shape, ShapeId};
use crate::s2::shape_index::{IndexCellAccess, ShapeIndexCell, ShapeIndexLike};
use crate::s2::shape_index_encoding::{decode_cell, decode_tagged_shape_lazy};

/// Wire-format version (must match `ShapeIndex::encode_with_hint`).
const SHAPE_INDEX_VERSION: u64 = 2;

type ShapeSlot = OnceLock<Option<Box<dyn Shape>>>;
type CellSlot = OnceLock<Option<ShapeIndexCell>>;

/// An `S2ShapeIndex` served lazily from encoded bytes.
///
/// See the [module docs](self) for the laziness and error-degradation
/// policies. Implements [`ShapeIndexLike`], so every generic query
/// (`ContainsPointQuery`, `ClosestEdgeQuery`, joins, ...) runs on it
/// unchanged.
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
pub struct EncodedS2ShapeIndex {
    data: Arc<[u8]>,
    max_edges_per_cell: usize,
    /// Absolute start of the shape-strings data block within `data`.
    shapes_data_start: usize,
    /// Byte range of the encoded cell-id vector.
    cell_ids_start: usize,
    cell_ids_end: usize,
    /// Byte range of the encoded cell-contents string vector.
    cells_start: usize,
    cells_end: usize,
    /// Byte range of the shape string vector (header + data).
    shapes_start: usize,
    shapes_end: usize,
    /// Lazily decoded shapes / cells, one slot per entry.
    shape_cache: Vec<ShapeSlot>,
    cell_cache: Vec<CellSlot>,
}

impl std::fmt::Debug for EncodedS2ShapeIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncodedS2ShapeIndex")
            .field("num_shape_ids", &self.shape_cache.len())
            .field("num_cells", &self.cell_cache.len())
            .finish_non_exhaustive()
    }
}

impl EncodedS2ShapeIndex {
    /// Creates an uninitialized encoded shape index.
    pub fn new() -> Self {
        EncodedS2ShapeIndex {
            data: Arc::from(Vec::new()),
            max_edges_per_cell: 0,
            shapes_data_start: 0,
            cell_ids_start: 0,
            cell_ids_end: 0,
            cells_start: 0,
            cells_end: 0,
            shapes_start: 0,
            shapes_end: 0,
            shape_cache: Vec::new(),
            cell_cache: Vec::new(),
        }
    }

    /// Initializes the index from encoded data.
    ///
    /// The data must be in the format produced by
    /// [`ShapeIndex::encode_to_writer`](crate::s2::shape_index::ShapeIndex::encode_to_writer).
    /// Only the section headers are parsed;
    /// cost is independent of the number of shapes, cells or edges.
    ///
    /// # Errors
    ///
    /// Returns an error if the section structure is malformed or uses an
    /// unsupported version.
    pub fn init(&mut self, data: &[u8]) -> io::Result<()> {
        let data: Arc<[u8]> = data.into();
        let total = data.len();
        let mut cursor: &[u8] = &data;

        // 1. Version + options.
        let max_edges_version = read_uvarint(&mut cursor)?;
        let version = max_edges_version & 3;
        if version != SHAPE_INDEX_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported ShapeIndex version {version}"),
            ));
        }
        let max_edges_per_cell = (max_edges_version >> 2) as usize;

        // 2. Tagged shapes (lazy string vector).
        let shapes_start = total - cursor.len();
        let (shapes, rest) = EncodedStringVector::init(cursor)?;
        let shapes_end = total - rest.len();
        let shapes_data_start = shapes_end - shapes.data_len();
        let num_shapes = shapes.len();
        cursor = rest;

        // 3. Cell ids (lazy vector).
        let cell_ids_start = total - cursor.len();
        let (cell_ids, rest) = EncodedS2CellIdVector::init(cursor)?;
        let cell_ids_end = total - rest.len();
        cursor = rest;

        // 4. Cell contents (lazy string vector).
        let cells_start = total - cursor.len();
        let (cells, rest) = EncodedStringVector::init(cursor)?;
        let cells_end = total - rest.len();
        if cells.len() != cell_ids.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cell contents count does not match cell id count",
            ));
        }
        let num_cells = cells.len();

        // Validate the cell ids up front (allocation-free scan). Queries
        // rely on validity and strict ordering: `seek` binary-searches, and
        // `range_min`/`center` assume each id is a valid cell (id 0 would
        // underflow `lsb() - 1`). Same checks as the eager decoder.
        let mut prev: Option<CellId> = None;
        for i in 0..cell_ids.len() {
            let cid = cell_ids.get(i);
            if !cid.is_valid() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "shape index contains an invalid cell id",
                ));
            }
            if prev.is_some_and(|p| cid <= p) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "shape index cell ids must be strictly increasing",
                ));
            }
            prev = Some(cid);
        }

        self.data = data;
        self.max_edges_per_cell = max_edges_per_cell;
        self.shapes_data_start = shapes_data_start;
        self.cell_ids_start = cell_ids_start;
        self.cell_ids_end = cell_ids_end;
        self.cells_start = cells_start;
        self.cells_end = cells_end;
        self.shapes_start = shapes_start;
        self.shapes_end = shapes_end;
        self.shape_cache = std::iter::repeat_with(OnceLock::new)
            .take(num_shapes)
            .collect();
        self.cell_cache = std::iter::repeat_with(OnceLock::new)
            .take(num_cells)
            .collect();
        Ok(())
    }

    /// Initializes from a reader (reads to end first).
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed.
    pub fn init_from_reader(&mut self, r: &mut dyn Read) -> io::Result<()> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        self.init(&data)
    }

    fn ids_view(&self) -> EncodedS2CellIdVector<'_> {
        #[expect(
            clippy::expect_used,
            reason = "structure was validated by init(); re-parsing cannot fail"
        )]
        EncodedS2CellIdVector::init(&self.data[self.cell_ids_start..self.cell_ids_end])
            .expect("validated by init")
            .0
    }

    fn shapes_view(&self) -> EncodedStringVector<'_> {
        #[expect(
            clippy::expect_used,
            reason = "structure was validated by init(); re-parsing cannot fail"
        )]
        EncodedStringVector::init(&self.data[self.shapes_start..self.shapes_end])
            .expect("validated by init")
            .0
    }

    fn cells_view(&self) -> EncodedStringVector<'_> {
        #[expect(
            clippy::expect_used,
            reason = "structure was validated by init(); re-parsing cannot fail"
        )]
        EncodedStringVector::init(&self.data[self.cells_start..self.cells_end])
            .expect("validated by init")
            .0
    }

    /// The lazily-decoded contents of cell position `pos`, or `None` if the
    /// stored record is malformed.
    fn cell_contents(&self, pos: usize) -> Option<&ShapeIndexCell> {
        self.cell_cache[pos]
            .get_or_init(|| {
                let view = self.cells_view();
                let bytes = view.get(pos).ok()?;
                let cell = decode_cell(bytes, self.num_shape_ids()).ok()?;
                // Cross-validate the record against the shapes it references
                // (the eager decoder does the same after decoding everything;
                // here the referenced shapes decode lazily and are cached).
                // A record naming a missing shape or an out-of-range edge id
                // is malformed: treat the cell as absent.
                for clipped in &cell.shapes {
                    let shape = self.shape(clipped.shape_id.0)?;
                    let num_edges = shape.num_edges();
                    if clipped
                        .edges
                        .iter()
                        .any(|&e| usize::try_from(e).is_ok_and(|e| e >= num_edges) || e < 0)
                    {
                        return None;
                    }
                }
                Some(cell)
            })
            .as_ref()
    }

    /// Encodes the index data to a writer (re-emits the original bytes).
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    pub fn encode(&self, w: &mut dyn io::Write) -> io::Result<()> {
        w.write_all(&self.data)
    }

    /// Returns the number of distinct shape ids in the index.
    pub fn num_shape_ids(&self) -> usize {
        self.shape_cache.len()
    }

    /// Returns the shape with the given id, decoding it on first access.
    /// Lax polylines/polygons are returned as their lazy encoded variants.
    pub fn shape(&self, id: i32) -> Option<&dyn Shape> {
        let i = usize::try_from(id).ok()?;
        if i >= self.shape_cache.len() {
            return None;
        }
        self.shape_cache[i]
            .get_or_init(|| {
                let view = self.shapes_view();
                let (start, end) = view.range(i).ok()?;
                if start == end {
                    return None; // Removed shape (gap id).
                }
                decode_tagged_shape_lazy(
                    &self.data,
                    self.shapes_data_start + start,
                    self.shapes_data_start + end,
                )
                .ok()
            })
            .as_deref()
    }

    /// Returns the maximum number of edges per cell (an encoding option).
    pub fn max_edges_per_cell(&self) -> usize {
        self.max_edges_per_cell
    }

    /// Returns an iterator positioned at the beginning of the index.
    pub fn iter(&self) -> EncodedShapeIndexIterator<'_> {
        EncodedShapeIndexIterator {
            index: self,
            ids: self.ids_view(),
            pos: 0,
        }
    }

    /// Returns the total number of edges across all shapes.
    ///
    /// Note: this decodes every shape (the count is not stored separately).
    pub fn num_edges(&self) -> usize {
        (0..self.num_shape_ids())
            .filter_map(|i| self.shape(i32::try_from(i).ok()?))
            .map(Shape::num_edges)
            .sum()
    }

    /// Releases all lazily-decoded shapes and cells; they will be decoded
    /// again on next access. (C++ `Minimize`.)
    pub fn minimize(&mut self) {
        for slot in &mut self.shape_cache {
            *slot = OnceLock::new();
        }
        for slot in &mut self.cell_cache {
            *slot = OnceLock::new();
        }
    }
}

impl Default for EncodedS2ShapeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeIndexLike for EncodedS2ShapeIndex {
    type Iter<'a> = EncodedShapeIndexIterator<'a>;

    fn num_shape_ids(&self) -> usize {
        self.num_shape_ids()
    }
    fn shape(&self, id: ShapeId) -> Option<&dyn Shape> {
        self.shape(id.0)
    }
    fn iter(&self) -> EncodedShapeIndexIterator<'_> {
        self.iter()
    }
}

/// A cell iterator over an [`EncodedS2ShapeIndex`]; cell ids come straight
/// from the lazy id vector, contents decode on first access.
#[derive(Clone, Debug)]
pub struct EncodedShapeIndexIterator<'a> {
    index: &'a EncodedS2ShapeIndex,
    ids: EncodedS2CellIdVector<'a>,
    pos: usize,
}

impl<'a> EncodedShapeIndexIterator<'a> {
    /// The current cell id. Must not be called when [`Self::done`].
    pub fn cell_id(&self) -> CellId {
        self.ids.get(self.pos)
    }

    /// True if the iterator is past the last cell.
    pub fn done(&self) -> bool {
        self.pos >= self.ids.len()
    }

    /// Advances to the next cell.
    pub fn next(&mut self) {
        self.pos += 1;
    }

    /// The center point of the current cell.
    pub fn center(&self) -> crate::s2::point::Point {
        self.cell_id().to_point()
    }

    /// The contents of the current cell, or `None` if absent/malformed.
    pub fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        if self.done() {
            return None;
        }
        self.index.cell_contents(self.pos)
    }
}

impl CellIterator for EncodedShapeIndexIterator<'_> {
    fn cell_id(&self) -> CellId {
        self.ids.get(self.pos)
    }
    fn done(&self) -> bool {
        self.pos >= self.ids.len()
    }
    fn begin(&mut self) {
        self.pos = 0;
    }
    fn finish(&mut self) {
        self.pos = self.ids.len();
    }
    fn next(&mut self) {
        self.pos += 1;
    }
    fn prev(&mut self) -> bool {
        if self.pos == 0 {
            return false;
        }
        self.pos -= 1;
        true
    }
    fn seek(&mut self, target: CellId) {
        self.pos = self.ids.lower_bound(target);
    }
}

impl<'a> IndexCellAccess<'a> for EncodedShapeIndexIterator<'a> {
    fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        if self.done() {
            return None;
        }
        self.index.cell_contents(self.pos)
    }
}

impl<'a> IntoIterator for &'a EncodedS2ShapeIndex {
    type Item = (CellId, &'a ShapeIndexCell);
    type IntoIter = EncodedIndexCellsIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        EncodedIndexCellsIter { it: self.iter() }
    }
}

/// `Iterator` adapter over `(cell_id, contents)` pairs, skipping cells whose
/// stored record is malformed.
#[derive(Debug)]
pub struct EncodedIndexCellsIter<'a> {
    it: EncodedShapeIndexIterator<'a>,
}

impl<'a> Iterator for EncodedIndexCellsIter<'a> {
    type Item = (CellId, &'a ShapeIndexCell);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.it.done() {
            let id = self.it.cell_id();
            let cell = self.it.index_cell();
            self.it.next();
            if let Some(cell) = cell {
                return Some((id, cell));
            }
        }
        None
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

        // Verify cells AND contents match.
        let mut it_orig = index.iter();
        let mut it_enc = encoded.iter();
        while !it_orig.done() && !it_enc.done() {
            assert_eq!(it_orig.cell_id(), CellIterator::cell_id(&it_enc));
            let orig_cell = it_orig.index_cell().unwrap();
            let enc_cell = it_enc.index_cell().unwrap();
            assert_eq!(orig_cell.shapes.len(), enc_cell.shapes.len());
            it_orig.next();
            CellIterator::next(&mut it_enc);
        }
        assert!(it_orig.done());
        assert!(it_enc.done());
    }

    #[test]
    fn test_lazy_shapes_stay_encoded() {
        // Lax shapes come back as their lazy encoded variants (type tags
        // are preserved; the vertex data stays in the shared buffer).
        let index = crate::s2::text_format::make_index("# 1:1, 1:2 # 2:2, 2:3, 3:3");
        let buf = encode_index(&index);
        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        let polyline = encoded.shape(0).unwrap();
        assert_eq!(polyline.type_tag(), 4);
        assert_eq!(polyline.num_edges(), 1);
        let polygon = encoded.shape(1).unwrap();
        assert_eq!(polygon.type_tag(), 5);
        assert_eq!(polygon.num_edges(), 3);
    }

    #[test]
    fn test_re_encode_is_byte_identical() {
        let index = crate::s2::text_format::make_index("0:0 # 1:1, 2:2 # 3:3, 3:4, 4:3");
        let buf1 = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf1).unwrap();

        let mut buf2 = Vec::new();
        encoded.encode(&mut buf2).unwrap();
        assert_eq!(buf1, buf2, "encode() re-emits the original bytes");
    }

    #[test]
    fn test_minimize_releases_and_redecodes() {
        let index = crate::s2::text_format::make_index("0:0 # #");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();
        assert_eq!(1, encoded.num_shape_ids());
        assert!(encoded.shape(0).is_some());

        encoded.minimize();
        assert_eq!(1, encoded.num_shape_ids());
        assert!(encoded.shape(0).is_some(), "decodes again after minimize");
    }

    #[test]
    fn test_queries_run_directly_on_lazy_index() {
        // The H3 payoff: generic queries served straight from encoded bytes.
        let index = crate::s2::text_format::make_index("# # 0:0, 0:10, 10:10, 10:0");
        let buf = encode_index(&index);

        let mut encoded = EncodedS2ShapeIndex::new();
        encoded.init(&buf).unwrap();

        let mut query = crate::s2::contains_point_query::ContainsPointQuery::new(
            &encoded,
            crate::s2::contains_point_query::VertexModel::SemiOpen,
        );
        assert!(query.contains(LatLng::from_degrees(5.0, 5.0).to_point()));
        assert!(!query.contains(LatLng::from_degrees(20.0, 20.0).to_point()));

        let edge_query = crate::s2::closest_edge_query::ClosestEdgeQuery::new(&encoded);
        let target = crate::s2::closest_edge_query::PointTarget::new(
            LatLng::from_degrees(5.0, 11.0).to_point(),
        );
        let result = edge_query.find_closest_edge(&target);
        assert!(!result.is_empty());
        assert!(result.distance.to_angle().degrees() < 1.1);
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

    #[test]
    fn test_truncation_never_panics() {
        let index = crate::s2::text_format::make_index("0:0 # 1:1, 2:2 # 3:3, 3:4, 4:3");
        let buf = encode_index(&index);
        for cut in 0..buf.len() {
            let mut encoded = EncodedS2ShapeIndex::new();
            if encoded.init(&buf[..cut]).is_ok() {
                // Structure parsed: exercise all lazy paths.
                for i in 0..encoded.num_shape_ids() {
                    let _ = encoded.shape(i32::try_from(i).unwrap());
                }
                for (_, cell) in &encoded {
                    let _ = cell.shapes.len();
                }
            }
        }
    }
}
