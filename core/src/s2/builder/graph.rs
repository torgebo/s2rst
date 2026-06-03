// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/VertexId (i32) used as Vec indices throughout the graph API — requires newtype to fix"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId/VertexId (i32) <-> usize — requires newtype indices to fix"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId/VertexId — requires newtype indices to fix"
)]
// S2Builder::Graph — the assembled edge graph passed to layers.

use crate::s2::Point;
use crate::s2::predicates as s2pred;
use std::collections::BTreeMap;

use super::id_set_lexicon::{EMPTY_SET_ID, IdSetLexicon};
use super::layer::IsFullPolygonPredicate;
use super::{InputEdgeId, InputEdgeIdSetId, Label, LabelSetId, S2Error, S2ErrorCode};

// ─── Index newtypes ─────────────────────────────────────────────────────────

/// Vertex identifier within a [`Graph`].
///
/// Wraps an `i32` (matching the C++ API). Negative values are used as
/// sentinels in some algorithms.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VertexId(pub i32);

impl VertexId {
    /// Maximum valid `VertexId` value (sentinel).
    pub const MAX: VertexId = VertexId(i32::MAX);

    /// Creates a new `VertexId`.
    pub const fn new(v: i32) -> Self {
        VertexId(v)
    }

    /// Returns the raw `i32` value.
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Returns the value as `usize` for indexing.
    ///
    /// # Panics
    /// Panics if the value is negative.
    #[expect(clippy::cast_sign_loss, reason = "guarded by assert")]
    pub const fn as_usize(self) -> usize {
        assert!(self.0 >= 0, "VertexId must be non-negative for indexing");
        self.0 as usize
    }
}

impl std::fmt::Display for VertexId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for VertexId {
    fn from(v: i32) -> Self {
        VertexId(v)
    }
}

impl From<VertexId> for i32 {
    fn from(v: VertexId) -> Self {
        v.0
    }
}

impl std::ops::Add<i32> for VertexId {
    type Output = VertexId;
    fn add(self, rhs: i32) -> Self {
        VertexId(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for VertexId {
    type Output = VertexId;
    fn sub(self, rhs: i32) -> Self {
        VertexId(self.0 - rhs)
    }
}

impl std::ops::AddAssign<i32> for VertexId {
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl PartialEq<i32> for VertexId {
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<i32> for VertexId {
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::ops::Neg for VertexId {
    type Output = VertexId;
    fn neg(self) -> Self {
        VertexId(-self.0)
    }
}

/// Edge identifier within a [`Graph`].
///
/// Wraps an `i32` (matching the C++ API). Negative values are used as
/// sentinels in some algorithms (e.g. left-turn maps, sibling maps).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EdgeId(pub i32);

impl EdgeId {
    /// Maximum valid `EdgeId` value (sentinel).
    pub const MAX: EdgeId = EdgeId(i32::MAX);

    /// Creates a new `EdgeId`.
    pub const fn new(v: i32) -> Self {
        EdgeId(v)
    }

    /// Returns the raw `i32` value.
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Returns the value as `usize` for indexing.
    ///
    /// # Panics
    /// Panics if the value is negative.
    #[expect(clippy::cast_sign_loss, reason = "guarded by assert")]
    pub const fn as_usize(self) -> usize {
        assert!(self.0 >= 0, "EdgeId must be non-negative for indexing");
        self.0 as usize
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for EdgeId {
    fn from(v: i32) -> Self {
        EdgeId(v)
    }
}

impl From<EdgeId> for i32 {
    fn from(v: EdgeId) -> Self {
        v.0
    }
}

impl std::ops::Add<i32> for EdgeId {
    type Output = EdgeId;
    fn add(self, rhs: i32) -> Self {
        EdgeId(self.0 + rhs)
    }
}

impl std::ops::Sub<i32> for EdgeId {
    type Output = EdgeId;
    fn sub(self, rhs: i32) -> Self {
        EdgeId(self.0 - rhs)
    }
}

impl std::ops::Sub<EdgeId> for EdgeId {
    type Output = i32;
    fn sub(self, rhs: EdgeId) -> i32 {
        self.0 - rhs.0
    }
}

impl std::ops::AddAssign<i32> for EdgeId {
    fn add_assign(&mut self, rhs: i32) {
        self.0 += rhs;
    }
}

impl std::ops::SubAssign<i32> for EdgeId {
    fn sub_assign(&mut self, rhs: i32) {
        self.0 -= rhs;
    }
}

impl PartialEq<i32> for EdgeId {
    fn eq(&self, other: &i32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<i32> for EdgeId {
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::ops::Div<i32> for EdgeId {
    type Output = EdgeId;
    fn div(self, rhs: i32) -> Self {
        EdgeId(self.0 / rhs)
    }
}

impl std::ops::Rem<i32> for EdgeId {
    type Output = EdgeId;
    fn rem(self, rhs: i32) -> Self {
        EdgeId(self.0 % rhs)
    }
}

impl std::ops::Neg for EdgeId {
    type Output = EdgeId;
    fn neg(self) -> Self {
        EdgeId(-self.0)
    }
}

/// An edge as a (origin, destination) vertex pair.
pub type Edge = (VertexId, VertexId);

/// A loop consisting of a sequence of edge ids.
pub type EdgeLoop = Vec<EdgeId>;

/// A directed component: one or more loops connected by shared vertices.
pub type DirectedComponent = Vec<EdgeLoop>;

/// An undirected component: two complements, each a set of loops.
pub type UndirectedComponent = [Vec<EdgeLoop>; 2];

/// A polyline consisting of a sequence of edge ids.
pub type EdgePolyline = Vec<EdgeId>;

/// Value larger than any valid `InputEdgeId`.
pub const MAX_INPUT_EDGE_ID: InputEdgeId = InputEdgeId(i32::MAX);

/// Indicates that an edge does not correspond to any input edge.
pub const NO_INPUT_EDGE_ID: InputEdgeId = InputEdgeId(i32::MAX - 1);

// ─── GraphOptions enums ─────────────────────────────────────────────────────

/// Whether edges are directed or undirected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EdgeType {
    /// Each edge has a specified direction.
    #[default]
    Directed,
    /// Edges are considered undirected; each edge may be reversed during assembly.
    Undirected,
}

/// How degenerate edges (v0 == v1) are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DegenerateEdges {
    /// Remove all degenerate edges.
    Discard,
    /// Merge duplicate degenerate edges, discard if connected to non-degenerate.
    DiscardExcess,
    /// Keep all degenerate edges.
    #[default]
    Keep,
}

/// How duplicate edges are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DuplicateEdges {
    /// Combine duplicate edges, merging their labels.
    Merge,
    /// Keep all duplicate edges.
    #[default]
    Keep,
}

/// How sibling edge pairs (e and reverse(e)) are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SiblingPairs {
    /// Remove all sibling pairs.
    Discard,
    /// Discard siblings except one if the result would be empty.
    DiscardExcess,
    /// Keep all sibling pairs.
    #[default]
    Keep,
    /// All edges must have a sibling (error otherwise).
    Require,
    /// Ensure all edges have siblings (create them if needed).
    Create,
}

/// Whether loops should be simple cycles or circuits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LoopType {
    /// Simple cycles: no repeated vertices.
    #[default]
    Simple,
    /// Circuits: allow repeated vertices but not repeated edges.
    Circuit,
}

/// Whether degenerate boundaries (filaments) should be kept or discarded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DegenerateBoundaries {
    /// Discard degenerate boundaries (filaments).
    Discard,
    /// Keep degenerate boundaries (filaments).
    #[default]
    Keep,
}

/// Whether polylines are paths or walks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PolylineType {
    /// Paths: no duplicate vertices except possibly first/last.
    #[default]
    Path,
    /// Walks: allow duplicate vertices and edges.
    Walk,
}

/// `GraphOptions` controls how edges are processed when building a Graph.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GraphOptions {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
    /// How degenerate edges (both endpoints identical) are handled.
    pub degenerate_edges: DegenerateEdges,
    /// How duplicate edges are handled.
    pub duplicate_edges: DuplicateEdges,
    /// How sibling pairs (edge and its reverse) are handled.
    pub sibling_pairs: SiblingPairs,
    /// Whether vertices with no remaining edges are removed.
    pub allow_vertex_filtering: bool,
}

impl Default for GraphOptions {
    fn default() -> Self {
        GraphOptions {
            edge_type: EdgeType::Directed,
            degenerate_edges: DegenerateEdges::Keep,
            duplicate_edges: DuplicateEdges::Keep,
            sibling_pairs: SiblingPairs::Keep,
            allow_vertex_filtering: true,
        }
    }
}

impl GraphOptions {
    /// Creates graph options with the given settings and vertex filtering enabled.
    pub fn new(
        edge_type: EdgeType,
        degenerate_edges: DegenerateEdges,
        duplicate_edges: DuplicateEdges,
        sibling_pairs: SiblingPairs,
    ) -> Self {
        GraphOptions {
            edge_type,
            degenerate_edges,
            duplicate_edges,
            sibling_pairs,
            allow_vertex_filtering: true,
        }
    }
}

// ─── Graph ──────────────────────────────────────────────────────────────────

/// The assembled edge graph passed to layers for output construction.
///
/// A Graph contains a set of vertices and directed edges. Edges are stored
/// as (`VertexId`, `VertexId`) pairs sorted in lexicographic order. The graph
/// also tracks which input edges correspond to each graph edge, and any
/// labels attached to those edges.
pub struct Graph {
    options: GraphOptions,
    vertices: Vec<Point>,
    edges: Vec<Edge>,
    input_edge_id_set_ids: Vec<InputEdgeIdSetId>,
    input_edge_id_set_lexicon: IdSetLexicon,
    label_set_ids: Vec<LabelSetId>,
    label_set_lexicon: IdSetLexicon,
    is_full_polygon_predicate: Option<IsFullPolygonPredicate>,
}

impl std::fmt::Debug for Graph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Graph")
            .field("options", &self.options)
            .field("vertices", &self.vertices)
            .field("edges", &self.edges)
            .field("input_edge_id_set_ids", &self.input_edge_id_set_ids)
            .field("input_edge_id_set_lexicon", &self.input_edge_id_set_lexicon)
            .field("label_set_ids", &self.label_set_ids)
            .field("label_set_lexicon", &self.label_set_lexicon)
            .field(
                "is_full_polygon_predicate",
                &self.is_full_polygon_predicate.as_ref().map(|_| ".."),
            )
            .finish()
    }
}

impl Graph {
    /// Creates a new Graph with the given data. Edges are processed according
    /// to the `GraphOptions` (sorting, dedup, sibling handling, etc.).
    #[expect(clippy::too_many_arguments, reason = "matches C++ API")]
    pub fn new(
        mut options: GraphOptions,
        vertices: Vec<Point>,
        mut edges: Vec<Edge>,
        mut input_edge_id_set_ids: Vec<InputEdgeIdSetId>,
        mut input_edge_id_set_lexicon: IdSetLexicon,
        label_set_ids: Vec<LabelSetId>,
        label_set_lexicon: IdSetLexicon,
        is_full_polygon_predicate: Option<IsFullPolygonPredicate>,
    ) -> Self {
        let mut error = S2Error::ok();
        Self::process_edges(
            &mut options,
            &mut edges,
            &mut input_edge_id_set_ids,
            &mut input_edge_id_set_lexicon,
            &mut error,
        );
        Graph {
            options,
            vertices,
            edges,
            input_edge_id_set_ids,
            input_edge_id_set_lexicon,
            label_set_ids,
            label_set_lexicon,
            is_full_polygon_predicate,
        }
    }

    /// Creates a Graph directly from pre-processed data (edges already sorted,
    /// options already applied). No `ProcessEdges` is called.
    #[expect(clippy::too_many_arguments, reason = "matches C++ API")]
    pub fn from_raw_parts(
        options: GraphOptions,
        vertices: Vec<Point>,
        edges: Vec<Edge>,
        input_edge_id_set_ids: Vec<InputEdgeIdSetId>,
        input_edge_id_set_lexicon: IdSetLexicon,
        label_set_ids: Vec<LabelSetId>,
        label_set_lexicon: IdSetLexicon,
        is_full_polygon_predicate: Option<IsFullPolygonPredicate>,
    ) -> Self {
        debug_assert!(edges.windows(2).all(|w| w[0] <= w[1]));
        debug_assert_eq!(edges.len(), input_edge_id_set_ids.len());
        Graph {
            options,
            vertices,
            edges,
            input_edge_id_set_ids,
            input_edge_id_set_lexicon,
            label_set_ids,
            label_set_lexicon,
            is_full_polygon_predicate,
        }
    }

    // ─── Basic accessors ────────────────────────────────────────────────

    /// Returns the graph options.
    pub fn options(&self) -> &GraphOptions {
        &self.options
    }

    /// Returns the number of vertices in the graph.
    pub fn num_vertices(&self) -> VertexId {
        VertexId(self.vertices.len() as i32)
    }

    /// Returns the position of vertex `v`.
    pub fn vertex(&self, v: impl Into<VertexId>) -> Point {
        self.vertices[v.into().as_usize()]
    }

    /// Returns all vertex positions.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Returns the number of edges in the graph.
    pub fn num_edges(&self) -> EdgeId {
        EdgeId(self.edges.len() as i32)
    }

    /// Returns the edge with the given id as a `(src, dst)` vertex id pair.
    pub fn edge(&self, e: impl Into<EdgeId>) -> Edge {
        self.edges[e.into().as_usize()]
    }

    /// Returns all edges as `(src, dst)` vertex id pairs.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Returns the reverse of an edge.
    pub fn reverse(e: Edge) -> Edge {
        (e.1, e.0)
    }

    /// Stable comparison for edges: breaks ties by edge ID.
    pub fn stable_less_than(a: Edge, b: Edge, ai: EdgeId, bi: EdgeId) -> bool {
        if a.0 != b.0 {
            return a.0 < b.0;
        }
        if a.1 != b.1 {
            return a.1 < b.1;
        }
        ai < bi
    }

    // ─── Input edge tracking ────────────────────────────────────────────

    /// Returns the set of input edge IDs that correspond to graph edge `e`.
    pub fn input_edge_ids(&self, e: impl Into<EdgeId>) -> Vec<i32> {
        let e = e.into();
        self.input_edge_id_set_lexicon
            .id_set(self.input_edge_id_set_ids[e.as_usize()])
    }

    /// Returns the input edge id set id for graph edge `e`.
    pub fn input_edge_id_set_id(&self, e: impl Into<EdgeId>) -> InputEdgeIdSetId {
        self.input_edge_id_set_ids[e.into().as_usize()]
    }

    /// Returns the raw input edge id set ids for all graph edges.
    pub fn input_edge_id_set_ids(&self) -> &[InputEdgeIdSetId] {
        &self.input_edge_id_set_ids
    }

    /// Returns the lexicon used to decode input edge id sets.
    pub fn input_edge_id_set_lexicon(&self) -> &IdSetLexicon {
        &self.input_edge_id_set_lexicon
    }

    /// Returns the minimum input edge ID for graph edge `e`, or
    /// `NO_INPUT_EDGE_ID` if the edge has no input edges.
    pub fn min_input_edge_id(&self, e: impl Into<EdgeId>) -> InputEdgeId {
        let e = e.into();
        let ids = self.input_edge_ids(e);
        if ids.is_empty() {
            NO_INPUT_EDGE_ID
        } else {
            InputEdgeId(ids.into_iter().min().unwrap_or(NO_INPUT_EDGE_ID.0))
        }
    }

    /// Returns the minimum input edge ID for each graph edge.
    pub fn get_min_input_edge_ids(&self) -> Vec<InputEdgeId> {
        (0..self.num_edges().0)
            .map(EdgeId)
            .map(|e| self.min_input_edge_id(e))
            .collect()
    }

    /// Returns edge IDs sorted by minimum input edge ID (approximation of
    /// input edge ordering).
    pub fn get_input_edge_order(&self, min_input_ids: &[InputEdgeId]) -> Vec<EdgeId> {
        let mut order: Vec<EdgeId> = (0..min_input_ids.len() as i32).map(EdgeId).collect();
        order.sort_unstable_by(|&a, &b| {
            (min_input_ids[a.as_usize()], a).cmp(&(min_input_ids[b.as_usize()], b))
        });
        order
    }

    // ─── Label tracking ─────────────────────────────────────────────────

    /// Returns the labels for input edge `e`.
    pub fn labels(&self, e: impl Into<InputEdgeId>) -> Vec<Label> {
        let e = e.into();
        if self.label_set_ids.is_empty() {
            return Vec::new();
        }
        if e.as_usize() < self.label_set_ids.len() {
            self.label_set_lexicon
                .id_set(self.label_set_ids[e.as_usize()])
        } else {
            Vec::new()
        }
    }

    /// Returns the raw label set ids for all edges.
    pub fn label_set_ids(&self) -> &[LabelSetId] {
        &self.label_set_ids
    }

    /// Returns the lexicon used to decode label sets.
    pub fn label_set_lexicon(&self) -> &IdSetLexicon {
        &self.label_set_lexicon
    }

    // ─── Polygon predicate ──────────────────────────────────────────────

    /// Determines whether the graph represents a full polygon.
    ///
    /// # Errors
    ///
    /// Returns an error if the `IsFullPolygonPredicate` was not specified
    /// or if the predicate itself returns an error.
    pub fn is_full_polygon(&self) -> Result<bool, S2Error> {
        if let Some(ref pred) = self.is_full_polygon_predicate {
            pred(self)
        } else if self.edges.is_empty() {
            Ok(false)
        } else {
            Err(S2Error::new(
                S2ErrorCode::BuilderIsFullPredicateNotSpecified,
                "IsFullPolygonPredicate was not specified",
            ))
        }
    }

    /// Returns a clone of the `is_full_polygon_predicate`, if set.
    /// (The predicate is stored as an Arc so it can be shared.)
    pub fn is_full_polygon_predicate_clone(&self) -> Option<IsFullPolygonPredicate> {
        self.is_full_polygon_predicate.clone()
    }

    // ─── Edge processing (static) ───────────────────────────────────────

    /// Processes edges according to `GraphOptions` using a merge-join algorithm.
    /// This handles degenerate edges, duplicate edges, and sibling pairs in a
    /// single pass over two sorted arrays (outgoing and incoming edges).
    pub fn process_edges(
        options: &mut GraphOptions,
        edges: &mut Vec<Edge>,
        input_ids: &mut Vec<InputEdgeIdSetId>,
        id_set_lexicon: &mut IdSetLexicon,
        error: &mut S2Error,
    ) {
        let mut processor = EdgeProcessor::new(options, edges, input_ids, id_set_lexicon);
        processor.run(error);
        // REQUIRE/CREATE change edge_type to DIRECTED (see C++ docs).
        if options.sibling_pairs == SiblingPairs::Require
            || options.sibling_pairs == SiblingPairs::Create
        {
            options.edge_type = EdgeType::Directed;
        }
    }

    // ─── In-edge IDs and sibling maps ───────────────────────────────────

    /// Returns edge IDs sorted by (destination, origin) — i.e., sorted by the
    /// reversed edge. All incoming edges to each vertex form a contiguous range.
    pub fn get_in_edge_ids(&self) -> Vec<EdgeId> {
        let mut in_edge_ids: Vec<EdgeId> = (0..self.num_edges().0).map(EdgeId).collect();
        in_edge_ids.sort_unstable_by(|&ai, &bi| {
            let a = Self::reverse(self.edge(ai));
            let b = Self::reverse(self.edge(bi));
            if Self::stable_less_than(a, b, ai, bi) {
                std::cmp::Ordering::Less
            } else if Self::stable_less_than(b, a, bi, ai) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        in_edge_ids
    }

    /// Returns a map from each edge to its sibling (reverse) edge.
    /// Requires that every edge has a sibling.
    pub fn get_sibling_map(&self) -> Vec<EdgeId> {
        let mut in_edge_ids = self.get_in_edge_ids();
        self.make_sibling_map(&mut in_edge_ids);
        // Validate: each edge's sibling's sibling must be itself.
        for e in (0..self.num_edges().0).map(EdgeId) {
            debug_assert_eq!(e, in_edge_ids[in_edge_ids[e.as_usize()].as_usize()]);
        }
        in_edge_ids
    }

    /// Converts an `in_edge_ids` array into a sibling map by pairing up
    /// undirected degenerate edges. For directed edges or when degenerate
    /// edges are discarded, this is a no-op (`in_edge_ids` already maps each
    /// edge to its sibling).
    pub fn make_sibling_map(&self, in_edge_ids: &mut [EdgeId]) {
        debug_assert!(
            self.options.sibling_pairs == SiblingPairs::Require
                || self.options.sibling_pairs == SiblingPairs::Create
                || self.options.edge_type == EdgeType::Undirected
        );
        for e in (0..self.num_edges().0).map(EdgeId) {
            debug_assert_eq!(
                self.edge(e),
                Self::reverse(self.edge(in_edge_ids[e.as_usize()]))
            );
        }
        if self.options.edge_type == EdgeType::Directed {
            return;
        }
        if self.options.degenerate_edges == DegenerateEdges::Discard {
            return;
        }
        let mut e = EdgeId(0);
        while e < self.num_edges() {
            let edge = self.edge(e);
            if edge.1 == edge.0 {
                // Undirected degenerate edge: pair consecutive degenerate edges.
                debug_assert!(e + 1 < self.num_edges());
                debug_assert_eq!(self.edge(e + 1).0, edge.0);
                debug_assert_eq!(self.edge(e + 1).1, edge.0);
                in_edge_ids[e.as_usize()] = e + 1;
                in_edge_ids[(e + 1).as_usize()] = e;
                e += 2;
            } else {
                e += 1;
            }
        }
    }

    // ─── VertexOutMap / VertexInMap constructors ─────────────────────────

    /// Returns a compact outgoing edge map (uses sorted edge array directly).
    pub fn get_vertex_out_map(&self) -> VertexOutMap {
        VertexOutMap::new(self)
    }

    /// Returns a compact incoming edge map.
    pub fn get_vertex_in_map(&self) -> VertexInMap {
        VertexInMap::new(self)
    }

    // ─── GetLeftTurnMap ─────────────────────────────────────────────────

    /// Builds a left-turn map: maps each edge e=(v0,v1) to the next outgoing
    /// edge from v1 in clockwise order. Following left turns from any edge
    /// traces a loop whose interior contains no edges.
    ///
    /// `in_edge_ids` should be `GetInEdgeIds()` or `GetSiblingMap()`.
    pub fn get_left_turn_map(&self, in_edge_ids: &[EdgeId], error: &mut S2Error) -> Vec<EdgeId> {
        let mut left_turn_map = vec![EdgeId(-1); self.num_edges().as_usize()];
        if self.num_edges() == 0 {
            return left_turn_map;
        }

        let mut v0_edges: Vec<VertexEdge> = Vec::new();
        let mut e_in: Vec<EdgeId> = Vec::new();
        let mut e_out: Vec<EdgeId> = Vec::new();

        let sentinel: Edge = (self.num_vertices(), self.num_vertices());

        let mut out = 0usize;
        let mut inp = 0usize;
        let num_e = self.num_edges().as_usize();

        let out_edge = |idx: usize| -> Edge {
            if idx >= num_e {
                sentinel
            } else {
                self.edge(EdgeId(idx as i32))
            }
        };
        let in_edge = |idx: usize| -> Edge {
            if idx >= num_e {
                sentinel
            } else {
                Self::reverse(self.edge(in_edge_ids[idx]))
            }
        };

        let mut min_edge = std::cmp::min(out_edge(out), in_edge(inp));
        while min_edge != sentinel {
            let v0 = min_edge.0;
            while min_edge.0 == v0 {
                let v1 = min_edge.1;
                let out_begin = out;
                let in_begin = inp;
                while out < num_e && out_edge(out) == min_edge {
                    out += 1;
                }
                while inp < num_e && in_edge(inp) == min_edge {
                    inp += 1;
                }
                if v0 == v1 {
                    // Each degenerate edge becomes its own loop.
                    for idx in in_begin..inp {
                        left_turn_map[in_edge_ids[idx].as_usize()] = in_edge_ids[idx];
                    }
                } else {
                    add_vertex_edges(
                        EdgeId(out_begin as i32),
                        EdgeId(out as i32),
                        EdgeId(in_begin as i32),
                        EdgeId(inp as i32),
                        v1,
                        in_edge_ids,
                        &mut v0_edges,
                    );
                }
                min_edge = std::cmp::min(out_edge(out), in_edge(inp));
            }
            if v0_edges.is_empty() {
                continue;
            }

            // Sort edges in clockwise order around v0.
            let v0_point = self.vertex(v0);
            let min_endpoint = v0_edges[0].endpoint;
            let min_ep_point = self.vertex(min_endpoint);
            if v0_edges.len() > 1 {
                let vertices = &self.vertices;
                v0_edges[1..].sort_unstable_by(|a, b| {
                    if a.endpoint == b.endpoint {
                        return a.rank.cmp(&b.rank);
                    }
                    if a.endpoint == min_endpoint {
                        return std::cmp::Ordering::Less;
                    }
                    if b.endpoint == min_endpoint {
                        return std::cmp::Ordering::Greater;
                    }
                    let a_point = vertices[a.endpoint.as_usize()];
                    let b_point = vertices[b.endpoint.as_usize()];
                    if s2pred::ordered_ccw(a_point, b_point, min_ep_point, v0_point) {
                        std::cmp::Ordering::Greater // clockwise = reverse of CCW
                    } else {
                        std::cmp::Ordering::Less
                    }
                });
            }

            // Match incoming with outgoing edges using a stack.
            for ve in &v0_edges {
                if ve.incoming {
                    e_in.push(ve.index);
                } else if !e_in.is_empty() {
                    left_turn_map[e_in[e_in.len() - 1].as_usize()] = ve.index;
                    e_in.pop();
                } else {
                    e_out.push(ve.index);
                }
            }
            // Pair up remaining edges (circular wrap-around).
            e_out.reverse();
            while !e_out.is_empty() && !e_in.is_empty() {
                left_turn_map[e_in[e_in.len() - 1].as_usize()] = e_out[e_out.len() - 1];
                e_in.pop();
                e_out.pop();
            }
            if !e_in.is_empty() && error.is_ok() {
                *error = S2Error::new(
                    S2ErrorCode::BuilderEdgesDoNotFormLoops,
                    "Given edges do not form loops (indegree != outdegree)",
                );
            }
            e_in.clear();
            e_out.clear();
            v0_edges.clear();
        }
        left_turn_map
    }

    // ─── Canonicalization ───────────────────────────────────────────────

    /// Rotates `loop_edges` so the edge(s) with the largest input edge ids
    /// are last. This preserves input loop order.
    pub fn canonicalize_loop_order(min_input_ids: &[InputEdgeId], loop_edges: &mut [EdgeId]) {
        if loop_edges.is_empty() {
            return;
        }
        let mut pos = 0usize;
        let mut saw_gap = false;
        for i in 1..loop_edges.len() {
            let cmp =
                min_input_ids[loop_edges[i].as_usize()] - min_input_ids[loop_edges[pos].as_usize()];
            if cmp < 0 {
                saw_gap = true;
            } else if cmp > 0 || !saw_gap {
                pos = i;
                saw_gap = false;
            }
        }
        pos += 1;
        if pos == loop_edges.len() {
            pos = 0;
        }
        loop_edges.rotate_left(pos);
    }

    /// Sorts edge chains by the minimum input edge id of each chain's first
    /// edge. This preserves input ordering across multiple loops/polylines.
    pub fn canonicalize_vector_order(min_input_ids: &[InputEdgeId], chains: &mut [Vec<EdgeId>]) {
        chains.sort_unstable_by(|a, b| {
            (min_input_ids[a[0].as_usize()], a[0]).cmp(&(min_input_ids[b[0].as_usize()], b[0]))
        });
    }

    // ─── Loop building ──────────────────────────────────────────────────

    /// Builds loops from directed edges using a left-turn map. Supports both
    /// SIMPLE (break at repeated vertices) and CIRCUIT (break at repeated edges).
    pub fn get_directed_loops(&self, loop_type: LoopType, error: &mut S2Error) -> Vec<EdgeLoop> {
        debug_assert!(
            self.options.degenerate_edges == DegenerateEdges::Discard
                || self.options.degenerate_edges == DegenerateEdges::DiscardExcess
        );
        debug_assert_eq!(self.options.edge_type, EdgeType::Directed);
        let in_edge_ids = self.get_in_edge_ids();
        let mut left_turn_map = self.get_left_turn_map(&in_edge_ids, error);
        if !error.is_ok() {
            return Vec::new();
        }
        let min_input_ids = self.get_min_input_edge_ids();

        let mut path_index: Vec<i32> = if loop_type == LoopType::Simple {
            vec![-1; self.num_vertices().as_usize()]
        } else {
            Vec::new()
        };

        let mut loops = Vec::new();
        let mut path: Vec<EdgeId> = Vec::new();

        for start in (0..self.num_edges().0).map(EdgeId) {
            if left_turn_map[start.as_usize()] < 0 {
                continue;
            }

            let mut e = start;
            while left_turn_map[e.as_usize()] >= 0 {
                path.push(e);
                let next = left_turn_map[e.as_usize()];
                left_turn_map[e.as_usize()] = EdgeId(-1);
                if loop_type == LoopType::Simple {
                    path_index[self.edge(e).0.as_usize()] = path.len() as i32 - 1;
                    let loop_start = path_index[self.edge(e).1.as_usize()];
                    if loop_start >= 0 {
                        // Peel off a loop from the path.
                        let mut peeled: Vec<EdgeId> = path[loop_start as usize..].to_vec();
                        path.truncate(loop_start as usize);
                        for &e2 in &peeled {
                            path_index[self.edge(e2).0.as_usize()] = -1;
                        }
                        Self::canonicalize_loop_order(&min_input_ids, &mut peeled);
                        loops.push(peeled);
                    }
                }
                e = next;
            }
            if loop_type == LoopType::Simple {
                debug_assert!(path.is_empty());
            } else {
                Self::canonicalize_loop_order(&min_input_ids, &mut path);
                loops.push(std::mem::take(&mut path));
            }
        }
        Self::canonicalize_vector_order(&min_input_ids, &mut loops);
        loops
    }

    /// Builds directed components: groups of loops connected by shared vertices.
    /// Each component is a vector of loops. Requires sibling pairs.
    pub fn get_directed_components(
        &self,
        degenerate_boundaries: DegenerateBoundaries,
        error: &mut S2Error,
    ) -> Vec<DirectedComponent> {
        debug_assert!(
            self.options.degenerate_edges == DegenerateEdges::Discard
                || self.options.degenerate_edges == DegenerateEdges::DiscardExcess
        );
        debug_assert!(
            self.options.sibling_pairs == SiblingPairs::Require
                || self.options.sibling_pairs == SiblingPairs::Create
        );
        debug_assert_eq!(self.options.edge_type, EdgeType::Directed);
        let sibling_map = self.get_sibling_map();
        let mut left_turn_map = self.get_left_turn_map(&sibling_map, error);
        if !error.is_ok() {
            return Vec::new();
        }
        let min_input_ids = self.get_min_input_edge_ids();
        let mut frontier: Vec<EdgeId> = Vec::new();

        let mut path_index: Vec<i32> = if degenerate_boundaries == DegenerateBoundaries::Discard {
            vec![-1; self.num_edges().as_usize()]
        } else {
            Vec::new()
        };

        let mut components = Vec::new();

        for start in (0..self.num_edges().0).map(EdgeId) {
            if left_turn_map[start.as_usize()] < 0 {
                continue;
            }

            let mut component: DirectedComponent = Vec::new();
            frontier.push(start);
            while let Some(e_start) = frontier.pop() {
                if left_turn_map[e_start.as_usize()] < 0 {
                    continue;
                }

                let mut path: Vec<EdgeId> = Vec::new();
                let mut e = e_start;
                while left_turn_map[e.as_usize()] >= 0 {
                    path.push(e);
                    let next = left_turn_map[e.as_usize()];
                    left_turn_map[e.as_usize()] = EdgeId(-1);
                    let sibling = sibling_map[e.as_usize()];
                    if left_turn_map[sibling.as_usize()] >= 0 {
                        frontier.push(sibling);
                    }
                    if degenerate_boundaries == DegenerateBoundaries::Discard {
                        path_index[e.as_usize()] = path.len() as i32 - 1;
                        let sibling_index = path_index[sibling.as_usize()];
                        if sibling_index >= 0 {
                            // Adjacent sibling pair: just remove both.
                            if sibling_index as usize + 2 == path.len() {
                                path.truncate(sibling_index as usize);
                            } else {
                                // Peel off a loop.
                                let loop_start = sibling_index as usize + 1;
                                let loop_end = path.len() - 1;
                                let mut peeled: Vec<EdgeId> = path[loop_start..loop_end].to_vec();
                                path.truncate(sibling_index as usize);
                                for &e2 in &peeled {
                                    path_index[e2.as_usize()] = -1;
                                }
                                Self::canonicalize_loop_order(&min_input_ids, &mut peeled);
                                component.push(peeled);
                            }
                        }
                    }
                    e = next;
                }
                if degenerate_boundaries == DegenerateBoundaries::Discard {
                    for &e2 in &path {
                        path_index[e2.as_usize()] = -1;
                    }
                }
                Self::canonicalize_loop_order(&min_input_ids, &mut path);
                component.push(path);
            }
            Self::canonicalize_vector_order(&min_input_ids, &mut component);
            components.push(component);
        }
        // Sort components by input edge ordering.
        components.sort_unstable_by(|a, b| {
            min_input_ids[a[0][0].as_usize()].cmp(&min_input_ids[b[0][0].as_usize()])
        });
        components
    }

    /// Builds loops from undirected edges. Each component has two complements
    /// representing the two possible interpretations of the region.
    pub fn get_undirected_components(
        &self,
        loop_type: LoopType,
        error: &mut S2Error,
    ) -> Vec<UndirectedComponent> {
        debug_assert!(
            self.options.degenerate_edges == DegenerateEdges::Discard
                || self.options.degenerate_edges == DegenerateEdges::DiscardExcess
        );
        debug_assert_eq!(self.options.edge_type, EdgeType::Undirected);
        let mut sibling_map = self.get_in_edge_ids();
        let mut left_turn_map = self.get_left_turn_map(&sibling_map, error);
        if !error.is_ok() {
            return Vec::new();
        }
        self.make_sibling_map(&mut sibling_map);
        let min_input_ids = self.get_min_input_edge_ids();

        let mark_edge_used = |slot: i32| -> EdgeId { EdgeId(-1 - slot) };

        let mut frontier: Vec<(EdgeId, i32)> = Vec::new();
        let mut path_index: Vec<i32> = if loop_type == LoopType::Simple {
            vec![-1; self.num_vertices().as_usize()]
        } else {
            Vec::new()
        };

        let mut components = Vec::new();

        for min_start in (0..self.num_edges().0).map(EdgeId) {
            if left_turn_map[min_start.as_usize()] < 0 {
                continue;
            }

            let mut component: UndirectedComponent = [Vec::new(), Vec::new()];
            frontier.push((min_start, 0));
            while let Some((start, slot)) = frontier.pop() {
                if left_turn_map[start.as_usize()] < 0 {
                    continue;
                }

                let mut path: Vec<EdgeId> = Vec::new();
                let mut e = start;
                while left_turn_map[e.as_usize()] >= 0 {
                    path.push(e);
                    let next = left_turn_map[e.as_usize()];
                    left_turn_map[e.as_usize()] = mark_edge_used(slot);
                    let sibling = sibling_map[e.as_usize()];
                    if left_turn_map[sibling.as_usize()] >= 0 {
                        frontier.push((sibling, 1 - slot));
                    } else if left_turn_map[sibling.as_usize()] != mark_edge_used(1 - slot) {
                        *error = S2Error::new(
                            S2ErrorCode::BuilderEdgesDoNotFormLoops,
                            "Given undirected edges do not form loops",
                        );
                        return Vec::new();
                    }
                    if loop_type == LoopType::Simple {
                        path_index[self.edge(e).0.as_usize()] = path.len() as i32 - 1;
                        let loop_start = path_index[self.edge(e).1.as_usize()];
                        if loop_start >= 0 {
                            let mut peeled: Vec<EdgeId> = path[loop_start as usize..].to_vec();
                            path.truncate(loop_start as usize);
                            for &e2 in &peeled {
                                path_index[self.edge(e2).0.as_usize()] = -1;
                            }
                            Self::canonicalize_loop_order(&min_input_ids, &mut peeled);
                            component[slot as usize].push(peeled);
                        }
                    }
                    e = next;
                }
                if loop_type == LoopType::Simple {
                    debug_assert!(path.is_empty());
                } else {
                    Self::canonicalize_loop_order(&min_input_ids, &mut path);
                    component[slot as usize].push(path);
                }
            }
            Self::canonicalize_vector_order(&min_input_ids, &mut component[0]);
            Self::canonicalize_vector_order(&min_input_ids, &mut component[1]);
            // Swap so the complement whose first loop most closely follows
            // input edge ordering comes first.
            if !component[0].is_empty()
                && !component[1].is_empty()
                && min_input_ids[component[0][0][0].as_usize()]
                    > min_input_ids[component[1][0][0].as_usize()]
            {
                component.swap(0, 1);
            }
            components.push(component);
        }
        components.sort_unstable_by(|a, b| {
            min_input_ids[a[0][0][0].as_usize()].cmp(&min_input_ids[b[0][0][0].as_usize()])
        });
        components
    }

    // ─── Polyline building ──────────────────────────────────────────────

    /// Assembles edges into polylines. Returns edge-based polylines.
    pub fn get_polylines(&self, polyline_type: PolylineType) -> Vec<EdgePolyline> {
        debug_assert!(
            self.options.sibling_pairs == SiblingPairs::Discard
                || self.options.sibling_pairs == SiblingPairs::DiscardExcess
                || self.options.sibling_pairs == SiblingPairs::Keep
        );
        let mut builder = PolylineBuilder::new(self);
        match polyline_type {
            PolylineType::Path => builder.build_paths(),
            PolylineType::Walk => builder.build_walks(),
        }
    }

    // ─── FilterVertices ─────────────────────────────────────────────────

    /// Removes unused vertices and remaps edge indices. Returns the new
    /// minimal set of vertices.
    pub fn filter_vertices(vertices: &[Point], edges: &mut [Edge]) -> Vec<Point> {
        // Gather vertices actually used.
        let mut used: Vec<VertexId> = Vec::with_capacity(2 * edges.len());
        for &(v0, v1) in edges.iter() {
            used.push(v0);
            used.push(v1);
        }
        used.sort_unstable();
        used.dedup();

        // Build new vertices and mapping.
        let mut vmap = vec![VertexId(0); vertices.len()];
        let mut new_vertices = Vec::with_capacity(used.len());
        for (i, &old_v) in used.iter().enumerate() {
            new_vertices.push(vertices[old_v.as_usize()]);
            vmap[old_v.as_usize()] = VertexId(i as i32);
        }
        // Update edges.
        for edge in edges.iter_mut() {
            edge.0 = vmap[edge.0.as_usize()];
            edge.1 = vmap[edge.1.as_usize()];
        }
        new_vertices
    }

    /// Creates a subgraph with different `GraphOptions` from this graph's data.
    pub fn make_subgraph(
        &self,
        mut new_options: GraphOptions,
        new_edges: &mut Vec<Edge>,
        new_input_edge_id_set_ids: &mut Vec<InputEdgeIdSetId>,
        new_input_edge_id_set_lexicon: &mut IdSetLexicon,
        is_full_polygon_predicate: Option<IsFullPolygonPredicate>,
        error: &mut S2Error,
    ) -> Graph {
        // If converting directed → undirected, create reverse edges.
        if self.options.edge_type == EdgeType::Directed
            && new_options.edge_type == EdgeType::Undirected
        {
            let n = new_edges.len();
            for i in 0..n {
                new_edges.push(Self::reverse(new_edges[i]));
                new_input_edge_id_set_ids.push(EMPTY_SET_ID);
            }
        }
        Self::process_edges(
            &mut new_options,
            new_edges,
            new_input_edge_id_set_ids,
            new_input_edge_id_set_lexicon,
            error,
        );
        Graph::from_raw_parts(
            new_options,
            self.vertices.clone(),
            std::mem::take(new_edges),
            std::mem::take(new_input_edge_id_set_ids),
            std::mem::take(new_input_edge_id_set_lexicon),
            self.label_set_ids.clone(),
            self.label_set_lexicon.clone(),
            is_full_polygon_predicate,
        )
    }
}

// ─── AddVertexEdges helper ──────────────────────────────────────────────────

/// A struct for sorting incoming/outgoing edges around a vertex.
struct VertexEdge {
    incoming: bool,
    index: EdgeId,
    endpoint: VertexId,
    rank: i32,
}

/// Interleaves outgoing and incoming edges for consistent clockwise ordering.
fn add_vertex_edges(
    mut out_begin: EdgeId,
    out_end: EdgeId,
    in_begin: EdgeId,
    mut in_end: EdgeId,
    v1: VertexId,
    in_edge_ids: &[EdgeId],
    v0_edges: &mut Vec<VertexEdge>,
) {
    let mut rank = 0i32;
    // Extra incoming edges go at the beginning.
    while in_end - in_begin > out_end - out_begin {
        in_end -= 1;
        v0_edges.push(VertexEdge {
            incoming: true,
            index: in_edge_ids[in_end.as_usize()],
            endpoint: v1,
            rank,
        });
        rank += 1;
    }
    // Interleave outgoing and incoming edges.
    while in_end > in_begin {
        v0_edges.push(VertexEdge {
            incoming: false,
            index: out_begin,
            endpoint: v1,
            rank,
        });
        rank += 1;
        out_begin += 1;
        in_end -= 1;
        v0_edges.push(VertexEdge {
            incoming: true,
            index: in_edge_ids[in_end.as_usize()],
            endpoint: v1,
            rank,
        });
        rank += 1;
    }
    // Extra outgoing edges go at the end.
    while out_begin < out_end {
        v0_edges.push(VertexEdge {
            incoming: false,
            index: out_begin,
            endpoint: v1,
            rank,
        });
        rank += 1;
        out_begin += 1;
    }
}

// ─── EdgeProcessor (merge-join algorithm) ───────────────────────────────────

/// Implements the C++ `EdgeProcessor` merge-join algorithm for processing edges.
struct EdgeProcessor<'a> {
    options: &'a GraphOptions,
    edges: &'a mut Vec<Edge>,
    input_ids: &'a mut Vec<InputEdgeIdSetId>,
    id_set_lexicon: &'a mut IdSetLexicon,
    out_edges: Vec<EdgeId>,
    in_edges: Vec<EdgeId>,
    new_edges: Vec<Edge>,
    new_input_ids: Vec<InputEdgeIdSetId>,
    tmp_ids: Vec<i32>,
}

impl<'a> EdgeProcessor<'a> {
    fn new(
        options: &'a GraphOptions,
        edges: &'a mut Vec<Edge>,
        input_ids: &'a mut Vec<InputEdgeIdSetId>,
        id_set_lexicon: &'a mut IdSetLexicon,
    ) -> Self {
        let n = edges.len();
        let mut out_edges: Vec<EdgeId> = (0..n as i32).map(EdgeId).collect();
        out_edges.sort_unstable_by(|&a, &b| {
            if Graph::stable_less_than(edges[a.as_usize()], edges[b.as_usize()], a, b) {
                std::cmp::Ordering::Less
            } else if Graph::stable_less_than(edges[b.as_usize()], edges[a.as_usize()], b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let mut in_edges: Vec<EdgeId> = (0..n as i32).map(EdgeId).collect();
        in_edges.sort_unstable_by(|&a, &b| {
            let ra = Graph::reverse(edges[a.as_usize()]);
            let rb = Graph::reverse(edges[b.as_usize()]);
            if Graph::stable_less_than(ra, rb, a, b) {
                std::cmp::Ordering::Less
            } else if Graph::stable_less_than(rb, ra, b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        EdgeProcessor {
            options,
            edges,
            input_ids,
            id_set_lexicon,
            out_edges,
            in_edges,
            new_edges: Vec::with_capacity(n),
            new_input_ids: Vec::with_capacity(n),
            tmp_ids: Vec::new(),
        }
    }

    fn add_edge(&mut self, edge: Edge, input_edge_id_set_id: InputEdgeIdSetId) {
        self.new_edges.push(edge);
        self.new_input_ids.push(input_edge_id_set_id);
    }

    fn add_edges(&mut self, count: usize, edge: Edge, input_edge_id_set_id: InputEdgeIdSetId) {
        for _ in 0..count {
            self.add_edge(edge, input_edge_id_set_id);
        }
    }

    fn copy_edges(&mut self, out_begin: usize, out_end: usize) {
        for i in out_begin..out_end {
            let eidx = self.out_edges[i].as_usize();
            self.add_edge(self.edges[eidx], self.input_ids[eidx]);
        }
    }

    fn merge_input_ids(&mut self, out_begin: usize, out_end: usize) -> InputEdgeIdSetId {
        if out_end - out_begin == 1 {
            return self.input_ids[self.out_edges[out_begin].as_usize()];
        }
        self.tmp_ids.clear();
        for i in out_begin..out_end {
            let eidx = self.out_edges[i].as_usize();
            let ids = self.id_set_lexicon.id_set(self.input_ids[eidx]);
            self.tmp_ids.extend(ids);
        }
        self.id_set_lexicon.add_set(&self.tmp_ids)
    }

    fn run(&mut self, error: &mut S2Error) {
        let num_edges = self.edges.len();
        if num_edges == 0 {
            return;
        }

        let sentinel: Edge = (VertexId::MAX, VertexId::MAX);
        let mut out = 0usize;
        let mut inp = 0usize;

        loop {
            let out_edge = if out >= num_edges {
                sentinel
            } else {
                self.edges[self.out_edges[out].as_usize()]
            };
            let in_edge = if inp >= num_edges {
                sentinel
            } else {
                Graph::reverse(self.edges[self.in_edges[inp].as_usize()])
            };
            let edge = std::cmp::min(out_edge, in_edge);
            if edge == sentinel {
                break;
            }

            let out_begin = out;
            let in_begin = inp;
            while out < num_edges && self.edges[self.out_edges[out].as_usize()] == edge {
                out += 1;
            }
            while inp < num_edges
                && Graph::reverse(self.edges[self.in_edges[inp].as_usize()]) == edge
            {
                inp += 1;
            }
            let n_out = out - out_begin;
            let n_in = inp - in_begin;

            if edge.0 == edge.1 {
                // Degenerate edge.
                debug_assert_eq!(n_out, n_in);
                if self.options.degenerate_edges == DegenerateEdges::Discard {
                    continue;
                }
                if self.options.degenerate_edges == DegenerateEdges::DiscardExcess {
                    // Discard if there are adjacent non-degenerate edges.
                    let has_adjacent = (out_begin > 0
                        && self.edges[self.out_edges[out_begin - 1].as_usize()].0 == edge.0)
                        || (out < num_edges
                            && self.edges[self.out_edges[out].as_usize()].0 == edge.0)
                        || (in_begin > 0
                            && self.edges[self.in_edges[in_begin - 1].as_usize()].1 == edge.0)
                        || (inp < num_edges
                            && self.edges[self.in_edges[inp].as_usize()].1 == edge.0);
                    if has_adjacent {
                        continue;
                    }
                }

                let merge = self.options.duplicate_edges == DuplicateEdges::Merge
                    || self.options.degenerate_edges == DegenerateEdges::DiscardExcess;
                let merged_id = self.merge_input_ids(out_begin, out);

                if self.options.edge_type == EdgeType::Undirected
                    && (self.options.sibling_pairs == SiblingPairs::Require
                        || self.options.sibling_pairs == SiblingPairs::Create)
                {
                    debug_assert_eq!(n_out & 1, 0);
                    self.add_edges(if merge { 1 } else { n_out / 2 }, edge, merged_id);
                } else if merge {
                    let count = if self.options.edge_type == EdgeType::Undirected {
                        2
                    } else {
                        1
                    };
                    self.add_edges(count, edge, merged_id);
                } else if self.options.sibling_pairs == SiblingPairs::Discard
                    || self.options.sibling_pairs == SiblingPairs::DiscardExcess
                {
                    self.add_edges(n_out, edge, merged_id);
                } else {
                    self.copy_edges(out_begin, out);
                }
            } else if self.options.sibling_pairs == SiblingPairs::Keep {
                if n_out > 1 && self.options.duplicate_edges == DuplicateEdges::Merge {
                    let merged_id = self.merge_input_ids(out_begin, out);
                    self.add_edge(edge, merged_id);
                } else {
                    self.copy_edges(out_begin, out);
                }
            } else if self.options.sibling_pairs == SiblingPairs::Discard {
                if self.options.edge_type == EdgeType::Directed {
                    if n_out <= n_in {
                        continue;
                    }
                    let merged_id = self.merge_input_ids(out_begin, out);
                    let count = if self.options.duplicate_edges == DuplicateEdges::Merge {
                        1
                    } else {
                        n_out - n_in
                    };
                    self.add_edges(count, edge, merged_id);
                } else {
                    if (n_out & 1) == 0 {
                        continue;
                    }
                    let merged_id = self.merge_input_ids(out_begin, out);
                    self.add_edge(edge, merged_id);
                }
            } else if self.options.sibling_pairs == SiblingPairs::DiscardExcess {
                if self.options.edge_type == EdgeType::Directed {
                    if n_out < n_in {
                        continue;
                    }
                    let merged_id = self.merge_input_ids(out_begin, out);
                    let count = if self.options.duplicate_edges == DuplicateEdges::Merge {
                        1
                    } else {
                        std::cmp::max(1, n_out - n_in)
                    };
                    self.add_edges(count, edge, merged_id);
                } else {
                    let merged_id = self.merge_input_ids(out_begin, out);
                    let count = if (n_out & 1) != 0 { 1 } else { 2 };
                    self.add_edges(count, edge, merged_id);
                }
            } else {
                // REQUIRE or CREATE
                debug_assert!(
                    self.options.sibling_pairs == SiblingPairs::Require
                        || self.options.sibling_pairs == SiblingPairs::Create
                );
                if error.is_ok()
                    && self.options.sibling_pairs == SiblingPairs::Require
                    && (if self.options.edge_type == EdgeType::Directed {
                        n_out != n_in
                    } else {
                        (n_out & 1) != 0
                    })
                {
                    *error = S2Error::new(
                        S2ErrorCode::BuilderMissingExpectedSiblingEdges,
                        "Expected all input edges to have siblings, but some were missing",
                    );
                }
                if self.options.duplicate_edges == DuplicateEdges::Merge {
                    let merged_id = self.merge_input_ids(out_begin, out);
                    self.add_edge(edge, merged_id);
                } else if self.options.edge_type == EdgeType::Undirected {
                    let merged_id = self.merge_input_ids(out_begin, out);
                    self.add_edges(n_out.div_ceil(2), edge, merged_id);
                } else {
                    self.copy_edges(out_begin, out);
                    if n_in > n_out {
                        self.add_edges(n_in - n_out, edge, EMPTY_SET_ID);
                    }
                }
            }
        }

        std::mem::swap(self.edges, &mut self.new_edges);
        std::mem::swap(self.input_ids, &mut self.new_input_ids);
    }
}

// ─── PolylineBuilder ────────────────────────────────────────────────────────

/// Builds polylines from a graph's edges (C++ `PolylineBuilder` port).
struct PolylineBuilder<'a> {
    g: &'a Graph,
    in_map: VertexInMap,
    out_map: VertexOutMap,
    sibling_map: Vec<EdgeId>,
    min_input_ids: Vec<InputEdgeId>,
    directed: bool,
    edges_left: EdgeId,
    used: Vec<bool>,
    excess_used: BTreeMap<VertexId, i32>,
}

impl<'a> PolylineBuilder<'a> {
    fn new(g: &'a Graph) -> Self {
        let in_map = VertexInMap::new(g);
        let out_map = VertexOutMap::new(g);
        let min_input_ids = g.get_min_input_edge_ids();
        let directed = g.options().edge_type == EdgeType::Directed;
        let edges_left = g.num_edges() / if directed { 1 } else { 2 };
        let used = vec![false; g.num_edges().as_usize()];
        let sibling_map = if directed {
            Vec::new()
        } else {
            let in_ids = g.get_in_edge_ids();
            let mut smap = in_ids;
            g.make_sibling_map(&mut smap);
            smap
        };
        PolylineBuilder {
            g,
            in_map,
            out_map,
            sibling_map,
            min_input_ids,
            directed,
            edges_left,
            used,
            excess_used: BTreeMap::new(),
        }
    }

    fn is_interior(&self, v: VertexId) -> bool {
        if self.directed {
            self.in_map.degree(v) == 1 && self.out_map.degree(v) == 1
        } else {
            self.out_map.degree(v) == 2
        }
    }

    fn excess_degree(&self, v: VertexId) -> i32 {
        if self.directed {
            self.out_map.degree(v) as i32 - self.in_map.degree(v) as i32
        } else {
            (self.out_map.degree(v) % 2) as i32
        }
    }

    fn build_path(&mut self, start_e: EdgeId) -> EdgePolyline {
        let mut polyline = Vec::new();
        let start = self.g.edge(start_e).0;
        let mut e = start_e;
        loop {
            polyline.push(e);
            debug_assert!(!self.used[e.as_usize()]);
            self.used[e.as_usize()] = true;
            if !self.directed {
                self.used[self.sibling_map[e.as_usize()].as_usize()] = true;
            }
            self.edges_left -= 1;
            let v = self.g.edge(e).1;
            if !self.is_interior(v) || v == start {
                break;
            }
            if self.directed {
                debug_assert_eq!(self.out_map.degree(v), 1);
                e = self.out_map.edge_ids(v)[0];
            } else {
                debug_assert_eq!(self.out_map.degree(v), 2);
                for &e2 in self.out_map.edge_ids(v) {
                    if !self.used[e2.as_usize()] {
                        e = e2;
                    }
                }
            }
        }
        polyline
    }

    fn build_paths(&mut self) -> Vec<EdgePolyline> {
        let mut polylines = Vec::new();
        let edges = self.g.get_input_edge_order(&self.min_input_ids);

        // Build polylines from non-interior vertices.
        for &e in &edges {
            if !self.used[e.as_usize()] && !self.is_interior(self.g.edge(e).0) {
                polylines.push(self.build_path(e));
            }
        }
        // Build remaining loops.
        for &e in &edges {
            if self.edges_left == 0 {
                break;
            }
            if self.used[e.as_usize()] {
                continue;
            }
            let mut polyline = self.build_path(e);
            Graph::canonicalize_loop_order(&self.min_input_ids, &mut polyline);
            polylines.push(polyline);
        }
        debug_assert_eq!(self.edges_left, 0);
        debug_assert_eq!(self.edges_left, 0);
        Graph::canonicalize_vector_order(&self.min_input_ids, &mut polylines);
        polylines
    }

    fn build_walk(&mut self, v: VertexId) -> EdgePolyline {
        let mut polyline = Vec::new();
        let mut v = v;
        loop {
            // Follow edge with smallest input edge id.
            let mut best_edge = EdgeId(-1);
            let mut best_id = MAX_INPUT_EDGE_ID;
            for &e in self.out_map.edge_ids(v) {
                if self.used[e.as_usize()] || self.min_input_ids[e.as_usize()] >= best_id {
                    continue;
                }
                best_id = self.min_input_ids[e.as_usize()];
                best_edge = e;
            }
            if best_edge < 0 {
                return polyline;
            }
            // Stop early if best_edge might be a continuation of a different
            // incoming edge (for idempotency with multiple input polylines).
            let excess = self.excess_degree(v) - self.excess_used.get(&v).copied().unwrap_or(0);
            if if self.directed {
                excess < 0
            } else {
                (excess % 2) == 1
            } {
                let mut should_stop = false;
                for &e in self.in_map.edge_ids(v) {
                    if !self.used[e.as_usize()] && self.min_input_ids[e.as_usize()] <= best_id {
                        should_stop = true;
                        break;
                    }
                }
                if should_stop {
                    return polyline;
                }
            }
            polyline.push(best_edge);
            self.used[best_edge.as_usize()] = true;
            if !self.directed {
                self.used[self.sibling_map[best_edge.as_usize()].as_usize()] = true;
            }
            self.edges_left -= 1;
            v = self.g.edge(best_edge).1;
        }
    }

    fn maximize_walk(&mut self, polyline: &mut EdgePolyline) {
        let mut i = 0;
        while i <= polyline.len() {
            let v = if i == 0 {
                self.g.edge(polyline[i]).0
            } else {
                self.g.edge(polyline[i - 1]).1
            };
            let mut found = false;
            for &e in self.out_map.edge_ids(v) {
                if !self.used[e.as_usize()] {
                    let walk_loop = self.build_walk(v);
                    debug_assert_eq!(v, self.g.edge(walk_loop[walk_loop.len() - 1]).1);
                    let walk_len = walk_loop.len();
                    // Insert the loop into the polyline at position i.
                    polyline.splice(i..i, walk_loop);
                    i += walk_len;
                    debug_assert!(self.used[e.as_usize()]);
                    found = true;
                    break;
                }
            }
            if !found {
                i += 1;
            }
        }
    }

    fn build_walks(&mut self) -> Vec<EdgePolyline> {
        let mut polylines = Vec::new();
        let edges = self.g.get_input_edge_order(&self.min_input_ids);

        // Build polylines from vertices with excess degree.
        for &e in &edges {
            if self.used[e.as_usize()] {
                continue;
            }
            let v = self.g.edge(e).0;
            let excess = self.excess_degree(v);
            if excess <= 0 {
                continue;
            }
            let used_excess = self.excess_used.get(&v).copied().unwrap_or(0);
            if self.directed {
                if excess - used_excess <= 0 {
                    continue;
                }
            } else if (excess - used_excess) % 2 == 0 {
                continue;
            }
            *self.excess_used.entry(v).or_insert(0) += 1;
            let polyline = self.build_walk(v);
            if let Some(&last) = polyline.last() {
                let end_v = self.g.edge(last).1;
                *self.excess_used.entry(end_v).or_insert(0) -= 1;
            }
            polylines.push(polyline);
        }

        // Maximize existing polylines by adding loops.
        if self.edges_left > 0 {
            for polyline in &mut polylines {
                self.maximize_walk(polyline);
            }
        }

        // Build remaining loops.
        let mut i = 0;
        while i < edges.len() && self.edges_left > 0 {
            let e = edges[i];
            i += 1;
            if self.used[e.as_usize()] {
                continue;
            }
            // Check if this is the start of an edge chain.
            let v = self.g.edge(e).0;
            let id = self.min_input_ids[e.as_usize()];
            let mut excess = 0i32;
            let mut j = i - 1;
            while j < edges.len() && self.min_input_ids[edges[j].as_usize()] == id {
                let e2 = edges[j];
                if !self.used[e2.as_usize()] {
                    if self.g.edge(e2).0 == v {
                        excess += 1;
                    }
                    if self.g.edge(e2).1 == v {
                        excess -= 1;
                    }
                }
                j += 1;
            }
            if excess == 1 || self.g.edge(e).1 == v {
                let mut polyline = self.build_walk(v);
                self.maximize_walk(&mut polyline);
                polylines.push(polyline);
            }
        }

        Graph::canonicalize_vector_order(&self.min_input_ids, &mut polylines);
        polylines
    }
}

// ─── Helper types ───────────────────────────────────────────────────────────

/// Maps each vertex to its outgoing edge IDs using the sorted edge array.
#[derive(Debug)]
pub struct VertexOutMap {
    /// Pre-allocated vec of all `EdgeIds` [0, 1, 2, ..., num_edges-1].
    all_edge_ids: Vec<EdgeId>,
    edge_begins: Vec<EdgeId>,
}

impl VertexOutMap {
    /// Builds a vertex-to-outgoing-edge map for the given graph.
    pub fn new(g: &Graph) -> Self {
        let all_edge_ids: Vec<EdgeId> = (0..g.num_edges().0).map(EdgeId).collect();
        let mut edge_begins = Vec::with_capacity(g.num_vertices().as_usize() + 1);
        let mut e = EdgeId(0);
        for v in (0..=g.num_vertices().0).map(VertexId) {
            while e < g.num_edges() && g.edge(e).0 < v {
                e += 1;
            }
            edge_begins.push(e);
        }
        VertexOutMap {
            all_edge_ids,
            edge_begins,
        }
    }

    /// Returns the out-degree of vertex `v`.
    pub fn degree(&self, v: impl Into<VertexId>) -> usize {
        let v = v.into();
        (self.edge_begins[v.as_usize() + 1] - self.edge_begins[v.as_usize()]) as usize
    }

    /// Returns the outgoing edge IDs for vertex `v`.
    pub fn edge_ids(&self, v: impl Into<VertexId>) -> &[EdgeId] {
        let v = v.into();
        let begin = self.edge_begins[v.as_usize()].as_usize();
        let end = self.edge_begins[v.as_usize() + 1].as_usize();
        &self.all_edge_ids[begin..end]
    }

    /// Returns the edge IDs between a specific pair of vertices.
    /// Edges must be sorted by (v0, v1) which is the Graph invariant.
    pub fn edge_ids_between(&self, v0: VertexId, v1: VertexId, edges: &[Edge]) -> &[EdgeId] {
        let begin = self.edge_begins[v0.as_usize()].as_usize();
        let end = self.edge_begins[v0.as_usize() + 1].as_usize();
        // Binary search for the range of edges with destination v1.
        // Within the outgoing edges of v0, edges are sorted by destination.
        let slice = &edges[begin..end];
        let lo = slice.partition_point(|e| e.1 < v1);
        let hi = lo + slice[lo..].partition_point(|e| e.1 <= v1);
        &self.all_edge_ids[begin + lo..begin + hi]
    }
}

/// Maps each vertex to its incoming edge IDs.
#[derive(Debug)]
pub struct VertexInMap {
    in_edge_ids: Vec<EdgeId>,
    in_edge_begins: Vec<EdgeId>,
}

impl VertexInMap {
    /// Builds a vertex-to-incoming-edge map for the given graph.
    pub fn new(g: &Graph) -> Self {
        let in_edge_ids = g.get_in_edge_ids();
        let mut in_edge_begins = Vec::with_capacity(g.num_vertices().as_usize() + 1);
        let mut e = EdgeId(0);
        for v in (0..=g.num_vertices().0).map(VertexId) {
            while e < g.num_edges() && g.edge(in_edge_ids[e.as_usize()]).1 < v {
                e += 1;
            }
            in_edge_begins.push(e);
        }
        VertexInMap {
            in_edge_ids,
            in_edge_begins,
        }
    }

    /// Returns the in-degree of vertex `v`.
    pub fn degree(&self, v: impl Into<VertexId>) -> usize {
        let v = v.into();
        (self.in_edge_begins[v.as_usize() + 1] - self.in_edge_begins[v.as_usize()]) as usize
    }

    /// Returns the incoming edge IDs for vertex `v`.
    pub fn edge_ids(&self, v: impl Into<VertexId>) -> &[EdgeId] {
        let v = v.into();
        let begin = self.in_edge_begins[v.as_usize()].as_usize();
        let end = self.in_edge_begins[v.as_usize() + 1].as_usize();
        &self.in_edge_ids[begin..end]
    }

    /// Returns the full array of incoming edge IDs.
    pub fn in_edge_ids(&self) -> &[EdgeId] {
        &self.in_edge_ids
    }
}

/// Fetches labels for edges, handling undirected edge sibling labels.
#[derive(Debug)]
pub struct LabelFetcher {
    edge_type: EdgeType,
    sibling_map: Vec<EdgeId>,
}

impl LabelFetcher {
    /// Creates a new label fetcher for the given graph and edge type.
    /// For undirected edges, labels from sibling edges are merged.
    pub fn new(graph: &Graph, edge_type: EdgeType) -> Self {
        let sibling_map = if edge_type == EdgeType::Undirected {
            graph.get_sibling_map()
        } else {
            Vec::new()
        };
        LabelFetcher {
            edge_type,
            sibling_map,
        }
    }

    /// Returns the deduplicated, sorted labels for graph edge `e`.
    pub fn fetch(&self, graph: &Graph, e: impl Into<EdgeId>) -> Vec<Label> {
        let e = e.into();
        let mut labels = Vec::new();
        for input_edge_id in graph.input_edge_ids(e) {
            labels.extend(graph.labels(input_edge_id));
        }
        if self.edge_type == EdgeType::Undirected {
            for input_edge_id in graph.input_edge_ids(self.sibling_map[e.as_usize()]) {
                labels.extend(graph.labels(input_edge_id));
            }
        }
        if labels.len() > 1 {
            labels.sort_unstable();
            labels.dedup();
        }
        labels
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;
    use crate::s2::Point;
    use quickcheck_macros::quickcheck;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point::from_coords(x, y, z)
    }

    /// Convert `(i32, i32)` slice to `Vec<Edge>` for test convenience.
    fn edges(e: &[(i32, i32)]) -> Vec<Edge> {
        e.iter().map(|&(a, b)| (VertexId(a), VertexId(b))).collect()
    }

    #[test]
    fn test_graph_options_default() {
        let opts = GraphOptions::default();
        assert_eq!(opts.edge_type, EdgeType::Directed);
        assert_eq!(opts.degenerate_edges, DegenerateEdges::Keep);
        assert_eq!(opts.duplicate_edges, DuplicateEdges::Keep);
        assert_eq!(opts.sibling_pairs, SiblingPairs::Keep);
        assert!(opts.allow_vertex_filtering);
    }

    #[test]
    fn test_graph_basic_accessors() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);
        let id1 = lexicon.add_set(&[1]);

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2)]),
            vec![id0, id1],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        assert_eq!(g.num_vertices(), VertexId(3));
        assert_eq!(g.num_edges(), EdgeId(2));
        assert_eq!(g.edge(0), (VertexId(0), VertexId(1)));
        assert_eq!(g.edge(1), (VertexId(1), VertexId(2)));
    }

    #[test]
    fn test_graph_discard_degenerate() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);
        let id1 = lexicon.add_set(&[1]);
        let id2 = lexicon.add_set(&[2]);

        let g = Graph::new(
            opts,
            vec![v0, v1],
            edges(&[(0, 0), (0, 1), (1, 1)]),
            vec![id0, id1, id2],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // Only the non-degenerate edge should remain.
        assert_eq!(g.num_edges(), EdgeId(1));
        assert_eq!(g.edge(0), (VertexId(0), VertexId(1)));
    }

    #[test]
    fn test_graph_merge_duplicates() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);
        let id1 = lexicon.add_set(&[1]);

        let g = Graph::new(
            opts,
            vec![v0, v1],
            edges(&[(0, 1), (0, 1)]),
            vec![id0, id1],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // Duplicates merged into one edge.
        assert_eq!(g.num_edges(), EdgeId(1));
        assert_eq!(g.edge(0), (VertexId(0), VertexId(1)));
    }

    #[test]
    fn test_graph_directed_loop() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Discard,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2), (2, 0)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let mut err = S2Error::ok();
        let loops = g.get_directed_loops(LoopType::Circuit, &mut err);
        assert!(err.is_ok());
        assert_eq!(loops.len(), 1);
        assert_eq!(loops[0].len(), 3);
    }

    #[test]
    fn test_graph_polylines() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..2).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            opts,
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let polylines = g.get_polylines(PolylineType::Path);
        assert_eq!(polylines.len(), 1);
        assert_eq!(polylines[0].len(), 2); // 2 edges
        // Verify vertices: edge 0 is (0,1), edge 1 is (1,2)
        assert_eq!(g.edge(polylines[0][0]).0, 0);
        assert_eq!(g.edge(polylines[0][1]).1, 2);
    }

    #[test]
    fn test_get_in_edge_ids() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2), (2, 0)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let in_ids = g.get_in_edge_ids();
        // Edges: (0,1), (1,2), (2,0)
        // Reversed: (1,0), (2,1), (0,2)
        // Sorted by reversed: (0,2)=edge2, (1,0)=edge0, (2,1)=edge1
        assert_eq!(in_ids.len(), 3);
        // Edge 2 (reversed=(0,2)) should come first
        assert_eq!(g.edge(in_ids[0]), (VertexId(2), VertexId(0))); // reverse = (0,2)
        assert_eq!(g.edge(in_ids[1]), (VertexId(0), VertexId(1))); // reverse = (1,0)
        assert_eq!(g.edge(in_ids[2]), (VertexId(1), VertexId(2))); // reverse = (2,1)
    }

    #[test]
    fn test_get_sibling_map_with_siblings() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            opts,
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2), (2, 0)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // After Create, each edge has a sibling.
        let sibling_map = g.get_sibling_map();
        for e in (0..g.num_edges().0).map(EdgeId) {
            let sibling = sibling_map[e.as_usize()];
            assert!(sibling >= 0);
            assert_eq!(g.edge(e), Graph::reverse(g.edge(sibling)));
            assert_eq!(sibling_map[sibling.as_usize()], e);
        }
    }

    #[test]
    fn test_get_input_edge_order() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[5]); // input edge 5
        let id1 = lexicon.add_set(&[2]); // input edge 2
        let id2 = lexicon.add_set(&[8]); // input edge 8

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (0, 2), (1, 2)]),
            vec![id0, id1, id2],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let min_ids = g.get_min_input_edge_ids();
        let order = g.get_input_edge_order(&min_ids);
        // min_ids = [5, 2, 8], so sorted order should be [1, 0, 2]
        assert_eq!(order[0], 1); // input edge 2
        assert_eq!(order[1], 0); // input edge 5
        assert_eq!(order[2], 2); // input edge 8
    }

    #[test]
    fn test_canonicalize_loop_order() {
        let min_ids: Vec<InputEdgeId> = vec![7, 7, 4, 5, 6, 7]
            .into_iter()
            .map(InputEdgeId)
            .collect();
        let mut loop_edges: Vec<EdgeId> = (0..6).map(EdgeId).collect();
        Graph::canonicalize_loop_order(&min_ids, &mut loop_edges);
        // After canonicalization: [2, 3, 4, 5, 0, 1] (edges with id=4 first)
        assert_eq!(min_ids[loop_edges[0].as_usize()], 4);
    }

    #[test]
    fn test_filter_vertices() {
        let vertices = vec![
            p(1.0, 0.0, 0.0),
            p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0),
            p(0.5, 0.5, 0.0), // unused
        ];
        let mut edges: Vec<Edge> = edges(&[(0, 2), (2, 1)]);
        let new_verts = Graph::filter_vertices(&vertices, &mut edges);
        assert_eq!(new_verts.len(), 3); // only 3 vertices used
        // Edges should be remapped
        assert!(edges[0].0 < 3 && edges[0].1 < 3);
        assert!(edges[1].0 < 3 && edges[1].1 < 3);
    }

    #[test]
    fn test_stable_less_than() {
        assert!(Graph::stable_less_than(
            (VertexId(0), VertexId(1)),
            (VertexId(0), VertexId(2)),
            EdgeId(0),
            EdgeId(0)
        ));
        assert!(Graph::stable_less_than(
            (VertexId(0), VertexId(1)),
            (VertexId(1), VertexId(0)),
            EdgeId(0),
            EdgeId(0)
        ));
        assert!(!Graph::stable_less_than(
            (VertexId(0), VertexId(1)),
            (VertexId(0), VertexId(1)),
            EdgeId(1),
            EdgeId(0)
        ));
        assert!(Graph::stable_less_than(
            (VertexId(0), VertexId(1)),
            (VertexId(0), VertexId(1)),
            EdgeId(0),
            EdgeId(1)
        )); // tie broken by id
    }

    #[test]
    fn test_vertex_out_map_compact() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (0, 2), (1, 2)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let out_map = g.get_vertex_out_map();
        assert_eq!(out_map.degree(0), 2); // edges (0,1) and (0,2)
        assert_eq!(out_map.degree(1), 1); // edge (1,2)
        assert_eq!(out_map.degree(2), 0); // no outgoing edges
    }

    #[test]
    fn test_vertex_in_map_compact() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (0, 2), (1, 2)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let in_map = g.get_vertex_in_map();
        assert_eq!(in_map.degree(0), 0); // no incoming edges
        assert_eq!(in_map.degree(1), 1); // edge (0,1)
        assert_eq!(in_map.degree(2), 2); // edges (0,2) and (1,2)
    }

    #[test]
    fn test_label_fetcher_undirected() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);

        let mut label_lexicon = IdSetLexicon::new();
        let label_id = label_lexicon.add_set(&[42]);

        let g = Graph::new(
            opts,
            vec![v0, v1],
            edges(&[(0, 1)]),
            vec![id0],
            lexicon,
            vec![label_id],
            label_lexicon,
            None,
        );

        let fetcher = LabelFetcher::new(&g, EdgeType::Undirected);
        // The original edge (0,1) has label 42.
        // With undirected fetcher, both sibling edges should have label 42.
        for e in (0..g.num_edges().0).map(EdgeId) {
            let labels = fetcher.fetch(&g, e);
            assert!(labels.contains(&42), "edge {e} missing label 42");
        }
    }

    #[test]
    fn test_directed_components_simple() {
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let g = Graph::new(
            opts,
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2), (2, 0)]),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let mut err = S2Error::ok();
        let components = g.get_directed_components(DegenerateBoundaries::Keep, &mut err);
        assert!(err.is_ok());
        // A triangle with siblings creates one component with loops.
        assert!(!components.is_empty());
    }

    // ─── Property tests ─────────────────────────────────────────────────

    /// Helper: build a graph from a list of directed edges (v0, v1) where
    /// vertex IDs are in `0..num_vertices`. Returns None if input is empty.
    fn make_graph_from_edges(
        num_verts: usize,
        raw_edges: &[(i32, i32)],
        opts: GraphOptions,
    ) -> Option<Graph> {
        if num_verts == 0 || raw_edges.is_empty() {
            return None;
        }
        let vertices: Vec<Point> = (0..num_verts)
            .map(|i| {
                let angle = 2.0 * std::f64::consts::PI * (i as f64) / (num_verts as f64);
                Point::from_coords(angle.cos(), angle.sin(), 0.0)
            })
            .collect();

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..raw_edges.len())
            .map(|i| lexicon.add_set(&[i as i32]))
            .collect();

        Some(Graph::new(
            opts,
            vertices,
            edges(raw_edges),
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        ))
    }

    /// Discarding degenerate edges removes all (v, v) edges.
    #[quickcheck]
    fn prop_discard_degenerate_removes_self_loops(edge_data: Vec<u8>) -> bool {
        let num_verts = 5;
        let edges: Vec<(i32, i32)> = edge_data
            .chunks(2)
            .take(10)
            .map(|chunk| {
                let v0 = i32::from(chunk[0] % num_verts as u8);
                let v1 = if chunk.len() > 1 {
                    i32::from(chunk[1] % num_verts as u8)
                } else {
                    v0
                };
                (v0, v1)
            })
            .collect();

        if edges.is_empty() {
            return true;
        }

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );

        if let Some(g) = make_graph_from_edges(num_verts, &edges, opts) {
            for eid in (0..g.num_edges().0).map(EdgeId) {
                let (v0, v1) = g.edge(eid);
                if v0 == v1 {
                    return false;
                }
            }
        }
        true
    }

    /// Merging duplicate edges leaves at most one copy of each (v0, v1).
    #[quickcheck]
    fn prop_merge_duplicates_unique(edge_data: Vec<u8>) -> bool {
        let num_verts = 4;
        let edges: Vec<(i32, i32)> = edge_data
            .chunks(2)
            .take(10)
            .map(|chunk| {
                let v0 = i32::from(chunk[0] % num_verts as u8);
                let v1 = if chunk.len() > 1 {
                    i32::from(chunk[1] % num_verts as u8)
                } else {
                    0
                };
                (v0, v1)
            })
            .collect();

        if edges.is_empty() {
            return true;
        }

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );

        if let Some(g) = make_graph_from_edges(num_verts, &edges, opts) {
            let mut seen = std::collections::HashSet::new();
            for eid in (0..g.num_edges().0).map(EdgeId) {
                let e = g.edge(eid);
                if !seen.insert(e) {
                    return false;
                }
            }
        }
        true
    }

    /// Edge count after `process_edges` is never more than input edge count.
    #[quickcheck]
    fn prop_process_edges_never_increases_beyond_input(edge_data: Vec<u8>) -> bool {
        let num_verts = 4;
        let edges: Vec<(i32, i32)> = edge_data
            .chunks(2)
            .take(8)
            .map(|chunk| {
                let v0 = i32::from(chunk[0] % num_verts as u8);
                let v1 = if chunk.len() > 1 {
                    i32::from(chunk[1] % num_verts as u8)
                } else {
                    0
                };
                (v0, v1)
            })
            .collect();

        if edges.is_empty() {
            return true;
        }

        let input_count = edges.len();

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );

        if let Some(g) = make_graph_from_edges(num_verts, &edges, opts)
            && g.num_edges() > input_count as i32
        {
            return false;
        }
        true
    }

    /// A directed cycle of N vertices produces exactly one loop of length N.
    #[quickcheck]
    fn prop_directed_cycle_one_loop(n: u8) -> bool {
        let n = i32::from(n % 20) + 3;
        let num_verts = n as usize;
        let edges: Vec<(i32, i32)> = (0..n).map(|i| (i, (i + 1) % n)).collect();

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        if let Some(g) = make_graph_from_edges(num_verts, &edges, opts) {
            let mut err = S2Error::ok();
            let loops = g.get_directed_loops(LoopType::Circuit, &mut err);
            err.is_ok() && loops.len() == 1 && loops[0].len() == num_verts
        } else {
            false
        }
    }

    /// A chain of N edges produces exactly one polyline of N edges.
    #[quickcheck]
    fn prop_chain_one_polyline(n: u8) -> bool {
        let n = i32::from(n % 20) + 1;
        let num_verts = (n + 1) as usize;
        let edges: Vec<(i32, i32)> = (0..n).map(|i| (i, i + 1)).collect();

        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        if let Some(g) = make_graph_from_edges(num_verts, &edges, opts) {
            let polylines = g.get_polylines(PolylineType::Path);
            polylines.len() == 1 && polylines[0].len() == n as usize
        } else {
            false
        }
    }

    /// Graph vertex count is always the number of vertices passed in.
    #[quickcheck]
    fn prop_vertex_count_preserved(n: u8) -> bool {
        let n = (n % 50) as usize + 1;
        let vertices: Vec<Point> = (0..n)
            .map(|i| {
                let a = 2.0 * std::f64::consts::PI * (i as f64) / (n as f64);
                Point::from_coords(a.cos(), a.sin(), 0.0)
            })
            .collect();

        let g = Graph::new(
            GraphOptions::default(),
            vertices,
            vec![],
            vec![],
            IdSetLexicon::new(),
            vec![],
            IdSetLexicon::new(),
            None,
        );
        g.num_vertices() == n as i32
    }

    #[test]
    fn test_graph_vertices_accessor() {
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();

        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);
        let id1 = lexicon.add_set(&[1]);

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (1, 2)]),
            vec![id0, id1],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let verts = g.vertices();
        assert_eq!(verts.len(), 3);
        assert_eq!(verts[0], v0);
        assert_eq!(verts[1], v1);
        assert_eq!(verts[2], v2);
        // Verify consistency with individual vertex accessor.
        for i in (0..g.num_vertices().0).map(VertexId) {
            assert_eq!(g.vertex(i), g.vertices()[i.as_usize()]);
        }
    }

    #[test]
    fn test_graph_get_undirected_components_simple() {
        // Build a graph with undirected edges forming a triangle.
        // Undirected means every edge appears with its sibling.
        // Vertices on a unit circle for well-defined geometry.
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();

        let opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );

        // For undirected graphs, each logical edge is stored as two directed
        // edges (e and reverse(e)). Supply 6 edges for a triangle.
        let edges: Vec<Edge> = edges(&[(0, 1), (1, 0), (1, 2), (2, 1), (2, 0), (0, 2)]);
        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..edges.len())
            .map(|i| lexicon.add_set(&[i as i32]))
            .collect();

        let g = Graph::new(
            opts,
            vec![v0, v1, v2],
            edges,
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let mut err = S2Error::ok();
        let components = g.get_undirected_components(LoopType::Simple, &mut err);
        assert!(err.is_ok(), "error: {err}");
        // A single undirected triangle should produce exactly one component.
        assert_eq!(components.len(), 1);
        // Each component has two complements. At least one should be non-empty.
        assert!(
            !components[0][0].is_empty() || !components[0][1].is_empty(),
            "both complements are empty"
        );
        // Each loop in each complement should have length 3 (triangle).
        for (slot, complement) in components[0].iter().enumerate() {
            for lp in complement {
                assert_eq!(lp.len(), 3, "loop in slot {slot} has wrong length");
            }
        }
    }

    #[test]
    fn test_graph_get_directed_components_discard_degenerate() {
        // Build a triangle with sibling pairs, plus a degenerate filament
        // (edge + reverse that forms a back-and-forth spike).
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();
        let v3 = Point::from_coords(1.0, 1.0, 0.0).normalize(); // spike vertex

        // Triangle edges with siblings, plus a filament spike from v0 -> v3 -> v0.
        let opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );

        let edges: Vec<Edge> = edges(&[
            (0, 1),
            (1, 2),
            (2, 0),
            (0, 3),
            (3, 0), // filament spike
        ]);
        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..edges.len())
            .map(|i| lexicon.add_set(&[i as i32]))
            .collect();

        let g = Graph::new(
            opts,
            vec![v0, v1, v2, v3],
            edges,
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // With DegenerateBoundaries::Discard, the filament should be stripped.
        let mut err = S2Error::ok();
        let components_discard = g.get_directed_components(DegenerateBoundaries::Discard, &mut err);
        assert!(err.is_ok(), "error: {err}");

        // Count total edges across all components (Discard mode).
        let total_edges_discard: usize = components_discard
            .iter()
            .flat_map(|c| c.iter())
            .map(Vec::len)
            .sum();

        // With DegenerateBoundaries::Keep, the filament edges should be present.
        let mut err2 = S2Error::ok();
        let components_keep = g.get_directed_components(DegenerateBoundaries::Keep, &mut err2);
        assert!(err2.is_ok(), "error: {err2}");

        let total_edges_keep: usize = components_keep
            .iter()
            .flat_map(|c| c.iter())
            .map(Vec::len)
            .sum();

        // Discarding degenerate boundaries should yield fewer or equal total edges.
        assert!(
            total_edges_discard <= total_edges_keep,
            "discard ({total_edges_discard}) > keep ({total_edges_keep})"
        );
    }

    #[test]
    fn test_graph_is_full_polygon_error_branches() {
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();

        // Case 1: Predicate returns Err.
        {
            let pred: IsFullPolygonPredicate = std::sync::Arc::new(|_g: &Graph| {
                Err(S2Error::new(
                    S2ErrorCode::Internal,
                    "test error from predicate",
                ))
            });
            let mut lexicon = IdSetLexicon::new();
            let id0 = lexicon.add_set(&[0]);
            let g = Graph::new(
                GraphOptions::default(),
                vec![v0, v1],
                edges(&[(0, 1)]),
                vec![id0],
                lexicon,
                vec![],
                IdSetLexicon::new(),
                Some(pred),
            );
            let result = g.is_full_polygon();
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.code, S2ErrorCode::Internal);
            assert!(err.message.contains("test error from predicate"));
        }

        // Case 2: No predicate set, but edges exist.
        {
            let mut lexicon = IdSetLexicon::new();
            let id0 = lexicon.add_set(&[0]);
            let g = Graph::new(
                GraphOptions::default(),
                vec![v0, v1],
                edges(&[(0, 1)]),
                vec![id0],
                lexicon,
                vec![],
                IdSetLexicon::new(),
                None,
            );
            let result = g.is_full_polygon();
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err().code,
                S2ErrorCode::BuilderIsFullPredicateNotSpecified
            );
        }

        // Case 3: No predicate, no edges -- returns false without error.
        {
            let g = Graph::new(
                GraphOptions::default(),
                vec![v0],
                vec![],
                vec![],
                IdSetLexicon::new(),
                vec![],
                IdSetLexicon::new(),
                None,
            );
            let result = g.is_full_polygon();
            assert_eq!(result, Ok(false));
        }
    }

    #[test]
    fn test_graph_label_accessors() {
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();

        let mut input_lexicon = IdSetLexicon::new();
        let input_id0 = input_lexicon.add_set(&[0]);
        let input_id1 = input_lexicon.add_set(&[1]);
        let input_id2 = input_lexicon.add_set(&[2]);

        // Set up labels for 3 input edges.
        let mut label_lexicon = IdSetLexicon::new();
        let label_set_0 = label_lexicon.add_set(&[10, 20]); // input edge 0 has labels 10, 20
        let label_set_1 = label_lexicon.add_set(&[30]); // input edge 1 has label 30
        let label_set_2 = label_lexicon.add_set(&[]); // input edge 2 has no labels

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (0, 2), (1, 2)]),
            vec![input_id0, input_id1, input_id2],
            input_lexicon,
            vec![label_set_0, label_set_1, label_set_2],
            label_lexicon,
            None,
        );

        // Test labels() accessor.
        let labels_0 = g.labels(0);
        assert!(labels_0.contains(&10));
        assert!(labels_0.contains(&20));
        assert_eq!(labels_0.len(), 2);

        let labels_1 = g.labels(1);
        assert_eq!(labels_1, vec![30]);

        let labels_2 = g.labels(2);
        assert!(labels_2.is_empty());

        // Out-of-range input edge returns empty.
        let labels_oob = g.labels(999);
        assert!(labels_oob.is_empty());

        // Test label_set_ids() accessor.
        let lsids = g.label_set_ids();
        assert_eq!(lsids.len(), 3);
        assert_eq!(lsids[0], label_set_0);
        assert_eq!(lsids[1], label_set_1);
        assert_eq!(lsids[2], label_set_2);

        // Test label_set_lexicon() accessor.
        let lex = g.label_set_lexicon();
        let set0 = lex.id_set(label_set_0);
        assert_eq!(set0, vec![10, 20]);
    }

    #[test]
    fn test_graph_input_edge_id_set_accessors() {
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();

        let mut lexicon = IdSetLexicon::new();
        let set_a = lexicon.add_set(&[100]);
        let set_b = lexicon.add_set(&[200, 201]);
        let set_c = lexicon.add_set(&[300]);

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2],
            edges(&[(0, 1), (0, 2), (1, 2)]),
            vec![set_a, set_b, set_c],
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // Test input_edge_id_set_id() for each edge.
        // After process_edges (which sorts), edges remain in order since
        // (0,1) < (0,2) < (1,2) lexicographically.
        assert_eq!(g.input_edge_id_set_id(0), set_a);
        assert_eq!(g.input_edge_id_set_id(1), set_b);
        assert_eq!(g.input_edge_id_set_id(2), set_c);

        // Test input_edge_id_set_ids() slice.
        let all_ids = g.input_edge_id_set_ids();
        assert_eq!(all_ids.len(), 3);
        assert_eq!(all_ids[0], set_a);
        assert_eq!(all_ids[1], set_b);
        assert_eq!(all_ids[2], set_c);

        // Test input_edge_id_set_lexicon() by decoding sets.
        let lex = g.input_edge_id_set_lexicon();
        assert_eq!(lex.id_set(set_a), vec![100]);
        assert_eq!(lex.id_set(set_b), vec![200, 201]);
        assert_eq!(lex.id_set(set_c), vec![300]);

        // Test input_edge_ids() convenience method.
        assert_eq!(g.input_edge_ids(0), vec![100]);
        assert_eq!(g.input_edge_ids(1), vec![200, 201]);
        assert_eq!(g.input_edge_ids(2), vec![300]);
    }

    #[test]
    fn test_graph_get_in_edge_ids_stable_sort() {
        // Create edges where multiple edges share the same reversed (dest, src)
        // to exercise the stable sort comparator in get_in_edge_ids.
        //
        // Edges: (0,2), (1,2), (3,2) all have destination=2, so reversed
        // they are (2,0), (2,1), (2,3). Additionally add (0,1) and (3,1)
        // so reversed = (1,0), (1,3) which share destination=1.
        let v0 = Point::from_coords(1.0, 0.0, 0.0).normalize();
        let v1 = Point::from_coords(0.0, 1.0, 0.0).normalize();
        let v2 = Point::from_coords(0.0, 0.0, 1.0).normalize();
        let v3 = Point::from_coords(1.0, 1.0, 1.0).normalize();

        let edges: Vec<Edge> = edges(&[
            (0, 1), // edge 0, reversed = (1, 0)
            (0, 2), // edge 1, reversed = (2, 0)
            (1, 2), // edge 2, reversed = (2, 1)
            (3, 1), // edge 3, reversed = (1, 3)
            (3, 2), // edge 4, reversed = (2, 3)
        ]);
        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..edges.len())
            .map(|i| lexicon.add_set(&[i as i32]))
            .collect();

        let g = Graph::new(
            GraphOptions::default(),
            vec![v0, v1, v2, v3],
            edges,
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let in_ids = g.get_in_edge_ids();
        assert_eq!(in_ids.len(), g.num_edges().as_usize());

        // Verify that in_ids is sorted by reversed edge (dest, src).
        for i in 1..in_ids.len() {
            let prev = Graph::reverse(g.edge(in_ids[i - 1]));
            let curr = Graph::reverse(g.edge(in_ids[i]));
            assert!(
                prev <= curr,
                "in_edge_ids not sorted at index {i}: reversed edges {prev:?} > {curr:?}"
            );
            // When reversed edges are equal, edge IDs should be in ascending order (stable).
            if prev == curr {
                assert!(
                    in_ids[i - 1] < in_ids[i],
                    "stable sort violated at index {i}: edge ids {} >= {}",
                    in_ids[i - 1],
                    in_ids[i]
                );
            }
        }

        // Verify all edges appear exactly once.
        let mut sorted_ids = in_ids.clone();
        sorted_ids.sort_unstable();
        let expected: Vec<EdgeId> = (0..g.num_edges().0).map(EdgeId).collect();
        assert_eq!(sorted_ids, expected, "in_edge_ids is not a permutation");
    }

    // ─── ProcessEdges tests ─────────────────────────────────────────────

    /// Test helper: calls `process_edges` and compares output edges and their
    /// input IDs against expected values.
    fn test_process_edges(
        input: &[(i32, i32, &[i32])],
        expected: &[(i32, i32, &[i32])],
        options: &mut GraphOptions,
        expected_code: S2ErrorCode,
    ) {
        let mut lexicon = IdSetLexicon::new();
        let mut edges: Vec<Edge> = Vec::new();
        let mut input_ids: Vec<InputEdgeIdSetId> = Vec::new();

        for &(v0, v1, ids) in input {
            edges.push((VertexId(v0), VertexId(v1)));
            input_ids.push(lexicon.add_set(ids));
        }

        let mut error = S2Error::ok();
        Graph::process_edges(
            options,
            &mut edges,
            &mut input_ids,
            &mut lexicon,
            &mut error,
        );

        assert_eq!(
            error.code, expected_code,
            "expected error {expected_code:?}, got {:?}",
            error.code
        );
        assert_eq!(
            edges.len(),
            expected.len(),
            "edge count mismatch: got {edges:?}, expected {expected:?}"
        );

        for (i, &(ev0, ev1, eids)) in expected.iter().enumerate() {
            assert_eq!(
                edges[i],
                (VertexId(ev0), VertexId(ev1)),
                "edge {i}: got {:?}, expected ({ev0}, {ev1})",
                edges[i]
            );
            let actual_ids: Vec<i32> = lexicon.id_set(input_ids[i]).clone();
            if !eids.is_empty() {
                assert_eq!(
                    actual_ids, eids,
                    "edge {i} input IDs: got {actual_ids:?}, expected {eids:?}"
                );
            }
        }
    }

    // --- Degenerate edge tests ---

    #[test]
    fn test_pe_discard_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        test_process_edges(&[(0, 0, &[]), (0, 0, &[])], &[], &mut opts, S2ErrorCode::Ok);
    }

    #[test]
    fn test_pe_keep_duplicate_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 0, &[]), (0, 0, &[])],
            &[(0, 0, &[]), (0, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_merge_duplicate_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[2])],
            &[(0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_merge_undirected_duplicate_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[]), (0, 0, &[]), (0, 0, &[2])],
            &[(0, 0, &[1, 2]), (0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_converted_undirected_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Require,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[]), (0, 0, &[]), (0, 0, &[2])],
            &[(0, 0, &[1, 2]), (0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_merge_converted_undirected_duplicate_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Merge,
            SiblingPairs::Require,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[]), (0, 0, &[]), (0, 0, &[2])],
            &[(0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    // --- DiscardExcess degenerate tests ---

    #[test]
    fn test_pe_discard_excess_connected_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        // Degenerate edge at same vertex as non-degenerate: discard
        test_process_edges(
            &[(0, 0, &[]), (0, 1, &[])],
            &[(0, 1, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 0, &[]), (1, 0, &[])],
            &[(1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        let mut opts3 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (1, 1, &[])],
            &[(0, 1, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
        let mut opts4 = opts.clone();
        test_process_edges(
            &[(1, 0, &[]), (1, 1, &[])],
            &[(1, 0, &[])],
            &mut opts4,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_isolated_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[2])],
            &[(0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_undirected_isolated_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[]), (0, 0, &[2]), (0, 0, &[])],
            &[(0, 0, &[1, 2]), (0, 0, &[1, 2])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_converted_undirected_isolated_degenerate_edges() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Keep,
            SiblingPairs::Require,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[2]), (0, 0, &[3]), (0, 0, &[])],
            &[(0, 0, &[1, 2, 3])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_sibling_pairs_discard_merges_degenerate_edge_labels() {
        // SiblingPairs::DISCARD with degenerate edges
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Discard,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[2]), (0, 0, &[3])],
            &[(0, 0, &[1, 2, 3]), (0, 0, &[1, 2, 3]), (0, 0, &[1, 2, 3])],
            &mut opts,
            S2ErrorCode::Ok,
        );

        // SiblingPairs::DISCARD_EXCESS with degenerate edges
        let mut opts2 = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::DiscardExcess,
        );
        test_process_edges(
            &[(0, 0, &[1]), (0, 0, &[2]), (0, 0, &[3])],
            &[(0, 0, &[1, 2, 3]), (0, 0, &[1, 2, 3]), (0, 0, &[1, 2, 3])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
    }

    // --- Sibling pair tests ---

    #[test]
    fn test_pe_keep_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_merge_duplicate_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Keep,
        );
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Discard,
        );
        // 1 pair: both removed
        test_process_edges(&[(0, 1, &[]), (1, 0, &[])], &[], &mut opts, S2ErrorCode::Ok);
        // 2 pairs: both removed
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // 3 forward, 1 reverse: 2 forward remain
        let mut opts3 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (0, 1, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
        // 1 forward, 3 reverse: 2 reverse remain
        let mut opts4 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(1, 0, &[]), (1, 0, &[])],
            &mut opts4,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_sibling_pairs_merge_duplicates() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Discard,
        );
        // Equal counts: both removed
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[],
            &mut opts,
            S2ErrorCode::Ok,
        );
        // 3 forward, 1 reverse: 1 forward remains
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // 1 forward, 3 reverse: 1 reverse remains
        let mut opts3 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(1, 0, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::DiscardExcess,
        );
        // 1 pair: kept (excess = balanced)
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        // 2 pairs: 1 excess pair removed
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // 3 forward, 1 reverse: keep 2 forward
        let mut opts3 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (0, 1, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
        // 1 forward, 3 reverse: keep 2 reverse
        let mut opts4 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(1, 0, &[]), (1, 0, &[])],
            &mut opts4,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_sibling_pairs_merge_duplicates() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::DiscardExcess,
        );
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        let mut opts3 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(1, 0, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_undirected_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Discard,
        );
        // 1 undirected pair (2 directed) → kept (can't discard both)
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        // 2 undirected pairs (4 directed) → all removed
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // 3 undirected pairs (6 directed) → 1 undirected pair remains
        let mut opts3 = opts.clone();
        test_process_edges(
            &[
                (0, 1, &[]),
                (0, 1, &[]),
                (0, 1, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
            ],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_discard_excess_undirected_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::DiscardExcess,
        );
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        let mut opts3 = opts.clone();
        test_process_edges(
            &[
                (0, 1, &[]),
                (0, 1, &[]),
                (0, 1, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
            ],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_create_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[(0, 1, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[])],
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_require_sibling_pairs() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Require,
        );
        // Already has sibling: OK
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        // Missing sibling: error, but sibling created
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::BuilderMissingExpectedSiblingEdges,
        );
    }

    #[test]
    fn test_pe_create_undirected_sibling_pairs() {
        // Directed + CREATE with existing pair: no change
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        // Undirected + CREATE: 2 undirected → 1 directed pair
        let mut opts2 = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // Undirected + CREATE: 3 undirected → 2 directed pairs
        let mut opts3 = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[
                (0, 1, &[]),
                (0, 1, &[]),
                (0, 1, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
            ],
            &[(0, 1, &[]), (0, 1, &[]), (1, 0, &[]), (1, 0, &[])],
            &mut opts3,
            S2ErrorCode::Ok,
        );
    }

    #[test]
    fn test_pe_create_sibling_pairs_merge_duplicates() {
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[(0, 1, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        let mut opts2 = opts.clone();
        test_process_edges(
            &[(0, 1, &[]), (0, 1, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
    }

    // ─── Batch 5: Missing graph tests (from C++) ────────────────────────

    /// A layer that captures the graph for later inspection.
    #[derive(Debug)]
    struct GraphCloningLayer {
        graph_options: GraphOptions,
        captured_edges: Rc<RefCell<Vec<Edge>>>,
        captured_vertices: Rc<RefCell<Vec<Point>>>,
        captured_options: Rc<RefCell<GraphOptions>>,
    }

    impl GraphCloningLayer {
        fn new(
            graph_options: GraphOptions,
            captured_edges: Rc<RefCell<Vec<Edge>>>,
            captured_vertices: Rc<RefCell<Vec<Point>>>,
            captured_options: Rc<RefCell<GraphOptions>>,
        ) -> Self {
            GraphCloningLayer {
                graph_options,
                captured_edges,
                captured_vertices,
                captured_options,
            }
        }
    }

    impl crate::s2::builder::layer::Layer for GraphCloningLayer {
        fn graph_options(&self) -> GraphOptions {
            self.graph_options.clone()
        }

        fn build(&mut self, g: &Graph, _error: &mut S2Error) {
            let n = g.num_edges().as_usize();
            let mut edges = Vec::with_capacity(n);
            for e in (0..g.num_edges().0).map(EdgeId) {
                edges.push(g.edge(e));
            }
            *self.captured_edges.borrow_mut() = edges;
            *self.captured_vertices.borrow_mut() = g.vertices.clone();
            *self.captured_options.borrow_mut() = g.options.clone();
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    #[test]
    fn test_get_polylines_undirected_degenerate_paths() {
        // C++: GetPolylines::UndirectedDegeneratePaths
        use crate::s2::builder::S2Builder;
        use crate::s2::text_format::make_lax_polyline;

        let graph_options = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let captured_edges = Rc::new(RefCell::new(Vec::new()));
        let captured_vertices = Rc::new(RefCell::new(Vec::new()));
        let captured_options = Rc::new(RefCell::new(GraphOptions::default()));

        let mut builder = S2Builder::new(crate::s2::builder::Options::default());
        builder.start_layer(Box::new(GraphCloningLayer::new(
            graph_options,
            Rc::clone(&captured_edges),
            Rc::clone(&captured_vertices),
            Rc::clone(&captured_options),
        )));
        builder.add_shape(&make_lax_polyline("1:1, 1:1"));
        builder.add_shape(&make_lax_polyline("0:0, 0:0, 0:1, 0:1, 0:2, 0:2"));
        builder.add_shape(&make_lax_polyline("1:1, 1:1"));
        let result = builder.build();
        assert!(result.is_ok(), "build failed: {:?}", result.err());

        // Reconstruct graph from captured data to call get_polylines.
        let g = Graph::from_raw_parts(
            captured_options.borrow().clone(),
            captured_vertices.borrow().clone(),
            captured_edges.borrow().clone(),
            vec![EMPTY_SET_ID; captured_edges.borrow().len()],
            IdSetLexicon::new(),
            vec![],
            IdSetLexicon::new(),
            None,
        );
        let polylines = g.get_polylines(PolylineType::Path);
        assert_eq!(
            polylines.len(),
            7,
            "expected 7 path polylines, got {}",
            polylines.len()
        );
    }

    #[test]
    fn test_get_polylines_undirected_degenerate_walks() {
        // C++: GetPolylines::UndirectedDegenerateWalks
        use crate::s2::builder::S2Builder;
        use crate::s2::text_format::make_lax_polyline;

        let graph_options = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let captured_edges = Rc::new(RefCell::new(Vec::new()));
        let captured_vertices = Rc::new(RefCell::new(Vec::new()));
        let captured_options = Rc::new(RefCell::new(GraphOptions::default()));

        let mut builder = S2Builder::new(crate::s2::builder::Options::default());
        builder.start_layer(Box::new(GraphCloningLayer::new(
            graph_options,
            Rc::clone(&captured_edges),
            Rc::clone(&captured_vertices),
            Rc::clone(&captured_options),
        )));
        builder.add_shape(&make_lax_polyline("1:1, 1:1"));
        builder.add_shape(&make_lax_polyline("0:0, 0:0, 0:1, 0:1, 0:2, 0:2"));
        builder.add_shape(&make_lax_polyline("1:1, 1:1"));
        let result = builder.build();
        assert!(result.is_ok(), "build failed: {:?}", result.err());

        let g = Graph::from_raw_parts(
            captured_options.borrow().clone(),
            captured_vertices.borrow().clone(),
            captured_edges.borrow().clone(),
            vec![EMPTY_SET_ID; captured_edges.borrow().len()],
            IdSetLexicon::new(),
            vec![],
            IdSetLexicon::new(),
            None,
        );
        let polylines = g.get_polylines(PolylineType::Walk);
        assert_eq!(
            polylines.len(),
            2,
            "expected 2 walk polylines, got {}",
            polylines.len()
        );
        // Sort by length to get deterministic order.
        let mut lens: Vec<usize> = polylines.iter().map(Vec::len).collect();
        lens.sort_unstable();
        assert_eq!(lens, vec![2, 5]);
    }

    #[test]
    fn test_make_subgraph_undirected_to_undirected() {
        // C++: MakeSubgraph::UndirectedToUndirected
        // Test that MakeSubgraph() doesn't transform edges into edge pairs
        // when creating an undirected subgraph of an undirected graph.
        use crate::s2::text_format::parse_points;

        let options = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let vertices = parse_points("0:0, 0:1, 1:1");
        let edges: Vec<Edge> = edges(&[(0, 0), (0, 0), (1, 2), (2, 1)]);
        let input_ids = vec![0_i32, 0, 1, 1];
        let label_set_ids: Vec<LabelSetId> = vec![];
        let input_lexicon = IdSetLexicon::new();
        let label_lexicon = IdSetLexicon::new();

        let graph = Graph::from_raw_parts(
            options,
            vertices,
            edges.clone(),
            input_ids.clone(),
            input_lexicon,
            label_set_ids,
            label_lexicon,
            None,
        );

        // Create subgraph: discard degenerate edges.
        let new_options = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let mut new_edges = edges;
        let mut new_input_ids = input_ids;
        let mut new_lexicon = IdSetLexicon::new();
        let mut error = S2Error::ok();

        let new_g = graph.make_subgraph(
            new_options.clone(),
            &mut new_edges,
            &mut new_input_ids,
            &mut new_lexicon,
            None,
            &mut error,
        );

        assert!(error.is_ok());
        assert_eq!(new_g.options().edge_type, EdgeType::Undirected);
        assert_eq!(new_g.options().degenerate_edges, DegenerateEdges::Discard);
        // Degenerate (0,0) edges removed, only (1,2) and (2,1) remain.
        assert_eq!(new_g.num_edges(), EdgeId(2));
        let result_edges: Vec<Edge> = (0..new_g.num_edges().0)
            .map(EdgeId)
            .map(|e| new_g.edge(e))
            .collect();
        assert_eq!(
            result_edges,
            vec![(VertexId(1), VertexId(2)), (VertexId(2), VertexId(1))]
        );
    }

    #[test]
    fn test_make_subgraph_directed_to_undirected() {
        // C++: MakeSubgraph::DirectedToUndirected
        // Test transforming directed edges into undirected edges (which
        // doubles the number of non-degenerate edges).
        use crate::s2::text_format::parse_points;

        let options = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let vertices = parse_points("0:0, 0:1, 1:1");
        let mut lexicon = IdSetLexicon::new();
        let id1 = lexicon.add_set(&[1]);
        let id2 = lexicon.add_set(&[2]);
        let id3 = lexicon.add_set(&[3]);
        let edges: Vec<Edge> = edges(&[(0, 0), (0, 1), (1, 2), (1, 2), (2, 1)]);
        let input_ids = vec![id1, id2, id3, id3, id3];

        let graph = Graph::from_raw_parts(
            options,
            vertices,
            edges.clone(),
            input_ids.clone(),
            lexicon.clone(),
            vec![],
            IdSetLexicon::new(),
            None,
        );

        // Create undirected subgraph with DiscardExcess sibling pairs.
        let new_options = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::DiscardExcess,
        );
        let mut new_edges = edges;
        let mut new_input_ids = input_ids;
        let mut new_lexicon = lexicon;
        let mut error = S2Error::ok();

        let new_g = graph.make_subgraph(
            new_options.clone(),
            &mut new_edges,
            &mut new_input_ids,
            &mut new_lexicon,
            None,
            &mut error,
        );

        assert!(error.is_ok());
        // Directed → undirected doubles non-degenerate edges, then
        // DiscardExcess removes excess sibling pairs.
        // Expected: {0,0},{0,0}, {0,1},{1,0}, {1,2},{2,1}
        assert_eq!(
            new_g.num_edges(),
            6,
            "expected 6 edges, got {}. Edges: {:?}",
            new_g.num_edges(),
            (0..new_g.num_edges().0)
                .map(EdgeId)
                .map(|e| new_g.edge(e))
                .collect::<Vec<_>>()
        );
        let result_edges: Vec<Edge> = (0..new_g.num_edges().0)
            .map(EdgeId)
            .map(|e| new_g.edge(e))
            .collect();
        assert_eq!(
            result_edges,
            vec![
                (VertexId(0), VertexId(0)),
                (VertexId(0), VertexId(0)),
                (VertexId(0), VertexId(1)),
                (VertexId(1), VertexId(0)),
                (VertexId(1), VertexId(2)),
                (VertexId(2), VertexId(1))
            ]
        );
    }

    #[test]
    fn test_labels_requested_but_not_provided() {
        // C++: Graph::LabelsRequestedButNotProvided
        // Tests that when labels are requested but none were provided,
        // the graph returns empty labels gracefully.
        let options = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        );
        let vertices = vec![Point::from_coords(1.0, 0.0, 0.0)];
        let edges: Vec<Edge> = edges(&[(0, 0)]);
        let mut lexicon = IdSetLexicon::new();
        let id0 = lexicon.add_set(&[0]);
        let input_ids = vec![id0];
        let label_set_ids: Vec<LabelSetId> = vec![]; // Empty = no labels

        let g = Graph::from_raw_parts(
            options,
            vertices,
            edges,
            input_ids,
            lexicon,
            label_set_ids,
            IdSetLexicon::new(),
            None,
        );

        // label_set_ids should be empty.
        assert!(g.label_set_ids().is_empty());
        // labels() for input edge 0 should return empty.
        assert_eq!(g.labels(0).len(), 0);
        // LabelFetcher should also return empty.
        let fetcher = LabelFetcher::new(&g, EdgeType::Directed);
        let labels = fetcher.fetch(&g, 0);
        assert!(labels.is_empty());
    }

    #[test]
    fn test_pe_create_undirected_sibling_pairs_merge_duplicates() {
        // C++: ProcessEdges::CreateUndirectedSiblingPairsMergeDuplicates
        // Directed: Create sibling pairs, merge duplicates.
        let mut opts = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[(0, 1, &[]), (1, 0, &[])],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts,
            S2ErrorCode::Ok,
        );
        assert_eq!(opts.edge_type, EdgeType::Directed);

        // Undirected: multiple copies → merged down to one pair.
        let mut opts2 = GraphOptions::new(
            EdgeType::Undirected,
            DegenerateEdges::Discard,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );
        test_process_edges(
            &[
                (0, 1, &[]),
                (0, 1, &[]),
                (0, 1, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
                (1, 0, &[]),
            ],
            &[(0, 1, &[]), (1, 0, &[])],
            &mut opts2,
            S2ErrorCode::Ok,
        );
        // After processing, undirected with Create converts to Directed.
        assert_eq!(opts2.edge_type, EdgeType::Directed);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_enums_roundtrip() {
        for v in [EdgeType::Directed, EdgeType::Undirected] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<EdgeType>(&j).unwrap());
        }
        for v in [
            DegenerateEdges::Discard,
            DegenerateEdges::DiscardExcess,
            DegenerateEdges::Keep,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<DegenerateEdges>(&j).unwrap());
        }
        for v in [DuplicateEdges::Merge, DuplicateEdges::Keep] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<DuplicateEdges>(&j).unwrap());
        }
        for v in [
            SiblingPairs::Discard,
            SiblingPairs::DiscardExcess,
            SiblingPairs::Keep,
            SiblingPairs::Require,
            SiblingPairs::Create,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<SiblingPairs>(&j).unwrap());
        }
        for v in [LoopType::Simple, LoopType::Circuit] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<LoopType>(&j).unwrap());
        }
        for v in [DegenerateBoundaries::Discard, DegenerateBoundaries::Keep] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<DegenerateBoundaries>(&j).unwrap());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_graph_options_roundtrip() {
        let opts = GraphOptions {
            edge_type: EdgeType::Undirected,
            degenerate_edges: DegenerateEdges::Discard,
            duplicate_edges: DuplicateEdges::Merge,
            sibling_pairs: SiblingPairs::Create,
            allow_vertex_filtering: false,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let back: GraphOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(opts, back);
    }
}
