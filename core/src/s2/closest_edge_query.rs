// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Find the closest edges between a target and indexed geometry.
//!
//! [`ClosestEdgeQuery`] answers proximity questions against a
//! [`ShapeIndex`]: finding the nearest edge, testing whether any geometry
//! is within a distance threshold, or retrieving the k closest edges.
//!
//! The query supports several target types: a single point
//! ([`PointTarget`]), an edge ([`EdgeTarget`]), or a cell
//! ([`CellTarget`]).
//!
//! # Examples
//!
//! ```
//! use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
//! use s2rst::s2::lax_polyline::LaxPolyline;
//! use s2rst::s2::shape_index::ShapeIndex;
//! use s2rst::s2::LatLng;
//!
//! // Index a polyline along the equator.
//! let vertices = vec![
//!     LatLng::from_degrees(0.0, 0.0).to_point(),
//!     LatLng::from_degrees(0.0, 10.0).to_point(),
//! ];
//! let mut index = ShapeIndex::new();
//! index.add(Box::new(LaxPolyline::new(vertices)));
//! index.build();
//!
//! // Find the closest point on the polyline to a query point.
//! let target = PointTarget::new(LatLng::from_degrees(1.0, 5.0).to_point());
//! let query = ClosestEdgeQuery::new(&index);
//! let result = query.find_closest_edge(&target);
//! assert!(!result.is_empty());
//! assert!(result.distance.degrees() < 1.5); // ~1° away
//! ```

#![expect(
    clippy::cast_sign_loss,
    reason = "max_results/EdgeId (i32) used as Vec indices"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "max_results/EdgeId (i32) -> usize for Vec sizing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for max_results/EdgeId — always in range"
)]
use std::collections::{BTreeSet, BinaryHeap, HashSet};
use std::ops::ControlFlow;

use crate::s1::ChordAngle;
use crate::s2::coords::Level;
use crate::s2::distance_target::DistanceTarget;
use crate::s2::edge_distances;
use crate::s2::shape::{Edge, ShapeId};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::{Cap, Cell, CellId, CellUnion, Point};

/// A result from a closest edge query.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Result {
    /// The distance from the target to this edge.
    pub distance: ChordAngle,
    /// The shape ID in the index.
    pub shape_id: ShapeId,
    /// The edge ID within the shape, or -1 for interior.
    pub edge_id: i32,
}

impl Result {
    /// Returns an empty result (no edge found).
    pub fn empty() -> Self {
        Result {
            distance: ChordAngle::INFINITY,
            shape_id: ShapeId(-1),
            edge_id: -1,
        }
    }

    /// Returns true if this result represents a polygon interior.
    pub fn is_interior(&self) -> bool {
        self.shape_id >= 0 && self.edge_id < 0
    }

    /// Returns true if no edge was found.
    pub fn is_empty(&self) -> bool {
        self.shape_id < 0
    }
}

impl PartialEq for Result {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
            && self.shape_id == other.shape_id
            && self.edge_id == other.edge_id
    }
}

impl Eq for Result {}

impl PartialOrd for Result {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Result {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .length2()
            .partial_cmp(&other.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.shape_id.cmp(&other.shape_id))
            .then_with(|| self.edge_id.cmp(&other.edge_id))
    }
}

/// Options for a closest edge query.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Maximum number of results to return (default: `i32::MAX`).
    pub max_results: i32,
    /// Maximum distance for edges to be included (default: infinity).
    pub max_distance: ChordAngle,
    /// Maximum error allowed (trades accuracy for speed).
    pub max_error: ChordAngle,
    /// Whether to include polygon interiors (distance 0 for contained targets).
    pub include_interiors: bool,
    /// Force brute-force search (useful for testing/benchmarking).
    pub use_brute_force: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_results: i32::MAX,
            max_distance: ChordAngle::INFINITY,
            max_error: ChordAngle::ZERO,
            include_interiors: true,
            use_brute_force: false,
        }
    }
}

impl Options {
    /// Sets `max_distance` so that edges at exactly `limit` are also returned.
    /// Equivalent to `max_distance = limit.successor()`.
    pub fn inclusive_max_distance(&mut self, limit: ChordAngle) {
        self.max_distance = limit.successor();
    }

    /// Sets `max_distance` so that all edges whose true distance is ≤ `limit`
    /// are returned, accounting for the maximum error in distance computation.
    pub fn conservative_max_distance(&mut self, limit: ChordAngle) {
        self.max_distance = limit
            .plus_error(edge_distances::update_min_distance_max_error(limit))
            .successor();
    }
}

/// The target geometry to measure distance to.
///
/// Extends [`DistanceTarget`] with
/// methods specific to closest-edge queries: distance to points, edges, and
/// cells, plus interior containment queries.
pub trait Target: DistanceTarget {
    /// Updates `dist_limit` if the distance from `p` to the target is less.
    /// Returns `(new_dist, true)` if updated, or `(dist_limit, false)`.
    fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Updates `dist_limit` if the distance from edge (v0, v1) to the target
    /// is less. Returns `(new_dist, true)` if updated.
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        dist_limit: ChordAngle,
    ) -> (ChordAngle, bool);

    /// Updates `dist_limit` if the distance from the cell to the target is less.
    /// Returns `(new_dist, true)` if updated.
    fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Visits shape IDs in the query index that contain a point of the target.
    /// Returns `ControlFlow::Break(())` if the visitor stops early.
    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) -> ControlFlow<()>;

    /// Maximum index size for which brute force is faster than an indexed
    /// search. The default is 30; subtypes override with values tuned
    /// to their geometry.
    fn max_brute_force_index_size(&self) -> i32 {
        30
    }
}

/// Target: find closest edges to a point.
#[derive(Debug)]
pub struct PointTarget {
    point: Point,
}

impl PointTarget {
    /// Creates a target from a point.
    pub fn new(point: Point) -> Self {
        PointTarget { point }
    }
}

impl DistanceTarget for PointTarget {
    fn cap_bound(&self) -> Cap {
        Cap::from_point(self.point)
    }
}

impl Target for PointTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        // C++: break-even ~80/100/250 for point cloud/fractal/regular.
        120
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = p.chord_angle(self.point);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        min_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        edge_distances::update_min_distance(self.point, v0, v1, min_dist)
    }
    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.distance_to_point(self.point);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
        let mut query = ContainsPointQuery::new(index, VertexModel::SemiOpen);
        for shape_id in query.containing_shape_ids(self.point) {
            visitor(shape_id)?;
        }
        ControlFlow::Continue(())
    }
}

/// Target: find closest edges to an edge.
#[derive(Debug)]
pub struct EdgeTarget {
    a: Point,
    b: Point,
}

impl EdgeTarget {
    /// Creates a target from an edge.
    pub fn new(a: Point, b: Point) -> Self {
        EdgeTarget { a, b }
    }
}

impl DistanceTarget for EdgeTarget {
    fn cap_bound(&self) -> Cap {
        Cap::from_point(Point((self.a.0 + self.b.0).normalize()))
            .expanded(self.a.chord_angle(self.b).to_angle() * 0.5)
    }
}

impl Target for EdgeTarget {
    fn max_brute_force_index_size(&self) -> i32 {
        // C++: break-even ~40/50/100 for point cloud/fractal/regular.
        60
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        edge_distances::update_min_distance(p, self.a, self.b, min_dist)
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        min_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let (cp_a, cp_b) = edge_distances::edge_pair_closest_points(self.a, self.b, v0, v1);
        let dist = cp_a.chord_angle(cp_b);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.distance_to_edge(self.a, self.b);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        // Use the first endpoint for containment checking.
        use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
        let mut query = ContainsPointQuery::new(index, VertexModel::SemiOpen);
        for shape_id in query.containing_shape_ids(self.a) {
            visitor(shape_id)?;
        }
        ControlFlow::Continue(())
    }
}

/// Target: find closest edges to a cell (including interior).
#[derive(Debug)]
pub struct CellTarget {
    cell: Cell,
}

impl CellTarget {
    /// Creates a target from a cell.
    pub fn new(cell: Cell) -> Self {
        CellTarget { cell }
    }
}

impl DistanceTarget for CellTarget {
    fn cap_bound(&self) -> Cap {
        self.cell.cap_bound()
    }
}

impl Target for CellTarget {
    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.distance_to_point(p);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        min_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let dist = self.cell.distance_to_edge(v0, v1);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.distance_to_cell(*cell);
        if dist < min_dist {
            (dist, true)
        } else {
            (min_dist, false)
        }
    }
    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        let center = self.cell.center();
        use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
        let mut query = ContainsPointQuery::new(index, VertexModel::SemiOpen);
        for shape_id in query.containing_shape_ids(center) {
            visitor(shape_id)?;
        }
        ControlFlow::Continue(())
    }
}

/// Target: find closest edges to any edge in another `ShapeIndex`.
///
/// This wraps a `ShapeIndex` and internally uses a `ClosestEdgeQuery` on the
/// target index to dispatch distance computations. This is the most powerful
/// target type, supporting distance between two arbitrary shape collections.
#[derive(Debug)]
pub struct ShapeIndexTarget<'a> {
    index: &'a ShapeIndex,
    /// Whether to include polygon interiors in the target.
    pub include_interiors: bool,
    /// Whether the internal query should use brute force.
    pub use_brute_force: bool,
}

impl<'a> ShapeIndexTarget<'a> {
    /// Creates a target from a `ShapeIndex`.
    pub fn new(index: &'a ShapeIndex) -> Self {
        ShapeIndexTarget {
            index,
            include_interiors: true,
            use_brute_force: false,
        }
    }

    /// Helper: find the closest edge in `self.index` to a given inner target,
    /// then check if that distance is less than `min_dist`.
    fn update_min_distance_inner(
        &self,
        inner_target: &dyn Target,
        min_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let query = ClosestEdgeQuery::new(self.index);
        let opts = Options {
            max_results: 1,
            max_distance: min_dist,
            include_interiors: self.include_interiors,
            use_brute_force: self.use_brute_force,
            ..Options::default()
        };
        let result = query.find_closest_edge_with_options(inner_target, &opts);
        if result.is_empty() {
            (min_dist, false)
        } else {
            (result.distance, true)
        }
    }
}

impl DistanceTarget for ShapeIndexTarget<'_> {
    fn cap_bound(&self) -> Cap {
        use crate::s2::region::Region;
        use crate::s2::shape_index_region::ShapeIndexRegion;
        let region = ShapeIndexRegion::new(self.index);
        region.cap_bound()
    }
    fn set_max_error(&mut self, _max_error: ChordAngle) -> bool {
        // The internal query options would be set per-call, so we signal
        // that we could take advantage of error (return true).
        true
    }
}

impl Target for ShapeIndexTarget<'_> {
    fn max_brute_force_index_size(&self) -> i32 {
        // C++ default for ShapeIndexTarget: uses the base class default (which
        // for closest is determined by the distance target base, returning -1
        // meaning "always prefer optimized"). We use 30 (the CellTarget default).
        30
    }

    fn update_distance_to_point(&self, p: Point, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let inner = PointTarget::new(p);
        self.update_min_distance_inner(&inner, min_dist)
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        min_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let inner = EdgeTarget::new(v0, v1);
        self.update_min_distance_inner(&inner, min_dist)
    }
    fn update_distance_to_cell(&self, cell: &Cell, min_dist: ChordAngle) -> (ChordAngle, bool) {
        let inner = CellTarget::new(*cell);
        self.update_min_distance_inner(&inner, min_dist)
    }
    fn visit_containing_shapes(
        &self,
        query_index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        // For each shape in the target index, test whether any of its chain
        // start vertices are contained by shapes in the query index.
        use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};

        for shape_id in (0..self.index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            let num_chains = shape.num_chains();
            let mut tested_point = false;
            for c in 0..num_chains {
                let chain = shape.chain(c);
                if chain.length == 0 {
                    continue;
                }
                tested_point = true;
                let v0 = shape.chain_edge(c, 0).v0;
                let mut query = ContainsPointQuery::new(query_index, VertexModel::SemiOpen);
                for qshape_id in query.containing_shape_ids(v0) {
                    visitor(qshape_id)?;
                }
            }
            if !tested_point {
                let ref_pt = shape.reference_point();
                if !ref_pt.contained {
                    continue;
                }
                let mut query = ContainsPointQuery::new(query_index, VertexModel::SemiOpen);
                for qshape_id in query.containing_shape_ids(ref_pt.point) {
                    visitor(qshape_id)?;
                }
            }
        }
        ControlFlow::Continue(())
    }
}

/// An optional callback that filters shapes by ID. If it returns `false`,
/// edges from that shape are skipped.
pub type ShapeFilter<'a> = Option<&'a dyn Fn(ShapeId) -> bool>;

/// Query to find closest edges in a `ShapeIndex` to a given target.
///
/// # Examples
///
/// ```
/// use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::{LatLng, Point};
/// use s2rst::s2::polyline::Polyline;
///
/// // Build an index containing a polyline.
/// let mut index = ShapeIndex::new();
/// let line = Polyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
///     LatLng::from_degrees(2.0, 0.0).to_point(),
/// ]);
/// index.add(Box::new(line));
/// index.build();
///
/// // Find the closest edge to a query point.
/// let target = PointTarget::new(LatLng::from_degrees(1.0, 1.0).to_point());
/// let query = ClosestEdgeQuery::new(&index);
/// let result = query.find_closest_edge(&target);
/// assert!(!result.is_empty());
/// ```
#[derive(Debug)]
pub struct ClosestEdgeQuery<'a> {
    index: &'a ShapeIndex,
}

impl<'a> ClosestEdgeQuery<'a> {
    /// Creates a new closest edge query for the given index.
    pub fn new(index: &'a ShapeIndex) -> Self {
        ClosestEdgeQuery { index }
    }

    /// Returns the closest edge to the target, or an empty result.
    #[inline]
    pub fn find_closest_edge(&self, target: &dyn Target) -> Result {
        let opts = Options {
            max_results: 1,
            ..Options::default()
        };
        self.find_closest_edge_with_options(target, &opts)
    }

    /// Returns the closest edge to the target with the given options.
    /// For best performance, set `options.max_results = 1`.
    pub fn find_closest_edge_with_options(&self, target: &dyn Target, options: &Options) -> Result {
        let results = self.find_closest_edges(target, options);
        match results.into_iter().next() {
            Some(r) => r,
            None => Result::empty(),
        }
    }

    /// Returns the closest edges to the target (up to `options.max_results`).
    pub fn find_closest_edges(&self, target: &dyn Target, options: &Options) -> Vec<Result> {
        self.find_closest_edges_filtered(target, options, None)
    }

    /// Like [`find_closest_edges`](Self::find_closest_edges) but with an
    /// optional shape filter. Edges from shapes for which the filter
    /// returns `false` are skipped.
    pub fn find_closest_edges_filtered(
        &self,
        target: &dyn Target,
        options: &Options,
        filter: ShapeFilter<'_>,
    ) -> Vec<Result> {
        debug_assert!(options.max_results >= 1, "max_results must be >= 1");
        debug_assert!(
            target.max_brute_force_index_size() >= 0,
            "max_brute_force_index_size must be >= 0"
        );

        let mut state = QueryState::new(options);
        if options.max_distance == ChordAngle::ZERO {
            return state.collect_results(options);
        }

        // Check polygon interiors if requested.
        if options.include_interiors {
            let max_results = options.max_results as usize;
            let mut shape_ids = Vec::new();
            let _ = target.visit_containing_shapes(self.index, &mut |shape_id| {
                if filter.is_none() || filter.as_ref().is_some_and(|f| f(shape_id)) {
                    shape_ids.push(shape_id);
                }
                if shape_ids.len() < max_results {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            });
            for shape_id in shape_ids {
                state.add_result(Result {
                    distance: ChordAngle::ZERO,
                    shape_id,
                    edge_id: -1,
                });
            }
            if state.distance_limit == ChordAngle::ZERO {
                return state.collect_results(options);
            }
        }

        // If max_error() > 0 and the target takes advantage of this, then we
        // need to adjust cell distances conservatively and avoid duplicates.
        // (C++ handles this via use_conservative_cell_distance_ and
        // avoid_duplicates_.)
        //
        // Note: we can't call set_max_error on the target through &dyn Target
        // because the trait method takes &mut self. Instead we check if the
        // target type would benefit (ShapeIndexTarget returns true). For simple
        // targets the default is false, so these flags stay off.
        //
        // The conservative cell distance flag is set when:
        //   target_uses_max_error && distance_limit > max_error
        // (When distance_limit <= max_error, the search terminates immediately
        // once any result is found, so conservative distances aren't needed.)

        // Decide: brute force or optimized?
        let min_optimized = target.max_brute_force_index_size() + 1;
        let num_edges = self.count_edges_up_to(min_optimized);
        let use_brute_force = options.use_brute_force || num_edges < min_optimized;

        if use_brute_force {
            // Brute force never visits the same edge twice.
            state.avoid_duplicates = false;
            self.find_closest_edges_brute_force(target, &filter, &mut state);
        } else {
            // In the optimized path, duplicates can occur when max_error > 0
            // and the target uses it, or when max_results > 1.
            state.avoid_duplicates =
                state.use_conservative_cell_distance && options.max_results > 1;
            self.find_closest_edges_optimized(target, &filter, &mut state);
        }

        state.collect_results(options)
    }

    /// Returns the distance from the target to the closest edge.
    #[inline]
    pub fn get_distance(&self, target: &dyn Target) -> ChordAngle {
        self.find_closest_edge(target).distance
    }

    /// Returns true if the distance to the target is less than `limit`.
    #[inline]
    pub fn is_distance_less(&self, target: &dyn Target, limit: ChordAngle) -> bool {
        let opts = Options {
            max_results: 1,
            max_distance: limit,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        let result = self.find_closest_edge_with_options(target, &opts);
        !result.is_empty()
    }

    /// Returns true if the distance to the target is less than or equal to
    /// `limit`.
    pub fn is_distance_less_or_equal(&self, target: &dyn Target, limit: ChordAngle) -> bool {
        let mut opts = Options {
            max_results: 1,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        opts.inclusive_max_distance(limit);
        let result = self.find_closest_edge_with_options(target, &opts);
        !result.is_empty()
    }

    /// Like [`is_distance_less_or_equal`](Self::is_distance_less_or_equal)
    /// but `limit` is increased by the maximum error in distance computation,
    /// ensuring all truly-within-limit edges are found.
    pub fn is_conservative_distance_less_or_equal(
        &self,
        target: &dyn Target,
        limit: ChordAngle,
    ) -> bool {
        let mut opts = Options {
            max_results: 1,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        opts.conservative_max_distance(limit);
        let result = self.find_closest_edge_with_options(target, &opts);
        !result.is_empty()
    }

    /// Calls `visitor` with the closest edges in order of increasing distance.
    /// The visitor returns `true` to continue or `false` to stop.
    pub fn visit_closest_edges(
        &self,
        target: &dyn Target,
        options: &Options,
        mut visitor: impl FnMut(&Result) -> ControlFlow<()>,
    ) {
        let results = self.find_closest_edges(target, options);
        for r in &results {
            if visitor(r).is_break() {
                break;
            }
        }
    }

    /// Calls `visitor` with the closest edge of each distinct shape,
    /// in order of increasing distance. Each shape appears at most once.
    pub fn visit_closest_shapes(
        &self,
        target: &dyn Target,
        options: &Options,
        mut visitor: impl FnMut(&Result) -> ControlFlow<()>,
    ) {
        // Cache the last accepted shape_id to skip the HashSet lookup for
        // consecutive results from the same shape (matches C++ optimization).
        let mut last_shape = ShapeId(-1);
        let mut seen_shapes = HashSet::new();
        self.visit_closest_edges(target, options, |result| {
            let shape_id = result.shape_id;
            if shape_id != last_shape && seen_shapes.insert(shape_id) {
                last_shape = shape_id;
                visitor(result)
            } else {
                ControlFlow::Continue(())
            }
        });
    }

    /// Returns the edge corresponding to the given result.
    pub fn get_edge(&self, result: &Result) -> Option<Edge> {
        if result.is_empty() || result.is_interior() {
            return None;
        }
        self.index
            .shape(result.shape_id)
            .map(|shape| shape.edge(result.edge_id as usize))
    }

    /// Projects a point onto the closest edge and returns the projected point.
    pub fn project(&self, point: Point, result: &Result) -> Point {
        if result.is_empty() || result.is_interior() {
            return point;
        }
        if let Some(edge) = self.get_edge(result) {
            edge_distances::project(point, edge.v0, edge.v1)
        } else {
            point
        }
    }

    /// Counts the total number of edges in the index, stopping early at
    /// `limit` (matches C++ `CountEdgesUpTo`).
    fn count_edges_up_to(&self, limit: i32) -> i32 {
        let mut count = 0i32;
        for id in 0..self.index.num_shape_ids() {
            if let Some(shape) = self.index.shape(id as i32) {
                count = count.saturating_add(shape.num_edges() as i32);
                if count >= limit {
                    return count;
                }
            }
        }
        count
    }

    /// Brute force: check every edge in the index.
    fn find_closest_edges_brute_force(
        &self,
        target: &dyn Target,
        filter: &ShapeFilter<'_>,
        state: &mut QueryState,
    ) {
        for shape_id in (0..self.index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            if let Some(f) = filter
                && !f(shape_id)
            {
                continue;
            }
            for edge_id in 0..shape.num_edges() {
                let edge = shape.edge(edge_id);
                let (dist, updated) =
                    target.update_distance_to_edge(edge.v0, edge.v1, state.distance_limit);
                if updated {
                    state.add_result(Result {
                        distance: dist,
                        shape_id,
                        edge_id: edge_id as i32,
                    });
                }
            }
        }
    }

    /// Compute the top-level index covering: a small set of cells (at most 6)
    /// that cover all index cells. Returns `(cell_ids, is_leaf)` where
    /// `is_leaf[i]` is true if `cell_ids[i]` is an actual index cell.
    /// Matches C++ `InitCovering`.
    fn init_covering(&self) -> (Vec<CellId>, Vec<bool>) {
        let mut covering = Vec::with_capacity(6);
        let mut is_leaf = Vec::with_capacity(6);

        let first_iter = self.index.iter();
        if first_iter.done() {
            return (covering, is_leaf);
        }
        let first_id = first_iter.cell_id();

        // Find the last cell by seeking to the maximum possible cell ID.
        let mut last_iter = self.index.iter();
        last_iter.seek(CellId::sentinel());
        if !last_iter.prev() {
            return (covering, is_leaf);
        }
        let last_id = last_iter.cell_id();

        if first_id == last_id {
            // Single index cell.
            covering.push(first_id);
            is_leaf.push(true);
            return (covering, is_leaf);
        }

        // Multiple cells. Find common ancestor level + 1.
        let level = first_id
            .common_ancestor_level(last_id)
            .map_or(Level::MIN, |l| l + 1u8);

        let last_parent = last_id.parent_at_level(level);
        let mut id = first_id.parent_at_level(level);

        let mut next = self.index.iter();

        while id != last_parent {
            // Skip top-level cells that don't contain any index cells.
            if id.range_max() >= next.cell_id() {
                let cell_first = next.cell_id();
                next.seek(id.range_max().next());
                // Find the last cell in this range by going back.
                let mut tmp = self.index.iter();
                tmp.seek(id.range_max());
                if tmp.done() || tmp.cell_id() > id.range_max() {
                    if tmp.prev() {
                        Self::add_initial_range(
                            cell_first,
                            tmp.cell_id(),
                            &mut covering,
                            &mut is_leaf,
                        );
                    }
                } else {
                    Self::add_initial_range(cell_first, tmp.cell_id(), &mut covering, &mut is_leaf);
                }
            }
            id = id.next();
        }
        // Add the last range (from current next position to last cell).
        if !next.done() {
            Self::add_initial_range(next.cell_id(), last_id, &mut covering, &mut is_leaf);
        }

        (covering, is_leaf)
    }

    /// Add a covering entry for the range [first, last] (inclusive cell IDs).
    fn add_initial_range(
        first: CellId,
        last: CellId,
        covering: &mut Vec<CellId>,
        is_leaf: &mut Vec<bool>,
    ) {
        if first == last {
            covering.push(first);
            is_leaf.push(true);
        } else if let Some(level) = first.common_ancestor_level(last) {
            covering.push(first.parent_at_level(level));
            is_leaf.push(false);
        }
    }

    /// Optimized: use the `S2ShapeIndex` cell structure with a priority queue,
    /// hierarchical cell splitting, and search disc covering.
    /// Matches C++ `FindClosestEdgesOptimized` + `InitQueue`.
    fn find_closest_edges_optimized(
        &self,
        target: &dyn Target,
        filter: &ShapeFilter<'_>,
        state: &mut QueryState,
    ) {
        let (index_covering, index_is_leaf) = self.init_covering();
        if index_covering.is_empty() {
            return;
        }

        let mut queue: BinaryHeap<QueueEntry> = BinaryHeap::new();
        let mut iter = self.index.iter();

        // If searching for just one result, try the cell containing the
        // target center first (C++ InitQueue optimization).
        let cap = target.cap_bound();
        if !cap.is_empty()
            && state.max_results == 1
            && iter.locate_point(cap.center())
            && let Some(cell) = iter.index_cell()
        {
            self.process_edges(cell, target, filter, state);
            if state.distance_limit == ChordAngle::ZERO {
                return;
            }
        }

        if state.distance_limit == ChordAngle::INFINITY {
            // No distance limit: add all top-level covering cells.
            for (i, &cell_id) in index_covering.iter().enumerate() {
                self.process_or_enqueue(
                    cell_id,
                    index_is_leaf[i],
                    target,
                    filter,
                    state,
                    &mut queue,
                    &mut iter,
                );
            }
        } else {
            // Compute a covering of the search disc and intersect with the
            // index covering. This prunes cells that are too far away.
            let radius = cap.angle_radius() + state.distance_limit.to_angle();
            let search_cap = Cap::from_center_angle(cap.center(), radius);

            use crate::s2::region_coverer::RegionCoverer;
            let coverer = RegionCoverer::new().max_cells(4);
            let search_covering = coverer.fast_covering(&search_cap);

            let index_cu = CellUnion::from_cell_ids(index_covering.clone());
            let initial_cells = index_cu.intersection(&search_covering);

            // Process the intersection cells, matching C++ InitQueue logic.
            let mut j = 0usize;
            let mut i = 0usize;
            let ids = initial_cells.cell_ids();
            while i < ids.len() {
                let id_i = ids[i];
                // Find the top-level covering cell that contains this initial cell.
                while j < index_covering.len() && index_covering[j].range_max() < id_i {
                    j += 1;
                }
                if j >= index_covering.len() {
                    break;
                }
                let id_j = index_covering[j];
                if id_i == id_j {
                    // This initial cell IS one of the top-level cells.
                    self.process_or_enqueue(
                        id_j,
                        index_is_leaf[j],
                        target,
                        filter,
                        state,
                        &mut queue,
                        &mut iter,
                    );
                    i += 1;
                    j += 1;
                } else {
                    // This initial cell is a descendant of a top-level cell.
                    let r = iter.locate_cell_id(id_i);
                    match r {
                        crate::s2::shape_index::CellRelation::Indexed => {
                            let iter_id = iter.cell_id();
                            self.process_or_enqueue(
                                iter_id, true, target, filter, state, &mut queue, &mut iter,
                            );
                            let last_id = iter_id.range_max();
                            i += 1;
                            while i < ids.len() && ids[i] <= last_id {
                                i += 1;
                            }
                        }
                        crate::s2::shape_index::CellRelation::Subdivided => {
                            self.process_or_enqueue(
                                id_i, false, target, filter, state, &mut queue, &mut iter,
                            );
                            i += 1;
                        }
                        crate::s2::shape_index::CellRelation::Disjoint => {
                            i += 1;
                        }
                    }
                }
            }
        }

        // Main loop: process cells in order of increasing distance, splitting
        // non-leaf cells into their four children (hierarchical search).
        while let Some(entry) = queue.pop() {
            if entry.distance >= state.distance_limit {
                break;
            }
            if entry.is_index_cell {
                // This is an actual index cell — process its edges.
                if let Some(index_cell) = self.index.cell(entry.cell_id) {
                    self.process_edges(index_cell, target, filter, state);
                }
            } else {
                // Split this ancestor cell into 4 children.
                // Use 2 seeks instead of 4 (matches C++ trick).
                let id = entry.cell_id;
                let children = id.children();

                // Seek to child(1).range_min and check children 1 and 0.
                iter.seek(children[1].range_min());
                if !iter.done() && iter.cell_id() <= children[1].range_max() {
                    self.process_or_enqueue_at(
                        children[1],
                        target,
                        filter,
                        state,
                        &mut queue,
                        &mut iter,
                    );
                }
                if iter.prev() && iter.cell_id() >= id.range_min() {
                    self.process_or_enqueue_at(
                        children[0],
                        target,
                        filter,
                        state,
                        &mut queue,
                        &mut iter,
                    );
                }

                // Seek to child(3).range_min and check children 3 and 2.
                iter.seek(children[3].range_min());
                if !iter.done() && iter.cell_id() <= id.range_max() {
                    self.process_or_enqueue_at(
                        children[3],
                        target,
                        filter,
                        state,
                        &mut queue,
                        &mut iter,
                    );
                }
                if iter.prev() && iter.cell_id() >= children[2].range_min() {
                    self.process_or_enqueue_at(
                        children[2],
                        target,
                        filter,
                        state,
                        &mut queue,
                        &mut iter,
                    );
                }
            }
        }
    }

    /// Decide whether to process a cell immediately or enqueue it.
    /// `is_index_cell` indicates whether `id` is a known leaf index cell.
    fn process_or_enqueue(
        &self,
        id: CellId,
        is_index_cell: bool,
        target: &dyn Target,
        filter: &ShapeFilter<'_>,
        state: &mut QueryState,
        queue: &mut BinaryHeap<QueueEntry>,
        _iter: &mut crate::s2::shape_index::ShapeIndexIterator<'_>,
    ) {
        if is_index_cell {
            if let Some(index_cell) = self.index.cell(id) {
                let num_edges: usize = index_cell.shapes.iter().map(|c| c.edges.len()).sum();
                if num_edges == 0 {
                    return;
                }
                if num_edges < MIN_EDGES_TO_ENQUEUE {
                    self.process_edges(index_cell, target, filter, state);
                    return;
                }
                // Check shape filter at cell level.
                if let Some(f) = filter
                    && !index_cell.shapes.iter().any(|c| f(c.shape_id))
                {
                    return;
                }
            } else {
                return;
            }
        }

        // Compute distance to cell and enqueue if within limit.
        let cell = Cell::from_cell_id(id);
        let (dist, updated) = target.update_distance_to_cell(&cell, state.distance_limit);
        if updated {
            let dist = if state.use_conservative_cell_distance {
                // Subtract max_error to ensure distance is a lower bound.
                ChordAngle::from_length2((dist.length2() - state.max_error.length2()).max(0.0))
            } else {
                dist
            };
            queue.push(QueueEntry {
                distance: dist,
                cell_id: id,
                is_index_cell,
            });
        }
    }

    /// Like `process_or_enqueue` but determines `is_index_cell` from the
    /// iterator position. Called during hierarchical cell splitting.
    fn process_or_enqueue_at(
        &self,
        id: CellId,
        target: &dyn Target,
        filter: &ShapeFilter<'_>,
        state: &mut QueryState,
        queue: &mut BinaryHeap<QueueEntry>,
        iter: &mut crate::s2::shape_index::ShapeIndexIterator<'_>,
    ) {
        let is_index_cell = !iter.done() && iter.cell_id() == id;
        self.process_or_enqueue(id, is_index_cell, target, filter, state, queue, iter);
    }

    /// Process all edges in an index cell.
    fn process_edges(
        &self,
        cell: &crate::s2::shape_index::ShapeIndexCell,
        target: &dyn Target,
        filter: &ShapeFilter<'_>,
        state: &mut QueryState,
    ) {
        for clipped in &cell.shapes {
            let shape_id = clipped.shape_id;
            if let Some(f) = filter
                && !f(shape_id)
            {
                continue;
            }
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            for &edge_id in &clipped.edges {
                // Skip already-tested edges to avoid duplicates.
                if state.avoid_duplicates && !state.tested_edges.insert((shape_id, edge_id)) {
                    continue;
                }
                let edge = shape.edge(edge_id as usize);
                let (dist, updated) =
                    target.update_distance_to_edge(edge.v0, edge.v1, state.distance_limit);
                if updated {
                    state.add_result(Result {
                        distance: dist,
                        shape_id,
                        edge_id,
                    });
                }
            }
        }
    }
}

/// Minimum edges in a cell before we enqueue it rather than processing
/// immediately (matches C++ `kMinEdgesToEnqueue`).
const MIN_EDGES_TO_ENQUEUE: usize = 10;

// ─── QueueEntry ─────────────────────────────────────────────────────────

/// Priority queue entry for the optimized search. Ordered by distance
/// (min-heap via reverse `Ord`).
struct QueueEntry {
    distance: ChordAngle,
    cell_id: CellId,
    /// True if this is a known index cell (process edges directly).
    /// False if this is an ancestor cell (split into children).
    is_index_cell: bool,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}
impl Eq for QueueEntry {}
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap (BinaryHeap is a max-heap).
        other
            .distance
            .length2()
            .partial_cmp(&self.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

// ─── QueryState ─────────────────────────────────────────────────────────

/// Per-query mutable state. Corresponds to the member variables of
/// C++ `S2ClosestEdgeQueryBase` that are reset for each query.
struct QueryState {
    /// The distance beyond which candidates are ignored.
    distance_limit: ChordAngle,
    /// `max_results` from options (cached for convenience).
    max_results: i32,
    /// `max_error` from options.
    max_error: ChordAngle,
    /// Whether cell distances should be conservatively adjusted.
    use_conservative_cell_distance: bool,
    /// Whether to track tested edges to avoid duplicates.
    avoid_duplicates: bool,

    // --- Result storage (three strategies matching C++) ---
    /// For `max_results == 1`: the single best result.
    result_singleton: Result,
    /// For `max_results == i32::MAX`: append all, sort/unique at end.
    result_vector: Vec<Result>,
    /// For `1 < max_results < i32::MAX`: maintain sorted set, prune as we go.
    result_set: BTreeSet<Result>,

    /// Set of already-tested `(shape_id, edge_id)` when `avoid_duplicates`.
    tested_edges: HashSet<(ShapeId, i32)>,
}

impl QueryState {
    fn new(options: &Options) -> Self {
        QueryState {
            distance_limit: options.max_distance,
            max_results: options.max_results,
            max_error: options.max_error,
            use_conservative_cell_distance: false,
            avoid_duplicates: false,
            result_singleton: Result::empty(),
            result_vector: Vec::new(),
            result_set: BTreeSet::new(),
            tested_edges: HashSet::new(),
        }
    }

    /// Add a result, updating the distance limit as appropriate.
    /// Uses one of three storage strategies depending on `max_results`.
    fn add_result(&mut self, result: Result) {
        if self.max_results == 1 {
            // Singleton strategy: keep only the closest edge.
            if result.shape_id >= 0 {
                self.result_singleton = result;
                self.distance_limit = self.distance_limit_for(self.result_singleton.distance);
            }
        } else if self.max_results == i32::MAX {
            // Unlimited strategy: just append. Sort/unique at end.
            self.result_vector.push(result);
        } else {
            // Bounded strategy: maintain BTreeSet, prune worst when full.
            self.result_set.insert(result);
            let size = self.result_set.len() as i32;
            if size > self.max_results {
                // Remove the worst (farthest) result.
                if let Some(worst) = self.result_set.iter().next_back().cloned() {
                    self.result_set.remove(&worst);
                }
            }
            if self.result_set.len() as i32 >= self.max_results {
                // Update distance limit to the worst remaining result.
                if let Some(worst) = self.result_set.iter().next_back() {
                    self.distance_limit = self.distance_limit_for(worst.distance);
                }
            }
        }
    }

    /// Compute the effective distance limit given a result distance,
    /// accounting for `max_error`.
    fn distance_limit_for(&self, distance: ChordAngle) -> ChordAngle {
        if self.max_error > ChordAngle::ZERO {
            ChordAngle::from_length2((distance.length2() - self.max_error.length2()).max(0.0))
        } else {
            distance
        }
    }

    /// Collect all results into a sorted `Vec`.
    fn collect_results(self, options: &Options) -> Vec<Result> {
        if options.max_results == 1 {
            if self.result_singleton.shape_id >= 0 {
                vec![self.result_singleton]
            } else {
                Vec::new()
            }
        } else if options.max_results == i32::MAX {
            let mut results = self.result_vector;
            results.sort();
            results.dedup();
            results
        } else {
            self.result_set.into_iter().collect()
        }
    }
}

#[cfg(test)]
#[expect(
    clippy::field_reassign_with_default,
    reason = "clearer than a single struct literal with many fields"
)]
mod tests {
    use super::*;
    use crate::s2::{LatLng, Loop, Polygon};

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn make_polyline_index(vertices: Vec<Point>) -> ShapeIndex {
        let mut index = ShapeIndex::new();
        index.add(Box::new(crate::s2::polyline::Polyline::new(vertices)));
        index.build();
        index
    }

    #[test]
    fn test_closest_edge_to_point_simple() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));
        let result = query.find_closest_edge(&target);

        assert!(!result.is_empty());
        assert_eq!(result.shape_id, 0);
        assert_eq!(result.edge_id, 0);
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            (dist_degrees - 1.0).abs() < 0.1,
            "distance = {dist_degrees} degrees, expected ~1.0"
        );
    }

    #[test]
    fn test_closest_edge_to_point_on_edge() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));
        let result = query.find_closest_edge(&target);

        assert!(!result.is_empty());
        assert!(
            result.distance.to_angle().radians() < 1e-10,
            "distance should be ~0 for point on edge"
        );
    }

    #[test]
    fn test_get_distance() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));
        let dist = query.get_distance(&target);
        assert!(dist.to_angle().degrees() < 2.0);
        assert!(dist.to_angle().degrees() > 0.5);
    }

    #[test]
    fn test_is_distance_less() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));
        // 1 degree away, should be less than 5 degrees.
        assert!(query.is_distance_less(&target, ChordAngle::from_angle(Angle::from_degrees(5.0))));
        // Should not be less than 0.01 degrees.
        assert!(
            !query.is_distance_less(&target, ChordAngle::from_angle(Angle::from_degrees(0.01)))
        );
    }

    #[test]
    fn test_project_point() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));
        let result = query.find_closest_edge(&target);
        let projected = query.project(p(1.0, 5.0), &result);

        let ll = LatLng::from_point(projected);
        assert!(
            ll.lat.degrees().abs() < 0.01,
            "projected lat = {}, expected ~0",
            ll.lat.degrees()
        );
        assert!(
            (ll.lng.degrees() - 5.0).abs() < 0.5,
            "projected lng = {}, expected ~5",
            ll.lng.degrees()
        );
    }

    #[test]
    fn test_empty_index() {
        let index = ShapeIndex::new();
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let result = query.find_closest_edge(&target);
        assert!(result.is_empty());
    }

    #[test]
    fn test_multiple_shapes() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(10.0, 0.0),
            p(10.0, 10.0),
        ])));
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(1.0, 0.0),
            p(1.0, 10.0),
        ])));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));
        let result = query.find_closest_edge(&target);

        // The second polyline (shape_id=1) is closer.
        assert!(!result.is_empty());
        assert_eq!(result.shape_id, 1);
    }

    #[test]
    fn test_polygon_interior() {
        let poly = Polygon::from_loops(vec![Loop::new(vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ])]);

        let query = ClosestEdgeQuery::new(poly.shape_index());
        let target = PointTarget::new(p(0.0, 0.0));
        let result = query.find_closest_edge(&target);

        assert!(!result.is_empty());
        assert_eq!(result.distance, ChordAngle::ZERO);
    }

    #[test]
    fn test_edge_target() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = EdgeTarget::new(p(2.0, 3.0), p(2.0, 7.0));
        let result = query.find_closest_edge(&target);

        assert!(!result.is_empty());
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            (dist_degrees - 2.0).abs() < 0.5,
            "distance = {dist_degrees} degrees, expected ~2.0"
        );
    }

    #[test]
    fn test_brute_force_vs_optimized() {
        // Create an index with enough edges to trigger optimized path.
        let mut index = ShapeIndex::new();
        for i in 0..20 {
            let lat = f64::from(i) * 2.0 - 20.0;
            index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
                p(lat, -10.0),
                p(lat, 10.0),
            ])));
        }
        index.build();

        let target = PointTarget::new(p(0.0, 0.0));

        // Brute force.
        let mut opts_bf = Options::default();
        opts_bf.max_results = 1;
        opts_bf.use_brute_force = true;
        let query = ClosestEdgeQuery::new(&index);
        let result_bf = query.find_closest_edge_with_options(&target, &opts_bf);

        // Default (may use optimized).
        let mut opts = Options::default();
        opts.max_results = 1;
        let result_opt = query.find_closest_edge_with_options(&target, &opts);

        assert_eq!(result_bf.shape_id, result_opt.shape_id);
        assert!((result_bf.distance.length2() - result_opt.distance.length2()).abs() < 1e-10);
    }

    #[test]
    fn test_no_edges_returns_empty() {
        // Empty index: find_closest_edge should return empty result with infinity distance.
        let index = ShapeIndex::new();
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let result = query.find_closest_edge(&target);
        assert!(result.is_empty());
        assert!(result.distance >= ChordAngle::INFINITY);
        assert_eq!(result.shape_id, -1);
        assert_eq!(result.edge_id, -1);
    }

    #[test]
    fn test_distance_equal_to_limit() {
        // Create a polyline at latitude 1 degree, query from equator.
        let index = make_polyline_index(vec![p(1.0, 0.0), p(1.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));

        // The distance is ~1 degree. is_distance_less with limit exactly 1 degree
        // should return false (distance is NOT strictly less than limit).
        let dist = query.get_distance(&target);
        let actual_degrees = dist.to_angle().degrees();

        // With a limit slightly larger than actual distance, should be true.
        assert!(query.is_distance_less(
            &target,
            ChordAngle::from_angle(Angle::from_degrees(actual_degrees + 0.1))
        ));
        // With a limit slightly smaller, should be false.
        assert!(!query.is_distance_less(
            &target,
            ChordAngle::from_angle(Angle::from_degrees(actual_degrees - 0.1))
        ));
    }

    #[test]
    fn test_find_closest_edges_multiple() {
        // Create multiple parallel polylines and find the 3 closest.
        let mut index = ShapeIndex::new();
        for i in 0..5 {
            let lat = f64::from(i) * 2.0;
            index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
                p(lat, -10.0),
                p(lat, 10.0),
            ])));
        }
        index.build();

        let mut opts = Options::default();
        opts.max_results = 3;
        let target = PointTarget::new(p(0.5, 0.0));
        let query = ClosestEdgeQuery::new(&index);
        let results = query.find_closest_edges(&target, &opts);

        assert!(results.len() <= 3, "should return at most 3 results");
        assert!(!results.is_empty(), "should return at least 1 result");
        // Results should be sorted by distance.
        for i in 1..results.len() {
            assert!(
                results[i].distance >= results[i - 1].distance,
                "results not sorted by distance"
            );
        }
    }

    #[test]
    fn test_point_on_vertex() {
        // Query point exactly on a vertex should give distance 0.
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let result = query.find_closest_edge(&target);
        assert!(!result.is_empty());
        assert!(
            result.distance.to_angle().radians() < 1e-10,
            "distance to vertex should be ~0"
        );
    }

    #[test]
    fn test_target_point_inside_indexed_polygon() {
        // C++ TargetPointInsideIndexedPolygon — target point inside a polygon
        // in a mixed index (polyline + polygon). The polygon interior should
        // be found with distance 0.
        use crate::s2::lax_polygon::LaxPolygon;
        let mut index = ShapeIndex::new();
        // Shape 0: polyline loop (no interior)
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 5.0),
            p(5.0, 5.0),
            p(5.0, 0.0),
        ])));
        // Shape 1: polygon (has interior)
        index.add(Box::new(LaxPolygon::from_loops(&[&[
            p(0.0, 10.0),
            p(0.0, 15.0),
            p(5.0, 15.0),
            p(5.0, 10.0),
        ]])));
        index.build();

        let mut opts = Options::default();
        opts.include_interiors = true;
        opts.max_distance = ChordAngle::from_degrees(1.0);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(2.0, 12.0));
        let results = query.find_closest_edges(&target, &opts);
        assert_eq!(results.len(), 1, "should find exactly 1 interior result");
        assert_eq!(results[0].distance, ChordAngle::ZERO);
        assert_eq!(results[0].shape_id, 1, "should be polygon shape");
        assert_eq!(results[0].edge_id, -1, "interior result has edge_id=-1");
        assert!(results[0].is_interior());
        assert!(!results[0].is_empty());
    }

    #[test]
    fn test_target_point_outside_indexed_polygon() {
        // C++ TargetPointOutsideIndexedPolygon — target point inside a polyline
        // loop (which has no interior). With include_interiors=true and
        // max_distance=1°, no results should be found since the nearest edge
        // is more than 1° away.
        use crate::s2::lax_polygon::LaxPolygon;
        let mut index = ShapeIndex::new();
        // Shape 0: polyline loop (no interior)
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 5.0),
            p(5.0, 5.0),
            p(5.0, 0.0),
        ])));
        // Shape 1: polygon (has interior, but far away)
        index.add(Box::new(LaxPolygon::from_loops(&[&[
            p(0.0, 10.0),
            p(0.0, 15.0),
            p(5.0, 15.0),
            p(5.0, 10.0),
        ]])));
        index.build();

        let mut opts = Options::default();
        opts.include_interiors = true;
        opts.max_distance = ChordAngle::from_degrees(1.0);
        let query = ClosestEdgeQuery::new(&index);
        // Point inside the polyline loop, but polylines have no interior.
        let target = PointTarget::new(p(2.0, 2.0));
        let results = query.find_closest_edges(&target, &opts);
        assert_eq!(
            results.len(),
            0,
            "point in polyline loop (no interior) should have no results"
        );
    }

    #[test]
    fn test_is_distance_less_or_equal() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));

        // Distance is ~1 degree. is_distance_less_or_equal(2°) → true.
        assert!(query.is_distance_less_or_equal(&target, ChordAngle::from_degrees(2.0)));
        // is_distance_less_or_equal(0.5°) → false.
        assert!(!query.is_distance_less_or_equal(&target, ChordAngle::from_degrees(0.5)));
    }

    #[test]
    fn test_is_conservative_distance_less_or_equal() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));

        // Conservative version should also find edges within 2°.
        assert!(
            query.is_conservative_distance_less_or_equal(&target, ChordAngle::from_degrees(2.0))
        );
        // But not within 0.01°.
        assert!(
            !query.is_conservative_distance_less_or_equal(&target, ChordAngle::from_degrees(0.01))
        );
    }

    #[test]
    fn test_inclusive_max_distance() {
        let mut opts = Options::default();
        opts.inclusive_max_distance(ChordAngle::from_degrees(5.0));
        // inclusive = limit.successor()
        assert!(opts.max_distance > ChordAngle::from_degrees(5.0));
    }

    #[test]
    fn test_conservative_max_distance() {
        let mut opts = Options::default();
        opts.conservative_max_distance(ChordAngle::from_degrees(5.0));
        // Conservative should be >= inclusive.
        let mut inclusive = Options::default();
        inclusive.inclusive_max_distance(ChordAngle::from_degrees(5.0));
        assert!(opts.max_distance >= inclusive.max_distance);
    }

    #[test]
    fn test_shape_filter() {
        let mut index = ShapeIndex::new();
        // Shape 0: polyline near (0,0)
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 1.0),
        ])));
        // Shape 1: polyline near (10,0)
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(10.0, 0.0),
            p(10.0, 1.0),
        ])));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.5));

        // Without filter: closest is shape 0.
        let result = query.find_closest_edge(&target);
        assert_eq!(result.shape_id, 0);

        // With filter excluding shape 0: closest should be shape 1.
        let opts = Options {
            max_results: 1,
            ..Options::default()
        };
        let results =
            query.find_closest_edges_filtered(&target, &opts, Some(&|shape_id| shape_id != 0));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].shape_id, 1);
    }

    #[test]
    fn test_shape_index_target() {
        // Query index: polyline near equator.
        let mut query_index = ShapeIndex::new();
        query_index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 10.0),
        ])));
        query_index.build();

        // Target index: polyline slightly north.
        let mut target_index = ShapeIndex::new();
        target_index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(1.0, 0.0),
            p(1.0, 10.0),
        ])));
        target_index.build();

        let query = ClosestEdgeQuery::new(&query_index);
        let target = ShapeIndexTarget::new(&target_index);
        let result = query.find_closest_edge(&target);

        assert!(!result.is_empty());
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            (dist_degrees - 1.0).abs() < 0.1,
            "distance = {dist_degrees} degrees, expected ~1.0"
        );
    }

    #[test]
    fn test_visit_closest_edges() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 5.0), p(0.0, 10.0)]);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(1.0, 5.0));
        let opts = Options {
            max_results: 10,
            ..Options::default()
        };

        let mut visited = Vec::new();
        query.visit_closest_edges(&target, &opts, |result| {
            visited.push(result.edge_id);
            ControlFlow::Continue(())
        });
        assert!(!visited.is_empty());
    }

    #[test]
    fn test_visit_closest_shapes() {
        let mut index = ShapeIndex::new();
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 5.0),
        ])));
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 7.0),
            p(0.0, 12.0),
        ])));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 3.0));
        let opts = Options {
            max_results: 100,
            ..Options::default()
        };

        let mut shape_ids = Vec::new();
        query.visit_closest_shapes(&target, &opts, |result| {
            shape_ids.push(result.shape_id);
            ControlFlow::Continue(())
        });
        // Should see each shape at most once.
        let unique: HashSet<ShapeId> = shape_ids.iter().copied().collect();
        assert_eq!(shape_ids.len(), unique.len());
    }

    #[test]
    fn test_set_max_error_on_target() {
        let mut target = PointTarget::new(p(1.0, 2.0));
        // PointTarget doesn't use max_error (returns false).
        assert!(!target.set_max_error(ChordAngle::from_degrees(1.0)));
    }

    // ─── C++ port: additional edge cases ───────────────────────────────

    #[test]
    fn test_empty_target_optimized() {
        // C++: EmptyTargetOptimized
        // Ensure the optimized algorithm handles empty targets with a distance
        // limit.
        use crate::s2::Loop;

        let mut index = ShapeIndex::new();
        let loop_ = Loop::make_regular(
            Point::from_coords(1.0, 0.0, 0.0),
            Angle::from_radians(0.1),
            1000,
        );
        index.add(Box::new(loop_));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target_index = ShapeIndex::new();
        let target = ShapeIndexTarget::new(&target_index);
        let opts = Options {
            max_results: i32::MAX,
            max_distance: ChordAngle::from_angle(Angle::from_radians(1e-5)),
            ..Options::default()
        };
        let results = query.find_closest_edges(&target, &opts);
        assert!(results.is_empty());
    }

    #[test]
    fn test_empty_polygon_target() {
        // C++: EmptyPolygonTarget
        // Distances measured correctly to empty polygon targets.
        use crate::s2::text_format::make_index;

        let empty_idx = make_index("# # empty");
        let point_idx = make_index("1:1 # #");

        let target = ShapeIndexTarget::new(&empty_idx);

        // Empty index → empty target → infinity distance.
        let q1 = ClosestEdgeQuery::new(&empty_idx);
        let opts = Options {
            include_interiors: true,
            ..Options::default()
        };
        assert_eq!(
            q1.find_closest_edge_with_options(&target, &opts).distance,
            ChordAngle::INFINITY,
        );

        // Point index → empty target → infinity distance.
        let q2 = ClosestEdgeQuery::new(&point_idx);
        assert_eq!(
            q2.find_closest_edge_with_options(&target, &opts).distance,
            ChordAngle::INFINITY,
        );
    }

    #[test]
    fn test_full_lax_polygon_target() {
        // C++: FullLaxPolygonTarget
        // A full polygon target should contain everything at distance zero.
        use crate::s2::text_format::make_index;

        let full_idx = make_index("# # full");
        let point_idx = make_index("1:1 # #");

        let mut target = ShapeIndexTarget::new(&full_idx);
        target.include_interiors = true;
        let opts = Options {
            include_interiors: true,
            ..Options::default()
        };

        let q = ClosestEdgeQuery::new(&point_idx);
        let result = q.find_closest_edge_with_options(&target, &opts);
        // The point should be contained by the full polygon at distance zero.
        assert_eq!(result.distance, ChordAngle::ZERO);
    }

    #[test]
    fn test_target_polygon_containing_indexed_points() {
        // C++: TargetPolygonContainingIndexedPoints
        // Two points inside a polyline loop (no interior) and two points
        // inside a polygon. Only the polygon points should match.
        use crate::s2::text_format::make_index;

        let index = make_index("2:2 | 3:3 | 1:11 | 3:13 # #");
        let target_index = make_index("# 0:0, 0:5, 5:5, 5:0 # 0:10, 0:15, 5:15, 5:10");
        let mut target = ShapeIndexTarget::new(&target_index);
        target.include_interiors = true;

        let query = ClosestEdgeQuery::new(&index);
        let opts = Options {
            max_results: i32::MAX,
            max_distance: ChordAngle::from_angle(Angle::from_degrees(1.0)),
            ..Options::default()
        };
        let results = query.find_closest_edges(&target, &opts);
        assert_eq!(results.len(), 2, "expected 2 contained points");

        // Both results should be at distance zero (contained by polygon).
        for r in &results {
            assert_eq!(r.distance, ChordAngle::ZERO);
            assert_eq!(r.shape_id, 0); // All points are in shape 0
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Randomized brute-force vs. optimized tests — ported from C++
    // s2closest_edge_query_test.cc (CircleEdges / FractalEdges /
    // PointCloudEdges / ConservativeCellDistanceIsUsed).
    // ═══════════════════════════════════════════════════════════════════

    use crate::s1::Angle;
    use crate::s2::cap::Cap;
    use crate::s2::cell::Cell;
    use crate::s2::cell_id::CellId;
    use crate::s2::coords::MAX_CELL_LEVEL;
    use crate::s2::fractal::S2Fractal;
    use crate::s2::metric;
    use crate::s2::point_vector::PointVector;
    use crate::s2::testing::{frame_at, random_point, sample_point_from_cap};
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashSet;

    /// A factory trait that adds edges to a `ShapeIndex` within a cap.
    /// Ported from C++ `s2testing::ShapeIndexFactory`.
    trait ShapeIndexFactory {
        fn add_edges(&self, cap: &Cap, num_edges: usize, index: &mut ShapeIndex, rng: &mut StdRng);
    }

    /// Generates a regular loop that approximately fills the given cap.
    /// Regular loops are nearly the worst case for distance calculations,
    /// since many edges are nearly equidistant from any query point.
    struct RegularLoopFactory;
    impl ShapeIndexFactory for RegularLoopFactory {
        fn add_edges(
            &self,
            cap: &Cap,
            num_edges: usize,
            index: &mut ShapeIndex,
            _rng: &mut StdRng,
        ) {
            index.add(Box::new(Loop::make_regular(
                cap.center(),
                cap.angle_radius(),
                num_edges,
            )));
        }
    }

    /// Generates a fractal loop that approximately fills the given cap.
    struct FractalLoopFactory;
    impl ShapeIndexFactory for FractalLoopFactory {
        fn add_edges(&self, cap: &Cap, num_edges: usize, index: &mut ShapeIndex, rng: &mut StdRng) {
            let seed = rng.r#gen::<u64>();
            let mut fractal = S2Fractal::new(seed);
            fractal.level_for_approx_max_edges(num_edges as i32);
            let frame = frame_at(rng, cap.center());
            let frame_mat =
                crate::r3::matrix::Matrix3x3::from_cols(frame.0.0, frame.1.0, frame.2.0);
            let loop_ = fractal.make_loop(&frame_mat, cap.angle_radius());
            index.add(Box::new(loop_));
        }
    }

    /// Generates a cloud of points that approximately fills the given cap.
    struct PointCloudFactory;
    impl ShapeIndexFactory for PointCloudFactory {
        fn add_edges(&self, cap: &Cap, num_edges: usize, index: &mut ShapeIndex, rng: &mut StdRng) {
            let mut points = Vec::with_capacity(num_edges);
            for _ in 0..num_edges {
                points.push(sample_point_from_cap(rng, cap));
            }
            index.add(Box::new(PointVector::new(points)));
        }
    }

    /// Count total edges across all shapes in an index.
    fn count_edges(index: &ShapeIndex) -> usize {
        let mut total = 0;
        for id in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(id as i32) {
                total += shape.num_edges();
            }
        }
        total
    }

    /// `log_uniform(rng, lo, hi)` — value whose log2 is uniform in
    /// `[log2(lo), log2(hi)]`. Matches C++ `s2random::LogUniform`.
    fn log_uniform(rng: &mut StdRng, lo: f64, hi: f64) -> f64 {
        let lo_log = lo.log2();
        let hi_log = hi.log2();
        let v: f64 = rng.gen_range(lo_log..hi_log);
        v.exp2()
    }

    /// Check that `actual` results (from optimized search) are consistent
    /// with `expected` results (from brute force) given the query options.
    ///
    /// Ported from C++ `CheckDistanceResults` / `CheckResultSet`.
    #[expect(clippy::print_stderr, reason = "test diagnostic output")]
    fn check_distance_results(
        expected: &[Result],
        actual: &[Result],
        max_results: i32,
        max_distance: ChordAngle,
        max_error: ChordAngle,
    ) -> bool {
        const MAX_PRUNING_ERROR: f64 = 1e-15; // conservative bound on cell distance error

        // Both should be sorted by distance.
        for slice in [expected, actual] {
            for w in slice.windows(2) {
                if w[1].distance < w[0].distance {
                    return false;
                }
            }
        }

        let check_result_set = |x: &[Result], y: &[Result], label: &str| -> bool {
            // Compute the distance limit below which all results from y
            // should appear in x.
            let limit = if (x.len() as i32) < max_results {
                // Not limited by max_results → should have everything up
                // to max_distance (minus pruning error).
                if max_distance == ChordAngle::INFINITY {
                    ChordAngle::INFINITY
                } else {
                    ChordAngle::from_length2((max_distance.length2() - MAX_PRUNING_ERROR).max(0.0))
                }
            } else if !x.is_empty() {
                // Limited by max_results → everything within (farthest -
                // max_error - pruning_error) should be present.
                let back = x.last().unwrap().distance.length2();
                ChordAngle::from_length2((back - max_error.length2() - MAX_PRUNING_ERROR).max(0.0))
            } else {
                ChordAngle::ZERO
            };

            let mut ok = true;
            for yp in y {
                if yp.distance < limit {
                    let count = x
                        .iter()
                        .filter(|xp| xp.shape_id == yp.shape_id && xp.edge_id == yp.edge_id)
                        .count();
                    if count != 1 {
                        eprintln!(
                            "{label}: distance={:?}, shape={}, edge={}, count={count}",
                            yp.distance, yp.shape_id, yp.edge_id,
                        );
                        ok = false;
                    }
                }
            }
            ok
        };

        let ok1 = check_result_set(actual, expected, "Missing");
        let ok2 = check_result_set(expected, actual, "Extra");
        ok1 && ok2
    }

    /// Run a single randomized query and verify brute-force results match
    /// optimized results. Returns the brute-force closest result so the
    /// caller can also test `Project`.
    fn test_find_closest_edges(
        target: &dyn Target,
        query: &ClosestEdgeQuery<'_>,
        options: &Options,
        allowed_shapes: &HashSet<ShapeId>,
    ) -> Result {
        // Brute-force search.
        let mut bf_opts = options.clone();
        bf_opts.use_brute_force = true;
        let filter_fn = |id: ShapeId| allowed_shapes.contains(&id);
        let filter: ShapeFilter<'_> = if allowed_shapes.is_empty() {
            None
        } else {
            Some(&filter_fn)
        };
        let expected = query.find_closest_edges_filtered(target, &bf_opts, filter);

        // Verify allowed_shapes filter.
        if !allowed_shapes.is_empty() {
            for r in &expected {
                assert!(
                    allowed_shapes.contains(&r.shape_id),
                    "brute-force result from disallowed shape {}",
                    r.shape_id
                );
            }
        }

        // Verify brute-force result count.
        assert!(
            expected.len() <= options.max_results as usize,
            "too many results: {} > {}",
            expected.len(),
            options.max_results
        );
        if options.max_distance == ChordAngle::INFINITY && !options.include_interiors {
            let filter_count = if allowed_shapes.is_empty() {
                count_edges(query.index)
            } else {
                allowed_shapes
                    .iter()
                    .filter_map(|&id| query.index.shape(id))
                    .map(super::super::shape::Shape::num_edges)
                    .sum()
            };
            let min_expected = (options.max_results as usize).min(filter_count);
            assert_eq!(
                expected.len(),
                min_expected,
                "brute-force returned {} edges, expected {min_expected}",
                expected.len()
            );
        }
        for r in &expected {
            assert!(
                r.distance < options.max_distance,
                "result distance {:?} >= max_distance {:?}",
                r.distance,
                options.max_distance
            );
        }

        // Optimized search.
        let mut opt_opts = options.clone();
        opt_opts.use_brute_force = false;
        let actual = query.find_closest_edges_filtered(target, &opt_opts, filter);

        assert!(
            check_distance_results(
                &expected,
                &actual,
                options.max_results,
                options.max_distance,
                options.max_error,
            ),
            "max_results={}, max_distance={:?}, max_error={:?}",
            options.max_results,
            options.max_distance,
            options.max_error,
        );

        if expected.is_empty() {
            return Result::empty();
        }

        // Verify GetDistance() and IsDistanceLess() are consistent with
        // max_error. Note: when max_error > 0, expected[0].distance may
        // not be the true minimum.
        let filter3: ShapeFilter<'_> = if allowed_shapes.is_empty() {
            None
        } else {
            // Can't easily pass allowed_shapes here without lifetime issues.
            // Skip this sub-check for filtered queries.
            None
        };
        if allowed_shapes.is_empty() {
            let got_dist = query.get_distance(target);
            assert!(
                got_dist <= expected[0].distance + options.max_error,
                "GetDistance {:?} > min {:?} + error {:?}",
                got_dist,
                expected[0].distance,
                options.max_error,
            );

            // Verify IsDistanceLess / IsConservativeDistanceLessOrEqual
            // using a fresh brute-force single-result search to get the
            // tightest available minimum. C++ uses expected[0] directly,
            // but edge processing order may differ; a single-result
            // brute-force search gives a stronger bound.
            let bf1_opts = Options {
                max_results: 1,
                max_error: options.max_error,
                use_brute_force: true,
                ..Options::default()
            };
            let bf1 = query.find_closest_edge_with_options(target, &bf1_opts);
            let min_distance = bf1.distance;

            let _ = filter3; // suppress unused warning
            let too_close = min_distance - options.max_error;
            if too_close > ChordAngle::ZERO {
                assert!(
                    !query.is_distance_less(target, too_close),
                    "IsDistanceLess should be false for limit below min - error, \
                     min_dist={:?}, max_error={:?}, too_close={:?}",
                    min_distance,
                    options.max_error,
                    too_close,
                );
            }
            assert!(
                query.is_conservative_distance_less_or_equal(target, expected[0].distance),
                "IsConservativeDistanceLessOrEqual should be true"
            );
        }

        expected[0].clone()
    }

    /// C++ `kTestCapRadius = S2Testing::KmToAngle(10)` ≈ 0.00157 radians.
    fn test_cap_radius() -> Angle {
        // 10 km on Earth's surface.
        Angle::from_radians(10.0 / 6371.01)
    }

    /// Core test driver: build indexes using the factory, generate random
    /// queries, and verify brute-force vs optimized results match.
    ///
    /// Ported from C++ `TestWithIndexFactory`.
    fn test_with_closest_index_factory(
        factory: &dyn ShapeIndexFactory,
        num_indexes: usize,
        num_edges: usize,
        num_queries: usize,
        allowed_shapes: &HashSet<ShapeId>,
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);

        // Build indexes.
        let mut index_caps = Vec::with_capacity(num_indexes);
        let mut indexes = Vec::with_capacity(num_indexes);
        for _ in 0..num_indexes {
            let center = random_point(&mut rng);
            let cap = Cap::from_center_angle(center, test_cap_radius());
            let mut index = ShapeIndex::new();
            // Add at least two shapes (like C++).
            factory.add_edges(&cap, num_edges, &mut index, &mut rng);
            factory.add_edges(&cap, num_edges, &mut index, &mut rng);
            index.build();
            index_caps.push(cap);
            indexes.push(index);
        }

        for _ in 0..num_queries {
            let i_index = rng.gen_range(0..num_indexes);
            let index_cap = &index_caps[i_index];

            // Query points from an area ~4x larger than the geometry.
            let query_radius = 2.0 * index_cap.angle_radius().radians();
            let query_cap =
                Cap::from_center_angle(index_cap.center(), Angle::from_radians(query_radius));

            let mut opts = Options::default();

            // 80% of the time, limit max_results.
            if rng.gen_range(0..5) != 0 {
                opts.max_results = rng.gen_range(1..11);
            }
            // 2/3 of the time, set a distance limit.
            if rng.gen_range(0..3) != 0 {
                let frac: f64 = rng.gen_range(0.0..1.0);
                opts.max_distance =
                    ChordAngle::from_angle(Angle::from_radians(frac * query_radius));
            }
            // 50% of the time, set a max_error.
            if rng.gen_range(0..2) == 0 {
                let e = log_uniform(&mut rng, 1e-4, 1.0) * query_radius;
                opts.max_error = ChordAngle::from_angle(Angle::from_radians(e));
            }
            opts.include_interiors = rng.gen_range(0..2) == 0;

            let query = ClosestEdgeQuery::new(&indexes[i_index]);
            let target_type = rng.gen_range(0..4);

            match target_type {
                0 => {
                    // Point target.
                    let point = sample_point_from_cap(&mut rng, &query_cap);
                    let target = PointTarget::new(point);
                    let closest = test_find_closest_edges(&target, &query, &opts, allowed_shapes);
                    if !closest.distance.is_infinity() {
                        // Also test Project.
                        if let Some(_edge) = query.get_edge(&closest) {
                            let projected = query.project(point, &closest);
                            let proj_dist = point.chord_angle(projected);
                            let diff = (proj_dist.to_angle().radians()
                                - closest.distance.to_angle().radians())
                            .abs();
                            assert!(diff < 1e-10, "Project distance mismatch: {diff}");
                        }
                    }
                }
                1 => {
                    // Edge target.
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let edge_radius = log_uniform(&mut rng, 1e-4, 1.0) * query_radius;
                    let b_cap = Cap::from_center_angle(a, Angle::from_radians(edge_radius));
                    let b = sample_point_from_cap(&mut rng, &b_cap);
                    let target = EdgeTarget::new(a, b);
                    test_find_closest_edges(&target, &query, &opts, allowed_shapes);
                }
                2 => {
                    // Cell target.
                    let min_level = metric::MAX_DIAG.max_level(query_radius);
                    let level = Level::new(rng.gen_range(min_level.as_u8()..=MAX_CELL_LEVEL));
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let cell = Cell::from(CellId::from_point(&a).parent_at_level(level));
                    let target = CellTarget::new(cell);
                    test_find_closest_edges(&target, &query, &opts, allowed_shapes);
                }
                3 => {
                    // ShapeIndex target (another pre-built index).
                    let j_index = rng.gen_range(0..num_indexes);
                    let target = ShapeIndexTarget::new(&indexes[j_index]);
                    test_find_closest_edges(&target, &query, &opts, allowed_shapes);
                }
                _ => unreachable!(),
            }
        }
    }

    const NUM_INDEXES: usize = 50;
    const NUM_EDGES: usize = 100;
    const NUM_QUERIES: usize = 200;

    // ─── CircleEdges / FractalEdges / PointCloudEdges ────────────────

    #[test]
    fn test_circle_edges() {
        // C++: CircleEdges with AllowedShapeTests(-1)
        // Regular loops are nearly the worst case for distance calculations.
        test_with_closest_index_factory(
            &RegularLoopFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &HashSet::new(),
            0x5eed_c1c1,
        );
    }

    #[test]
    fn test_circle_edges_filtered() {
        // C++: CircleEdges with AllowedShapeTests(0) — only shape 0
        let allowed: HashSet<ShapeId> = [ShapeId(0)].into_iter().collect();
        test_with_closest_index_factory(
            &RegularLoopFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &allowed,
            0x5eed_c1c2,
        );
    }

    #[test]
    fn test_fractal_edges() {
        // C++: FractalEdges with AllowedShapeTests(-1)
        test_with_closest_index_factory(
            &FractalLoopFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &HashSet::new(),
            0x5eed_f4ac,
        );
    }

    #[test]
    fn test_fractal_edges_filtered() {
        // C++: FractalEdges with AllowedShapeTests(0)
        let allowed: HashSet<ShapeId> = [ShapeId(0)].into_iter().collect();
        test_with_closest_index_factory(
            &FractalLoopFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &allowed,
            0x5eed_f4ad,
        );
    }

    #[test]
    fn test_point_cloud_edges() {
        // C++: PointCloudEdges with AllowedShapeTests(-1)
        test_with_closest_index_factory(
            &PointCloudFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &HashSet::new(),
            0x5eed_9c1d,
        );
    }

    #[test]
    fn test_point_cloud_edges_filtered() {
        // C++: PointCloudEdges with AllowedShapeTests(0)
        let allowed: HashSet<ShapeId> = [ShapeId(0)].into_iter().collect();
        test_with_closest_index_factory(
            &PointCloudFactory,
            NUM_INDEXES,
            NUM_EDGES,
            NUM_QUERIES,
            &allowed,
            0x5eed_9c1e,
        );
    }

    #[test]
    fn test_conservative_cell_distance_is_used() {
        // C++: ConservativeCellDistanceIsUsed — flaky if max_error() is
        // not properly taken into account for cell distances.
        test_with_closest_index_factory(
            &FractalLoopFactory,
            5,
            100,
            10,
            &HashSet::new(),
            0x5eed_c0d1,
        );
    }

    #[test]
    fn test_conservative_cell_distance_is_used_filtered() {
        let allowed: HashSet<ShapeId> = [ShapeId(0)].into_iter().collect();
        test_with_closest_index_factory(&FractalLoopFactory, 5, 100, 10, &allowed, 0x5eed_c0d2);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_result_roundtrip() {
        let r = Result {
            distance: ChordAngle::from_degrees(45.0),
            shape_id: ShapeId(2),
            edge_id: 5,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Result = serde_json::from_str(&json).unwrap();
        assert_eq!(r.distance, back.distance);
        assert_eq!(r.shape_id, back.shape_id);
        assert_eq!(r.edge_id, back.edge_id);
    }
}
