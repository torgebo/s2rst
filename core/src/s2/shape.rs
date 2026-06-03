// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! The [`Shape`] trait and supporting types for representing geometry in an
//! [`ShapeIndex`](super::shape_index).
//!
//! All geometry is represented as a collection of edges that optionally
//! define an interior. An `S2Shape` can represent a set of points, a set
//! of polylines, or a set of polygons — but all edges must have the same
//! *dimension* (0, 1, or 2).
//!
//! Corresponds to C++ `s2shape.h`, Go `s2/shape.go`.

use crate::s2::Point;

// ─── Dimension ──────────────────────────────────────────────────────────

/// The geometric dimension of a shape.
///
/// Every [`Shape`] has a fixed dimension that determines the type of
/// geometry it represents:
///
/// - [`Point`](Dimension::Point) (0): a collection of points.
/// - [`Polyline`](Dimension::Polyline) (1): a collection of open polylines.
/// - [`Polygon`](Dimension::Polygon) (2): a collection of closed polygons
///   with interior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum Dimension {
    /// Point geometry (0-dimensional).
    Point = 0,
    /// Polyline geometry (1-dimensional).
    Polyline = 1,
    /// Polygon geometry (2-dimensional).
    Polygon = 2,
}

impl Dimension {
    /// Returns the dimension as a `usize`, suitable for use as an array index.
    pub const fn as_usize(self) -> usize {
        self as usize
    }
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dimension::Point => write!(f, "0"),
            Dimension::Polyline => write!(f, "1"),
            Dimension::Polygon => write!(f, "2"),
        }
    }
}

impl From<Dimension> for u8 {
    fn from(d: Dimension) -> u8 {
        d as u8
    }
}

impl From<Dimension> for i8 {
    fn from(d: Dimension) -> i8 {
        d as i8
    }
}

impl From<Dimension> for i32 {
    fn from(d: Dimension) -> i32 {
        d as i32
    }
}

impl From<Dimension> for usize {
    fn from(d: Dimension) -> usize {
        d as usize
    }
}

impl TryFrom<u8> for Dimension {
    type Error = &'static str;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Dimension::Point),
            1 => Ok(Dimension::Polyline),
            2 => Ok(Dimension::Polygon),
            _ => Err("dimension must be 0, 1, or 2"),
        }
    }
}

impl TryFrom<i8> for Dimension {
    type Error = &'static str;
    fn try_from(v: i8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Dimension::Point),
            1 => Ok(Dimension::Polyline),
            2 => Ok(Dimension::Polygon),
            _ => Err("dimension must be 0, 1, or 2"),
        }
    }
}

impl TryFrom<i32> for Dimension {
    type Error = &'static str;
    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Dimension::Point),
            1 => Ok(Dimension::Polyline),
            2 => Ok(Dimension::Polygon),
            _ => Err("dimension must be 0, 1, or 2"),
        }
    }
}

// ─── Supporting types ───────────────────────────────────────────────────

/// A geodesic edge between two points on the sphere.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Edge {
    /// The first vertex of the edge.
    pub v0: Point,
    /// The second vertex of the edge.
    pub v1: Point,
}

impl Edge {
    /// Creates a new edge from two points.
    pub fn new(v0: Point, v1: Point) -> Self {
        Edge { v0, v1 }
    }

    /// Returns the reversed edge (v1 → v0).
    pub fn reversed(self) -> Edge {
        Edge {
            v0: self.v1,
            v1: self.v0,
        }
    }

    /// Reports whether this edge is degenerate (v0 == v1).
    pub fn is_degenerate(self) -> bool {
        self.v0 == self.v1
    }

    /// Lexicographic comparison: first by v0, then by v1.
    #[expect(
        clippy::should_implement_trait,
        reason = "named constructor avoids ambiguity with std traits"
    )]
    pub fn cmp(&self, other: &Edge) -> std::cmp::Ordering {
        let c = self.v0.cmp_point(other.v0);
        if c != std::cmp::Ordering::Equal {
            return c;
        }
        self.v1.cmp_point(other.v1)
    }
}

// ─── ShapeId ────────────────────────────────────────────────────────────

/// Index of a shape within a [`ShapeIndex`](super::shape_index::ShapeIndex).
///
/// Wraps an `i32`. Negative values (typically `-1`) serve as sentinel
/// "no shape" markers — prefer using `Option<ShapeId>` at API boundaries
/// where possible.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeId(pub i32);

impl ShapeId {
    /// Creates a new `ShapeId`.
    pub const fn new(v: i32) -> Self {
        ShapeId(v)
    }

    /// Returns the raw `i32` value.
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Returns the value as a `usize` for array indexing.
    ///
    /// # Panics
    ///
    /// Panics if the value is negative.
    #[expect(clippy::cast_sign_loss, reason = "guarded by assert")]
    pub const fn as_usize(self) -> usize {
        assert!(self.0 >= 0, "ShapeId must be non-negative for indexing");
        self.0 as usize
    }
}

impl std::ops::Add<i32> for ShapeId {
    type Output = ShapeId;
    fn add(self, rhs: i32) -> ShapeId {
        ShapeId(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for ShapeId {
    type Output = ShapeId;
    fn sub(self, rhs: i32) -> ShapeId {
        ShapeId(self.0 - rhs)
    }
}

impl std::ops::AddAssign<i32> for ShapeId {
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl PartialEq<i32> for ShapeId {
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<i32> for ShapeId {
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::fmt::Display for ShapeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for ShapeId {
    fn from(v: i32) -> Self {
        ShapeId(v)
    }
}

impl From<ShapeId> for i32 {
    fn from(id: ShapeId) -> i32 {
        id.0
    }
}

// ─── ShapeEdgeId ────────────────────────────────────────────────────────

/// A unique identifier for an edge within a [`ShapeIndex`](super::shape_index::ShapeIndex).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeEdgeId {
    /// The shape's index within the `ShapeIndex`.
    pub shape_id: ShapeId,
    /// The edge's index within the shape.
    pub edge_id: i32,
}

impl ShapeEdgeId {
    /// Creates a new `ShapeEdgeId`.
    pub fn new(shape_id: impl Into<ShapeId>, edge_id: i32) -> Self {
        ShapeEdgeId {
            shape_id: shape_id.into(),
            edge_id,
        }
    }
}

impl std::fmt::Display for ShapeEdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.shape_id, self.edge_id)
    }
}

/// Combines a [`ShapeEdgeId`] with its corresponding [`Edge`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeEdge {
    /// The shape and edge identifiers.
    pub id: ShapeEdgeId,
    /// The edge geometry.
    pub edge: Edge,
}

impl ShapeEdge {
    /// Creates a new `ShapeEdge`.
    pub fn new(id: ShapeEdgeId, edge: Edge) -> Self {
        ShapeEdge { id, edge }
    }
}

/// A contiguous range of edge IDs within a shape.
///
/// Edges within a chain are connected: the `v1` of edge *i* equals the
/// `v0` of edge *i+1*. For dimension-2 shapes (polygons), chains form
/// closed loops; for dimension-1 shapes (polylines), they form open paths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Chain {
    /// The first edge ID in this chain.
    pub start: usize,
    /// The number of edges in this chain.
    pub length: usize,
}

impl Chain {
    /// Creates a new chain.
    pub fn new(start: usize, length: usize) -> Self {
        Chain { start, length }
    }
}

/// The position of an edge within a particular chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChainPosition {
    /// The index of the chain containing this edge.
    pub chain_id: usize,
    /// The offset of the edge within the chain.
    pub offset: usize,
}

impl ChainPosition {
    /// Creates a new `ChainPosition`.
    pub fn new(chain_id: usize, offset: usize) -> Self {
        ChainPosition { chain_id, offset }
    }
}

/// An arbitrary reference point with a boolean indicating whether the
/// shape's interior contains the point.
///
/// For shapes that have an interior (dimension 2), `contained` reports
/// whether `point` is inside the shape. For other shapes, `contained`
/// is false.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReferencePoint {
    /// The reference point.
    pub point: Point,
    /// Whether the shape's interior contains the reference point.
    pub contained: bool,
}

impl ReferencePoint {
    /// Creates a new `ReferencePoint`.
    pub fn new(point: Point, contained: bool) -> Self {
        ReferencePoint { point, contained }
    }
}

impl Default for ReferencePoint {
    fn default() -> Self {
        ReferencePoint {
            point: Point::origin(),
            contained: false,
        }
    }
}

// ─── Shape trait ────────────────────────────────────────────────────────

/// The fundamental abstraction for geometry in the S2 library.
///
/// All geometry is represented as a collection of edges. Edges are
/// organized into chains — connected sequences where the endpoint of
/// one edge is the start of the next.
///
/// The *dimension* determines the type of geometry:
/// - **0**: A set of points (each edge is degenerate: v0 == v1).
/// - **1**: A set of polylines (open paths).
/// - **2**: A set of polygons (closed loops with interior).
///
/// Shapes are immutable once added to a [`ShapeIndex`](super::shape_index::ShapeIndex). The trait
/// requires `Send + Sync` for safe sharing across threads.
pub trait Shape: Send + Sync + std::fmt::Debug {
    /// Returns the number of edges in this shape.
    fn num_edges(&self) -> usize;

    /// Returns the edge with the given index.
    fn edge(&self, id: usize) -> Edge;

    /// Returns an arbitrary reference point and whether it is contained
    /// by the shape's interior. Only meaningful for dimension-2 shapes.
    fn reference_point(&self) -> ReferencePoint;

    /// Returns the number of edge chains.
    fn num_chains(&self) -> usize;

    /// Returns the chain with the given index.
    fn chain(&self, chain_id: usize) -> Chain;

    /// Returns the edge at the given offset within the specified chain.
    fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge;

    /// Returns the chain and offset for the given edge ID.
    fn chain_position(&self, edge_id: usize) -> ChainPosition;

    /// Returns the dimension of this shape's geometry.
    fn dimension(&self) -> Dimension;

    /// Reports whether this shape contains no geometry.
    fn is_empty(&self) -> bool {
        self.num_edges() == 0 && (self.dimension() < Dimension::Polygon || self.num_chains() == 0)
    }

    /// Reports whether this shape contains the entire sphere.
    /// Only possible for dimension-2 shapes.
    fn is_full(&self) -> bool {
        !self.is_empty()
            && self.dimension() == Dimension::Polygon
            && self.reference_point().contained
    }

    /// Reports whether this shape has an interior (dimension == 2).
    fn has_interior(&self) -> bool {
        self.dimension() == Dimension::Polygon
    }

    /// Returns the type tag for this shape type, used for serialization.
    ///
    /// Returns 0 (`kNoTypeTag`) if this shape type does not support encoding.
    /// Standard type tags match C++:
    /// - 1 = `Polygon::Shape`
    /// - 2 = `Polyline::Shape`
    /// - 3 = `PointVector`
    /// - 4 = `LaxPolyline`
    /// - 5 = `LaxPolygon`
    fn type_tag(&self) -> u32 {
        0
    }

    /// Encodes this shape's data (without the type tag) to the writer.
    ///
    /// The `hint` controls the trade-off between encoding speed and size.
    ///
    /// # Errors
    ///
    /// Returns an error if this shape type does not support encoding, or
    /// if the write fails.
    fn encode_tagged(
        &self,
        _w: &mut dyn std::io::Write,
        _hint: super::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "this shape type does not support encoding",
        ))
    }
}

// ─── Utilities ──────────────────────────────────────────────────────────

/// Computes a [`ReferencePoint`] for a dimension-2 shape by finding an
/// unbalanced vertex and using [`ContainsVertexQuery`](super::contains_vertex_query::ContainsVertexQuery) to determine
/// containment.
///
/// A "matched" edge is one that can be paired with a corresponding reversed
/// edge. A "balanced" vertex is one where all edges are matched. This
/// function finds an unbalanced vertex to determine containment.
///
/// This is the Rust equivalent of Go's `referencePointForShape`.
pub fn reference_point_for_shape(shape: &dyn Shape) -> ReferencePoint {
    if shape.num_edges() == 0 {
        // A shape with no edges is full if and only if it contains at
        // least one chain (i.e., an empty loop).
        return ReferencePoint::new(Point::origin(), shape.num_chains() > 0);
    }

    // Try the first edge's starting vertex.
    let edge = shape.edge(0);
    if let Some(rp) = reference_point_at_vertex(shape, edge.v0) {
        return rp;
    }

    // Gather all edges and their reverses, then sort to find unmatched edges.
    let n = shape.num_edges();
    let mut edges: Vec<Edge> = (0..n).map(|i| shape.edge(i)).collect();
    let mut rev_edges: Vec<Edge> = edges.iter().map(|e| e.reversed()).collect();

    edges.sort_by(Edge::cmp);
    rev_edges.sort_by(Edge::cmp);

    for i in 0..n {
        if edges[i].cmp(&rev_edges[i]) == std::cmp::Ordering::Less
            && let Some(rp) = reference_point_at_vertex(shape, edges[i].v0)
        {
            return rp;
        }
        if rev_edges[i].cmp(&edges[i]) == std::cmp::Ordering::Less
            && let Some(rp) = reference_point_at_vertex(shape, rev_edges[i].v0)
        {
            return rp;
        }
    }

    // All vertices are balanced. The shape is full if it contains any
    // chain with no edges (an empty loop).
    for i in 0..shape.num_chains() {
        if shape.chain(i).length == 0 {
            return ReferencePoint::new(Point::origin(), true);
        }
    }

    ReferencePoint::new(Point::origin(), false)
}

/// Tests whether `v_test` is an unbalanced vertex and computes containment.
/// Returns `None` if the vertex is balanced (all edges are matched).
fn reference_point_at_vertex(shape: &dyn Shape, v_test: Point) -> Option<ReferencePoint> {
    use crate::s2::contains_vertex_query::ContainsVertexQuery;

    let mut query = ContainsVertexQuery::new(v_test);
    let n = shape.num_edges();
    for i in 0..n {
        let edge = shape.edge(i);
        if edge.v0 == v_test {
            query.add_edge(edge.v1, 1);
        }
        if edge.v1 == v_test {
            query.add_edge(edge.v0, -1);
        }
    }
    let sign = query.contains_vertex();
    if sign == 0 {
        return None; // vertex is balanced
    }
    Some(ReferencePoint::new(v_test, sign > 0))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r3::Vector;

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn edge_is_send_sync() {
        is_send_sync::<Edge>();
    }

    #[test]
    fn shape_edge_id_is_send_sync() {
        is_send_sync::<ShapeEdgeId>();
    }

    #[test]
    fn chain_is_send_sync() {
        is_send_sync::<Chain>();
    }

    #[test]
    fn chain_position_is_send_sync() {
        is_send_sync::<ChainPosition>();
    }

    #[test]
    fn reference_point_is_send_sync() {
        is_send_sync::<ReferencePoint>();
    }

    #[test]
    fn test_edge_reversed() {
        let a = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let b = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        let e = Edge::new(a, b);
        let r = e.reversed();
        assert_eq!(r.v0, b);
        assert_eq!(r.v1, a);
    }

    #[test]
    fn test_edge_is_degenerate() {
        let a = Point(Vector {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let b = Point(Vector {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        });
        assert!(!Edge::new(a, b).is_degenerate());
        assert!(Edge::new(a, a).is_degenerate());
    }

    #[test]
    fn test_shape_edge_id_display() {
        let id = ShapeEdgeId::new(3, 7);
        assert_eq!(format!("{id}"), "3:7");
    }

    #[test]
    fn test_shape_edge_id_ordering() {
        let a = ShapeEdgeId::new(1, 5);
        let b = ShapeEdgeId::new(1, 6);
        let c = ShapeEdgeId::new(2, 0);
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn test_chain_new() {
        let c = Chain::new(10, 5);
        assert_eq!(c.start, 10);
        assert_eq!(c.length, 5);
    }

    #[test]
    fn test_chain_position_new() {
        let cp = ChainPosition::new(2, 3);
        assert_eq!(cp.chain_id, 2);
        assert_eq!(cp.offset, 3);
    }

    #[test]
    fn test_reference_point_default() {
        let rp = ReferencePoint::default();
        assert!(!rp.contained);
    }

    /// A trivial point-vector shape for testing the trait.
    #[derive(Debug)]
    struct TestShape {
        points: Vec<Point>,
    }

    impl Shape for TestShape {
        fn num_edges(&self) -> usize {
            self.points.len()
        }
        fn edge(&self, id: usize) -> Edge {
            Edge::new(self.points[id], self.points[id])
        }
        fn reference_point(&self) -> ReferencePoint {
            ReferencePoint::default()
        }
        fn num_chains(&self) -> usize {
            self.points.len()
        }
        fn chain(&self, chain_id: usize) -> Chain {
            Chain::new(chain_id, 1)
        }
        fn chain_edge(&self, chain_id: usize, _offset: usize) -> Edge {
            self.edge(chain_id)
        }
        fn chain_position(&self, edge_id: usize) -> ChainPosition {
            ChainPosition::new(edge_id, 0)
        }
        fn dimension(&self) -> Dimension {
            Dimension::Point
        }
    }

    #[test]
    fn test_shape_trait() {
        let shape = TestShape {
            points: vec![
                Point(Vector {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                }),
                Point(Vector {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                }),
            ],
        };
        assert_eq!(shape.num_edges(), 2);
        assert_eq!(shape.dimension(), Dimension::Point);
        assert!(!shape.is_empty());
        assert!(!shape.is_full());
        assert!(!shape.has_interior());
    }

    #[test]
    fn test_shape_empty() {
        let shape = TestShape { points: vec![] };
        assert!(shape.is_empty());
        assert!(!shape.is_full());
    }

    #[test]
    fn test_dyn_shape_is_object_safe() {
        let shape: Box<dyn Shape> = Box::new(TestShape {
            points: vec![Point(Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            })],
        });
        assert_eq!(shape.num_edges(), 1);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let p0 = Point::from_coords(1.0, 0.0, 0.0);
        let p1 = Point::from_coords(0.0, 1.0, 0.0);

        let edge = Edge::new(p0, p1);
        let json = serde_json::to_string(&edge).unwrap();
        let back: Edge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, back);

        let seid = ShapeEdgeId::new(3, 7);
        let json = serde_json::to_string(&seid).unwrap();
        let back: ShapeEdgeId = serde_json::from_str(&json).unwrap();
        assert_eq!(seid, back);

        let se = ShapeEdge::new(seid, edge);
        let json = serde_json::to_string(&se).unwrap();
        let back: ShapeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(se, back);

        let chain = Chain {
            start: 0,
            length: 5,
        };
        let json = serde_json::to_string(&chain).unwrap();
        let back: Chain = serde_json::from_str(&json).unwrap();
        assert_eq!(chain, back);

        let cp = ChainPosition::new(2, 3);
        let json = serde_json::to_string(&cp).unwrap();
        let back: ChainPosition = serde_json::from_str(&json).unwrap();
        assert_eq!(cp, back);

        let rp = ReferencePoint::new(p0, true);
        let json = serde_json::to_string(&rp).unwrap();
        let back: ReferencePoint = serde_json::from_str(&json).unwrap();
        assert_eq!(rp, back);
    }
}

#[cfg(test)]
#[path = "shape_tests.rs"]
mod shape_tests;
