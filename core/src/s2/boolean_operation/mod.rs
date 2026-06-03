// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Boolean operations on regions defined by geodesic edges.
//!
//! [`S2BooleanOperation`] computes the union, intersection, difference, or
//! symmetric difference of two collections of geometry (points, polylines,
//! and polygons) stored in [`ShapeIndex`] instances.
//!
//! The operation handles all combinations of geometry dimensions and supports
//! three boundary models for polygons ([`PolygonModel`]) and polylines
//! ([`PolylineModel`]): open, semi-open, and closed. The semi-open model
//! (the default) ensures that when polygons tile the sphere, every point is
//! contained by exactly one polygon.
//!
//! Results are produced via an [`S2Builder`]
//! layer, so the output geometry is automatically snapped and topologically
//! valid.

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/ShapeId (i32) used as Vec indices in boolean operation"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId/ShapeId (i32) <-> usize for Vec indexing in boolean ops"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId/ShapeId — always in range"
)]
mod crossing_processor;
mod graph_edge_clipper;

use std::cmp::min;
use std::f64::consts::PI;
use std::ops::ControlFlow;

use crate::s1;
use crate::s2::builder::graph::Graph;
use crate::s2::builder::layer::Layer;
use crate::s2::builder::snap::IdentitySnapFunction;
use crate::s2::builder::{InputEdgeId, S2Builder, S2Error, S2ErrorCode};
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::edge_crossings;
use crate::s2::predicates;
use crate::s2::shape::{Dimension, Shape, ShapeEdge, ShapeEdgeId, ShapeId};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_util;

use crossing_processor::{CrossingIterator, CrossingProcessor};

// ─── Public types ────────────────────────────────────────────────────────

/// The supported operation types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OpType {
    /// Contained by either region.
    #[default]
    Union,
    /// Contained by both regions.
    Intersection,
    /// Contained by the first region but not the second.
    Difference,
    /// Contained by one region but not the other.
    SymmetricDifference,
}

/// Defines whether polygons are considered to contain their vertices/edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolygonModel {
    /// Polygons do not contain their vertices or edges.
    Open,
    /// Polygon containment is defined such that if several polygons tile a
    /// region, exactly one contains each shared vertex. Polygons contain
    /// their edges but not their reversed edges.
    #[default]
    SemiOpen,
    /// Polygons contain all their vertices, edges, and reversed edges.
    Closed,
}

/// Defines whether polylines are considered to contain their endpoints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolylineModel {
    /// Polylines do not contain their first or last vertex (except for loops
    /// when `polyline_loops_have_boundaries` is false).
    Open,
    /// Polylines contain all vertices except the last.
    SemiOpen,
    /// Polylines contain all of their vertices.
    #[default]
    Closed,
}

/// Identifies one of the two input regions in a boolean operation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum RegionId {
    /// The first input region (index 0).
    #[default]
    A = 0,
    /// The second input region (index 1).
    B = 1,
}

impl RegionId {
    /// Returns the other region.
    pub const fn other(self) -> Self {
        match self {
            RegionId::A => RegionId::B,
            RegionId::B => RegionId::A,
        }
    }

    /// Returns the region as a `usize`, suitable for indexing.
    pub const fn as_usize(self) -> usize {
        self as usize
    }
}

impl From<RegionId> for u32 {
    fn from(r: RegionId) -> u32 {
        r as u32
    }
}

/// Identifies an edge from one of the two input `ShapeIndexes`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceId {
    region_id: RegionId,
    shape_id: ShapeId,
    edge_id: i32,
}

impl SourceId {
    /// Creates a new `SourceId` from a region id, shape id, and edge id.
    pub fn new(region_id: RegionId, shape_id: impl Into<ShapeId>, edge_id: i32) -> Self {
        SourceId {
            region_id,
            shape_id: shape_id.into(),
            edge_id,
        }
    }

    /// Creates a `SourceId` from a special edge id (used for sentinel edges).
    pub fn from_special(edge_id: InputEdgeId) -> Self {
        SourceId {
            region_id: RegionId::A,
            shape_id: ShapeId(0),
            edge_id: edge_id.0,
        }
    }

    /// Returns the region id.
    pub fn region_id(&self) -> RegionId {
        self.region_id
    }

    /// Returns the shape id within the region's `ShapeIndex`.
    pub fn shape_id(&self) -> ShapeId {
        self.shape_id
    }

    /// Returns the edge id within the shape.
    pub fn edge_id(&self) -> i32 {
        self.edge_id
    }
}

/// Options for `S2BooleanOperation`.
#[derive(Debug)]
pub struct Options {
    /// The snap function used to snap the output geometry.
    pub snap_function: Box<dyn crate::s2::builder::snap::SnapFunction>,
    /// Defines whether polygons contain their vertices and edges.
    /// Default: `SemiOpen`.
    pub polygon_model: PolygonModel,
    /// Defines whether polylines contain their endpoints.
    /// Default: `Closed`.
    pub polyline_model: PolylineModel,
    /// Whether a polyline loop has a non-empty boundary. If true (the
    /// default), even if the first and last vertices are the same, the
    /// polyline has a well-defined start and end.
    pub polyline_loops_have_boundaries: bool,
    /// Whether to add a new vertex whenever a polyline edge crosses
    /// another polyline edge. If false (the default), new vertices are
    /// added only when polylines from different input regions cross.
    pub split_all_crossing_polyline_edges: bool,
    /// Optional memory tracker for limiting and monitoring memory usage.
    /// Shared with the underlying `S2Builder`.
    ///
    /// C++: `S2BooleanOperation::Options::memory_tracker()`
    pub memory_tracker:
        Option<std::sync::Arc<std::sync::Mutex<crate::s2::memory_tracker::S2MemoryTracker>>>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            snap_function: Box::new(IdentitySnapFunction::new(s1::Angle::default())),
            polygon_model: PolygonModel::SemiOpen,
            polyline_model: PolylineModel::Closed,
            polyline_loops_have_boundaries: true,
            split_all_crossing_polyline_edges: false,
            memory_tracker: None,
        }
    }
}

// ─── Internal types ──────────────────────────────────────────────────────

/// Special `InputEdgeIds` for `GraphEdgeClipper` state modifications.
const K_SET_INSIDE: InputEdgeId = InputEdgeId(-1);
const K_SET_INVERT_B: InputEdgeId = InputEdgeId(-2);
const K_SET_REVERSE_A: InputEdgeId = InputEdgeId(-3);

/// Sentinel value for `ShapeEdgeId` to mark end of vectors.
const SENTINEL: ShapeEdgeId = ShapeEdgeId {
    shape_id: ShapeId(i32::MAX),
    edge_id: 0,
};

/// Represents a crossing input edge B that crosses some input edge A.
#[derive(Clone, Copy, Debug)]
struct CrossingInputEdge {
    input_id: InputEdgeId,
    left_to_right: bool,
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "matches C++ API")]
impl CrossingInputEdge {
    fn new(input_id: impl Into<InputEdgeId>, left_to_right: bool) -> Self {
        CrossingInputEdge {
            input_id: input_id.into(),
            left_to_right,
        }
    }
    fn input_id(&self) -> InputEdgeId {
        self.input_id
    }
    fn left_to_right(&self) -> bool {
        self.left_to_right
    }
}

impl PartialOrd for CrossingInputEdge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CrossingInputEdge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.input_id.cmp(&other.input_id)
    }
}

impl PartialEq for CrossingInputEdge {
    fn eq(&self, other: &Self) -> bool {
        self.input_id == other.input_id
    }
}

impl Eq for CrossingInputEdge {}

/// Pairs of (`InputEdgeId`, `CrossingInputEdge`) representing all intersections.
type InputEdgeCrossings = Vec<(InputEdgeId, CrossingInputEdge)>;

/// An `IndexCrossing` represents a pair of intersecting `ShapeIndex` edges.
#[derive(Clone, Debug)]
struct IndexCrossing {
    a: ShapeEdgeId,
    b: ShapeEdgeId,
    is_interior_crossing: bool,
    left_to_right: bool,
    is_vertex_crossing: bool,
}

impl IndexCrossing {
    fn new(a: ShapeEdgeId, b: ShapeEdgeId) -> Self {
        IndexCrossing {
            a,
            b,
            is_interior_crossing: false,
            left_to_right: false,
            is_vertex_crossing: false,
        }
    }
}

impl PartialEq for IndexCrossing {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b
    }
}

impl Eq for IndexCrossing {}

impl PartialOrd for IndexCrossing {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexCrossing {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.a.cmp(&other.a).then(self.b.cmp(&other.b))
    }
}

// ─── S2BooleanOperation ─────────────────────────────────────────────────

/// Computes boolean operations on two `S2ShapeIndex` regions.
///
/// Supports union, intersection, difference, and symmetric difference
/// on arbitrary collections of points, polylines, and polygons.
///
/// # Examples
///
/// ```
/// use s2rst::s2::boolean_operation::{S2BooleanOperation, OpType, Options};
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::LatLng;
///
/// // Check whether two polygons intersect.
/// let mut a = ShapeIndex::new();
/// a.add(Box::new(s2rst::s2::lax_loop::LaxLoop::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 1.0).to_point(),
///     LatLng::from_degrees(1.0, 1.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
/// ])));
/// a.build();
/// let mut b = ShapeIndex::new();
/// b.add(Box::new(s2rst::s2::lax_loop::LaxLoop::new(vec![
///     LatLng::from_degrees(0.5, 0.5).to_point(),
///     LatLng::from_degrees(0.5, 1.5).to_point(),
///     LatLng::from_degrees(1.5, 1.5).to_point(),
///     LatLng::from_degrees(1.5, 0.5).to_point(),
/// ])));
/// b.build();
/// assert!(S2BooleanOperation::intersects(&mut a, &mut b, Options::default()));
/// ```
#[derive(Debug)]
pub struct S2BooleanOperation {
    op_type: OpType,
    options: Options,
    layers: Vec<Box<dyn Layer>>,
    result_empty: Option<bool>,
}

impl S2BooleanOperation {
    /// Creates an operation that sends output to a single layer.
    pub fn new(op_type: OpType, layer: Box<dyn Layer>, options: Options) -> Self {
        S2BooleanOperation {
            op_type,
            options,
            layers: vec![layer],
            result_empty: None,
        }
    }

    /// Creates an operation that sends output to three layers (one per dimension).
    pub fn multi(op_type: OpType, layers: Vec<Box<dyn Layer>>, options: Options) -> Self {
        debug_assert!(layers.len() == 3);
        S2BooleanOperation {
            op_type,
            options,
            layers,
            result_empty: None,
        }
    }

    /// Creates a predicate-only operation (no output layers).
    fn new_predicate(op_type: OpType, options: Options) -> Self {
        S2BooleanOperation {
            op_type,
            options,
            layers: Vec::new(),
            result_empty: Some(false),
        }
    }

    /// Returns the operation type.
    pub fn op_type(&self) -> OpType {
        self.op_type
    }
    /// Returns the options for this operation.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// Executes the operation. On success, returns the layers with their
    /// built output. The caller can downcast each layer to extract geometry.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails (e.g., due to snap rounding issues).
    pub fn build(
        &mut self,
        a: &mut ShapeIndex,
        b: &mut ShapeIndex,
    ) -> Result<Vec<Box<dyn Layer>>, S2Error> {
        a.build();
        b.build();
        let mut error = S2Error::ok();
        let mut imp = Impl::new(self);
        let ok = imp.build(a, b, &mut error);
        if ok {
            Ok(std::mem::take(&mut self.layers))
        } else {
            Err(error)
        }
    }

    /// Returns true if the result of the given operation is empty.
    pub fn is_empty(
        op_type: OpType,
        a: &mut ShapeIndex,
        b: &mut ShapeIndex,
        options: Options,
    ) -> bool {
        a.build();
        b.build();
        let mut op = S2BooleanOperation::new_predicate(op_type, options);
        let mut error = S2Error::ok();
        let mut imp = Impl::new(&mut op);
        imp.build(a, b, &mut error);
        op.result_empty.unwrap_or(true)
    }

    /// Returns true if A intersects B.
    pub fn intersects(a: &mut ShapeIndex, b: &mut ShapeIndex, options: Options) -> bool {
        !Self::is_empty(OpType::Intersection, b, a, options)
    }

    /// Returns true if A contains B (i.e., B - A is empty).
    pub fn contains(a: &mut ShapeIndex, b: &mut ShapeIndex, options: Options) -> bool {
        Self::is_empty(OpType::Difference, b, a, options)
    }

    /// Returns true if the symmetric difference of A and B is empty.
    pub fn equals(a: &mut ShapeIndex, b: &mut ShapeIndex, options: Options) -> bool {
        Self::is_empty(OpType::SymmetricDifference, b, a, options)
    }
}

// ─── Impl: the actual algorithm ──────────────────────────────────────────

/// Records an intersection point and the two `ShapeEdge` IDs that cross.
struct InteriorCrossingRecord {
    point: crate::s2::Point,
    a: ShapeEdgeId,
    b: ShapeEdgeId,
}

struct Impl<'a> {
    op: &'a mut S2BooleanOperation,
    index_crossings: Vec<IndexCrossing>,
    index_crossings_first_region_id: Option<RegionId>,
    input_dimensions: Vec<Dimension>,
    input_crossings: InputEdgeCrossings,
    tmp_crossings: Vec<IndexCrossing>,
    builder: Option<S2Builder>,
    /// Interior crossings with their intersection points, for post-processing.
    interior_crossings: Vec<InteriorCrossingRecord>,
    /// Map from (`region_id`, `shape_id`, `edge_id`) to intersection points.
    crossing_point_map:
        Option<std::collections::HashMap<(RegionId, ShapeId, i32), Vec<crate::s2::Point>>>,
}

impl<'a> Impl<'a> {
    fn new(op: &'a mut S2BooleanOperation) -> Self {
        Impl {
            op,
            index_crossings: Vec::new(),
            index_crossings_first_region_id: None,
            input_dimensions: Vec::new(),
            input_crossings: Vec::new(),
            tmp_crossings: Vec::new(),
            builder: None,
            interior_crossings: Vec::new(),
            crossing_point_map: None,
        }
    }

    fn is_boolean_output(&self) -> bool {
        self.op.result_empty.is_some()
    }

    fn build(&mut self, a: &ShapeIndex, b: &ShapeIndex, error: &mut S2Error) -> bool {
        *error = S2Error::ok();
        self.do_build(a, b, error);
        error.code == S2ErrorCode::Ok
    }

    fn do_build(&mut self, a: &ShapeIndex, b: &ShapeIndex, error: &mut S2Error) {
        let builder_options = crate::s2::builder::Options {
            snap_function: self.op.options.snap_function.clone_snap(),
            intersection_tolerance: edge_crossings::intersection_error(),
            // Don't use the builder's automatic edge crossing detection unless
            // split_all_crossing_polyline_edges is set, which enables splitting
            // of same-region polyline self-intersections.
            split_crossing_edges: self.op.options.split_all_crossing_polyline_edges,
            idempotent: false,
            memory_tracker: self.op.options.memory_tracker.clone(),
            ..Default::default()
        };

        if self.is_boolean_output() {
            let op_type = self.op.op_type;
            let result_empty = self.build_op_type(op_type, a, b);
            let is_full = if result_empty {
                is_full_polygon_result_fn(op_type, a, b)
            } else {
                false
            };
            self.op.result_empty = Some(result_empty && !is_full);
            return;
        }

        use std::cell::RefCell;
        use std::rc::Rc;

        let shared_dims: Rc<RefCell<Vec<Dimension>>> = Rc::new(RefCell::new(Vec::new()));
        let shared_crossings: Rc<RefCell<InputEdgeCrossings>> = Rc::new(RefCell::new(Vec::new()));

        let layers = std::mem::take(&mut self.op.layers);

        let clipping_layer = SharedEdgeClippingLayer {
            layers,
            input_dimensions: Rc::clone(&shared_dims),
            input_crossings: Rc::clone(&shared_crossings),
        };

        // Pre-compute values for the is_full_polygon predicate (captures only Copy types).
        let op_type = self.op.op_type;
        let a_face_mask = get_face_mask(a);
        let b_face_mask = get_face_mask(b);
        let a_area = get_area(a);
        let b_area = get_area(b);
        let edge_snap_radius_radians = builder_options.edge_snap_radius().radians();

        let mut builder = S2Builder::new(builder_options);
        builder.start_layer(Box::new(clipping_layer));
        builder.add_is_full_polygon_predicate(std::sync::Arc::new(
            move |_g: &Graph| -> Result<bool, S2Error> {
                Ok(is_full_polygon_precomputed(
                    op_type,
                    a_face_mask,
                    b_face_mask,
                    a_area,
                    b_area,
                    edge_snap_radius_radians,
                ))
            },
        ));

        self.builder = Some(builder);

        // Pre-compute index crossings (needs builder for add_intersection).
        self.init_index_crossings(RegionId::A, [a, b]);

        // Build a map of intersection points per (region_id, shape_id, edge_id).
        // This is used by the crossing processor to register intersection points
        // with the builder when edges are added.
        let mut crossing_point_map: std::collections::HashMap<
            (RegionId, ShapeId, i32),
            Vec<crate::s2::Point>,
        > = std::collections::HashMap::new();
        for record in &self.interior_crossings {
            crossing_point_map
                .entry((RegionId::A, record.a.shape_id, record.a.edge_id))
                .or_default()
                .push(record.point);
            crossing_point_map
                .entry((RegionId::B, record.b.shape_id, record.b.edge_id))
                .or_default()
                .push(record.point);
        }
        self.crossing_point_map = Some(crossing_point_map);

        // Process crossings and add edges to builder.
        self.build_op_type(op_type, a, b);

        // Copy dimensions and crossings to shared state.
        *shared_dims.borrow_mut() = std::mem::take(&mut self.input_dimensions);
        *shared_crossings.borrow_mut() = std::mem::take(&mut self.input_crossings);

        // Build (this will call SharedEdgeClippingLayer::build).
        self.index_crossings.clear();
        if let Some(mut builder) = self.builder.take() {
            match builder.build() {
                Ok(mut built_layers) => {
                    // Extract the user's layers from inside SharedEdgeClippingLayer.
                    if let Some(wrapper) = built_layers.pop()
                        && let Ok(clipping) =
                            wrapper.into_any().downcast::<SharedEdgeClippingLayer>()
                    {
                        self.op.layers = clipping.layers;
                    }
                }
                Err(e) => *error = e,
            }
        }
    }

    /// Returns true if result has no edges (for boolean output mode).
    fn build_op_type(&mut self, op_type: OpType, a: &ShapeIndex, b: &ShapeIndex) -> bool {
        let regions = [a, b];
        let is_bool = self.is_boolean_output();

        // Take fields out of self to give to CrossingProcessor,
        // so we can still use &mut self for add_boundary_pair.
        let mut builder_opt = self.builder.take();
        let mut input_dims = std::mem::take(&mut self.input_dimensions);
        let mut input_crossings_vec = std::mem::take(&mut self.input_crossings);
        let crossing_point_map = self.crossing_point_map.take();

        let mut cp = CrossingProcessor::new(
            self.op.options.polygon_model,
            self.op.options.polyline_model,
            self.op.options.polyline_loops_have_boundaries,
            builder_opt.as_mut(),
            if is_bool { None } else { Some(&mut input_dims) },
            if is_bool {
                None
            } else {
                Some(&mut input_crossings_vec)
            },
            crossing_point_map,
        );

        let result = match op_type {
            OpType::Union => self.add_boundary_pair(true, true, true, &mut cp, regions),
            OpType::Intersection => self.add_boundary_pair(false, false, false, &mut cp, regions),
            OpType::Difference => self.add_boundary_pair(false, true, false, &mut cp, regions),
            OpType::SymmetricDifference => {
                self.add_boundary_pair(false, true, false, &mut cp, regions)
                    && self.add_boundary_pair(true, false, false, &mut cp, regions)
            }
        };

        // Put fields back.
        drop(cp);
        self.builder = builder_opt;
        self.input_dimensions = input_dims;
        self.input_crossings = input_crossings_vec;

        result
    }

    fn add_boundary_pair(
        &mut self,
        invert_a: bool,
        invert_b: bool,
        invert_result: bool,
        cp: &mut CrossingProcessor<'_>,
        regions: [&ShapeIndex; 2],
    ) -> bool {
        // Optimization for DIFFERENCE/SYMMETRIC_DIFFERENCE.
        let op_type = self.op.op_type;
        if (op_type == OpType::Difference || op_type == OpType::SymmetricDifference)
            && self.are_regions_identical(regions)
        {
            return true;
        }

        let mut a_starts = Vec::new();
        let mut b_starts = Vec::new();

        if !self.get_chain_starts(
            RegionId::A,
            invert_a,
            invert_b,
            invert_result,
            cp,
            &mut a_starts,
            regions,
        ) {
            return false;
        }
        if !self.get_chain_starts(
            RegionId::B,
            invert_b,
            invert_a,
            invert_result,
            cp,
            &mut b_starts,
            regions,
        ) {
            return false;
        }
        if !self.add_boundary(
            RegionId::A,
            invert_a,
            invert_b,
            invert_result,
            &a_starts,
            cp,
            regions,
        ) {
            return false;
        }
        if !self.add_boundary(
            RegionId::B,
            invert_b,
            invert_a,
            invert_result,
            &b_starts,
            cp,
            regions,
        ) {
            return false;
        }
        if !self.is_boolean_output() {
            cp.done_boundary_pair();
        }
        true
    }

    fn add_boundary(
        &mut self,
        a_region_id: RegionId,
        invert_a: bool,
        invert_b: bool,
        invert_result: bool,
        chain_starts: &[ShapeEdgeId],
        cp: &mut CrossingProcessor<'_>,
        regions: [&ShapeIndex; 2],
    ) -> bool {
        let a_index = regions[a_region_id.as_usize()];
        let b_index = regions[a_region_id.other().as_usize()];

        if !self.init_index_crossings(a_region_id, regions) {
            return false;
        }

        cp.start_boundary(a_region_id, invert_a, invert_b, invert_result);

        let mut chain_start_idx = 0;
        let mut next_crossing = CrossingIterator::new(b_index, &self.index_crossings, true);

        let chain_start_id = if chain_start_idx < chain_starts.len() {
            chain_starts[chain_start_idx]
        } else {
            SENTINEL
        };
        let mut next_id = min(chain_start_id, next_crossing.a_id());

        while next_id != SENTINEL {
            let a_shape_id = next_id.shape_id;
            let Some(a_shape) = a_index.shape(a_shape_id) else {
                break;
            };
            cp.start_shape(a_shape.dimension());

            while next_id.shape_id == a_shape_id {
                let edge_id = next_id.edge_id;
                let chain_pos = a_shape.chain_position(edge_id as usize);
                let chain_id = chain_pos.chain_id;
                let chain = a_shape.chain(chain_id);

                let current_chain_start = if chain_start_idx < chain_starts.len() {
                    chain_starts[chain_start_idx]
                } else {
                    SENTINEL
                };
                let start_inside = next_id == current_chain_start;
                if start_inside {
                    chain_start_idx += 1;
                }

                cp.start_chain(chain_id, chain.start, chain.length, start_inside);
                let chain_limit = (chain.start + chain.length) as i32;
                let mut eid = edge_id;

                while eid < chain_limit {
                    let a_id = ShapeEdgeId::new(a_shape_id, eid);
                    let a_edge = a_shape.chain_edge(chain_id, (eid - chain.start as i32) as usize);
                    debug_assert!(cp.inside || next_crossing.a_id() == a_id);
                    if !cp.process_edge(a_id, a_edge, a_shape, &mut next_crossing) {
                        return false;
                    }
                    if cp.inside {
                        eid += 1;
                    } else if next_crossing.a_id().shape_id == a_shape_id
                        && next_crossing.a_id().edge_id < chain_limit
                    {
                        eid = next_crossing.a_id().edge_id;
                    } else {
                        break;
                    }
                }

                let current_chain_start = if chain_start_idx < chain_starts.len() {
                    chain_starts[chain_start_idx]
                } else {
                    SENTINEL
                };
                next_id = min(current_chain_start, next_crossing.a_id());
            }
        }
        true
    }

    fn get_chain_starts(
        &mut self,
        a_region_id: RegionId,
        invert_a: bool,
        invert_b: bool,
        invert_result: bool,
        cp: &mut CrossingProcessor<'_>,
        chain_starts: &mut Vec<ShapeEdgeId>,
        regions: [&ShapeIndex; 2],
    ) -> bool {
        let a_index = regions[a_region_id.as_usize()];
        let b_index = regions[a_region_id.other().as_usize()];

        if self.is_boolean_output() {
            cp.start_boundary(a_region_id, invert_a, invert_b, invert_result);
        }

        let b_has_interior = has_interior(b_index);
        if b_has_interior || invert_b || self.is_boolean_output() {
            let mut query = ContainsPointQuery::new(b_index, VertexModel::SemiOpen);
            let num_shape_ids = a_index.num_shape_ids();
            for shape_id in 0..num_shape_ids {
                let Some(a_shape) = a_index.shape(shape_id as i32) else {
                    continue;
                };
                if invert_a != invert_result && a_shape.dimension() < Dimension::Polygon {
                    continue;
                }
                if self.is_boolean_output() {
                    cp.start_shape(a_shape.dimension());
                }
                let num_chains = a_shape.num_chains();
                for chain_id in 0..num_chains {
                    let chain = a_shape.chain(chain_id);
                    if chain.length == 0 {
                        continue;
                    }
                    let first_edge = a_shape.chain_edge(chain_id, 0);
                    let inside = (b_has_interior && query.contains(first_edge.v0)) != invert_b;
                    if inside {
                        chain_starts.push(ShapeEdgeId::new(shape_id as i32, chain.start as i32));
                    }
                    if self.is_boolean_output() {
                        cp.start_chain(chain_id, chain.start, chain.length, inside);
                        let a = ShapeEdge::new(
                            ShapeEdgeId::new(shape_id as i32, chain.start as i32),
                            first_edge,
                        );
                        if !self.process_incident_edges(&a, &mut query, cp, regions, a_region_id) {
                            return false;
                        }
                    }
                }
            }
        }
        chain_starts.push(SENTINEL);
        true
    }

    fn process_incident_edges(
        &mut self,
        a: &ShapeEdge,
        query: &mut ContainsPointQuery<'_>,
        cp: &mut CrossingProcessor<'_>,
        regions: [&ShapeIndex; 2],
        a_region_id: RegionId,
    ) -> bool {
        self.tmp_crossings.clear();
        let tmp = &mut self.tmp_crossings;
        let _ = query.visit_incident_edges(a.edge.v0, |b: &ShapeEdge| {
            let mut crossing = IndexCrossing::new(a.id, b.id);
            if edge_crossings::vertex_crossing(a.edge.v0, a.edge.v1, b.edge.v0, b.edge.v1) {
                crossing.is_vertex_crossing = true;
            }
            tmp.push(crossing);
            ControlFlow::Continue(())
        });

        if self.tmp_crossings.is_empty() {
            return !cp.inside;
        }

        if self.tmp_crossings.len() > 1 {
            self.tmp_crossings.sort_unstable();
            self.tmp_crossings.dedup();
        }
        self.tmp_crossings
            .push(IndexCrossing::new(SENTINEL, SENTINEL));

        let b_index = regions[a_region_id.other().as_usize()];
        let mut next_crossing = CrossingIterator::new(b_index, &self.tmp_crossings, false);
        let Some(a_shape) = regions[a_region_id.as_usize()].shape(a.id.shape_id) else {
            return false;
        };
        let a_edge = a_shape.chain_edge(
            a_shape.chain_position(a.id.edge_id as usize).chain_id,
            a_shape.chain_position(a.id.edge_id as usize).offset,
        );
        cp.process_edge(a.id, a_edge, a_shape, &mut next_crossing)
    }

    fn init_index_crossings(&mut self, region_id: RegionId, regions: [&ShapeIndex; 2]) -> bool {
        if Some(region_id) == self.index_crossings_first_region_id {
            return true;
        }
        if self.index_crossings_first_region_id.is_none() {
            let is_bool = self.is_boolean_output();
            let builder = &mut self.builder;
            let crossings = &mut self.index_crossings;
            let interior_records = &mut self.interior_crossings;

            let result = shape_util::visit_crossing_edge_pairs_ab(
                regions[0],
                regions[1],
                crate::s2::crossing_edge_query::CrossingType::All,
                &mut |a: &ShapeEdge, b: &ShapeEdge, is_interior: bool| {
                    if is_interior && is_bool {
                        return ControlFlow::Break(());
                    }
                    let mut crossing = IndexCrossing::new(a.id, b.id);
                    if is_interior {
                        crossing.is_interior_crossing = true;
                        if predicates::sign(a.edge.v0, a.edge.v1, b.edge.v0) {
                            crossing.left_to_right = true;
                        }
                        if let Some(builder) = builder.as_mut() {
                            let intersection_pt = edge_crossings::intersection(
                                a.edge.v0, a.edge.v1, b.edge.v0, b.edge.v1,
                            );
                            builder.add_intersection(intersection_pt);
                            interior_records.push(InteriorCrossingRecord {
                                point: intersection_pt,
                                a: a.id,
                                b: b.id,
                            });
                        }
                    } else if edge_crossings::vertex_crossing(
                        a.edge.v0, a.edge.v1, b.edge.v0, b.edge.v1,
                    ) {
                        crossing.is_vertex_crossing = true;
                    }
                    crossings.push(crossing);
                    ControlFlow::Continue(())
                },
            );
            if result.is_break() {
                return false;
            }

            if self.index_crossings.len() > 1 {
                self.index_crossings.sort_unstable();
                self.index_crossings.dedup();
            }
            self.index_crossings
                .push(IndexCrossing::new(SENTINEL, SENTINEL));
            self.index_crossings_first_region_id = Some(RegionId::A);
        }

        if Some(region_id) != self.index_crossings_first_region_id {
            for crossing in &mut self.index_crossings {
                std::mem::swap(&mut crossing.a, &mut crossing.b);
                crossing.left_to_right ^= true;
                crossing.is_vertex_crossing ^= true;
            }
            self.index_crossings.sort_unstable();
            self.index_crossings_first_region_id = Some(region_id);
        }
        true
    }

    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn are_regions_identical(&self, regions: [&ShapeIndex; 2]) -> bool {
        let a = regions[0];
        let b = regions[1];
        if std::ptr::eq(a, b) {
            return true;
        }

        let num_shape_ids = a.num_shape_ids();
        if num_shape_ids != b.num_shape_ids() {
            return false;
        }
        for s in 0..num_shape_ids {
            let a_shape = a.shape(s as i32);
            let b_shape = b.shape(s as i32);
            match (a_shape, b_shape) {
                (Some(as_), Some(bs)) => {
                    if as_.dimension() != bs.dimension() {
                        return false;
                    }
                    if as_.num_chains() != bs.num_chains() {
                        return false;
                    }
                    if as_.num_edges() != bs.num_edges() {
                        return false;
                    }
                    if as_.dimension() == Dimension::Point {
                        // All chains are of length 1 for dimension-0 shapes.
                        debug_assert_eq!(as_.num_edges(), as_.num_chains());
                    }
                    for c in 0..as_.num_chains() {
                        let ac = as_.chain(c);
                        let bc = bs.chain(c);
                        if ac.length != bc.length {
                            return false;
                        }
                    }
                    // Check vertices.
                    for c in 0..as_.num_chains() {
                        let ac = as_.chain(c);
                        for i in 0..ac.length {
                            let ae = as_.chain_edge(c, i);
                            let be = bs.chain_edge(c, i);
                            if ae.v0 != be.v0 || ae.v1 != be.v1 {
                                return false;
                            }
                        }
                    }
                }
                (None, None) => {}
                _ => return false,
            }
        }
        true
    }
}

fn has_interior(index: &ShapeIndex) -> bool {
    for s in (0..index.num_shape_ids()).rev() {
        if let Some(shape) = index.shape(s as i32)
            && shape.dimension() == Dimension::Polygon
        {
            return true;
        }
    }
    false
}

/// Computes the area of all dimension-2 shapes in the index.
fn get_area(index: &ShapeIndex) -> f64 {
    let mut area = 0.0;
    for s in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(s as i32)
            && shape.dimension() == Dimension::Polygon
        {
            area += get_shape_area(shape);
        }
    }
    area
}

/// Computes the area of a dimension-2 shape by summing signed loop areas.
///
/// Handles the "full polygon" case: a shape with a 0-vertex chain whose
/// `reference_point` indicates containment represents the full sphere (area ≈ 4π).
fn get_shape_area(shape: &dyn Shape) -> f64 {
    let mut area = 0.0;
    for c in 0..shape.num_chains() {
        let chain = shape.chain(c);
        if chain.length == 0 {
            // By S2 convention, a loop with 0 vertices represents the full loop
            // (containing all points on the sphere). Its signed area is a tiny
            // negative value, matching C++ GetCurvature() returning -2*PI for
            // empty loops, which leads GetSignedArea to return -DBL_MIN.
            area -= f64::MIN_POSITIVE;
            continue;
        }
        if chain.length < 3 {
            continue;
        }
        // Get chain vertices.
        let mut vertices = Vec::with_capacity(chain.length);
        for i in 0..chain.length {
            vertices.push(shape.chain_edge(c, i).v0);
        }
        // Compute signed area using the girard formula.
        let mut loop_area = 0.0;
        for i in 1..vertices.len() - 1 {
            loop_area +=
                crate::s2::point_measures::signed_area(vertices[0], vertices[i], vertices[i + 1]);
        }
        area += loop_area;
    }
    // Normalize: negative total area means the polygon covers more than half
    // the sphere; add 4*PI to get the correct positive area.
    if area < 0.0 {
        area += 4.0 * PI;
    }
    area
}

/// Returns a bit mask indicating which of the 6 S2 cube faces intersect the index.
fn get_face_mask(index: &ShapeIndex) -> u8 {
    let mut mask = 0u8;
    let mut it = index.iter();
    while !it.done() {
        let face = it.cell_id().face();
        mask |= 1 << face.as_u8();
        it.seek(crate::s2::CellId::from_face(face.as_u8() + 1).range_min());
    }
    // A full polygon (0 edges, reference_point contained) covers all faces
    // but has no indexed cells. Detect this case explicitly.
    if mask != ALL_FACES_MASK {
        for s in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(s as i32)
                && shape.dimension() == Dimension::Polygon
                && shape.num_edges() == 0
                && shape.reference_point().contained
            {
                return ALL_FACES_MASK;
            }
        }
    }
    mask
}

const ALL_FACES_MASK: u8 = 0x3f;

/// Checks if the result is a full polygon, computing face masks and areas from indexes.
/// Uses 0 for `edge_snap_radius` (boolean output mode doesn't need ambiguity detection).
fn is_full_polygon_result_fn(op_type: OpType, a: &ShapeIndex, b: &ShapeIndex) -> bool {
    is_full_polygon_precomputed(
        op_type,
        get_face_mask(a),
        get_face_mask(b),
        get_area(a),
        get_area(b),
        0.0,
    )
}

/// Checks if the result is a full polygon using pre-computed face masks and areas.
fn is_full_polygon_precomputed(
    op_type: OpType,
    a_mask: u8,
    b_mask: u8,
    a_area: f64,
    b_area: f64,
    edge_snap_radius_radians: f64,
) -> bool {
    match op_type {
        OpType::Union => {
            if (a_mask | b_mask) != ALL_FACES_MASK {
                return false;
            }
            let min_area = a_area.max(b_area);
            let max_area = (4.0 * PI).min(a_area + b_area);
            min_area > 4.0 * PI - max_area
        }
        OpType::Intersection => {
            if (a_mask & b_mask) != ALL_FACES_MASK {
                return false;
            }
            let min_area = (a_area + b_area - 4.0 * PI).max(0.0);
            let max_area = a_area.min(b_area);
            min_area > 4.0 * PI - max_area
        }
        OpType::Difference => {
            if a_mask != ALL_FACES_MASK {
                return false;
            }
            let min_area = (a_area - b_area).max(0.0);
            let max_area = a_area.min(4.0 * PI - b_area);
            min_area > 4.0 * PI - max_area
        }
        OpType::SymmetricDifference => {
            if (a_mask | b_mask) != ALL_FACES_MASK {
                return false;
            }
            let min_area = (a_area - b_area).abs();
            let max_area = 4.0 * PI - (4.0 * PI - (a_area + b_area)).abs();

            // C++ ambiguity detection: when both polygons have area ~2π,
            // both empty and full results are equally plausible.
            let hemisphere_area_error = 2.0 * PI * edge_snap_radius_radians + 40.0 * f64::EPSILON;
            let error_sign = min_area - (4.0 * PI - max_area);
            if error_sign.abs() <= hemisphere_area_error {
                // Ambiguous case: if both inputs don't cover all faces,
                // the result is more likely full (disjoint enough).
                // Otherwise default to empty (nearly identical polygons).
                if (a_mask & b_mask) != ALL_FACES_MASK {
                    return true;
                }
                return false;
            }
            error_sign > 0.0
        }
    }
}

// ─── SharedEdgeClippingLayer ─────────────────────────────────────────────

use std::cell::RefCell;
use std::rc::Rc;

/// `EdgeClippingLayer` that uses shared (Rc<`RefCell`<>>) data for dimensions and crossings.
#[derive(Debug)]
struct SharedEdgeClippingLayer {
    layers: Vec<Box<dyn Layer>>,
    input_dimensions: Rc<RefCell<Vec<Dimension>>>,
    input_crossings: Rc<RefCell<InputEdgeCrossings>>,
}

impl Layer for SharedEdgeClippingLayer {
    fn graph_options(&self) -> crate::s2::builder::graph::GraphOptions {
        use crate::s2::builder::graph::{
            DegenerateEdges, DuplicateEdges, EdgeType, GraphOptions, SiblingPairs,
        };
        GraphOptions {
            edge_type: EdgeType::Directed,
            degenerate_edges: DegenerateEdges::Keep,
            duplicate_edges: DuplicateEdges::Keep,
            sibling_pairs: SiblingPairs::Keep,
            allow_vertex_filtering: false,
        }
    }

    fn build(&mut self, g: &Graph, error: &mut S2Error) {
        use crate::s2::builder::InputEdgeIdSetId;

        let input_dimensions = self.input_dimensions.borrow();
        let input_crossings = self.input_crossings.borrow();

        let mut new_edges: Vec<crate::s2::builder::graph::Edge> = Vec::new();
        let mut new_input_edge_ids: Vec<InputEdgeIdSetId> = Vec::new();

        {
            let mut clipper = graph_edge_clipper::GraphEdgeClipper::new(
                g,
                &input_dimensions,
                &input_crossings,
                &mut new_edges,
                &mut new_input_edge_ids,
            );
            clipper.run();
        }

        let mut new_input_edge_id_set_lexicon = g.input_edge_id_set_lexicon().clone();

        if self.layers.len() == 1 {
            let layer_options = self.layers[0].graph_options();
            let new_graph = g.make_subgraph(
                layer_options,
                &mut new_edges,
                &mut new_input_edge_ids,
                &mut new_input_edge_id_set_lexicon,
                g.is_full_polygon_predicate_clone(),
                error,
            );
            self.layers[0].build(&new_graph, error);
        } else if self.layers.len() == 3 {
            let mut layer_edges: [Vec<crate::s2::builder::graph::Edge>; 3] =
                [Vec::new(), Vec::new(), Vec::new()];
            let mut layer_input_edge_ids: [Vec<InputEdgeIdSetId>; 3] =
                [Vec::new(), Vec::new(), Vec::new()];

            for i in 0..new_edges.len() {
                let d = input_dimensions[new_input_edge_ids[i] as usize].as_usize();
                layer_edges[d].push(new_edges[i]);
                layer_input_edge_ids[d].push(new_input_edge_ids[i]);
            }

            let mut layer_graphs = Vec::with_capacity(3);
            for d in 0..3 {
                let layer_options = self.layers[d].graph_options();
                layer_graphs.push(g.make_subgraph(
                    layer_options,
                    &mut layer_edges[d],
                    &mut layer_input_edge_ids[d],
                    &mut new_input_edge_id_set_lexicon,
                    g.is_full_polygon_predicate_clone(),
                    error,
                ));
            }
            for (d, lg) in layer_graphs.iter().enumerate() {
                if error.code == S2ErrorCode::Ok {
                    self.layers[d].build(lg, error);
                }
            }
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
#[expect(
    clippy::field_reassign_with_default,
    reason = "clearer than a single struct literal with many fields"
)]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::builder::S2Error;
    use crate::s2::builder::graph::{
        DegenerateEdges, DuplicateEdges, EdgeId, EdgeType, Graph, GraphOptions, SiblingPairs,
    };
    use crate::s2::builder::layer::Layer;
    use crate::s2::builder::snap::IntLatLngSnapFunction;
    use crate::s2::shape::Shape;
    use crate::s2::text_format;
    use std::cell::RefCell;
    use std::rc::Rc;

    // ─── Test infrastructure ─────────────────────────────────────────

    /// A test Layer that simply collects all edges from the graph.
    #[derive(Debug)]
    struct EdgeCollectorLayer {
        edges: Rc<RefCell<Vec<(Point, Point)>>>,
        options: GraphOptions,
    }

    impl EdgeCollectorLayer {
        fn new(edges: Rc<RefCell<Vec<(Point, Point)>>>, options: GraphOptions) -> Self {
            Self { edges, options }
        }
    }

    impl Layer for EdgeCollectorLayer {
        fn graph_options(&self) -> GraphOptions {
            self.options.clone()
        }

        fn build(&mut self, g: &Graph, _error: &mut S2Error) {
            let mut edges = self.edges.borrow_mut();
            for eid in (0..g.num_edges().0).map(EdgeId) {
                let (v0_id, v1_id) = g.edge(eid);
                edges.push((g.vertex(v0_id), g.vertex(v1_id)));
            }
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    /// Extracts edges from a `ShapeIndex` grouped by dimension.
    fn expected_edges_by_dimension(index: &ShapeIndex) -> [Vec<(Point, Point)>; 3] {
        let mut result: [Vec<(Point, Point)>; 3] = [vec![], vec![], vec![]];
        for sid in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(sid as i32) {
                let dim = shape.dimension().as_usize();
                for eid in 0..shape.num_edges() {
                    let edge = shape.edge(eid);
                    result[dim].push((edge.v0, edge.v1));
                }
            }
        }
        result
    }

    /// Canonical ordering for points (total order on f64 coordinates).
    fn point_key(p: &Point) -> [u64; 3] {
        [p.0.x.to_bits(), p.0.y.to_bits(), p.0.z.to_bits()]
    }

    /// Sort edges and compare.
    fn sorted_edges(edges: &[(Point, Point)]) -> Vec<[u64; 6]> {
        let mut keys: Vec<[u64; 6]> = edges
            .iter()
            .map(|(a, b)| {
                let ak = point_key(a);
                let bk = point_key(b);
                [ak[0], ak[1], ak[2], bk[0], bk[1], bk[2]]
            })
            .collect();
        keys.sort_unstable();
        keys
    }

    /// The main test helper: runs a boolean operation and compares the result
    /// against an expected `ShapeIndex` (given as a string).
    ///
    /// Corresponds to C++ `ExpectResult`.
    fn expect_result(
        op_type: OpType,
        options: Options,
        a_str: &str,
        b_str: &str,
        expected_str: &str,
    ) {
        let mut a = text_format::make_index(a_str);
        let mut b = text_format::make_index(b_str);
        let expected = text_format::make_index(expected_str);

        // Create 3 edge collector layers (one per dimension).
        let dim_edges: Vec<Rc<RefCell<Vec<(Point, Point)>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(Vec::new()))).collect();

        let layers: Vec<Box<dyn Layer>> = (0..3_usize)
            .map(|dim| -> Box<dyn Layer> {
                let options = GraphOptions {
                    edge_type: EdgeType::Directed,
                    degenerate_edges: if dim == 2 {
                        DegenerateEdges::DiscardExcess
                    } else {
                        DegenerateEdges::Keep
                    },
                    duplicate_edges: DuplicateEdges::Keep,
                    sibling_pairs: if dim == 2 {
                        SiblingPairs::DiscardExcess
                    } else {
                        SiblingPairs::Keep
                    },
                    allow_vertex_filtering: false,
                };
                Box::new(EdgeCollectorLayer::new(Rc::clone(&dim_edges[dim]), options))
            })
            .collect();

        // Clone options for the is_empty check later.
        let options2 = Options {
            snap_function: options.snap_function.clone_snap(),
            polygon_model: options.polygon_model,
            polyline_model: options.polyline_model,
            polyline_loops_have_boundaries: options.polyline_loops_have_boundaries,
            split_all_crossing_polyline_edges: options.split_all_crossing_polyline_edges,
            memory_tracker: options.memory_tracker.clone(),
        };

        let mut op = S2BooleanOperation::multi(op_type, layers, options);
        let result = op.build(&mut a, &mut b);
        assert!(
            result.is_ok(),
            "{op_type:?} failed: {:?}\n  a = {a_str}\n  b = {b_str}\n  expected = {expected_str}",
            result.err()
        );

        // Compare edges by dimension.
        let expected_dims = expected_edges_by_dimension(&expected);
        for dim in 0..3 {
            let actual = dim_edges[dim].borrow();
            let actual_sorted = sorted_edges(&actual);
            let expected_sorted = sorted_edges(&expected_dims[dim]);
            assert_eq!(
                actual_sorted,
                expected_sorted,
                "{op_type:?} dim {dim} mismatch:\n  a = {a_str}\n  b = {b_str}\n  expected = {expected_str}\n  actual edges: {}\n  expected edges: {}",
                actual.len(),
                expected_dims[dim].len()
            );
        }

        // Also check IsEmpty predicate.
        let expected_empty = expected.num_shape_ids() == 0
            || (0..expected.num_shape_ids()).all(|sid| {
                expected
                    .shape(sid as i32)
                    .is_none_or(|s| s.num_edges() == 0)
            });
        assert_eq!(
            expected_empty,
            S2BooleanOperation::is_empty(op_type, &mut a, &mut b, options2),
            "{op_type:?} IsEmpty mismatch:\n  a = {a_str}\n  b = {b_str}"
        );
    }

    /// Helper: create Options with IntLatLngSnapFunction(exp).
    fn round_to_e(exp: i32) -> Options {
        Options {
            snap_function: Box::new(IntLatLngSnapFunction::new(exp)),
            ..Options::default()
        }
    }

    // ─── Predicate tests ─────────────────────────────────────────────

    #[test]
    fn test_basic_intersection() {
        let mut a_index = ShapeIndex::new();
        a_index.add(Box::new(text_format::make_lax_polygon("0:0, 0:10, 10:5")));
        a_index.build();

        let mut b_index = ShapeIndex::new();
        b_index.add(Box::new(text_format::make_lax_polygon("0:5, 0:15, 10:10")));
        b_index.build();

        assert!(
            S2BooleanOperation::intersects(&mut a_index, &mut b_index, Options::default()),
            "Overlapping triangles should intersect"
        );
    }

    #[test]
    fn test_disjoint_not_intersecting() {
        let mut a = text_format::make_index("# # 0:0, 0:1, 1:0");
        let mut b = text_format::make_index("# # 10:10, 10:11, 11:10");
        assert!(
            !S2BooleanOperation::intersects(&mut a, &mut b, Options::default()),
            "Disjoint triangles should not intersect"
        );
    }

    #[test]
    fn test_contains() {
        let mut big = text_format::make_index("# # 0:0, 0:20, 20:10");
        let mut small = text_format::make_index("# # 2:5, 2:10, 5:7");

        assert!(S2BooleanOperation::contains(
            &mut big,
            &mut small,
            Options::default()
        ));
        assert!(!S2BooleanOperation::contains(
            &mut small,
            &mut big,
            Options::default()
        ));
    }

    #[test]
    fn test_difference_identical_is_empty() {
        let mut a = text_format::make_index("# # 0:0, 0:10, 10:5");
        let mut b = text_format::make_index("# # 0:0, 0:10, 10:5");
        assert!(S2BooleanOperation::is_empty(
            OpType::Difference,
            &mut a,
            &mut b,
            Options::default()
        ));
    }

    #[test]
    fn test_symmetric_difference_identical_is_empty() {
        let mut a = text_format::make_index("# # 0:0, 0:10, 10:5");
        let mut b = text_format::make_index("# # 0:0, 0:10, 10:5");
        assert!(S2BooleanOperation::equals(
            &mut a,
            &mut b,
            Options::default()
        ));
    }

    // ─── Equals tests ────────────────────────────────────────────────

    fn test_equal(a_str: &str, b_str: &str) -> bool {
        let mut a = text_format::make_index(a_str);
        let mut b = text_format::make_index(b_str);
        S2BooleanOperation::equals(&mut a, &mut b, Options::default())
    }

    #[test]
    fn test_equals() {
        assert!(test_equal("# #", "# #"));
        assert!(test_equal("# # full", "# # full"));

        assert!(!test_equal("# #", "# # full"));
        assert!(!test_equal("0:0 # #", "# #"));
        assert!(!test_equal("0:0 # #", "# # full"));
        assert!(!test_equal("# 0:0, 1:1 #", "# #"));
        assert!(!test_equal("# 0:0, 1:1 #", "# # full"));
        assert!(!test_equal("# # 0:0, 0:1, 1:0", "# #"));
        assert!(!test_equal("# # 0:0, 0:1, 1:0", "# # full"));
    }

    #[test]
    fn test_contains_empty_and_full() {
        // empty contains empty
        assert!(S2BooleanOperation::contains(
            &mut text_format::make_index("# #"),
            &mut text_format::make_index("# #"),
            Options::default()
        ));
        // empty does not contain full
        assert!(!S2BooleanOperation::contains(
            &mut text_format::make_index("# #"),
            &mut text_format::make_index("# # full"),
            Options::default()
        ));
        // full contains empty
        assert!(S2BooleanOperation::contains(
            &mut text_format::make_index("# # full"),
            &mut text_format::make_index("# #"),
            Options::default()
        ));
        // full contains full
        assert!(S2BooleanOperation::contains(
            &mut text_format::make_index("# # full"),
            &mut text_format::make_index("# # full"),
            Options::default()
        ));
    }

    #[test]
    fn test_intersects_empty_and_full() {
        assert!(!S2BooleanOperation::intersects(
            &mut text_format::make_index("# #"),
            &mut text_format::make_index("# #"),
            Options::default()
        ));
        assert!(!S2BooleanOperation::intersects(
            &mut text_format::make_index("# #"),
            &mut text_format::make_index("# # full"),
            Options::default()
        ));
        assert!(!S2BooleanOperation::intersects(
            &mut text_format::make_index("# # full"),
            &mut text_format::make_index("# #"),
            Options::default()
        ));
        assert!(S2BooleanOperation::intersects(
            &mut text_format::make_index("# # full"),
            &mut text_format::make_index("# # full"),
            Options::default()
        ));
    }

    // ─── SourceId tests ──────────────────────────────────────────────

    #[test]
    fn test_source_id_accessors() {
        let id = SourceId::new(RegionId::B, 2, 3);
        assert_eq!(id.region_id(), RegionId::B);
        assert_eq!(id.shape_id(), 2);
        assert_eq!(id.edge_id(), 3);
    }

    #[test]
    fn test_source_id_equality() {
        assert_eq!(
            SourceId::new(RegionId::B, 2, 3),
            SourceId::new(RegionId::B, 2, 3)
        );
        assert_ne!(
            SourceId::new(RegionId::B, 2, 3),
            SourceId::new(RegionId::A, 2, 3)
        );
        assert_ne!(
            SourceId::new(RegionId::B, 2, 3),
            SourceId::new(RegionId::B, 0, 3)
        );
        assert_ne!(
            SourceId::new(RegionId::B, 2, 3),
            SourceId::new(RegionId::B, 2, 0)
        );
    }

    #[test]
    fn test_source_id_ordering() {
        assert!((SourceId::new(RegionId::B, 2, 3) >= SourceId::new(RegionId::B, 2, 3)));
        assert!(SourceId::new(RegionId::A, 2, 3) < SourceId::new(RegionId::B, 2, 3));
        assert!(SourceId::new(RegionId::B, 2, 3) < SourceId::new(RegionId::B, 20, 3));
        assert!(SourceId::new(RegionId::B, 2, 3) < SourceId::new(RegionId::B, 2, 30));
    }

    // ─── Point/polygon constructive tests ────────────────────────────

    #[test]
    fn test_point_polygon_interior() {
        let options = Options::default(); // PolygonModel is irrelevant.
        let a = "1:1 | 4:4 # #";
        let b = "# # 0:0, 0:3, 3:0";
        expect_result(
            OpType::Union,
            Options {
                ..options_clone(&options)
            },
            a,
            b,
            "4:4 # # 0:0, 0:3, 3:0",
        );
        expect_result(
            OpType::Intersection,
            Options {
                ..options_clone(&options)
            },
            a,
            b,
            "1:1 # #",
        );
        expect_result(
            OpType::Difference,
            Options {
                ..options_clone(&options)
            },
            a,
            b,
            "4:4 # #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options_clone(&options),
            a,
            b,
            "4:4 # # 0:0, 0:3, 3:0",
        );
    }

    // ─── Polygon/polygon constructive tests ──────────────────────────

    #[test]
    fn test_polygon_vertex_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5";
        let b = "# # 0:0, 5:3, 5:2";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
        expect_result(OpType::Intersection, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
    }

    #[test]
    fn test_polygon_vertex_semi_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::SemiOpen;
        let a = "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5";
        let b = "# # 0:0, 5:3, 5:2";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
        expect_result(OpType::Intersection, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
    }

    #[test]
    fn test_polygon_vertex_closed_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Closed;
        let a = "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5";
        let b = "# # 0:0, 5:3, 5:2";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 0:0",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            b,
            a,
            "# # 0:0, 5:3, 5:2",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:5, 1:5, 0:0, 2:5, 3:5, 0:0, 5:3, 5:2",
        );
    }

    #[test]
    fn test_polygon_edge_polygon_edge_crossing() {
        // Two polygons whose edges cross at points interior to both edges.
        let options = round_to_e(2);
        let a = "# # 0:0, 0:2, 2:2, 2:0";
        let b = "# # 1:1, 1:3, 3:3, 3:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:3, 3:3, 3:1, 2:1, 2:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 1:1, 1:2, 2:2, 2:1",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:1, 2:1, 2:0",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:1, 2:1, 2:0; \
                       1:2, 1:3, 3:3, 3:1, 2:1, 2:2",
        );
    }

    #[test]
    fn test_polygon_edge_open_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "# # 0:0, 0:4, 2:4, 2:0";
        let b = "# # 0:0, 1:1, 2:0; 0:4, 1:5, 2:4";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0; 0:4, 1:5, 2:4",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 1:1, 2:0",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1; 0:4, 1:5, 2:4",
        );
    }

    #[test]
    fn test_polygon_edge_semi_open_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::SemiOpen;
        let a = "# # 0:0, 0:4, 2:4, 2:0";
        let b = "# # 0:0, 1:1, 2:0; 0:4, 1:5, 2:4";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 1:5, 2:4, 2:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 1:1, 2:0",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1; 0:4, 1:5, 2:4",
        );
    }

    #[test]
    fn test_polygon_edge_closed_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Closed;
        let a = "# # 0:0, 0:4, 2:4, 2:0";
        let b = "# # 0:0, 1:1, 2:0; 0:4, 1:5, 2:4";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 1:5, 2:4, 2:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 1:1, 2:0; 0:4, 2:4",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:4, 2:4, 2:0, 1:1; 0:4, 1:5, 2:4",
        );
    }

    #[test]
    fn test_polygon_polygon_interior() {
        // One loop in the interior of another polygon and one loop in the exterior.
        let options = Options::default(); // PolygonModel is irrelevant.
        let a = "# # 0:0, 0:4, 4:4, 4:0";
        let b = "# # 1:1, 1:2, 2:2, 2:1; 5:5, 5:6, 6:6, 6:5";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 4:4, 4:0; 5:5, 5:6, 6:6, 6:5",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 1:1, 1:2, 2:2, 2:1",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:4, 4:4, 4:0; 2:1, 2:2, 1:2, 1:1",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:4, 4:4, 4:0; 2:1, 2:2, 1:2, 1:1; \
                       5:5, 5:6, 6:6, 6:5",
        );
    }

    #[test]
    fn test_polygon_edges_degenerate_after_snapping() {
        let options = round_to_e(0);
        let a = "# # 0:-1, 0:1, 0.1:1, 0.1:-1";
        let b = "# # -1:0.1, 1:0.1, 1:0, -1:0";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:-1, 0:0, 0:1, 0:0 | \
                       -1:0, 0:0, 1:0, 0:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 0:0",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:-1, 0:0, 0:1, 0:0",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:-1, 0:0, 0:1, 0:0 | \
                       -1:0, 0:0, 1:0, 0:0",
        );
    }

    // ─── Complex polygon tests ───────────────────────────────────────

    #[test]
    fn test_three_overlapping_bars() {
        let options = round_to_e(2);
        let a = "# # 0:0, 0:2, 3:2, 3:0; 0:3, 0:5, 3:5, 3:3";
        let b = "# # 1:1, 1:4, 2:4, 2:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:3, 0:3, 0:5, 3:5, 3:3, 2:3, 2:2, 3:2, 3:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 1:1, 1:2, 2:2, 2:1; 1:3, 1:4, 2:4, 2:3",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:1, 2:1, 2:2, 3:2, 3:0; \
                       0:3, 0:5, 3:5, 3:3, 2:3, 2:4, 1:4, 1:3",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:2, 1:2, 1:1, 2:1, 2:2, 3:2, 3:0; \
                       0:3, 0:5, 3:5, 3:3, 2:3, 2:4, 1:4, 1:3; \
                       1:2, 1:3, 2:3, 2:2",
        );
    }

    #[test]
    fn test_four_overlapping_bars() {
        let options = round_to_e(2);
        let a = "# # 1:88, 1:93, 2:93, 2:88; -1:88, -1:93, 0:93, 0:88";
        let b = "# # -2:89, -2:90, 3:90, 3:89; -2:91, -2:92, 3:92, 3:91";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # -1:88, -1:89, -2:89, -2:90, -1:90, -1:91, -2:91, -2:92, -1:92, \
                       -1:93, 0:93, 0:92, 1:92, 1:93, 2:93, 2:92, 3:92, 3:91, 2:91, \
                       2:90, 3:90, 3:89, 2:89, 2:88, 1:88, 1:89, 0:89, 0:88; \
                       0:90, 1:90, 1:91, 0:91",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # 1:89, 1:90, 2:90, 2:89; 1:91, 1:92, 2:92, 2:91; \
                       -1:89, -1:90, 0:90, 0:89; -1:91, -1:92, 0:92, 0:91",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # 1:88, 1:89, 2:89, 2:88; 1:90, 1:91, 2:91, 2:90; \
                       1:92, 1:93, 2:93, 2:92; -1:88, -1:89, 0:89, 0:88; \
                       -1:90, -1:91, 0:91, 0:90; -1:92, -1:93, 0:93, 0:92",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 1:88, 1:89, 2:89, 2:88; -1:88, -1:89, 0:89, 0:88; \
                       1:90, 1:91, 2:91, 2:90; -1:90, -1:91, 0:91, 0:90; \
                       1:92, 1:93, 2:93, 2:92; -1:92, -1:93, 0:93, 0:92; \
                       -2:89, -2:90, -1:90, -1:89; -2:91, -2:92, -1:92, -1:91; \
                       0:89, 0:90, 1:90, 1:89; 0:91, 0:92, 1:92, 1:91; \
                       2:89, 2:90, 3:90, 3:89; 2:91, 2:92, 3:92, 3:91",
        );
    }

    #[test]
    fn test_overlapping_doughnuts() {
        let options = round_to_e(1);
        let a = "# # -1:-93, -1:-89, 3:-89, 3:-93; \
                      0:-92, 2:-92, 2:-90, 0:-90";
        let b = "# # -3:-91, -3:-87, 1:-87, 1:-91; \
                      -2:-90, 0:-90, 0:-88, -2:-88";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # -1:-93, -1:-91, -3:-91, -3:-87, 1:-87, 1:-89, 3:-89, 3:-93; \
                       0:-92, 2:-92, 2:-90, 1:-90, 1:-91, 0:-91; \
                       -2:-90, -1:-90, -1:-89, 0:-89, 0:-88, -2:-88",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# # -1:-91, -1:-90, 0:-90, 0:-91; \
                       0:-90, 0:-89, 1:-89, 1:-90",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# # -1:-93, -1:-91, 0:-91, 0:-92, 2:-92, \
                       2:-90, 1:-90, 1:-89, 3:-89, 3:-93; \
                       -1:-90, -1:-89, 0:-89, 0:-90",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # -1:-93, -1:-91, 0:-91, 0:-92, 2:-92, \
                       2:-90, 1:-90, 1:-89, 3:-89, 3:-93; \
                       -3:-91, -3:-87, 1:-87, 1:-89, 0:-89, 0:-88,-2:-88,-2:-90,-1:-90,-1:-91; \
                       -1:-90, -1:-89, 0:-89, 0:-90; \
                       1:-91, 0:-91, 0:-90, 1:-90",
        );
    }

    // ─── Polyline/polygon constructive tests ─────────────────────────

    #[test]
    fn test_polyline_edge_polygon_interior() {
        let options = Options::default(); // PolygonModel is irrelevant.
        let a = "# 1:1, 2:2 | 3:3, 3:3 | 6:6, 7:7 | 8:8, 8:8 # ";
        let b = "# # 0:0, 0:5, 5:5, 5:0";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 6:6, 7:7 | 8:8, 8:8 # 0:0, 0:5, 5:5, 5:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 2:2 | 3:3, 3:3 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 6:6, 7:7 | 8:8, 8:8 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 6:6, 7:7 | 8:8, 8:8 # 0:0, 0:5, 5:5, 5:0",
        );
    }

    #[test]
    fn test_polyline_entering_rectangle() {
        let options = round_to_e(1);
        let a = "# 0:0, 2:2 #";
        let b = "# # 1:1, 1:3, 3:3, 3:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:1 # 1:1, 1:3, 3:3, 3:1",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 2:2 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:1 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 1:1 # 1:1, 1:3, 3:3, 3:1",
        );
    }

    #[test]
    fn test_polyline_crossing_rectangle_twice() {
        let options = round_to_e(1);
        let a = "# 0:-5, 0:5, 5:0, -5:0 #";
        let b = "# # 1:1, 1:-1, -1:-1, -1:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:-5, 0:-1 | 0:1, 0:5, 5:0, 1:0 | -1:0, -5:0 \
                       # 1:1, 1:0, 1:-1, 0:-1, -1:-1, -1:0, -1:1, 0:1",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:-1, 0:1 | 1:0, -1:0 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:-5, 0:-1 | 0:1, 0:5, 5:0, 1:0 | -1:0, -5:0 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:-5, 0:-1 | 0:1, 0:5, 5:0, 1:0 | -1:0, -5:0 \
                       # 1:1, 1:0, 1:-1, 0:-1, -1:-1, -1:0, -1:1, 0:1",
        );
    }

    // ─── Polyline/polyline constructive tests ────────────────────────

    #[test]
    fn test_polyline_edge_polyline_edge_crossing() {
        let options = round_to_e(1);
        let a = "# 0:0, 2:2 #";
        let b = "# 2:0, 0:2 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:1, 2:2 | 2:0, 1:1, 0:2 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:1 | 1:1, 1:1 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:1, 2:2 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 1:1, 2:2 | 2:0, 1:1, 0:2 #",
        );
    }

    #[test]
    fn test_polyline_edge_polyline_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "# 0:0, 1:0, 2:0, 2:5 | 3:0, 3:0 | 6:0, 5:0, 4:0 #";
        let b = "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:0, 2:0, 2:5 | 0:0, 1:0, 2:0 | 3:0, 3:0 | 3:0, 3:0 \
                       | 6:0, 5:0, 4:0 | 4:0, 5:0 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 1:0, 2:0 | 0:0, 1:0, 2:0 | 3:0, 3:0 | 3:0, 3:0 \
                       | 5:0, 4:0 | 4:0, 5:0 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 2:0, 2:5 | 6:0, 5:0 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 2:0, 2:5 | 6:0, 5:0 #",
        );
    }

    // ─── OpType/Options tests ───────────────────────────────────────

    #[test]
    fn test_op_type_accessor() {
        let options = Options::default();
        let op = S2BooleanOperation::new_predicate(OpType::Union, options);
        assert_eq!(OpType::Union, op.op_type());
    }

    #[test]
    fn test_options_fields_copied() {
        let mut options = Options::default();
        options.polyline_loops_have_boundaries = false;
        let no_boundary_op = S2BooleanOperation::new_predicate(OpType::Union, options);
        assert!(!no_boundary_op.options().polyline_loops_have_boundaries);

        let mut options = Options::default();
        options.polyline_loops_have_boundaries = true;
        let boundary_op = S2BooleanOperation::new_predicate(OpType::Union, options);
        assert!(boundary_op.options().polyline_loops_have_boundaries);
    }

    // ─── Degenerate geometry tests ──────────────────────────────────

    #[test]
    fn test_degenerate_polylines() {
        // Verify that degenerate polylines are preserved under all boundary models.
        let a = "# 0:0, 0:0 #";
        let b = "# #";
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        expect_result(OpType::Union, options_clone(&options), a, b, a);
        options.polyline_model = PolylineModel::SemiOpen;
        expect_result(OpType::Union, options_clone(&options), a, b, a);
        options.polyline_model = PolylineModel::Closed;
        expect_result(OpType::Union, options, a, b, a);
    }

    #[test]
    fn test_degenerate_polygons() {
        // Verify that degenerate polygon features (single-vertex and sibling pair
        // shells and holes) are preserved under all boundary models.
        let a = "# # 0:0, 0:5, 5:5, 5:0; 1:1; 2:2, 3:3; 6:6; 7:7, 8:8";
        let b = "# #";
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        expect_result(OpType::Union, options_clone(&options), a, b, a);
        options.polygon_model = PolygonModel::SemiOpen;
        expect_result(OpType::Union, options_clone(&options), a, b, a);
        options.polygon_model = PolygonModel::Closed;
        expect_result(OpType::Union, options, a, b, a);
    }

    // ─── Point/point tests ──────────────────────────────────────────

    #[test]
    fn test_point_point() {
        let options = Options::default();
        let a = "0:0 | 1:0 # #";
        let b = "0:0 | 2:0 # #";
        // Note: results have duplicates (correct); clients can eliminate with GraphOptions.
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "0:0 | 0:0 | 1:0 | 2:0 # #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "0:0 | 0:0 # #",
        );
        expect_result(OpType::Difference, options_clone(&options), a, b, "1:0 # #");
        expect_result(OpType::SymmetricDifference, options, a, b, "1:0 | 2:0 # #");
    }

    // ─── Point/polyline tests ───────────────────────────────────────

    #[test]
    fn test_point_open_polyline() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        let a = "0:0 | 1:0 | 2:0 | 3:0 | 4:0 | 5:0 # #";
        let b = "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "0:0 | 2:0 | 3:0 | 4:0 \
             # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "1:0 | 5:0 # #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "0:0 | 2:0 | 3:0 | 4:0 # #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "0:0 | 2:0 | 3:0 | 4:0\
             # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
        );
    }

    #[test]
    fn test_point_open_polyline_loop_boundaries_false() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        options.polyline_loops_have_boundaries = false;
        let a = "0:0 | 1:0 | 2:0 | 3:0 | 4:0 | 5:0 # #";
        let b = "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "0:0 | 2:0 | 3:0 \
             # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "1:0 | 4:0 | 5:0 # #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "0:0 | 2:0 | 3:0 # #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "0:0 | 2:0 | 3:0 \
             # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
        );
    }

    #[test]
    fn test_point_semi_open_polyline() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::SemiOpen;
        for plhb in [false, true] {
            options.polyline_loops_have_boundaries = plhb;
            let a = "0:0 | 1:0 | 2:0 | 3:0 | 4:0 | 5:0 # #";
            let b = "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #";
            expect_result(
                OpType::Union,
                options_clone(&options),
                a,
                b,
                "2:0 | 3:0 # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
            );
            expect_result(
                OpType::Intersection,
                options_clone(&options),
                a,
                b,
                "0:0 | 1:0 | 4:0 | 5:0 # #",
            );
            expect_result(
                OpType::Difference,
                options_clone(&options),
                a,
                b,
                "2:0 | 3:0 # #",
            );
            expect_result(
                OpType::SymmetricDifference,
                options_clone(&options),
                a,
                b,
                "2:0 | 3:0 # 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
            );
        }
    }

    #[test]
    fn test_point_closed_polyline() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Closed;
        for plhb in [false, true] {
            options.polyline_loops_have_boundaries = plhb;
            let a = "0:0 | 1:0 | 2:0 | 3:0 | 4:0 | 5:0 # #";
            let b = "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #";
            expect_result(
                OpType::Union,
                options_clone(&options),
                a,
                b,
                "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
            );
            expect_result(
                OpType::Intersection,
                options_clone(&options),
                a,
                b,
                "0:0 | 1:0 | 2:0 | 3:0 | 4:0 | 5:0 # #",
            );
            expect_result(OpType::Difference, options_clone(&options), a, b, "# #");
            expect_result(
                OpType::SymmetricDifference,
                options_clone(&options),
                a,
                b,
                "# 0:0, 1:0, 2:0 | 3:0, 3:0 | 4:0, 5:0, 4:0 #",
            );
        }
    }

    // ─── Point/polygon vertex tests ─────────────────────────────────

    #[test]
    fn test_point_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "0:1 | 1:0 # #";
        let b = "# # 0:0, 0:1, 1:0";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "0:1 | 1:0 # # 0:0, 0:1, 1:0",
        );
        expect_result(OpType::Intersection, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "0:1 | 1:0 # #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "0:1 | 1:0 # # 0:0, 0:1, 1:0",
        );
    }

    #[test]
    fn test_point_semi_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::SemiOpen;
        let a = "0:1 | 1:0 # #";
        let b = "# # 0:0, 0:1, 1:0";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "1:0 # # 0:0, 0:1, 1:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "0:1 # #",
        );
        expect_result(OpType::Difference, options_clone(&options), a, b, "1:0 # #");
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "1:0 # # 0:0, 0:1, 1:0",
        );
    }

    #[test]
    fn test_point_closed_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Closed;
        let a = "0:1 | 1:0 # #";
        let b = "# # 0:0, 0:1, 1:0";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 0:0, 0:1, 1:0",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "0:1 | 1:0 # #",
        );
        expect_result(OpType::Difference, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 0:0, 0:1, 1:0",
        );
    }

    // ─── Polyline vertex / polyline vertex tests ────────────────────

    #[test]
    fn test_polyline_vertex_open_polyline_vertex() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        let a = "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #";
        let b = "# 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:1, 0:1 | 0:1, 0:1 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
    }

    #[test]
    fn test_polyline_vertex_open_polyline_vertex_loop_boundaries_false() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        options.polyline_loops_have_boundaries = false;
        let a = "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #";
        let b = "# 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:1, 0:1 | 0:1, 0:1 \
             | 0:3, 0:3 | 0:3, 0:3 | 0:4, 0:4 | 0:4, 0:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
    }

    #[test]
    fn test_polyline_vertex_semi_open_polyline_vertex() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::SemiOpen;
        for plhb in [false, true] {
            options.polyline_loops_have_boundaries = plhb;
            let a = "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #";
            let b = "# 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                     | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #";
            expect_result(
                OpType::Union,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
            );
            expect_result(
                OpType::Intersection,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:0 | 0:0, 0:0 | 0:1, 0:1 | 0:1, 0:1 \
                 | 0:3, 0:3 | 0:3, 0:3 | 0:4, 0:4 | 0:4, 0:4 #",
            );
            expect_result(
                OpType::Difference,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #",
            );
            expect_result(
                OpType::SymmetricDifference,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
            );
        }
    }

    #[test]
    fn test_polyline_vertex_closed_polyline_vertex() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Closed;
        let a = "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #";
        let b = "# 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
        // Since polyline_loops_have_boundaries == true (default), the polyline
        // "0:3, 0:4, 0:3" has three vertices. Therefore 0:3 is emitted twice for
        // that polyline, plus once for the other polyline, for a total of thrice.
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:0 | 0:0, 0:0 | 0:1, 0:1 | 0:1, 0:1 \
             | 0:2, 0:2 | 0:2, 0:2 \
             | 0:3, 0:3 | 0:3, 0:3 | 0:3, 0:3 \
             | 0:4, 0:4 | 0:4, 0:4 | 0:4, 0:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
    }

    #[test]
    fn test_polyline_vertex_closed_polyline_vertex_loop_boundaries_false() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Closed;
        options.polyline_loops_have_boundaries = false;
        let a = "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #";
        let b = "# 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
                 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
        // Since polyline_loops_have_boundaries == false, the polyline
        // "0:3, 0:4, 0:3" has two vertices. Therefore 0:3 is emitted once for
        // that polyline, plus once for the other polyline, for a total of twice.
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:0 | 0:0, 0:0 | 0:1, 0:1 | 0:1, 0:1 \
             | 0:2, 0:2 | 0:2, 0:2 \
             | 0:3, 0:3 | 0:3, 0:3 | 0:4, 0:4 | 0:4, 0:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:3, 0:4, 0:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:0, 0:1, 0:2 | 0:0, 1:0 | -1:1, 0:1, 1:1 | -1:2, 0:2 \
             | 0:3, 0:4, 0:3 | 1:3, 0:3, 1:3 | 0:4, 1:4, 0:4 #",
        );
    }

    // ─── Polyline vertex / polygon vertex tests ─────────────────────

    fn vertex_test_polygon_str() -> String {
        "0:0, 0:1, 0:2, 0:3, 0:4, 0:5, 5:5, 5:4, 5:3, 5:2, 5:1, 5:0".to_string()
    }

    #[test]
    fn test_semi_open_polygon_vertices_contained() {
        // Verify whether certain vertices of the test polygon are contained
        // under the semi-open boundary model.
        use crate::s2::region::Region;
        let polygon = text_format::make_polygon(&vertex_test_polygon_str());
        assert!(polygon.contains_point(&text_format::parse_point("0:1")));
        assert!(polygon.contains_point(&text_format::parse_point("0:2")));
        assert!(polygon.contains_point(&text_format::parse_point("0:3")));
        assert!(polygon.contains_point(&text_format::parse_point("0:4")));
        assert!(!polygon.contains_point(&text_format::parse_point("5:1")));
        assert!(!polygon.contains_point(&text_format::parse_point("5:2")));
        assert!(!polygon.contains_point(&text_format::parse_point("5:3")));
        assert!(!polygon.contains_point(&text_format::parse_point("5:4")));
    }

    #[test]
    fn test_polyline_vertex_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "# 1:1, 0:1 | 0:2, 1:2 | -1:3, 0:3 | 0:4, -1:4 \
                 | 6:1, 5:1 | 5:2, 6:2 | 4:3, 5:3 | 5:4, 4:4 #";
        let b = &format!("# # {}", vertex_test_polygon_str());
        let diff_result = "# 0:1, 0:1 | 0:2, 0:2 | -1:3, 0:3 | 0:4, -1:4\
             | 6:1, 5:1 | 5:2, 6:2 | 5:3, 5:3 | 5:4, 5:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 0:1 | 0:2, 1:2 | 4:3, 5:3 | 5:4, 4:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            diff_result,
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
    }

    #[test]
    fn test_polyline_vertex_semi_open_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::SemiOpen;
        let a = "# 1:1, 0:1 | 0:2, 1:2 | -1:3, 0:3 | 0:4, -1:4 \
                 | 6:1, 5:1 | 5:2, 6:2 | 4:3, 5:3 | 5:4, 4:4 #";
        let b = &format!("# # {}", vertex_test_polygon_str());
        let diff_result = "# -1:3, 0:3 | 0:4, -1:4 | 6:1, 5:1 | 5:2, 6:2 | 5:3, 5:3 | 5:4, 5:4 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 0:1 | 0:2, 1:2 | 0:3, 0:3 | 0:4, 0:4 \
             | 4:3, 5:3 | 5:4, 4:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            diff_result,
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
    }

    #[test]
    fn test_polyline_vertex_closed_polygon_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Closed;
        let a = "# 1:1, 0:1 | 0:2, 1:2 | -1:3, 0:3 | 0:4, -1:4 \
                 | 6:1, 5:1 | 5:2, 6:2 | 4:3, 5:3 | 5:4, 4:4 #";
        let b = &format!("# # {}", vertex_test_polygon_str());
        let diff_result = "# -1:3, 0:3 | 0:4, -1:4 | 6:1, 5:1 | 5:2, 6:2 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 0:1 | 0:2, 1:2 | 0:3, 0:3 | 0:4, 0:4\
             | 5:1, 5:1 | 5:2, 5:2 | 4:3, 5:3 | 5:4, 4:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            diff_result,
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            &format!(
                "{}{}",
                diff_result.replace(" #", ""),
                &format!(" # {}", vertex_test_polygon_str())
            ),
        );
    }

    // ─── Polyline loop / polyline edge tests ────────────────────────

    #[test]
    fn test_polyline_loop_multiple_open_polyline_edge() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Open;
        let a = "# 0:0, 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2, 2:2 #";
        let b = "# 0:0, 0:0 | 0:0, 0:1 | 2:2, 2:2 | 2:2, 3:2 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 1:0, 0:0 | 0:0, 0:0 | 0:0, 0:1 \
             | 2:2, 2:3, 3:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1 | 0:0, 0:1 | 2:2, 3:2 | 3:2, 2:2 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:1, 1:0, 0:0 | 0:0, 0:0 | 2:2, 2:3, 3:2 | 2:2, 2:2 #",
        );
    }

    #[test]
    fn test_polyline_loop_multiple_semi_open_polyline_edge() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::SemiOpen;
        let a = "# 0:0, 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2, 2:2 #";
        let b = "# 0:0, 0:0 | 0:0, 0:1 | 2:2, 2:2 | 2:2, 3:2 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 1:0, 0:0 | 0:0, 0:0 | 0:0, 0:1 \
             | 2:2, 2:3, 3:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:0 | 0:0, 0:1 | 0:0, 0:1 \
             | 2:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 | 3:2, 2:2 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
        );
    }

    #[test]
    fn test_polyline_loop_multiple_closed_polyline_edge() {
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Closed;
        let a = "# 0:0, 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2, 2:2 #";
        let b = "# 0:0, 0:0 | 0:0, 0:1 | 2:2, 2:2 | 2:2, 3:2 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:1, 1:0, 0:0 | 0:0, 0:0 | 0:0, 0:1 \
             | 2:2, 2:3, 3:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 0:0, 0:0 | 0:0, 0:0 | 0:0, 0:1 | 0:0, 0:1 \
             | 2:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 | 3:2, 2:2 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
        );
    }

    #[test]
    fn test_polyline_loop_multiple_polyline_edge_loop_boundaries_false() {
        // Like the tests above but with polyline_loops_have_boundaries = false.
        // The result does not depend on the polyline model.
        for polyline_model in [
            PolylineModel::Open,
            PolylineModel::SemiOpen,
            PolylineModel::Closed,
        ] {
            let mut options = Options::default();
            options.polyline_model = polyline_model;
            options.polyline_loops_have_boundaries = false;
            let a = "# 0:0, 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2, 2:2 #";
            let b = "# 0:0, 0:0 | 0:0, 0:1 | 2:2, 2:2 | 2:2, 3:2 #";
            expect_result(
                OpType::Union,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:1, 1:0, 0:0 | 0:0, 0:0 | 0:0, 0:1 \
                 | 2:2, 2:3, 3:2, 2:2 | 2:2, 2:2 | 2:2, 3:2 #",
            );
            expect_result(
                OpType::Intersection,
                options_clone(&options),
                a,
                b,
                "# 0:0, 0:0 | 0:0, 0:1 | 0:0, 0:1 \
                 | 2:2, 2:2 | 2:2, 3:2 | 3:2, 2:2 #",
            );
            expect_result(
                OpType::Difference,
                options_clone(&options),
                a,
                b,
                "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
            );
            expect_result(
                OpType::SymmetricDifference,
                options_clone(&options),
                a,
                b,
                "# 0:1, 1:0, 0:0 | 2:2, 2:3, 3:2 #",
            );
        }
    }

    // ─── Polyline edge / polygon edge overlap tests ─────────────────

    #[test]
    fn test_polyline_edge_open_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let a = "# 1:1, 1:3, 3:3 | 3:3, 1:3 # ";
        let b = "# # 1:1, 1:3, 3:3, 3:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:3, 3:3 | 3:3, 1:3 # 1:1, 1:3, 3:3, 3:1",
        );
        expect_result(OpType::Intersection, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:3, 3:3 | 3:3, 1:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 1:1, 1:3, 3:3 | 3:3, 1:3 # 1:1, 1:3, 3:3, 3:1",
        );
    }

    #[test]
    fn test_polyline_edge_semi_open_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::SemiOpen;
        let a = "# 1:1, 1:3, 3:3 | 3:3, 1:3 # ";
        let b = "# # 1:1, 1:3, 3:3, 3:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:1 | 3:3, 3:3 | 3:3, 1:3 # 1:1, 1:3, 3:3, 3:1",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:3, 1:3 | 1:1, 1:3, 3:3 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:1 | 3:3, 3:3 | 3:3, 1:3 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# 1:1, 1:1 | 3:3, 3:3 | 3:3, 1:3 # 1:1, 1:3, 3:3, 3:1",
        );
    }

    #[test]
    fn test_polyline_edge_closed_polygon_edge_overlap() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Closed;
        let a = "# 1:1, 1:3, 3:3 | 3:3, 1:3 # ";
        let b = "# # 1:1, 1:3, 3:3, 3:1";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# # 1:1, 1:3, 3:3, 3:1",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 1:3, 3:3 | 3:3, 1:3 #",
        );
        expect_result(OpType::Difference, options_clone(&options), a, b, "# #");
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            "# # 1:1, 1:3, 3:3, 3:1",
        );
    }

    // ─── Polygon vertex matching test ───────────────────────────────

    #[test]
    fn test_polygon_vertex_matching() {
        // Tests that CrossingProcessor::ProcessEdgeCrossings() sets
        // a0_matches_polygon and a1_matches_polygon correctly even with
        // degenerate polygon geometry.
        let mut options = Options::default();
        options.polyline_model = PolylineModel::Closed;
        options.polygon_model = PolygonModel::Closed;
        let a = "# 0:0, 1:1 # ";
        let b = "# # 0:0, 1:1";
        expect_result(OpType::Union, options, a, b, "# # 0:0, 1:1");
    }

    // ─── Isolated start vertex + interior crossing tests ────────────

    #[test]
    fn test_polyline_edge_isolated_start_vertex_plus_interior_crossing() {
        let options = round_to_e(1);
        let a = "# 0:0, 0:10, 0:4 # ";
        let b = "# # 0:0, -5:5, 5:5";
        expect_result(
            OpType::Difference,
            options,
            a,
            b,
            "# 0:0, 0:0 | 0:5, 0:10, 0:5 #",
        );
    }

    #[test]
    fn test_polygon_edge_isolated_start_vertex_plus_interior_crossing() {
        let mut options = round_to_e(1);
        options.polygon_model = PolygonModel::Closed;
        let a = "# # 0:0, 5:5, -5:5";
        let b = "# # 1:4, 0:0, 0:8";
        expect_result(
            OpType::Intersection,
            options,
            a,
            b,
            "# # 0:0; 0:5, 0:8, 0.8:5",
        );
    }

    // ─── Self-intersecting polylines test ────────────────────────────

    #[test]
    fn test_self_intersecting_polylines() {
        let options = round_to_e(1);
        let a = "# 0:2, 4:2, 2:0, 2:5 #";
        let b = "# 0:4, 5:4, 3:6, 3:3 #";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            "# 0:2, 4:2, 2:0, 2:4, 2:5 | 0:4, 2:4, 5:4, 3:6, 3:3 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 2:4, 2:4 | 2:4, 2:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            "# 0:2, 4:2, 2:0, 2:4, 2:5 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options_clone(&options),
            a,
            b,
            "# 0:2, 4:2, 2:0, 2:4, 2:5 | 0:4, 2:4, 5:4, 3:6, 3:3 #",
        );

        // Now test with split_all_crossing_polyline_edges = true.
        let mut options2 = round_to_e(1);
        options2.split_all_crossing_polyline_edges = true;
        expect_result(
            OpType::Union,
            options_clone(&options2),
            a,
            b,
            "# 0:2, 2:2, 4:2, 2:0, 2:2, 2:4, 2:5 \
             | 0:4, 2:4, 3:4, 5:4, 3:6, 3:4, 3:3 #",
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options2),
            a,
            b,
            "# 2:4, 2:4 | 2:4, 2:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options2),
            a,
            b,
            "# 0:2, 2:2, 4:2, 2:0, 2:2, 2:4, 2:5 #",
        );
        expect_result(
            OpType::SymmetricDifference,
            options2,
            a,
            b,
            "# 0:2, 2:2, 4:2, 2:0, 2:2, 2:4, 2:5 \
             | 0:4, 2:4, 3:4, 5:4, 3:6, 3:4, 3:3 #",
        );
    }

    // ─── GetCrossedVertexIndex bug regression tests ─────────────────

    fn compute_test_union(
        a_loops: Vec<Vec<Point>>,
        b_loops: Vec<Vec<Point>>,
        snap_radius: s1::Angle,
    ) {
        use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
        use crate::s2::lax_polygon::LaxPolygon;

        let mut a = ShapeIndex::new();
        a.add(Box::new(LaxPolygon::from_loops_owned(a_loops)));
        a.build();

        let mut b = ShapeIndex::new();
        b.add(Box::new(LaxPolygon::from_loops_owned(b_loops)));
        b.build();

        let output = Rc::new(RefCell::new(LaxPolygon::empty()));
        let layer = LaxPolygonLayer::new_legacy(Rc::clone(&output));

        let options = Options {
            snap_function: Box::new(IdentitySnapFunction::new(snap_radius)),
            ..Options::default()
        };

        let mut op = S2BooleanOperation::new(OpType::Union, Box::new(layer), options);
        op.build(&mut a, &mut b).expect("Union failed");

        let result = output.borrow();
        assert!(!result.is_empty(), "Union result should not be empty");
    }

    #[test]
    #[ignore = "GetCrossedVertexIndex edge case not yet handled"]
    fn test_get_crossed_vertex_index_bug1() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![vec![
            Point(Vector::new(
                -0.38306437985388492,
                -0.74921955334206214,
                0.54030708099846292,
            )),
            Point(Vector::new(
                -0.3830643798552798,
                -0.74921955334134249,
                0.5403070809984718,
            )),
            Point(Vector::new(
                -0.38306437985529124,
                -0.74921955334136414,
                0.54030708099843361,
            )),
            Point(Vector::new(
                -0.38306437985389635,
                -0.74921955334208379,
                0.54030708099842473,
            )),
        ]];
        let b_loops = vec![vec![
            Point(Vector::new(
                -0.38306437985390962,
                -0.74921955334210588,
                0.54030708099838465,
            )),
            Point(Vector::new(
                -0.38306437985527797,
                -0.74921955334134205,
                0.54030708099847369,
            )),
            Point(Vector::new(
                -0.38306437985527941,
                -0.74921955334134405,
                0.54030708099847014,
            )),
            Point(Vector::new(
                -0.38306437985391095,
                -0.74921955334210777,
                0.54030708099838098,
            )),
        ]];
        compute_test_union(
            a_loops,
            b_loops,
            edge_crossings::intersection_merge_radius(),
        );
    }

    #[test]
    #[ignore = "GetCrossedVertexIndex edge case not yet handled"]
    fn test_get_crossed_vertex_index_bug2() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![vec![
            Point(Vector::new(
                -0.3837392878495085,
                -0.7477800800281974,
                0.5418201831546835,
            )),
            Point(Vector::new(
                -0.38373928785696076,
                -0.7477800800212292,
                0.54182018315902258,
            )),
            Point(Vector::new(
                -0.38373928785701278,
                -0.74778008002124685,
                0.5418201831589613,
            )),
            Point(Vector::new(
                -0.38373928785703426,
                -0.7477800800212544,
                0.54182018315893576,
            )),
            Point(Vector::new(
                -0.38373947205489456,
                -0.74778014227795497,
                0.5418199667802881,
            )),
            Point(Vector::new(
                -0.38373947204434411,
                -0.74778014228781997,
                0.54181996677414512,
            )),
            Point(Vector::new(
                -0.38373947205872994,
                -0.74778014228185352,
                0.54181996677219124,
            )),
            Point(Vector::new(
                -0.38373947218468357,
                -0.74778014288930306,
                0.54181996584462788,
            )),
            Point(Vector::new(
                -0.3837396702525171,
                -0.74778021044361542,
                0.54181973233114322,
            )),
            Point(Vector::new(
                -0.38373967023137123,
                -0.74778021046333043,
                0.54181973231891067,
            )),
            Point(Vector::new(
                -0.38373947216030285,
                -0.74778014290791484,
                0.54181996583620895,
            )),
            Point(Vector::new(
                -0.38373947217087578,
                -0.74778014289805739,
                0.54181996584232528,
            )),
            Point(Vector::new(
                -0.38373947215649007,
                -0.74778014290402395,
                0.54181996584427927,
            )),
            Point(Vector::new(
                -0.3837394720305386,
                -0.74778014229658485,
                0.5418199667718262,
            )),
            Point(Vector::new(
                -0.38373928783585998,
                -0.74778008004095942,
                0.54182018314673686,
            )),
            Point(Vector::new(
                -0.38373928784641037,
                -0.7477800800310942,
                0.54182018315287972,
            )),
            Point(Vector::new(
                -0.38373928783578648,
                -0.74778008004093421,
                0.54182018314682368,
            )),
            Point(Vector::new(
                -0.383739287835765,
                -0.74778008004092666,
                0.54182018314684921,
            )),
        ]];
        let b_loops = vec![vec![
            Point(Vector::new(
                -0.38373923813692823,
                -0.7477800632164362,
                0.54182024156551456,
            )),
            Point(Vector::new(
                -0.3837392878569364,
                -0.74778008002122087,
                0.54182018315905123,
            )),
            Point(Vector::new(
                -0.38373928784640354,
                -0.74778008003106944,
                0.54182018315291858,
            )),
            Point(Vector::new(
                -0.38373928784638789,
                -0.74778008003108642,
                0.54182018315290648,
            )),
            Point(Vector::new(
                -0.38373928784638023,
                -0.74778008003109453,
                0.54182018315290048,
            )),
            Point(Vector::new(
                -0.38373928783692102,
                -0.74778008004124585,
                0.54182018314559,
            )),
            Point(Vector::new(
                -0.38373928783691913,
                -0.74778008004124541,
                0.54182018314559188,
            )),
            Point(Vector::new(
                -0.38373928784636568,
                -0.74778008003110774,
                0.54182018315289271,
            )),
            Point(Vector::new(
                -0.38373928784637329,
                -0.74778008003109953,
                0.54182018315289848,
            )),
            Point(Vector::new(
                -0.38373928783583561,
                -0.74778008004095109,
                0.5418201831467655,
            )),
            Point(Vector::new(
                -0.38373923811582744,
                -0.74778006323616641,
                0.54182024155322883,
            )),
            Point(Vector::new(
                -0.38373857650312843,
                -0.74777983961840766,
                0.54182101875399913,
            )),
            Point(Vector::new(
                -0.38373857652422921,
                -0.74777983959867744,
                0.54182101876628486,
            )),
        ]];
        compute_test_union(
            a_loops,
            b_loops,
            edge_crossings::intersection_merge_radius(),
        );
    }

    #[test]
    fn test_get_crossed_vertex_index_bug3() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![vec![
            Point(Vector::new(1.0, 0.0, 2.4678234835261742e-72)),
            Point(Vector::new(
                0.99984769515639127,
                0.017452406437283512,
                1.8530922845942552e-27,
            )),
            Point(Vector::new(
                0.99740259703611311,
                0.069881849826437858,
                0.017452406437283512,
            )),
        ]];
        let b_loops = vec![vec![
            Point(Vector::new(
                0.99999999999999989,
                2.4674476220564615e-72,
                2.4678234835261742e-72,
            )),
            Point(Vector::new(
                0.99999999999999989,
                2.8837981406657438e-169,
                2.4678234835261742e-72,
            )),
            Point(Vector::new(
                1.0,
                2.8837981406657432e-169,
                2.4678234835261742e-72,
            )),
        ]];
        compute_test_union(a_loops, b_loops, s1::Angle::from_radians(0.0));
    }

    #[test]
    fn test_get_crossed_vertex_index_bug4() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![vec![
            Point(Vector::new(
                0.62233331065911901,
                -0.0014161759526823048,
                0.78275107466533156,
            )),
            Point(Vector::new(
                0.6223328557578689,
                -0.0014164217071954736,
                0.78275143589379825,
            )),
            text_format::parse_point("51.51317:-0.1306"),
        ]];
        let b_loops = vec![vec![
            Point(Vector::new(
                0.62233331033809591,
                -0.001416176126110953,
                0.78275107492024998,
            )),
            Point(Vector::new(
                0.62233331033809591,
                -0.0014161761261109063,
                0.78275107492025009,
            )),
            text_format::parse_point("51.52:-0.12"),
            text_format::parse_point("51.52:-0.14"),
        ]];
        compute_test_union(a_loops, b_loops, s1::Angle::from_radians(0.0));
    }

    #[test]
    fn test_get_crossed_vertex_index_bug5() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![vec![
            Point(Vector::new(0.99984769515639127, 0.0, 0.017452406437283512)),
            Point(Vector::new(
                0.99923861495548261,
                0.017441774902830158,
                0.034899496702500969,
            )),
            Point(Vector::new(
                0.99847743863945992,
                0.052327985223313139,
                0.017452406437283512,
            )),
            Point(Vector::new(
                0.99802119662406841,
                0.034851668155187324,
                0.052335956242943835,
            )),
        ]];
        let b_loops = vec![
            vec![
                Point(Vector::new(
                    0.99802119662406841,
                    0.034851668155187324,
                    0.052335956242943835,
                )),
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99802098681615425,
                    0.034839714972148959,
                    0.052347914334467859,
                )),
                Point(Vector::new(
                    0.99741208276778681,
                    0.017411821260589495,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99741219210106513,
                    0.017411340538768819,
                    0.069755030419252628,
                )),
                Point(Vector::new(
                    0.99741211642315963,
                    0.017409893252357169,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99984769515639116,
                    4.9500424645560228e-16,
                    0.017452406437284993,
                )),
                Point(Vector::new(
                    0.99984769515639127,
                    3.7368529835165677e-16,
                    0.017452406437284632,
                )),
                Point(Vector::new(
                    0.99984769515639116,
                    3.3065924905014365e-16,
                    0.017452406437284504,
                )),
                Point(Vector::new(
                    0.99984769515639127,
                    9.9060035932242025e-16,
                    0.017452406437284504,
                )),
                Point(Vector::new(
                    0.99969541350954794,
                    0.017449748351250485,
                    0.017452406437283512,
                )),
            ],
            vec![
                Point(Vector::new(
                    0.99984769515639116,
                    3.3065924905014365e-16,
                    0.017452406437284504,
                )),
                Point(Vector::new(
                    0.99984769515639116,
                    3.3006856770496304e-16,
                    0.017452406437284504,
                )),
                Point(Vector::new(0.99984769515639127, 0.0, 0.017452406437284504)),
                Point(Vector::new(0.99984769515639127, 0.0, 0.017452406437283512)),
            ],
        ];
        compute_test_union(a_loops, b_loops, s1::Angle::from_radians(0.0));
    }

    #[test]
    #[ignore = "GetCrossedVertexIndex edge case not yet handled"]
    fn test_get_crossed_vertex_index_bug6() {
        use crate::r3::Vector;
        use crate::s2::Point;
        let a_loops = vec![
            vec![
                Point(Vector::new(
                    0.99870488823558456,
                    0.026138065586168355,
                    0.043650289137205818,
                )),
                Point(Vector::new(
                    0.99876259434149239,
                    0.030513215246694664,
                    0.0392711578586665,
                )),
                Point(Vector::new(0.99984769515639127, 0.017452406437283512, 0.0)),
                Point(Vector::new(
                    0.998782023517925,
                    0.034862286684437908,
                    0.034915476003791211,
                )),
                Point(Vector::new(
                    0.99878202512991221,
                    0.034878236872062651,
                    0.034899496702500969,
                )),
                Point(Vector::new(0.9975640502598242, 0.069756473744125302, 0.0)),
                Point(Vector::new(
                    0.99877979583714305,
                    0.034883478425067296,
                    0.034958008531414335,
                )),
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99847581234813876,
                    0.017465633646566288,
                    0.052354596713645812,
                )),
                Point(Vector::new(0.9975640502598242, 0.0, 0.069756473744125302)),
                Point(Vector::new(
                    0.99847674250410212,
                    0.017444393356200013,
                    0.052343937746706169,
                )),
                Point(Vector::new(
                    0.99847743863945992,
                    0.017428488520812163,
                    0.052335956242943835,
                )),
                Point(Vector::new(0.99984769515639127, 0.0, 0.017452406437283512)),
            ],
            vec![
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99802119661969568,
                    0.034851668280404598,
                    0.052335956242943835,
                )),
                Point(Vector::new(
                    0.9987605225894034,
                    0.030527121154938986,
                    0.039313018084772409,
                )),
                Point(Vector::new(
                    0.99870321796526884,
                    0.026161932439896601,
                    0.043674199670139441,
                )),
            ],
            vec![
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99619692339885657,
                    0.06966087492121549,
                    0.052335956242943835,
                )),
                Point(Vector::new(
                    0.99513403437078507,
                    0.069586550480032719,
                    0.069756473744125302,
                )),
            ],
        ];
        let b_loops = vec![
            vec![
                Point(Vector::new(
                    0.99802200429988497,
                    0.034828499898458924,
                    0.052335977377554299,
                )),
                Point(Vector::new(0.99862953475457383, 0.0, 0.052335956242943835)),
                Point(Vector::new(
                    0.99923793061512223,
                    0.017455729388178846,
                    0.034912111530741322,
                )),
                Point(Vector::new(
                    0.99923859085845868,
                    0.017443155365764275,
                    0.034899496702500969,
                )),
                Point(Vector::new(
                    0.99923793076147094,
                    0.017455737780810811,
                    0.034912103145779166,
                )),
                Point(Vector::new(
                    0.9992865072388355,
                    0.020934110218524152,
                    0.0314362764933699,
                )),
                Point(Vector::new(1.0, 0.0, 0.0)),
                Point(Vector::new(
                    0.99929987808789411,
                    0.022418034384064717,
                    0.029953053064335624,
                )),
                Point(Vector::new(
                    0.99931406232431441,
                    0.02616995393092059,
                    0.026201876881811362,
                )),
                Point(Vector::new(0.99984769515639127, 0.017452406437283512, 0.0)),
                Point(Vector::new(
                    0.99930573320200933,
                    0.029072747464899757,
                    0.023298646837028814,
                )),
                Point(Vector::new(
                    0.99862953475457383,
                    0.052335956242943835,
                    1.700986599320836e-73,
                )),
                Point(Vector::new(
                    0.99838518277004218,
                    0.038347188759395717,
                    0.041910857059723181,
                )),
                Point(Vector::new(
                    0.99619692339885668,
                    0.052208468483931979,
                    0.069756473744125289,
                )),
            ],
            vec![
                Point(Vector::new(
                    0.99802119662406841,
                    0.052304074592470849,
                    0.034899496702500969,
                )),
                Point(Vector::new(
                    0.99847743834686298,
                    0.052327990806397578,
                    0.017452406437283512,
                )),
                Point(Vector::new(
                    0.99619645281505653,
                    0.052208443821680058,
                    0.069763212314351342,
                )),
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483932,
                    0.069756473744125316,
                )),
                Point(Vector::new(
                    0.99619692339885657,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99619692339885679,
                    0.052208468483931993,
                    0.069756473744125316,
                )),
                Point(Vector::new(
                    0.99619692339885679,
                    0.052208468483931986,
                    0.069756473744125302,
                )),
                Point(Vector::new(
                    0.99619692339885668,
                    0.052208468483931979,
                    0.069756473744125289,
                )),
            ],
        ];
        compute_test_union(a_loops, b_loops, s1::Angle::from_radians(0.0));
    }

    // ─── Full/empty polygon results ──────────────────────────────────

    fn expect_polygon(op_type: OpType, a_str: &str, b_str: &str, expected_str: &str) {
        use crate::s1::Angle;
        use crate::s2::builder::lax_polygon_layer::{DegenerateBoundaries, LaxPolygonLayer};
        use crate::s2::lax_polygon::LaxPolygon;

        let mut a = text_format::make_index(&format!("# # {a_str}"));
        let mut b = text_format::make_index(&format!("# # {b_str}"));

        let output = Rc::new(RefCell::new(LaxPolygon::empty()));
        // C++ uses DegenerateBoundaries::DISCARD here.
        let layer_opts = crate::s2::builder::lax_polygon_layer::Options {
            degenerate_boundaries: DegenerateBoundaries::Discard,
            ..Default::default()
        };
        let layer = LaxPolygonLayer::with_options_legacy(Rc::clone(&output), layer_opts);

        let options = Options {
            snap_function: Box::new(IdentitySnapFunction::new(Angle::from_degrees(1.1))),
            ..Options::default()
        };

        let mut op = S2BooleanOperation::new(op_type, Box::new(layer), options);
        let result = op.build(&mut a, &mut b);
        assert!(
            result.is_ok(),
            "{op_type:?} failed: {:?}\n  a = {a_str}\n  b = {b_str}",
            result.err()
        );

        let result = output.borrow();
        if expected_str.is_empty() {
            assert!(
                result.is_empty(),
                "{op_type:?}: expected empty, got non-empty\n  a = {a_str}\n  b = {b_str}"
            );
        } else if expected_str == "full" {
            assert!(
                result.is_full(),
                "{op_type:?}: expected full, got non-full\n  a = {a_str}\n  b = {b_str}"
            );
        } else {
            let expected = text_format::make_lax_polygon(expected_str);
            assert_eq!(
                result.num_loops(),
                expected.num_loops(),
                "{op_type:?}: loop count mismatch\n  a = {a_str}\n  b = {b_str}\n  expected = {expected_str}"
            );
        }
    }

    #[test]
    fn test_full_and_empty_results() {
        let empty = "";
        let full = "full";
        let shell1 = "10:0, 10:10, 20:10";
        let hole1 = "10:0, 20:10, 10:10";
        let shell1_minus = "11:2, 11:9, 18:9";
        let shell1_plus = "9:-2, 9:11, 22:11";
        let shell2 = "10:20, 10:30, 20:30";
        let hole2 = "10:20, 20:30, 10:30";
        let north_hemi = "0:0, 0:120, 0:-120";
        let south_hemi = "0:0, 0:-120, 0:120";
        let south_hemi_plus = "0.5:0, 0.5:-120, 0.5:120";

        // 6-face shells/holes
        let shell6 = "0:-45, 45:0, 45:90, 0:135, -45:180, -45:-90";
        let hole6 = "0:-45, -45:-90, -45:180, 0:135, 45:90, 45:0";
        let shell6_minus = "-1:-45, 44:0, 44:90, -1:135, -46:180, -46:-90";
        let shell6_plus = "1:-45, 46:0, 46:90, 1:135, -44:180, -44:-90";

        // Small polygons that disappear when snap radius is used
        let almost_empty1 = "2:0, 2:10, 3:0";
        let almost_full1 = "2:0, 3:0, 2:10";
        let almost_empty2 = "4:0, 4:10, 5:0";
        let almost_full2 = "4:0, 5:0, 4:10";

        let face6_almost_empty1 = &format!("{shell6_minus}; {hole6}");

        // Test empty UNION results.
        expect_polygon(OpType::Union, empty, empty, empty);
        expect_polygon(OpType::Union, almost_empty1, almost_empty2, empty);
        expect_polygon(
            OpType::Union,
            face6_almost_empty1,
            face6_almost_empty1,
            empty,
        );

        // Test full UNION results.
        expect_polygon(OpType::Union, empty, full, full);
        expect_polygon(OpType::Union, full, full, full);
        expect_polygon(OpType::Union, full, shell1, full);
        expect_polygon(OpType::Union, hole1, hole2, full);
        expect_polygon(OpType::Union, hole1, shell1, full);
        expect_polygon(OpType::Union, hole1, shell1_minus, full);
        expect_polygon(OpType::Union, hole6, shell6_minus, full);

        // Test empty INTERSECTION results.
        expect_polygon(OpType::Intersection, empty, empty, empty);
        expect_polygon(OpType::Intersection, empty, full, empty);
        expect_polygon(OpType::Intersection, full, empty, empty);
        expect_polygon(OpType::Intersection, empty, hole1, empty);
        expect_polygon(OpType::Intersection, shell1, shell2, empty);
        expect_polygon(OpType::Intersection, shell1, hole1, empty);
        expect_polygon(OpType::Intersection, shell6, hole6, empty);
        expect_polygon(OpType::Intersection, shell1_plus, hole1, empty);
        expect_polygon(OpType::Intersection, shell6_plus, hole6, empty);

        // Test full INTERSECTION results.
        expect_polygon(OpType::Intersection, full, full, full);
        expect_polygon(OpType::Intersection, almost_full1, almost_full2, full);

        // Test empty DIFFERENCE results.
        expect_polygon(OpType::Difference, empty, empty, empty);
        expect_polygon(OpType::Difference, empty, full, empty);
        expect_polygon(OpType::Difference, full, full, empty);
        expect_polygon(OpType::Difference, empty, shell1, empty);
        expect_polygon(OpType::Difference, shell1, full, empty);
        expect_polygon(OpType::Difference, shell1, shell1, empty);
        expect_polygon(OpType::Difference, shell1, hole2, empty);
        expect_polygon(OpType::Difference, shell6, shell6_plus, empty);
        expect_polygon(OpType::Difference, shell1_plus, shell1, empty);
        expect_polygon(OpType::Difference, shell6_plus, shell6, empty);

        // Test full DIFFERENCE results.
        expect_polygon(OpType::Difference, full, empty, full);
        expect_polygon(OpType::Difference, almost_full1, almost_empty2, full);

        // Test empty SYMMETRIC_DIFFERENCE results.
        expect_polygon(OpType::SymmetricDifference, empty, empty, empty);
        expect_polygon(OpType::SymmetricDifference, full, full, empty);
        expect_polygon(OpType::SymmetricDifference, shell1, shell1, empty);
        expect_polygon(OpType::SymmetricDifference, north_hemi, north_hemi, empty);
        expect_polygon(OpType::SymmetricDifference, shell6, shell6, empty);
        expect_polygon(OpType::SymmetricDifference, shell1_plus, shell1, empty);
        expect_polygon(OpType::SymmetricDifference, shell6_plus, shell6, empty);
        expect_polygon(OpType::SymmetricDifference, shell6_minus, shell6, empty);

        // Test full SYMMETRIC_DIFFERENCE results.
        expect_polygon(OpType::SymmetricDifference, full, empty, full);
        expect_polygon(OpType::SymmetricDifference, empty, full, full);
        expect_polygon(OpType::SymmetricDifference, shell1, hole1, full);
        expect_polygon(
            OpType::SymmetricDifference,
            almost_empty1,
            almost_full1,
            full,
        );
        expect_polygon(OpType::SymmetricDifference, shell1_plus, hole1, full);
        expect_polygon(
            OpType::SymmetricDifference,
            almost_full1,
            almost_empty2,
            full,
        );
        expect_polygon(OpType::SymmetricDifference, north_hemi, south_hemi, full);
        expect_polygon(
            OpType::SymmetricDifference,
            north_hemi,
            south_hemi_plus,
            full,
        );
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    fn options_clone(o: &Options) -> Options {
        Options {
            snap_function: o.snap_function.clone_snap(),
            polygon_model: o.polygon_model,
            polyline_model: o.polyline_model,
            polyline_loops_have_boundaries: o.polyline_loops_have_boundaries,
            split_all_crossing_polyline_edges: o.split_all_crossing_polyline_edges,
            memory_tracker: o.memory_tracker.clone(),
        }
    }

    /// C++ TEST(S2BooleanOperation, `PolylineVertexOpenPolygonClosedPolylineVertex`)
    ///
    /// Tests the interaction between polyline vertices and both open polygon
    /// vertices and closed polyline vertices from the other region. This
    /// exercises `crossing_processor` paths for mixed polyline+polygon geometry
    /// in region B.
    #[test]
    fn test_polyline_vertex_open_polygon_closed_polyline_vertex() {
        let mut options = Options::default();
        options.polygon_model = PolygonModel::Open;
        let polygon_str = vertex_test_polygon_str();
        let test_geometry_suffix = format!(
            "-2:0, 0:1 | -2:1, 0:2 | -2:2, 0:3 | -2:3, 0:4 | \
             7:0, 5:1 | 7:1, 5:2 | 7:2, 5:3 | 7:3, 5:4 # {polygon_str}"
        );
        let a = "# 1:1, 0:1 | 0:2, 1:2 | -1:3, 0:3 | 0:4, -1:4 \
                 | 6:1, 5:1 | 5:2, 6:2 | 4:3, 5:3 | 5:4, 4:4 #";
        let b = &format!("# {test_geometry_suffix}");
        let difference_prefix = "# -1:3, 0:3 | 0:4, -1:4 | 6:1, 5:1 | 5:2, 6:2";
        expect_result(
            OpType::Union,
            options_clone(&options),
            a,
            b,
            &format!(
                "{difference_prefix} | 0:1, 0:1 | 0:2, 0:2 | 5:3, 5:3 | 5:4, 5:4 | {test_geometry_suffix}"
            ),
        );
        expect_result(
            OpType::Intersection,
            options_clone(&options),
            a,
            b,
            "# 1:1, 0:1 | 0:2, 1:2 | 0:3, 0:3 | 0:4, 0:4\
             | 5:1, 5:1 | 5:2, 5:2 | 4:3, 5:3 | 5:4, 4:4\
             | 0:1, 0:1 | 0:2, 0:2 | 0:3, 0:3 | 0:4, 0:4\
             | 5:1, 5:1 | 5:2, 5:2 | 5:3, 5:3 | 5:4, 5:4 #",
        );
        expect_result(
            OpType::Difference,
            options_clone(&options),
            a,
            b,
            &format!("{difference_prefix} #"),
        );
        expect_result(
            OpType::SymmetricDifference,
            options,
            a,
            b,
            &format!("{difference_prefix} | {test_geometry_suffix}"),
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_enums_roundtrip() {
        for v in [
            OpType::Union,
            OpType::Intersection,
            OpType::Difference,
            OpType::SymmetricDifference,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<OpType>(&j).unwrap());
        }
        for v in [
            PolygonModel::Open,
            PolygonModel::SemiOpen,
            PolygonModel::Closed,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<PolygonModel>(&j).unwrap());
        }
        for v in [
            PolylineModel::Open,
            PolylineModel::SemiOpen,
            PolylineModel::Closed,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<PolylineModel>(&j).unwrap());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_source_id_roundtrip() {
        let sid = SourceId::new(RegionId::B, 3, 7);
        let json = serde_json::to_string(&sid).unwrap();
        let back: SourceId = serde_json::from_str(&json).unwrap();
        assert_eq!(sid, back);
    }
}
