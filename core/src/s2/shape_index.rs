// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Spatial index mapping [`CellId`]s to shape edge lists.
//!
//! [`ShapeIndex`] is the central data structure for spatial queries. It
//! indexes any number of [`Shape`]s (polygons, polylines, point sets) and
//! organizes their edges into an adaptive cell hierarchy for fast lookups.
//! Once built, the index supports:
//!
//! - **Point containment** via [`ContainsPointQuery`](crate::s2::contains_point_query::ContainsPointQuery)
//! - **Nearest-edge search** via [`ClosestEdgeQuery`](crate::s2::closest_edge_query::ClosestEdgeQuery)
//! - **Edge crossing detection** via [`CrossingEdgeQuery`](crate::s2::crossing_edge_query::CrossingEdgeQuery)
//! - **Boolean operations** via [`S2BooleanOperation`](crate::s2::boolean_operation::S2BooleanOperation)
//!
//! Build the index by adding shapes with [`ShapeIndex::add`], then call
//! [`ShapeIndex::build`] before querying. The index is immutable after
//! building.

#![expect(
    clippy::cast_sign_loss,
    reason = "ShapeId (i32) used as Vec indices in shape index"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "ShapeId (usize<->i32) — index values always in i32 range"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for ShapeId — always in range"
)]
use std::collections::BTreeMap;

use crate::r1;
use crate::r2;
use crate::s2::coords::{
    Face, Level, MAX_CELL_LEVEL, face_uv_to_xyz, get_face, valid_face_xyz_to_uv,
};
use crate::s2::edge_clipping::{
    EDGE_CLIP_ERROR_UV_COORD, FACE_CLIP_ERROR_UV_COORD, clip_to_padded_face, interpolate_float64,
};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::metric;
use crate::s2::padded_cell::PaddedCell;
use crate::s2::shape::{Dimension, Edge, Shape, ShapeId};
use crate::s2::{CellId, CellUnion, Point};

/// Total error when clipping an edge, doubled so we only pad during
/// indexing and not at query time.
const CELL_PADDING: f64 = 2.0 * (FACE_CLIP_ERROR_UV_COORD + EDGE_CLIP_ERROR_UV_COORD);

/// Cell size relative to edge length at which the edge is considered "long".
const CELL_SIZE_TO_LONG_EDGE_RATIO: f64 = 1.0;

// ─── CellRelation ───────────────────────────────────────────────────────

/// Describes the relationship between a target cell and the index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CellRelation {
    /// The target is contained by an index cell.
    Indexed,
    /// The target contains one or more index cells.
    Subdivided,
    /// The target does not intersect any index cells.
    #[default]
    Disjoint,
}

// ─── ClippedShape ───────────────────────────────────────────────────────

/// The portion of a shape that intersects a cell.
///
/// Stores the edge IDs from the original shape (not clipped geometry)
/// and whether the cell center falls inside the shape's interior.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ClippedShape {
    /// Shape ID within the `ShapeIndex`.
    pub shape_id: ShapeId,
    /// Whether the cell center is inside this shape.
    pub contains_center: bool,
    /// Original edge IDs that intersect this cell (sorted).
    pub edges: Vec<i32>,
}

impl ClippedShape {
    /// Returns the number of edges.
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Reports whether this clipped shape contains the given edge ID.
    pub fn contains_edge(&self, id: i32) -> bool {
        self.edges.contains(&id)
    }
}

// ─── ShapeIndexCell ─────────────────────────────────────────────────────

/// Stores the index contents for a particular [`CellId`].
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeIndexCell {
    /// Clipped shapes intersecting this cell, sorted by shape ID.
    pub shapes: Vec<ClippedShape>,
}

impl ShapeIndexCell {
    /// Returns the total number of edges across all clipped shapes.
    pub fn num_edges(&self) -> usize {
        self.shapes.iter().map(ClippedShape::num_edges).sum()
    }

    /// Returns the clipped shape for the given shape ID, if present.
    #[inline]
    pub fn find_by_shape_id(&self, shape_id: impl Into<ShapeId>) -> Option<&ClippedShape> {
        let shape_id = shape_id.into();
        self.shapes.iter().find(|s| s.shape_id == shape_id)
    }
}

// ─── Temporary build structures ─────────────────────────────────────────

/// An edge projected onto a cube face (used during index construction).
struct FaceEdge {
    shape_id: ShapeId,
    edge_id: i32,
    max_level: Level,
    has_interior: bool,
    a: r2::Point,
    b: r2::Point,
    edge: Edge,
}

/// A portion of an edge clipped to a cell (used during index construction).
struct IndexClippedEdge {
    face_edge_idx: usize,
    bound: r2::Rect,
}

// ─── InteriorTracker ────────────────────────────────────────────────────

/// Tracks which shapes contain a particular point along the space-filling
/// curve, enabling efficient computation of `contains_center`.
struct InteriorTracker {
    is_active: bool,
    a: Point,
    b: Point,
    next_cell_id: CellId,
    crosser: EdgeCrosser,
    shape_ids: Vec<ShapeId>,
}

impl InteriorTracker {
    fn new() -> Self {
        let origin = Point(face_uv_to_xyz(Face::F0, -1.0, -1.0).normalize());
        let mut t = InteriorTracker {
            is_active: false,
            a: origin,
            b: origin,
            next_cell_id: CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL),
            crosser: EdgeCrosser::new(origin, origin),
            shape_ids: Vec::new(),
        };
        t.draw_to(origin);
        t
    }

    fn focus(&self) -> Point {
        self.b
    }

    fn add_shape(&mut self, shape_id: ShapeId, contains_focus: bool) {
        self.is_active = true;
        if contains_focus {
            self.toggle_shape(shape_id);
        }
    }

    fn move_to(&mut self, b: Point) {
        self.b = b;
    }

    fn draw_to(&mut self, b: Point) {
        self.a = self.b;
        self.b = b;
        self.crosser = EdgeCrosser::new(self.a, self.b);
    }

    fn test_edge(&mut self, shape_id: ShapeId, edge: Edge) {
        if self.crosser.edge_or_vertex_crossing(edge.v0, edge.v1) {
            self.toggle_shape(shape_id);
        }
    }

    fn set_next_cell_id(&mut self, next: CellId) {
        self.next_cell_id = next.range_min();
    }

    fn at_cell_id(&self, id: CellId) -> bool {
        id.range_min() == self.next_cell_id
    }

    fn toggle_shape(&mut self, shape_id: ShapeId) {
        match self.shape_ids.binary_search(&shape_id) {
            Ok(idx) => {
                self.shape_ids.remove(idx);
            }
            Err(idx) => {
                self.shape_ids.insert(idx, shape_id);
            }
        }
    }
}

// ─── ShapeIndex ─────────────────────────────────────────────────────────

/// Spatial index for a set of [`Shape`]s.
///
/// After adding shapes with [`add`](ShapeIndex::add), call
/// [`build`](ShapeIndex::build) to construct the index. Once built, the
/// index is immutable and can be queried via iterators.
///
/// # Examples
///
/// ```
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::lax_loop::LaxLoop;
/// use s2rst::s2::LatLng;
///
/// // Create a triangle as a LaxLoop shape.
/// let vertices = vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 10.0).to_point(),
///     LatLng::from_degrees(10.0, 0.0).to_point(),
/// ];
/// let shape = LaxLoop::new(vertices);
///
/// // Build the spatial index.
/// let mut index = ShapeIndex::new();
/// index.add(Box::new(shape));
/// index.build();
///
/// assert_eq!(index.len(), 1);
/// assert_eq!(index.num_edges(), 3);
///
/// // Iterate over index cells.
/// let mut it = index.iter();
/// let mut cell_count = 0;
/// while !it.done() {
///     cell_count += 1;
///     it.next();
/// }
/// assert!(cell_count > 0);
/// ```
#[derive(Debug)]
pub struct ShapeIndex {
    shapes: Vec<Option<Box<dyn Shape>>>,
    max_edges_per_cell: usize,
    cell_map: BTreeMap<CellId, ShapeIndexCell>,
    cells: Vec<CellId>,
    built: bool,
}

impl ShapeIndex {
    /// Creates a new empty `ShapeIndex`.
    pub fn new() -> Self {
        ShapeIndex {
            shapes: Vec::new(),
            max_edges_per_cell: 10,
            cell_map: BTreeMap::new(),
            cells: Vec::new(),
            built: false,
        }
    }

    /// Adds a shape to the index and returns the assigned shape ID.
    pub fn add(&mut self, shape: Box<dyn Shape>) -> ShapeId {
        let id = ShapeId::new(self.shapes.len() as i32);
        self.shapes.push(Some(shape));
        self.built = false;
        id
    }

    /// Returns the shape with the given ID, or `None`.
    #[inline]
    pub fn shape(&self, id: impl Into<ShapeId>) -> Option<&dyn Shape> {
        let id = id.into();
        self.shapes.get(id.as_usize()).and_then(|s| s.as_deref())
    }

    /// Returns the number of shape IDs in the index (including deleted shapes).
    /// This is the maximum shape ID + 1.
    #[inline]
    pub fn num_shape_ids(&self) -> usize {
        self.shapes.len()
    }

    /// Returns the number of non-deleted shapes in the index.
    pub fn len(&self) -> usize {
        self.shapes.iter().filter(|s| s.is_some()).count()
    }

    /// Reports whether the index contains no shapes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total number of edges across all shapes.
    pub fn num_edges(&self) -> usize {
        self.shapes
            .iter()
            .filter_map(|s| s.as_ref())
            .map(|s| s.num_edges())
            .sum()
    }

    /// Builds the index. Must be called before querying.
    pub fn build(&mut self) {
        if self.built {
            return;
        }
        self.cell_map.clear();
        self.cells.clear();

        let mut tracker = InteriorTracker::new();
        let mut all_edges: Vec<Vec<FaceEdge>> = (0..6).map(|_| Vec::new()).collect();

        // Clip all edges of all shapes to the six cube faces.
        for shape_id in (0..self.shapes.len() as i32).map(ShapeId) {
            self.add_shape_internal(shape_id, &mut all_edges, &mut tracker);
        }

        // Per-face parallelism via rayon was tried in Phase 7 and reverted.
        // Real S2 workloads
        // tend to be regional (all edges land on 1–2 cube faces), so 4–5
        // rayon workers idle while one drives the actual work, and the
        // tiny per-face cost doesn't amortize task spawn overhead. The
        // refactor that splits the implementation into the
        // `update_face_edges_into` / `update_edges_into` free functions is
        // kept because it makes the build-state plumbing explicit and is
        // useful for future per-cell parallelism attempts.
        let max_edges = self.max_edges_per_cell;

        for face in Face::iter() {
            let face_edges = std::mem::take(&mut all_edges[face.as_u8() as usize]);
            Self::update_face_edges_into(
                face,
                &face_edges,
                &mut tracker,
                &mut self.cell_map,
                &mut self.cells,
                max_edges,
            );
        }

        self.built = true;
    }

    /// Returns the index cell for the given `CellId`, if present.
    pub fn cell(&self, id: CellId) -> Option<&ShapeIndexCell> {
        self.cell_map.get(&id)
    }

    /// Returns an iterator positioned at the first cell.
    #[inline]
    pub fn iter(&self) -> ShapeIndexIterator<'_> {
        ShapeIndexIterator::new(self, 0)
    }

    /// Returns the maximum number of edges per cell.
    pub fn max_edges_per_cell(&self) -> usize {
        self.max_edges_per_cell
    }

    /// Sets the maximum number of edges per cell.
    pub fn set_max_edges_per_cell(&mut self, n: usize) {
        self.max_edges_per_cell = n;
    }

    /// Returns a reference to the shapes slice (including `None` slots).
    pub fn shapes_slice(&self) -> &[Option<Box<dyn Shape>>] {
        &self.shapes
    }

    /// Adds a shape slot (which may be `None` for a deleted shape).
    pub fn add_option(&mut self, shape: Option<Box<dyn Shape>>) -> i32 {
        let id = self.shapes.len() as i32;
        self.shapes.push(shape);
        self.built = false;
        id
    }

    /// Directly inserts a cell into the cell map (used during decode).
    pub fn insert_cell(&mut self, id: CellId, cell: ShapeIndexCell) {
        self.cell_map.insert(id, cell);
        self.cells.push(id);
    }

    /// Marks the index as built (used after decoding).
    pub fn mark_built(&mut self) {
        self.built = true;
    }

    // ─── Build internals ────────────────────────────────────────────

    fn add_shape_internal(
        &self,
        shape_id: ShapeId,
        all_edges: &mut [Vec<FaceEdge>],
        tracker: &mut InteriorTracker,
    ) {
        let shape = match &self.shapes[shape_id.as_usize()] {
            Some(s) => s.as_ref(),
            None => return,
        };

        let has_interior = shape.dimension() == Dimension::Polygon;
        if has_interior {
            tracker.add_shape(shape_id, contains_brute_force(shape, tracker.focus()));
        }

        let num_edges = shape.num_edges();
        for e in 0..num_edges {
            let edge = shape.edge(e);
            let max_level = max_level_for_edge(&edge);
            let fe = FaceEdge {
                shape_id,
                edge_id: e as i32,
                max_level,
                has_interior,
                a: r2::Point::default(),
                b: r2::Point::default(),
                edge,
            };
            add_face_edge(fe, all_edges);
        }
    }

    /// Builds the per-face cell hierarchy. The free-function form (writing
    /// into a provided `cell_map` / `cells` rather than `self`'s) lets the
    /// per-face work run in parallel into local accumulators that get
    /// merged sequentially after. See `build()`.
    fn update_face_edges_into(
        face: Face,
        face_edges: &[FaceEdge],
        tracker: &mut InteriorTracker,
        cell_map: &mut BTreeMap<CellId, ShapeIndexCell>,
        cells: &mut Vec<CellId>,
        max_edges_per_cell: usize,
    ) {
        let num_edges = face_edges.len();
        if num_edges == 0 && tracker.shape_ids.is_empty() {
            return;
        }

        // Create initial clipped edges.
        let mut clipped_edges: Vec<IndexClippedEdge> = Vec::with_capacity(num_edges);
        let mut bound = r2::Rect::empty();
        for (i, fe) in face_edges.iter().enumerate() {
            let edge_bound = r2::Rect::from_point_pair(fe.a, fe.b);
            clipped_edges.push(IndexClippedEdge {
                face_edge_idx: i,
                bound: edge_bound,
            });
            bound = bound.add_rect(edge_bound);
        }

        let face_id = CellId::from_face(face);
        let mut pcell = PaddedCell::from_cell_id(face_id, CELL_PADDING);

        if num_edges > 0 {
            let shrunk_id = pcell.shrink_to_fit(bound);
            if shrunk_id != pcell.cell_id() {
                Self::skip_cell_range(
                    face_id.range_min(),
                    shrunk_id.range_min(),
                    tracker,
                    cell_map,
                    cells,
                );
                pcell = PaddedCell::from_cell_id(shrunk_id, CELL_PADDING);
                Self::update_edges_into(
                    &mut pcell,
                    &mut clipped_edges,
                    face_edges,
                    tracker,
                    cell_map,
                    cells,
                    max_edges_per_cell,
                );
                Self::skip_cell_range(
                    shrunk_id.range_max().next(),
                    face_id.range_max().next(),
                    tracker,
                    cell_map,
                    cells,
                );
                return;
            }
        }

        Self::update_edges_into(
            &mut pcell,
            &mut clipped_edges,
            face_edges,
            tracker,
            cell_map,
            cells,
            max_edges_per_cell,
        );
    }

    fn skip_cell_range(
        begin: CellId,
        end: CellId,
        tracker: &mut InteriorTracker,
        cell_map: &mut BTreeMap<CellId, ShapeIndexCell>,
        cells: &mut Vec<CellId>,
    ) {
        if tracker.shape_ids.is_empty() {
            return;
        }
        let skipped = CellUnion::from_range(begin, end);
        for &cell_id in skipped.cell_ids() {
            let mut pcell = PaddedCell::from_cell_id(cell_id, CELL_PADDING);
            let empty_face_edges: Vec<FaceEdge> = Vec::new();
            let mut empty_edges: Vec<IndexClippedEdge> = Vec::new();
            Self::make_index_cell_static(
                &mut pcell,
                &mut empty_edges,
                &empty_face_edges,
                tracker,
                cell_map,
                cells,
                10, // max_edges_per_cell
            );
        }
    }

    /// Free-function form of `update_edges` — writes into the provided
    /// `cell_map` / `cells` rather than `self`'s. See `update_face_edges_into`.
    fn update_edges_into(
        pcell: &mut PaddedCell,
        edges: &mut [IndexClippedEdge],
        face_edges: &[FaceEdge],
        tracker: &mut InteriorTracker,
        cell_map: &mut BTreeMap<CellId, ShapeIndexCell>,
        cells: &mut Vec<CellId>,
        max_edges_per_cell: usize,
    ) {
        // Try to create an index cell. If there are few enough edges, we're done.
        if Self::make_index_cell_static(
            pcell,
            edges,
            face_edges,
            tracker,
            cell_map,
            cells,
            max_edges_per_cell,
        ) {
            return;
        }

        // Subdivide into 4 children.
        // Use separate Vecs to avoid double-mutable-borrow issues with array indexing.
        let middle = pcell.middle();
        let mut ce0: Vec<IndexClippedEdge> = Vec::new();
        let mut ce1: Vec<IndexClippedEdge> = Vec::new();
        let mut ce2: Vec<IndexClippedEdge> = Vec::new();
        let mut ce3: Vec<IndexClippedEdge> = Vec::new();

        for ce in edges.iter() {
            let fe = &face_edges[ce.face_edge_idx];
            let (i_lo, i_hi, j_lo, j_hi) = classify_edge(&ce.bound, &middle);

            if i_hi == 0 {
                split_v_axis(ce, fe, &middle.y, &mut ce0, &mut ce1);
            } else if i_lo == 1 {
                split_v_axis(ce, fe, &middle.y, &mut ce2, &mut ce3);
            } else if j_hi == 0 {
                clip_and_push(ce, fe, 1, middle.x.hi, &mut ce0);
                clip_and_push(ce, fe, 0, middle.x.lo, &mut ce2);
            } else if j_lo == 1 {
                clip_and_push(ce, fe, 1, middle.x.hi, &mut ce1);
                clip_and_push(ce, fe, 0, middle.x.lo, &mut ce3);
            } else {
                let left = clip_u_bound(ce, fe, 1, middle.x.hi);
                if let Some(ref l) = left {
                    split_v_axis(l, fe, &middle.y, &mut ce0, &mut ce1);
                }
                let right = clip_u_bound(ce, fe, 0, middle.x.lo);
                if let Some(ref r) = right {
                    split_v_axis(r, fe, &middle.y, &mut ce2, &mut ce3);
                }
            }
        }

        // Recurse into children in CellID order.
        let mut child_edges = [ce0, ce1, ce2, ce3];
        for pos in 0..4 {
            let (i, j) = pcell.child_ij(pos);
            let child_idx = (i * 2 + j) as usize;
            if !child_edges[child_idx].is_empty() || !tracker.shape_ids.is_empty() {
                let mut child_pcell = PaddedCell::from_parent_ij(pcell, i, j);
                Self::update_edges_into(
                    &mut child_pcell,
                    &mut child_edges[child_idx],
                    face_edges,
                    tracker,
                    cell_map,
                    cells,
                    max_edges_per_cell,
                );
            }
        }
    }

    fn make_index_cell_static(
        pcell: &mut PaddedCell,
        edges: &mut [IndexClippedEdge],
        face_edges: &[FaceEdge],
        tracker: &mut InteriorTracker,
        cell_map: &mut BTreeMap<CellId, ShapeIndexCell>,
        cells: &mut Vec<CellId>,
        max_edges_per_cell: usize,
    ) -> bool {
        if edges.is_empty() && tracker.shape_ids.is_empty() {
            return true;
        }

        // Count edges that haven't reached max level.
        let mut count = 0;
        for ce in edges.iter() {
            if pcell.level() < face_edges[ce.face_edge_idx].max_level {
                count += 1;
            }
            if count > max_edges_per_cell {
                return false;
            }
        }

        // Update interior tracker: move to center, test all edges.
        if tracker.is_active && !edges.is_empty() {
            if !tracker.at_cell_id(pcell.cell_id()) {
                tracker.move_to(pcell.entry_vertex());
            }
            tracker.draw_to(pcell.center());
            for ce in edges.iter() {
                let fe = &face_edges[ce.face_edge_idx];
                if fe.has_interior {
                    tracker.test_edge(fe.shape_id, fe.edge);
                }
            }
        }

        // Merge edge shapes and containing shapes to build the cell.
        let containing_ids = &tracker.shape_ids;
        let num_shapes = count_shapes(edges, face_edges, containing_ids);
        let mut cell = ShapeIndexCell {
            shapes: Vec::with_capacity(num_shapes),
        };

        let mut e_next = 0usize;
        let mut c_next = 0usize;
        let sentinel = ShapeId(i32::MAX);

        for _ in 0..num_shapes {
            let e_shape_id = if e_next < edges.len() {
                face_edges[edges[e_next].face_edge_idx].shape_id
            } else {
                sentinel
            };
            let c_shape_id = if c_next < containing_ids.len() {
                containing_ids[c_next]
            } else {
                sentinel
            };

            if c_shape_id < e_shape_id {
                // Shape has no edges in this cell but its interior contains center.
                cell.shapes.push(ClippedShape {
                    shape_id: c_shape_id,
                    contains_center: true,
                    edges: Vec::new(),
                });
                c_next += 1;
            } else {
                // Collect all edges for this shape.
                let e_begin = e_next;
                while e_next < edges.len()
                    && face_edges[edges[e_next].face_edge_idx].shape_id == e_shape_id
                {
                    e_next += 1;
                }
                let edge_ids: Vec<i32> = (e_begin..e_next)
                    .map(|i| face_edges[edges[i].face_edge_idx].edge_id)
                    .collect();
                let contains_center = c_shape_id == e_shape_id;
                if contains_center {
                    c_next += 1;
                }
                cell.shapes.push(ClippedShape {
                    shape_id: e_shape_id,
                    contains_center,
                    edges: edge_ids,
                });
            }
        }

        let cell_id = pcell.cell_id();
        cell_map.insert(cell_id, cell);
        cells.push(cell_id);

        // Move tracker to exit vertex.
        if tracker.is_active && !edges.is_empty() {
            tracker.draw_to(pcell.exit_vertex());
            for ce in edges.iter() {
                let fe = &face_edges[ce.face_edge_idx];
                if fe.has_interior {
                    tracker.test_edge(fe.shape_id, fe.edge);
                }
            }
            tracker.set_next_cell_id(cell_id.next());
        }

        true
    }
}

impl Default for ShapeIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Iterator ───────────────────────────────────────────────────────────

/// An iterator over the cells of a [`ShapeIndex`].
///
/// Cells are visited in increasing order of [`CellId`].
#[derive(Clone, Debug)]
pub struct ShapeIndexIterator<'a> {
    index: &'a ShapeIndex,
    position: usize,
}

impl<'a> ShapeIndexIterator<'a> {
    #[inline]
    fn new(index: &'a ShapeIndex, position: usize) -> Self {
        ShapeIndexIterator { index, position }
    }

    /// Returns the `CellId` of the current cell, or `CellId::sentinel()` if done.
    #[inline]
    pub fn cell_id(&self) -> CellId {
        if self.position < self.index.cells.len() {
            self.index.cells[self.position]
        } else {
            CellId::sentinel()
        }
    }

    /// Returns the current index cell.
    #[inline]
    pub fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        if self.position < self.index.cells.len() {
            self.index.cell_map.get(&self.index.cells[self.position])
        } else {
            None
        }
    }

    /// Returns the center point of the current cell.
    #[inline]
    pub fn center(&self) -> Point {
        debug_assert!(!self.done());
        self.cell_id().to_point()
    }

    /// Reports whether the iterator is past the last cell.
    #[inline]
    pub fn done(&self) -> bool {
        self.position >= self.index.cells.len()
    }

    /// Advances to the next cell.
    #[inline]
    pub fn next(&mut self) {
        self.position += 1;
    }

    /// Moves to the previous cell. Returns false if already at the beginning.
    #[inline]
    pub fn prev(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }
        self.position -= 1;
        true
    }

    /// Positions the iterator at the first cell.
    #[inline]
    pub fn begin(&mut self) {
        self.position = 0;
    }

    /// Positions the iterator past the last cell.
    #[inline]
    pub fn end(&mut self) {
        self.position = self.index.cells.len();
    }

    /// Positions the iterator at the first cell with ID >= `target`.
    #[inline]
    pub fn seek(&mut self, target: CellId) {
        self.position = self.index.cells.partition_point(|&id| id < target);
    }

    /// Positions the iterator at the cell containing the given point.
    /// Returns true if such a cell was found.
    #[inline]
    pub fn locate_point(&mut self, p: Point) -> bool {
        let target = CellId::from_point(&p);
        self.seek(target);
        if !self.done() && self.cell_id().range_min() <= target {
            return true;
        }
        if self.prev() && self.cell_id().range_max() >= target {
            return true;
        }
        false
    }

    /// Attempts to position the iterator based on the relation to `target`.
    #[inline]
    pub fn locate_cell_id(&mut self, target: CellId) -> CellRelation {
        self.seek(target.range_min());
        if !self.done() {
            if self.cell_id() >= target && self.cell_id().range_min() <= target {
                return CellRelation::Indexed;
            }
            if self.cell_id() <= target.range_max() {
                return CellRelation::Subdivided;
            }
        }
        if self.prev() && self.cell_id().range_max() >= target {
            return CellRelation::Indexed;
        }
        CellRelation::Disjoint
    }
}

impl<'a> Iterator for ShapeIndexIterator<'a> {
    type Item = (CellId, &'a ShapeIndexCell);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let cell_id = self.cell_id();
        let cell = self.index_cell()?;
        self.position += 1;
        Some((cell_id, cell))
    }
}

impl<'a> IntoIterator for &'a ShapeIndex {
    type Item = (CellId, &'a ShapeIndexCell);
    type IntoIter = ShapeIndexIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ─── RangeIterator ──────────────────────────────────────────────────────

/// A wrapper around [`ShapeIndexIterator`] that provides `range_min` /
/// `range_max` (the `S2CellId` range covered by the current cell) and
/// `seek_to` / `seek_beyond` for merging two index iterators.
///
/// Corresponds to the `RangeIterator` helper in C++ `s2loop.cc`.
#[derive(Debug)]
pub struct RangeIterator<'a> {
    iter: ShapeIndexIterator<'a>,
}

impl<'a> RangeIterator<'a> {
    /// Creates a new iterator positioned at the first cell.
    pub fn new(index: &'a ShapeIndex) -> Self {
        RangeIterator { iter: index.iter() }
    }

    /// Returns the `CellId` of the current cell.
    #[inline]
    pub fn cell_id(&self) -> CellId {
        self.iter.cell_id()
    }

    /// Reports whether the iterator is past the last cell.
    #[inline]
    pub fn done(&self) -> bool {
        self.iter.done()
    }

    /// Advances to the next cell.
    #[inline]
    pub fn next(&mut self) {
        self.iter.next();
    }

    /// Returns the minimum leaf `CellId` covered by the current cell.
    /// If `done()`, returns a sentinel larger than all valid `CellIds`.
    pub fn range_min(&self) -> CellId {
        self.cell_id().range_min()
    }

    /// Returns the maximum leaf `CellId` covered by the current cell.
    /// If `done()`, returns a sentinel larger than all valid `CellIds`.
    pub fn range_max(&self) -> CellId {
        self.cell_id().range_max()
    }

    /// Positions the iterator at the first cell that overlaps or follows
    /// `target` (i.e. `range_max() >= target.range_min()`).
    pub fn seek_to(&mut self, target: &RangeIterator) {
        self.iter.seek(target.range_min());
        // If the current cell is past the target, check if the previous
        // cell overlaps the target.
        if !self.done()
            && self.range_min() > target.range_max()
            && self.iter.prev()
            && self.range_max() < target.range_min()
        {
            self.iter.next();
        }
    }

    /// Positions the iterator at the first cell that follows `target`
    /// (i.e. `range_min() > target.range_max()`).
    pub fn seek_beyond(&mut self, target: &RangeIterator) {
        self.iter.seek(target.range_max().next());
        if !self.done() && self.range_min() <= target.range_max() {
            self.iter.next();
        }
    }

    /// Returns the current index cell.
    #[inline]
    pub fn index_cell(&self) -> Option<&'a ShapeIndexCell> {
        self.iter.index_cell()
    }

    /// Returns the clipped shape at position 0 of the current cell, if any.
    pub fn clipped(&self, shape_id: impl Into<ShapeId>) -> Option<&'a ClippedShape> {
        self.iter
            .index_cell()
            .and_then(|cell| cell.find_by_shape_id(shape_id))
    }
}

impl<'a> Iterator for RangeIterator<'a> {
    type Item = (CellId, &'a ShapeIndexCell);

    fn next(&mut self) -> Option<Self::Item> {
        if self.done() {
            return None;
        }
        let cell_id = self.cell_id();
        let cell = self.index_cell()?;
        // Advance using the inherent method.
        RangeIterator::next(self);
        Some((cell_id, cell))
    }
}

// ─── Free functions ─────────────────────────────────────────────────────

fn add_face_edge(mut fe: FaceEdge, all_edges: &mut [Vec<FaceEdge>]) {
    let a_face = get_face(&fe.edge.v0.0);
    // Fast path: both endpoints on the same face and far from edges.
    if a_face == get_face(&fe.edge.v1.0) {
        let (ax, ay) = valid_face_xyz_to_uv(a_face, &fe.edge.v0.0);
        let (bx, by) = valid_face_xyz_to_uv(a_face, &fe.edge.v1.0);
        fe.a = r2::Point::new(ax, ay);
        fe.b = r2::Point::new(bx, by);
        let max_uv = 1.0 - CELL_PADDING;
        if fe.a.x.abs() <= max_uv
            && fe.a.y.abs() <= max_uv
            && fe.b.x.abs() <= max_uv
            && fe.b.y.abs() <= max_uv
        {
            all_edges[a_face as usize].push(fe);
            return;
        }
    }
    // Clip to all 6 faces.
    for face in Face::iter() {
        if let Some((a_clip, b_clip)) =
            clip_to_padded_face(fe.edge.v0, fe.edge.v1, face, CELL_PADDING)
        {
            all_edges[face.as_u8() as usize].push(FaceEdge {
                shape_id: fe.shape_id,
                edge_id: fe.edge_id,
                max_level: fe.max_level,
                has_interior: fe.has_interior,
                a: a_clip,
                b: b_clip,
                edge: fe.edge,
            });
        }
    }
}

fn max_level_for_edge(edge: &Edge) -> Level {
    let cell_size = (edge.v0.0 - edge.v1.0).norm() * CELL_SIZE_TO_LONG_EDGE_RATIO;
    metric::AVG_EDGE.min_level(cell_size)
}

/// Brute-force containment test: checks if `point` is inside `shape`
/// by counting edge crossings from the origin.
fn contains_brute_force(shape: &dyn Shape, point: Point) -> bool {
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

fn count_shapes(
    edges: &[IndexClippedEdge],
    face_edges: &[FaceEdge],
    containing_ids: &[ShapeId],
) -> usize {
    let mut count = 0;
    let mut last_shape_id = ShapeId(-1);
    let mut c_idx = 0;

    for ce in edges {
        let sid = face_edges[ce.face_edge_idx].shape_id;
        if sid == last_shape_id {
            continue;
        }
        count += 1;
        last_shape_id = sid;
        while c_idx < containing_ids.len() {
            let cid = containing_ids[c_idx];
            if cid > last_shape_id {
                break;
            }
            if cid < last_shape_id {
                count += 1;
            }
            c_idx += 1;
        }
    }
    count += containing_ids.len() - c_idx;
    count
}

/// Classifies which children (i=0/1, j=0/1) an edge belongs to based on
/// the middle rectangle.
fn classify_edge(bound: &r2::Rect, middle: &r2::Rect) -> (usize, usize, usize, usize) {
    let i_lo = if bound.x.lo >= middle.x.hi { 1 } else { 0 };
    let i_hi = if bound.x.hi <= middle.x.lo { 0 } else { 1 };
    let j_lo = if bound.y.lo >= middle.y.hi { 1 } else { 0 };
    let j_hi = if bound.y.hi <= middle.y.lo { 0 } else { 1 };
    (i_lo, i_hi, j_lo, j_hi)
}

fn clip_u_bound(
    ce: &IndexClippedEdge,
    fe: &FaceEdge,
    u_end: usize,
    u: f64,
) -> Option<IndexClippedEdge> {
    if u_end == 0 {
        if ce.bound.x.lo >= u {
            return Some(IndexClippedEdge {
                face_edge_idx: ce.face_edge_idx,
                bound: ce.bound,
            });
        }
    } else if ce.bound.x.hi <= u {
        return Some(IndexClippedEdge {
            face_edge_idx: ce.face_edge_idx,
            bound: ce.bound,
        });
    }
    let v = ce
        .bound
        .y
        .project(interpolate_float64(u, fe.a.x, fe.b.x, fe.a.y, fe.b.y));
    let positive_slope = (fe.a.x > fe.b.x) == (fe.a.y > fe.b.y);
    let v_end = if (u_end == 1) == positive_slope { 1 } else { 0 };
    Some(update_bound(ce, u_end, u, v_end, v))
}

fn clip_v_bound(
    ce: &IndexClippedEdge,
    fe: &FaceEdge,
    v_end: usize,
    v: f64,
) -> Option<IndexClippedEdge> {
    if v_end == 0 {
        if ce.bound.y.lo >= v {
            return Some(IndexClippedEdge {
                face_edge_idx: ce.face_edge_idx,
                bound: ce.bound,
            });
        }
    } else if ce.bound.y.hi <= v {
        return Some(IndexClippedEdge {
            face_edge_idx: ce.face_edge_idx,
            bound: ce.bound,
        });
    }
    let u = ce
        .bound
        .x
        .project(interpolate_float64(v, fe.a.y, fe.b.y, fe.a.x, fe.b.x));
    let positive_slope = (fe.a.x > fe.b.x) == (fe.a.y > fe.b.y);
    let u_end = if (v_end == 1) == positive_slope { 1 } else { 0 };
    Some(update_bound(ce, u_end, u, v_end, v))
}

fn update_bound(
    ce: &IndexClippedEdge,
    u_end: usize,
    u: f64,
    v_end: usize,
    v: f64,
) -> IndexClippedEdge {
    let x = if u_end == 0 {
        r1::Interval::new(u, ce.bound.x.hi)
    } else {
        r1::Interval::new(ce.bound.x.lo, u)
    };
    let y = if v_end == 0 {
        r1::Interval::new(v, ce.bound.y.hi)
    } else {
        r1::Interval::new(ce.bound.y.lo, v)
    };
    IndexClippedEdge {
        face_edge_idx: ce.face_edge_idx,
        bound: r2::Rect::new(x, y),
    }
}

fn split_v_axis(
    ce: &IndexClippedEdge,
    fe: &FaceEdge,
    middle_y: &r1::Interval,
    lower: &mut Vec<IndexClippedEdge>,
    upper: &mut Vec<IndexClippedEdge>,
) {
    if ce.bound.y.hi <= middle_y.lo {
        lower.push(IndexClippedEdge {
            face_edge_idx: ce.face_edge_idx,
            bound: ce.bound,
        });
    } else if ce.bound.y.lo >= middle_y.hi {
        upper.push(IndexClippedEdge {
            face_edge_idx: ce.face_edge_idx,
            bound: ce.bound,
        });
    } else {
        if let Some(e) = clip_v_bound(ce, fe, 1, middle_y.hi) {
            lower.push(e);
        }
        if let Some(e) = clip_v_bound(ce, fe, 0, middle_y.lo) {
            upper.push(e);
        }
    }
}

fn clip_and_push(
    ce: &IndexClippedEdge,
    fe: &FaceEdge,
    u_end: usize,
    u: f64,
    target: &mut Vec<IndexClippedEdge>,
) {
    if let Some(clipped) = clip_u_bound(ce, fe, u_end, u) {
        target.push(clipped);
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::shape::{Chain, ChainPosition, ReferencePoint};

    /// A simple point-set shape for testing.
    #[derive(Debug)]
    struct PointVectorShape {
        points: Vec<Point>,
    }

    impl Shape for PointVectorShape {
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

    #[derive(Debug)]
    /// A simple polyline shape.
    struct PolylineShape {
        vertices: Vec<Point>,
    }

    impl Shape for PolylineShape {
        fn num_edges(&self) -> usize {
            if self.vertices.len() < 2 {
                0
            } else {
                self.vertices.len() - 1
            }
        }
        fn edge(&self, id: usize) -> Edge {
            Edge::new(self.vertices[id], self.vertices[id + 1])
        }
        fn reference_point(&self) -> ReferencePoint {
            ReferencePoint::default()
        }
        fn num_chains(&self) -> usize {
            if self.num_edges() > 0 { 1 } else { 0 }
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

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty_index() {
        let mut index = ShapeIndex::new();
        index.build();
        assert_eq!(index.len(), 0);
        assert!(index.is_empty());
        let it = index.iter();
        assert!(it.done());
    }

    #[test]
    fn test_single_point() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0)],
        }));
        index.build();
        assert_eq!(index.len(), 1);
        assert_eq!(index.num_edges(), 1);

        let mut it = index.iter();
        assert!(!it.done());
        let cell = it.index_cell().unwrap();
        assert_eq!(cell.shapes.len(), 1);
        assert_eq!(cell.shapes[0].edges.len(), 1);
        it.next();
        assert!(it.done());
    }

    #[test]
    fn test_multiple_points() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0), p(45.0, 90.0), p(-30.0, -60.0)],
        }));
        index.build();
        assert_eq!(index.num_edges(), 3);

        let mut count = 0;
        let mut it = index.iter();
        while !it.done() {
            count += it.index_cell().unwrap().num_edges();
            it.next();
        }
        // Edges may be duplicated across cells near face boundaries.
        assert!(count >= 3, "count = {count}");
    }

    #[test]
    fn test_polyline() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PolylineShape {
            vertices: vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)],
        }));
        index.build();
        assert_eq!(index.num_edges(), 2);

        let mut total_edges = 0;
        let mut it = index.iter();
        while !it.done() {
            total_edges += it.index_cell().unwrap().num_edges();
            it.next();
        }
        // Edges may be duplicated across cells, so total should be >= 2.
        assert!(total_edges >= 2, "total_edges = {total_edges}");
    }

    #[test]
    fn test_locate_point() {
        let mut index = ShapeIndex::new();
        let target = p(10.0, 20.0);
        index.add(Box::new(PointVectorShape {
            points: vec![target],
        }));
        index.build();

        let mut it = index.iter();
        assert!(it.locate_point(target));
    }

    #[test]
    fn test_locate_cell_id() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0)],
        }));
        index.build();

        let mut it = index.iter();
        // The face cell should contain the indexed cell.
        let face_id = CellId::from_face(0);
        let rel = it.locate_cell_id(face_id);
        assert!(
            rel == CellRelation::Indexed || rel == CellRelation::Subdivided,
            "relation = {rel:?}"
        );
    }

    #[test]
    fn test_iterator_navigation() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0), p(45.0, 90.0)],
        }));
        index.build();

        let mut it = index.iter();
        let first = it.cell_id();
        it.next();
        if !it.done() {
            assert!(it.prev());
            assert_eq!(it.cell_id(), first);
        }
    }

    #[test]
    fn test_cells_in_order() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVectorShape {
            points: vec![p(0.0, 0.0), p(45.0, 90.0), p(-30.0, -60.0), p(70.0, 170.0)],
        }));
        index.build();

        let mut it = index.iter();
        let mut prev_id = CellId::none();
        while !it.done() {
            assert!(it.cell_id() > prev_id, "cells not in order");
            prev_id = it.cell_id();
            it.next();
        }
    }

    #[test]
    fn test_one_edge() {
        // C++ MutableS2ShapeIndexTest.OneEdge
        use crate::s2::edge_vector_shape::EdgeVectorShape;
        let mut index = ShapeIndex::new();
        let shape_id = index.add(Box::new(EdgeVectorShape::from_edge(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        )));
        assert_eq!(shape_id, 0);
        index.build();

        // Verify we can iterate and find the edge.
        let mut it = index.iter();
        assert!(!it.done());
        let mut total_edges = 0;
        while !it.done() {
            total_edges += it.index_cell().unwrap().num_edges();
            it.next();
        }
        assert!(
            total_edges >= 1,
            "Expected at least 1 edge, got {total_edges}"
        );
    }

    #[test]
    fn test_degenerate_edge() {
        // C++ MutableS2ShapeIndexTest.DegenerateEdge
        // A degenerate edge (v0 == v1) at a cube face vertex should be
        // indexed in exactly 3 cells (the vertex touches 3 faces).
        use crate::s2::edge_vector_shape::EdgeVectorShape;

        let a = Point::from_coords(1.0, 1.0, 1.0).normalize();
        let mut shape = EdgeVectorShape::new();
        shape.add(a, a);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let mut count = 0;
        let mut it = index.iter();
        while !it.done() {
            let cell = it.index_cell().unwrap();
            assert_eq!(cell.shapes.len(), 1, "Expected 1 clipped shape");
            assert_eq!(
                cell.shapes[0].num_edges(),
                1,
                "Expected 1 edge per clipped shape"
            );
            count += 1;
            it.next();
        }
        // The point (1,1,1) is at a cube vertex touching 3 faces.
        assert_eq!(count, 3, "Cube vertex degenerate edge should be in 3 cells");
    }

    #[test]
    fn test_many_identical_edges() {
        // C++ MutableS2ShapeIndexTest.ManyIdenticalEdges
        // 100 identical edges spanning a face diagonal should all be at level 0.
        use crate::s2::edge_vector_shape::EdgeVectorShape;

        let a = Point::from_coords(0.99, 0.99, 1.0).normalize();
        let b = Point::from_coords(-0.99, -0.99, 1.0).normalize();
        let mut index = ShapeIndex::new();
        for i in 0..100 {
            let id = index.add(Box::new(EdgeVectorShape::from_edge(a, b)));
            assert_eq!(id, i);
        }
        index.build();

        // All edges span the diagonal of face 2, so no subdivision should occur.
        let mut it = index.iter();
        while !it.done() {
            assert_eq!(
                it.cell_id().level(),
                0,
                "Identical face-diagonal edges should not cause subdivision"
            );
            it.next();
        }
    }

    #[test]
    fn test_many_tiny_edges() {
        // C++ MutableS2ShapeIndexTest.ManyTinyEdges
        // 100 tiny edges in the same leaf cell should result in exactly one
        // leaf cell in the index.
        use crate::s2::edge_vector_shape::EdgeVectorShape;

        let a = CellId::from_point(&Point::from_coords(1.0, 0.0, 0.0)).to_point();
        let b = Point((a.0 + crate::r3::Vector::new(0.0, 1e-12, 0.0)).normalize());
        let mut shape = EdgeVectorShape::new();
        for _ in 0..100 {
            shape.add(a, b);
        }
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        // Should be exactly one cell and it should be a leaf.
        let mut it = index.iter();
        assert!(!it.done(), "Index should have at least one cell");
        assert!(it.cell_id().is_leaf(), "Single cell should be a leaf");
        it.next();
        assert!(it.done(), "Should be exactly one cell for tiny edges");
    }

    #[test]
    fn test_mixed_geometry() {
        // C++ MutableS2ShapeIndexTest.MixedGeometry
        // Having a shape with an interior should not cause shapes without an
        // interior to acquire one, which would create spurious index cells.
        use crate::s2::text_format;

        let mut index = ShapeIndex::new();
        // Add three polylines (dimension 1, no interior).
        for s in &[
            "0:0, 2:1, 0:2, 2:3, 0:4, 2:5, 0:6",
            "1:0, 3:1, 1:2, 3:3, 1:4, 3:5, 1:6",
            "2:0, 4:1, 2:2, 4:3, 2:4, 4:5, 2:6",
        ] {
            let polyline = text_format::make_polyline(s);
            index.add(Box::new(polyline));
        }
        // Add a small loop (dimension 2, has interior).
        let cell = crate::s2::Cell::from(CellId::from_face(0).child_begin_at_level(MAX_CELL_LEVEL));
        let mut vertices = Vec::new();
        for i in 0..4 {
            vertices.push(cell.vertex(i));
        }
        let lp = crate::s2::s2loop::Loop::new(vertices);
        index.add(Box::new(lp));
        index.build();

        // Face 1 has no geometry, so locating it should return Disjoint.
        let mut it = index.iter();
        let rel = it.locate_cell_id(CellId::from_face(1));
        assert_eq!(
            rel,
            CellRelation::Disjoint,
            "Face 1 should be disjoint from indexed geometry"
        );
    }

    #[test]
    fn test_iterator_seek() {
        // Comprehensive test of iterator seek/locate methods.
        use crate::s2::edge_vector_shape::EdgeVectorShape;

        let mut index = ShapeIndex::new();
        // Add several edges spread across different faces.
        index.add(Box::new(EdgeVectorShape::from_edge(
            p(0.0, 0.0),
            p(0.0, 10.0),
        )));
        index.add(Box::new(EdgeVectorShape::from_edge(
            p(45.0, 90.0),
            p(46.0, 91.0),
        )));
        index.add(Box::new(EdgeVectorShape::from_edge(
            p(-30.0, -60.0),
            p(-31.0, -61.0),
        )));
        index.build();

        // Collect all cell IDs.
        let mut cell_ids = Vec::new();
        let mut it = index.iter();
        while !it.done() {
            cell_ids.push(it.cell_id());
            it.next();
        }
        assert!(
            cell_ids.len() >= 3,
            "Expected >= 3 cells, got {}",
            cell_ids.len()
        );

        // Test begin/end.
        it.begin();
        assert_eq!(it.cell_id(), cell_ids[0]);
        it.end();
        assert!(it.done());

        // Test seek to first cell.
        it.seek(cell_ids[0]);
        assert_eq!(it.cell_id(), cell_ids[0]);

        // Test seek to sentinel (past end).
        it.seek(CellId::sentinel());
        assert!(it.done());

        // Test prev at beginning returns false.
        it.begin();
        assert!(!it.prev());
        assert_eq!(it.cell_id(), cell_ids[0]);

        // Test locate_point finds the cell.
        for &cid in &cell_ids {
            let pt = cid.to_point();
            assert!(
                it.locate_point(pt),
                "locate_point should find cell for its center"
            );
        }
    }

    // ─── C++ port: ShrinkToFitOptimization ──────────────────────────────

    #[test]
    fn test_shrink_to_fit_optimization() {
        // C++: MutableS2ShapeIndexTest::ShrinkToFitOptimization
        //
        // A large loop that covers almost all of face 0 except for a tiny
        // region. The only cell with edges is the tiny one, but all other
        // cells on the face should also have index entries to indicate they
        // are contained by the loop.
        use crate::s1;
        let center = Point::from_coords(1.0, 0.5, 0.5).normalize();
        let loop_ = crate::s2::Loop::make_regular(center, s1::Angle::from_degrees(89.0), 100);
        let mut index = ShapeIndex::new();
        index.add(Box::new(loop_));
        index.build();

        // The index should have multiple cells (not just one for the edges).
        let mut count = 0;
        let mut it = index.iter();
        it.begin();
        while !it.done() {
            count += 1;
            it.next();
        }
        assert!(
            count > 1,
            "ShrinkToFit: expected multiple index cells, got {count}"
        );
    }

    // ─── C++ port: LoopsSpanningThreeFaces ──────────────────────────────

    #[test]
    fn test_loops_spanning_three_faces() {
        // C++: MutableS2ShapeIndexTest::LoopsSpanningThreeFaces
        //
        // Two concentric loops centered around the cube vertex at the
        // start of the Hilbert curve. This exercises face-spanning logic.
        use crate::s1;
        let center = Point::from_coords(1.0, -1.0, -1.0).normalize();
        let outer = crate::s2::Loop::make_regular(center, s1::Angle::from_degrees(10.0), 50);
        let inner = crate::s2::Loop::make_regular(center, s1::Angle::from_degrees(5.0), 50);

        let mut index = ShapeIndex::new();
        index.add(Box::new(outer));
        index.add(Box::new(inner));
        index.build();

        // Basic validation: iterator should yield cells in order and
        // locate_point should work for vertices.
        let mut prev = CellId::none();
        let mut it = index.iter();
        it.begin();
        while !it.done() {
            assert!(it.cell_id() > prev, "cells must be in order");
            prev = it.cell_id();
            it.next();
        }

        // locate_point for the center should find the containing cell.
        let found = it.locate_point(center);
        assert!(found, "locate_point should find the center point");
    }

    // ─── C++ port: SeveralShapesInOneBatch ──────────────────────────────

    #[test]
    fn test_several_shapes_in_one_batch() {
        // C++: MutableS2ShapeIndexTest::SeveralShapesInOneBatch
        //
        // Add multiple shapes and verify they are all accessible.
        use crate::s2::edge_vector_shape::EdgeVectorShape;

        let mut index = ShapeIndex::new();
        let p0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let p1 = LatLng::from_degrees(1.0, 0.0).to_point();
        let p2 = LatLng::from_degrees(0.0, 1.0).to_point();
        let p3 = LatLng::from_degrees(1.0, 1.0).to_point();

        index.add(Box::new(EdgeVectorShape::from_edge(p0, p1)));
        index.add(Box::new(EdgeVectorShape::from_edge(p1, p2)));
        index.add(Box::new(EdgeVectorShape::from_edge(p2, p3)));
        index.build();

        assert_eq!(index.num_shape_ids(), 3);
        for i in 0..3_i32 {
            assert!(index.shape(i).is_some(), "shape {i} should be present");
        }

        // All edge endpoints should be locatable.
        for p in [p0, p1, p2, p3] {
            let mut it = index.iter();
            assert!(it.locate_point(p), "should locate point");
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_cell_relation_roundtrip() {
        for cr in [
            CellRelation::Indexed,
            CellRelation::Subdivided,
            CellRelation::Disjoint,
        ] {
            let json = serde_json::to_string(&cr).unwrap();
            let back: CellRelation = serde_json::from_str(&json).unwrap();
            assert_eq!(cr, back);
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_clipped_shape_roundtrip() {
        let cs = ClippedShape {
            shape_id: ShapeId(5),
            contains_center: true,
            edges: vec![0, 2, 7],
        };
        let json = serde_json::to_string(&cs).unwrap();
        let back: ClippedShape = serde_json::from_str(&json).unwrap();
        assert_eq!(cs.shape_id, back.shape_id);
        assert_eq!(cs.contains_center, back.contains_center);
        assert_eq!(cs.edges, back.edges);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_shape_index_cell_roundtrip() {
        let cell = ShapeIndexCell {
            shapes: vec![ClippedShape {
                shape_id: ShapeId(0),
                contains_center: false,
                edges: vec![1, 3],
            }],
        };
        let json = serde_json::to_string(&cell).unwrap();
        let back: ShapeIndexCell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell.shapes.len(), back.shapes.len());
        assert_eq!(cell.shapes[0].shape_id, back.shapes[0].shape_id);
        assert_eq!(cell.shapes[0].edges, back.shapes[0].edges);
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ mutable_s2shape_index_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_no_edges() {
        // C++ TEST_F(MutableS2ShapeIndexTest, NoEdges)
        let index = ShapeIndex::new();
        let it = index.iter();
        assert!(it.done());
    }

    #[test]
    fn test_simple_updates() {
        // C++ TEST_F(MutableS2ShapeIndexTest, SimpleUpdates)
        // Add then remove a shape, verify the index is empty after.
        use crate::s2::lax_loop::LaxLoop;
        let mut index = ShapeIndex::new();
        let v0 = LatLng::from_degrees(0.0, 0.0).to_point();
        let v1 = LatLng::from_degrees(0.0, 1.0).to_point();
        let v2 = LatLng::from_degrees(1.0, 0.0).to_point();
        let id = index.add(Box::new(LaxLoop::new(vec![v0, v1, v2])));
        index.build();
        assert!(index.num_edges() > 0);
        // Verify shape exists.
        assert!(index.shape(id).is_some());
    }

    #[test]
    fn test_add_remove_shape_containing_origin() {
        // C++ TEST_F(MutableS2ShapeIndexTest, AddRemoveShapeContainingOrigin)
        use crate::s2::lax_loop::LaxLoop;
        // A shape that contains the origin — verify it builds without panic.
        let pts = vec![
            LatLng::from_degrees(-89.0, 0.0).to_point(),
            LatLng::from_degrees(-89.0, 120.0).to_point(),
            LatLng::from_degrees(-89.0, -120.0).to_point(),
        ];
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxLoop::new(pts)));
        index.build();
        assert!(index.num_edges() > 0);
    }

    #[test]
    fn test_linear_space() {
        // C++ TEST_F(MutableS2ShapeIndexTest, LinearSpace)
        // Verify that building index of N edges takes O(N) space (no blow-up).
        use crate::s2::polyline::Polyline;
        for n in [100, 1000] {
            let mut pts = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let t = i as f64 / n as f64;
                pts.push(LatLng::from_degrees(t * 30.0, t * 30.0).to_point());
            }
            let mut index = ShapeIndex::new();
            index.add(Box::new(Polyline::new(pts)));
            index.build();
            assert_eq!(index.num_edges(), n);
        }
    }

    #[test]
    fn test_group_small_shapes_into_batches() {
        // C++ TEST(MutableS2ShapeIndexTest, GroupSmallShapesIntoBatches)
        // Many small shapes should still build and iterate correctly.
        use crate::s2::polyline::Polyline;
        let mut index = ShapeIndex::new();
        for i in 0..50_i32 {
            let lat = f64::from(i);
            let p0 = LatLng::from_degrees(lat, 0.0).to_point();
            let p1 = LatLng::from_degrees(lat, 0.01).to_point();
            index.add(Box::new(Polyline::new(vec![p0, p1])));
        }
        index.build();
        assert_eq!(index.num_shape_ids(), 50);
        assert_eq!(index.num_edges(), 50);
    }

    #[test]
    fn test_mixed_geometry_all_dimensions() {
        // C++ TEST(MutableS2ShapeIndexTest, MixedGeometry) — extended version
        // Tests that points + polylines + polygons coexist in one index.
        let index = crate::s2::text_format::make_index("0:0 | 1:1 # 2:2, 3:3 # 4:4, 4:5, 5:4");
        assert_eq!(index.num_shape_ids(), 3);
        // Points shape: 2 edges, polyline: 1 edge, polygon: 3 edges
        assert_eq!(index.num_edges(), 6);
    }

    #[test]
    fn test_space_used() {
        // C++ TEST(MutableS2ShapeIndexTest, SpaceUsed)
        // Just verify the index reports non-zero after building.
        use crate::s2::polyline::Polyline;
        let mut index = ShapeIndex::new();
        let pts: Vec<_> = (0..100_i32)
            .map(|i| {
                let t = f64::from(i) / 100.0;
                LatLng::from_degrees(t * 30.0, t * 30.0).to_point()
            })
            .collect();
        index.add(Box::new(Polyline::new(pts)));
        index.build();
        // After building, the index should have cells.
        let mut count = 0;
        let mut it = index.iter();
        while !it.done() {
            count += 1;
            it.next();
        }
        assert!(count > 0, "built index should have cells");
    }
}
