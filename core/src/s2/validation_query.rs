// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Geometry validation for S2 shape indexes.
//!
//! Provides two validation queries with different strictness levels:
//!
//! - [`S2ValidQuery`] — least strict, compatible with `S2BooleanOperation`
//! - [`S2LegacyValidQuery`] — stricter, matching `S2Polygon::IsValid()` semantics
//!
//! Corresponds to C++ `s2validation_query.h`.

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/ShapeId (i32) used as Vec indices in validation"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "ShapeId/EdgeId (usize<->i32) for shape index iteration"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for ShapeId/EdgeId — always in range"
)]
use std::collections::{HashMap, HashSet};

use crate::s2::Point;
use crate::s2::builder::{S2Error, S2ErrorCode};
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::contains_vertex_query::ContainsVertexQuery;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::Crossing;
use crate::s2::shape::{Dimension, Edge, Shape, ShapeId};
use crate::s2::shape_index::{ClippedShape, ShapeIndex, ShapeIndexIterator};
use crate::s2::shape_util::{self, sort_edges_ccw};

// ─── Options ──────────────────────────────────────────────────────────────

/// Types of single-vertex touches allowed between shapes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchType {
    /// Allow no touches between shapes.
    #[default]
    None = 0b00,
    /// Interior point may touch the other shape.
    Interior = 0b01,
    /// Boundary point may touch the other shape.
    Boundary = 0b10,
    /// Allow any touches between shapes.
    Any = 0b11,
}

/// Configuration for validation queries.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[expect(clippy::struct_excessive_bools, reason = "matches C++ structure")]
pub struct ValidationOptions {
    /// Whether degenerate edges {A,A} are allowed.
    pub allow_degenerate_edges: bool,
    /// Whether duplicate polyline edges are allowed.
    pub allow_duplicate_polyline_edges: bool,
    /// Whether reverse-duplicate edges {A,B},{B,A} are allowed.
    pub allow_reverse_duplicates: bool,
    /// Whether polyline edges can cross each other.
    pub allow_polyline_interior_crossings: bool,
    /// Whether to enforce legacy validation semantics.
    pub legacy_mode: bool,
    /// Touch matrix: `allowed_touches`[min(dima,dimb)][max(dima,dimb)]
    /// Each entry is (`TypeA`, `TypeB`).
    allowed_touches: [[(TouchType, TouchType); 3]; 3],
}

impl Default for ValidationOptions {
    fn default() -> Self {
        let any = (TouchType::Any, TouchType::Any);
        ValidationOptions {
            allow_degenerate_edges: true,
            allow_duplicate_polyline_edges: true,
            allow_reverse_duplicates: true,
            allow_polyline_interior_crossings: true,
            legacy_mode: false,
            allowed_touches: [[any; 3]; 3],
        }
    }
}

impl ValidationOptions {
    fn allowed_touches(&self, dima: Dimension, dimb: Dimension) -> (TouchType, TouchType) {
        let (a, b) = if dima > dimb {
            (dimb, dima)
        } else {
            (dima, dimb)
        };
        self.allowed_touches[a.as_usize()][b.as_usize()]
    }
}

// ─── Helper: point validity ──────────────────────────────────────────────

fn valid_point(p: Point) -> bool {
    p.0.x.is_finite() && p.0.y.is_finite() && p.0.z.is_finite()
}

fn is_unit_length(p: Point) -> bool {
    let n2 = p.0.norm2();
    // C++ S2::IsUnitLength uses 1e-15 tolerance.
    (n2 - 1.0).abs() <= 1e-15
}

// ─── Incident edge tracking ──────────────────────────────────────────────

/// Key for the incident edge map: (`shape_id`, vertex bit pattern).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct VertexKey {
    shape_id: ShapeId,
    x_bits: u64,
    y_bits: u64,
    z_bits: u64,
}

impl VertexKey {
    fn new(shape_id: ShapeId, vertex: Point) -> Self {
        VertexKey {
            shape_id,
            x_bits: vertex.0.x.to_bits(),
            y_bits: vertex.0.y.to_bits(),
            z_bits: vertex.0.z.to_bits(),
        }
    }

    fn vertex(&self) -> Point {
        Point(crate::r3::Vector::new(
            f64::from_bits(self.x_bits),
            f64::from_bits(self.y_bits),
            f64::from_bits(self.z_bits),
        ))
    }
}

/// Tracks which edge IDs are incident on each (`shape_id`, vertex) pair.
struct IncidentEdgeTracker {
    edges: HashMap<VertexKey, HashSet<i32>>,
}

impl IncidentEdgeTracker {
    fn new() -> Self {
        IncidentEdgeTracker {
            edges: HashMap::new(),
        }
    }

    fn add_edge(&mut self, shape_id: ShapeId, edge_id: i32, edge: Edge) {
        self.edges
            .entry(VertexKey::new(shape_id, edge.v0))
            .or_default()
            .insert(edge_id);
        self.edges
            .entry(VertexKey::new(shape_id, edge.v1))
            .or_default()
            .insert(edge_id);
    }
}

// ─── Edge info for per-cell processing ───────────────────────────────────

/// An edge with its metadata from a cell.
#[derive(Clone, Debug)]
struct CellEdge {
    v0: Point,
    v1: Point,
    edge_id: i32,
    shape_id: ShapeId,
    chain_id: usize,
    offset: usize,
    dim: Dimension,
}

/// Collects edges from a cell, sorted by dimension.
fn collect_cell_edges(
    index: &ShapeIndex,
    cell: &crate::s2::shape_index::ShapeIndexCell,
) -> Vec<CellEdge> {
    let mut edges = Vec::new();
    for clipped in &cell.shapes {
        let Some(shape) = index.shape(clipped.shape_id) else {
            continue;
        };
        let dim = shape.dimension();
        for &edge_id in &clipped.edges {
            let edge = shape.edge(edge_id as usize);
            let pos = shape.chain_position(edge_id as usize);
            edges.push(CellEdge {
                v0: edge.v0,
                v1: edge.v1,
                edge_id,
                shape_id: clipped.shape_id,
                chain_id: pos.chain_id,
                offset: pos.offset,
                dim,
            });
        }
    }
    // Sort by dimension so we can partition easily.
    edges.sort_by_key(|e| e.dim);
    edges
}

// ─── S2ValidQuery ─────────────────────────────────────────────────────────

/// Least-strict validation query, compatible with `S2BooleanOperation`.
///
/// Checks:
/// - Points are unit magnitude and finite
/// - No antipodal edges
/// - Polygon chains are closed, connected, and interior-on-left
/// - Polygon interiors are disjoint from all other geometry
/// - No duplicate polygon edges
#[derive(Debug)]
pub struct S2ValidQuery {
    options: ValidationOptions,
}

impl Default for S2ValidQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl S2ValidQuery {
    /// Creates a new validation query with default options.
    pub fn new() -> Self {
        S2ValidQuery {
            options: ValidationOptions::default(),
        }
    }

    /// Returns a reference to the options.
    pub fn options(&self) -> &ValidationOptions {
        &self.options
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut ValidationOptions {
        &mut self.options
    }

    /// Validates an index. Returns `Ok(())` if valid, `Err(S2Error)` with
    /// details on failure.
    ///
    /// # Errors
    ///
    /// Returns an `S2Error` describing the first validation failure found
    /// (e.g., non-unit-length vertices, self-intersections, duplicate edges).
    pub fn validate(&self, index: &ShapeIndex) -> Result<(), S2Error> {
        let mut error = S2Error::ok();
        if self.validate_inner(index, &mut error) {
            Ok(())
        } else {
            Err(error)
        }
    }

    fn validate_inner(&self, index: &ShapeIndex, error: &mut S2Error) -> bool {
        let mut tracker = IncidentEdgeTracker::new();

        // Phase 1: Per-shape checks.
        let mut iter = index.iter();
        for shape_id in (0..index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = index.shape(shape_id) else {
                continue;
            };
            if !self.check_shape(&mut iter, shape, shape_id, error) {
                return false;
            }
        }

        // Phase 2: Per-cell checks.
        iter.begin();
        while !iter.done() {
            let Some(cell) = iter.index_cell() else {
                iter.next();
                continue;
            };

            let cell_edges = collect_cell_edges(index, cell);

            // Track 2D edges for vertex crossing checks.
            for e in &cell_edges {
                if e.dim == Dimension::Polygon {
                    tracker.add_edge(e.shape_id, e.edge_id, Edge::new(e.v0, e.v1));
                }
            }

            // Check for duplicate edges.
            if !self.check_for_duplicate_edges(index, &cell_edges, error) {
                return false;
            }

            // Check for interior crossings.
            if !self.check_for_interior_crossings(&cell_edges, error) {
                return false;
            }

            // Check touches.
            if !self.check_touches_are_valid(index, cell, &cell_edges, error) {
                return false;
            }

            // Legacy: check duplicate vertices within chains.
            if self.options.legacy_mode
                && !Self::check_duplicate_vertices_in_chain(&cell_edges, error)
            {
                return false;
            }

            // Check points for containment in polygons.
            let cell_center = iter.cell_id().to_point();
            for e in &cell_edges {
                if e.dim == Dimension::Point
                    && self.point_contained(index, cell, cell_center, e.shape_id, e.v0, error)
                {
                    return false;
                }
            }

            iter.next();
        }

        // Phase 3: Global checks.
        // Vertex crossing check for 2D shapes.
        for (key, edge_ids) in &tracker.edges {
            let Some(shape) = index.shape(key.shape_id) else {
                continue;
            };
            if shape.dimension() == Dimension::Polygon
                && !check_vertex_crossings(key.vertex(), shape, key.shape_id, edge_ids, error)
            {
                return false;
            }
        }

        // Containment check: no chain's first vertex should be inside another polygon.
        let mut query = ContainsPointQuery::new(index, VertexModel::Open);
        for shape_id in (0..index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = index.shape(shape_id) else {
                continue;
            };
            if shape.dimension() == Dimension::Point {
                continue;
            }
            for chain_id in 0..shape.num_chains() {
                let chain = shape.chain(chain_id);
                if chain.length < 1 {
                    continue;
                }
                let vertex = shape.chain_edge(chain_id, 0).v0;
                if query.contains(vertex) {
                    *error = S2Error::new(
                        S2ErrorCode::OverlappingGeometry,
                        format!("Shape {shape_id} has edges contained in another shape."),
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Per-shape validation checks.
    fn check_shape(
        &self,
        iter: &mut ShapeIndexIterator<'_>,
        shape: &dyn Shape,
        shape_id: ShapeId,
        error: &mut S2Error,
    ) -> bool {
        let dim = shape.dimension();
        // With `Dimension` as an enum, invalid dimension values (> 2) are
        // impossible — the check is now enforced at compile time.

        let mut chains_to_check = Vec::new();

        for chain_id in 0..shape.num_chains() {
            let chain = shape.chain(chain_id);

            // Polygon chains must be closed.
            if dim == Dimension::Polygon && chain.length > 0 {
                let first_edge = shape.chain_edge(chain_id, 0);
                let last_edge = shape.chain_edge(chain_id, chain.length - 1);
                if last_edge.v1 != first_edge.v0 {
                    *error = S2Error::new(
                        S2ErrorCode::LoopNotEnoughVertices,
                        format!("Chain {chain_id} of shape {shape_id} isn't closed"),
                    );
                    return false;
                }
            }

            for offset in 0..chain.length {
                let edge = shape.chain_edge(chain_id, offset);

                // Check for inf/nan coordinates.
                if !valid_point(edge.v0) || !valid_point(edge.v1) {
                    *error = S2Error::new(
                        S2ErrorCode::InvalidVertex,
                        format!("Shape {shape_id} has invalid coordinates"),
                    );
                    return false;
                }

                // Check unit length.
                if !is_unit_length(edge.v0) || !is_unit_length(edge.v1) {
                    *error = S2Error::new(
                        S2ErrorCode::NotUnitLength,
                        format!("Shape {shape_id} has non-unit length vertices"),
                    );
                    return false;
                }

                // Check for degenerate edges.
                if dim > Dimension::Point
                    && !self.options.allow_degenerate_edges
                    && edge.v0 == edge.v1
                {
                    *error = S2Error::new(
                        S2ErrorCode::DuplicateVertices,
                        format!(
                            "Shape {}: chain {}, edge {} is degenerate",
                            shape_id,
                            chain_id,
                            chain.start + offset,
                        ),
                    );
                    return false;
                }

                // Check for antipodal vertices.
                if edge.v0 == Point(-edge.v1.0) {
                    *error = S2Error::new(
                        S2ErrorCode::AntipodalVertices,
                        format!("Shape {shape_id} has adjacent antipodal vertices"),
                    );
                    return false;
                }

                // Check chain connectivity.
                if dim > Dimension::Point && chain.length >= 2 && offset > 0 {
                    let prev = shape.chain_edge(chain_id, offset - 1);
                    if prev.v1 != edge.v0 {
                        *error = S2Error::new(
                            S2ErrorCode::NotContinuous,
                            format!(
                                "Chain {chain_id} of shape {shape_id} has neighboring edges that don't connect.",
                            ),
                        );
                        return false;
                    }
                }
            }

            // Polygon chain orientation check: need at least 2 distinct points.
            if dim != Dimension::Polygon || chain.length == 0 {
                continue;
            }

            let first_vertex = shape.chain_edge(chain_id, 0).v0;
            let mut has_distinct = false;
            for offset in 0..chain.length {
                let v = shape.chain_edge(chain_id, offset).v0;
                if v != first_vertex {
                    has_distinct = true;
                    break;
                }
            }
            if !has_distinct {
                continue;
            }
            chains_to_check.push(chain_id);
        }

        // Check chain orientation for selected chains.
        for chain_id in chains_to_check {
            if !self.check_chain_orientation(iter, shape, shape_id, chain_id, error) {
                return false;
            }
        }

        true
    }

    /// Checks that a polygon chain is oriented with interior on the left.
    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn check_chain_orientation(
        &self,
        iter: &mut ShapeIndexIterator<'_>,
        shape: &dyn Shape,
        shape_id: ShapeId,
        chain_id: usize,
        error: &mut S2Error,
    ) -> bool {
        let chain = shape.chain(chain_id);
        let mut query = ContainsVertexQuery::new(Point::default());

        for offset in 0..chain.length {
            let vertex = shape.chain_edge(chain_id, offset).v0;
            query.init(vertex);

            // Seek to the cell containing this vertex.
            if !iter.locate_point(vertex) {
                *error = S2Error::new(S2ErrorCode::DataLoss, "Shape vertex was not indexed");
                return false;
            }

            let center = iter.cell_id().to_point();
            let Some(cell) = iter.index_cell() else {
                *error = S2Error::new(S2ErrorCode::DataLoss, "Shape vertex was not indexed");
                return false;
            };

            let Some(clipped) = cell.find_by_shape_id(shape_id) else {
                *error = S2Error::new(S2ErrorCode::DataLoss, "Shape vertex was not indexed");
                return false;
            };

            // Compute winding number and vertex sign together.
            let mut winding = i32::from(clipped.contains_center);
            let mut crosser = EdgeCrosser::new(center, vertex);

            for &edge_id in &clipped.edges {
                let edge = shape.edge(edge_id as usize);
                winding += crosser.signed_edge_or_vertex_crossing(edge.v0, edge.v1);

                // Include edges incident on vertex.
                if edge.v0 == vertex {
                    query.add_edge(edge.v1, 1);
                } else if edge.v1 == vertex {
                    query.add_edge(edge.v0, -1);
                }
            }

            let duplicates = query.duplicate_edges();
            if !duplicates {
                let sign = query.contains_vertex();
                if sign == 0 {
                    continue;
                }
                let expected_winding = if sign < 0 { 0 } else { 1 };
                if winding != expected_winding {
                    *error = S2Error::new(
                        S2ErrorCode::PolygonInconsistentLoopOrientations,
                        format!(
                            "Shape {shape_id} has one or more edges with interior on the right."
                        ),
                    );
                    return false;
                }
                return true;
            }

            // Duplicate edges at this vertex; try the next one.
        }

        true
    }

    /// Checks for duplicate edges in a cell.
    fn check_for_duplicate_edges(
        &self,
        _index: &ShapeIndex,
        cell_edges: &[CellEdge],
        error: &mut S2Error,
    ) -> bool {
        let dim0 = if self.options.allow_duplicate_polyline_edges {
            Dimension::Polygon
        } else {
            Dimension::Polyline
        };

        // Get edges in the dim range [dim0, 2].
        let edges: Vec<&CellEdge> = cell_edges.iter().filter(|e| e.dim >= dim0).collect();

        for i in 0..edges.len() {
            for j in (i + 1)..edges.len() {
                let mut duplicate = edges[i].v0 == edges[j].v0 && edges[i].v1 == edges[j].v1;
                if !self.options.allow_reverse_duplicates {
                    duplicate |= edges[i].v0 == edges[j].v1 && edges[i].v1 == edges[j].v0;
                }
                if duplicate {
                    *error = S2Error::new(
                        S2ErrorCode::OverlappingGeometry,
                        "One or more duplicate polygon edges detected",
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Checks for interior crossings between edges in a cell.
    fn check_for_interior_crossings(&self, cell_edges: &[CellEdge], error: &mut S2Error) -> bool {
        // Get polyline and polygon edges (dim >= 1).
        let edges: Vec<&CellEdge> = cell_edges
            .iter()
            .filter(|e| e.dim >= Dimension::Polyline)
            .collect();

        if edges.is_empty() {
            return true;
        }

        // Find where polygon edges start.
        let polyline_count = edges
            .iter()
            .filter(|e| e.dim == Dimension::Polyline)
            .count();
        let check_start = if self.options.allow_polyline_interior_crossings {
            polyline_count
        } else {
            0
        };

        if check_start >= edges.len() {
            return true;
        }

        for i in 0..edges.len().saturating_sub(1) {
            let mut j = if i + 1 > check_start {
                i + 1
            } else {
                check_start
            };

            // Skip adjacent edges.
            if j < edges.len() && edges[i].v1 == edges[j].v0 {
                j += 1;
            }

            if j >= edges.len() {
                continue;
            }

            let mut crosser = EdgeCrosser::new(edges[i].v0, edges[i].v1);
            for k in j..edges.len() {
                if crosser.crossing_sign(edges[k].v0, edges[k].v1) == Crossing::Cross {
                    *error = S2Error::new(
                        S2ErrorCode::OverlappingGeometry,
                        format!(
                            "Chain {} edge {} crosses chain {} edge {}",
                            edges[i].chain_id, edges[i].offset, edges[k].chain_id, edges[k].offset,
                        ),
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Checks that vertex touches are valid under configured semantics.
    fn check_touches_are_valid(
        &self,
        index: &ShapeIndex,
        _cell: &crate::s2::shape_index::ShapeIndexCell,
        cell_edges: &[CellEdge],
        error: &mut S2Error,
    ) -> bool {
        // Check if all touches are allowed — if so, skip.
        let any = (TouchType::Any, TouchType::Any);
        let dims = [Dimension::Point, Dimension::Polyline, Dimension::Polygon];
        // need_check[d] is false when all dimension pairs involving d allow Any touches.
        let mut need_check = [true; 3];
        for (idx, &di) in dims.iter().enumerate() {
            let mut all_any = true;
            for &dj in &dims {
                if self.options.allowed_touches(di, dj) != any {
                    all_any = false;
                    break;
                }
            }
            need_check[idx] = !all_any;
        }

        if !need_check[0] && !need_check[1] && !need_check[2] {
            return true;
        }

        // Gather test vertices.
        struct TestVertex {
            vertex: Point,
            edge_id: i32,
            shape_id: ShapeId,
            dim: Dimension,
            on_boundary: bool,
        }

        let mut test_vertices = Vec::new();
        for e in cell_edges {
            if !need_check[e.dim.as_usize()] {
                continue;
            }
            let Some(shape) = index.shape(e.shape_id) else {
                continue;
            };
            if e.dim == Dimension::Polyline {
                let on_boundary = polyline_vertex_is_boundary(shape, e.edge_id as usize, 0);
                test_vertices.push(TestVertex {
                    vertex: e.v0,
                    edge_id: e.edge_id,
                    shape_id: e.shape_id,
                    dim: e.dim,
                    on_boundary,
                });
                let on_boundary = polyline_vertex_is_boundary(shape, e.edge_id as usize, 1);
                if on_boundary {
                    test_vertices.push(TestVertex {
                        vertex: e.v1,
                        edge_id: e.edge_id,
                        shape_id: e.shape_id,
                        dim: e.dim,
                        on_boundary: true,
                    });
                }
            } else {
                test_vertices.push(TestVertex {
                    vertex: e.v0,
                    edge_id: e.edge_id,
                    shape_id: e.shape_id,
                    dim: e.dim,
                    on_boundary: e.dim == Dimension::Polygon,
                });
            }
        }

        // Check each test vertex against all edges.
        for tv in &test_vertices {
            for e in cell_edges {
                // Don't compare an edge against itself.
                if tv.shape_id == e.shape_id && tv.edge_id == e.edge_id {
                    continue;
                }

                let vertidx = if tv.vertex == e.v0 {
                    0
                } else if tv.vertex == e.v1 {
                    1
                } else {
                    continue;
                };

                // Skip closed polyline self-touches.
                if tv.shape_id == e.shape_id
                    && e.dim == Dimension::Polyline
                    && let Some(shape) = index.shape(e.shape_id)
                {
                    if vertidx == 0
                        && let Some(prev) = shape_util::prev_edge_wrap(shape, e.edge_id as usize)
                        && prev == tv.edge_id as usize
                    {
                        continue;
                    }
                    if vertidx == 1
                        && let Some(next) = shape_util::next_edge_wrap(shape, e.edge_id as usize)
                        && next == tv.edge_id as usize
                    {
                        continue;
                    }
                }

                let on_boundary = if e.dim == Dimension::Polygon {
                    true
                } else if e.dim == Dimension::Polyline {
                    if let Some(shape) = index.shape(e.shape_id) {
                        polyline_vertex_is_boundary(shape, e.edge_id as usize, vertidx)
                    } else {
                        false
                    }
                } else {
                    false
                };

                let typea = if tv.on_boundary {
                    TouchType::Boundary
                } else {
                    TouchType::Interior
                };
                let typeb = if on_boundary {
                    TouchType::Boundary
                } else {
                    TouchType::Interior
                };

                let allowed = self.options.allowed_touches(tv.dim, e.dim);
                let permitted_ab =
                    (allowed.0 as u8 & typea as u8 != 0) && (allowed.1 as u8 & typeb as u8 != 0);
                let permitted_ba =
                    (allowed.0 as u8 & typeb as u8 != 0) && (allowed.1 as u8 & typea as u8 != 0);

                if !permitted_ab && !permitted_ba {
                    *error = S2Error::new(
                        S2ErrorCode::OverlappingGeometry,
                        "Index has geometry with invalid vertex touches.",
                    );
                    return false;
                }
            }
        }
        true
    }

    /// Checks if a point is contained in any polygon in the cell.
    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn point_contained(
        &self,
        index: &ShapeIndex,
        cell: &crate::s2::shape_index::ShapeIndexCell,
        cell_center: Point,
        point_shape_id: ShapeId,
        point: Point,
        error: &mut S2Error,
    ) -> bool {
        for clipped in &cell.shapes {
            if clipped.shape_id == point_shape_id {
                continue;
            }
            let Some(shape) = index.shape(clipped.shape_id) else {
                continue;
            };
            if shape.dimension() != Dimension::Polygon {
                continue;
            }
            if shape_contains_in_cell(clipped, shape, cell_center, point) {
                *error = S2Error::new(
                    S2ErrorCode::OverlappingGeometry,
                    format!(
                        "Shape {point_shape_id} has one or more edges contained in another shape.",
                    ),
                );
                return true;
            }
        }
        false
    }

    /// Legacy mode: checks for duplicate vertices within the same chain.
    fn check_duplicate_vertices_in_chain(cell_edges: &[CellEdge], error: &mut S2Error) -> bool {
        for i in 0..cell_edges.len() {
            for j in (i + 1)..cell_edges.len() {
                if cell_edges[j].chain_id != cell_edges[i].chain_id {
                    continue;
                }
                if cell_edges[j].shape_id != cell_edges[i].shape_id {
                    continue;
                }
                if cell_edges[j].v0 == cell_edges[i].v0 {
                    *error = S2Error::new(
                        S2ErrorCode::DuplicateVertices,
                        format!(
                            "Chain {} of shape {} has duplicate vertices",
                            cell_edges[i].chain_id, cell_edges[i].shape_id,
                        ),
                    );
                    return false;
                }
            }
        }
        true
    }
}

// ─── S2LegacyValidQuery ───────────────────────────────────────────────────

/// Stricter validation query matching `S2Polygon::IsValid()` semantics.
///
/// Additional constraints beyond `S2ValidQuery`:
/// - No degenerate edges
/// - All shapes must have the same dimension
/// - No duplicate vertices within a chain
/// - Polygon chains must have at least 3 edges
#[derive(Debug)]
pub struct S2LegacyValidQuery {
    inner: S2ValidQuery,
}

impl Default for S2LegacyValidQuery {
    fn default() -> Self {
        Self::new()
    }
}

impl S2LegacyValidQuery {
    /// Creates a new legacy validation query with stricter defaults.
    pub fn new() -> Self {
        let mut q = S2ValidQuery::new();
        q.options.allow_degenerate_edges = false;
        q.options.allow_reverse_duplicates = false;
        q.options.legacy_mode = true;
        S2LegacyValidQuery { inner: q }
    }

    /// Validates an index. Returns `Ok(())` if valid, `Err(S2Error)` on failure.
    ///
    /// # Errors
    ///
    /// Returns an `S2Error` describing the first validation failure found
    /// under legacy validation rules.
    pub fn validate(&self, index: &ShapeIndex) -> Result<(), S2Error> {
        let mut error = S2Error::ok();
        if self.validate_inner(index, &mut error) {
            Ok(())
        } else {
            Err(error)
        }
    }

    fn validate_inner(&self, index: &ShapeIndex, error: &mut S2Error) -> bool {
        // Check: all shapes must have the same dimension.
        let mut dim: Option<Dimension> = None;
        for shape_id in (0..index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = index.shape(shape_id) else {
                continue;
            };
            let d = shape.dimension();
            match dim {
                Some(prev) if prev != d => {
                    *error = S2Error::new(
                        S2ErrorCode::InvalidDimension,
                        "Mixed dimensional geometry is invalid for legacy semantics.",
                    );
                    return false;
                }
                _ => dim = Some(d),
            }
        }

        // Check: polygon chains must have >= 3 edges.
        for shape_id in (0..index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = index.shape(shape_id) else {
                continue;
            };
            if shape.dimension() == Dimension::Polygon {
                let mut has_empty = false;
                for chain_id in 0..shape.num_chains() {
                    let chain = shape.chain(chain_id);
                    if chain.length == 0 {
                        has_empty = true;
                    } else if chain.length < 3 {
                        *error = S2Error::new(
                            S2ErrorCode::LoopNotEnoughVertices,
                            format!(
                                "Shape {shape_id} has a non-empty chain with less than three edges.",
                            ),
                        );
                        return false;
                    }
                }
                if has_empty && shape.num_chains() > 1 {
                    *error = S2Error::new(
                        S2ErrorCode::PolygonEmptyLoop,
                        format!("Shape {shape_id} has too many empty chains"),
                    );
                    return false;
                }
            }
        }

        // Delegate to the inner S2ValidQuery.
        self.inner.validate_inner(index, error)
    }
}

// ─── Helper functions ─────────────────────────────────────────────────────

/// Returns true if a polyline vertex is a boundary point (start or end of open chain).
fn polyline_vertex_is_boundary(shape: &dyn Shape, edge_id: usize, vertex: usize) -> bool {
    debug_assert!(vertex == 0 || vertex == 1);
    let pos = shape.chain_position(edge_id);
    let chain = shape.chain(pos.chain_id);

    if pos.offset == 0 && vertex == 0 {
        return shape_util::prev_edge_wrap(shape, edge_id).is_none();
    }
    if pos.offset == chain.length - 1 && vertex == 1 {
        return shape_util::next_edge_wrap(shape, edge_id).is_none();
    }
    false
}

/// Checks if a shape contains a point within a cell, using edge crossing
/// from the cell center to the point.
fn shape_contains_in_cell(
    clipped: &ClippedShape,
    shape: &dyn Shape,
    cell_center: Point,
    point: Point,
) -> bool {
    let mut inside = clipped.contains_center;
    if clipped.edges.is_empty() {
        return inside;
    }

    let mut crosser = EdgeCrosser::new(cell_center, point);
    for &edge_id in &clipped.edges {
        let edge = shape.edge(edge_id as usize);
        if crosser.edge_or_vertex_crossing(edge.v0, edge.v1) {
            inside = !inside;
        }
    }
    inside
}

/// Checks that polygon chains don't cross at a vertex.
fn check_vertex_crossings(
    vertex: Point,
    shape: &dyn Shape,
    shape_id: ShapeId,
    edge_ids: &HashSet<i32>,
    error: &mut S2Error,
) -> bool {
    if edge_ids.len() < 2 {
        return true;
    }

    // Build edges with metadata and sort CCW.
    struct EdgeWithInfo {
        edge: Edge,
        edge_id: i32,
        chain_id: usize,
        prev_id: Option<usize>,
        sign: i32, // -1 for outgoing (v0 == vertex), +1 for incoming
    }

    let mut edges_info: Vec<EdgeWithInfo> = Vec::new();
    for &edge_id in edge_ids {
        let pos = shape.chain_position(edge_id as usize);
        let edge = shape.edge(edge_id as usize);
        let prev = shape_util::prev_edge_wrap(shape, edge_id as usize);
        let sign = if edge.v0 == vertex { -1 } else { 1 };
        edges_info.push(EdgeWithInfo {
            edge,
            edge_id,
            chain_id: pos.chain_id,
            prev_id: prev,
            sign,
        });
    }

    // Sort the edges CCW around the vertex.
    let mut raw_edges: Vec<Edge> = edges_info.iter().map(|e| e.edge).collect();
    if raw_edges.is_empty() {
        return true;
    }
    let first = raw_edges[0];
    sort_edges_ccw(vertex, first, &mut raw_edges);

    // Rebuild edges_info in sorted order.
    let mut sorted_info: Vec<&EdgeWithInfo> = Vec::with_capacity(edges_info.len());
    for sorted_edge in &raw_edges {
        // Find matching edge_info.
        if let Some(idx) = edges_info.iter().position(|e| e.edge == *sorted_edge) {
            sorted_info.push(&edges_info[idx]);
        }
    }

    // For each outgoing edge (sign == -1), scan CCW until we find its matching
    // incoming edge. All chain sums should be zero at that point.
    let n = sorted_info.len();
    let mut chain_sums: HashMap<usize, i32> = HashMap::new();

    for i in 0..n {
        let curr = sorted_info[i];
        if curr.sign > 0 {
            continue; // Skip incoming edges.
        }

        chain_sums.clear();
        let mut found = false;
        for j in 1..n {
            let edge = sorted_info[(i + j) % n];
            // Check if this is the matching incoming edge.
            // curr.prev_id is the edge before curr in the chain — that's the
            // incoming edge at this vertex.
            if curr.chain_id == edge.chain_id
                && let Some(curr_prev) = curr.prev_id
                && edge.edge_id as usize == curr_prev
            {
                // Found matching incoming edge. Check chain sums.
                for &sum in chain_sums.values() {
                    if sum != 0 {
                        *error = S2Error::new(
                            S2ErrorCode::OverlappingGeometry,
                            format!(
                                "Shape {shape_id} has one or more chains that cross at a vertex",
                            ),
                        );
                        return false;
                    }
                }
                found = true;
                break;
            }
            *chain_sums.entry(edge.chain_id).or_insert(0) += edge.sign;
        }

        if !found {
            *error = S2Error::new(
                S2ErrorCode::InvalidVertex,
                "Outgoing edge with no incoming edge",
            );
            return false;
        }
    }

    true
}

// ─── Module registration ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format;

    fn expect_valid(geometry: &str) {
        let index = text_format::make_index(geometry);
        let result = S2ValidQuery::new().validate(&index);
        assert!(result.is_ok(), "Expected valid but got: {:?}", result.err());
    }

    fn expect_invalid(geometry: &str, expected_code: S2ErrorCode) {
        let index = text_format::make_index(geometry);
        let err = S2ValidQuery::new().validate(&index).expect_err(&format!(
            "Expected invalid geometry to fail validation: {geometry}"
        ));
        assert_eq!(
            err.code, expected_code,
            "Expected {:?} but got {:?}: {}",
            expected_code, err.code, err.message
        );
    }

    fn expect_legacy_valid(geometry: &str) {
        let index = text_format::make_index(geometry);
        let result = S2LegacyValidQuery::new().validate(&index);
        assert!(result.is_ok(), "Expected valid but got: {:?}", result.err());
    }

    fn expect_legacy_invalid(geometry: &str, expected_code: S2ErrorCode) {
        let index = text_format::make_index(geometry);
        let err = S2LegacyValidQuery::new()
            .validate(&index)
            .expect_err(&format!(
                "Expected invalid geometry to fail validation: {geometry}"
            ));
        assert_eq!(
            err.code, expected_code,
            "Expected {:?} but got {:?}: {}",
            expected_code, err.code, err.message
        );
    }

    /// Checks a geometry is valid under both `S2ValidQuery` and `S2LegacyValidQuery`.
    fn expect_both_valid(geometry: &str) {
        expect_valid(geometry);
        expect_legacy_valid(geometry);
    }

    /// Checks a geometry is invalid under both queries with the same error code.
    fn expect_both_invalid(geometry: &str, expected_code: S2ErrorCode) {
        expect_invalid(geometry, expected_code);
        expect_legacy_invalid(geometry, expected_code);
    }

    // ─── Both queries: BasicGeometryOk ──────────────────────────────

    #[test]
    fn test_basic_geometry_ok() {
        // Basic polygon.
        expect_both_valid("## 1:0, 0:-1, -1:0, 0:1");
        // Polyline.
        expect_both_valid("# 0:0, 1:0, 0:-1, -1:0, 0:1 #");
        // Multi-point.
        expect_both_valid("0:0 | 1:0 | 0:-1 | -1:0 | 0:1 ##");
        // Polygon with properly oriented hole (CW: E→S→W→N).
        expect_both_valid("## 2:0, 0:-2, -2:0, 0:2; 0:1, -1:0, 0:-1, 1:0;");
        // Polygon with improperly oriented hole (CCW, same as shell) should fail.
        expect_both_invalid(
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-1, -1:0, 0:1;",
            S2ErrorCode::PolygonInconsistentLoopOrientations,
        );
    }

    #[test]
    fn test_empty_geometry_ok() {
        expect_both_valid("##");
    }

    #[test]
    fn test_full_geometry_ok() {
        expect_both_valid("## full");
    }

    #[test]
    fn test_interior_on_right_regression() {
        // Regression: polygon that confused duplicate edge detection.
        expect_both_valid("## 0:4, 3:128, 4:2, 0:0");
    }

    #[test]
    fn test_tangent_polygons_ok() {
        // Two polygons touching at one vertex.
        expect_both_valid("## 1:0, 0:-1, -1:0, 0:1 | 0:1, -1:2, 0:3, 1:2");
    }

    #[test]
    fn test_antipodal_edge_fails() {
        let mut index = ShapeIndex::new();
        let v0 = Point::from_coords(1.0, 0.0, 0.0);
        let v1 = Point(-v0.0);
        index.add(Box::new(crate::s2::lax_polyline::LaxPolyline::new(vec![
            v0, v1,
        ])));
        index.build();

        let err = S2ValidQuery::new().validate(&index).unwrap_err();
        assert_eq!(err.code, S2ErrorCode::AntipodalVertices);
    }

    #[test]
    fn test_open_chain_fails() {
        let mut index = ShapeIndex::new();
        use crate::s2::LatLng;
        use crate::s2::shape::{Chain, ChainPosition, ReferencePoint};

        #[derive(Debug)]
        struct OpenShape {
            vertices: Vec<Point>,
        }
        impl Shape for OpenShape {
            fn num_edges(&self) -> usize {
                self.vertices.len() - 1
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
            fn chain(&self, _: usize) -> Chain {
                Chain::new(0, self.num_edges())
            }
            fn chain_edge(&self, _: usize, offset: usize) -> Edge {
                self.edge(offset)
            }
            fn chain_position(&self, edge_id: usize) -> ChainPosition {
                ChainPosition::new(0, edge_id)
            }
            fn dimension(&self) -> Dimension {
                Dimension::Polygon
            }
        }

        let p = |lat: f64, lng: f64| LatLng::from_degrees(lat, lng).to_point();
        index.add(Box::new(OpenShape {
            vertices: vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)],
        }));
        index.build();

        let err = S2ValidQuery::new().validate(&index).unwrap_err();
        assert_eq!(err.code, S2ErrorCode::LoopNotEnoughVertices);
    }

    #[test]
    fn test_duplicate_polygon_edges_fail() {
        // Two separate polygon shapes sharing an edge.
        expect_both_invalid(
            "## 2:0, 0:-2, -2:0, 0:2 | 2:0, 0:-2, 0:0",
            S2ErrorCode::OverlappingGeometry,
        );
    }

    // ─── Chain orientation ───────────────────────────────────────────

    #[test]
    fn test_chains_touching_ok() {
        // Polygon with hole touching at vertex 0:2 (hole CW: E→S→W→N).
        expect_both_valid("## 2:0, 0:-2, -2:0, 0:2; 0:2, -1:0, 0:-1, 1:0;");
        // Touching at vertex -2:0.
        expect_both_valid("## 2:0, 0:-2, -2:0, 0:2; 0:1, -2:0, 0:-1, 1:0;");
        // Improperly oriented hole touching shell (should fail).
        expect_both_invalid(
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-2, -1:0, 0:2;",
            S2ErrorCode::PolygonInconsistentLoopOrientations,
        );
    }

    #[test]
    fn test_nested_shells_fail() {
        // Various nested CCW shells sharing 0, 1, or 2 vertices with the outer shell.
        let cases = [
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-1, -1:0, 0:1",
            "## 2:0, 0:-2, -2:0, 0:2; 2:0, 0:-1, -1:0, 0:1",
            "## 2:0, 0:-2, -2:0, 0:2; 2:0, 0:-1, -2:0, 0:1",
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-2, -1:0, 0:1",
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-1, -2:0, 0:1",
            "## 2:0, 0:-2, -2:0, 0:2; 1:0, 0:-1, -1:0, 0:2",
        ];
        for case in cases {
            expect_both_invalid(case, S2ErrorCode::PolygonInconsistentLoopOrientations);
        }
    }

    #[test]
    fn test_chains_cannot_cross() {
        // Two chains that cross each other.
        expect_both_invalid(
            "## 3:0, 0:-3, -3:0, 0:+3; 3:2, 0:-1, -3:2, 0:+5",
            S2ErrorCode::PolygonInconsistentLoopOrientations,
        );
        // More crossing cases (detected as overlapping geometry).
        expect_both_invalid(
            "## 0:3, 3:0, 0:-3, -3:0; 3:2, 0:+5, -3:2, 0:-1",
            S2ErrorCode::OverlappingGeometry,
        );
    }

    #[test]
    fn test_shell_in_hole_fails() {
        // A shell contained in a hole.
        expect_both_invalid(
            "## 0:0, 10:10, 10:0; 5:21, 8:21, 6:23",
            S2ErrorCode::PolygonInconsistentLoopOrientations,
        );
    }

    // ─── Multi-dimensional tests (S2ValidQuery only) ────────────────

    #[test]
    fn test_multi_dimensional_ok() {
        // Multi-dimensional geometry: points + polyline + polygon.
        expect_valid(" 3:0| 0:-3| -3:0| 0:3# 2:0, 0:-2, -2:0, 0:2# 1:0, 0:-1, -1:0, 0:1");
    }

    #[test]
    fn test_contained_geometry_fails() {
        // Point inside a polygon.
        expect_invalid(
            "0:0 ## 2:0, 0:-2, -2:0, 0:2",
            S2ErrorCode::OverlappingGeometry,
        );
        // Polyline inside a polygon.
        expect_invalid(
            "# 0:-1, 0:1 # 2:0, 0:-2, -2:0, 0:2",
            S2ErrorCode::OverlappingGeometry,
        );
        // Polygon inside a polygon.
        expect_invalid(
            "## 2:0, 0:-2, -2:0, 0:2 | 1:0, 0:-1, -1:0, 0:1",
            S2ErrorCode::OverlappingGeometry,
        );
    }

    // ─── Legacy-only tests ──────────────────────────────────────────

    #[test]
    fn test_legacy_multi_dimensional_fails() {
        expect_legacy_invalid(
            " 3:0| 0:-3| -3:0| 0:3# 2:0, 0:-2, -2:0, 0:2# 1:0, 0:-1, -1:0, 0:1",
            S2ErrorCode::InvalidDimension,
        );
    }

    #[test]
    fn test_legacy_degenerate_edges_fail() {
        // Degenerate polygon edge.
        expect_legacy_invalid(
            "## 2:0, 2:0, 0:-2, -2:0, 0:-2",
            S2ErrorCode::DuplicateVertices,
        );
        // Degenerate polyline edge.
        expect_legacy_invalid("# 0:0, 0:0, 1:1, 2:2 #", S2ErrorCode::DuplicateVertices);
    }

    #[test]
    fn test_legacy_short_chains_fail() {
        expect_legacy_invalid("## 0:0", S2ErrorCode::LoopNotEnoughVertices);
        expect_legacy_invalid("## 0:0, 1:1", S2ErrorCode::LoopNotEnoughVertices);
    }

    #[test]
    fn test_legacy_split_interiors_ok() {
        expect_legacy_valid("## 3:0, 0:-3, -3:0, 0:+3; 3:0, 0:+1, -3:0, 0:-1");
    }

    #[test]
    fn test_legacy_self_touching_loop_fails() {
        expect_legacy_invalid(
            "## 2:0, 0:-2, -2:0, -1:1, 0:-2, 1:1",
            S2ErrorCode::DuplicateVertices,
        );
    }

    // ─── S2ValidQuery-specific tests ─────────────────────────────────

    #[test]
    fn test_valid_degenerate_rings_allowed() {
        expect_valid("## 0:0");
        expect_valid("## 0:0, 1:1");
    }

    #[test]
    fn test_valid_split_interiors_ok() {
        expect_valid("## 3:0, 0:-3, -3:0, 0:+3; 3:0, 0:+1, -3:0, 0:-1");
    }

    #[test]
    fn test_valid_polyline_crossings_ok() {
        // Interior crossings between polylines.
        expect_valid("# 0:-1, 0:1 | -1:0, 1:0 #");
        // More complex polyline crossings.
        expect_valid("# 0:0, 1:1, 0:2, 1:3, 0:4 | 1:0, 0:1, 1:2, 0:3, 1:4 #");
        // Interior crossings within a polyline.
        expect_valid("# 0:0, 1:1, 0:2, 1:3, 0:4, 1:4, 0:3, 1:2, 0:1, 1:0 #");
    }

    #[test]
    fn test_valid_reverse_duplicate_on_center() {
        // Reverse duplicate pair touching cell center.
        expect_valid("## 2:0, 0:-2, -2:0, 0:2; 0:0, 1:1");
    }

    // test_badly_dimensioned_fails removed: `Dimension` is now an enum,
    // so invalid dimensions (> 2) are prevented at compile time.

    // ─── sort_edges_ccw tests ────────────────────────────────────────

    /// Returns `num` evenly spaced edges all sharing a common origin.
    /// C++: `CcwEdgesAbout`
    fn ccw_edges_about(center: Point, num: usize) -> Vec<Edge> {
        use crate::s2::LatLng;
        let mut edges = Vec::with_capacity(num);
        for i in 0..num {
            let angle = 2.0 * std::f64::consts::PI / num as f64 * i as f64;
            let other = LatLng::from_radians(angle.sin(), angle.cos()).to_point();
            edges.push(Edge::new(center, other));
        }
        edges
    }

    #[test]
    fn test_sort_edges_ccw_start_edge_first() {
        use crate::s2::point;

        let origin = Point::from_coords(0.0, 0.0, 1.0);
        let m = point::get_frame(origin);

        let num_edges = 10usize;
        let mut edges: Vec<Edge> = Vec::new();
        for i in 0..num_edges {
            let angle = (i as f64) * 2.0 * std::f64::consts::PI / num_edges as f64;
            let p = Point(
                crate::r3::Vector::new(0.01 * angle.cos(), 0.01 * angle.sin(), 1.0).normalize(),
            );
            let other = point::from_frame(&m, p);
            edges.push(Edge::new(origin, other));
        }

        for i in 0..num_edges {
            let mut shuffled = edges.clone();
            shuffled.swap(0, 3);
            shuffled.swap(2, 7);
            sort_edges_ccw(origin, edges[i], &mut shuffled);
            assert_eq!(shuffled[0], edges[i]);
        }
    }

    #[test]
    fn test_sort_edges_ccw_sorts_edges() {
        // C++: SortEdgesCcw::SortsEdges
        use rand::SeedableRng;
        use rand::seq::SliceRandom;

        let origin = crate::s2::LatLng::from_radians(0.0, 0.0).to_point();
        let num_edges = 10;
        let mut sorted = ccw_edges_about(origin, num_edges);

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        for _ in 0..num_edges {
            sorted.rotate_left(1);
            let mut shuffled = sorted.clone();
            shuffled.shuffle(&mut rng);
            sort_edges_ccw(origin, sorted[0], &mut shuffled);
            assert_eq!(shuffled, sorted);
        }
    }

    #[test]
    fn test_sort_edges_ccw_sorts_edges_flipped() {
        // C++: SortEdgesCcw::SortsEdgesFlipped
        use rand::SeedableRng;
        use rand::seq::SliceRandom;

        let origin = crate::s2::LatLng::from_radians(0.0, 0.0).to_point();
        let num_edges = 10;
        let mut sorted = ccw_edges_about(origin, num_edges);

        // Flip the orientation of some edges.
        sorted[3] = sorted[3].reversed();
        sorted[8] = sorted[8].reversed();

        let mut rng = rand::rngs::StdRng::seed_from_u64(43);
        for _ in 0..num_edges {
            sorted.rotate_left(1);
            let mut shuffled = sorted.clone();
            shuffled.shuffle(&mut rng);
            sort_edges_ccw(origin, sorted[0], &mut shuffled);
            assert_eq!(shuffled, sorted);
        }
    }

    #[test]
    fn test_sort_edges_ccw_reverse_duplicates_ordered() {
        // C++: SortEdgesCcw::ReverseDuplicatesOrdered
        use rand::SeedableRng;
        use rand::seq::SliceRandom;

        let origin = crate::s2::LatLng::from_radians(0.0, 0.0).to_point();
        let num_edges = 10;
        let mut sorted = ccw_edges_about(origin, num_edges);

        // Insert reverse duplicates at positions 8 and 3 (after the original).
        // C++ inserts at begin+8 then begin+3, which shifts indices.
        let rev8 = sorted[8].reversed();
        let rev3 = sorted[3].reversed();
        sorted.insert(9, rev8); // after index 8
        sorted.insert(4, rev3); // after index 3 (was 3, now insert at 4 due to 0-indexed after)

        // Pick sorted[4] as the first edge (matching C++ which uses sorted[4]).
        // Actually C++ uses sorted[4] after both insertions.
        let first = sorted[5]; // adjust: C++ sorted[4] after insert at 3 then 8

        let mut rng = rand::rngs::StdRng::seed_from_u64(44);
        let mut shuffled = sorted.clone();
        shuffled.shuffle(&mut rng);

        sort_edges_ccw(origin, first, &mut shuffled);

        // After sorting, reverse duplicates should be adjacent, and the one
        // with v0 == origin should come first.
        // Find the reverse duplicate pairs.
        let mut found_pairs = 0;
        for i in 0..shuffled.len() - 1 {
            if shuffled[i] == shuffled[i + 1].reversed() {
                assert_eq!(
                    shuffled[i].v0, origin,
                    "reverse duplicate pair: first should have v0 == origin"
                );
                found_pairs += 1;
            }
        }
        assert_eq!(found_pairs, 2, "expected 2 reverse duplicate pairs");
    }

    // ─── Quilt tests ────────────────────────────────────────────────

    /// Builds a "quilt" test shape — a grid of diamond loops stretching from
    /// south to north pole, where every vertex has two chains incident on it.
    /// C++: `MakeQuilt`
    fn make_quilt() -> crate::s2::lax_polygon::LaxPolygon {
        use crate::s2::LatLng;

        let grid_point = |x: i32, y: i32| -> Point {
            debug_assert!((0..=12).contains(&y));
            let x = x.rem_euclid(24);
            if y == 0 {
                return Point::from_coords(0.0, 0.0, -1.0);
            }
            if y == 12 {
                return Point::from_coords(0.0, 0.0, 1.0);
            }
            let lat = -90.0 + 15.0 * f64::from(y);
            let lng = -180.0 + 15.0 * f64::from(x);
            LatLng::from_degrees(lat, lng).to_point()
        };

        let mut loops = Vec::new();
        for x in (0..24).step_by(2) {
            for y in (0..12).step_by(2) {
                let lp = vec![
                    grid_point(x, y + 1),
                    grid_point(x + 1, y + 2),
                    grid_point(x + 2, y + 1),
                    grid_point(x + 1, y),
                ];
                loops.push(lp);
            }
        }
        crate::s2::lax_polygon::LaxPolygon::from_loops_owned(loops)
    }

    #[test]
    fn test_quilt_is_valid() {
        // C++: S2ValidTest::QuiltIsValid
        let quilt = make_quilt();
        let mut index = ShapeIndex::new();
        index.add(Box::new(quilt));
        index.build();

        assert!(
            S2ValidQuery::new().validate(&index).is_ok(),
            "Expected quilt to be valid"
        );
    }

    #[test]
    fn test_quilt_is_not_valid_legacy() {
        // C++: S2LegacyValidTest::QuiltIsNotValid
        // The quilt has reverse duplicate edges near the poles.
        let quilt = make_quilt();
        let mut index = ShapeIndex::new();
        index.add(Box::new(quilt));
        index.build();

        let err = S2LegacyValidQuery::new()
            .validate(&index)
            .expect_err("Expected quilt to be invalid under legacy validation");
        assert_eq!(err.code, S2ErrorCode::OverlappingGeometry);
    }

    // ─── Polygon on cell centers ─────────────────────────────────────

    /// Helper to get cell center from a token string.
    fn cell_center(token: &str) -> Point {
        use crate::s2::cell::Cell;
        use crate::s2::cell_id::CellId;
        Cell::from(CellId::from_token(token)).center()
    }

    #[test]
    fn test_polygon_on_centers_works() {
        // C++: S2ValidTest::PolygonOnCentersWorks
        // Diamond polygon using cell centers straddling equator/prime meridian.
        let loops = vec![
            vec![
                cell_center("0ec"),
                cell_center("044"),
                cell_center("1bc"),
                cell_center("114"),
            ],
            vec![
                cell_center("104"),
                cell_center("1ac"),
                cell_center("054"),
                cell_center("0fc"),
            ],
        ];
        let poly = crate::s2::lax_polygon::LaxPolygon::from_loops_owned(loops);
        let mut index = ShapeIndex::new();
        index.add(Box::new(poly));
        index.build();

        assert!(
            S2ValidQuery::new().validate(&index).is_ok(),
            "Expected polygon on centers to be valid"
        );
    }

    #[test]
    fn test_degenerate_polygon_on_centers_works() {
        // C++: S2ValidTest::DegeneratePolygonOnCentersworks
        // Polygon with reverse-duplicate pairs between cell centers.
        let loops = vec![vec![
            cell_center("0ec"),
            cell_center("044"),
            cell_center("1bc"),
            cell_center("114"),
            cell_center("1bc"),
            cell_center("044"),
        ]];
        let poly = crate::s2::lax_polygon::LaxPolygon::from_loops_owned(loops);
        let mut index = ShapeIndex::new();
        index.add(Box::new(poly));
        index.build();

        assert!(
            S2ValidQuery::new().validate(&index).is_ok(),
            "Expected degenerate polygon on centers to be valid"
        );

        // Second case: diagonal out and back.
        let tokens = ["1004", "1014", "1044", "1054", "1104", "1114"];
        let mut loop_pts: Vec<Point> = tokens.iter().map(|t| cell_center(t)).collect();
        for i in (1..5).rev() {
            loop_pts.push(cell_center(tokens[i]));
        }
        let poly2 = crate::s2::lax_polygon::LaxPolygon::from_loops_owned(vec![loop_pts]);
        let mut index2 = ShapeIndex::new();
        index2.add(Box::new(poly2));
        index2.build();

        assert!(
            S2ValidQuery::new().validate(&index2).is_ok(),
            "Expected diagonal degenerate polygon to be valid"
        );
    }

    // ─── LoopsCrossing ───────────────────────────────────────────────

    #[test]
    fn test_loops_crossing() {
        // C++: AllValidationQueries::LoopsCrossing
        // Generate concentric loops and swap vertices to create crossings.
        use crate::s1;
        use crate::s2::testing::{make_regular_points, random_point};
        use rand::Rng;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(0xDEAD_BEEF);
        let num_iters = 100;

        for _ in 0..num_iters {
            let center = random_point(&mut rng);
            let num_vertices = 4 + rng.gen_range(0..10);

            // Create two concentric loops with decreasing radii.
            let loop0 = make_regular_points(center, s1::Angle::from_degrees(80.0), num_vertices);
            let loop1 = make_regular_points(center, s1::Angle::from_degrees(8.0), num_vertices);

            let mut loop0 = loop0;
            let mut loop1 = loop1;

            // Swap one vertex to create a crossing.
            let i = rng.gen_range(0..num_vertices);
            std::mem::swap(&mut loop0[i], &mut loop1[i]);

            // Optionally also copy adjacent vertices for vertex-crossing.
            if rng.r#gen::<bool>() {
                let n = num_vertices;
                loop0[(i + 1) % n] = loop1[(i + 1) % n];
                loop0[(i + n - 1) % n] = loop1[(i + n - 1) % n];
            }

            // Build S2Polygon from these loops (disabling normal validation).
            let polygon = crate::s2::Polygon::from_loops(vec![
                crate::s2::Loop::new(loop0),
                crate::s2::Loop::new(loop1),
            ]);

            let mut index = ShapeIndex::new();
            index.add(Box::new(polygon));
            index.build();

            // Should be invalid under both queries.
            assert!(
                S2ValidQuery::new().validate(&index).is_err(),
                "iter: crossing polygon should be invalid under S2ValidQuery"
            );

            assert!(
                S2LegacyValidQuery::new().validate(&index).is_err(),
                "iter: crossing polygon should be invalid under S2LegacyValidQuery"
            );
        }
    }

    // ─── Decoder-dependent tests (manual equivalents) ────────────────

    #[test]
    fn test_outgoing_edge_no_incoming() {
        // C++: AllValidationQueries::OutgoingEdgeButNoIncomingEdge
        // Manually construct a polygon shape where one vertex has an outgoing
        // edge but no matching incoming edge (open chain pretending to be dim=2).
        use crate::s2::shape::{Chain, ChainPosition, ReferencePoint};

        #[derive(Debug)]
        struct BrokenPolygonShape {
            edges: Vec<(Point, Point)>,
        }

        impl Shape for BrokenPolygonShape {
            fn num_edges(&self) -> usize {
                self.edges.len()
            }
            fn edge(&self, id: usize) -> Edge {
                Edge::new(self.edges[id].0, self.edges[id].1)
            }
            fn reference_point(&self) -> ReferencePoint {
                ReferencePoint::new(self.edges[0].0, false)
            }
            fn num_chains(&self) -> usize {
                1
            }
            fn chain(&self, _: usize) -> Chain {
                Chain::new(0, self.edges.len())
            }
            fn chain_edge(&self, _: usize, offset: usize) -> Edge {
                self.edge(offset)
            }
            fn chain_position(&self, edge_id: usize) -> ChainPosition {
                ChainPosition::new(0, edge_id)
            }
            fn dimension(&self) -> Dimension {
                Dimension::Polygon
            }
        }

        // Create a "polygon" chain A→B→C→D that doesn't close (D != A).
        use crate::s2::LatLng;
        let a = LatLng::from_degrees(0.0, 0.0).to_point();
        let b = LatLng::from_degrees(1.0, 0.0).to_point();
        let c = LatLng::from_degrees(1.0, 1.0).to_point();
        let d = LatLng::from_degrees(0.5, 0.5).to_point(); // != A

        let shape = BrokenPolygonShape {
            edges: vec![(a, b), (b, c), (c, d)],
        };

        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        // Should fail because the chain isn't closed.
        let err = S2ValidQuery::new().validate(&index).unwrap_err();
        assert_eq!(err.code, S2ErrorCode::LoopNotEnoughVertices);
    }

    #[test]
    fn test_invalid_chain_near_chain() {
        // C++: AllValidationQueries::InvalidChainNearChain
        // A shape with one valid chain and one chain with non-unit-length vertices.
        use crate::s2::shape::{Chain, ChainPosition, ReferencePoint};

        #[derive(Debug)]
        struct TwoChainsOneInvalid {
            /// Chain 0: valid polygon chain (3 edges, closed).
            /// Chain 1: chain with non-unit-length vertices (3 edges, closed).
            vertices_0: Vec<Point>,
            vertices_1: Vec<Point>,
        }

        impl Shape for TwoChainsOneInvalid {
            fn num_edges(&self) -> usize {
                self.vertices_0.len() + self.vertices_1.len()
            }
            fn edge(&self, id: usize) -> Edge {
                let n0 = self.vertices_0.len();
                if id < n0 {
                    let v0 = self.vertices_0[id];
                    let v1 = self.vertices_0[(id + 1) % n0];
                    Edge::new(v0, v1)
                } else {
                    let i = id - n0;
                    let n1 = self.vertices_1.len();
                    let v0 = self.vertices_1[i];
                    let v1 = self.vertices_1[(i + 1) % n1];
                    Edge::new(v0, v1)
                }
            }
            fn reference_point(&self) -> ReferencePoint {
                ReferencePoint::new(self.vertices_0[0], false)
            }
            fn num_chains(&self) -> usize {
                2
            }
            fn chain(&self, chain_id: usize) -> Chain {
                if chain_id == 0 {
                    Chain::new(0, self.vertices_0.len())
                } else {
                    Chain::new(self.vertices_0.len(), self.vertices_1.len())
                }
            }
            fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge {
                self.edge(if chain_id == 0 {
                    offset
                } else {
                    self.vertices_0.len() + offset
                })
            }
            fn chain_position(&self, edge_id: usize) -> ChainPosition {
                let n0 = self.vertices_0.len();
                if edge_id < n0 {
                    ChainPosition::new(0, edge_id)
                } else {
                    ChainPosition::new(1, edge_id - n0)
                }
            }
            fn dimension(&self) -> Dimension {
                Dimension::Polygon
            }
        }

        use crate::s2::LatLng;
        // Chain 0: valid triangle.
        let v0 = vec![
            LatLng::from_degrees(10.0, 0.0).to_point(),
            LatLng::from_degrees(10.0, 10.0).to_point(),
            LatLng::from_degrees(20.0, 5.0).to_point(),
        ];
        // Chain 1: non-unit-length vertices (scaled by 2.0).
        let v1 = vec![
            Point(crate::r3::Vector::new(2.0, 0.0, 0.0)),
            Point(crate::r3::Vector::new(0.0, 2.0, 0.0)),
            Point(crate::r3::Vector::new(0.0, 0.0, 2.0)),
        ];

        let shape = TwoChainsOneInvalid {
            vertices_0: v0,
            vertices_1: v1,
        };

        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let err = S2ValidQuery::new().validate(&index).unwrap_err();
        assert_eq!(err.code, S2ErrorCode::NotUnitLength);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_touch_type_roundtrip() {
        for v in [
            TouchType::None,
            TouchType::Interior,
            TouchType::Boundary,
            TouchType::Any,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<TouchType>(&j).unwrap());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_validation_options_roundtrip() {
        let opts = ValidationOptions::default();
        let json = serde_json::to_string(&opts).unwrap();
        let back: ValidationOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(opts.allow_degenerate_edges, back.allow_degenerate_edges);
        assert_eq!(opts.legacy_mode, back.legacy_mode);
    }
}
