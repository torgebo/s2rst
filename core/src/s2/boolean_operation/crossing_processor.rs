// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! `CrossingProcessor`: processes edges and determines which belong to the output.
//!
//! This is the core algorithm of `S2BooleanOperation`. It processes edges from
//! one region and determines which belong to the output based on crossings
//! with edges from the other region.

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/ShapeId (i32) used as Vec indices"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId/ShapeId (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize edge/shape counts -> i32 ShapeId/EdgeId — always in range"
)]
use std::collections::{BTreeMap, HashMap};

use crate::s2::Point;
use crate::s2::builder::InputEdgeId;
use crate::s2::builder::S2Builder;
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::shape::{Dimension, Edge, Shape, ShapeEdgeId, ShapeId};
use crate::s2::shape_index::ShapeIndex;

use super::{
    CrossingInputEdge, IndexCrossing, InputEdgeCrossings, K_SET_INSIDE, K_SET_INVERT_B,
    K_SET_REVERSE_A, PolygonModel, PolylineModel, RegionId, SENTINEL, SourceId,
};

/// A crossing between a `SourceId` edge and a boolean (`left_to_right`).
type SourceEdgeCrossing = (SourceId, bool);

/// Temporary representation of crossings using `SourceId` keys.
type SourceEdgeCrossings = Vec<(InputEdgeId, SourceEdgeCrossing)>;

/// A `SourceId` → `InputEdgeId` mapping.
type SourceIdMap = BTreeMap<SourceId, InputEdgeId>;

/// Result of processing point crossings.
#[derive(Default)]
struct PointCrossingResult {
    matches_point: bool,
    matches_polyline: bool,
    matches_polygon: bool,
}

/// Result of processing edge crossings.
#[expect(clippy::struct_excessive_bools, reason = "matches C++ structure")]
struct EdgeCrossingResult {
    matches_polyline: bool,
    a0_matches_polyline: bool,
    a1_matches_polyline: bool,
    a0_matches_polygon: bool,
    a1_matches_polygon: bool,
    polygon_match_id: ShapeEdgeId,
    sibling_match_id: ShapeEdgeId,
    a0_loop_match_id: ShapeEdgeId,
    a0_crossings: i32,
    a1_crossings: i32,
    interior_crossings: i32,
}

impl Default for EdgeCrossingResult {
    fn default() -> Self {
        EdgeCrossingResult {
            matches_polyline: false,
            a0_matches_polyline: false,
            a1_matches_polyline: false,
            a0_matches_polygon: false,
            a1_matches_polygon: false,
            polygon_match_id: ShapeEdgeId::new(0, -1),
            sibling_match_id: ShapeEdgeId::new(0, -1),
            a0_loop_match_id: ShapeEdgeId::new(0, -1),
            a0_crossings: 0,
            a1_crossings: 0,
            interior_crossings: 0,
        }
    }
}

impl EdgeCrossingResult {
    fn matches_polygon(&self) -> bool {
        self.polygon_match_id.edge_id >= 0
    }
    fn matches_sibling(&self) -> bool {
        self.sibling_match_id.edge_id >= 0
    }
    fn loop_matches_a0(&self) -> bool {
        self.a0_loop_match_id.edge_id >= 0
    }
}

/// Iterates through `IndexCrossing` entries for a specific edge from region A.
pub(super) struct CrossingIterator<'a> {
    b_index: &'a ShapeIndex,
    crossings: &'a [IndexCrossing],
    pos: usize,
    b_shape_id: ShapeId,
    b_dimension: Dimension,
    crossings_complete: bool,
    // Lazy chain info.
    cached_chain_id: i32,
    cached_chain_start: usize,
    cached_chain_limit: usize,
}

impl<'a> CrossingIterator<'a> {
    pub(super) fn new(
        b_index: &'a ShapeIndex,
        crossings: &'a [IndexCrossing],
        crossings_complete: bool,
    ) -> Self {
        let mut it = CrossingIterator {
            b_index,
            crossings,
            pos: 0,
            b_shape_id: ShapeId(-1),
            b_dimension: Dimension::Point,
            crossings_complete,
            cached_chain_id: -1,
            cached_chain_start: 0,
            cached_chain_limit: 0,
        };
        it.update();
        it
    }

    pub(super) fn next(&mut self) {
        self.pos += 1;
        self.update();
    }

    pub(super) fn done(&self, id: ShapeEdgeId) -> bool {
        self.a_id() != id
    }

    pub(super) fn crossings_complete(&self) -> bool {
        self.crossings_complete
    }

    pub(super) fn is_interior_crossing(&self) -> bool {
        self.crossings[self.pos].is_interior_crossing
    }

    pub(super) fn is_vertex_crossing(&self) -> bool {
        self.crossings[self.pos].is_vertex_crossing
    }

    pub(super) fn left_to_right(&self) -> bool {
        self.crossings[self.pos].left_to_right
    }

    pub(super) fn a_id(&self) -> ShapeEdgeId {
        if self.pos < self.crossings.len() {
            self.crossings[self.pos].a
        } else {
            SENTINEL
        }
    }

    pub(super) fn b_id(&self) -> ShapeEdgeId {
        self.crossings[self.pos].b
    }

    pub(super) fn b_index(&self) -> &'a ShapeIndex {
        self.b_index
    }

    pub(super) fn b_dimension(&self) -> Dimension {
        self.b_dimension
    }

    pub(super) fn b_shape_id(&self) -> ShapeId {
        self.b_shape_id
    }

    pub(super) fn b_edge_id(&self) -> i32 {
        self.b_id().edge_id
    }

    pub(super) fn b_edge(&self) -> Edge {
        let Some(shape) = self.b_index.shape(self.b_shape_id) else {
            return Edge {
                v0: Point::default(),
                v1: Point::default(),
            };
        };
        shape.edge(self.b_edge_id() as usize)
    }

    pub(super) fn b_shape(&self) -> Option<&'a dyn Shape> {
        self.b_index.shape(self.b_shape_id)
    }

    /// Returns chain info for current B edge (computed lazily).
    pub(super) fn b_chain_info(&mut self) -> (i32, usize, usize) {
        if self.cached_chain_id < 0 {
            let Some(shape) = self.b_index.shape(self.b_shape_id) else {
                return (-1, 0, 0);
            };
            let cp = shape.chain_position(self.b_edge_id() as usize);
            self.cached_chain_id = cp.chain_id as i32;
            let chain = shape.chain(cp.chain_id);
            self.cached_chain_start = chain.start;
            self.cached_chain_limit = chain.start + chain.length;
        }
        (
            self.cached_chain_id,
            self.cached_chain_start,
            self.cached_chain_limit,
        )
    }

    fn update(&mut self) {
        if self.pos < self.crossings.len()
            && self.a_id() != SENTINEL
            && self.b_id().shape_id != self.b_shape_id
        {
            self.b_shape_id = self.b_id().shape_id;
            if let Some(shape) = self.b_index.shape(self.b_shape_id) {
                self.b_dimension = shape.dimension();
            }
            self.cached_chain_id = -1;
        }
    }
}

/// `CrossingProcessor` processes edges from one region and determines which
/// belong to the output based on crossings with the other region.
#[expect(clippy::struct_excessive_bools, reason = "matches C++ structure")]
pub(super) struct CrossingProcessor<'a> {
    polygon_model: PolygonModel,
    polyline_model: PolylineModel,
    polyline_loops_have_boundaries: bool,

    builder: Option<&'a mut S2Builder>,
    input_dimensions: Option<&'a mut Vec<Dimension>>,
    input_crossings: Option<&'a mut InputEdgeCrossings>,

    /// Map from (`region_id`, `shape_id`, `edge_id`) to intersection points.
    /// Used to register intersection points with the builder when edges are added.
    crossing_point_map: Option<HashMap<(RegionId, ShapeId, i32), Vec<Point>>>,

    // Fields set by start_boundary:
    a_region_id: RegionId,
    b_region_id: RegionId,
    invert_a: bool,
    invert_b: bool,
    invert_result: bool,
    is_union: bool,

    // Fields set by start_shape:
    a_dimension: Dimension,

    // Fields set by start_chain:
    chain_id: usize,
    chain_start: usize,
    chain_limit: usize,

    // Fields updated by process_edge:
    pub inside: bool,
    prev_inside: bool,
    v0_emitted_max_edge_id: i32,
    chain_v0_emitted: bool,

    source_edge_crossings: SourceEdgeCrossings,
    pending_source_edge_crossings: Vec<SourceEdgeCrossing>,
    source_id_map: SourceIdMap,
    is_degenerate_hole: HashMap<ShapeEdgeId, bool>,
}

impl<'a> CrossingProcessor<'a> {
    pub(super) fn new(
        polygon_model: PolygonModel,
        polyline_model: PolylineModel,
        polyline_loops_have_boundaries: bool,
        builder: Option<&'a mut S2Builder>,
        input_dimensions: Option<&'a mut Vec<Dimension>>,
        input_crossings: Option<&'a mut InputEdgeCrossings>,
        crossing_point_map: Option<HashMap<(RegionId, ShapeId, i32), Vec<Point>>>,
    ) -> Self {
        CrossingProcessor {
            polygon_model,
            polyline_model,
            polyline_loops_have_boundaries,
            builder,
            input_dimensions,
            input_crossings,
            crossing_point_map,
            a_region_id: RegionId::A,
            b_region_id: RegionId::B,
            invert_a: false,
            invert_b: false,
            invert_result: false,
            is_union: false,
            a_dimension: Dimension::Point,
            chain_id: 0,
            chain_start: 0,
            chain_limit: 0,
            inside: false,
            prev_inside: false,
            v0_emitted_max_edge_id: -1,
            chain_v0_emitted: false,
            source_edge_crossings: Vec::new(),
            pending_source_edge_crossings: Vec::new(),
            source_id_map: BTreeMap::new(),
            is_degenerate_hole: HashMap::new(),
        }
    }

    fn input_edge_id(&self) -> InputEdgeId {
        match &self.input_dimensions {
            Some(dims) => InputEdgeId(dims.len() as i32),
            None => InputEdgeId(0),
        }
    }

    pub(super) fn start_boundary(
        &mut self,
        a_region_id: RegionId,
        invert_a: bool,
        invert_b: bool,
        invert_result: bool,
    ) {
        self.a_region_id = a_region_id;
        self.b_region_id = a_region_id.other();
        self.invert_a = invert_a;
        self.invert_b = invert_b;
        self.invert_result = invert_result;
        self.is_union = invert_b && invert_result;

        self.set_clipping_state(K_SET_REVERSE_A, invert_a != invert_result);
        self.set_clipping_state(K_SET_INVERT_B, invert_b);
    }

    pub(super) fn start_shape(&mut self, dimension: Dimension) {
        self.a_dimension = dimension;
    }

    pub(super) fn start_chain(
        &mut self,
        chain_id: usize,
        chain_start: usize,
        chain_length: usize,
        inside: bool,
    ) {
        self.chain_id = chain_id;
        self.chain_start = chain_start;
        self.chain_limit = chain_start + chain_length;
        self.inside = inside;
        self.v0_emitted_max_edge_id = chain_start as i32 - 1;
        self.chain_v0_emitted = false;
    }

    /// Processes a single edge. Returns false for early exit (boolean output).
    pub(super) fn process_edge(
        &mut self,
        a_id: ShapeEdgeId,
        a: Edge,
        a_shape: &dyn Shape,
        it: &mut CrossingIterator<'_>,
    ) -> bool {
        match self.a_dimension {
            Dimension::Point => self.process_edge0(a_id, a, it),
            Dimension::Polyline => self.process_edge1(a_id, a, a_shape, it),
            Dimension::Polygon => self.process_edge2(a_id, a, it),
        }
    }

    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn skip_crossings(&self, a_id: ShapeEdgeId, it: &mut CrossingIterator<'_>) {
        while !it.done(a_id) {
            it.next();
        }
    }

    fn is_v0_isolated(&self, a_id: ShapeEdgeId) -> bool {
        !self.inside && self.v0_emitted_max_edge_id < a_id.edge_id
    }

    fn is_chain_last_vertex_isolated(&self, a_id: ShapeEdgeId) -> bool {
        a_id.edge_id == self.chain_limit as i32 - 1
            && !self.chain_v0_emitted
            && self.v0_emitted_max_edge_id <= a_id.edge_id
    }

    fn polyline_contains_v0(&self, edge_id: i32, chain_start: usize) -> bool {
        self.polyline_model != PolylineModel::Open || edge_id > chain_start as i32
    }

    fn is_degenerate(&self, a_id: ShapeEdgeId) -> bool {
        self.is_degenerate_hole.contains_key(&a_id)
    }

    fn add_crossing(&mut self, crossing: SourceEdgeCrossing) {
        let input_id = self.input_edge_id();
        self.source_edge_crossings.push((input_id, crossing));
    }

    fn add_interior_crossing(&mut self, crossing: SourceEdgeCrossing) {
        self.pending_source_edge_crossings.push(crossing);
    }

    fn set_clipping_state(&mut self, parameter: InputEdgeId, state: bool) {
        self.add_crossing((SourceId::from_special(parameter), state));
    }

    /// Adds an edge to the builder. Returns false for early exit (boolean output).
    fn add_edge(
        &mut self,
        a_id: ShapeEdgeId,
        a: Edge,
        dimension: Dimension,
        interior_crossings: i32,
    ) -> bool {
        if self.builder.is_none() {
            return false; // Boolean output
        }
        if interior_crossings > 0 {
            let input_id = self.input_edge_id();
            for crossing in &self.pending_source_edge_crossings {
                self.source_edge_crossings.push((input_id, *crossing));
            }
            let src_id = SourceId::new(self.a_region_id, a_id.shape_id, a_id.edge_id);
            self.source_id_map.insert(src_id, input_id);
        }
        if self.inside != self.prev_inside {
            let input_id = self.input_edge_id();
            self.source_edge_crossings.push((
                input_id,
                (SourceId::from_special(K_SET_INSIDE), self.inside),
            ));
        }
        if let Some(dims) = &mut self.input_dimensions {
            dims.push(dimension);
        }
        let Some(builder) = self.builder.as_mut() else {
            return false;
        };
        builder.add_edge(a.v0, a.v1);
        // Register intersection points from the crossing_point_map.
        // This ensures edges are split at inter-region crossing points
        // even when the other region's edges aren't in the builder.
        if let Some(ref map) = self.crossing_point_map {
            let key = (self.a_region_id, a_id.shape_id, a_id.edge_id);
            if let Some(points) = map.get(&key) {
                let edge_idx = (builder.num_input_edges() - 1) as usize;
                for &pt in points {
                    builder.add_intersection_for_edge(pt, edge_idx);
                }
            }
        }
        self.inside ^= (interior_crossings & 1) != 0;
        self.prev_inside = self.inside;
        true
    }

    /// Adds a point (degenerate) edge. Returns false for early exit.
    fn add_point_edge(&mut self, p: Point, dimension: Dimension) -> bool {
        if self.builder.is_none() {
            return false;
        }
        if !self.prev_inside {
            let input_id = self.input_edge_id();
            self.source_edge_crossings
                .push((input_id, (SourceId::from_special(K_SET_INSIDE), true)));
        }
        if let Some(dims) = &mut self.input_dimensions {
            dims.push(dimension);
        }
        if let Some(builder) = self.builder.as_mut() {
            builder.add_edge(p, p);
        }
        self.prev_inside = true;
        true
    }

    // ─── Dimension-specific processing ───────────────────────────────

    fn process_edge0(&mut self, a_id: ShapeEdgeId, a: Edge, it: &mut CrossingIterator<'_>) -> bool {
        debug_assert_eq!(a.v0, a.v1);
        if self.invert_a != self.invert_result {
            self.skip_crossings(a_id, it);
            return true;
        }
        let r = self.process_point_crossings(a_id, a.v0, it);

        let mut contained = self.inside ^ self.invert_b;
        if r.matches_polygon && self.polygon_model != PolygonModel::SemiOpen {
            contained = self.polygon_model == PolygonModel::Closed;
        }
        if r.matches_polyline {
            contained = true;
        }
        if r.matches_point && !self.is_union {
            contained = true;
        }
        if contained == self.invert_b {
            return true;
        }
        self.add_point_edge(a.v0, Dimension::Point)
    }

    fn process_point_crossings(
        &self,
        a_id: ShapeEdgeId,
        a0: Point,
        it: &mut CrossingIterator<'_>,
    ) -> PointCrossingResult {
        let mut r = PointCrossingResult::default();
        while !it.done(a_id) {
            match it.b_dimension() {
                Dimension::Point => r.matches_point = true,
                Dimension::Polyline => {
                    if self.polyline_edge_contains_vertex(a0, it, Dimension::Point) {
                        r.matches_polyline = true;
                    }
                }
                Dimension::Polygon => r.matches_polygon = true,
            }
            it.next();
        }
        r
    }

    fn process_edge1(
        &mut self,
        a_id: ShapeEdgeId,
        a: Edge,
        a_shape: &dyn Shape,
        it: &mut CrossingIterator<'_>,
    ) -> bool {
        if self.invert_a != self.invert_result {
            self.skip_crossings(a_id, it);
            return true;
        }
        let mut r = self.process_edge_crossings(a_id, a, it);
        let a0_inside = self.is_polyline_vertex_inside(r.a0_matches_polyline, r.a0_matches_polygon);

        let is_degenerate = a.v0 == a.v1;
        self.inside ^= (r.a0_crossings & 1) != 0;
        if self.inside != self.is_polyline_edge_inside(&r, is_degenerate) {
            self.inside ^= true;
            r.a1_crossings += 1;
        }

        // Check for isolated v0 vertex.
        if !self.polyline_loops_have_boundaries
            && a_id.edge_id == self.chain_start as i32
            && a.v0
                == a_shape
                    .chain_edge(self.chain_id, self.chain_limit - self.chain_start - 1)
                    .v1
        {
            // This is the first vertex of a polyline loop, so we can't decide
            // if it is isolated until we process the last polyline edge.
            self.chain_v0_emitted = self.inside;
        } else if self.is_v0_isolated(a_id)
            && !is_degenerate
            && self.polyline_contains_v0(a_id.edge_id, self.chain_start)
            && a0_inside
            && !self.add_point_edge(a.v0, Dimension::Polyline)
        {
            return false;
        }

        if (self.inside || r.interior_crossings > 0)
            && !self.add_edge(a_id, a, Dimension::Polyline, r.interior_crossings)
        {
            return false;
        }
        if self.inside {
            self.v0_emitted_max_edge_id = a_id.edge_id + 1;
        }

        self.inside ^= (r.a1_crossings & 1) != 0;

        // Verify that edge crossings are being counted correctly.
        if it.crossings_complete() {
            debug_assert_eq!(
                ContainsPointQuery::new(it.b_index(), VertexModel::SemiOpen).contains(a.v1),
                self.inside ^ self.invert_b,
                "crossing count mismatch in process_edge1"
            );
        }

        // Check for isolated last vertex.
        if it.crossings_complete()
            && !is_degenerate
            && self.is_chain_last_vertex_isolated(a_id)
            && (self.polyline_model == PolylineModel::Closed
                || (!self.polyline_loops_have_boundaries
                    && a.v1 == a_shape.chain_edge(self.chain_id, 0).v0))
            && self.is_polyline_vertex_inside(r.a1_matches_polyline, r.a1_matches_polygon)
            && !self.add_point_edge(a.v1, Dimension::Polyline)
        {
            return false;
        }
        true
    }

    fn is_polyline_vertex_inside(&self, matches_polyline: bool, matches_polygon: bool) -> bool {
        let mut contained = self.inside ^ self.invert_b;
        if matches_polyline && !self.is_union {
            contained = true;
        } else if matches_polygon && self.polygon_model != PolygonModel::SemiOpen {
            contained = self.polygon_model == PolygonModel::Closed;
        }
        contained ^ self.invert_b
    }

    fn is_polyline_edge_inside(&self, r: &EdgeCrossingResult, is_degenerate: bool) -> bool {
        let mut contained = self.inside ^ self.invert_b;
        if r.matches_polyline && !self.is_union {
            contained = true;
        } else if is_degenerate {
            if self.polygon_model != PolygonModel::SemiOpen && r.a0_matches_polygon {
                contained = self.polygon_model == PolygonModel::Closed;
            }
            if r.a0_matches_polyline && !self.is_union {
                contained = true;
            }
        } else if r.matches_polygon() {
            if !(self.polygon_model == PolygonModel::SemiOpen && r.matches_sibling()) {
                contained = self.polygon_model != PolygonModel::Open;
            }
        } else if r.matches_sibling() {
            contained = self.polygon_model == PolygonModel::Closed;
        }
        contained ^ self.invert_b
    }

    fn process_edge2(&mut self, a_id: ShapeEdgeId, a: Edge, it: &mut CrossingIterator<'_>) -> bool {
        let emit_shared = self.a_region_id == RegionId::B;
        let create_degen =
            (self.polygon_model == PolygonModel::Closed && !self.invert_a && !self.invert_b)
                || (self.polygon_model == PolygonModel::Open && self.invert_a && self.invert_b);
        let keep_degen_a = self.polygon_model == PolygonModel::Open && self.invert_b;
        let keep_degen_b = self.polygon_model == PolygonModel::Open && self.invert_a;

        let mut r = self.process_edge_crossings(a_id, a, it);
        debug_assert!(!r.matches_polyline);

        if self.invert_a != self.invert_b {
            std::mem::swap(&mut r.polygon_match_id, &mut r.sibling_match_id);
        }

        let is_point = a.v0 == a.v1;
        if !emit_shared {
            if r.loop_matches_a0() {
                self.is_degenerate_hole
                    .insert(r.a0_loop_match_id, self.inside);
                if is_point {
                    return true;
                }
            }
            if self.polygon_model != PolygonModel::SemiOpen && is_point && r.a0_matches_polygon {
                return true;
            }
        }

        self.inside ^= (r.a0_crossings & 1) != 0;

        if !emit_shared && (r.matches_polygon() || r.matches_sibling()) {
            if r.matches_polygon() && r.matches_sibling() {
                self.is_degenerate_hole
                    .insert(r.polygon_match_id, self.inside);
                self.is_degenerate_hole
                    .insert(r.sibling_match_id, self.inside);
            }
            debug_assert_eq!(r.interior_crossings, 0);
            self.inside ^= (r.a1_crossings & 1) != 0;
            return true;
        }

        let is_b_hole = r.matches_polygon() && r.matches_sibling() && self.inside;
        let semi_open_inside = self.inside;

        if is_point {
            if r.loop_matches_a0() {
                let is_degen_hole = self
                    .is_degenerate_hole
                    .get(&r.a0_loop_match_id)
                    .copied()
                    .unwrap_or(false);
                self.inside = create_degen || keep_degen_a || (self.inside == is_degen_hole);
            } else if r.a0_matches_polygon && self.polygon_model != PolygonModel::SemiOpen {
                self.inside = create_degen || keep_degen_a;
            }
        } else if r.matches_polygon() {
            if self.is_degenerate(a_id) {
                let is_degen_hole_a = self.is_degenerate_hole.get(&a_id).copied().unwrap_or(false);
                self.inside = create_degen
                    || keep_degen_a
                    || (!r.matches_sibling() || self.inside) == is_degen_hole_a;
            } else if !r.matches_sibling() || create_degen || keep_degen_b {
                self.inside = true;
            }
        } else if r.matches_sibling() {
            if self.is_degenerate(a_id) {
                let is_degen_hole_a = self.is_degenerate_hole.get(&a_id).copied().unwrap_or(false);
                self.inside = (create_degen || keep_degen_a) && !is_degen_hole_a;
            } else {
                self.inside = create_degen;
            }
        }

        if self.inside != semi_open_inside {
            r.a1_crossings += 1;
        }

        // Emit isolated degenerate vertex if needed.
        if emit_shared
            && r.a0_matches_polygon
            && !self.inside
            && (create_degen || (keep_degen_b && r.loop_matches_a0()))
            && !self.add_point_edge(a.v0, Dimension::Polygon)
        {
            return false;
        }

        // Emit sibling pair edge if needed.
        if r.matches_sibling()
            && (create_degen || keep_degen_b)
            && !self.is_degenerate(a_id)
            && !is_b_hole
        {
            let sibling = Edge::new(a.v1, a.v0);
            if !self.add_edge(r.sibling_match_id, sibling, Dimension::Polygon, 0) {
                return false;
            }
        }

        if (self.inside || r.interior_crossings > 0)
            && !self.add_edge(a_id, a, Dimension::Polygon, r.interior_crossings)
        {
            return false;
        }
        self.inside ^= (r.a1_crossings & 1) != 0;

        // Verify that edge crossings are being counted correctly.
        if it.crossings_complete() {
            debug_assert_eq!(
                ContainsPointQuery::new(it.b_index(), VertexModel::SemiOpen).contains(a.v1),
                self.inside ^ self.invert_b,
                "crossing count mismatch in process_edge2"
            );
        }
        true
    }

    fn process_edge_crossings(
        &mut self,
        a_id: ShapeEdgeId,
        a: Edge,
        it: &mut CrossingIterator<'_>,
    ) -> EdgeCrossingResult {
        self.pending_source_edge_crossings.clear();
        let mut r = EdgeCrossingResult::default();
        if it.done(a_id) {
            return r;
        }

        while !it.done(a_id) {
            if it.b_dimension() == Dimension::Point {
                it.next();
                continue;
            }
            let b = it.b_edge();
            if it.is_interior_crossing() {
                if self.a_dimension <= it.b_dimension()
                    && !(self.invert_b != self.invert_result
                        && it.b_dimension() == Dimension::Polyline)
                {
                    let src_id = SourceId::new(self.b_region_id, it.b_shape_id(), it.b_edge_id());
                    self.add_interior_crossing((src_id, it.left_to_right()));
                }
                r.interior_crossings += if it.b_dimension() == Dimension::Polyline {
                    2
                } else {
                    1
                };
            } else if it.b_dimension() == Dimension::Polyline {
                if self.a_dimension == Dimension::Polygon {
                    it.next();
                    continue;
                }
                if (a.v0 == b.v0 && a.v1 == b.v1) || (a.v0 == b.v1 && a.v1 == b.v0) {
                    r.matches_polyline = true;
                }
                if (a.v0 == b.v0 || a.v0 == b.v1)
                    && self.polyline_edge_contains_vertex(a.v0, it, Dimension::Polyline)
                {
                    r.a0_matches_polyline = true;
                }
                if (a.v1 == b.v0 || a.v1 == b.v1)
                    && self.polyline_edge_contains_vertex(a.v1, it, Dimension::Polyline)
                {
                    r.a1_matches_polyline = true;
                }
            } else {
                debug_assert_eq!(Dimension::Polygon, it.b_dimension());
                if a.v0 == a.v1 || b.v0 == b.v1 {
                    if a.v0 == b.v0 && a.v0 == b.v1 {
                        r.a0_loop_match_id = it.b_id();
                    }
                } else if a.v0 == b.v0 && a.v1 == b.v1 {
                    r.a0_crossings += 1;
                    r.polygon_match_id = it.b_id();
                } else if a.v0 == b.v1 && a.v1 == b.v0 {
                    r.a0_crossings += 1;
                    r.sibling_match_id = it.b_id();
                } else if it.is_vertex_crossing() {
                    if a.v0 == b.v0 || a.v0 == b.v1 {
                        r.a0_crossings += 1;
                    } else {
                        r.a1_crossings += 1;
                    }
                }
                if a.v0 == b.v0 || a.v0 == b.v1 {
                    r.a0_matches_polygon = true;
                }
                if a.v1 == b.v0 || a.v1 == b.v1 {
                    r.a1_matches_polygon = true;
                }
            }
            it.next();
        }
        r
    }

    fn polyline_edge_contains_vertex(
        &self,
        v: Point,
        it: &mut CrossingIterator<'_>,
        dimension: Dimension,
    ) -> bool {
        debug_assert_eq!(Dimension::Polyline, it.b_dimension());
        debug_assert!(it.b_edge().v0 == v || it.b_edge().v1 == v);
        debug_assert!(dimension <= Dimension::Polyline);
        if self.polyline_model == PolylineModel::Closed {
            return true;
        }

        let Some(shape) = it.b_shape() else {
            return true;
        };
        let (chain_id, b_chain_start, b_chain_limit) = it.b_chain_info();
        let b_edge_id = it.b_edge_id();

        let b_edge = it.b_edge();
        if b_edge_id as usize == b_chain_limit - 1
            && v == b_edge.v1
            && (dimension == Dimension::Point
                || b_edge_id as usize > b_chain_start
                || v != b_edge.v0)
        {
            return false;
        }

        if self.polyline_model != PolylineModel::Open || b_edge_id as usize > b_chain_start {
            return true;
        }
        if v != b_edge.v0 {
            return true;
        }
        if self.polyline_loops_have_boundaries {
            return false;
        }
        v == shape
            .chain_edge(chain_id as usize, b_chain_limit - b_chain_start - 1)
            .v1
    }

    /// Translates `SourceId` crossings to `InputEdgeId` crossings.
    pub(super) fn done_boundary_pair(&mut self) {
        // Add special crossings.
        self.source_id_map
            .insert(SourceId::from_special(K_SET_INSIDE), K_SET_INSIDE);
        self.source_id_map
            .insert(SourceId::from_special(K_SET_INVERT_B), K_SET_INVERT_B);
        self.source_id_map
            .insert(SourceId::from_special(K_SET_REVERSE_A), K_SET_REVERSE_A);

        if let Some(input_crossings) = &mut self.input_crossings {
            for (input_id, (src_id, left_to_right)) in &self.source_edge_crossings {
                let mapped_id = self.source_id_map.get(src_id);
                debug_assert!(
                    mapped_id.is_some(),
                    "source_id_map missing entry for {src_id:?}"
                );
                if let Some(&mapped_id) = mapped_id {
                    input_crossings
                        .push((*input_id, CrossingInputEdge::new(mapped_id, *left_to_right)));
                }
            }
        }

        self.source_edge_crossings.clear();
        self.source_id_map.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::shape::ShapeEdgeId;

    // ─── EdgeCrossingResult ──────────────────────────────────────────

    #[test]
    fn test_edge_crossing_result_default() {
        let r = EdgeCrossingResult::default();
        assert!(!r.matches_polyline);
        assert!(!r.a0_matches_polyline);
        assert!(!r.a1_matches_polyline);
        assert!(!r.a0_matches_polygon);
        assert!(!r.a1_matches_polygon);
        assert!(!r.matches_polygon());
        assert!(!r.matches_sibling());
        assert!(!r.loop_matches_a0());
        assert_eq!(r.a0_crossings, 0);
        assert_eq!(r.a1_crossings, 0);
        assert_eq!(r.interior_crossings, 0);
    }

    #[test]
    fn test_edge_crossing_result_matches_polygon() {
        let mut r = EdgeCrossingResult::default();
        assert!(!r.matches_polygon());
        r.polygon_match_id = ShapeEdgeId::new(0, 5);
        assert!(r.matches_polygon());
    }

    #[test]
    fn test_edge_crossing_result_matches_sibling() {
        let mut r = EdgeCrossingResult::default();
        assert!(!r.matches_sibling());
        r.sibling_match_id = ShapeEdgeId::new(1, 3);
        assert!(r.matches_sibling());
    }

    #[test]
    fn test_edge_crossing_result_loop_matches_a0() {
        let mut r = EdgeCrossingResult::default();
        assert!(!r.loop_matches_a0());
        r.a0_loop_match_id = ShapeEdgeId::new(2, 0);
        assert!(r.loop_matches_a0());
    }

    // ─── PointCrossingResult ─────────────────────────────────────────

    #[test]
    fn test_point_crossing_result_default() {
        let r = PointCrossingResult::default();
        assert!(!r.matches_point);
        assert!(!r.matches_polyline);
        assert!(!r.matches_polygon);
    }

    // ─── CrossingIterator ────────────────────────────────────────────

    #[test]
    fn test_crossing_iterator_empty_crossings() {
        let index = ShapeIndex::new();
        let crossings: Vec<IndexCrossing> = vec![];
        let it = CrossingIterator::new(&index, &crossings, true);
        // With no crossings, should immediately be done for any edge id.
        assert!(it.done(ShapeEdgeId::new(0, 0)));
        assert!(it.done(ShapeEdgeId::new(1, 5)));
        assert_eq!(it.a_id(), SENTINEL);
    }

    #[test]
    fn test_crossing_iterator_single_crossing() {
        let index = ShapeIndex::new();
        let a_id = ShapeEdgeId::new(0, 0);
        let b_id = ShapeEdgeId::new(0, 1);
        let crossings = vec![
            IndexCrossing::new(a_id, b_id),
            IndexCrossing::new(SENTINEL, SENTINEL),
        ];
        let mut it = CrossingIterator::new(&index, &crossings, true);
        assert!(!it.done(a_id));
        assert_eq!(it.a_id(), a_id);
        assert_eq!(it.b_id(), b_id);
        assert!(!it.is_interior_crossing());
        assert!(!it.is_vertex_crossing());
        assert!(!it.left_to_right());
        it.next();
        assert!(it.done(a_id));
    }

    #[test]
    fn test_crossing_iterator_multiple_crossings_same_a() {
        let index = ShapeIndex::new();
        let a_id = ShapeEdgeId::new(0, 3);
        let crossings = vec![
            IndexCrossing::new(a_id, ShapeEdgeId::new(1, 0)),
            IndexCrossing::new(a_id, ShapeEdgeId::new(1, 1)),
            IndexCrossing::new(SENTINEL, SENTINEL),
        ];
        let mut it = CrossingIterator::new(&index, &crossings, false);
        assert!(!it.done(a_id));
        assert_eq!(it.b_id(), ShapeEdgeId::new(1, 0));
        it.next();
        assert!(!it.done(a_id));
        assert_eq!(it.b_id(), ShapeEdgeId::new(1, 1));
        it.next();
        assert!(it.done(a_id));
        // Different a_id should also be done.
        assert!(it.done(ShapeEdgeId::new(0, 2)));
    }

    #[test]
    fn test_crossing_iterator_interior_and_vertex_flags() {
        let index = ShapeIndex::new();
        let a_id = ShapeEdgeId::new(0, 0);
        let mut c = IndexCrossing::new(a_id, ShapeEdgeId::new(1, 0));
        c.is_interior_crossing = true;
        c.is_vertex_crossing = true;
        c.left_to_right = true;
        let crossings = vec![c, IndexCrossing::new(SENTINEL, SENTINEL)];
        let it = CrossingIterator::new(&index, &crossings, true);
        assert!(it.is_interior_crossing());
        assert!(it.is_vertex_crossing());
        assert!(it.left_to_right());
        assert!(it.crossings_complete());
    }

    #[test]
    fn test_crossing_iterator_crossings_complete() {
        let index = ShapeIndex::new();
        let crossings = vec![IndexCrossing::new(SENTINEL, SENTINEL)];
        let it_complete = CrossingIterator::new(&index, &crossings, true);
        assert!(it_complete.crossings_complete());

        let it_incomplete = CrossingIterator::new(&index, &crossings, false);
        assert!(!it_incomplete.crossings_complete());
    }

    #[test]
    fn test_crossing_iterator_done_for_wrong_a_id() {
        let index = ShapeIndex::new();
        let a_id = ShapeEdgeId::new(0, 5);
        let crossings = vec![
            IndexCrossing::new(a_id, ShapeEdgeId::new(1, 0)),
            IndexCrossing::new(SENTINEL, SENTINEL),
        ];
        let it = CrossingIterator::new(&index, &crossings, true);
        // The iterator is positioned at a_id (0,5), so it should NOT be done
        // for (0,5) but SHOULD be done for a different a_id.
        assert!(!it.done(a_id));
        assert!(it.done(ShapeEdgeId::new(0, 4)));
        assert!(it.done(ShapeEdgeId::new(1, 5)));
    }

    // ─── CrossingProcessor helper methods ────────────────────────────

    #[test]
    fn test_is_v0_isolated() {
        let cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        // inside=false, v0_emitted_max_edge_id=-1 (default)
        assert!(cp.is_v0_isolated(ShapeEdgeId::new(0, 0)));
        assert!(cp.is_v0_isolated(ShapeEdgeId::new(0, 5)));
    }

    #[test]
    fn test_is_v0_isolated_when_inside() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.inside = true;
        // inside=true → never isolated.
        assert!(!cp.is_v0_isolated(ShapeEdgeId::new(0, 0)));
    }

    #[test]
    fn test_is_v0_isolated_when_already_emitted() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.v0_emitted_max_edge_id = 3;
        // Edge 3: v0_emitted_max_edge_id == edge_id → not isolated.
        assert!(!cp.is_v0_isolated(ShapeEdgeId::new(0, 3)));
        assert!(!cp.is_v0_isolated(ShapeEdgeId::new(0, 2)));
        // Edge 4: beyond the last emitted v0 → isolated.
        assert!(cp.is_v0_isolated(ShapeEdgeId::new(0, 4)));
    }

    #[test]
    fn test_polyline_contains_v0() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Open,
            true,
            None,
            None,
            None,
            None,
        );
        // Open model: first edge (id=chain_start) does not contain v0.
        assert!(!cp.polyline_contains_v0(0, 0));
        // But second edge onwards does.
        assert!(cp.polyline_contains_v0(1, 0));
        assert!(cp.polyline_contains_v0(5, 0));

        // SemiOpen: always contains v0.
        cp.polyline_model = PolylineModel::SemiOpen;
        assert!(cp.polyline_contains_v0(0, 0));
        assert!(cp.polyline_contains_v0(5, 0));

        // Closed: always contains v0.
        cp.polyline_model = PolylineModel::Closed;
        assert!(cp.polyline_contains_v0(0, 0));
    }

    #[test]
    fn test_is_chain_last_vertex_isolated() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.chain_start = 0;
        cp.chain_limit = 5;
        cp.chain_v0_emitted = false;
        cp.v0_emitted_max_edge_id = -1;

        // Edge 4 = chain_limit - 1 = last edge, chain_v0_emitted = false,
        // v0_emitted_max_edge_id (-1) <= 4 → isolated.
        assert!(cp.is_chain_last_vertex_isolated(ShapeEdgeId::new(0, 4)));

        // Edge 3 != chain_limit - 1 → not the last vertex.
        assert!(!cp.is_chain_last_vertex_isolated(ShapeEdgeId::new(0, 3)));
    }

    #[test]
    fn test_is_chain_last_vertex_not_isolated_when_v0_emitted() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.chain_start = 0;
        cp.chain_limit = 5;
        cp.chain_v0_emitted = true;
        cp.v0_emitted_max_edge_id = -1;

        // chain_v0_emitted = true → not isolated.
        assert!(!cp.is_chain_last_vertex_isolated(ShapeEdgeId::new(0, 4)));
    }

    #[test]
    fn test_start_boundary_sets_fields() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.start_boundary(RegionId::B, true, false, true);
        assert_eq!(cp.a_region_id, RegionId::B);
        assert_eq!(cp.b_region_id, RegionId::A);
        assert!(cp.invert_a);
        assert!(!cp.invert_b);
        assert!(cp.invert_result);
        assert!(!cp.is_union); // is_union = invert_b && invert_result = false && true = false
    }

    #[test]
    fn test_start_boundary_union_detection() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        // Union: invert_b=true, invert_result=true
        cp.start_boundary(RegionId::A, false, true, true);
        assert!(cp.is_union);
    }

    #[test]
    fn test_start_shape_sets_dimension() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.start_shape(Dimension::Polygon);
        assert_eq!(cp.a_dimension, Dimension::Polygon);
        cp.start_shape(Dimension::Point);
        assert_eq!(cp.a_dimension, Dimension::Point);
    }

    #[test]
    fn test_start_chain_sets_fields() {
        let mut cp = CrossingProcessor::new(
            PolygonModel::SemiOpen,
            PolylineModel::Closed,
            true,
            None,
            None,
            None,
            None,
        );
        cp.start_chain(3, 10, 5, true);
        assert_eq!(cp.chain_id, 3);
        assert_eq!(cp.chain_start, 10);
        assert_eq!(cp.chain_limit, 15);
        assert!(cp.inside);
        assert_eq!(cp.v0_emitted_max_edge_id, 9); // chain_start - 1
        assert!(!cp.chain_v0_emitted);
    }
}
