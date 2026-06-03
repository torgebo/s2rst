// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Finding closest or furthest edges in a [`ShapeIndex`].
//!
//! [`ClosestEdgeQuery`] and [`FurthestEdgeQuery`] efficiently find edges that
//! are closest or furthest from a given target (point, edge, etc.).
//!
//! Corresponds to C++ `s2closest_edge_query.h`, `s2furthest_edge_query.h`,
//! Go `s2/edge_query.go`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId (i32) -> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId — always in range"
)]
use crate::s1::ChordAngle;
use crate::s2::Point;
use crate::s2::edge_distances;
use crate::s2::shape::{Dimension, Edge};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_util;

// ─── Options ────────────────────────────────────────────────────────────

/// Options controlling how an edge query operates.
#[derive(Clone, Debug, PartialEq)]
pub struct EdgeQueryOptions {
    /// Maximum number of results to return. Must be >= 1.
    pub max_results: usize,
    /// Only return edges within this distance of the target.
    /// For closest queries, this is a maximum distance.
    /// For furthest queries, this is a minimum distance.
    pub distance_limit: ChordAngle,
    /// Edges up to this much further than the true closest/furthest may
    /// be returned. Only has an effect if `max_results` is specified.
    pub max_error: ChordAngle,
    /// Whether to include polygon interiors. When true, polygons that
    /// contain the target have zero distance (returned with `edge_id` == -1).
    pub include_interiors: bool,
    /// Force brute-force algorithm (test every edge).
    pub use_brute_force: bool,
}

impl Default for EdgeQueryOptions {
    fn default() -> Self {
        EdgeQueryOptions {
            max_results: usize::MAX,
            distance_limit: ChordAngle::INFINITY,
            max_error: ChordAngle::ZERO,
            include_interiors: true,
            use_brute_force: true,
        }
    }
}

impl EdgeQueryOptions {
    /// Sets the maximum number of results.
    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    /// Sets the distance limit.
    pub fn distance_limit(mut self, limit: ChordAngle) -> Self {
        self.distance_limit = limit;
        self
    }

    /// Sets the maximum allowable error.
    pub fn max_error(mut self, err: ChordAngle) -> Self {
        self.max_error = err;
        self
    }

    /// Sets whether polygon interiors are included.
    pub fn include_interiors(mut self, include: bool) -> Self {
        self.include_interiors = include;
        self
    }

    /// Sets whether to force brute-force search.
    pub fn use_brute_force(mut self, brute_force: bool) -> Self {
        self.use_brute_force = brute_force;
        self
    }
}

// ─── Result ─────────────────────────────────────────────────────────────

/// A single result from an edge query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeQueryResult {
    /// The distance from the target to this edge.
    pub distance: ChordAngle,
    /// The shape ID within the `ShapeIndex`.
    pub shape_id: i32,
    /// The edge ID within the shape, or -1 for polygon interiors.
    pub edge_id: i32,
}

impl EdgeQueryResult {
    /// Reports whether this result represents a polygon interior.
    pub fn is_interior(&self) -> bool {
        self.shape_id >= 0 && self.edge_id < 0
    }

    /// Reports whether this result is empty (no edge found).
    pub fn is_empty(&self) -> bool {
        self.shape_id < 0
    }
}

impl Default for EdgeQueryResult {
    fn default() -> Self {
        EdgeQueryResult {
            distance: ChordAngle::INFINITY,
            shape_id: -1,
            edge_id: -1,
        }
    }
}

// ─── Target trait ───────────────────────────────────────────────────────

/// A target geometry for distance queries.
pub trait DistanceTarget {
    /// Updates `dist` if the distance to `p` is less. Returns `Some(new_dist)`
    /// if updated, `None` otherwise.
    fn update_min_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle>;

    /// Updates `dist` if the distance to edge `e` is less.
    fn update_min_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle>;

    /// Updates `dist` if the distance to `p` is greater. Returns `Some(new_dist)`
    /// if updated, `None` otherwise.
    fn update_max_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle>;

    /// Updates `dist` if the distance to edge `e` is greater.
    fn update_max_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle>;

    /// Returns the point to use for interior containment checks, if applicable.
    fn interior_point(&self) -> Option<Point> {
        None
    }
}

// ─── Point target ───────────────────────────────────────────────────────

/// A target consisting of a single point.
#[derive(Debug)]
pub struct PointTarget {
    point: Point,
}

impl PointTarget {
    /// Creates a new point target.
    pub fn new(point: Point) -> Self {
        PointTarget { point }
    }
}

impl DistanceTarget for PointTarget {
    fn update_min_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle> {
        let d = self.point.chord_angle(p);
        if d < dist { Some(d) } else { None }
    }

    fn update_min_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) = edge_distances::update_min_distance(self.point, e.v0, e.v1, dist);
        if ok { Some(d) } else { None }
    }

    fn update_max_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle> {
        let d = self.point.chord_angle(p);
        if d > dist { Some(d) } else { None }
    }

    fn update_max_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) = edge_distances::update_max_distance(self.point, e.v0, e.v1, dist);
        if ok { Some(d) } else { None }
    }

    fn interior_point(&self) -> Option<Point> {
        Some(self.point)
    }
}

// ─── Edge target ────────────────────────────────────────────────────────

/// A target consisting of an edge (geodesic segment).
#[derive(Debug)]
pub struct EdgeTarget {
    a: Point,
    b: Point,
}

impl EdgeTarget {
    /// Creates a new edge target.
    pub fn new(a: Point, b: Point) -> Self {
        EdgeTarget { a, b }
    }
}

impl DistanceTarget for EdgeTarget {
    fn update_min_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) = edge_distances::update_min_distance(p, self.a, self.b, dist);
        if ok { Some(d) } else { None }
    }

    fn update_min_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) =
            edge_distances::update_edge_pair_min_distance(self.a, self.b, e.v0, e.v1, dist);
        if ok { Some(d) } else { None }
    }

    fn update_max_distance_to_point(&self, p: Point, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) = edge_distances::update_max_distance(p, self.a, self.b, dist);
        if ok { Some(d) } else { None }
    }

    fn update_max_distance_to_edge(&self, e: Edge, dist: ChordAngle) -> Option<ChordAngle> {
        let (d, ok) =
            edge_distances::update_edge_pair_max_distance(self.a, self.b, e.v0, e.v1, dist);
        if ok { Some(d) } else { None }
    }

    fn interior_point(&self) -> Option<Point> {
        // Use the edge midpoint to ensure AB and BA yield identical results.
        Some(Point::from_coords(
            f64::midpoint(self.a.0.x, self.b.0.x),
            f64::midpoint(self.a.0.y, self.b.0.y),
            f64::midpoint(self.a.0.z, self.b.0.z),
        ))
    }
}

// ─── Closest Edge Query ─────────────────────────────────────────────────

/// Finds the closest edges in a [`ShapeIndex`] to a given target.
///
/// # Examples
///
/// ```
/// use s2rst::s2::edge_query::ClosestEdgeQuery;
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::lax_polyline::LaxPolyline;
/// use s2rst::s2::LatLng;
///
/// // Index a polyline from (0,0) to (0,10).
/// let shape = LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 10.0).to_point(),
/// ]);
/// let mut index = ShapeIndex::new();
/// index.add(Box::new(shape));
/// index.build();
///
/// // Find the closest edge to a nearby point.
/// let query = ClosestEdgeQuery::new(&index);
/// let target = LatLng::from_degrees(1.0, 5.0).to_point();
/// let result = query.find_closest_to_point(target);
///
/// assert!(!result.is_empty());
/// assert_eq!(result.shape_id, 0);
/// assert_eq!(result.edge_id, 0);
/// // Distance should be small (about 1 degree).
/// assert!(result.distance.to_angle().degrees() < 2.0);
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

    /// Returns the underlying index.
    pub fn index(&self) -> &ShapeIndex {
        self.index
    }

    /// Finds the single closest edge to the given point.
    pub fn find_closest_to_point(&self, point: Point) -> EdgeQueryResult {
        let target = PointTarget::new(point);
        let opts = EdgeQueryOptions::default().max_results(1);
        let results = self.find_edges(&target, &opts);
        results.into_iter().next().unwrap_or_default()
    }

    /// Finds the single closest edge to the given edge.
    pub fn find_closest_to_edge(&self, a: Point, b: Point) -> EdgeQueryResult {
        let target = EdgeTarget::new(a, b);
        let opts = EdgeQueryOptions::default().max_results(1);
        let results = self.find_edges(&target, &opts);
        results.into_iter().next().unwrap_or_default()
    }

    /// Returns the distance from the target point to the closest edge.
    pub fn distance_to_point(&self, point: Point) -> ChordAngle {
        self.find_closest_to_point(point).distance
    }

    /// Reports whether the distance to the target point is less than `limit`.
    pub fn is_distance_less(&self, point: Point, limit: ChordAngle) -> bool {
        let target = PointTarget::new(point);
        let opts = EdgeQueryOptions::default()
            .max_results(1)
            .distance_limit(limit)
            .max_error(limit);
        !self.find_edges(&target, &opts).is_empty()
    }

    /// Finds edges matching the given options.
    pub fn find_edges(
        &self,
        target: &dyn DistanceTarget,
        opts: &EdgeQueryOptions,
    ) -> Vec<EdgeQueryResult> {
        debug_assert!(opts.max_results >= 1, "max_results must be >= 1");

        let mut results = Vec::new();
        let mut dist_limit = opts.distance_limit;

        // Check polygon interiors: if a dimension-2 shape contains the target
        // point, the distance is zero.
        if opts.include_interiors
            && let Some(p) = target.interior_point()
        {
            for shape_id in 0..self.index.len() as i32 {
                let Some(shape) = self.index.shape(shape_id) else {
                    continue;
                };
                if shape.dimension() == Dimension::Polygon
                    && shape_util::contains_brute_force(shape, p)
                {
                    results.push(EdgeQueryResult {
                        distance: ChordAngle::ZERO,
                        shape_id,
                        edge_id: -1,
                    });
                    if opts.max_results == 1 {
                        dist_limit = ChordAngle::ZERO;
                    }
                }
            }
        }

        // Brute force: test every edge in every shape.
        for shape_id in 0..self.index.len() as i32 {
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            for edge_id in 0..shape.num_edges() as i32 {
                let edge = shape.edge(edge_id as usize);
                if let Some(d) = target.update_min_distance_to_edge(edge, dist_limit) {
                    let result = EdgeQueryResult {
                        distance: d,
                        shape_id,
                        edge_id,
                    };
                    results.push(result);

                    if opts.max_results == 1 {
                        // Keep only the closest.
                        dist_limit = d;
                    }
                }
            }
        }

        // Sort by distance, then prune to max_results.
        results.sort_unstable_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if results.len() > opts.max_results {
            results.truncate(opts.max_results);
        }

        // For max_results == 1, keep only the single best.
        if opts.max_results == 1 && results.len() > 1 {
            results.truncate(1);
        }

        results
    }
}

// ─── Furthest Edge Query ────────────────────────────────────────────────

/// Finds the furthest edges in a [`ShapeIndex`] from a given target.
#[derive(Debug)]
pub struct FurthestEdgeQuery<'a> {
    index: &'a ShapeIndex,
}

impl<'a> FurthestEdgeQuery<'a> {
    /// Creates a new furthest edge query for the given index.
    pub fn new(index: &'a ShapeIndex) -> Self {
        FurthestEdgeQuery { index }
    }

    /// Finds the single furthest edge from the given point.
    pub fn find_furthest_from_point(&self, point: Point) -> EdgeQueryResult {
        let target = PointTarget::new(point);
        let mut best = EdgeQueryResult {
            distance: ChordAngle::ZERO, // Start with zero for furthest
            ..Default::default()
        };

        for shape_id in 0..self.index.len() as i32 {
            let Some(shape) = self.index.shape(shape_id) else {
                continue;
            };
            for edge_id in 0..shape.num_edges() as i32 {
                let edge = shape.edge(edge_id as usize);
                if let Some(d) = target.update_max_distance_to_edge(edge, best.distance) {
                    best = EdgeQueryResult {
                        distance: d,
                        shape_id,
                        edge_id,
                    };
                }
            }
        }

        best
    }

    /// Returns the distance from the target point to the furthest edge.
    pub fn distance_to_point(&self, point: Point) -> ChordAngle {
        self.find_furthest_from_point(point).distance
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::lax_loop::LaxLoop;
    use crate::s2::lax_polyline::LaxPolyline;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty_index_closest() {
        let index = ShapeIndex::new();
        let query = ClosestEdgeQuery::new(&index);
        let result = query.find_closest_to_point(p(0.0, 0.0));
        assert!(result.is_empty());
    }

    #[test]
    fn test_closest_to_point_single_edge() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Point on the edge itself
        let result = query.find_closest_to_point(p(0.0, 5.0));
        assert!(!result.is_empty());
        assert_eq!(result.shape_id, 0);
        assert_eq!(result.edge_id, 0);
        // Distance should be very small (point is approximately on the edge)
        assert!(result.distance.length2() < 0.001);
    }

    #[test]
    fn test_closest_to_point_triangle() {
        let shape = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Point near vertex 0
        let result = query.find_closest_to_point(p(0.01, 0.01));
        assert!(!result.is_empty());
        assert_eq!(result.shape_id, 0);
        // Distance should be small
        assert!(result.distance.length2() < 0.01);
    }

    #[test]
    fn test_closest_to_point_far_away() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 1.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Point far away
        let result = query.find_closest_to_point(p(80.0, 80.0));
        assert!(!result.is_empty());
        // Distance should be large
        assert!(result.distance.length2() > 1.0);
    }

    #[test]
    fn test_distance_to_point() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let dist = query.distance_to_point(p(1.0, 5.0));
        assert!(dist.length2() > 0.0);
        assert!(dist.length2() < 0.01); // ~1 degree away, chord angle small
    }

    #[test]
    fn test_is_distance_less() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Point right on the edge
        assert!(query.is_distance_less(p(0.0, 5.0), ChordAngle::from_length2(0.01)));

        // Point far away: not within a tiny limit
        assert!(!query.is_distance_less(p(80.0, 80.0), ChordAngle::from_length2(0.01)));
    }

    #[test]
    fn test_closest_to_edge() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Query edge parallel and close
        let result = query.find_closest_to_edge(p(1.0, 2.0), p(1.0, 8.0));
        assert!(!result.is_empty());
        assert!(result.distance.length2() < 0.01);
    }

    #[test]
    fn test_multiple_shapes() {
        let shape1 = LaxPolyline::new(vec![p(10.0, 10.0), p(10.0, 20.0)]);
        let shape2 = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 1.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape1));
        index.add(Box::new(shape2));
        index.build();

        let query = ClosestEdgeQuery::new(&index);

        // Point near shape2
        let result = query.find_closest_to_point(p(0.0, 0.5));
        assert_eq!(result.shape_id, 1);
        assert_eq!(result.edge_id, 0);
    }

    #[test]
    fn test_furthest_from_point() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = FurthestEdgeQuery::new(&index);
        let result = query.find_furthest_from_point(p(0.0, 0.0));
        assert!(!result.is_empty());
        // Furthest edge endpoint is at p(0,10), distance should be non-trivial
        assert!(result.distance.length2() > 0.0);
    }

    #[test]
    fn test_find_edges_with_distance_limit() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(0.0, 20.0), p(0.0, 30.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(p(0.0, 5.0));

        // With a tight distance limit, should only get nearby edges
        let opts = EdgeQueryOptions::default().distance_limit(ChordAngle::from_length2(0.05));
        let results = query.find_edges(&target, &opts);
        // At least edge 0 (from p(0,0) to p(0,10)) should be within range
        assert!(!results.is_empty());
    }
}
