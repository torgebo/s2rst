// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Query points on `S2Shape` edges by spherical distance.
//!
//! [`S2ChainInterpolationQuery`] computes cumulative distance along shape edges
//! and supports O(log n) interpolation queries by distance or fraction.
//!
//! Corresponds to C++ `s2chain_interpolation_query.h/cc`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
#![expect(
    clippy::cast_possible_truncation,
    reason = "edge index (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for edge index — always in range"
)]
use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::edge_distances;
use crate::s2::shape::Shape;

/// Result of a chain interpolation query.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainInterpolationResult {
    /// The interpolated point on the chain.
    pub point: Point,
    /// The edge ID on which the point lies.
    pub edge_id: usize,
    /// The cumulative distance along the chain to this point.
    pub distance: Angle,
}

/// Queries points on `S2Shape` edges by cumulative spherical distance.
///
/// Once initialized, each query is O(log(number of edges)). Initialization
/// and memory are both O(number of edges).
///
/// If a specific `chain_id` is given, only that chain's edges are used.
/// Otherwise all edges across all chains are used (which may be discontinuous
/// at chain boundaries).
///
/// # Examples
///
/// ```
/// use s2rst::s2::chain_interpolation_query::S2ChainInterpolationQuery;
/// use s2rst::s2::lax_polyline::LaxPolyline;
/// use s2rst::s2::shape::Shape;
/// use s2rst::s2::LatLng;
///
/// let line = LaxPolyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(1.0, 0.0).to_point(),
///     LatLng::from_degrees(2.0, 0.0).to_point(),
/// ]);
/// let query = S2ChainInterpolationQuery::new(&line);
///
/// // Interpolate at 50% of the total length.
/// let result = query.at_fraction(0.5);
/// assert!(result.is_some());
/// let r = result.unwrap();
/// // Should be near (1.0, 0.0).
/// let ll = LatLng::from_point(r.point);
/// assert!((ll.lat.degrees() - 1.0).abs() < 0.01);
/// ```
#[derive(Debug)]
pub struct S2ChainInterpolationQuery<'a> {
    shape: &'a dyn Shape,
    cumulative_values: Vec<Angle>,
    first_edge_id: usize,
    last_edge_id: usize, // inclusive; 0 when empty (but cumulative_values is empty too)
}

impl<'a> S2ChainInterpolationQuery<'a> {
    /// Creates a new query for the given shape, using all chains.
    pub fn new(shape: &'a dyn Shape) -> Self {
        Self::with_chain(shape, None)
    }

    /// Creates a new query for the given shape and optional chain ID.
    ///
    /// If `chain_id` is `Some(id)`, only edges from that chain are used.
    /// If `None`, all edges in the shape are used.
    ///
    /// # Panics
    ///
    /// Panics if `chain_id` is `Some(id)` and `id >= shape.num_chains()`.
    pub fn with_chain(shape: &'a dyn Shape, chain_id: Option<usize>) -> Self {
        let (first_edge_id, last_edge_id) = if let Some(cid) = chain_id {
            assert!(cid < shape.num_chains());
            let chain = shape.chain(cid);
            let first = chain.start;
            let last = if chain.length == 0 {
                0 // Will result in empty cumulative_values
            } else {
                first + chain.length - 1
            };
            (first, last)
        } else {
            let n = shape.num_edges();
            if n == 0 { (0, 0) } else { (0, n - 1) }
        };

        let mut cumulative_values = Vec::new();
        let num_edges =
            if shape.num_edges() == 0 || chain_id.is_some_and(|cid| shape.chain(cid).length == 0) {
                0
            } else {
                last_edge_id + 1 - first_edge_id
            };

        if num_edges > 0 {
            cumulative_values.reserve(num_edges + 1);
            let mut cumulative_angle = Angle::ZERO;
            for i in first_edge_id..=last_edge_id {
                cumulative_values.push(cumulative_angle);
                let edge = shape.edge(i);
                cumulative_angle = cumulative_angle + edge.v0.distance(edge.v1);
            }
            cumulative_values.push(cumulative_angle);
        }

        S2ChainInterpolationQuery {
            shape,
            cumulative_values,
            first_edge_id,
            last_edge_id,
        }
    }

    /// Returns the total length of the chain(s).
    pub fn get_length(&self) -> Angle {
        self.cumulative_values
            .last()
            .copied()
            .unwrap_or(Angle::ZERO)
    }

    /// Returns the cumulative length up to the end of the given edge ID.
    ///
    /// Returns `Angle::INFINITY` if the edge ID is outside the interpolated range.
    /// Returns zero if the query has no edges.
    pub fn get_length_at_edge_end(&self, edge_id: i32) -> Angle {
        if self.cumulative_values.is_empty() {
            return Angle::ZERO;
        }

        if edge_id < self.first_edge_id as i32 || edge_id > self.last_edge_id as i32 {
            return Angle::INFINITY;
        }

        self.cumulative_values[edge_id as usize - self.first_edge_id + 1]
    }

    /// Returns the interpolated point at the given distance along the chain.
    ///
    /// Returns `None` if the query has no edges.
    /// If distance exceeds total length, returns the last vertex at total length.
    /// If distance is negative, returns the first vertex at distance 0.
    pub fn at_distance(&self, distance: Angle) -> Option<ChainInterpolationResult> {
        if self.cumulative_values.is_empty() {
            return None;
        }

        // Binary search for the position.
        let idx = self.cumulative_values.partition_point(|&v| v < distance);

        if idx == 0 {
            // Before the start: snap to first vertex.
            let edge = self.shape.edge(self.first_edge_id);
            return Some(ChainInterpolationResult {
                point: edge.v0,
                edge_id: self.first_edge_id,
                distance: self.cumulative_values[0],
            });
        }

        if idx >= self.cumulative_values.len() {
            // Past the end: snap to last vertex.
            let edge = self.shape.edge(self.last_edge_id);
            return Some(ChainInterpolationResult {
                point: edge.v1,
                edge_id: self.last_edge_id,
                distance: self.cumulative_values[self.cumulative_values.len() - 1],
            });
        }

        // Interpolate within the edge.
        let edge_id = idx - 1 + self.first_edge_id;
        let edge = self.shape.edge(edge_id);
        let edge_start_dist = self.cumulative_values[idx - 1];
        let point = edge_distances::point_on_line(edge.v0, edge.v1, distance - edge_start_dist);

        Some(ChainInterpolationResult {
            point,
            edge_id,
            distance,
        })
    }

    /// Returns the interpolated point at the given fraction (0.0 = start, 1.0 = end).
    pub fn at_fraction(&self, fraction: f64) -> Option<ChainInterpolationResult> {
        self.at_distance(Angle::from_radians(fraction * self.get_length().radians()))
    }

    /// Returns a slice of points from `begin_fraction` to `end_fraction`.
    ///
    /// If `begin_fraction > end_fraction`, points are returned in reverse order.
    /// Returns empty vec if the query has no edges.
    pub fn slice(&self, begin_fraction: f64, end_fraction: f64) -> Vec<Point> {
        let mut result = Vec::new();
        self.add_slice(begin_fraction, end_fraction, &mut result);
        result
    }

    /// Appends a slice of points to the given vector.
    ///
    /// If `begin_fraction > end_fraction`, points are appended in reverse order.
    pub fn add_slice(
        &self,
        mut begin_fraction: f64,
        mut end_fraction: f64,
        slice: &mut Vec<Point>,
    ) {
        if self.cumulative_values.is_empty() {
            return;
        }

        let original_size = slice.len();
        let reverse = begin_fraction > end_fraction;
        if reverse {
            std::mem::swap(&mut begin_fraction, &mut end_fraction);
        }

        let begin_result = self.at_fraction(begin_fraction);
        let end_result = self.at_fraction(end_fraction);

        if let (Some(begin_r), Some(end_r)) = (begin_result, end_result) {
            let begin_edge = begin_r.edge_id;
            let mut last_point = begin_r.point;
            slice.push(last_point);

            let end_edge = end_r.edge_id;
            for edge_id in begin_edge..end_edge {
                let edge = self.shape.edge(edge_id);
                if last_point != edge.v1 {
                    last_point = edge.v1;
                    slice.push(last_point);
                }
            }
            slice.push(end_r.point);
        }

        if reverse {
            slice[original_size..].reverse();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::text_format;

    const EPSILON: f64 = 1.0e-8;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_simple_polylines() {
        let lat_b = 1.0_f64;
        let lat_c = 2.5_f64;
        let total_length_abc = lat_c;

        let a = p(0.0, 0.0);
        let b = p(lat_b, 0.0);
        let c = p(lat_c, 0.0);

        let empty_shape = LaxPolyline::new(vec![]);
        let shape_ac = LaxPolyline::new(vec![a, c]);
        let shape_abc = LaxPolyline::new(vec![a, b, c]);
        let shape_bb = LaxPolyline::new(vec![b, b]);

        let query_empty = S2ChainInterpolationQuery::new(&empty_shape);
        let query_ac = S2ChainInterpolationQuery::new(&shape_ac);
        let query_abc = S2ChainInterpolationQuery::new(&shape_abc);
        let query_bb = S2ChainInterpolationQuery::new(&shape_bb);

        let distances = [
            -1.0,
            0.0,
            1.0e-8,
            lat_b / 2.0,
            lat_b - 1.0e-7,
            lat_b,
            lat_b + 1.0e-5,
            lat_b + 0.5,
            lat_c - 10.0e-7,
            lat_c,
            lat_c + 10.0e-16,
            1.0e6,
        ];

        // Check lengths
        assert!(query_empty.get_length().degrees() <= EPSILON);
        assert!((query_ac.get_length().degrees() - total_length_abc).abs() < EPSILON);
        assert!((query_abc.get_length().degrees() - total_length_abc).abs() < EPSILON);
        assert!(query_bb.get_length().degrees() <= EPSILON);

        // AtDistance at infinity should return last vertex
        let ac_at_inf = query_ac.at_distance(Angle::INFINITY);
        assert!(ac_at_inf.is_some());
        let ac_pt = ac_at_inf.unwrap().point;
        assert!(ac_pt.distance(c).degrees() <= EPSILON);

        // Empty query returns None
        assert!(query_empty.at_fraction(0.0).is_none());

        for &dist in &distances {
            let frac = dist / total_length_abc;
            let lat = dist.clamp(0.0, total_length_abc);
            let expected_point = p(lat, 0.0);
            let expected_edge_id: usize = if dist < lat_b { 0 } else { 1 };

            let ac_r = query_ac.at_fraction(frac);
            let abc_r = query_abc.at_fraction(frac);
            let bb_r = query_bb.at_fraction(frac);

            assert!(ac_r.is_some());
            assert!(abc_r.is_some());
            assert!(bb_r.is_some());

            let ac_r = ac_r.unwrap();
            let abc_r = abc_r.unwrap();
            let bb_r = bb_r.unwrap();

            assert!(ac_r.point.distance(expected_point).radians() <= EPSILON);
            assert!(abc_r.point.distance(expected_point).radians() <= EPSILON);
            assert!(bb_r.point.distance(shape_bb.vertex(0)).radians() <= EPSILON);

            assert_eq!(ac_r.edge_id, 0);
            assert_eq!(bb_r.edge_id, 0);
            assert_eq!(abc_r.edge_id, expected_edge_id);
        }
    }

    #[test]
    fn test_distance() {
        let distances = [
            -1.0,
            -1.0e-8,
            0.0,
            1.0e-8,
            0.2,
            0.5,
            1.0 - 1.0e-8,
            1.0,
            1.0 + 1.0e-8,
            1.2,
            1.2,
            1.2 + 1.0e-10,
            1.5,
            1.999999,
            2.0,
            2.00000001,
            1.0e6,
        ];
        let vertices = text_format::parse_points(
            "0:0, 0:0, 1.0e-7:0, 0.1:0, 0.2:0, 0.2:0, 0.6:0, 0.999999:0, 0.999999:0, \
             1:0, 1:0, 1.000001:0, 1.000001:0, 1.1:0, 1.2:0, 1.2000001:0, 1.7:0, \
             1.99999999:0, 2:0",
        );
        let total_length = vertices[0].distance(*vertices.last().unwrap()).degrees();

        let shape = LaxPolyline::new(vertices.clone());
        let query = S2ChainInterpolationQuery::new(&shape);

        assert!((query.get_length().degrees() - total_length).abs() < EPSILON);

        for &d in &distances {
            let result = query.at_distance(Angle::from_degrees(d));
            assert!(result.is_some());
            let r = result.unwrap();

            let lat = LatLng::from(r.point).lat.degrees();
            let edge_id = r.edge_id;

            if d < 0.0 {
                assert!((lat - 0.0).abs() < EPSILON);
                assert_eq!(edge_id, 0);
                assert!((r.distance.degrees() - 0.0).abs() < EPSILON);
            } else if d > 2.0 {
                assert!((lat - 2.0).abs() < EPSILON);
                assert_eq!(edge_id, shape.num_edges() - 1);
                assert!((r.distance.degrees() - total_length).abs() < EPSILON);
            } else {
                assert!((lat - d).abs() < EPSILON);
                let edge = shape.edge(edge_id);
                let v0_lat = LatLng::from(edge.v0).lat.degrees();
                let v1_lat = LatLng::from(edge.v1).lat.degrees();
                assert!(lat >= v0_lat - EPSILON);
                assert!(lat <= v1_lat + EPSILON);
                assert!((r.distance.degrees() - d).abs() < EPSILON);
            }
        }
    }

    #[test]
    fn test_chains() {
        let loop0 = text_format::parse_points("0:0, 1:0");
        let loop1 = text_format::parse_points("2:0, 3:0");
        let shape = LaxPolygon::from_loops(&[&loop0, &loop1]);

        let query = S2ChainInterpolationQuery::new(&shape);
        let query0 = S2ChainInterpolationQuery::with_chain(&shape, Some(0));
        let query1 = S2ChainInterpolationQuery::with_chain(&shape, Some(1));

        let r = query.at_fraction(0.25);
        let r0 = query0.at_fraction(0.25);
        let r1 = query1.at_fraction(0.25);

        assert!(r.is_some());
        assert!(r0.is_some());
        assert!(r1.is_some());

        let r = r.unwrap();
        let r0 = r0.unwrap();
        let r1 = r1.unwrap();

        assert!((LatLng::from(r.point).lat.degrees() - 1.0).abs() < EPSILON);
        assert!((LatLng::from(r0.point).lat.degrees() - 0.5).abs() < EPSILON);
        assert!((LatLng::from(r1.point).lat.degrees() - 2.5).abs() < EPSILON);
    }

    #[test]
    fn test_get_length_at_edge_empty() {
        let shape = LaxPolyline::new(vec![]);
        let query = S2ChainInterpolationQuery::new(&shape);
        assert_eq!(query.get_length_at_edge_end(0).radians(), 0.0);
    }

    #[test]
    fn test_get_length_at_edge_polyline() {
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 1.0), p(0.0, 3.0), p(0.0, 6.0)]);
        let query = S2ChainInterpolationQuery::new(&shape);

        assert!((query.get_length().degrees() - 6.0).abs() < 0.01);
        assert!(query.get_length_at_edge_end(-100).is_infinite());
        assert!((query.get_length_at_edge_end(0).degrees() - 1.0).abs() < 0.01);
        assert!((query.get_length_at_edge_end(1).degrees() - 3.0).abs() < 0.01);
        assert!((query.get_length_at_edge_end(2).degrees() - 6.0).abs() < 0.01);
        assert!(query.get_length_at_edge_end(100).is_infinite());
    }

    #[test]
    fn test_get_length_at_edge_polygon() {
        let loop0 = vec![p(1.0, 1.0), p(2.0, 1.0), p(2.0, 3.0), p(1.0, 3.0)];
        let loop1 = vec![p(0.0, 0.0), p(0.0, 4.0), p(3.0, 4.0), p(3.0, 0.0)];
        let shape = LaxPolygon::from_loops(&[&loop0, &loop1]);
        let tolerance = 0.01;

        // Query chain 0 (inner loop: 4 edges forming a rectangle)
        let query0 = S2ChainInterpolationQuery::with_chain(&shape, Some(0));
        assert!((query0.get_length().degrees() - 6.0).abs() < tolerance);
        assert!(query0.get_length_at_edge_end(-100).is_infinite());
        assert!((query0.get_length_at_edge_end(0).degrees() - 1.0).abs() < tolerance);
        assert!((query0.get_length_at_edge_end(1).degrees() - 3.0).abs() < tolerance);
        assert!((query0.get_length_at_edge_end(2).degrees() - 4.0).abs() < tolerance);
        assert!((query0.get_length_at_edge_end(3).degrees() - 6.0).abs() < tolerance);
        // Edges 4-7 are in chain 1, not chain 0
        assert!(query0.get_length_at_edge_end(4).is_infinite());
        assert!(query0.get_length_at_edge_end(5).is_infinite());
        assert!(query0.get_length_at_edge_end(6).is_infinite());
        assert!(query0.get_length_at_edge_end(7).is_infinite());
        assert!(query0.get_length_at_edge_end(100).is_infinite());

        // Query chain 1 (outer loop: 4 edges forming a larger rectangle)
        let query1 = S2ChainInterpolationQuery::with_chain(&shape, Some(1));
        assert!((query1.get_length().degrees() - 14.0).abs() < tolerance);
        assert!(query1.get_length_at_edge_end(-100).is_infinite());
        // Edges 0-3 are in chain 0, not chain 1
        assert!(query1.get_length_at_edge_end(0).is_infinite());
        assert!(query1.get_length_at_edge_end(1).is_infinite());
        assert!(query1.get_length_at_edge_end(2).is_infinite());
        assert!(query1.get_length_at_edge_end(3).is_infinite());
        assert!((query1.get_length_at_edge_end(4).degrees() - 4.0).abs() < tolerance);
        assert!((query1.get_length_at_edge_end(5).degrees() - 7.0).abs() < tolerance);
        assert!((query1.get_length_at_edge_end(6).degrees() - 11.0).abs() < tolerance);
        assert!((query1.get_length_at_edge_end(7).degrees() - 14.0).abs() < tolerance);
        assert!(query1.get_length_at_edge_end(100).is_infinite());
    }

    #[test]
    fn test_slice() {
        // Empty query returns empty slice.
        let empty_shape = LaxPolyline::new(vec![]);
        let empty_query = S2ChainInterpolationQuery::new(&empty_shape);
        assert!(empty_query.slice(0.0, 1.0).is_empty());

        let shape = text_format::make_lax_polyline("0:0, 0:1, 0:2");
        let query = S2ChainInterpolationQuery::new(&shape);

        // Full slice
        let full = query.slice(0.0, 1.0);
        assert_eq!(text_format::points_to_string(&full), "0:0, 0:1, 0:2");

        // First half
        let first_half = query.slice(0.0, 0.5);
        assert_eq!(text_format::points_to_string(&first_half), "0:0, 0:1");

        // Reverse second half
        let rev = query.slice(1.0, 0.5);
        assert_eq!(text_format::points_to_string(&rev), "0:2, 0:1");

        // Middle quarter
        let mid = query.slice(0.25, 0.75);
        assert_eq!(text_format::points_to_string(&mid), "0:0.5, 0:1, 0:1.5");
    }

    // ─── C++ counterpart tests for refactored code paths ──────────────

    #[test]
    fn test_get_length_single_edge() {
        // A single-edge polyline: get_length should use cumulative_values[len-1].
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(1.0, 0.0)]);
        let query = S2ChainInterpolationQuery::new(&shape);
        assert!((query.get_length().degrees() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_get_length_empty_shape() {
        // Empty shape: get_length should return zero (no cumulative_values).
        let shape = LaxPolyline::new(vec![]);
        let query = S2ChainInterpolationQuery::new(&shape);
        assert_eq!(query.get_length().radians(), 0.0);
    }

    #[test]
    fn test_at_fraction_past_end_uses_last_cumulative() {
        // Fraction > 1 should snap to last vertex, using cumulative_values[len-1].
        let shape = LaxPolyline::new(vec![p(0.0, 0.0), p(0.0, 1.0), p(0.0, 3.0)]);
        let query = S2ChainInterpolationQuery::new(&shape);
        let result = query.at_fraction(1.5);
        assert!(result.is_some());
        let r = result.unwrap();
        // Should snap to last vertex.
        let ll = LatLng::from(r.point);
        assert!((ll.lng.degrees() - 3.0).abs() < 0.01);
        // Distance should be total length.
        assert!((r.distance.degrees() - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_chain_with_zero_length_chain() {
        // C++: chain_id.is_some_and(|cid| shape.chain(cid).length == 0)
        // A polygon with a degenerate (single-vertex) chain has length 0.
        // The query should handle this without panic.
        let shape = LaxPolyline::new(vec![p(0.0, 0.0)]);
        let query = S2ChainInterpolationQuery::new(&shape);
        assert_eq!(query.get_length().radians(), 0.0);
    }
}
