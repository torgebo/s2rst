// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A sequence of vertices connected by geodesic edges.
//!
//! A [`Polyline`] represents an open path on the sphere. Adjacent vertices
//! should not be identical or antipodal.
//!
//! Corresponds to C++ `s2polyline.h`, Go `s2/polyline.go`.

use std::collections::HashSet;

use crate::s1::{self, Angle};
use crate::s2::builder::{S2Error, S2ErrorCode};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_distances;
use crate::s2::predicates;
use crate::s2::shape::{Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape};
use crate::s2::{Cap, Cell, CellId, LatLng, Point, Rect, Region};

/// An open path on the sphere defined by a sequence of vertices.
///
/// Adjacent vertices must not be identical or antipodal. A polyline with
/// fewer than 2 vertices has no edges. Implements [`Shape`] (dimension 1)
/// and [`Region`].
///
/// # Examples
///
/// ```
/// use s2rst::s2::LatLng;
/// use s2rst::s2::polyline::Polyline;
///
/// let polyline = Polyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 90.0).to_point(),
/// ]);
/// // Length of a quarter great circle is pi/2 radians.
/// let length = polyline.length();
/// assert!((length.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-13);
///
/// // Interpolate to the midpoint (0, 45).
/// let (mid, _next) = polyline.interpolate(0.5);
/// let expected = LatLng::from_degrees(0.0, 45.0).to_point();
/// assert!(mid.distance(expected).radians() < 1e-10);
/// ```
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline {
    vertices: Vec<Point>,
}

impl Polyline {
    /// Creates a new `Polyline` from a list of vertices.
    pub fn new(vertices: Vec<Point>) -> Self {
        Polyline { vertices }
    }

    /// Creates a new `Polyline` from latitude-longitude pairs.
    pub fn from_lat_lngs(latlngs: &[LatLng]) -> Self {
        let vertices = latlngs.iter().map(|ll| ll.to_point()).collect();
        Polyline { vertices }
    }

    /// Returns the number of vertices.
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the vertex at the given index.
    pub fn vertex(&self, i: usize) -> Point {
        debug_assert!(i < self.vertices.len());
        self.vertices[i]
    }

    /// Reverses the order of the vertices.
    pub fn reverse(&mut self) {
        self.vertices.reverse();
    }

    /// Returns the total arc length of the polyline.
    pub fn length(&self) -> Angle {
        let mut len = Angle::from_radians(0.0);
        for i in 1..self.vertices.len() {
            len = len + self.vertices[i - 1].distance(self.vertices[i]);
        }
        len
    }

    /// Returns the true centroid of the polyline multiplied by the length.
    ///
    /// Scaling by the polyline length makes it easy to compute the centroid
    /// of several polylines (by summing their centroids).
    pub fn centroid(&self) -> Point {
        let mut centroid = Point::origin();
        centroid.0.x = 0.0;
        centroid.0.y = 0.0;
        centroid.0.z = 0.0;
        for i in 1..self.vertices.len() {
            let v_sum = self.vertices[i - 1].0 + self.vertices[i].0;
            let v_diff = self.vertices[i - 1].0 - self.vertices[i].0;
            let norm2_sum = v_sum.norm2();
            if norm2_sum > 0.0 {
                centroid.0 = centroid.0 + v_sum * (v_diff.norm2() / norm2_sum).sqrt();
            }
        }
        centroid
    }

    /// Validates the polyline.
    ///
    /// # Errors
    ///
    /// Returns a description of the first validation error found (non-unit
    /// vertex, identical or antipodal adjacent vertices).
    pub fn validate(&self) -> Result<(), String> {
        for (i, p) in self.vertices.iter().enumerate() {
            let norm = p.0.norm();
            if (norm - 1.0).abs() > 1e-15 {
                return Err(format!("vertex {i} is not unit length: {norm}"));
            }
        }
        for i in 1..self.vertices.len() {
            let prev = self.vertices[i - 1];
            let cur = self.vertices[i];
            if prev == cur {
                return Err(format!("vertices {} and {i} are identical", i - 1));
            }
            let neg = Point(-cur.0);
            if prev == neg {
                return Err(format!("vertices {} and {i} are antipodal", i - 1));
            }
        }
        Ok(())
    }

    /// Checks the polyline for validity, returning an `S2Error` if invalid.
    ///
    /// Returns `true` (and sets `error`) if the polyline has any problems:
    /// - Non-unit-length vertices
    /// - Adjacent identical vertices
    /// - Adjacent antipodal vertices
    ///
    /// C++: `S2Polyline::FindValidationError(S2Error* error)`
    pub fn find_validation_error(&self) -> Option<S2Error> {
        for (i, p) in self.vertices.iter().enumerate() {
            if !p.0.is_unit() {
                return Some(S2Error::new(
                    S2ErrorCode::NotUnitLength,
                    format!("Vertex {i} is not unit length"),
                ));
            }
        }
        for i in 1..self.vertices.len() {
            if self.vertices[i - 1] == self.vertices[i] {
                return Some(S2Error::new(
                    S2ErrorCode::DuplicateVertices,
                    format!("Vertices {} and {i} are identical", i - 1),
                ));
            }
            if self.vertices[i - 1] == Point(-self.vertices[i].0) {
                return Some(S2Error::new(
                    S2ErrorCode::AntipodalVertices,
                    format!("Vertices {} and {i} are antipodal", i - 1),
                ));
            }
        }
        None
    }

    /// Returns the point on the polyline closest to `point`, and the index
    /// of the next vertex after the projected point.
    ///
    /// The next vertex index is always in the range `[1, num_vertices()]`.
    ///
    /// # Panics
    ///
    /// Panics if the polyline has no vertices.
    pub fn project(&self, point: Point) -> (Point, usize) {
        assert!(!self.vertices.is_empty());
        if self.vertices.len() == 1 {
            return (self.vertices[0], 1);
        }

        let mut min_dist = Angle::from_radians(10.0); // Larger than any distance on unit sphere
        let mut min_index = 0usize;

        for i in 1..self.vertices.len() {
            let dist = edge_distances::distance_from_segment(
                point,
                self.vertices[i - 1],
                self.vertices[i],
            );
            if dist < min_dist {
                min_dist = dist;
                min_index = i;
            }
        }

        let closest = edge_distances::project(
            point,
            self.vertices[min_index - 1],
            self.vertices[min_index],
        );
        if closest == self.vertices[min_index] {
            min_index += 1;
        }

        (closest, min_index)
    }

    /// Returns the point at the given fraction along the polyline, and the
    /// index of the next vertex after that point.
    ///
    /// Fractions are clamped to [0, 1]. The next vertex index is always in
    /// the range `[1, num_vertices()]`.
    ///
    /// # Panics
    ///
    /// Panics if the polyline has no vertices.
    pub fn interpolate(&self, fraction: f64) -> (Point, usize) {
        assert!(!self.vertices.is_empty());
        if fraction <= 0.0 {
            return (self.vertices[0], 1);
        }
        let mut target = Angle::from_radians(fraction * self.length().radians());

        for i in 1..self.vertices.len() {
            let length = self.vertices[i - 1].distance(self.vertices[i]);
            if target < length {
                let result = edge_distances::interpolate_at_distance(
                    target,
                    self.vertices[i - 1],
                    self.vertices[i],
                );
                if result == self.vertices[i] {
                    return (result, i + 1);
                }
                return (result, i);
            }
            target = target - length;
        }

        (self.vertices[self.vertices.len() - 1], self.vertices.len())
    }

    /// Inverse of [`interpolate`](Polyline::interpolate). Given a point on
    /// the polyline and the next vertex index, returns the fraction of the
    /// total length from the beginning to that point.
    pub fn uninterpolate(&self, point: Point, next_vertex: usize) -> f64 {
        if self.vertices.len() < 2 {
            return 0.0;
        }
        let mut sum = Angle::from_radians(0.0);
        for i in 1..next_vertex {
            sum = sum + self.vertices[i - 1].distance(self.vertices[i]);
        }
        let length_to_point = sum + self.vertices[next_vertex - 1].distance(point);
        for i in next_vertex..self.vertices.len() {
            sum = sum + self.vertices[i - 1].distance(self.vertices[i]);
        }
        if sum.radians() == 0.0 {
            return 0.0;
        }
        (length_to_point.radians() / sum.radians()).min(1.0)
    }

    /// Reports whether two polylines have the same vertices.
    pub fn equal(&self, other: &Polyline) -> bool {
        self.vertices == other.vertices
    }

    /// Reports whether two polylines have the same number of vertices and
    /// corresponding vertices are within `max_error` of each other.
    pub fn approx_eq_with(&self, other: &Polyline, max_error: Angle) -> bool {
        if self.vertices.len() != other.vertices.len() {
            return false;
        }
        for (a, b) in self.vertices.iter().zip(other.vertices.iter()) {
            if a.distance(*b) > max_error {
                return false;
            }
        }
        true
    }

    /// Reports whether the given point is to the right of the polyline.
    ///
    /// The polyline must have at least 2 vertices. If the closest point is
    /// an interior vertex, we use `ordered_ccw` to determine the side.
    /// Otherwise, we test against the closest edge using `sign`.
    ///
    /// Corresponds to C++ `S2Polyline::IsOnRight`.
    ///
    /// # Panics
    ///
    /// Panics if the polyline has fewer than 2 vertices.
    pub fn is_on_right(&self, point: Point) -> bool {
        assert!(self.num_vertices() >= 2);
        let (closest_point, next_vertex) = self.project(point);

        // If the closest point is an interior vertex, check using ordered_ccw.
        if closest_point == self.vertex(next_vertex - 1)
            && next_vertex > 1
            && next_vertex < self.num_vertices()
        {
            if point == self.vertex(next_vertex - 1) {
                return false; // Polyline vertices are not on the RHS.
            }
            return predicates::ordered_ccw(
                self.vertex(next_vertex - 2),
                point,
                self.vertex(next_vertex),
                self.vertex(next_vertex - 1),
            );
        }

        // The closest point is on exactly one polyline edge.
        let nv = if next_vertex == self.num_vertices() {
            next_vertex - 1
        } else {
            next_vertex
        };
        predicates::sign(point, self.vertex(nv), self.vertex(nv - 1))
    }

    /// Reports whether this polyline intersects the given polyline. If the
    /// polylines share a vertex they are considered to be intersecting. When
    /// a polyline endpoint is the only intersection with the other polyline,
    /// the return value is undefined.
    ///
    /// Corresponds to C++ `S2Polyline::Intersects`.
    pub fn intersects(&self, line: &Polyline) -> bool {
        if self.num_vertices() == 0 || line.num_vertices() == 0 {
            return false;
        }
        if !self.rect_bound().intersects(line.rect_bound()) {
            return false;
        }
        for i in 1..self.num_vertices() {
            let mut crosser = EdgeCrosser::new(self.vertex(i - 1), self.vertex(i));
            for j in 1..line.num_vertices() {
                if crosser.crossing_sign(line.vertex(j - 1), line.vertex(j))
                    != crate::s2::edge_crossings::Crossing::DoNotCross
                {
                    return true;
                }
            }
        }
        false
    }

    /// Returns a subsequence of vertex indices such that the polyline
    /// connecting the subsequence of vertices is never farther than
    /// `tolerance` from the original polyline. The output always includes
    /// the first and last vertices, except when the polyline is empty.
    ///
    /// Corresponds to C++ `S2Polyline::SubsampleVertices`.
    pub fn subsample_vertices(&self, tolerance: Angle) -> Vec<usize> {
        let mut indices = Vec::new();
        if self.num_vertices() == 0 {
            return indices;
        }
        indices.push(0);
        let clamped_tolerance = if tolerance.radians() < 0.0 {
            Angle::from_radians(0.0)
        } else {
            tolerance
        };
        let mut index = 0;
        while index + 1 < self.num_vertices() {
            let next_index = find_end_vertex(self, clamped_tolerance, index);
            if self.vertex(next_index) != self.vertex(index) {
                indices.push(next_index);
            }
            index = next_index;
        }
        indices
    }

    /// Reports whether this polyline "nearly covers" the given polyline
    /// `covered`. This means that this polyline is within `max_error` of
    /// `covered`, following the "driving a car" analogy: you can drive one
    /// car along this polyline and another car along `covered`, never going
    /// backward, and the cars stay within `max_error` of each other.
    ///
    /// Corresponds to C++ `S2Polyline::NearlyCovers`.
    pub fn nearly_covers(&self, covered: &Polyline, max_error: Angle) -> bool {
        if covered.num_vertices() == 0 {
            return true;
        }
        if self.num_vertices() == 0 {
            return false;
        }

        let mut pending: Vec<(usize, usize, bool)> = Vec::new();
        let mut done: HashSet<(usize, usize, bool)> = HashSet::new();

        // Find all possible starting states.
        let mut i = 0;
        let mut next_i = next_distinct_vertex(self, 0);
        while next_i < self.num_vertices() {
            let next_next_i = next_distinct_vertex(self, next_i);
            let closest =
                edge_distances::project(covered.vertex(0), self.vertex(i), self.vertex(next_i));
            if (next_next_i == self.num_vertices() || closest != self.vertex(next_i))
                && closest.distance(covered.vertex(0)) <= max_error
            {
                pending.push((i, 0, true));
            }
            i = next_i;
            next_i = next_next_i;
        }

        while let Some(state) = pending.pop() {
            if !done.insert(state) {
                continue;
            }

            let (si, sj, i_in_progress) = state;
            let ni = next_distinct_vertex(self, si);
            let nj = next_distinct_vertex(covered, sj);
            if nj == covered.num_vertices() {
                return true;
            }
            if ni == self.num_vertices() {
                continue;
            }

            let (i_begin, j_begin);
            if i_in_progress {
                j_begin = covered.vertex(sj);
                i_begin = edge_distances::project(j_begin, self.vertex(si), self.vertex(ni));
            } else {
                i_begin = self.vertex(si);
                j_begin = edge_distances::project(i_begin, covered.vertex(sj), covered.vertex(nj));
            }

            if edge_distances::is_edge_b_near_edge_a(
                j_begin,
                covered.vertex(nj),
                i_begin,
                self.vertex(ni),
                max_error,
            ) {
                pending.push((ni, sj, false));
            }
            if edge_distances::is_edge_b_near_edge_a(
                i_begin,
                self.vertex(ni),
                j_begin,
                covered.vertex(nj),
                max_error,
            ) {
                pending.push((si, nj, true));
            }
        }
        false
    }

    /// Returns the internal vertices vector.
    pub fn vertices_vec(&self) -> &Vec<Point> {
        &self.vertices
    }
}

// ─── Helper functions ──────────────────────────────────────────────────

/// Returns the index of the next vertex that is different from vertex `index`.
fn next_distinct_vertex(polyline: &Polyline, index: usize) -> usize {
    let initial = polyline.vertex(index);
    let mut i = index + 1;
    while i < polyline.num_vertices() && polyline.vertex(i) == initial {
        i += 1;
    }
    i
}

/// Returns the largest `end` such that the polyline from `index` to `end` stays
/// within `tolerance` of the straight edge from `index` to `end`.
fn find_end_vertex(polyline: &Polyline, tolerance: Angle, index: usize) -> usize {
    debug_assert!(tolerance.radians() >= 0.0);
    debug_assert!(index + 1 < polyline.num_vertices());

    let origin = polyline.vertex(index);
    let frame = crate::s2::point::get_frame(origin);

    let mut current_wedge = s1::Interval::full();
    let mut last_distance: f64 = 0.0;

    let mut idx = index + 1;
    while idx < polyline.num_vertices() {
        let candidate = polyline.vertex(idx);
        let distance = origin.0.angle(candidate.0);

        // Don't create edges longer than 90 degrees.
        if distance > std::f64::consts::FRAC_PI_2 && last_distance > 0.0 {
            break;
        }

        // Vertices must be in increasing order along the ray.
        if distance < last_distance && last_distance > tolerance.radians() {
            break;
        }
        last_distance = distance;

        // Points within tolerance of origin don't constrain the ray.
        if distance <= tolerance.radians() {
            idx += 1;
            continue;
        }

        // Check if the current wedge contains this vertex's angle.
        let direction = crate::s2::point::to_frame(&frame, candidate);
        let center = direction.y().atan2(direction.x());
        if !current_wedge.contains(center) {
            break;
        }

        // Compute the half-angle of the allowable wedge.
        let half_angle = (tolerance.radians().sin() / distance.sin()).asin();
        let target = s1::Interval::from_point(center).expanded(half_angle);
        current_wedge = current_wedge.intersection(target);
        debug_assert!(!current_wedge.is_empty());
        idx += 1;
    }
    idx - 1
}

// ─── Shape implementation ───────────────────────────────────────────────

impl Shape for Polyline {
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
        Edge::new(self.vertices[offset], self.vertices[offset + 1])
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        ChainPosition::new(0, edge_id)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polyline
    }

    fn type_tag(&self) -> u32 {
        2 // S2Polyline::Shape::kTypeTag
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        _hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        use crate::s2::encoding::S2Encode;
        self.encode(w)
    }
}

// ─── Region implementation ──────────────────────────────────────────────

impl Region for Polyline {
    fn cap_bound(&self) -> Cap {
        self.rect_bound().cap_bound()
    }

    fn rect_bound(&self) -> Rect {
        let mut bounder = crate::s2::latlng_rect_bounder::LatLngRectBounder::new();
        for v in &self.vertices {
            bounder.add_point(*v);
        }
        bounder.get_bound()
    }

    fn cell_union_bound(&self) -> Vec<CellId> {
        self.cap_bound().cell_union_bound()
    }

    fn contains_cell(&self, _cell: &Cell) -> bool {
        false
    }

    fn intersects_cell(&self, cell: &Cell) -> bool {
        if self.vertices.is_empty() {
            return false;
        }
        // Check if any vertex is inside the cell.
        for v in &self.vertices {
            if cell.contains_point(v) {
                return true;
            }
        }
        // Check if any polyline edge crosses any cell edge.
        for j in 0..4 {
            let cell_a = cell.vertex(j);
            let cell_b = cell.vertex((j + 1) & 3);
            let mut crosser = EdgeCrosser::new(cell_a, cell_b);
            for i in 1..self.vertices.len() {
                if crosser.crossing_sign(self.vertices[i - 1], self.vertices[i])
                    != crate::s2::edge_crossings::Crossing::DoNotCross
                {
                    return true;
                }
            }
        }
        false
    }

    fn contains_point(&self, _p: &Point) -> bool {
        false
    }
}

impl std::ops::Deref for Polyline {
    type Target = [Point];
    fn deref(&self) -> &[Point] {
        &self.vertices
    }
}

impl PartialEq for Polyline {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn polyline_is_send_sync() {
        is_send_sync::<Polyline>();
    }

    #[test]
    fn test_empty_polyline() {
        let pl = Polyline::new(vec![]);
        assert_eq!(pl.num_edges(), 0);
        assert_eq!(pl.num_chains(), 0);
        assert_eq!(pl.dimension(), Dimension::Polyline);
        assert!(pl.is_empty());
        assert!(!pl.is_full());
        assert!(!pl.has_interior());
    }

    #[test]
    fn test_single_vertex() {
        let pl = Polyline::new(vec![p(0.0, 0.0)]);
        assert_eq!(pl.num_edges(), 0);
        assert_eq!(pl.num_chains(), 0);
        assert!(pl.length().radians().abs() < 1e-15);
    }

    #[test]
    fn test_two_vertices() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        assert_eq!(pl.num_edges(), 1);
        assert_eq!(pl.num_chains(), 1);
        let chain = pl.chain(0);
        assert_eq!(chain.start, 0);
        assert_eq!(chain.length, 1);
    }

    #[test]
    fn test_length() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let expected = std::f64::consts::FRAC_PI_2;
        assert!((pl.length().radians() - expected).abs() < 1e-13);
    }

    #[test]
    fn test_length_three_vertices() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        let expected = std::f64::consts::PI;
        assert!((pl.length().radians() - expected).abs() < 1e-13);
    }

    #[test]
    fn test_reverse() {
        let mut pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        let v0 = pl.vertex(0);
        let v2 = pl.vertex(2);
        pl.reverse();
        assert_eq!(pl.vertex(0), v2);
        assert_eq!(pl.vertex(2), v0);
    }

    #[test]
    fn test_interpolate() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let (pt, _) = pl.interpolate(0.0);
        assert!(pt.distance(p(0.0, 0.0)).radians() < 1e-15);
        let (pt, _) = pl.interpolate(1.0);
        assert!(pt.distance(p(0.0, 90.0)).radians() < 1e-15);
        let (pt, _) = pl.interpolate(0.5);
        assert!(pt.distance(p(0.0, 45.0)).radians() < 1e-10);
    }

    #[test]
    fn test_validate_ok() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        assert!(pl.validate().is_ok());
    }

    #[test]
    fn test_validate_duplicate() {
        let v = p(0.0, 0.0);
        let pl = Polyline::new(vec![v, v]);
        assert!(pl.validate().is_err());
    }

    #[test]
    fn test_region_bounds() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(45.0, 90.0)]);
        let cap = pl.cap_bound();
        assert!(!cap.is_empty());
        assert!(cap.contains_point(p(0.0, 0.0)));
        assert!(cap.contains_point(p(45.0, 90.0)));

        let rect = pl.rect_bound();
        assert!(!rect.is_empty());
    }

    #[test]
    fn test_contains_point_always_false() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        assert!(!pl.contains_point(&p(0.0, 0.0)));
    }

    #[test]
    fn test_deref() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let slice: &[Point] = &pl;
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_equal() {
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let b = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let c = Polyline::new(vec![p(0.0, 0.0)]);
        assert!(a.equal(&b));
        assert!(!a.equal(&c));
    }

    #[test]
    fn test_from_lat_lngs() {
        let latlngs = vec![
            LatLng::from_degrees(0.0, 0.0),
            LatLng::from_degrees(0.0, 90.0),
        ];
        let pl = Polyline::from_lat_lngs(&latlngs);
        assert_eq!(pl.num_vertices(), 2);
    }

    #[test]
    fn test_polyline_project_and_interpolate() {
        // A three-segment polyline along the equator: 0->30->60->90 degrees longitude.
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 30.0), p(0.0, 60.0), p(0.0, 90.0)]);

        // Project the midpoint of the polyline (should be near (0, 45)).
        let mid = p(0.0, 45.0);
        let (projected, next_vertex) = pl.project(mid);
        // The projected point should be very close to the original point
        // since the point lies on the equator and the polyline runs along it.
        assert!(
            projected.distance(mid).radians() < 1e-10,
            "projected point should be very close to the query point"
        );
        // next_vertex should be 2 or 3 (the edge between vertex 1 and 2).
        assert!(
            next_vertex >= 1 && next_vertex <= pl.num_vertices(),
            "next_vertex {next_vertex} out of range"
        );

        // Interpolate at endpoints.
        let (start, _) = pl.interpolate(0.0);
        assert!(
            start.distance(p(0.0, 0.0)).radians() < 1e-15,
            "interpolate(0) should return the first vertex"
        );
        let (end, _) = pl.interpolate(1.0);
        assert!(
            end.distance(p(0.0, 90.0)).radians() < 1e-15,
            "interpolate(1) should return the last vertex"
        );

        // Interpolate at fraction 0.5 should give the midpoint ~(0, 45).
        let (half, half_next) = pl.interpolate(0.5);
        assert!(
            half.distance(p(0.0, 45.0)).radians() < 1e-10,
            "interpolate(0.5) distance = {}, expected ~0",
            half.distance(p(0.0, 45.0)).radians()
        );

        // Roundtrip: uninterpolate the interpolated point should give back ~0.5.
        let frac = pl.uninterpolate(half, half_next);
        assert!(
            (frac - 0.5).abs() < 1e-10,
            "uninterpolate roundtrip: got {frac}, expected ~0.5"
        );
    }

    #[test]
    fn test_polyline_region_intersects_cell() {
        // A polyline crossing through a cell. Pick a cell at a moderate level
        // and route a polyline through it.
        let center = p(10.0, 20.0);
        let cell_id = CellId::from_point(&center).parent_at_level(10);
        let cell = Cell::from(cell_id);

        // Build a polyline that starts well outside the cell, passes through
        // the center, and ends well outside the cell on the other side.
        let pl = Polyline::new(vec![p(9.0, 19.0), p(10.0, 20.0), p(11.0, 21.0)]);
        assert!(
            pl.intersects_cell(&cell),
            "polyline through cell center should intersect the cell"
        );

        // A polyline far away should not intersect.
        let far = Polyline::new(vec![p(-80.0, -170.0), p(-80.0, -169.0)]);
        assert!(
            !far.intersects_cell(&cell),
            "distant polyline should not intersect the cell"
        );
    }

    #[test]
    fn test_is_on_right() {
        // Polyline going from (0,0) to (0,10).
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        // Point to the right (south side of an east-going equatorial line).
        assert!(pl.is_on_right(p(-1.0, 5.0)));
        // Point to the left (north side).
        assert!(!pl.is_on_right(p(1.0, 5.0)));
    }

    #[test]
    fn test_is_on_right_at_vertex() {
        // Polyline with a bend: goes north then east.
        let pl = Polyline::new(vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0)]);
        // Point on the polyline vertex.
        assert!(!pl.is_on_right(p(10.0, 0.0)));
    }

    #[test]
    fn test_is_on_right_multi_edge() {
        // Polyline going east along the equator.
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(0.0, 20.0)]);
        // Point south of the polyline = right side.
        assert!(pl.is_on_right(p(-1.0, 10.0)));
        // Point north of the polyline = left side.
        assert!(!pl.is_on_right(p(1.0, 10.0)));
    }

    #[test]
    fn test_intersects_crossing() {
        // Two polylines that cross.
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let b = Polyline::new(vec![p(-1.0, 5.0), p(1.0, 5.0)]);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn test_intersects_non_crossing() {
        // Two parallel polylines.
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let b = Polyline::new(vec![p(5.0, 0.0), p(5.0, 10.0)]);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn test_intersects_empty() {
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let b = Polyline::new(vec![]);
        assert!(!a.intersects(&b));
        assert!(!b.intersects(&a));
    }

    #[test]
    fn test_intersects_shared_vertex() {
        // Polylines that share an endpoint.
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let b = Polyline::new(vec![p(0.0, 10.0), p(10.0, 10.0)]);
        // Shared vertex counts as intersection.
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_subsample_vertices_straight_line() {
        // A straight line along the equator shouldn't need intermediate vertices.
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(0.0, 20.0), p(0.0, 30.0)]);
        let indices = pl.subsample_vertices(Angle::from_degrees(1.0));
        // Should keep first and last.
        assert_eq!(*indices.first().unwrap(), 0);
        assert_eq!(*indices.last().unwrap(), 3);
        // May or may not keep intermediate vertices, but should have at most 4.
        assert!(indices.len() <= 4);
    }

    #[test]
    fn test_subsample_vertices_bent_line() {
        // A polyline with a significant bend — subsample should keep the bend vertex.
        let pl = Polyline::new(vec![p(0.0, 0.0), p(10.0, 20.0), p(0.0, 40.0)]);
        // With very small tolerance, all vertices should be kept.
        let indices = pl.subsample_vertices(Angle::from_degrees(0.001));
        assert_eq!(indices.len(), 3);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_subsample_vertices_empty() {
        let pl = Polyline::new(vec![]);
        let indices = pl.subsample_vertices(Angle::from_degrees(1.0));
        assert!(indices.is_empty());
    }

    #[test]
    fn test_nearly_covers_self() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(0.0, 20.0)]);
        // A polyline nearly covers itself.
        assert!(pl.nearly_covers(&pl, Angle::from_degrees(0.001)));
    }

    #[test]
    fn test_nearly_covers_empty() {
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let empty = Polyline::new(vec![]);
        // Any polyline covers an empty polyline.
        assert!(pl.nearly_covers(&empty, Angle::from_degrees(1.0)));
        // An empty polyline does not cover a non-empty one.
        assert!(!empty.nearly_covers(&pl, Angle::from_degrees(1.0)));
    }

    #[test]
    fn test_nearly_covers_nearby_polyline() {
        // A polyline slightly off the equator should be covered by an equatorial polyline
        // within a sufficient error.
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0)]);
        let b = Polyline::new(vec![p(0.01, 0.0), p(0.01, 10.0)]);
        // 0.01 degrees is about 1.1 km, so 0.1 degree tolerance should cover it.
        assert!(a.nearly_covers(&b, Angle::from_degrees(0.1)));
        // Very tight tolerance should fail.
        assert!(!a.nearly_covers(&b, Angle::from_degrees(0.001)));
    }

    #[test]
    fn test_nearly_covers_partial() {
        // A shorter polyline (subset of a longer one) should be covered.
        let a = Polyline::new(vec![p(0.0, 0.0), p(0.0, 20.0)]);
        let b = Polyline::new(vec![p(0.0, 5.0), p(0.0, 15.0)]);
        assert!(a.nearly_covers(&b, Angle::from_degrees(0.001)));
        // But the longer one is not covered by the shorter one.
        assert!(!b.nearly_covers(&a, Angle::from_degrees(0.001)));
    }

    // ===== NearlyCovers tests (ported from C++ S2PolylineCoveringTest) =====

    fn check_nearly_covers(
        a_str: &str,
        b_str: &str,
        max_error_degrees: f64,
        expect_b_covers_a: bool,
        expect_a_covers_b: bool,
    ) {
        use crate::s2::text_format::make_polyline;
        let a = make_polyline(a_str);
        let b = make_polyline(b_str);
        let max_error = Angle::from_degrees(max_error_degrees);
        assert_eq!(
            expect_b_covers_a,
            b.nearly_covers(&a, max_error),
            "b.nearly_covers(a) failed for a={a_str}, b={b_str}, max_error={max_error_degrees}",
        );
        assert_eq!(
            expect_a_covers_b,
            a.nearly_covers(&b, max_error),
            "a.nearly_covers(b) failed for a={a_str}, b={b_str}, max_error={max_error_degrees}",
        );
    }

    #[test]
    fn test_nearly_covers_overlaps_self() {
        let pline = "1:1, 2:2, -1:10";
        check_nearly_covers(pline, pline, 1e-10, true, true);
    }

    #[test]
    fn test_nearly_covers_does_not_overlap_reverse() {
        check_nearly_covers("1:1, 2:2, -1:10", "-1:10, 2:2, 1:1", 1e-10, false, false);
    }

    #[test]
    fn test_nearly_covers_overlaps_equivalent() {
        // These two polylines trace the exact same path, but the second one uses
        // three points instead of two.
        check_nearly_covers("1:1, 2:1", "1:1, 1.5:1, 2:1", 1e-10, true, true);
    }

    #[test]
    fn test_nearly_covers_short_covered_by_long() {
        // The second polyline is always within 0.001 degrees of the first polyline,
        // but the first polyline is too long to be covered by the second.
        check_nearly_covers(
            "-5:1, 10:1, 10:5, 5:10",
            "9:1, 9.9995:1, 10.0005:5",
            1e-3,
            false,
            true,
        );
    }

    #[test]
    fn test_nearly_covers_partial_overlap_only() {
        // Neither polyline fully overlaps the other.
        check_nearly_covers("-5:1, 10:1", "0:1, 20:1", 1.0, false, false);
    }

    #[test]
    fn test_nearly_covers_short_backtracking() {
        // Two lines that backtrack a bit (less than 1.5 degrees) on different edges.
        let t1 = "0:0, 0:2, 0:1, 0:4, 0:5";
        let t2 = "0:0, 0:2, 0:4, 0:3, 0:5";
        check_nearly_covers(t1, t2, 1.5, true, true);
        check_nearly_covers(t1, t2, 0.5, false, false);
    }

    #[test]
    fn test_nearly_covers_long_backtracking() {
        // Two arcs with opposite direction do not overlap if the shorter arc is
        // longer than max_error, but do if the shorter arc is shorter.
        check_nearly_covers("5:1, -5:1", "1:1, 3:1", 1.0, false, false);
        check_nearly_covers("5:1, -5:1", "1:1, 3:1", 2.5, false, true);
    }

    #[test]
    fn test_nearly_covers_choose_between_starting_points() {
        // Can handle two possible starting points, only one of which leads to
        // finding a correct path.
        check_nearly_covers("0:11, 0:0, 0:9, 0:20", "0:10, 0:15", 1.5, false, true);
    }

    #[test]
    fn test_nearly_covers_straight_and_wiggly() {
        check_nearly_covers(
            "40:1, 20:1",
            "39.9:0.9, 40:1.1, 30:1.15, 29:0.95, 28:1.1, 27:1.15, \
             26:1.05, 25:0.85, 24:1.1, 23:0.9, 20:0.99",
            0.2,
            true,
            true,
        );
    }

    #[test]
    fn test_nearly_covers_match_starts_at_last_vertex() {
        // The first polyline covers the second, but the matching segment starts at
        // the last vertex of the first polyline.
        check_nearly_covers("0:0, 0:2", "0:2, 0:3", 1.5, false, true);
    }

    // ===== Shape interface tests (ported from C++ s2polyline_test.cc) =====

    #[test]
    fn test_polyline_shape_basic() {
        use crate::s2::shape::Shape;
        let pl = Polyline::new(vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(2.0, 1.0)]);
        assert_eq!(pl.num_edges(), 3);
        assert_eq!(pl.num_chains(), 1);
        assert_eq!(pl.chain(0).start, 0);
        assert_eq!(pl.chain(0).length, 3);
        let edge2 = pl.edge(2);
        assert_eq!(edge2.v0, LatLng::from_degrees(1.0, 1.0).to_point());
        assert_eq!(edge2.v1, LatLng::from_degrees(2.0, 1.0).to_point());
        assert_eq!(pl.dimension(), Dimension::Polyline);
        assert!(!pl.is_empty());
        assert!(!pl.is_full());
        assert!(!pl.reference_point().contained);
    }

    #[test]
    fn test_polyline_shape_empty() {
        use crate::s2::shape::Shape;
        let pl = Polyline::new(vec![]);
        assert_eq!(pl.num_edges(), 0);
        assert_eq!(pl.num_chains(), 0);
        assert!(pl.is_empty());
        assert!(!pl.is_full());
        assert!(!pl.reference_point().contained);
    }

    #[test]
    fn test_polyline_shape_single_vertex() {
        use crate::s2::shape::Shape;
        // A polyline with a single vertex has no edges.
        let pl = Polyline::new(vec![p(0.0, 0.0)]);
        assert_eq!(pl.num_edges(), 0);
        assert_eq!(pl.num_chains(), 0);
        assert!(pl.is_empty());
        assert!(!pl.is_full());
    }

    // ===== MayIntersect/intersects_cell tests (ported from C++ s2polyline_test.cc) =====

    #[test]
    fn test_polyline_intersects_cell() {
        use crate::s2::cell::Cell;
        use crate::s2::cell_id::CellId;
        use crate::s2::region::Region;
        // A short polyline near face 0.
        let a = Point::from_coords(1.0, -1.1, 0.8).normalize();
        let b = Point::from_coords(1.0, -0.8, 1.1).normalize();
        let line = Polyline::new(vec![a, b]);
        for face in 0u8..6 {
            let cell = Cell::from_cell_id(CellId::from_face(face));
            // The polyline should intersect faces 0, 2, 4 (even faces).
            assert_eq!((face & 1) == 0, line.intersects_cell(&cell), "face {face}");
        }
    }

    // ===== Reverse tests (ported from C++ s2polyline_test.cc) =====

    #[test]
    fn test_polyline_reverse() {
        let mut pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(5.0, 5.0)]);
        let v0 = pl.vertex(0);
        let v2 = pl.vertex(2);
        pl.reverse();
        assert_eq!(pl.vertex(0), v2);
        assert_eq!(pl.vertex(2), v0);
    }

    // ===== Rect bound tests (ported from C++ s2polyline_test.cc) =====

    #[test]
    fn test_polyline_rect_bound_empty() {
        use crate::s2::region::Region;
        let pl = Polyline::new(vec![]);
        assert!(pl.rect_bound().is_empty());
    }

    #[test]
    fn test_polyline_rect_bound_equator() {
        use crate::s2::region::Region;
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0), p(0.0, 180.0)]);
        let bound = pl.rect_bound();
        // Latitude should be near zero.
        assert!(bound.lat.lo.abs() < 1e-14);
        assert!(bound.lat.hi.abs() < 1e-14);
        // Longitude should span 0 to PI.
        assert!(bound.lng.lo.abs() < 1e-14);
        assert!((bound.lng.hi - std::f64::consts::PI).abs() < 1e-14);
    }

    // ===== Length and centroid tests =====

    #[test]
    fn test_polyline_length() {
        // Equator arc from 0:0 to 0:90 = PI/2 radians
        let pl = Polyline::new(vec![p(0.0, 0.0), p(0.0, 90.0)]);
        let length = pl.length();
        assert!(
            (length.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-14,
            "length = {}",
            length.radians()
        );
    }

    #[test]
    fn test_polyline_centroid() {
        // Symmetric polyline should have centroid along the equator.
        let pl = Polyline::new(vec![p(0.0, -10.0), p(0.0, 10.0)]);
        let centroid = pl.centroid();
        // Should point toward (0, 0).
        let ll = LatLng::from_point(centroid.normalize());
        assert!(ll.lat.degrees().abs() < 1e-10);
        assert!(ll.lng.degrees().abs() < 1e-10);
    }

    // ===== Intersection, projection, interpolation tests (ported from C++) =====

    #[test]
    fn test_intersects_empty_polyline() {
        use crate::s2::text_format::make_polyline;
        let line1 = make_polyline("1:1, 4:4");
        let empty = Polyline::new(vec![]);
        assert!(!empty.intersects(&line1));
    }

    #[test]
    fn test_intersects_one_point_polyline() {
        use crate::s2::text_format::make_polyline;
        let line1 = make_polyline("1:1, 4:4");
        let line2 = make_polyline("1:1");
        assert!(!line1.intersects(&line2));
    }

    #[test]
    fn test_intersects() {
        use crate::s2::text_format::make_polyline;
        let line1 = make_polyline("1:1, 4:4");
        let small_crossing = make_polyline("1:2, 2:1");
        let small_noncrossing = make_polyline("1:2, 2:3");
        let big_crossing = make_polyline("1:2, 2:3, 4:3");

        assert!(line1.intersects(&small_crossing));
        assert!(!line1.intersects(&small_noncrossing));
        assert!(line1.intersects(&big_crossing));
    }

    #[test]
    fn test_intersects_at_vertex() {
        use crate::s2::text_format::make_polyline;
        let line1 = make_polyline("1:1, 4:4, 4:6");
        let line2 = make_polyline("1:1, 1:2");
        let line3 = make_polyline("5:1, 4:4, 2:2");
        assert!(line1.intersects(&line2));
        assert!(line1.intersects(&line3));
    }

    #[test]
    fn test_intersects_vertex_on_edge() {
        use crate::s2::text_format::make_polyline;
        let h_lr = make_polyline("0:1, 0:3");
        let v_bt = make_polyline("-1:2, 0:2, 1:2");
        let h_rl = make_polyline("0:3, 0:1");
        let v_tb = make_polyline("1:2, 0:2, -1:2");
        assert!(h_lr.intersects(&v_bt));
        assert!(h_lr.intersects(&v_tb));
        assert!(h_rl.intersects(&v_bt));
        assert!(h_rl.intersects(&v_tb));
    }

    #[test]
    fn test_no_data() {
        let mut poly = Polyline::new(vec![]);
        assert_eq!(poly.length(), Angle::from_radians(0.0));
        let centroid = poly.centroid();
        assert_eq!(centroid.0.x, 0.0);
        assert_eq!(centroid.0.y, 0.0);
        assert_eq!(centroid.0.z, 0.0);
        poly.reverse(); // Should not panic.
    }

    #[test]
    fn test_no_data_clone() {
        let poly = Polyline::new(vec![]);
        let cloned = poly.clone();
        assert_eq!(cloned.num_vertices(), 0);
    }

    #[test]
    fn test_approx_equals() {
        use crate::s2::text_format::make_polyline;
        let degree = Angle::from_degrees(1.0);

        // Close lines, differences within max_error.
        assert!(
            make_polyline("0:0, 0:10, 5:5")
                .approx_eq_with(&make_polyline("0:0.1, -0.1:9.9, 5:5.2"), degree * 0.5)
        );

        // Close lines, differences outside max_error.
        assert!(
            !make_polyline("0:0, 0:10, 5:5")
                .approx_eq_with(&make_polyline("0:0.1, -0.1:9.9, 5:5.2"), degree * 0.01)
        );

        // Same line, but different number of vertices.
        assert!(
            !make_polyline("0:0, 0:10, 0:20")
                .approx_eq_with(&make_polyline("0:0, 0:20"), degree * 0.1)
        );

        // Same vertices, in different order.
        assert!(
            !make_polyline("0:0, 5:5, 0:10")
                .approx_eq_with(&make_polyline("5:5, 0:10, 0:0"), degree * 0.1)
        );
    }

    #[test]
    fn test_project() {
        let line = Polyline::new(vec![p(0.0, 0.0), p(0.0, 1.0), p(0.0, 2.0), p(1.0, 2.0)]);

        // Point off the start.
        let (proj, next) = line.project(p(0.5, -0.5));
        assert!(proj.approx_eq(p(0.0, 0.0)));
        assert_eq!(next, 1);

        // Point near middle of first edge.
        let (proj, next) = line.project(p(0.5, 0.5));
        assert!(proj.approx_eq(p(0.0, 0.5)));
        assert_eq!(next, 1);

        // Point near vertex 1.
        let (proj, next) = line.project(p(0.5, 1.0));
        assert!(proj.approx_eq(p(0.0, 1.0)));
        assert_eq!(next, 2);

        // Point off the right side near edge 1-2.
        let (proj, next) = line.project(p(-0.5, 2.5));
        assert!(proj.approx_eq(p(0.0, 2.0)));
        assert_eq!(next, 3);

        // Point beyond end.
        let (proj, next) = line.project(p(2.0, 2.0));
        assert!(proj.approx_eq(p(1.0, 2.0)));
        assert_eq!(next, 4);

        // Single-vertex polyline projects all points to that vertex.
        let single = Polyline::new(vec![p(1.0, 1.0)]);
        let (proj, next) = single.project(p(2.0, 2.0));
        assert!(proj.approx_eq(p(1.0, 1.0)));
        assert_eq!(next, 1);
        let (proj, next) = single.project(p(-1.0, 0.0));
        assert!(proj.approx_eq(p(1.0, 1.0)));
        assert_eq!(next, 1);
    }

    #[test]
    fn test_uninterpolate() {
        // Single-vertex polyline.
        let point_line = Polyline::new(vec![Point::from_coords(1.0, 0.0, 0.0)]);
        assert!(
            (point_line.uninterpolate(Point::from_coords(0.0, 1.0, 0.0), 1) - 0.0).abs() < 1e-15
        );

        // Multi-segment polyline.
        let line = Polyline::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 1.0, 1.0).normalize(),
            Point::from_coords(0.0, 0.0, 1.0),
        ]);

        let (interp, next) = line.interpolate(-0.1);
        assert!((line.uninterpolate(interp, next) - 0.0).abs() < 1e-15);

        let (interp, next) = line.interpolate(0.0);
        assert!((line.uninterpolate(interp, next) - 0.0).abs() < 1e-15);

        let (interp, next) = line.interpolate(0.5);
        assert!((line.uninterpolate(interp, next) - 0.5).abs() < 1e-15);

        let (interp, next) = line.interpolate(0.75);
        assert!((line.uninterpolate(interp, next) - 0.75).abs() < 1e-15);

        let (interp, next) = line.interpolate(1.1);
        assert!((line.uninterpolate(interp, next) - 1.0).abs() < 1e-15);

        // Check clamped to 1.0.
        assert!(
            (line.uninterpolate(Point::from_coords(0.0, 1.0, 0.0), line.num_vertices()) - 1.0)
                .abs()
                < 1e-15
        );
    }

    // ===== Subsample tests (ported from C++ s2polyline_test.cc) =====

    fn check_subsample(polyline_str: &str, tolerance_degrees: f64, expected_indices: &[usize]) {
        use crate::s2::text_format::make_polyline;
        let polyline = make_polyline(polyline_str);
        let indices = polyline.subsample_vertices(Angle::from_degrees(tolerance_degrees));
        assert_eq!(
            indices, expected_indices,
            "polyline=\"{polyline_str}\", tolerance={tolerance_degrees}"
        );
    }

    #[test]
    fn test_subsample_vertices_trivial_inputs() {
        // No vertices.
        check_subsample("", 1.0, &[]);
        // One vertex.
        check_subsample("0:1", 1.0, &[0]);
        // Two vertices.
        check_subsample("10:10, 11:11", 5.0, &[0, 1]);
        // Three points on a straight line (near-zero tolerance).
        check_subsample("-1:0, 0:0, 1:0", 1e-15, &[0, 2]);
        // Zero tolerance on a non-straight line.
        check_subsample("-1:0, 0:0, 1:1", 0.0, &[0, 1, 2]);
        // Negative tolerance should return all vertices.
        check_subsample("-1:0, 0:0, 1:1", -1.0, &[0, 1, 2]);
        // Non-zero tolerance with a straight line.
        check_subsample("0:1, 0:2, 0:3, 0:4, 0:5", 1.0, &[0, 4]);
    }

    #[test]
    fn test_subsample_vertices_simple_example() {
        let poly_str = "0:0, 0:1, -1:2, 0:3, 0:4, 1:4, 2:4.5, 3:4, 3.5:4, 4:4";
        check_subsample(poly_str, 3.0, &[0, 9]);
        check_subsample(poly_str, 2.0, &[0, 6, 9]);
        check_subsample(poly_str, 0.9, &[0, 2, 6, 9]);
        check_subsample(poly_str, 0.4, &[0, 1, 2, 3, 4, 6, 9]);
        check_subsample(poly_str, 0.0, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_subsample_vertices_guarantees() {
        // Check that duplicate vertices are never generated.
        check_subsample("10:10, 12:12, 10:10", 5.0, &[0]);
        check_subsample("0:0, 1:1, 0:0, 0:120, 0:130", 5.0, &[0, 3, 4]);

        // Check that points are not collapsed if they would create a line
        // segment longer than 90 degrees.
        check_subsample(
            "90:0, 50:180, 20:180, -20:180, -50:180, -90:0, 30:0, 90:0",
            5.0,
            &[0, 2, 4, 5, 6, 7],
        );

        // Check that backtracking is preserved (parametric equivalence).
        check_subsample("10:10, 10:20, 10:30, 10:15, 10:40", 5.0, &[0, 2, 3, 4]);
        check_subsample(
            "10:10, 10:20, 10:30, 10:10, 10:30, 10:40",
            5.0,
            &[0, 2, 3, 5],
        );
        check_subsample("10:10, 12:12, 9:9, 10:20, 10:30", 5.0, &[0, 4]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let pl = Polyline::new(vec![
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            Point::from_coords(0.0, 0.0, 1.0),
        ]);
        let json = serde_json::to_string(&pl).unwrap();
        let back: Polyline = serde_json::from_str(&json).unwrap();
        assert_eq!(pl.num_vertices(), back.num_vertices());
        for i in 0..pl.num_vertices() {
            assert_eq!(pl.vertex(i), back.vertex(i));
        }
    }
}
