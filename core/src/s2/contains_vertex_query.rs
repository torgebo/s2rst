// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Vertex containment query for polygons.
//!
//! [`ContainsVertexQuery`] tracks the edges entering and leaving a vertex of a
//! polygon to determine whether the point is contained. Point containment is
//! defined according to the semi-open boundary model: if several polygons tile
//! the region around a vertex, exactly one of them contains that vertex.
//!
//! Corresponds to C++ `s2contains_vertex_query.h`, Go `s2/contains_vertex_query.go`.

use std::collections::HashMap;

use crate::s2::Point;
use crate::s2::predicates;

/// Tracks edges entering and leaving a vertex to determine containment.
///
/// The target vertex is specified at construction. Edges are added via
/// [`add_edge`](Self::add_edge), then [`contains_vertex`](Self::contains_vertex) returns the result.
///
/// # Examples
///
/// ```
/// use s2rst::s2::contains_vertex_query::ContainsVertexQuery;
/// use s2rst::s2::LatLng;
///
/// // Determine whether vertex v is contained by a triangle.
/// let v0 = LatLng::from_degrees(0.0, 0.0).to_point();
/// let v1 = LatLng::from_degrees(0.0, 1.0).to_point();
/// let v2 = LatLng::from_degrees(1.0, 0.0).to_point();
///
/// // Query containment of v0: add the two edges leaving v0.
/// let mut q = ContainsVertexQuery::new(v0);
/// q.add_edge(v1, 1);  // outgoing edge
/// q.add_edge(v2, -1); // incoming edge
/// // Result depends on semi-open boundary rules.
/// let _ = q.contains_vertex();
/// ```
#[derive(Debug)]
pub struct ContainsVertexQuery {
    target: Point,
    edge_map: HashMap<OrderedPoint, i32>,
}

/// A wrapper around Point that provides Eq + Hash for use as a `HashMap` key.
/// Points are compared by their raw bit patterns.
#[derive(Clone, Copy, Debug)]
struct OrderedPoint(Point);

impl PartialEq for OrderedPoint {
    fn eq(&self, other: &Self) -> bool {
        self.0.0.x.to_bits() == other.0.0.x.to_bits()
            && self.0.0.y.to_bits() == other.0.0.y.to_bits()
            && self.0.0.z.to_bits() == other.0.0.z.to_bits()
    }
}

impl Eq for OrderedPoint {}

impl std::hash::Hash for OrderedPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.0.x.to_bits().hash(state);
        self.0.0.y.to_bits().hash(state);
        self.0.0.z.to_bits().hash(state);
    }
}

impl ContainsVertexQuery {
    /// Creates a new query for the given target vertex.
    pub fn new(target: Point) -> Self {
        ContainsVertexQuery {
            target,
            edge_map: HashMap::new(),
        }
    }

    /// Reinitializes the query for a new target vertex.
    pub fn init(&mut self, target: Point) {
        self.target = target;
        self.edge_map.clear();
    }

    /// Returns true if any duplicate edges were seen incident on target.
    ///
    /// A duplicate is an edge seen more than once with the same orientation.
    pub fn duplicate_edges(&self) -> bool {
        self.edge_map.values().any(|&v| v.abs() >= 2)
    }

    /// Adds the edge between `target` and `v` with the given direction.
    ///
    /// - `direction = 1` means outgoing (target → v)
    /// - `direction = -1` means incoming (v → target)
    /// - `direction = 0` means degenerate
    pub fn add_edge(&mut self, v: Point, direction: i32) {
        *self.edge_map.entry(OrderedPoint(v)).or_insert(0) += direction;
    }

    /// Reports whether the target vertex is contained.
    ///
    /// Returns `1` if contained, `-1` if not contained, and `0` if the
    /// incident edges consisted of matched sibling pairs (ambiguous).
    pub fn contains_vertex(&self) -> i32 {
        // Find the unmatched edge that is immediately clockwise from
        // the reference direction.
        let ref_dir = self.target.reference_dir();
        let mut best_point = ref_dir;
        let mut best_dir = 0i32;

        for (&OrderedPoint(k), &v) in &self.edge_map {
            if v == 0 {
                continue; // Matched edge
            }
            if predicates::ordered_ccw(ref_dir, best_point, k, self.target) {
                best_point = k;
                best_dir = v;
            }
        }
        best_dir
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::r3::Vector;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point(Vector { x, y, z }.normalize())
    }

    #[test]
    fn test_contains_vertex_query_no_edges() {
        let q = ContainsVertexQuery::new(p(1.0, 0.0, 0.0));
        assert_eq!(q.contains_vertex(), 0);
    }

    #[test]
    fn test_contains_vertex_query_matched_edges() {
        let target = p(1.0, 0.0, 0.0);
        let mut q = ContainsVertexQuery::new(target);
        let v = p(0.0, 1.0, 0.0);
        q.add_edge(v, 1);
        q.add_edge(v, -1);
        assert_eq!(q.contains_vertex(), 0);
    }

    #[test]
    fn test_contains_vertex_query_contained() {
        // Create a simple loop around the target where the vertex is contained.
        let target = p(1.0, 0.0, 0.0);
        let mut q = ContainsVertexQuery::new(target);
        // Add a CCW loop: outgoing to v1, incoming from v2.
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        q.add_edge(v1, 1); // outgoing
        q.add_edge(v2, -1); // incoming
        let result = q.contains_vertex();
        // The result should be non-zero.
        assert_ne!(result, 0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2contains_vertex_query_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_undetermined() {
        // C++ TEST(S2ContainsVertexQuery, Undetermined)
        let target = crate::s2::text_format::parse_point("1:2");
        let edge = crate::s2::text_format::parse_point("3:4");
        let mut q = ContainsVertexQuery::new(target);
        q.add_edge(edge, 1);
        q.add_edge(edge, -1);
        assert_eq!(0, q.contains_vertex());
        assert!(!q.duplicate_edges());
    }

    #[test]
    fn test_contained_with_duplicates() {
        // C++ TEST(S2ContainsVertexQuery, ContainedWithDuplicates)
        let target = crate::s2::text_format::parse_point("0:0");
        let mut q = ContainsVertexQuery::new(target);
        q.add_edge(crate::s2::text_format::parse_point("3:-3"), -1);
        q.add_edge(crate::s2::text_format::parse_point("1:-5"), 1);
        q.add_edge(crate::s2::text_format::parse_point("2:-4"), 1);
        q.add_edge(crate::s2::text_format::parse_point("1:-5"), -1);
        assert_eq!(1, q.contains_vertex());
        assert!(!q.duplicate_edges());

        // Incoming and outgoing edges to 1:-5 cancel, so one more isn't a duplicate.
        q.add_edge(crate::s2::text_format::parse_point("1:-5"), -1);
        assert!(!q.duplicate_edges());

        // 3:-3 has only been seen once incoming, another time is a duplicate.
        q.add_edge(crate::s2::text_format::parse_point("3:-3"), -1);
        assert!(q.duplicate_edges());
    }

    #[test]
    fn test_not_contained_with_duplicates() {
        // C++ TEST(S2ContainsVertexQuery, NotContainedWithDuplicates)
        let target = crate::s2::text_format::parse_point("1:1");
        let mut q = ContainsVertexQuery::new(target);
        q.add_edge(crate::s2::text_format::parse_point("1:-5"), 1);
        q.add_edge(crate::s2::text_format::parse_point("2:-4"), -1);
        q.add_edge(crate::s2::text_format::parse_point("3:-3"), 1);
        q.add_edge(crate::s2::text_format::parse_point("1:-5"), -1);
        assert_eq!(-1, q.contains_vertex());
        assert!(!q.duplicate_edges());

        q.add_edge(crate::s2::text_format::parse_point("1:-5"), -1);
        assert!(!q.duplicate_edges());

        q.add_edge(crate::s2::text_format::parse_point("3:-3"), 1);
        assert!(q.duplicate_edges());
    }

    #[test]
    fn test_compatible_with_angle_contains_vertex() {
        // C++ TEST(S2ContainsVertexQuery, CompatibleWithAngleContainsVertex)
        use crate::s2::edge_crossings::angle_contains_vertex;
        let center = crate::s2::text_format::parse_point("89:1");
        let points = crate::s2::testing::make_regular_points(
            center,
            crate::s1::Angle::from_degrees(5.0),
            10,
        );
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let c = points[(i + 2) % points.len()];
            let mut q = ContainsVertexQuery::new(b);
            q.add_edge(a, -1);
            q.add_edge(c, 1);
            assert_eq!(
                q.contains_vertex() > 0,
                angle_contains_vertex(a, b, c),
                "mismatch at vertex {i}"
            );
            assert!(!q.duplicate_edges());
        }
    }

    #[test]
    fn test_compatible_with_angle_contains_vertex_degenerate() {
        // C++ TEST(S2ContainsVertexQuery, CompatibleWithAngleContainsVertexDegenerate)
        use crate::s2::edge_crossings::angle_contains_vertex;
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let mut q = ContainsVertexQuery::new(b);
        q.add_edge(a, -1);
        q.add_edge(a, 1);
        assert_eq!(q.contains_vertex() > 0, angle_contains_vertex(a, b, a));
        assert!(!q.duplicate_edges());
    }
}
