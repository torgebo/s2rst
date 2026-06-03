// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Find furthest edges between geometries using an `S2ShapeIndex`.
//!
//! This is the dual of [`closest_edge_query`](super::closest_edge_query): it finds edges that maximize
//! distance rather than minimize it.
//!
//! Corresponds to C++ `s2furthest_edge_query.h`.

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
use std::ops::ControlFlow;

use crate::s1::ChordAngle;
use crate::s2::distance_target::DistanceTarget;
use crate::s2::edge_distances;
use crate::s2::shape::{Edge, ShapeId};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::{Cap, Cell, Point};

/// A result from a furthest edge query.
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
            distance: ChordAngle::NEGATIVE,
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
        // Reverse: furthest first.
        other
            .distance
            .length2()
            .partial_cmp(&self.distance.length2())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.shape_id.cmp(&other.shape_id))
            .then_with(|| self.edge_id.cmp(&other.edge_id))
    }
}

/// Options for a furthest edge query.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Maximum number of results to return (default: `i32::MAX`).
    pub max_results: i32,
    /// Minimum distance for edges to be included (default: zero).
    pub min_distance: ChordAngle,
    /// Maximum error allowed (trades accuracy for speed).
    pub max_error: ChordAngle,
    /// Whether to include polygon interiors.
    pub include_interiors: bool,
    /// Force brute-force search.
    pub use_brute_force: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            max_results: i32::MAX,
            // C++ default: S2MaxDistance::Infinity() which maps to
            // S1ChordAngle::Negative(), meaning no minimum distance limit.
            min_distance: ChordAngle::NEGATIVE,
            max_error: ChordAngle::ZERO,
            include_interiors: true,
            use_brute_force: false,
        }
    }
}

impl Options {
    /// Sets `min_distance` so that edges at exactly `limit` are also returned.
    /// Equivalent to `min_distance = limit.predecessor()`.
    pub fn inclusive_min_distance(&mut self, limit: ChordAngle) {
        self.min_distance = limit.predecessor();
    }

    /// Sets `min_distance` so that all edges whose true distance is ≥ `limit`
    /// are returned, accounting for the maximum error in distance computation.
    pub fn conservative_min_distance(&mut self, limit: ChordAngle) {
        self.min_distance = limit
            .plus_error(-edge_distances::update_min_distance_max_error(limit))
            .predecessor();
    }
}

/// The target geometry to measure distance from.
/// The target geometry to measure distance from (furthest-edge variant).
///
/// Extends [`DistanceTarget`] with
/// methods specific to furthest-edge queries.
pub trait Target: DistanceTarget {
    /// Updates `dist_limit` if the distance from `p` to the target is greater.
    /// Returns `(new_dist, true)` if updated, or `(dist_limit, false)`.
    fn update_distance_to_point(&self, p: Point, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Updates `dist_limit` if the distance from edge (v0, v1) to the target
    /// is greater. Returns `(new_dist, true)` if updated.
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        dist_limit: ChordAngle,
    ) -> (ChordAngle, bool);

    /// Updates `dist_limit` if the distance from the cell to the target is
    /// greater. Returns `(new_dist, true)` if updated.
    fn update_distance_to_cell(&self, cell: &Cell, dist_limit: ChordAngle) -> (ChordAngle, bool);

    /// Visits shapes in `index` whose interior contains the target (or its
    /// antipode, for furthest queries). For furthest-edge queries, polygons
    /// containing the **antipode** of the target are visited, since those
    /// represent distance π (the maximum possible).
    ///
    /// Corresponds to C++ `S2MaxDistanceTarget::VisitContainingShapeIds`.
    fn visit_containing_shapes(
        &self,
        _index: &ShapeIndex,
        _visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) {
        // Default: do nothing. Subtypes override.
    }

    /// Maximum index size for which brute force is faster than an indexed
    /// search. The default is 100; subtypes override with values tuned
    /// to their geometry.
    fn max_brute_force_index_size(&self) -> i32 {
        100
    }
}

/// Target: find furthest edges from a point.
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
        // C++: break-even ~100/400/600 for point cloud/fractal/regular.
        300
    }

    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) {
        // For furthest-edge queries, visit polygons whose interior contains
        // the antipode of the target point. Matches C++
        // S2MaxDistancePointTarget::VisitContainingShapeIds.
        use crate::s2::contains_point_query::ContainsPointQuery;
        let mut query = ContainsPointQuery::new(
            index,
            crate::s2::contains_point_query::VertexModel::SemiOpen,
        );
        for shape_id in query.containing_shape_ids(-self.point) {
            if visitor(shape_id).is_break() {
                break;
            }
        }
    }
    fn update_distance_to_point(&self, p: Point, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = p.chord_angle(self.point);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        max_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        edge_distances::update_max_distance(self.point, v0, v1, max_dist)
    }
    fn update_distance_to_cell(&self, cell: &Cell, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.max_distance_to_point(self.point);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
}

/// Target: find furthest edges from an edge.
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
        // C++: break-even ~80/100/230 for point cloud/fractal/regular.
        110
    }

    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) {
        // Only need to test one endpoint. If the tested vertex antipode is
        // not contained, the full edge antipode is not contained; if it is
        // contained, the edge at least intersects the polygon.
        // Matches C++ S2MaxDistanceEdgeTarget::VisitContainingShapeIds.
        use crate::s2::contains_point_query::ContainsPointQuery;
        let mut query = ContainsPointQuery::new(
            index,
            crate::s2::contains_point_query::VertexModel::SemiOpen,
        );
        for shape_id in query.containing_shape_ids(-self.a) {
            if visitor(shape_id).is_break() {
                break;
            }
        }
    }
    fn update_distance_to_point(&self, p: Point, max_dist: ChordAngle) -> (ChordAngle, bool) {
        edge_distances::update_max_distance(p, self.a, self.b, max_dist)
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        max_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        // Delegates to update_edge_pair_max_distance which checks for
        // antipodal crossing (max distance = π) before falling back to
        // the four vertex-to-edge distance computations.
        edge_distances::update_edge_pair_max_distance(self.a, self.b, v0, v1, max_dist)
    }
    fn update_distance_to_cell(&self, cell: &Cell, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = cell.max_distance_to_edge(self.a, self.b);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
}

/// Target: find furthest edges from a cell.
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
    fn visit_containing_shapes(
        &self,
        index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) {
        // Matches C++ S2MaxDistanceCellTarget::VisitContainingShapeIds.
        use crate::s2::contains_point_query::ContainsPointQuery;
        let mut query = ContainsPointQuery::new(
            index,
            crate::s2::contains_point_query::VertexModel::SemiOpen,
        );
        for shape_id in query.containing_shape_ids(-self.cell.center()) {
            if visitor(shape_id).is_break() {
                break;
            }
        }
    }
    fn update_distance_to_point(&self, p: Point, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.max_distance_to_point(p);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        max_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let dist = self.cell.max_distance_to_edge(v0, v1);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
    fn update_distance_to_cell(&self, cell: &Cell, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let dist = self.cell.max_distance_to_cell(*cell);
        if dist > max_dist {
            (dist, true)
        } else {
            (max_dist, false)
        }
    }
}

/// Target: find furthest edges from any edge in another `ShapeIndex`.
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

    /// Helper: find the furthest edge in `self.index` from a given inner target,
    /// then check if that distance exceeds `max_dist`.
    fn update_max_distance_inner(
        &self,
        inner_target: &dyn Target,
        max_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let query = FurthestEdgeQuery::new(self.index);
        let opts = Options {
            max_results: 1,
            min_distance: max_dist,
            include_interiors: self.include_interiors,
            use_brute_force: self.use_brute_force,
            ..Options::default()
        };
        let result = query.find_furthest_edge_with_options(inner_target, &opts);
        if result.is_empty() {
            (max_dist, false)
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
        true
    }
}

impl Target for ShapeIndexTarget<'_> {
    fn max_brute_force_index_size(&self) -> i32 {
        // C++: break-even ~30/100/130 for point cloud/fractal/regular.
        70
    }

    fn visit_containing_shapes(
        &self,
        query_index: &ShapeIndex,
        visitor: &mut dyn FnMut(ShapeId) -> ControlFlow<()>,
    ) {
        // For each shape in the target index, test chain start vertices
        // (one per connected component). For shapes with no edges, use
        // the reference point if contained. Matches C++
        // S2MaxDistanceShapeIndexTarget::VisitContainingShapeIds.
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
                let pt = PointTarget::new(v0);
                pt.visit_containing_shapes(query_index, visitor);
            }
            if !tested_point {
                // Handle full polygons with no edges.
                let ref_pt = shape.reference_point();
                if !ref_pt.contained {
                    continue;
                }
                let pt = PointTarget::new(ref_pt.point);
                pt.visit_containing_shapes(query_index, visitor);
            }
        }
    }
    fn update_distance_to_point(&self, p: Point, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let inner = PointTarget::new(p);
        self.update_max_distance_inner(&inner, max_dist)
    }
    fn update_distance_to_edge(
        &self,
        v0: Point,
        v1: Point,
        max_dist: ChordAngle,
    ) -> (ChordAngle, bool) {
        let inner = EdgeTarget::new(v0, v1);
        self.update_max_distance_inner(&inner, max_dist)
    }
    fn update_distance_to_cell(&self, cell: &Cell, max_dist: ChordAngle) -> (ChordAngle, bool) {
        let inner = CellTarget::new(*cell);
        self.update_max_distance_inner(&inner, max_dist)
    }
}

/// Query to find furthest edges in a `ShapeIndex` from a given target.
#[derive(Debug)]
pub struct FurthestEdgeQuery<'a> {
    index: &'a ShapeIndex,
}

impl<'a> FurthestEdgeQuery<'a> {
    /// Creates a new furthest edge query for the given index.
    pub fn new(index: &'a ShapeIndex) -> Self {
        FurthestEdgeQuery { index }
    }

    /// Returns the furthest edge from the target, or an empty result.
    pub fn find_furthest_edge(&self, target: &dyn Target) -> Result {
        let opts = Options {
            max_results: 1,
            ..Options::default()
        };
        self.find_furthest_edge_with_options(target, &opts)
    }

    /// Returns the furthest edge from the target with the given options.
    /// Returns the furthest edge with the given options.
    /// For best performance, set `options.max_results = 1`.
    pub fn find_furthest_edge_with_options(
        &self,
        target: &dyn Target,
        options: &Options,
    ) -> Result {
        let results = self.find_furthest_edges(target, options);
        match results.into_iter().next() {
            Some(r) => r,
            None => Result::empty(),
        }
    }

    /// Returns the furthest edges from the target (up to `options.max_results`).
    pub fn find_furthest_edges(&self, target: &dyn Target, options: &Options) -> Vec<Result> {
        debug_assert!(options.max_results >= 1, "max_results must be >= 1");
        debug_assert!(
            target.max_brute_force_index_size() >= 0,
            "max_brute_force_index_size must be >= 0"
        );

        let mut results = Vec::new();

        let mut distance_limit = options.min_distance;

        // Check polygon interiors if requested. For furthest queries,
        // polygons containing the antipode of the target have distance π
        // (STRAIGHT). Matches C++ include_interiors handling.
        if options.include_interiors {
            let max_results = options.max_results as usize;
            let mut shape_ids = Vec::new();
            target.visit_containing_shapes(self.index, &mut |shape_id| {
                shape_ids.push(shape_id);
                if shape_ids.len() < max_results {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            });
            for shape_id in shape_ids {
                // Only add if STRAIGHT exceeds the current distance limit.
                if ChordAngle::STRAIGHT > distance_limit {
                    self.add_result(
                        Result {
                            distance: ChordAngle::STRAIGHT,
                            shape_id,
                            edge_id: -1,
                        },
                        options,
                        &mut distance_limit,
                        &mut results,
                    );
                }
            }
            if distance_limit == ChordAngle::STRAIGHT {
                // Can't do better than π; no need to search edges.
                results.sort_unstable();
                results.dedup_by(|a, b| a.shape_id == b.shape_id && a.edge_id == b.edge_id);
                if results.len() > options.max_results as usize {
                    results.truncate(options.max_results as usize);
                }
                return results;
            }
        }

        // Always use brute force for furthest queries (the cell-based
        // optimization doesn't apply as directly for max-distance).
        self.find_furthest_edges_brute_force(target, options, &mut distance_limit, &mut results);

        // Sort (furthest first) and deduplicate.
        results.sort_unstable();
        results.dedup_by(|a, b| a.shape_id == b.shape_id && a.edge_id == b.edge_id);
        if results.len() > options.max_results as usize {
            results.truncate(options.max_results as usize);
        }
        results
    }

    /// Returns the distance from the target to the furthest edge.
    pub fn get_distance(&self, target: &dyn Target) -> ChordAngle {
        self.find_furthest_edge(target).distance
    }

    /// Returns true if the max distance to the target is greater than `limit`.
    pub fn is_distance_greater(&self, target: &dyn Target, limit: ChordAngle) -> bool {
        let opts = Options {
            max_results: 1,
            min_distance: limit,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        let result = self.find_furthest_edge_with_options(target, &opts);
        !result.is_empty()
    }

    /// Returns true if the max distance to the target is greater than or
    /// equal to `limit`.
    pub fn is_distance_greater_or_equal(&self, target: &dyn Target, limit: ChordAngle) -> bool {
        let mut opts = Options {
            max_results: 1,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        opts.inclusive_min_distance(limit);
        let result = self.find_furthest_edge_with_options(target, &opts);
        !result.is_empty()
    }

    /// Like [`is_distance_greater_or_equal`](Self::is_distance_greater_or_equal)
    /// but `limit` is decreased by the maximum error in distance computation,
    /// ensuring all truly-beyond-limit edges are found.
    pub fn is_conservative_distance_greater_or_equal(
        &self,
        target: &dyn Target,
        limit: ChordAngle,
    ) -> bool {
        let mut opts = Options {
            max_results: 1,
            max_error: ChordAngle::STRAIGHT,
            ..Options::default()
        };
        opts.conservative_min_distance(limit);
        let result = self.find_furthest_edge_with_options(target, &opts);
        !result.is_empty()
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

    /// Brute force: check every edge in the index.
    fn find_furthest_edges_brute_force(
        &self,
        target: &dyn Target,
        options: &Options,
        distance_limit: &mut ChordAngle,
        results: &mut Vec<Result>,
    ) {
        for shape_id in (0..self.index.num_shape_ids() as i32).map(ShapeId) {
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            for edge_id in 0..shape.num_edges() {
                let edge = shape.edge(edge_id);
                let (dist, updated) =
                    target.update_distance_to_edge(edge.v0, edge.v1, *distance_limit);
                if updated {
                    self.add_result(
                        Result {
                            distance: dist,
                            shape_id,
                            edge_id: edge_id as i32,
                        },
                        options,
                        distance_limit,
                        results,
                    );
                }
            }
        }
    }

    /// Adds a result, potentially updating the distance limit.
    ///
    /// For `max_results == 1`, keeps only the single furthest edge.
    /// For bounded results, maintains a sorted set and prunes the worst.
    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn add_result(
        &self,
        result: Result,
        options: &Options,
        distance_limit: &mut ChordAngle,
        results: &mut Vec<Result>,
    ) {
        if options.max_results == 1 {
            // Singleton strategy: the new result is always better (it passed
            // the distance_limit check) so just replace.
            if results.is_empty() {
                results.push(result);
            } else {
                results[0] = result;
            }
            let best_dist = results[0].distance;
            // For furthest queries with max_error, the limit *increases*
            // (we can skip edges that can't beat best - error).
            *distance_limit = if options.max_error > ChordAngle::ZERO {
                ChordAngle::from_length2(
                    (best_dist.length2() + options.max_error.length2())
                        .min(ChordAngle::STRAIGHT.length2()),
                )
            } else {
                best_dist
            };
        } else {
            results.push(result);
            if results.len() as i32 >= options.max_results {
                // Sort (furthest first) and truncate.
                results.sort_unstable();
                if results.len() > options.max_results as usize {
                    results.truncate(options.max_results as usize);
                }
                if let Some(last) = results.last() {
                    // Raise limit to worst result's distance.
                    *distance_limit = last.distance;
                }
            }
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
    use crate::s2::LatLng;
    use crate::s2::coords::Level;

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
    fn test_no_edges() {
        let index = ShapeIndex::new();
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let result = query.find_furthest_edge(&target);
        assert!(result.is_empty());
    }

    #[test]
    fn test_furthest_edge_simple() {
        // Polyline along the equator from 0 to 10 degrees.
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));
        let result = query.find_furthest_edge(&target);

        assert!(!result.is_empty());
        // The furthest point on the edge from (0,5) is one of the endpoints
        // (0,0) or (0,10), both ~5 degrees away.
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            (dist_degrees - 5.0).abs() < 0.5,
            "distance = {dist_degrees} degrees, expected ~5.0"
        );
    }

    #[test]
    fn test_furthest_from_antipodal() {
        // Polyline near the equator. Query from near the antipode.
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, -175.0));
        let result = query.find_furthest_edge(&target);

        assert!(!result.is_empty());
        // Distance should be close to 180 degrees.
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            dist_degrees > 170.0,
            "distance = {dist_degrees} degrees, expected > 170"
        );
    }

    #[test]
    fn test_get_distance() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));
        let dist = query.get_distance(&target);
        // Max distance ~5 degrees (to either endpoint).
        assert!(dist.to_angle().degrees() > 4.0);
        assert!(dist.to_angle().degrees() < 6.0);
    }

    #[test]
    fn test_is_distance_greater() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));
        // Max distance is ~5 degrees.
        assert!(
            query.is_distance_greater(&target, ChordAngle::from_angle(Angle::from_degrees(3.0)))
        );
        assert!(
            !query.is_distance_greater(&target, ChordAngle::from_angle(Angle::from_degrees(10.0)))
        );
    }

    #[test]
    fn test_multiple_shapes() {
        let mut index = ShapeIndex::new();
        // Polyline near the equator.
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 5.0),
        ])));
        // Polyline far from the query point.
        index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 170.0),
            p(0.0, 175.0),
        ])));
        index.build();

        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, -5.0));
        let result = query.find_furthest_edge(&target);

        // The second polyline is furthest.
        assert!(!result.is_empty());
        assert_eq!(result.shape_id, 1);
    }

    #[test]
    fn test_edge_target() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = EdgeTarget::new(p(0.0, -5.0), p(0.0, -10.0));
        let result = query.find_furthest_edge(&target);

        assert!(!result.is_empty());
        // Furthest distance: from endpoint (0,-10) to (0,10) = 20 degrees.
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            dist_degrees > 15.0,
            "distance = {dist_degrees} degrees, expected ~20"
        );
    }

    #[test]
    fn test_top_k_results() {
        let mut index = ShapeIndex::new();
        for i in 0..5 {
            let lng = f64::from(i) * 30.0;
            index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
                p(0.0, lng),
                p(0.0, lng + 5.0),
            ])));
        }
        index.build();

        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, -5.0));
        let mut opts = Options::default();
        opts.max_results = 3;
        let results = query.find_furthest_edges(&target, &opts);

        assert_eq!(results.len(), 3);
        // Results should be ordered furthest first.
        assert!(results[0].distance >= results[1].distance);
        assert!(results[1].distance >= results[2].distance);
    }

    #[test]
    fn test_is_distance_greater_or_equal() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 180.0)); // antipode

        // Furthest distance is ~170-180 degrees. >= 160° → true.
        assert!(query.is_distance_greater_or_equal(&target, ChordAngle::from_degrees(160.0)));
        // >= INFINITY → false (nothing has distance INFINITY).
        assert!(!query.is_distance_greater_or_equal(&target, ChordAngle::INFINITY));
    }

    #[test]
    fn test_is_conservative_distance_greater_or_equal() {
        let index = make_polyline_index(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 180.0));

        assert!(
            query.is_conservative_distance_greater_or_equal(
                &target,
                ChordAngle::from_degrees(160.0)
            )
        );
    }

    #[test]
    fn test_inclusive_min_distance() {
        let mut opts = Options::default();
        opts.inclusive_min_distance(ChordAngle::from_degrees(5.0));
        // inclusive = limit.predecessor()
        assert!(opts.min_distance < ChordAngle::from_degrees(5.0));
    }

    #[test]
    fn test_conservative_min_distance() {
        let mut opts = Options::default();
        opts.conservative_min_distance(ChordAngle::from_degrees(5.0));
        // Conservative should be <= inclusive.
        let mut inclusive = Options::default();
        inclusive.inclusive_min_distance(ChordAngle::from_degrees(5.0));
        assert!(opts.min_distance <= inclusive.min_distance);
    }

    #[test]
    fn test_shape_index_target() {
        // Query index: polyline along the equator.
        let mut query_index = ShapeIndex::new();
        query_index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(0.0, 0.0),
            p(0.0, 10.0),
        ])));
        query_index.build();

        // Target index: polyline near the north pole.
        let mut target_index = ShapeIndex::new();
        target_index.add(Box::new(crate::s2::polyline::Polyline::new(vec![
            p(89.0, 0.0),
            p(89.0, 10.0),
        ])));
        target_index.build();

        let query = FurthestEdgeQuery::new(&query_index);
        let target = ShapeIndexTarget::new(&target_index);
        let result = query.find_furthest_edge(&target);

        assert!(!result.is_empty());
        let dist_degrees = result.distance.to_angle().degrees();
        assert!(
            dist_degrees > 85.0,
            "distance = {dist_degrees} degrees, expected >85"
        );
    }

    #[test]
    fn test_set_max_error_on_target() {
        let mut target = PointTarget::new(p(1.0, 2.0));
        assert!(!target.set_max_error(ChordAngle::from_degrees(1.0)));
    }

    #[test]
    fn test_include_interiors_antipodal() {
        // A polygon containing the antipode of the target should produce
        // an interior result at distance STRAIGHT (π).
        use crate::s2::lax_polygon::LaxPolygon;
        use crate::s2::shape_index::ShapeIndex;

        // Create a large polygon covering most of the sphere.
        let mut index = ShapeIndex::new();
        // A full polygon contains everything, including the antipode.
        index.add(Box::new(LaxPolygon::full()));
        index.build();

        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let opts = Options {
            max_results: i32::MAX,
            include_interiors: true,
            ..Options::default()
        };
        let results = query.find_furthest_edges(&target, &opts);

        // Should find the interior at distance STRAIGHT.
        let interior_results: Vec<_> = results.iter().filter(|r| r.is_interior()).collect();
        assert!(
            !interior_results.is_empty(),
            "should find interior result for full polygon"
        );
        assert_eq!(interior_results[0].distance, ChordAngle::STRAIGHT);
    }

    #[test]
    fn test_include_interiors_false() {
        // With include_interiors=false, no interior results should appear.
        use crate::s2::lax_polygon::LaxPolygon;
        use crate::s2::shape_index::ShapeIndex;

        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolygon::full()));
        index.build();

        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 0.0));
        let opts = Options {
            max_results: i32::MAX,
            include_interiors: false,
            ..Options::default()
        };
        let results = query.find_furthest_edges(&target, &opts);
        assert!(
            results.iter().all(|r| !r.is_interior()),
            "no interior results with include_interiors=false"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // Randomized brute-force correctness tests — ported from C++
    // s2furthest_edge_query_test.cc (CircleEdges / FractalEdges /
    // PointCloudEdges).
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

    trait ShapeIndexFactory {
        fn add_edges(&self, cap: &Cap, num_edges: usize, index: &mut ShapeIndex, rng: &mut StdRng);
    }

    struct RegularLoopFactory;
    impl ShapeIndexFactory for RegularLoopFactory {
        fn add_edges(
            &self,
            cap: &Cap,
            num_edges: usize,
            index: &mut ShapeIndex,
            _rng: &mut StdRng,
        ) {
            index.add(Box::new(crate::s2::Loop::make_regular(
                cap.center(),
                cap.angle_radius(),
                num_edges,
            )));
        }
    }

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

    fn count_edges(index: &ShapeIndex) -> usize {
        let mut total = 0;
        for id in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(id as i32) {
                total += shape.num_edges();
            }
        }
        total
    }

    fn log_uniform(rng: &mut StdRng, lo: f64, hi: f64) -> f64 {
        let v: f64 = rng.gen_range(lo.log2()..hi.log2());
        v.exp2()
    }

    /// Verify that furthest edge query results satisfy the search criteria.
    fn get_furthest_edges(
        target: &dyn Target,
        query: &FurthestEdgeQuery<'_>,
        options: &Options,
    ) -> Vec<Result> {
        let results = query.find_furthest_edges(target, options);
        assert!(
            results.len() <= options.max_results as usize,
            "too many results: {} > {}",
            results.len(),
            options.max_results
        );
        if options.min_distance == ChordAngle::NEGATIVE {
            let min_expected = (options.max_results as usize).min(count_edges(query.index));
            if options.include_interiors {
                assert!(results.len() >= min_expected);
            } else {
                assert_eq!(
                    results.len(),
                    min_expected,
                    "expected {min_expected} results"
                );
            }
        }
        for r in &results {
            assert!(
                r.distance >= options.min_distance,
                "result distance {:?} < min_distance {:?}",
                r.distance,
                options.min_distance
            );
        }
        results
    }

    /// Compare brute-force vs default (also brute-force for furthest queries,
    /// but exercises the full code path including dedup and sort).
    fn test_find_furthest_edges(
        target: &dyn Target,
        query: &FurthestEdgeQuery<'_>,
        options: &Options,
    ) {
        let mut bf_opts = options.clone();
        bf_opts.use_brute_force = true;
        let expected = get_furthest_edges(target, query, &bf_opts);

        let mut opt_opts = options.clone();
        opt_opts.use_brute_force = false;
        let actual = get_furthest_edges(target, query, &opt_opts);

        // Check that expected and actual agree on the maximum distance.
        // (Full CheckDistanceResults is overkill since both paths are brute-force.)
        if !expected.is_empty() && !actual.is_empty() {
            let diff = (expected[0].distance.length2() - actual[0].distance.length2()).abs();
            assert!(
                diff < 1e-12,
                "furthest distance mismatch: expected {:?}, actual {:?}",
                expected[0].distance,
                actual[0].distance,
            );
        }

        if expected.is_empty() {
            return;
        }

        // Verify GetDistance and IsDistanceGreater consistency.
        // Use options that match the original search (especially
        // include_interiors) to avoid spurious interior results.
        let max_error = options.max_error;
        let expected_distance = expected[0].distance;

        let get_dist_opts = Options {
            max_results: 1,
            include_interiors: options.include_interiors,
            ..Options::default()
        };
        let got = query
            .find_furthest_edge_with_options(target, &get_dist_opts)
            .distance;
        assert!(
            got >= expected_distance - max_error,
            "GetDistance {got:?} < expected {expected_distance:?} - error {max_error:?}",
        );

        let check_opts = Options {
            max_results: 1,
            min_distance: expected_distance + max_error,
            max_error: ChordAngle::STRAIGHT,
            include_interiors: options.include_interiors,
            ..Options::default()
        };
        let check_result = query.find_furthest_edge_with_options(target, &check_opts);
        assert!(
            check_result.is_empty(),
            "IsDistanceGreater should be false above max + error"
        );
    }

    fn test_cap_radius() -> Angle {
        Angle::from_radians(10.0 / 6371.01)
    }

    fn test_with_furthest_index_factory(
        factory: &dyn ShapeIndexFactory,
        num_indexes: usize,
        num_edges: usize,
        num_queries: usize,
        seed: u64,
    ) {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut index_caps = Vec::with_capacity(num_indexes);
        let mut indexes = Vec::with_capacity(num_indexes);
        for _ in 0..num_indexes {
            let center = random_point(&mut rng);
            let cap = Cap::from_center_angle(center, test_cap_radius());
            let mut index = ShapeIndex::new();
            factory.add_edges(&cap, num_edges, &mut index, &mut rng);
            index.build();
            index_caps.push(cap);
            indexes.push(index);
        }

        for _ in 0..num_queries {
            let i_index = rng.gen_range(0..num_indexes);
            let index_cap = &index_caps[i_index];
            let query_radius = 2.0 * index_cap.angle_radius().radians();

            // Exercise the opposite-hemisphere code 1/5 of the time.
            let antipodal: f64 = if rng.gen_range(0..5) == 0 { -1.0 } else { 1.0 };
            let query_center = Point(index_cap.center().0 * antipodal);
            let query_cap = Cap::from_center_angle(query_center, Angle::from_radians(query_radius));

            let mut opts = Options::default();
            if rng.gen_range(0..5) != 0 {
                opts.max_results = rng.gen_range(1..11);
            }
            if rng.gen_range(0..3) != 0 {
                let frac: f64 = rng.gen_range(0.0..1.0);
                opts.min_distance =
                    ChordAngle::from_angle(Angle::from_radians(frac * query_radius));
            }
            if rng.gen_range(0..2) == 0 {
                let e = log_uniform(&mut rng, 1e-4, 1.0) * query_radius;
                opts.max_error = ChordAngle::from_angle(Angle::from_radians(e));
            }
            opts.include_interiors = rng.gen_range(0..2) == 0;

            let query = FurthestEdgeQuery::new(&indexes[i_index]);
            let target_type = rng.gen_range(0..4);

            match target_type {
                0 => {
                    let point = sample_point_from_cap(&mut rng, &query_cap);
                    let target = PointTarget::new(point);
                    test_find_furthest_edges(&target, &query, &opts);
                }
                1 => {
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let edge_radius = log_uniform(&mut rng, 1e-4, 1.0) * query_radius;
                    let b_cap = Cap::from_center_angle(a, Angle::from_radians(edge_radius));
                    let b = sample_point_from_cap(&mut rng, &b_cap);
                    let target = EdgeTarget::new(a, b);
                    test_find_furthest_edges(&target, &query, &opts);
                }
                2 => {
                    let min_level = metric::MAX_DIAG.max_level(query_radius);
                    let level = Level::new(rng.gen_range(min_level.as_u8()..=MAX_CELL_LEVEL));
                    let a = sample_point_from_cap(&mut rng, &query_cap);
                    let cell = Cell::from(CellId::from_point(&a).parent_at_level(level));
                    let target = CellTarget::new(cell);
                    test_find_furthest_edges(&target, &query, &opts);
                }
                3 => {
                    let j_index = rng.gen_range(0..num_indexes);
                    let target = ShapeIndexTarget::new(&indexes[j_index]);
                    test_find_furthest_edges(&target, &query, &opts);
                }
                _ => unreachable!(),
            }
        }
    }

    const FURTHEST_NUM_INDEXES: usize = 50;
    const FURTHEST_NUM_EDGES: usize = 100;
    const FURTHEST_NUM_QUERIES: usize = 200;

    #[test]
    fn test_furthest_circle_edges() {
        test_with_furthest_index_factory(
            &RegularLoopFactory,
            FURTHEST_NUM_INDEXES,
            FURTHEST_NUM_EDGES,
            FURTHEST_NUM_QUERIES,
            0xfeed_c1c1,
        );
    }

    #[test]
    fn test_furthest_fractal_edges() {
        test_with_furthest_index_factory(
            &FractalLoopFactory,
            FURTHEST_NUM_INDEXES,
            FURTHEST_NUM_EDGES,
            FURTHEST_NUM_QUERIES,
            0xfeed_f4ac,
        );
    }

    #[test]
    fn test_furthest_point_cloud_edges() {
        test_with_furthest_index_factory(
            &PointCloudFactory,
            FURTHEST_NUM_INDEXES,
            FURTHEST_NUM_EDGES,
            FURTHEST_NUM_QUERIES,
            0xfeed_9c1d,
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_result_roundtrip() {
        let r = Result {
            distance: ChordAngle::from_degrees(120.0),
            shape_id: ShapeId(1),
            edge_id: 3,
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: Result = serde_json::from_str(&json).unwrap();
        assert_eq!(r.distance, back.distance);
        assert_eq!(r.shape_id, back.shape_id);
        assert_eq!(r.edge_id, back.edge_id);
    }
}
