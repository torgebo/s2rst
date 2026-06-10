// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A lightweight open polyline shape.
//!
//! [`LaxPolyline`] is similar to [`Polyline`](super::polyline::Polyline)
//! except that adjacent vertices are allowed to be identical or antipodal,
//! and the representation is slightly more compact.
//!
//! Corresponds to C++ `s2lax_polyline_shape.h`, Go `s2/lax_polyline.go`.

use crate::s2::Point;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};

/// An open polyline represented as a [`Shape`] (dimension 1).
///
/// Unlike [`Polyline`](super::polyline::Polyline), this type allows
/// identical or antipodal adjacent vertices. Polylines with fewer than
/// 2 vertices have no edges.
///
/// # Examples
///
/// ```
/// use s2rst::s2::lax_polyline::LaxPolyline;
/// use s2rst::s2::shape::{Dimension, Shape};
/// use s2rst::s2::LatLng;
///
/// let line = LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
///     LatLng::from_degrees(2.0, 0.0).to_point(),
/// ]);
/// assert_eq!(line.num_edges(), 2);
/// assert_eq!(line.dimension(), Dimension::Polyline);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LaxPolyline {
    vertices: Vec<Point>,
}

impl LaxPolyline {
    /// Creates a new `LaxPolyline` from the given vertices.
    pub fn new(vertices: Vec<Point>) -> Self {
        LaxPolyline { vertices }
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the vertex at the given index.
    pub fn vertex(&self, i: usize) -> Point {
        self.vertices[i]
    }

    /// Returns a slice of all vertices.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }
}

impl Shape for LaxPolyline {
    fn num_edges(&self) -> usize {
        if self.vertices.len() < 2 {
            0
        } else {
            self.vertices.len() - 1
        }
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        Edge::new(self.vertices[id], self.vertices[id + 1])
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), false)
    }

    fn num_chains(&self) -> usize {
        if self.num_edges() > 0 { 1 } else { 0 }
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.num_edges())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        Edge::new(self.vertices[offset], self.vertices[offset + 1])
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }

    fn type_tag(&self) -> u32 {
        4 // S2LaxPolylineShape::kTypeTag
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        self.encode_with_hint(w, hint)
    }
}

impl std::ops::Deref for LaxPolyline {
    type Target = [Point];
    fn deref(&self) -> &[Point] {
        &self.vertices
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

// ─── Encoded (lazy) variant ─────────────────────────────────────────────

/// A lax polyline served directly from its encoded bytes.
///
/// Port of C++ `EncodedS2LaxPolylineShape` (Phase 3): the vertex vector
/// stays encoded; each access decodes one vertex through
/// [`EncodedS2PointVector`](crate::s2::encoded_s2point_vector::EncodedS2PointVector).
/// Owns its buffer (`Arc<[u8]>`), so it is a `'static` [`Shape`] and can be
/// stored in any index.
///
/// The structure (headers, lengths) is validated by [`Self::init`];
/// `CELL_IDS` block payloads are validated lazily; a malformed payload
/// yields arbitrary points (never panics). Untrusted bytes should be
/// fully decoded once via the error-returning vector API instead.
#[derive(Clone, Debug)]
pub struct EncodedLaxPolyline {
    data: std::sync::Arc<[u8]>,
    start: usize,
    end: usize,
    num_vertices: usize,
}

impl EncodedLaxPolyline {
    /// Wraps the encoded vertex vector starting at `offset` within `data`.
    /// Returns the shape and the number of bytes it occupies.
    ///
    /// # Errors
    /// Returns `InvalidData`/`UnexpectedEof` if the structure is malformed
    /// or truncated.
    pub fn init(data: std::sync::Arc<[u8]>, offset: usize) -> std::io::Result<(Self, usize)> {
        let slice = data.get(offset..).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "offset out of bounds")
        })?;
        let (view, rest) = crate::s2::encoded_s2point_vector::EncodedS2PointVector::init(slice)?;
        let consumed = slice.len() - rest.len();
        let num_vertices = view.len();
        Ok((
            EncodedLaxPolyline {
                data,
                start: offset,
                end: offset + consumed,
                num_vertices,
            },
            consumed,
        ))
    }

    fn view(&self) -> crate::s2::encoded_s2point_vector::EncodedS2PointVector<'_> {
        #[expect(
            clippy::expect_used,
            reason = "structure was validated by init(); re-parsing cannot fail"
        )]
        crate::s2::encoded_s2point_vector::EncodedS2PointVector::init(
            &self.data[self.start..self.end],
        )
        .expect("validated by init")
        .0
    }

    /// The number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.num_vertices
    }

    /// Decodes vertex `i`.
    ///
    /// # Panics
    /// Panics if `i >= num_vertices()`.
    pub fn vertex(&self, i: usize) -> Point {
        // Malformed lazily-validated payloads degrade to an arbitrary point
        // (matching C++'s non-validating decoders) so index queries and
        // fuzzing never panic; validate untrusted bytes eagerly instead.
        self.view().get(i).unwrap_or_default()
    }
}

impl Shape for EncodedLaxPolyline {
    fn num_edges(&self) -> usize {
        self.num_vertices.saturating_sub(1)
    }

    fn edge(&self, id: usize) -> Edge {
        debug_assert!(id < self.num_edges());
        Edge::new(self.vertex(id), self.vertex(id + 1))
    }

    fn reference_point(&self) -> ReferencePoint {
        ReferencePoint::new(Point::origin(), false)
    }

    fn num_chains(&self) -> usize {
        usize::from(self.num_edges() > 0)
    }

    fn chain(&self, _chain_id: usize) -> Chain {
        Chain::new(0, self.num_edges())
    }

    fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
        debug_assert!(offset < self.num_edges());
        Edge::new(self.vertex(offset), self.vertex(offset + 1))
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        debug_assert!(edge_id < self.num_edges());
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }

    fn type_tag(&self) -> u32 {
        4 // Same wire identity as S2LaxPolylineShape::kTypeTag.
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        _hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        // Already encoded: re-emit the original bytes regardless of hint.
        crate::s2::encoding::write_uvarint(w, u64::from(self.type_tag()))?;
        w.write_all(&self.data[self.start..self.end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoded_lax_polyline_matches_decoded() {
        use crate::s2::text_format::parse_points;
        for hint in [
            crate::s2::encoded_s2point_vector::CodingHint::Fast,
            crate::s2::encoded_s2point_vector::CodingHint::Compact,
        ] {
            let vertices = parse_points("0:0, 0:5, 5:5, 5:10, 10:10");
            let plain = LaxPolyline::new(vertices.clone());
            let mut buf = Vec::new();
            plain.encode_with_hint(&mut buf, hint).unwrap();
            buf.extend_from_slice(b"@@"); // trailing data must be left alone
            let data: std::sync::Arc<[u8]> = buf.into();
            let (encoded, consumed) =
                EncodedLaxPolyline::init(std::sync::Arc::clone(&data), 0).unwrap();
            assert_eq!(consumed, data.len() - 2, "hint {hint:?}");

            assert_eq!(encoded.num_vertices(), plain.num_vertices());
            assert_eq!(encoded.num_edges(), plain.num_edges());
            assert_eq!(encoded.num_chains(), plain.num_chains());
            assert_eq!(encoded.dimension(), plain.dimension());
            assert_eq!(encoded.type_tag(), plain.type_tag());
            for i in 0..plain.num_vertices() {
                assert_eq!(encoded.vertex(i), plain.vertex(i), "vertex {i}");
            }
            for e in 0..plain.num_edges() {
                assert_eq!(encoded.edge(e), plain.edge(e), "edge {e}");
                assert_eq!(encoded.chain_position(e), plain.chain_position(e));
            }
        }
    }

    #[test]
    fn test_encoded_lax_polyline_served_by_queries() {
        // The Phase 3 payoff: an encoded shape, never materialized, indexed
        // and queried end-to-end.
        use crate::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
        use crate::s2::shape_index::ShapeIndex;
        use crate::s2::text_format::{parse_point, parse_points};

        let plain = LaxPolyline::new(parse_points("0:0, 0:5, 5:5"));
        let mut buf = Vec::new();
        plain
            .encode_with_hint(
                &mut buf,
                crate::s2::encoded_s2point_vector::CodingHint::Compact,
            )
            .unwrap();
        let data: std::sync::Arc<[u8]> = buf.into();
        let (encoded, _) = EncodedLaxPolyline::init(data, 0).unwrap();

        let mut index = ShapeIndex::new();
        index.add(Box::new(encoded));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(parse_point("1:0"));
        let result = query.find_closest_edge(&target);
        assert!(!result.is_empty());
        assert!(result.distance.to_angle().degrees() < 1.1);
    }

    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn lax_polyline_is_send_sync() {
        is_send_sync::<LaxPolyline>();
    }

    #[test]
    fn test_empty() {
        let l = LaxPolyline::new(vec![]);
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
        assert_eq!(l.dimension(), Dimension::Polyline);
        assert!(l.is_empty());
        assert!(!l.is_full());
        assert!(!l.has_interior());
    }

    #[test]
    fn test_single_vertex() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0)]);
        assert_eq!(l.num_edges(), 0);
        assert_eq!(l.num_chains(), 0);
    }

    #[test]
    fn test_two_vertices() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        assert_eq!(l.num_edges(), 1);
        assert_eq!(l.num_chains(), 1);
        let e = l.edge(0);
        assert_eq!(e.v0, l.vertex(0));
        assert_eq!(e.v1, l.vertex(1));
    }

    #[test]
    fn test_three_vertices() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        assert_eq!(l.num_edges(), 2);
        assert_eq!(l.num_chains(), 1);
        let chain = l.chain(0);
        assert_eq!(chain.start, 0);
        assert_eq!(chain.length, 2);
    }

    #[test]
    fn test_chain_position() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        for i in 0..2 {
            let cp = l.chain_position(i);
            assert_eq!(cp.chain_id, 0);
            assert_eq!(cp.offset, i);
        }
    }

    #[test]
    fn test_reference_point_not_contained() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let rp = l.reference_point();
        assert!(!rp.contained);
    }

    #[test]
    fn test_deref() {
        let l = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let slice: &[Point] = &l;
        assert_eq!(slice.len(), 2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let lp = LaxPolyline::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        ]);
        let json = serde_json::to_string(&lp).unwrap();
        let back: LaxPolyline = serde_json::from_str(&json).unwrap();
        assert_eq!(lp.num_vertices(), back.num_vertices());
        for i in 0..lp.num_vertices() {
            assert_eq!(lp.vertex(i), back.vertex(i));
        }
    }
}
