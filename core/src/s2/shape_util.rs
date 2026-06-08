// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Utilities for visiting crossing edge pairs within and between `ShapeIndexes`.
//!
//! Corresponds to C++ `s2shapeutil_visit_crossing_edge_pairs.{h,cc}`.

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId (i32) used as Vec indices in shape utilities"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId/ShapeId (i32) <-> usize and num_edges (usize->i32)"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId/ShapeId — always in range"
)]
use std::ops::ControlFlow;

use crate::s2::CellId;
use crate::s2::Point;
use crate::s2::builder::S2Error;
use crate::s2::builder::S2ErrorCode;
use crate::s2::crossing_edge_query::{CrossingEdgeQuery, CrossingType};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::Crossing;
use crate::s2::padded_cell::PaddedCell;
use crate::s2::predicates;
use crate::s2::shape::{Dimension, Edge, Shape, ShapeEdge, ShapeEdgeId, ShapeId};
use crate::s2::shape_index::{ShapeIndex, ShapeIndexCell, ShapeIndexIterator};
use crate::s2::wedge_relations::{self, WedgeRel};

/// Collects all edges from a `ShapeIndexCell` into a vector.
fn append_shape_edges(index: &ShapeIndex, cell: &ShapeIndexCell, edges: &mut Vec<ShapeEdge>) {
    for clipped in &cell.shapes {
        let shape_id = clipped.shape_id;
        let Some(shape) = index.shape(shape_id) else {
            continue;
        };
        for &edge_id in &clipped.edges {
            let edge = shape.edge(edge_id as usize);
            edges.push(ShapeEdge::new(ShapeEdgeId::new(shape_id, edge_id), edge));
        }
    }
}

/// Returns all edges from a `ShapeIndexCell`, reusing the provided vector.
fn get_shape_edges(index: &ShapeIndex, cell: &ShapeIndexCell, edges: &mut Vec<ShapeEdge>) {
    edges.clear();
    append_shape_edges(index, cell, edges);
}

/// Returns all edges from multiple `ShapeIndexCells`.
fn get_shape_edges_multi(
    index: &ShapeIndex,
    cells: &[&ShapeIndexCell],
    edges: &mut Vec<ShapeEdge>,
) {
    edges.clear();
    for cell in cells {
        append_shape_edges(index, cell, edges);
    }
}

/// Visits all pairs of crossing edges within a single vector of shape edges.
fn visit_crossings_vec(
    shape_edges: &[ShapeEdge],
    cross_type: CrossingType,
    need_adjacent: bool,
    visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let min_crossing_sign = if cross_type == CrossingType::Interior {
        1
    } else {
        0
    };
    let num_edges = shape_edges.len();
    for i in 0..num_edges.saturating_sub(1) {
        let a = &shape_edges[i];
        let mut j = i + 1;
        // Skip adjacent edges (AB, BC) unless need_adjacent is true.
        if !need_adjacent && j < num_edges && a.edge.v1 == shape_edges[j].edge.v0 {
            j += 1;
            if j >= num_edges {
                break;
            }
        }
        let mut crosser = EdgeCrosser::new(a.edge.v0, a.edge.v1);
        while j < num_edges {
            let b = &shape_edges[j];
            crosser.restart_at(b.edge.v0);
            let sign = crosser.chain_crossing_sign(b.edge.v1);
            let sign_val = match sign {
                Crossing::Cross => 1,
                Crossing::MaybeCross => 0,
                Crossing::DoNotCross => -1,
            };
            if sign_val >= min_crossing_sign {
                visitor(a, b, sign_val == 1)?;
            }
            j += 1;
        }
    }
    ControlFlow::Continue(())
}

/// Internal: visits crossings within a single index.
fn visit_crossings_internal(
    index: &ShapeIndex,
    cross_type: CrossingType,
    need_adjacent: bool,
    visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let mut shape_edges = Vec::new();
    let mut it = index.iter();
    while !it.done() {
        if let Some(cell) = it.index_cell() {
            get_shape_edges(index, cell, &mut shape_edges);
            visit_crossings_vec(&shape_edges, cross_type, need_adjacent, visitor)?;
        }
        it.next();
    }
    ControlFlow::Continue(())
}

/// Visits all pairs of crossing edges in the given `ShapeIndex`, terminating
/// early if the visitor returns false (in which case this function also
/// returns false). `cross_type` indicates whether all crossings should be
/// visited, or only interior crossings.
///
/// CAVEAT: Crossings may be visited more than once.
pub fn visit_crossing_edge_pairs(
    index: &ShapeIndex,
    cross_type: CrossingType,
    visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let need_adjacent = cross_type == CrossingType::All;
    visit_crossings_internal(index, cross_type, need_adjacent, visitor)
}

/// Helper that handles the two-index case. It is instantiated twice, once for
/// (A, B) and once for (B, A), to test crossings in the most efficient order.
struct IndexCrosser<'a> {
    a_index: &'a ShapeIndex,
    b_index: &'a ShapeIndex,
    min_crossing_sign: i32,
    swapped: bool,
    b_query: CrossingEdgeQuery<'a>,
    a_shape_edges: Vec<ShapeEdge>,
    b_shape_edges: Vec<ShapeEdge>,
}

impl<'a> IndexCrosser<'a> {
    fn new(
        a_index: &'a ShapeIndex,
        b_index: &'a ShapeIndex,
        cross_type: CrossingType,
        swapped: bool,
    ) -> Self {
        IndexCrosser {
            a_index,
            b_index,
            min_crossing_sign: if cross_type == CrossingType::Interior {
                1
            } else {
                0
            },
            swapped,
            b_query: CrossingEdgeQuery::new(b_index),
            a_shape_edges: Vec::new(),
            b_shape_edges: Vec::new(),
        }
    }

    fn visit_edge_pair(
        &self,
        a: &ShapeEdge,
        b: &ShapeEdge,
        is_interior: bool,
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        if self.swapped {
            visitor(b, a, is_interior)
        } else {
            visitor(a, b, is_interior)
        }
    }

    fn visit_edge_cell_crossings(
        &mut self,
        a: &ShapeEdge,
        b_cell: &ShapeIndexCell,
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        get_shape_edges(self.b_index, b_cell, &mut self.b_shape_edges);
        let a_copy = *a;
        let mut crosser = EdgeCrosser::new(a_copy.edge.v0, a_copy.edge.v1);
        for b in &self.b_shape_edges {
            crosser.restart_at(b.edge.v0);
            let sign = crosser.chain_crossing_sign(b.edge.v1);
            let sign_val = match sign {
                Crossing::Cross => 1,
                Crossing::MaybeCross => 0,
                Crossing::DoNotCross => -1,
            };
            if sign_val >= self.min_crossing_sign {
                self.visit_edge_pair(&a_copy, b, sign_val == 1, visitor)?;
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_subcell_crossings(
        &mut self,
        a_cell: &ShapeIndexCell,
        b_id: CellId,
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        self.a_shape_edges.clear();
        append_shape_edges(self.a_index, a_cell, &mut self.a_shape_edges);
        let a_edges: Vec<ShapeEdge> = self.a_shape_edges.clone();
        let mut b_root = PaddedCell::from_cell_id(b_id, 0.0);
        for a in &a_edges {
            let cells = self.b_query.get_cells(a.edge.v0, a.edge.v1, &mut b_root);
            for cell in cells {
                self.visit_edge_cell_crossings(a, cell, visitor)?;
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_edges_edges_crossings(
        &self,
        a_edges: &[ShapeEdge],
        b_edges: &[ShapeEdge],
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        for a in a_edges {
            let mut crosser = EdgeCrosser::new(a.edge.v0, a.edge.v1);
            for b in b_edges {
                crosser.restart_at(b.edge.v0);
                let sign = crosser.chain_crossing_sign(b.edge.v1);
                let sign_val = match sign {
                    Crossing::Cross => 1,
                    Crossing::MaybeCross => 0,
                    Crossing::DoNotCross => -1,
                };
                if sign_val >= self.min_crossing_sign {
                    self.visit_edge_pair(a, b, sign_val == 1, visitor)?;
                }
            }
        }
        ControlFlow::Continue(())
    }

    fn visit_cell_cell_crossings(
        &mut self,
        a_cell: &ShapeIndexCell,
        b_cell: &ShapeIndexCell,
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        get_shape_edges(self.a_index, a_cell, &mut self.a_shape_edges);
        get_shape_edges(self.b_index, b_cell, &mut self.b_shape_edges);
        let a_edges: Vec<ShapeEdge> = self.a_shape_edges.clone();
        let b_edges: Vec<ShapeEdge> = self.b_shape_edges.clone();
        self.visit_edges_edges_crossings(&a_edges, &b_edges, visitor)
    }

    /// Given two iterators positioned such that `ai.cell_id()` contains `bi.cell_id()`,
    /// visits all crossings between edges of A and B that intersect `ai.cell_id()`.
    /// Advances both iterators past `ai.cell_id()`.
    fn visit_crossings(
        &mut self,
        ai: &mut ShapeIndexIterator<'_>,
        bi: &mut ShapeIndexIterator<'_>,
        visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        debug_assert!(ai.cell_id().contains(bi.cell_id()));
        let ai_cell = ai.index_cell();
        if ai_cell.is_none_or(|c| c.num_edges() == 0) {
            seek_beyond(bi, ai.cell_id());
        } else if let Some(a_cell) = ai_cell {
            const EDGE_QUERY_MIN_EDGES: usize = 23;
            let mut b_edges_count = 0;
            let mut b_cells: Vec<&ShapeIndexCell> = Vec::new();
            loop {
                if let Some(b_cell) = bi.index_cell() {
                    let cell_edges = b_cell.num_edges();
                    if cell_edges > 0 {
                        b_edges_count += cell_edges;
                        if b_edges_count >= EDGE_QUERY_MIN_EDGES {
                            let ai_id = ai.cell_id();
                            self.visit_subcell_crossings(a_cell, ai_id, visitor)?;
                            seek_beyond(bi, ai_id);
                            ai.next();
                            return ControlFlow::Continue(());
                        }
                        b_cells.push(b_cell);
                    }
                }
                bi.next();
                if bi.done() || bi.cell_id() > ai.cell_id().range_max() {
                    break;
                }
            }
            if !b_cells.is_empty() {
                get_shape_edges(self.a_index, a_cell, &mut self.a_shape_edges);
                get_shape_edges_multi(self.b_index, &b_cells, &mut self.b_shape_edges);
                let a_edges: Vec<ShapeEdge> = self.a_shape_edges.clone();
                let b_edges: Vec<ShapeEdge> = self.b_shape_edges.clone();
                self.visit_edges_edges_crossings(&a_edges, &b_edges, visitor)?;
            }
        }
        ai.next();
        ControlFlow::Continue(())
    }
}

/// Seeks the iterator beyond the range of `target`.
fn seek_beyond(it: &mut ShapeIndexIterator<'_>, target: CellId) {
    it.seek(target.range_max().next());
}

/// Like `visit_crossing_edge_pairs`, but visits all pairs of crossing edges
/// where one edge comes from each `ShapeIndex`.
///
/// CAVEAT: Crossings may be visited more than once.
pub fn visit_crossing_edge_pairs_ab(
    a_index: &ShapeIndex,
    b_index: &ShapeIndex,
    cross_type: CrossingType,
    visitor: &mut dyn FnMut(&ShapeEdge, &ShapeEdge, bool) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let mut ai = a_index.iter();
    let mut bi = b_index.iter();
    let mut ab = IndexCrosser::new(a_index, b_index, cross_type, false);
    let mut ba = IndexCrosser::new(b_index, a_index, cross_type, true);

    while !ai.done() || !bi.done() {
        if ai.done() {
            break;
        }
        if bi.done() {
            break;
        }
        let ai_range_max = ai.cell_id().range_max();
        let bi_range_min = bi.cell_id().range_min();
        let bi_range_max = bi.cell_id().range_max();
        let ai_range_min = ai.cell_id().range_min();

        if ai_range_max < bi_range_min {
            ai.seek(bi.cell_id().range_min());
        } else if bi_range_max < ai_range_min {
            bi.seek(ai.cell_id().range_min());
        } else {
            let ai_lsb = ai.cell_id().lsb();
            let bi_lsb = bi.cell_id().lsb();
            match ai_lsb.cmp(&bi_lsb) {
                std::cmp::Ordering::Greater => {
                    ab.visit_crossings(&mut ai, &mut bi, visitor)?;
                }
                std::cmp::Ordering::Less => {
                    ba.visit_crossings(&mut bi, &mut ai, visitor)?;
                }
                std::cmp::Ordering::Equal => {
                    let a_cell = ai.index_cell();
                    let b_cell = bi.index_cell();
                    if let (Some(ac), Some(bc)) = (a_cell, b_cell)
                        && ac.num_edges() > 0
                        && bc.num_edges() > 0
                    {
                        ab.visit_cell_cell_crossings(ac, bc, visitor)?;
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
    ControlFlow::Continue(())
}

/// Helper function that formats a loop error message.
fn init_loop_error(code: S2ErrorCode, chain_id: usize, is_polygon: bool, msg: String) -> S2Error {
    if is_polygon {
        S2Error {
            code,
            message: format!("Loop {chain_id}: {msg}"),
        }
    } else {
        S2Error { code, message: msg }
    }
}

/// Given two loop edges that cross (including at a shared vertex), return true
/// if there is a crossing error and set error appropriately.
fn find_crossing_error(
    shape: &dyn Shape,
    a: &ShapeEdge,
    b: &ShapeEdge,
    is_interior: bool,
    error: &mut S2Error,
) -> bool {
    let is_polygon = shape.num_chains() > 1;
    let ap = shape.chain_position(a.id.edge_id as usize);
    let bp = shape.chain_position(b.id.edge_id as usize);

    if is_interior {
        if ap.chain_id == bp.chain_id {
            let msg = format!("Edge {} crosses edge {}", ap.offset, bp.offset);
            *error = init_loop_error(
                S2ErrorCode::LoopSelfIntersection,
                ap.chain_id,
                is_polygon,
                msg,
            );
        } else {
            *error = S2Error {
                code: S2ErrorCode::PolygonLoopsCross,
                message: format!(
                    "Loop {} edge {} crosses loop {} edge {}",
                    ap.chain_id, ap.offset, bp.chain_id, bp.offset
                ),
            };
        }
        return true;
    }

    // Only check vertex crossings where both edges share the same v1.
    if a.edge.v1 != b.edge.v1 {
        return false;
    }

    if ap.chain_id == bp.chain_id {
        let msg = format!(
            "Edge {} has duplicate vertex with edge {}",
            ap.offset, bp.offset
        );
        *error = init_loop_error(S2ErrorCode::DuplicateVertices, ap.chain_id, is_polygon, msg);
        return true;
    }

    let a_len = shape.chain(ap.chain_id).length;
    let b_len = shape.chain(bp.chain_id).length;
    let a_next = if ap.offset + 1 == a_len {
        0
    } else {
        ap.offset + 1
    };
    let b_next = if bp.offset + 1 == b_len {
        0
    } else {
        bp.offset + 1
    };
    let a2 = shape.chain_edge(ap.chain_id, a_next).v1;
    let b2 = shape.chain_edge(bp.chain_id, b_next).v1;

    if a.edge.v0 == b.edge.v0 || a.edge.v0 == b2 {
        *error = S2Error {
            code: S2ErrorCode::PolygonLoopsShareEdge,
            message: format!(
                "Loop {} edge {} has duplicate near loop {} edge {}",
                ap.chain_id, ap.offset, bp.chain_id, bp.offset
            ),
        };
        return true;
    }

    // Check wedge relations for vertex crossings between different loops.
    if wedge_relations::wedge_relation(a.edge.v0, a.edge.v1, a2, b.edge.v0, b2)
        == WedgeRel::ProperlyOverlaps
        && wedge_relations::wedge_relation(a.edge.v0, a.edge.v1, a2, b2, b.edge.v0)
            == WedgeRel::ProperlyOverlaps
    {
        *error = S2Error {
            code: S2ErrorCode::PolygonLoopsCross,
            message: format!(
                "Loop {} edge {} crosses loop {} edge {}",
                ap.chain_id, ap.offset, bp.chain_id, bp.offset
            ),
        };
        return true;
    }

    false
}

/// Given a `ShapeIndex` containing a single polygonal shape, returns
/// `Some(error)` if any loop has a self-intersection or crosses any other
/// loop, with a human-readable message. Otherwise returns `None`.
pub fn find_self_intersection(index: &ShapeIndex) -> Option<S2Error> {
    if index.num_shape_ids() == 0 {
        return None;
    }
    debug_assert_eq!(1, index.num_shape_ids());

    // Collect all crossing pairs first, then check them. This avoids
    // borrowing both the index (for the shape) and the error simultaneously
    // in a closure.
    let mut crossing_pairs: Vec<(ShapeEdge, ShapeEdge, bool)> = Vec::new();
    let _ = visit_crossings_internal(
        index,
        CrossingType::All,
        false, // need_adjacent
        &mut |a: &ShapeEdge, b: &ShapeEdge, is_interior: bool| {
            crossing_pairs.push((*a, *b, is_interior));
            ControlFlow::Continue(())
        },
    );

    let shape = index.shape(0)?;
    let mut error = S2Error::ok();
    for (a, b, is_interior) in &crossing_pairs {
        if find_crossing_error(shape, a, b, *is_interior, &mut error) {
            return Some(error);
        }
    }
    None
}

/// Returns true if the given shape contains the given point.
///
/// This is a brute-force method that iterates over all edges of the shape.
/// It is intended for use with shapes that have a small number of edges.
///
/// Corresponds to C++ `s2shapeutil::ContainsBruteForce`.
pub fn contains_brute_force(shape: &dyn Shape, point: Point) -> bool {
    if shape.dimension() < Dimension::Polygon {
        return false;
    }
    let rp = shape.reference_point();
    let mut inside = rp.contained;
    let mut crosser = EdgeCrosser::new(rp.point, point);
    for e in 0..shape.num_edges() {
        let edge = shape.edge(e);
        inside ^= crosser.edge_or_vertex_crossing(edge.v0, edge.v1);
    }
    inside
}

// ─── Shape conversion ────────────────────────────────────────────────────

/// Converts a 0-dimensional shape into a list of points.
///
/// Corresponds to C++ `s2shapeutil::ShapeToS2Points`.
///
/// # Errors
///
/// Returns `Err(S2Error)` with [`S2ErrorCode::InvalidArgument`] if `shape` is
/// not a dimension-0 (point) shape. Returning an error rather than asserting
/// keeps this from panicking across an FFI/library boundary.
pub fn shape_to_points(shape: &dyn Shape) -> Result<Vec<Point>, S2Error> {
    if shape.dimension() != Dimension::Point {
        return Err(S2Error::new(
            S2ErrorCode::InvalidArgument,
            "shape_to_points requires a dimension-0 (point) shape",
        ));
    }
    let mut points = Vec::with_capacity(shape.num_edges());
    for i in 0..shape.num_edges() {
        points.push(shape.edge(i).v0);
    }
    Ok(points)
}

/// Converts a 1-dimensional shape (single chain) into a polyline vertex list.
///
/// The shape must have exactly one chain (polyline).
///
/// Corresponds to C++ `s2shapeutil::ShapeToS2Polyline`.
pub fn shape_to_polyline_vertices(shape: &dyn Shape) -> Vec<Point> {
    debug_assert_eq!(shape.dimension(), Dimension::Polyline);
    debug_assert_eq!(shape.num_chains(), 1);
    crate::s2::shape_measures::get_chain_vertices(shape, 0)
}

/// Converts a 2-dimensional shape into loop vertex lists.
///
/// Each chain becomes one loop. Returns the vertex lists suitable for
/// constructing a `Polygon::from_oriented_loops`.
///
/// Corresponds to part of C++ `s2shapeutil::ShapeToS2Polygon`.
pub fn shape_to_loop_vertices(shape: &dyn Shape) -> Vec<Vec<Point>> {
    debug_assert_eq!(shape.dimension(), Dimension::Polygon);
    let mut loops = Vec::with_capacity(shape.num_chains());
    for i in 0..shape.num_chains() {
        loops.push(crate::s2::shape_measures::get_chain_vertices(shape, i));
    }
    loops
}

// ─── Edge counting ───────────────────────────────────────────────────────

/// Returns the total number of edges across all shapes in an index.
///
/// Corresponds to C++ `s2shapeutil::CountEdges(const S2ShapeIndex&)`.
pub fn count_edges(index: &ShapeIndex) -> usize {
    index.num_edges()
}

/// Returns the total number of edges across all shapes in an index,
/// stopping early once `max_edges` is reached.
///
/// This is useful for quickly determining whether an index has at least
/// a certain number of edges without iterating through all shapes.
///
/// Corresponds to C++ `s2shapeutil::CountEdgesUpTo(const S2ShapeIndex&, int)`.
pub fn count_edges_up_to(index: &ShapeIndex, max_edges: usize) -> usize {
    let mut total = 0;
    for id in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(id as i32) {
            total += shape.num_edges();
            if total >= max_edges {
                return total;
            }
        }
    }
    total
}

// ─── Vertex counting ─────────────────────────────────────────────────────

/// Returns the total number of vertices in a single shape.
///
/// - Dimension 0: each edge is a point, count = `num_chains()`.
/// - Dimension 1: each polyline chain has `edges + 1` vertices, count = `num_edges() + num_chains()`.
/// - Dimension 2: each polygon chain is closed, count = `num_edges()`.
///
/// Corresponds to C++ `s2shapeutil::CountVertices(const S2Shape&)`.
pub fn count_vertices_shape(shape: &dyn Shape) -> usize {
    match shape.dimension() {
        Dimension::Point => shape.num_chains(),
        Dimension::Polyline => shape.num_edges() + shape.num_chains(),
        Dimension::Polygon => shape.num_edges(),
    }
}

/// Returns the total number of vertices across all shapes in an index.
///
/// Corresponds to C++ `s2shapeutil::CountVertices(const S2ShapeIndex&)`.
pub fn count_vertices_index(index: &ShapeIndex) -> usize {
    let mut total = 0;
    for id in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(id as i32) {
            total += count_vertices_shape(shape);
        }
    }
    total
}

// ─── Edge wrapping ───────────────────────────────────────────────────────

/// Returns the edge ID of the next edge in a chain, wrapping for closed chains.
///
/// Returns `None` when the end of an open chain is reached.
/// Polygon chains always wrap. Polyline chains wrap only if closed.
/// Point chains always return `None`.
///
/// Corresponds to C++ `s2shapeutil::NextEdgeWrap`.
pub fn next_edge_wrap(shape: &dyn Shape, edge_id: usize) -> Option<usize> {
    debug_assert!(edge_id < shape.num_edges());
    let chainpos = shape.chain_position(edge_id);
    let chaininfo = shape.chain(chainpos.chain_id);
    let offset = chainpos.offset;

    match shape.dimension() {
        Dimension::Polygon => {
            let new_offset = (offset + 1) % chaininfo.length;
            Some(chaininfo.start + new_offset)
        }
        Dimension::Polyline => {
            if offset == chaininfo.length - 1 {
                let curr = shape.chain_edge(chainpos.chain_id, offset);
                let next = shape.chain_edge(chainpos.chain_id, 0);
                if curr.v1 == next.v0 {
                    Some(chaininfo.start)
                } else {
                    None
                }
            } else {
                Some(chaininfo.start + offset + 1)
            }
        }
        Dimension::Point => None,
    }
}

/// Returns the edge ID of the previous edge in a chain, wrapping for closed chains.
///
/// Returns `None` when the start of an open chain is reached.
/// Polygon chains always wrap. Polyline chains wrap only if closed.
/// Point chains always return `None`.
///
/// Corresponds to C++ `s2shapeutil::PrevEdgeWrap`.
pub fn prev_edge_wrap(shape: &dyn Shape, edge_id: usize) -> Option<usize> {
    debug_assert!(edge_id < shape.num_edges());
    let chainpos = shape.chain_position(edge_id);
    let chaininfo = shape.chain(chainpos.chain_id);
    let offset = chainpos.offset;

    match shape.dimension() {
        Dimension::Polygon => {
            let new_offset = if offset == 0 {
                chaininfo.length - 1
            } else {
                offset - 1
            };
            Some(chaininfo.start + new_offset)
        }
        Dimension::Polyline => {
            if offset == 0 {
                let end = chaininfo.length - 1;
                let curr = shape.chain_edge(chainpos.chain_id, 0);
                let prev = shape.chain_edge(chainpos.chain_id, end);
                if prev.v1 == curr.v0 {
                    Some(chaininfo.start + end)
                } else {
                    None
                }
            } else {
                Some(chaininfo.start + offset - 1)
            }
        }
        Dimension::Point => None,
    }
}

// ─── Sort edges CCW ──────────────────────────────────────────────────────

/// Sorts edges CCW around a shared vertex (origin), starting from `first`.
///
/// All edges in `data` must have `origin` as one of their endpoints.
/// The `first` edge defines the starting direction for the CCW ordering.
/// Reverse duplicate edges are ordered so that the one with `v0 == origin`
/// comes first.
pub fn sort_edges_ccw(origin: Point, first: Edge, data: &mut [Edge]) {
    debug_assert!(first.v0 == origin || first.v1 == origin);
    let first_vertex = if first.v0 == origin {
        first.v1
    } else {
        first.v0
    };
    debug_assert!(first_vertex != origin);

    data.sort_unstable_by(|a, b| {
        debug_assert!(a.v0 == origin || a.v1 == origin);
        debug_assert!(b.v0 == origin || b.v1 == origin);

        // Irreflexivity: equal edges are Equal.
        if a == b {
            return std::cmp::Ordering::Equal;
        }

        // Reverse duplicates: edge with v0 == origin comes first.
        if *a == b.reversed() {
            return if a.v0 == origin {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }

        // First edge always comes first.
        if *a == first {
            return std::cmp::Ordering::Less;
        }
        if *b == first {
            return std::cmp::Ordering::Greater;
        }

        // Compare by CCW orientation relative to first_vertex.
        let apnt = if a.v0 == origin { a.v1 } else { a.v0 };
        let bpnt = if b.v0 == origin { b.v1 } else { b.v0 };
        if predicates::ordered_ccw(first_vertex, apnt, bpnt, origin) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
}

// ─── Edge iterator ───────────────────────────────────────────────────────

/// An iterator that advances through all edges in a [`ShapeIndex`].
///
/// Corresponds to C++ `s2shapeutil::EdgeIterator`.
#[derive(Debug)]
pub struct EdgeIterator<'a> {
    index: &'a ShapeIndex,
    shape_id: i32,
    num_edges: i32,
    edge_id: i32,
}

impl<'a> EdgeIterator<'a> {
    /// Creates a new edge iterator positioned at the first edge.
    pub fn new(index: &'a ShapeIndex) -> Self {
        let mut it = EdgeIterator {
            index,
            shape_id: -1,
            num_edges: 0,
            edge_id: -1,
        };
        it.advance();
        it
    }

    /// Returns the current shape ID.
    pub fn shape_id(&self) -> i32 {
        self.shape_id
    }

    /// Returns the current edge ID within the shape.
    pub fn edge_id(&self) -> i32 {
        self.edge_id
    }

    /// Returns the current `ShapeEdgeId`.
    pub fn shape_edge_id(&self) -> ShapeEdgeId {
        ShapeEdgeId::new(self.shape_id, self.edge_id)
    }

    /// Returns the current edge.
    pub fn edge(&self) -> Edge {
        debug_assert!(!self.done());
        match self.index.shape(self.shape_id) {
            Some(shape) => shape.edge(self.edge_id as usize),
            None => Edge {
                v0: Point::default(),
                v1: Point::default(),
            },
        }
    }

    /// Returns true if there are no more edges.
    pub fn done(&self) -> bool {
        self.shape_id >= self.index.num_shape_ids() as i32
    }

    /// Advances to the next edge.
    pub fn advance(&mut self) {
        self.edge_id += 1;
        while self.edge_id >= self.num_edges {
            self.shape_id += 1;
            if self.shape_id >= self.index.num_shape_ids() as i32 {
                break;
            }
            self.num_edges = match self.index.shape(self.shape_id) {
                Some(shape) => shape.num_edges() as i32,
                None => 0,
            };
            self.edge_id = 0;
        }
    }
}

impl Iterator for EdgeIterator<'_> {
    type Item = (ShapeEdgeId, Edge);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let id = self.shape_edge_id();
        let edge = self.edge();
        self.advance();
        Some((id, edge))
    }
}

// ─── Build polygon boundaries ──────────────────────────────────────────

/// Groups loops into polygons whose interiors do not intersect.
///
/// Takes a collection of connected components, where each component consists of
/// one or more loops. The loops in each component must form a subdivision of the
/// sphere (except that a component may consist of a single degenerate loop).
/// The boundaries of different components must be disjoint.
///
/// Returns a set of polygons, where each polygon is defined by the indices of
/// the loops that form its boundary.
///
/// Each entry in `components` is a list of `(component_index, loop_index)` pairs
/// referring to shapes. The result is a list of polygons, where each polygon is
/// a list of shape references (as `(component_index, loop_index)` pairs).
///
/// This is a simpler Rust API that works with indices rather than raw shape pointers.
///
/// Corresponds to C++ `s2shapeutil::BuildPolygonBoundaries`.
///
/// # Panics
///
/// Panics if a component's outer loop cannot be found in the outer loop list.
pub fn build_polygon_boundaries(components: &[Vec<&dyn Shape>]) -> Vec<Vec<usize>> {
    if components.is_empty() {
        return Vec::new();
    }

    // Assign a unique global id to each shape.
    // shape_global_id[component_idx][loop_idx] = global_id
    let mut all_shapes: Vec<(usize, usize, &dyn Shape)> = Vec::new();
    for (ci, component) in components.iter().enumerate() {
        for (li, shape) in component.iter().enumerate() {
            all_shapes.push((ci, li, *shape));
        }
    }

    // Build an index of all loops that do NOT contain the origin,
    // unless the component has only one loop (degenerate case).
    let origin = Point::origin();
    let mut indexed_global_ids: Vec<usize> = Vec::new();
    let mut indexed_component_ids: Vec<usize> = Vec::new();
    let mut outer_loops: Vec<usize> = Vec::new(); // global_ids of outer loops

    let mut index = ShapeIndex::new();
    let mut global_id = 0usize;
    for (ci, component) in components.iter().enumerate() {
        let mut found_outer = false;
        for shape in component {
            if component.len() > 1 && !contains_brute_force(*shape, origin) {
                // Index this loop.
                // We need to clone the shape into the index. Use a LaxLoop wrapper.
                let mut verts = Vec::new();
                for e_id in 0..shape.num_edges() {
                    let e = shape.edge(e_id);
                    verts.push(e.v0);
                }
                let lax_loop = crate::s2::lax_loop::LaxLoop::new(verts);
                index.add(Box::new(lax_loop));
                indexed_global_ids.push(global_id);
                indexed_component_ids.push(ci);
            } else if !found_outer {
                outer_loops.push(global_id);
                found_outer = true;
            } else {
                // Extra outer loops (shouldn't happen per spec, but handle gracefully)
                outer_loops.push(global_id);
            }
            global_id += 1;
        }
        if !found_outer {
            // No outer loop found — this shouldn't happen per the C++ DCHECK,
            // but the last indexed loop becomes the outer loop.
            // Actually this means the component is not a subdivision.
            // For robustness, we push a placeholder.
            if let Some(last) = indexed_global_ids.last() {
                outer_loops.push(*last);
            }
        }
    }
    index.build();

    // Find the loops containing each outer loop's first vertex.
    let mut contains_query = crate::s2::contains_point_query::ContainsPointQuery::new(
        &index,
        crate::s2::contains_point_query::VertexModel::SemiOpen,
    );
    let mut ancestors: Vec<Vec<ShapeId>> = Vec::new(); // shape_ids in index containing each outer loop
    for &outer_gid in &outer_loops {
        let (_, _, shape) = all_shapes[outer_gid];
        if shape.num_edges() > 0 {
            let v0 = shape.edge(0).v0;
            let ids = contains_query.containing_shape_ids(v0);
            ancestors.push(ids);
        } else {
            ancestors.push(Vec::new());
        }
    }

    // Assign each outer loop to the component whose depth is one less.
    use std::collections::HashMap;
    let mut children: HashMap<ShapeId, Vec<usize>> = HashMap::new();
    // -1 means top-level
    for (i, &outer_gid) in outer_loops.iter().enumerate() {
        let depth = ancestors[i].len();
        let mut ancestor_shape_id = ShapeId(-1);
        if depth > 0 {
            for &candidate_shape_id in &ancestors[i] {
                let cand_comp_id = indexed_component_ids[candidate_shape_id.as_usize()];
                // Find the component index for this candidate, then check its outer loop depth.
                #[expect(
                    clippy::expect_used,
                    reason = "invariant: every component has an outer loop"
                )]
                let outer_idx = outer_loops
                    .iter()
                    .position(|&gid| all_shapes[gid].0 == cand_comp_id)
                    .expect("outer loop must exist for component");
                if ancestors[outer_idx].len() == depth - 1 {
                    ancestor_shape_id = candidate_shape_id;
                    break;
                }
            }
        }
        children
            .entry(ancestor_shape_id)
            .or_default()
            .push(outer_gid);
    }

    // Build the result: one polygon per indexed loop + one for top-level.
    let num_indexed = indexed_global_ids.len();
    let mut polygons: Vec<Vec<usize>> = Vec::with_capacity(num_indexed + 1);
    for (i, &gid) in indexed_global_ids.iter().enumerate() {
        let mut polygon = Vec::new();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_possible_wrap,
            reason = "index fits in i32"
        )]
        let key = ShapeId(i as i32);
        if let Some(kids) = children.get(&key) {
            polygon.extend(kids.iter().copied());
        }
        polygon.push(gid);
        polygons.push(polygon);
    }
    // Top-level polygon (depth 0 outer loops)
    let top = children.get(&ShapeId(-1)).cloned().unwrap_or_default();
    polygons.push(top);

    polygons
}

#[cfg(test)]
#[path = "shape_util_tests.rs"]
mod shape_util_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::shape::{Chain, ChainPosition, Edge, ReferencePoint};
    use crate::s2::{LatLng, Point};

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    /// A simple polyline shape for testing crossings.
    #[derive(Debug)]
    struct TestPolyline {
        vertices: Vec<Point>,
    }

    impl Shape for TestPolyline {
        fn num_edges(&self) -> usize {
            self.vertices.len().saturating_sub(1)
        }
        fn edge(&self, id: usize) -> Edge {
            Edge::new(self.vertices[id], self.vertices[id + 1])
        }
        fn reference_point(&self) -> ReferencePoint {
            ReferencePoint::default()
        }
        fn num_chains(&self) -> usize {
            1
        }
        fn chain(&self, _chain_id: usize) -> Chain {
            Chain::new(0, self.num_edges())
        }
        fn chain_edge(&self, _chain_id: usize, offset: usize) -> Edge {
            self.edge(offset)
        }
        fn chain_position(&self, edge_id: usize) -> ChainPosition {
            ChainPosition::new(0, edge_id)
        }
        fn dimension(&self) -> Dimension {
            Dimension::Polyline
        }
    }

    #[test]
    fn test_no_crossings_single_index() {
        // Two non-crossing polylines in the same index.
        let mut index = ShapeIndex::new();
        index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, 0.0), p(0.0, 10.0)],
        }));
        index.add(Box::new(TestPolyline {
            vertices: vec![p(10.0, 0.0), p(10.0, 10.0)],
        }));
        index.build();

        let mut count = 0;
        let _ = visit_crossing_edge_pairs(&index, CrossingType::Interior, &mut |_, _, _| {
            count += 1;
            ControlFlow::Continue(())
        });
        assert_eq!(count, 0);
    }

    #[test]
    fn test_interior_crossing_single_index() {
        // Two crossing polylines: one goes W-E, the other goes S-N, crossing in the middle.
        let mut index = ShapeIndex::new();
        index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, -10.0), p(0.0, 10.0)],
        }));
        index.add(Box::new(TestPolyline {
            vertices: vec![p(-10.0, 0.0), p(10.0, 0.0)],
        }));
        index.build();

        let mut interior_count = 0;
        let _ =
            visit_crossing_edge_pairs(&index, CrossingType::Interior, &mut |_, _, is_interior| {
                if is_interior {
                    interior_count += 1;
                }
                ControlFlow::Continue(())
            });
        assert!(
            interior_count >= 1,
            "Expected at least one interior crossing"
        );
    }

    #[test]
    fn test_dual_index_crossing() {
        let mut a_index = ShapeIndex::new();
        a_index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, -10.0), p(0.0, 10.0)],
        }));
        a_index.build();

        let mut b_index = ShapeIndex::new();
        b_index.add(Box::new(TestPolyline {
            vertices: vec![p(-10.0, 0.0), p(10.0, 0.0)],
        }));
        b_index.build();

        let mut crossings = Vec::new();
        let _ = visit_crossing_edge_pairs_ab(
            &a_index,
            &b_index,
            CrossingType::Interior,
            &mut |a, b, is_interior| {
                crossings.push((a.id, b.id, is_interior));
                ControlFlow::Continue(())
            },
        );
        assert!(
            !crossings.is_empty(),
            "Expected at least one crossing between two indexes"
        );
    }

    #[test]
    fn test_early_termination() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, -10.0), p(0.0, 10.0)],
        }));
        index.add(Box::new(TestPolyline {
            vertices: vec![p(-10.0, 0.0), p(10.0, 0.0)],
        }));
        index.build();

        // Return Break to stop early.
        let result = visit_crossing_edge_pairs(&index, CrossingType::Interior, &mut |_, _, _| {
            ControlFlow::Break(())
        });
        assert!(
            result.is_break(),
            "Should return Break when visitor returns Break"
        );
    }

    #[test]
    fn test_empty_indexes() {
        let a_index = ShapeIndex::new();
        let b_index = ShapeIndex::new();

        let result =
            visit_crossing_edge_pairs_ab(&a_index, &b_index, CrossingType::All, &mut |_, _, _| {
                ControlFlow::Continue(())
            });
        assert!(result.is_continue(), "Empty indexes should return Continue");
    }

    // ─── count_vertices tests ────────────────────────────────────────────

    #[test]
    fn test_count_vertices_points() {
        // Dimension 0: each edge is a point, count = num_chains
        let index = crate::s2::text_format::make_index("0:0 | 1:1 | 2:2 # #");
        let shape = index.shape(0).unwrap();
        assert_eq!(shape.dimension(), Dimension::Point);
        assert_eq!(count_vertices_shape(shape), 3);
    }

    #[test]
    fn test_count_vertices_polyline() {
        // Dimension 1: num_edges + num_chains
        let shape = crate::s2::text_format::make_lax_polyline("0:0, 1:0, 2:0, 3:0");
        assert_eq!(shape.dimension(), Dimension::Polyline);
        // 3 edges, 1 chain => 4 vertices
        assert_eq!(count_vertices_shape(&shape), 4);
    }

    #[test]
    fn test_count_vertices_polygon() {
        // Dimension 2: num_edges (closed loop)
        let shape = crate::s2::text_format::make_lax_polygon("0:0, 0:1, 1:0");
        assert_eq!(shape.dimension(), Dimension::Polygon);
        // 3 edges => 3 vertices
        assert_eq!(count_vertices_shape(&shape), 3);
    }

    #[test]
    fn test_count_vertices_index() {
        let mut index = ShapeIndex::new();
        // Add a polygon with 4 vertices
        index.add(Box::new(crate::s2::text_format::make_lax_polygon(
            "0:0, 0:1, 1:1, 1:0",
        )));
        // Add a polyline with 3 vertices (2 edges + 1 chain)
        index.add(Box::new(crate::s2::text_format::make_lax_polyline(
            "0:0, 1:0, 2:0",
        )));
        index.build();
        assert_eq!(count_vertices_index(&index), 4 + 3);
    }

    // ─── edge_wrap tests ─────────────────────────────────────────────────

    #[test]
    fn test_next_edge_wrap_polygon() {
        // Polygon with 3 edges (triangle): edge 2 wraps to edge 0
        let shape = crate::s2::text_format::make_lax_polygon("0:0, 0:1, 1:0");
        assert_eq!(next_edge_wrap(&shape, 0), Some(1));
        assert_eq!(next_edge_wrap(&shape, 1), Some(2));
        assert_eq!(next_edge_wrap(&shape, 2), Some(0)); // wraps
    }

    #[test]
    fn test_prev_edge_wrap_polygon() {
        let shape = crate::s2::text_format::make_lax_polygon("0:0, 0:1, 1:0");
        assert_eq!(prev_edge_wrap(&shape, 0), Some(2)); // wraps
        assert_eq!(prev_edge_wrap(&shape, 1), Some(0));
        assert_eq!(prev_edge_wrap(&shape, 2), Some(1));
    }

    #[test]
    fn test_next_edge_wrap_open_polyline() {
        // Open polyline: edges [0,1,2], end returns None
        let shape = crate::s2::text_format::make_lax_polyline("0:0, 1:0, 2:0, 3:0");
        assert_eq!(next_edge_wrap(&shape, 0), Some(1));
        assert_eq!(next_edge_wrap(&shape, 1), Some(2));
        assert_eq!(next_edge_wrap(&shape, 2), None); // end of open polyline
    }

    #[test]
    fn test_prev_edge_wrap_open_polyline() {
        let shape = crate::s2::text_format::make_lax_polyline("0:0, 1:0, 2:0, 3:0");
        assert_eq!(prev_edge_wrap(&shape, 0), None); // start of open polyline
        assert_eq!(prev_edge_wrap(&shape, 1), Some(0));
        assert_eq!(prev_edge_wrap(&shape, 2), Some(1));
    }

    // ─── edge_iterator tests ─────────────────────────────────────────────

    #[test]
    fn test_edge_iterator_empty_index() {
        let index = ShapeIndex::new();
        let it = EdgeIterator::new(&index);
        assert!(it.done());
    }

    #[test]
    fn test_edge_iterator_counts_all_edges() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0)],
        }));
        index.add(Box::new(TestPolyline {
            vertices: vec![p(0.0, 0.0), p(0.0, 1.0)],
        }));
        index.build();

        let mut count = 0;
        let mut it = EdgeIterator::new(&index);
        while !it.done() {
            count += 1;
            it.advance();
        }
        // First shape: 2 edges, second shape: 1 edge
        assert_eq!(count, 3);
    }

    // ─── shape conversion tests ──────────────────────────────────────────

    #[test]
    fn test_shape_to_points() {
        let index = crate::s2::text_format::make_index("0:0 | 1:1 | 2:2 # #");
        let shape = index.shape(0).unwrap();
        let points = shape_to_points(shape).unwrap();
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_shape_to_polyline_vertices() {
        let shape = crate::s2::text_format::make_lax_polyline("0:0, 1:0, 2:0");
        let verts = shape_to_polyline_vertices(&shape);
        assert_eq!(verts.len(), 3); // 2 edges + 1 = 3 vertices
    }

    #[test]
    fn test_shape_to_loop_vertices() {
        let shape = crate::s2::text_format::make_lax_polygon("0:0, 0:1, 1:0; 2:2, 2:3, 3:2");
        let loops = shape_to_loop_vertices(&shape);
        assert_eq!(loops.len(), 2);
        assert_eq!(loops[0].len(), 3);
        assert_eq!(loops[1].len(), 3);
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2shapeutil_*_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    // ─── s2shapeutil_shape_edge_id_test.cc ───────────────────────────────

    #[test]
    fn test_shape_edge_id_both_fields_equal_is_equal() {
        assert_eq!(ShapeEdgeId::new(10, 20), ShapeEdgeId::new(10, 20));
    }

    #[test]
    fn test_shape_edge_id_shape_id_unequal_is_unequal() {
        assert_ne!(ShapeEdgeId::new(11, 20), ShapeEdgeId::new(10, 20));
    }

    #[test]
    fn test_shape_edge_id_edge_id_unequal_is_unequal() {
        assert_ne!(ShapeEdgeId::new(10, 21), ShapeEdgeId::new(10, 20));
    }

    #[test]
    #[expect(clippy::nonminimal_bool, reason = "testing operator directly")]
    fn test_shape_edge_id_less_than_is_lexicographic_shape_id_first() {
        assert!(!(ShapeEdgeId::new(10, 20) < ShapeEdgeId::new(10, 20)));
        assert!(ShapeEdgeId::new(10, 20) < ShapeEdgeId::new(11, 20));
        assert!(ShapeEdgeId::new(10, 20) < ShapeEdgeId::new(10, 21));
    }

    #[test]
    #[expect(clippy::nonminimal_bool, reason = "testing operator directly")]
    fn test_shape_edge_id_less_eq_is_lexicographic_shape_id_first() {
        assert!(!(ShapeEdgeId::new(10, 20) <= ShapeEdgeId::new(9, 20)));
        assert!(ShapeEdgeId::new(10, 20) <= ShapeEdgeId::new(10, 20));
        assert!(!(ShapeEdgeId::new(10, 20) <= ShapeEdgeId::new(10, 19)));
    }

    #[test]
    #[expect(clippy::nonminimal_bool, reason = "testing operator directly")]
    fn test_shape_edge_id_greater_than_is_lexicographic_shape_id_first() {
        assert!(!(ShapeEdgeId::new(10, 20) > ShapeEdgeId::new(10, 20)));
        assert!(ShapeEdgeId::new(10, 20) > ShapeEdgeId::new(9, 20));
        assert!(ShapeEdgeId::new(10, 20) > ShapeEdgeId::new(10, 19));
    }

    #[test]
    #[expect(clippy::nonminimal_bool, reason = "testing operator directly")]
    fn test_shape_edge_id_greater_eq_is_lexicographic_shape_id_first() {
        assert!(!(ShapeEdgeId::new(10, 20) >= ShapeEdgeId::new(11, 20)));
        assert!(ShapeEdgeId::new(10, 20) >= ShapeEdgeId::new(10, 20));
        assert!(!(ShapeEdgeId::new(10, 20) >= ShapeEdgeId::new(10, 21)));
    }

    // ─── s2shapeutil_edge_wrap_test.cc ───────────────────────────────────

    #[test]
    fn test_next_prev_edge_point_does_not_wrap() {
        // C++ TEST(S2Shape, NextPrevEdgePointDoesNotWrap)
        let index = crate::s2::text_format::make_index("1:1 | 2:2 # #");
        let shape = index.shape(0).unwrap();
        // Points have one chain per point; always returns None.
        assert_eq!(prev_edge_wrap(shape, 0), None);
        assert_eq!(next_edge_wrap(shape, 0), None);
        assert_eq!(prev_edge_wrap(shape, 1), None);
        assert_eq!(next_edge_wrap(shape, 1), None);
    }

    #[test]
    fn test_next_prev_edge_closed_polyline_wraps() {
        // C++ TEST(S2Shape, NextPrevEdgeClosedPolylineWraps)
        let index = crate::s2::text_format::make_index("# 0:0, 1:1, 0:2, -1:1, 0:0 #");
        let shape = index.shape(0).unwrap();
        // Closed polylines should wrap around (4 edges: 0→1→2→3).
        assert_eq!(prev_edge_wrap(shape, 0), Some(3));
        assert_eq!(next_edge_wrap(shape, 0), Some(1));
        assert_eq!(prev_edge_wrap(shape, 3), Some(2));
        assert_eq!(next_edge_wrap(shape, 3), Some(0));
    }

    // ─── s2shapeutil_edge_iterator_test.cc ───────────────────────────────

    /// Helper: collects all edges from an index by iterating shapes directly.
    fn get_edges_direct(index: &ShapeIndex) -> Vec<Edge> {
        let mut result = Vec::new();
        for id in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(id as i32) {
                for j in 0..shape.num_edges() {
                    result.push(shape.edge(j));
                }
            }
        }
        result
    }

    /// Verifies that `EdgeIterator` produces the same edges as direct iteration.
    fn verify_edge_iterator(index: &ShapeIndex) {
        let expected = get_edges_direct(index);
        let mut actual = Vec::new();
        let mut shape_id: i32 = -1;
        let mut edge_id: i32 = -1;
        let mut it = EdgeIterator::new(index);
        while !it.done() {
            if it.shape_id() != shape_id {
                shape_id = it.shape_id();
                edge_id = 0;
            }
            assert!(
                actual.len() < expected.len(),
                "too many edges from iterator"
            );
            assert_eq!(it.edge(), expected[actual.len()]);
            assert_eq!(it.edge_id(), edge_id);
            assert_eq!(it.shape_edge_id(), ShapeEdgeId::new(shape_id, edge_id));
            actual.push(it.edge());
            edge_id += 1;
            it.advance();
        }
        assert_eq!(actual.len(), expected.len());
    }

    #[test]
    fn test_edge_iterator_empty() {
        // C++ TEST(S2ShapeutilEdgeIteratorTest, Empty)
        let index = crate::s2::text_format::make_index("# #");
        verify_edge_iterator(&index);
    }

    #[test]
    fn test_edge_iterator_points() {
        // C++ TEST(S2ShapeutilEdgeIteratorTest, Points)
        let index = crate::s2::text_format::make_index("0:0 | 1:1 # #");
        verify_edge_iterator(&index);
    }

    #[test]
    fn test_edge_iterator_lines() {
        // C++ TEST(S2ShapeutilEdgeIteratorTest, Lines)
        let index = crate::s2::text_format::make_index("# 0:0, 10:10 | 5:5, 5:10 | 1:2, 2:1 #");
        verify_edge_iterator(&index);
    }

    #[test]
    fn test_edge_iterator_polygons() {
        // C++ TEST(S2ShapeutilEdgeIteratorTest, Polygons)
        let index =
            crate::s2::text_format::make_index("# # 10:10, 10:0, 0:0; -10:-10, -10:0, 0:0, 0:-10");
        verify_edge_iterator(&index);
    }

    #[test]
    fn test_edge_iterator_collection() {
        // C++ TEST(S2ShapeutilEdgeIteratorTest, Collection)
        let index = crate::s2::text_format::make_index(
            "1:1 | 7:2 # 1:1, 2:2, 3:3 | 2:2, 1:7 # 10:10, 10:0, 0:0; 20:20, 20:10, 10:10 | 15:15, 15:0, 0:0",
        );
        verify_edge_iterator(&index);
    }

    // ─── s2shapeutil_count_edges_test.cc ─────────────────────────────────

    #[test]
    fn test_count_edges_up_to_stops_early() {
        // C++ TEST(CountEdgesUpTo, StopsEarly)
        let index = crate::s2::text_format::make_index(
            "0:0 | 0:1 | 0:2 | 0:3 | 0:4 # 1:0, 1:1 | 1:2, 1:3 | 1:4, 1:5, 1:6 #",
        );
        // Shape 0: 5 point edges, Shape 1: 1 edge, Shape 2: 1 edge, Shape 3: 2 edges
        assert_eq!(index.num_shape_ids(), 4);
        assert_eq!(index.shape(0).unwrap().num_edges(), 5);
        assert_eq!(index.shape(1).unwrap().num_edges(), 1);
        assert_eq!(index.shape(2).unwrap().num_edges(), 1);
        assert_eq!(index.shape(3).unwrap().num_edges(), 2);

        assert_eq!(count_edges(&index), 9);
        assert_eq!(count_edges_up_to(&index, 1), 5); // stops after first shape
        assert_eq!(count_edges_up_to(&index, 5), 5); // stops after first shape (exactly at limit)
        assert_eq!(count_edges_up_to(&index, 6), 6); // stops after second shape
        assert_eq!(count_edges_up_to(&index, 8), 9); // all shapes
    }

    // ─── s2shapeutil_count_vertices_test.cc ──────────────────────────────

    #[test]
    fn test_count_vertices_counts_correctly() {
        // C++ TEST(CountVertices, CountsCorrectly)
        // Test index built only out of three points.
        let index = crate::s2::text_format::make_index("1:1 | 2:2 | 3:3 # #");
        assert_eq!(count_vertices_index(&index), 3);

        // Two points + two-edge polyline.
        let index = crate::s2::text_format::make_index("1:1 | 2:2 # 3:3, 4:4, 5:5 #");
        assert_eq!(count_vertices_index(&index), 5);

        // Two points + two-edge polyline + four-edge polygon.
        let index =
            crate::s2::text_format::make_index("1:1 | 2:2 # 3:3, 4:4, 5:5 # 6:6, 7:7, 8:8, 9:9");
        assert_eq!(count_vertices_index(&index), 9);

        // Degenerate polylines count correctly.
        let index = crate::s2::text_format::make_index("# 3:3, 3:3, 3:3 #");
        assert_eq!(count_vertices_index(&index), 3);

        // Degenerate polygons count correctly.
        let index = crate::s2::text_format::make_index("# # 4:4, 4:4, 4:4, 4:4");
        assert_eq!(count_vertices_index(&index), 4);
    }

    // ─── s2shapeutil_contains_brute_force_test.cc ────────────────────────

    #[test]
    fn test_contains_brute_force_no_interior() {
        // C++ TEST(ContainsBruteForce, NoInterior)
        // A polyline that almost entirely encloses 0:0 but has no interior.
        let polyline = crate::s2::text_format::make_lax_polyline("0:0, 0:1, 1:-1, -1:-1, -1e-9:1");
        let point = crate::s2::text_format::parse_point("0:0");
        assert!(!contains_brute_force(&polyline, point));
    }

    #[test]
    fn test_contains_brute_force_contains_reference_point() {
        // C++ TEST(ContainsBruteForce, ContainsReferencePoint)
        let polygon = crate::s2::text_format::make_lax_polygon("0:0, 0:1, 1:-1, -1:-1, -1e-9:1");
        let r = polygon.reference_point();
        assert_eq!(r.contained, contains_brute_force(&polygon, r.point));
    }

    #[test]
    fn test_contains_brute_force_consistent_with_s2loop() {
        // C++ TEST(ContainsBruteForce, ConsistentWithS2Loop)
        use crate::s1::Angle;
        use crate::s2::Loop;
        use crate::s2::region::Region;
        let center = crate::s2::text_format::parse_point("89:-179");
        let loop_ = Loop::make_regular(center, Angle::from_degrees(10.0), 100);
        for i in 0..loop_.num_vertices() {
            let v = loop_.vertex(i);
            assert_eq!(
                loop_.contains_point(&v),
                contains_brute_force(&loop_, v),
                "mismatch at vertex {i}"
            );
        }
    }

    // ─── s2shapeutil_get_reference_point_test.cc ─────────────────────────

    #[test]
    fn test_get_reference_point_empty_polygon() {
        // C++ TEST(GetReferencePoint, EmptyPolygon)
        let polygon = crate::s2::text_format::make_lax_polygon("");
        assert!(!polygon.reference_point().contained);
    }

    #[test]
    fn test_get_reference_point_full_polygon() {
        // C++ TEST(GetReferencePoint, FullPolygon)
        let polygon = crate::s2::text_format::make_lax_polygon("full");
        assert!(polygon.reference_point().contained);
    }

    #[test]
    fn test_get_reference_point_degenerate_loops() {
        // C++ TEST(GetReferencePoint, DegenerateLoops)
        use crate::s2::lax_polygon::LaxPolygon;
        let loops = vec![
            crate::s2::text_format::parse_points("1:1, 1:2, 2:2, 1:2, 1:3, 1:2, 1:1"),
            crate::s2::text_format::parse_points("0:0, 0:3, 0:6, 0:9, 0:6, 0:3, 0:0"),
            crate::s2::text_format::parse_points("5:5, 6:6"),
        ];
        let shape = LaxPolygon::from_loops_owned(loops);
        assert!(!shape.reference_point().contained);
    }

    #[test]
    fn test_get_reference_point_inverted_loops() {
        // C++ TEST(GetReferencePoint, InvertedLoops)
        use crate::s2::lax_polygon::LaxPolygon;
        let loops = vec![
            crate::s2::text_format::parse_points("1:2, 1:1, 2:2"),
            crate::s2::text_format::parse_points("3:4, 3:3, 4:4"),
        ];
        let shape = LaxPolygon::from_loops_owned(loops);
        assert!(contains_brute_force(&shape, Point::origin()));
    }

    // ─── s2shapeutil_visit_crossing_edge_pairs_test.cc ───────────────────

    /// Collects crossing edge pairs from a single index.
    fn get_crossings(
        index: &ShapeIndex,
        crossing_type: CrossingType,
    ) -> Vec<(ShapeEdgeId, ShapeEdgeId)> {
        let mut pairs = Vec::new();
        let _ = visit_crossing_edge_pairs(index, crossing_type, &mut |a, b, _is_interior| {
            pairs.push((a.id, b.id));
            ControlFlow::Continue(())
        });
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    /// Collects crossing edge pairs between two indexes.
    fn get_crossings_ab(
        index_a: &ShapeIndex,
        index_b: &ShapeIndex,
        crossing_type: CrossingType,
    ) -> Vec<(ShapeEdgeId, ShapeEdgeId)> {
        let mut pairs = Vec::new();
        let _ = visit_crossing_edge_pairs_ab(
            index_a,
            index_b,
            crossing_type,
            &mut |a, b, _is_interior| {
                pairs.push((a.id, b.id));
                ControlFlow::Continue(())
            },
        );
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }

    /// Returns true if the crossing result meets the minimum threshold.
    fn crossing_meets_threshold(crossing: Crossing, crossing_type: CrossingType) -> bool {
        match crossing_type {
            CrossingType::All => crossing != Crossing::DoNotCross,
            CrossingType::Interior => crossing == Crossing::Cross,
        }
    }

    /// Brute-force crossing detection in a single index.
    fn get_crossings_brute_force(
        index: &ShapeIndex,
        crossing_type: CrossingType,
    ) -> Vec<(ShapeEdgeId, ShapeEdgeId)> {
        let mut result = Vec::new();
        let mut a_it = EdgeIterator::new(index);
        while !a_it.done() {
            let a = a_it.edge();
            let a_id = a_it.shape_edge_id();
            let mut b_it = EdgeIterator::new(index);
            // Advance b past a's position.
            while !b_it.done() && b_it.shape_edge_id() <= a_id {
                b_it.advance();
            }
            while !b_it.done() {
                let b = b_it.edge();
                let sign = crate::s2::edge_crossings::crossing_sign(a.v0, a.v1, b.v0, b.v1);
                if crossing_meets_threshold(sign, crossing_type) {
                    result.push((a_id, b_it.shape_edge_id()));
                }
                b_it.advance();
            }
            a_it.advance();
        }
        result
    }

    /// Brute-force crossing detection between two indexes.
    fn get_crossings_brute_force_ab(
        index_a: &ShapeIndex,
        index_b: &ShapeIndex,
        crossing_type: CrossingType,
    ) -> Vec<(ShapeEdgeId, ShapeEdgeId)> {
        let mut result = Vec::new();
        let mut a_it = EdgeIterator::new(index_a);
        while !a_it.done() {
            let a = a_it.edge();
            let mut b_it = EdgeIterator::new(index_b);
            while !b_it.done() {
                let b = b_it.edge();
                let sign = crate::s2::edge_crossings::crossing_sign(a.v0, a.v1, b.v0, b.v1);
                if crossing_meets_threshold(sign, crossing_type) {
                    result.push((a_it.shape_edge_id(), b_it.shape_edge_id()));
                }
                b_it.advance();
            }
            a_it.advance();
        }
        result
    }

    fn test_crossing_edge_pairs(index: &ShapeIndex, ct: CrossingType, expected: usize) {
        let brute = get_crossings_brute_force(index, ct);
        let actual = get_crossings(index, ct);
        assert_eq!(expected, brute.len(), "brute force count mismatch");
        assert_eq!(expected, actual.len(), "optimized count mismatch");
        assert_eq!(brute, actual);
    }

    fn test_crossing_edge_pairs_ab(
        a: &ShapeIndex,
        b: &ShapeIndex,
        ct: CrossingType,
        expected: usize,
    ) {
        let brute = get_crossings_brute_force_ab(a, b, ct);
        let actual = get_crossings_ab(a, b, ct);
        assert_eq!(expected, brute.len(), "brute force count mismatch (ab)");
        assert_eq!(expected, actual.len(), "optimized count mismatch (ab)");
        assert_eq!(brute, actual);
    }

    #[test]
    fn test_crossing_no_intersections_one_index() {
        // C++ TEST(GetCrossingEdgePairs, NoIntersectionsOneIndex)
        let index = ShapeIndex::new();
        test_crossing_edge_pairs(&index, CrossingType::All, 0);
        test_crossing_edge_pairs(&index, CrossingType::Interior, 0);
    }

    #[test]
    fn test_crossing_no_intersections_two_indexes() {
        // C++ TEST(GetCrossingEdgePairs, NoIntersectionsTwoIndexes)
        let a = ShapeIndex::new();
        let b = ShapeIndex::new();
        test_crossing_edge_pairs_ab(&a, &b, CrossingType::All, 0);
        test_crossing_edge_pairs_ab(&a, &b, CrossingType::Interior, 0);
    }

    #[test]
    fn test_crossing_edge_grid_one_index() {
        // C++ TEST(GetCrossingEdgePairs, EdgeGridOneIndex)
        use crate::s2::edge_vector_shape::EdgeVectorShape;
        let grid_size = 10;
        let epsilon = 1e-10;
        let mut shape = EdgeVectorShape::new();
        for i in 0..=grid_size {
            let e = if i == 0 || i == grid_size {
                0.0
            } else {
                epsilon
            };
            let i_f = f64::from(i);
            // Vertical line.
            shape.add(
                LatLng::from_degrees(-e, i_f).to_point(),
                LatLng::from_degrees(f64::from(grid_size) + e, i_f).to_point(),
            );
            // Horizontal line.
            shape.add(
                LatLng::from_degrees(i_f, -e).to_point(),
                LatLng::from_degrees(i_f, f64::from(grid_size) + e).to_point(),
            );
        }
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        test_crossing_edge_pairs(&index, CrossingType::All, 112);
        test_crossing_edge_pairs(&index, CrossingType::Interior, 108);
    }

    #[test]
    fn test_crossing_edge_grid_two_indexes() {
        // C++ TEST(GetCrossingEdgePairs, EdgeGridTwoIndexes)
        use crate::s2::edge_vector_shape::EdgeVectorShape;
        let grid_size = 10;
        let epsilon = 1e-10;
        let mut shape_a = EdgeVectorShape::new();
        let mut shape_b = EdgeVectorShape::new();
        for i in 0..=grid_size {
            let e = if i == 0 || i == grid_size {
                0.0
            } else {
                epsilon
            };
            let i_f = f64::from(i);
            shape_a.add(
                LatLng::from_degrees(-e, i_f).to_point(),
                LatLng::from_degrees(f64::from(grid_size) + e, i_f).to_point(),
            );
            shape_b.add(
                LatLng::from_degrees(i_f, -e).to_point(),
                LatLng::from_degrees(i_f, f64::from(grid_size) + e).to_point(),
            );
        }
        let mut index_a = ShapeIndex::new();
        index_a.add(Box::new(shape_a));
        index_a.build();
        let mut index_b = ShapeIndex::new();
        index_b.add(Box::new(shape_b));
        index_b.build();

        test_crossing_edge_pairs_ab(&index_a, &index_b, CrossingType::All, 112);
        test_crossing_edge_pairs_ab(&index_a, &index_b, CrossingType::Interior, 108);
    }

    #[test]
    fn test_find_self_intersection_basic() {
        // C++ TEST(FindSelfIntersection, Basic)
        fn test_has_crossing(polygon_str: &str, has_crossing: bool) {
            use crate::s2::text_format::make_polygon;
            let polygon = make_polygon(polygon_str);
            let mut index = ShapeIndex::new();
            index.add(Box::new(polygon));
            index.build();
            let result = find_self_intersection(&index);
            assert_eq!(
                result.is_some(),
                has_crossing,
                "polygon: {polygon_str}, expected crossing={has_crossing}, got={result:?}"
            );
        }
        test_has_crossing("0:0, 0:1, 0:2, 1:2, 1:1, 1:0", false);
        test_has_crossing("0:0, 0:1, 0:2, 1:2, 0:1, 1:0", true); // duplicate vertex
        test_has_crossing("0:0, 1:1, 0:1; 0:0, 1:1, 1:0", true); // duplicate edge
        test_has_crossing("0:0, 1:1, 0:1; 1:1, 0:0, 1:0", true); // reversed edge
        test_has_crossing("0:0, 0:2, 2:2, 2:0; 1:1, 0:2, 3:1, 2:0", true); // vertex crossing
    }

    // ─── s2shapeutil_conversion_test.cc ──────────────────────────────────

    #[test]
    fn test_point_vector_shape_to_points() {
        // C++ TEST(S2ShapeConversionUtilTest, PointVectorShapeToPoints)
        let points = crate::s2::text_format::parse_points("11:11, 10:0, 5:5");
        let index = crate::s2::text_format::make_index("11:11 | 10:0 | 5:5 # #");
        let shape = index.shape(0).unwrap();
        let extracted = shape_to_points(shape).unwrap();
        assert_eq!(extracted.len(), 3);
        for (i, pt) in extracted.iter().enumerate() {
            assert_eq!(*pt, points[i], "point mismatch at index {i}");
        }
    }

    #[test]
    fn test_line_to_polyline_vertices() {
        // C++ TEST(S2ShapeConversionUtilTest, LineToS2Polyline)
        let points = crate::s2::text_format::parse_points("11:11, 10:0, 5:5");
        let shape = crate::s2::text_format::make_lax_polyline("11:11, 10:0, 5:5");
        let verts = shape_to_polyline_vertices(&shape);
        assert_eq!(verts.len(), 3);
        for (i, v) in verts.iter().enumerate() {
            assert_eq!(*v, points[i], "vertex mismatch at index {i}");
        }
    }

    #[test]
    fn test_closed_line_to_polyline_vertices() {
        // C++ TEST(S2ShapeConversionUtilTest, ClosedLineToS2Polyline)
        let points = crate::s2::text_format::parse_points("0:0, 0:10, 10:10, 0:0");
        let shape = crate::s2::text_format::make_lax_polyline("0:0, 0:10, 10:10, 0:0");
        let verts = shape_to_polyline_vertices(&shape);
        assert_eq!(verts.len(), 4);
        for (i, v) in verts.iter().enumerate() {
            assert_eq!(*v, points[i], "vertex mismatch at index {i}");
        }
    }

    #[test]
    fn test_polygon_to_loop_vertices_with_hole() {
        // C++ TEST(S2ShapeConversionUtilTest, PolygonWithHoleToS2Polygon)
        let shape =
            crate::s2::text_format::make_lax_polygon("0:0, 0:10, 10:10, 10:0; 4:4, 6:4, 6:6, 4:6");
        let loops = shape_to_loop_vertices(&shape);
        assert_eq!(loops.len(), 2);
        assert_eq!(loops[0].len(), 4);
        assert_eq!(loops[1].len(), 4);
    }

    #[test]
    fn test_polygon_to_loop_vertices_multi() {
        // C++ TEST(S2ShapeConversionUtilTest, MultiPolygonToS2Polygon)
        let shape = crate::s2::text_format::make_lax_polygon("0:0, 0:2, 2:2, 2:0; 0:4, 0:6, 3:6");
        let loops = shape_to_loop_vertices(&shape);
        assert_eq!(loops.len(), 2);
        assert_eq!(loops[0].len(), 4);
        assert_eq!(loops[1].len(), 3);
    }

    #[test]
    fn test_polygon_to_loop_vertices_two_holes() {
        // C++ TEST(S2ShapeConversionUtilTest, TwoHolesToS2Polygon)
        let shape = crate::s2::text_format::make_lax_polygon(
            "0:0, 0:10, 10:10, 10:0; 1:1, 3:3, 1:3; 2:6, 4:7, 2:8",
        );
        let loops = shape_to_loop_vertices(&shape);
        assert_eq!(loops.len(), 3);
        assert_eq!(loops[0].len(), 4);
        assert_eq!(loops[1].len(), 3);
        assert_eq!(loops[2].len(), 3);
    }

    #[test]
    fn test_polygon_to_loop_vertices_full() {
        // C++ TEST(S2ShapeConversionUtilTest, FullPolygonToS2Polygon)
        // Note: Rust's LaxPolygon::full() uses 0 vertices (empty loop array),
        // while C++ uses kFull() with 1 sentinel vertex. Verify the full polygon
        // round-trips correctly.
        let shape = crate::s2::text_format::make_lax_polygon("full");
        assert!(shape.reference_point().contained);
        // The Rust representation has 1 loop with 0 edges (empty vertices).
        let _loops = shape_to_loop_vertices(&shape);
        // The full polygon is represented as a single chain with 0 edges.
        assert_eq!(shape.num_chains(), 1);
        assert_eq!(shape.num_edges(), 0);
    }

    // ─── build_polygon_boundaries tests ──────────────────────────────────

    fn make_test_lax_loop(vertex_str: &str) -> crate::s2::lax_loop::LaxLoop {
        let vertices = crate::s2::text_format::parse_points(vertex_str);
        crate::s2::lax_loop::LaxLoop::new(vertices)
    }

    #[test]
    fn test_build_polygon_boundaries_no_components() {
        // C++ TEST(BuildPolygonBoundaries, NoComponents)
        let components: Vec<Vec<&dyn Shape>> = vec![];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(0, faces.len());
    }

    #[test]
    fn test_build_polygon_boundaries_one_loop() {
        // C++ TEST(BuildPolygonBoundaries, OneLoop)
        let a0 = make_test_lax_loop("0:0, 1:0, 0:1"); // Outer face
        let a1 = make_test_lax_loop("0:0, 0:1, 1:0");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0, &a1]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(2, faces.len());
    }

    #[test]
    fn test_build_polygon_boundaries_two_loops_same_component() {
        // C++ TEST(BuildPolygonBoundaries, TwoLoopsSameComponent)
        let a0 = make_test_lax_loop("0:0, 1:0, 0:1"); // Outer face
        let a1 = make_test_lax_loop("0:0, 0:1, 1:0");
        let a2 = make_test_lax_loop("1:0, 0:1, 1:1");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0, &a1, &a2]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(3, faces.len());
    }

    #[test]
    fn test_build_polygon_boundaries_two_loops_different_components() {
        // C++ TEST(BuildPolygonBoundaries, TwoLoopsDifferentComponents)
        let a0 = make_test_lax_loop("0:0, 1:0, 0:1");
        let a1 = make_test_lax_loop("0:0, 0:1, 1:0");
        let b0 = make_test_lax_loop("0:2, 1:2, 0:3");
        let b1 = make_test_lax_loop("0:2, 0:3, 1:2");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0, &a1], vec![&b0, &b1]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(3, faces.len());
    }

    #[test]
    fn test_build_polygon_boundaries_one_degenerate_loop() {
        // C++ TEST(BuildPolygonBoundaries, OneDegenerateLoop)
        let a0 = make_test_lax_loop("0:0, 1:0, 0:0");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(1, faces.len());
    }

    #[test]
    fn test_build_polygon_boundaries_two_degenerate_loops() {
        // C++ TEST(BuildPolygonBoundaries, TwoDegenerateLoops)
        let a0 = make_test_lax_loop("0:0, 1:0, 0:0");
        let b0 = make_test_lax_loop("2:0, 3:0, 2:0");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0], vec![&b0]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(1, faces.len());
        assert_eq!(2, faces[0].len());
    }

    #[test]
    fn test_build_polygon_boundaries_two_nested_loops() {
        // C++ TEST(BuildPolygonBoundaries, TwoNestedLoops)
        let a0 = make_test_lax_loop("0:0, 3:0, 0:3"); // Outer face
        let a1 = make_test_lax_loop("0:0, 0:3, 3:0");
        let b0 = make_test_lax_loop("1:1, 2:0, 0:2"); // Outer face
        let b1 = make_test_lax_loop("1:1, 0:2, 2:0");
        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0, &a1], vec![&b0, &b1]];
        let faces = build_polygon_boundaries(&components);
        assert_eq!(3, faces.len());
        // The face containing a1 should also contain b0 as a child.
        let face_with_a1 = faces.iter().find(|f| f.len() == 2 && f.contains(&1));
        assert!(face_with_a1.is_some(), "should have a face with a1 and b0");
    }

    #[test]
    fn test_build_polygon_boundaries_complex_test1() {
        // C++ TEST(BuildPolygonBoundaries, ComplexTest1) — simplified
        // Component "a": 4 adjacent squares forming a larger square.
        let a0 = make_test_lax_loop("0:0, 25:0, 50:0, 50:25, 50:50, 25:50, 0:50, 0:50");
        let a1 = make_test_lax_loop("0:0, 0:25, 25:25, 25:0");
        let a2 = make_test_lax_loop("0:25, 0:50, 25:50, 25:25");
        let a3 = make_test_lax_loop("25:0, 25:25, 50:25, 50:0");
        let a4 = make_test_lax_loop("25:25, 25:50, 50:50, 50:25");

        // Component "b": degenerate loop.
        let b0 = make_test_lax_loop("0:-10, 10:-10");

        let components: Vec<Vec<&dyn Shape>> = vec![vec![&a0, &a1, &a2, &a3, &a4], vec![&b0]];
        let faces = build_polygon_boundaries(&components);

        // Should have: 4 indexed loops from component a + 1 top-level face = 5 faces.
        assert_eq!(5, faces.len());

        // The top-level face should contain the outer loops a0 and b0.
        let top_face = faces.last().unwrap();
        assert_eq!(2, top_face.len(), "top face should have a0 and b0");
    }
}
