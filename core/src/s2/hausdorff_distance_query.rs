// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Discrete Hausdorff distance between two geometries.
//!
//! [`S2HausdorffDistanceQuery`] computes the discrete (vertex-based) Hausdorff
//! distance between two [`ShapeIndex`] geometries — both directed and undirected.
//!
//! The discrete directed Hausdorff distance from A to B is defined as the maximum,
//! over all vertices of A, of the closest edge distance from the vertex to B.
//!
//! Corresponds to C++ `s2hausdorff_distance_query.h/cc`.

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
use crate::s2::edge_query::{ClosestEdgeQuery, EdgeQueryOptions, PointTarget};
use crate::s2::predicates;
use crate::s2::shape_index::ShapeIndex;

/// Options for the Hausdorff distance query.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HausdorffOptions {
    /// Whether to include polygon interiors when computing distances.
    /// When true (default), points inside a polygon have zero distance to it.
    pub include_interiors: bool,
}

impl Default for HausdorffOptions {
    fn default() -> Self {
        HausdorffOptions {
            include_interiors: true,
        }
    }
}

/// Result of a directed Hausdorff distance query.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirectedResult {
    /// The directed Hausdorff distance.
    pub distance: ChordAngle,
    /// The point on the target geometry where the distance is achieved.
    pub target_point: Point,
}

/// Result of an undirected Hausdorff distance query.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HausdorffResult {
    /// The directed result from target to source.
    pub target_to_source: DirectedResult,
    /// The directed result from source to target.
    pub source_to_target: DirectedResult,
}

impl HausdorffResult {
    /// Returns the undirected Hausdorff distance (max of both directions).
    pub fn distance(&self) -> ChordAngle {
        let a = self.target_to_source.distance;
        let b = self.source_to_target.distance;
        if a > b { a } else { b }
    }
}

/// Computes discrete Hausdorff distances between two [`ShapeIndex`] geometries.
///
/// # Examples
///
/// ```
/// use s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery;
/// use s2rst::s2::shape_index::ShapeIndex;
/// use s2rst::s2::lax_polyline::LaxPolyline;
/// use s2rst::s2::LatLng;
///
/// let mut a = ShapeIndex::new();
/// a.add(Box::new(LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
/// ])));
/// a.build();
///
/// let mut b = ShapeIndex::new();
/// b.add(Box::new(LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 1.0).to_point(),
///     LatLng::from_degrees(1.0, 1.0).to_point(),
/// ])));
/// b.build();
///
/// let query = S2HausdorffDistanceQuery::new();
/// let result = query.get_result(&a, &b);
/// assert!(result.is_some());
/// assert!(result.unwrap().distance().to_angle().degrees() > 0.9);
/// ```
#[derive(Debug, Default)]
pub struct S2HausdorffDistanceQuery {
    options: HausdorffOptions,
}

impl S2HausdorffDistanceQuery {
    /// Creates a new query with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new query with the given options.
    pub fn with_options(options: HausdorffOptions) -> Self {
        S2HausdorffDistanceQuery { options }
    }

    /// Returns a reference to the options.
    pub fn options(&self) -> &HausdorffOptions {
        &self.options
    }

    /// Returns a mutable reference to the options.
    pub fn options_mut(&mut self) -> &mut HausdorffOptions {
        &mut self.options
    }

    /// Computes the directed Hausdorff distance from `target` to `source`.
    ///
    /// Returns `None` if either index is empty (has no vertices).
    pub fn get_directed_result(
        &self,
        target: &ShapeIndex,
        source: &ShapeIndex,
    ) -> Option<DirectedResult> {
        let query = ClosestEdgeQuery::new(source);
        let opts = EdgeQueryOptions::default()
            .max_results(1)
            .include_interiors(self.options.include_interiors);

        let mut max_distance = ChordAngle::NEGATIVE;
        let mut target_point = Point::origin();
        let mut source_point = Point::origin();

        // Iterate over all vertices of the target index.
        for shape_id in 0..target.len() as i32 {
            let Some(shape) = target.shape(shape_id) else {
                continue;
            };

            for chain_id in 0..shape.num_chains() {
                let chain = shape.chain(chain_id);
                // Visit all vertices in this chain.
                // For a chain of length L, there are L+1 vertices (for polylines)
                // or L vertices (for loops). We gather them from edges.
                if chain.length == 0 {
                    continue;
                }

                // First vertex of first edge
                let first_edge = shape.chain_edge(chain_id, 0);
                self.update_max_distance(
                    first_edge.v0,
                    &query,
                    &opts,
                    &mut max_distance,
                    &mut target_point,
                    &mut source_point,
                );

                // v1 of each edge
                for offset in 0..chain.length {
                    let edge = shape.chain_edge(chain_id, offset);
                    self.update_max_distance(
                        edge.v1,
                        &query,
                        &opts,
                        &mut max_distance,
                        &mut target_point,
                        &mut source_point,
                    );
                }
            }
        }

        if max_distance.is_negative() {
            None
        } else {
            Some(DirectedResult {
                distance: max_distance,
                target_point,
            })
        }
    }

    /// Returns the directed Hausdorff distance, or `ChordAngle::INFINITY` if
    /// either index is empty.
    pub fn get_directed_distance(&self, target: &ShapeIndex, source: &ShapeIndex) -> ChordAngle {
        self.get_directed_result(target, source)
            .map_or(ChordAngle::INFINITY, |r| r.distance)
    }

    /// Returns whether the directed Hausdorff distance is less than `limit`.
    pub fn is_directed_distance_less(
        &self,
        target: &ShapeIndex,
        source: &ShapeIndex,
        distance_limit: ChordAngle,
    ) -> bool {
        let query = ClosestEdgeQuery::new(source);
        let opts = EdgeQueryOptions::default()
            .max_results(1)
            .include_interiors(self.options.include_interiors);

        let mut max_distance = ChordAngle::NEGATIVE;
        let mut target_point = Point::origin();
        let mut source_point = Point::origin();

        for shape_id in 0..target.len() as i32 {
            let Some(shape) = target.shape(shape_id) else {
                continue;
            };

            for chain_id in 0..shape.num_chains() {
                let chain = shape.chain(chain_id);
                if chain.length == 0 {
                    continue;
                }

                let first_edge = shape.chain_edge(chain_id, 0);
                self.update_max_distance(
                    first_edge.v0,
                    &query,
                    &opts,
                    &mut max_distance,
                    &mut target_point,
                    &mut source_point,
                );
                if max_distance > distance_limit {
                    return false;
                }

                for offset in 0..chain.length {
                    let edge = shape.chain_edge(chain_id, offset);
                    self.update_max_distance(
                        edge.v1,
                        &query,
                        &opts,
                        &mut max_distance,
                        &mut target_point,
                        &mut source_point,
                    );
                    if max_distance > distance_limit {
                        return false;
                    }
                }
            }
        }

        !max_distance.is_negative()
    }

    /// Computes the undirected Hausdorff distance between `target` and `source`.
    ///
    /// Returns `None` if either index is empty.
    pub fn get_result(&self, target: &ShapeIndex, source: &ShapeIndex) -> Option<HausdorffResult> {
        let t2s = self.get_directed_result(target, source)?;
        let s2t = self.get_directed_result(source, target)?;
        Some(HausdorffResult {
            target_to_source: t2s,
            source_to_target: s2t,
        })
    }

    /// Returns the undirected Hausdorff distance, or `ChordAngle::INFINITY` if
    /// either index is empty.
    pub fn get_distance(&self, target: &ShapeIndex, source: &ShapeIndex) -> ChordAngle {
        self.get_result(target, source)
            .map_or(ChordAngle::INFINITY, |r| r.distance())
    }

    /// Returns whether the undirected Hausdorff distance is less than `limit`.
    pub fn is_distance_less(
        &self,
        target: &ShapeIndex,
        source: &ShapeIndex,
        distance_limit: ChordAngle,
    ) -> bool {
        self.is_directed_distance_less(target, source, distance_limit)
            && self.is_directed_distance_less(source, target, distance_limit)
    }

    /// Internal: updates `max_distance` if the closest edge distance from `point`
    /// to the source index exceeds the current max.
    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn update_max_distance(
        &self,
        point: Point,
        query: &ClosestEdgeQuery,
        opts: &EdgeQueryOptions,
        max_distance: &mut ChordAngle,
        target_point: &mut Point,
        source_point: &mut Point,
    ) {
        // Optimization: skip if point is closer to last source_point than current max.
        if !max_distance.is_negative()
            && predicates::compare_distance(point, *source_point, *max_distance) <= 0
        {
            return;
        }

        let target = PointTarget::new(point);
        let results = query.find_edges(&target, opts);
        if let Some(result) = results.first()
            && *max_distance < result.distance
        {
            *max_distance = result.distance;
            *target_point = point;
            // Project point onto the closest edge to get source_point.
            if result.edge_id >= 0
                && let Some(shape) = query.index().shape(result.shape_id)
            {
                let edge = shape.edge(result.edge_id as usize);
                *source_point = crate::s2::edge_distances::project(point, edge.v0, edge.v1);
            } else {
                // Interior result — source_point is the point itself.
                *source_point = point;
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
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::point_vector::PointVector;
    use crate::s2::text_format;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn make_index_with_polyline(points: Vec<Point>) -> ShapeIndex {
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolyline::new(points)));
        index.build();
        index
    }

    fn make_index_with_points(points: Vec<Point>) -> ShapeIndex {
        let mut index = ShapeIndex::new();
        index.add(Box::new(PointVector::new(points)));
        index.build();
        index
    }

    #[test]
    fn test_result_constructors() {
        let point1 = p(3.0, 4.0);
        let point2 = p(5.0, 6.0);
        let dist1 = ChordAngle::from_degrees(5.0);
        let dist2 = ChordAngle::from_degrees(5.0);

        let dr1 = DirectedResult {
            distance: dist1,
            target_point: point1,
        };
        let dr2 = DirectedResult {
            distance: dist2,
            target_point: point2,
        };
        let result = HausdorffResult {
            target_to_source: dr1.clone(),
            source_to_target: dr2.clone(),
        };

        assert_eq!(dr1.target_point, point1);
        assert_eq!(dr1.distance, dist1);
        assert_eq!(result.target_to_source.target_point, point1);
        assert_eq!(result.source_to_target.target_point, point2);
        assert_eq!(result.distance(), dist2);
    }

    #[test]
    fn test_options() {
        let default_opts = HausdorffOptions::default();
        assert!(default_opts.include_interiors);

        let mut opts = HausdorffOptions::default();
        opts.include_interiors = false;
        assert!(!opts.include_interiors);
    }

    #[test]
    fn test_query_options_accessors() {
        let mut query = S2HausdorffDistanceQuery::new();
        assert!(query.options().include_interiors);

        query.options_mut().include_interiors = false;
        assert!(!query.options().include_interiors);
    }

    #[test]
    fn test_simple_polyline_queries() {
        let a0 = text_format::parse_points("0:0, 0:1, 0:1.5");
        let a1 = text_format::parse_points("0:2, 0:1.5, -10:1");
        let b0 = text_format::parse_points("1:0, 1:1, 3:2");

        let empty_index = ShapeIndex::new();

        // Shape index a has 2 polylines: a0 and a1.
        let mut a = ShapeIndex::new();
        a.add(Box::new(LaxPolyline::new(a0.clone())));
        a.add(Box::new(LaxPolyline::new(a1.clone())));
        a.build();

        // Shape index b has 1 polyline: b0.
        let mut b = ShapeIndex::new();
        b.add(Box::new(LaxPolyline::new(b0.clone())));
        b.build();

        let query = S2HausdorffDistanceQuery::new();

        // Empty index tests
        assert!(query.get_directed_result(&empty_index, &a).is_none());
        assert!(query.get_directed_result(&a, &empty_index).is_none());
        assert!(query.get_directed_distance(&a, &empty_index).is_infinity());
        assert!(!query.is_directed_distance_less(
            &empty_index,
            &a,
            ChordAngle::from_degrees(360.0)
        ));
        assert!(!query.is_directed_distance_less(
            &a,
            &empty_index,
            ChordAngle::from_degrees(360.0)
        ));

        // Directed distances
        let expected_a_to_b = a1[2].chord_angle(b0[1]);
        let expected_b_to_a = b0[2].chord_angle(a1[0]);

        let dir_a_to_b = query.get_directed_result(&a, &b);
        let dir_b_to_a = query.get_directed_result(&b, &a);

        assert!(dir_a_to_b.is_some());
        assert!(dir_b_to_a.is_some());

        let dir_a_to_b = dir_a_to_b.unwrap();
        let dir_b_to_a = dir_b_to_a.unwrap();

        assert!((dir_a_to_b.distance.degrees() - expected_a_to_b.degrees()).abs() < 1e-10);
        assert!((dir_b_to_a.distance.degrees() - expected_b_to_a.degrees()).abs() < 1e-10);

        let dir_a_to_b_dist = query.get_directed_distance(&a, &b);
        assert!((dir_a_to_b_dist.degrees() - expected_a_to_b.degrees()).abs() < 1e-10);

        // IsDirectedDistanceLess
        assert!(query.is_directed_distance_less(
            &a,
            &b,
            dir_a_to_b_dist + ChordAngle::from_degrees(1.0)
        ));
        assert!(!query.is_directed_distance_less(
            &a,
            &b,
            dir_a_to_b_dist - ChordAngle::from_degrees(1.0)
        ));

        // Undirected tests
        let a_to_b = query.get_result(&a, &b);
        let b_to_a = query.get_result(&b, &a);
        let bb = query.get_result(&b, &b);

        assert!(a_to_b.is_some());
        assert!(b_to_a.is_some());
        assert!(bb.is_some());

        let a_to_b = a_to_b.unwrap();
        let b_to_a_r = b_to_a.unwrap();
        let bb = bb.unwrap();

        assert!((a_to_b.distance().degrees() - b_to_a_r.distance().degrees()).abs() < 1e-10);
        assert!(bb.distance().degrees() < 1e-10);

        let b_to_a_dist = query.get_distance(&b, &a);
        assert!((b_to_a_dist.degrees() - b_to_a_r.distance().degrees()).abs() < 1e-10);

        // IsDistanceLess
        let larger = dir_a_to_b
            .distance
            .radians()
            .max(dir_b_to_a.distance.radians());
        let smaller = dir_a_to_b
            .distance
            .radians()
            .min(dir_b_to_a.distance.radians());
        let average = f64::midpoint(larger, smaller);

        assert!(query.is_distance_less(&a, &b, ChordAngle::from_radians(larger + 0.001)));
        assert!(!query.is_distance_less(&a, &b, ChordAngle::from_radians(average)));
        assert!(!query.is_distance_less(&a, &b, ChordAngle::from_radians(smaller - 0.001)));
        assert!(query.is_distance_less(&b, &b, ChordAngle::from_degrees(0.0)));
    }

    #[test]
    fn test_point_vector_shape_queries() {
        let a_points = text_format::parse_points("2:0, 0:1, 1:2, 0:3, 0:4");
        let b_points = text_format::parse_points("-1:2, -0.5:0.5, -0.5:3.5");

        let a = make_index_with_polyline(a_points.clone());
        let b = make_index_with_points(b_points.clone());

        let query = S2HausdorffDistanceQuery::new();

        let expected_a_to_b = a_points[0].chord_angle(b_points[1]);
        let expected_b_to_a = b_points[0].chord_angle(a_points[3]);
        let expected_a_b = if expected_a_to_b > expected_b_to_a {
            expected_a_to_b
        } else {
            expected_b_to_a
        };

        let dir_a_to_b = query.get_directed_result(&a, &b).unwrap();
        let dir_b_to_a = query.get_directed_result(&b, &a).unwrap();
        let undirected_a_b = query.get_distance(&a, &b);

        assert!(!undirected_a_b.is_infinity());
        assert!((undirected_a_b.degrees() - expected_a_b.degrees()).abs() < 1e-10);
        assert!((dir_a_to_b.distance.degrees() - expected_a_to_b.degrees()).abs() < 1e-10);
        assert_eq!(dir_a_to_b.target_point, a_points[0]);
        assert!((dir_b_to_a.distance.degrees() - expected_b_to_a.degrees()).abs() < 1e-10);
        assert_eq!(dir_b_to_a.target_point, b_points[0]);

        // IsDirectedDistanceLess
        assert!(query.is_directed_distance_less(
            &a,
            &b,
            ChordAngle::from_degrees(expected_a_to_b.degrees() + 0.01)
        ));
        assert!(query.is_directed_distance_less(
            &b,
            &a,
            ChordAngle::from_degrees(expected_b_to_a.degrees() + 0.01)
        ));
        assert!(!query.is_directed_distance_less(
            &a,
            &b,
            ChordAngle::from_degrees(expected_a_to_b.degrees() - 0.01)
        ));
        assert!(!query.is_directed_distance_less(
            &b,
            &a,
            ChordAngle::from_degrees(expected_b_to_a.degrees() - 0.01)
        ));

        // IsDistanceLess
        assert!(query.is_distance_less(
            &a,
            &b,
            ChordAngle::from_degrees(expected_a_b.degrees() + 0.01)
        ));
        assert!(!query.is_distance_less(
            &b,
            &a,
            ChordAngle::from_degrees(expected_b_to_a.degrees() - 0.01)
        ));
    }

    #[test]
    fn test_overlapping_polygons() {
        let epsilon = 3.0e-3;

        // Triangle with first two vertices inside quadrangle.
        let mut a = ShapeIndex::new();
        a.add(Box::new(text_format::make_lax_polygon("1:1, 1:2, 3.5:1.5")));
        a.build();

        // Quadrangle.
        let mut b = ShapeIndex::new();
        b.add(Box::new(text_format::make_lax_polygon(
            "0:0, 0:3, 3:3, 3:0",
        )));
        b.build();

        // Triangle fully inside quadrangle.
        let mut c = ShapeIndex::new();
        c.add(Box::new(text_format::make_lax_polygon("0:0, 0:2, 3:0")));
        c.build();

        // Query 1: exclude interiors
        let query1 = S2HausdorffDistanceQuery::with_options(HausdorffOptions {
            include_interiors: false,
        });

        let a_to_b_1 = query1.get_directed_result(&a, &b).unwrap();
        // Without interiors, the Hausdorff distance is from the vertex (1,2)
        // which is inside the quadrangle, to the nearest edge — about 1 degree.
        assert!((a_to_b_1.distance.degrees() - 1.0).abs() < epsilon);
        assert_eq!(a_to_b_1.target_point, p(1.0, 2.0));

        assert!(query1.is_directed_distance_less(&c, &b, ChordAngle::from_degrees(1.0 + epsilon)));

        // Query 2: include interiors
        let query2 = S2HausdorffDistanceQuery::with_options(HausdorffOptions {
            include_interiors: true,
        });

        let a_to_b_2 = query2.get_directed_result(&a, &b).unwrap();
        // With interiors, the two vertices inside the quadrangle have zero distance.
        // The max distance is from (3.5, 1.5) which is outside — about 0.5 degrees.
        assert!((a_to_b_2.distance.degrees() - 0.5).abs() < epsilon);
        assert_eq!(a_to_b_2.target_point, p(3.5, 1.5));

        // C is fully inside B, so directed distance should be ~0.
        assert!(query2.is_directed_distance_less(&c, &b, ChordAngle::from_degrees(epsilon)));
    }

    #[test]
    fn test_whole_world() {
        let mut a = ShapeIndex::new();
        a.add(Box::new(PointVector::new(text_format::parse_points("1:1"))));
        a.build();

        let b = text_format::make_index("# # full");

        let query = S2HausdorffDistanceQuery::with_options(HausdorffOptions {
            include_interiors: true,
        });

        // Point to full polygon: directed distance should be 0.
        let a_to_b = query.get_directed_result(&a, &b);
        assert!(a_to_b.is_some());
        assert_eq!(a_to_b.unwrap().distance.degrees(), 0.0);

        // Full polygon to point: no vertices in the full polygon, returns None.
        let b_to_a = query.get_directed_result(&b, &a);
        assert!(b_to_a.is_none());

        // Undirected: should fail since one direction returns None.
        assert!(query.get_result(&b, &a).is_none());
        assert!(query.get_result(&a, &b).is_none());

        // IsDirectedDistanceLess
        assert!(query.is_directed_distance_less(&a, &b, ChordAngle::ZERO));
        assert!(!query.is_directed_distance_less(&b, &a, ChordAngle::INFINITY));
        assert!(!query.is_distance_less(&a, &b, ChordAngle::INFINITY));
    }

    #[test]
    fn test_whole_world_same_reference() {
        let a = text_format::make_index("# # full");
        let b = text_format::make_index("# # full");

        let query = S2HausdorffDistanceQuery::with_options(HausdorffOptions {
            include_interiors: true,
        });

        assert!(query.get_result(&a, &b).is_none());
        assert!(query.get_result(&a, &a).is_none());
        assert!(!query.is_distance_less(&a, &b, ChordAngle::INFINITY));
        assert!(!query.is_distance_less(&a, &a, ChordAngle::INFINITY));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_options_roundtrip() {
        let opts = HausdorffOptions {
            include_interiors: false,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let back: HausdorffOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(opts.include_interiors, back.include_interiors);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_directed_result_roundtrip() {
        let dr = DirectedResult {
            distance: ChordAngle::from_degrees(45.0),
            target_point: Point::from_coords(1.0, 0.0, 0.0),
        };
        let json = serde_json::to_string(&dr).unwrap();
        let back: DirectedResult = serde_json::from_str(&json).unwrap();
        assert_eq!(dr.distance, back.distance);
        assert_eq!(dr.target_point, back.target_point);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_hausdorff_result_roundtrip() {
        let p1 = Point::from_coords(1.0, 0.0, 0.0);
        let p2 = Point::from_coords(0.0, 1.0, 0.0);
        let hr = HausdorffResult {
            target_to_source: DirectedResult {
                distance: ChordAngle::from_degrees(30.0),
                target_point: p1,
            },
            source_to_target: DirectedResult {
                distance: ChordAngle::from_degrees(60.0),
                target_point: p2,
            },
        };
        let json = serde_json::to_string(&hr).unwrap();
        let back: HausdorffResult = serde_json::from_str(&json).unwrap();
        assert_eq!(hr.distance(), back.distance());
    }
}
