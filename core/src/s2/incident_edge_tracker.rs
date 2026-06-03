// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Tracks shape edges that are incident on the same vertex.
//!
//! Ported from Java `S2IncidentEdgeTracker`. This is useful for detecting
//! vertices where more than two edges meet (which often indicates self-
//! intersections or complex topology).
//!
//! # Usage
//!
//! ```
//! use s2rst::s2::incident_edge_tracker::IncidentEdgeTracker;
//! use s2rst::s2::{LatLng, Point};
//!
//! let a = LatLng::from_degrees(1.0, 1.0).to_point();
//! let b = LatLng::from_degrees(1.0, 2.0).to_point();
//! let c = LatLng::from_degrees(2.0, 1.0).to_point();
//!
//! let mut tracker = IncidentEdgeTracker::new();
//! tracker.start_shape(0);
//! tracker.add_edge(0, a, b);
//! tracker.add_edge(1, b, a);
//! tracker.add_edge(2, a, c);
//! tracker.add_edge(3, c, a);
//! tracker.finish_shape();
//!
//! let map = tracker.incident_edges();
//! // Vertex 'a' has 4 incident edges, so it appears in the map.
//! use s2rst::s2::incident_edge_tracker::IncidentEdgeKey;
//! assert!(map.contains_key(&IncidentEdgeKey { shape_id: 0, vertex: a }));
//! ```

use std::collections::{BTreeMap, HashSet};

use crate::s2::Point;

/// A (`shape_id`, vertex) key for looking up incident edges.
///
/// The ordering is by `shape_id` first, then by vertex coordinates lexicographically.
#[derive(Clone, Debug, PartialEq)]
pub struct IncidentEdgeKey {
    /// The shape ID.
    pub shape_id: i32,
    /// The vertex point.
    pub vertex: Point,
}

impl Eq for IncidentEdgeKey {}

impl PartialOrd for IncidentEdgeKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IncidentEdgeKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.shape_id
            .cmp(&other.shape_id)
            .then_with(|| cmp_point(&self.vertex, &other.vertex))
    }
}

/// Lexicographic comparison of two points by (x, y, z).
fn cmp_point(a: &Point, b: &Point) -> std::cmp::Ordering {
    a.0.x
        .total_cmp(&b.0.x)
        .then_with(|| a.0.y.total_cmp(&b.0.y))
        .then_with(|| a.0.z.total_cmp(&b.0.z))
}

/// Map from `(shape_id, vertex)` keys to sets of edge IDs.
pub type IncidentEdgeMap = BTreeMap<IncidentEdgeKey, HashSet<i32>>;

/// Temporary storage for vertex-edge associations.
#[derive(Debug)]
struct VertexEdge {
    vertex: Point,
    edge_id: i32,
}

/// Detects and tracks shape edges that are incident on the same vertex.
///
/// Edges of multiple shapes may be tracked. Vertices with more than two
/// incident edges are recorded; vertices with only one or two edges (the
/// common case) are discarded to save memory.
///
/// Usage: call [`start_shape`](Self::start_shape), then
/// [`add_edge`](Self::add_edge) for each edge, then
/// [`finish_shape`](Self::finish_shape). Repeat for each shape. Finally, call
/// [`incident_edges`](Self::incident_edges) to retrieve the result.
#[derive(Debug)]
pub struct IncidentEdgeTracker {
    nursery: Vec<VertexEdge>,
    current_shape_id: i32,
    map: IncidentEdgeMap,
}

impl IncidentEdgeTracker {
    /// Creates a new, empty tracker.
    pub fn new() -> Self {
        Self {
            nursery: Vec::new(),
            current_shape_id: -1,
            map: BTreeMap::new(),
        }
    }

    /// Begins tracking edges for a new shape. Must be called before
    /// [`add_edge`](Self::add_edge).
    pub fn start_shape(&mut self, shape_id: i32) {
        self.nursery.clear();
        self.current_shape_id = shape_id;
    }

    /// Adds an edge (identified by `edge_id`) with endpoints `a` and `b`
    /// for the current shape.
    ///
    /// # Panics
    ///
    /// Panics if [`start_shape`](Self::start_shape) has not been called.
    pub fn add_edge(&mut self, edge_id: i32, a: Point, b: Point) {
        assert!(
            self.current_shape_id >= 0,
            "start_shape() must be called before add_edge()"
        );
        self.nursery.push(VertexEdge { vertex: a, edge_id });
        if a != b {
            self.nursery.push(VertexEdge { vertex: b, edge_id });
        }
    }

    /// Finishes the current shape. Vertices with more than 2 incident edges
    /// are added to the internal map.
    ///
    /// # Panics
    ///
    /// Panics if [`start_shape`](Self::start_shape) has not been called.
    pub fn finish_shape(&mut self) {
        assert!(
            self.current_shape_id >= 0,
            "start_shape() must be called before finish_shape()"
        );

        let n = self.nursery.len();
        // Scan through the nursery, grouping entries with the same vertex.
        // Use an in-place swap-partitioning approach like the Java version.
        let mut start = 0;
        while start < n {
            let mut end = start + 1;
            let curr_vertex = self.nursery[start].vertex;

            // Scan forward, swapping matching vertices into contiguous range.
            let mut next = end;
            while next < n {
                if self.nursery[next].vertex == curr_vertex {
                    self.nursery.swap(next, end);
                    end += 1;
                }
                next += 1;
            }

            let num_edges = end - start;
            if num_edges > 2 {
                let key = IncidentEdgeKey {
                    shape_id: self.current_shape_id,
                    vertex: curr_vertex,
                };
                let edges = self
                    .map
                    .entry(key)
                    .or_insert_with(|| HashSet::with_capacity(8));
                for ve in &self.nursery[start..end] {
                    edges.insert(ve.edge_id);
                }
            }

            start = end;
        }

        self.nursery.clear();
    }

    /// Clears all accumulated state.
    pub fn reset(&mut self) {
        self.map.clear();
    }

    /// Returns a reference to the incident edge map.
    pub fn incident_edges(&self) -> &IncidentEdgeMap {
        &self.map
    }
}

impl Default for IncidentEdgeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn pt(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn assert_incident_at(map: &IncidentEdgeMap, shape_id: i32, vertex: Point, expected: &[i32]) {
        let key = IncidentEdgeKey { shape_id, vertex };
        if expected.is_empty() {
            assert!(
                !map.contains_key(&key),
                "Expected no entry for shape_id={shape_id}"
            );
        } else {
            let edges = map.get(&key).expect("Expected entry in map");
            let expected_set: HashSet<i32> = expected.iter().copied().collect();
            assert_eq!(*edges, expected_set);
        }
    }

    // Java: testVerticesWithTwoIncidentEdges
    #[test]
    fn test_vertices_with_two_incident_edges() {
        let (a, b, c, d) = (pt(1.0, 1.0), pt(1.0, 2.0), pt(2.0, 1.0), pt(2.0, 2.0));
        let mut tracker = IncidentEdgeTracker::new();

        // Square: ab, bc, cd, da — each vertex has exactly 2 incident edges.
        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(1, b, c);
        tracker.add_edge(2, c, d);
        tracker.add_edge(3, d, a);
        tracker.finish_shape();

        assert!(tracker.incident_edges().is_empty());
    }

    // Java: testVerticesWithFourIncidentEdges
    #[test]
    fn test_vertices_with_four_incident_edges() {
        let (a, b, c, d) = (pt(1.0, 1.0), pt(1.0, 2.0), pt(2.0, 1.0), pt(2.0, 2.0));
        let mut tracker = IncidentEdgeTracker::new();

        // Square with edges in each direction.
        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(5, b, a);
        tracker.add_edge(1, b, c);
        tracker.add_edge(6, c, b);
        tracker.add_edge(2, c, d);
        tracker.add_edge(7, d, c);
        tracker.add_edge(3, d, a);
        tracker.add_edge(4, a, d);
        tracker.finish_shape();

        let map = tracker.incident_edges();
        assert!(!map.is_empty());

        assert_incident_at(map, 0, a, &[0, 5, 4, 3]);
        assert_incident_at(map, 0, b, &[0, 5, 1, 6]);
        assert_incident_at(map, 0, c, &[7, 2, 1, 6]);
        assert_incident_at(map, 0, d, &[2, 4, 3, 7]);
    }

    // Java: testTwoShapes
    #[test]
    fn test_two_shapes() {
        let (a, b, c, d) = (pt(1.0, 1.0), pt(1.0, 2.0), pt(2.0, 1.0), pt(2.0, 2.0));
        let mut tracker = IncidentEdgeTracker::new();

        // Shape 0: triangle around a,b,c with edges in each direction.
        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(1, b, c);
        tracker.add_edge(2, c, a);
        tracker.add_edge(3, a, c);
        tracker.add_edge(4, c, b);
        tracker.add_edge(5, b, a);
        tracker.finish_shape();

        // Shape 1: triangle around b,c,d with edges in each direction.
        tracker.start_shape(1);
        tracker.add_edge(0, b, c);
        tracker.add_edge(1, c, d);
        tracker.add_edge(2, d, b);
        tracker.add_edge(3, b, d);
        tracker.add_edge(4, d, c);
        tracker.add_edge(5, c, b);
        tracker.finish_shape();

        let map = tracker.incident_edges();
        assert!(!map.is_empty());

        // Shape 0
        assert_incident_at(map, 0, a, &[0, 2, 3, 5]);
        assert_incident_at(map, 0, b, &[0, 1, 4, 5]);
        assert_incident_at(map, 0, c, &[1, 2, 3, 4]);
        assert_incident_at(map, 0, d, &[]);

        // Shape 1
        assert_incident_at(map, 1, a, &[]);
        assert_incident_at(map, 1, b, &[0, 2, 3, 5]);
        assert_incident_at(map, 1, c, &[0, 1, 4, 5]);
        assert_incident_at(map, 1, d, &[1, 2, 3, 4]);
    }

    // Java: testTwoShapesAsTwoAddEdgesSequences
    #[test]
    fn test_two_shapes_as_two_sequences() {
        let (a, b, c, d) = (pt(1.0, 1.0), pt(1.0, 2.0), pt(2.0, 1.0), pt(2.0, 2.0));
        let mut tracker = IncidentEdgeTracker::new();

        // Shape 0, edges adjacent to a.
        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(5, b, a);
        tracker.add_edge(2, c, a);
        tracker.add_edge(3, a, c);
        tracker.finish_shape();

        // Shape 1, edges adjacent to b.
        tracker.start_shape(1);
        tracker.add_edge(0, b, c);
        tracker.add_edge(5, c, b);
        tracker.add_edge(2, d, b);
        tracker.add_edge(3, b, d);
        tracker.finish_shape();

        // Shape 0, edges adjacent to b.
        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(5, b, a);
        tracker.add_edge(1, b, c);
        tracker.add_edge(4, c, b);
        tracker.finish_shape();

        // Shape 1, edges adjacent to c.
        tracker.start_shape(1);
        tracker.add_edge(0, b, c);
        tracker.add_edge(5, c, b);
        tracker.add_edge(1, c, d);
        tracker.add_edge(4, d, c);
        tracker.finish_shape();

        // Shape 0, edges adjacent to c.
        tracker.start_shape(0);
        tracker.add_edge(1, b, c);
        tracker.add_edge(4, c, b);
        tracker.add_edge(2, c, a);
        tracker.add_edge(3, a, c);
        tracker.finish_shape();

        // Shape 1, edges adjacent to d.
        tracker.start_shape(1);
        tracker.add_edge(1, c, d);
        tracker.add_edge(4, d, c);
        tracker.add_edge(2, d, b);
        tracker.add_edge(3, b, d);
        tracker.finish_shape();

        let map = tracker.incident_edges();
        assert!(!map.is_empty());

        // Shape 0
        assert_incident_at(map, 0, a, &[0, 2, 3, 5]);
        assert_incident_at(map, 0, b, &[0, 1, 4, 5]);
        assert_incident_at(map, 0, c, &[1, 2, 3, 4]);
        assert_incident_at(map, 0, d, &[]);

        // Shape 1
        assert_incident_at(map, 1, a, &[]);
        assert_incident_at(map, 1, b, &[0, 2, 3, 5]);
        assert_incident_at(map, 1, c, &[0, 1, 4, 5]);
        assert_incident_at(map, 1, d, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_reset_clears_state() {
        let (a, b, c) = (pt(1.0, 1.0), pt(1.0, 2.0), pt(2.0, 1.0));
        let mut tracker = IncidentEdgeTracker::new();

        tracker.start_shape(0);
        tracker.add_edge(0, a, b);
        tracker.add_edge(1, b, a);
        tracker.add_edge(2, a, c);
        tracker.add_edge(3, c, a);
        tracker.finish_shape();

        assert!(!tracker.incident_edges().is_empty());
        tracker.reset();
        assert!(tracker.incident_edges().is_empty());
    }

    #[test]
    fn test_degenerate_edge() {
        let (a, b) = (pt(1.0, 1.0), pt(1.0, 2.0));
        let mut tracker = IncidentEdgeTracker::new();

        // Degenerate edge a→a plus two more edges — 3 incident at 'a'.
        tracker.start_shape(0);
        tracker.add_edge(0, a, a); // degenerate: only one endpoint added
        tracker.add_edge(1, a, b);
        tracker.add_edge(2, b, a);
        tracker.finish_shape();

        let map = tracker.incident_edges();
        assert_incident_at(map, 0, a, &[0, 1, 2]);
    }
}
