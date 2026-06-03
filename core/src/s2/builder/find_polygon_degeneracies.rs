// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId (i32) used as Vec indices in degeneracy detection"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId — always in range"
)]
//! Identifies degenerate edges in polygon graphs and classifies each as a
//! shell or hole.
//!
// C++ ref: s2builderutil_find_polygon_degeneracies.h/cc

use crate::s2::builder::S2Error;
use crate::s2::builder::graph::{EdgeId, Graph, VertexId, VertexInMap, VertexOutMap};
use crate::s2::builder::graph_shape::GraphShape;
use crate::s2::contains_vertex_query::ContainsVertexQuery;
use crate::s2::crossing_edge_query::CrossingEdgeQuery;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::predicates;
use crate::s2::shape_index::ShapeIndex;

/// A degenerate edge in a polygon graph, classified as shell or hole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolygonDegeneracy {
    /// The edge ID within the graph.
    pub edge_id: EdgeId,
    /// Whether this degeneracy is a hole (true) or a shell (false).
    pub is_hole: bool,
}

impl PartialOrd for PolygonDegeneracy {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PolygonDegeneracy {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.edge_id
            .cmp(&other.edge_id)
            .then(self.is_hole.cmp(&other.is_hole))
    }
}

/// Finds all degenerate edges (self-loops and sibling pairs) in a polygon
/// graph and classifies each as a shell or hole.
///
/// The graph must use DIRECTED edges with `DISCARD_EXCESS` for both
/// `degenerate_edges` and `sibling_pairs`.
pub fn find_polygon_degeneracies(g: &Graph, error: &mut S2Error) -> Vec<PolygonDegeneracy> {
    let mut finder = DegeneracyFinder::new(g);
    finder.run(error)
}

/// Reports whether every edge in the graph is degenerate (a self-loop
/// or part of a sibling pair).
pub(crate) fn is_fully_degenerate(g: &Graph) -> bool {
    let edges = g.edges();
    for e in (0..g.num_edges().0).map(EdgeId) {
        let edge = edges[e.as_usize()];
        if edge.0 == edge.1 {
            continue;
        }
        if edges.binary_search(&Graph::reverse(edge)).is_ok() {
            continue;
        }
        return false;
    }
    true
}

// ─── Internal: connected component of degenerate edges ─────────────────

struct Component {
    root: VertexId,
    root_sign: i32, // +1 inside, -1 outside, 0 unknown
    degeneracies: Vec<PolygonDegeneracy>,
}

// ─── DegeneracyFinder ──────────────────────────────────────────────────

struct DegeneracyFinder<'a> {
    g: &'a Graph,
    out: VertexOutMap,
    inp: VertexInMap,
    is_vertex_used: Vec<bool>,
    is_edge_degeneracy: Vec<bool>,
    is_vertex_unbalanced: Vec<bool>,
}

impl<'a> DegeneracyFinder<'a> {
    fn new(g: &'a Graph) -> Self {
        DegeneracyFinder {
            out: VertexOutMap::new(g),
            inp: VertexInMap::new(g),
            g,
            is_vertex_used: vec![false; g.num_vertices().as_usize()],
            is_edge_degeneracy: Vec::new(),
            is_vertex_unbalanced: Vec::new(),
        }
    }

    fn run(&mut self, _error: &mut S2Error) -> Vec<PolygonDegeneracy> {
        let num_degeneracies = self.compute_degeneracies();
        if num_degeneracies == 0 {
            return Vec::new();
        }

        // Special case: if ALL edges are degenerate, classify uniformly.
        if num_degeneracies == self.g.num_edges().as_usize() {
            let is_hole = self.g.is_full_polygon().unwrap_or(false);
            return (0..self.g.num_edges().0)
                .map(EdgeId)
                .map(|e| PolygonDegeneracy {
                    edge_id: e,
                    is_hole,
                })
                .collect();
        }

        // Build connected components of degenerate edges via BFS.
        let mut components = Vec::new();
        let mut num_unknown = 0;
        for v in (0..self.g.num_vertices().0).map(VertexId) {
            if self.is_vertex_used[v.as_usize()] {
                continue;
            }
            // Only start BFS from vertices that have at least one degenerate edge.
            let has_degen = self
                .out
                .edge_ids(v)
                .iter()
                .any(|&e| self.is_edge_degeneracy[e.as_usize()]);
            if !has_degen {
                continue;
            }
            let component = self.build_component(v);
            if component.root_sign == 0 {
                num_unknown += 1;
            }
            components.push(component);
        }

        // Resolve unknown component signs.
        if num_unknown > 0 {
            // Find a vertex with known sign (from a known component, or brute force).
            let (known_vertex, known_sign) = self.find_known_vertex(&components);

            if num_unknown <= 25 {
                self.compute_unknown_signs_brute_force(known_vertex, known_sign, &mut components);
            } else {
                self.compute_unknown_signs_indexed(known_vertex, known_sign, &mut components);
            }
        }

        self.merge_degeneracies(&components)
    }

    /// Identifies degenerate edges (self-loops and sibling pairs) and marks
    /// unbalanced vertices. Returns the count of degenerate edges.
    fn compute_degeneracies(&mut self) -> usize {
        self.is_edge_degeneracy = vec![false; self.g.num_edges().as_usize()];
        self.is_vertex_unbalanced = vec![false; self.g.num_vertices().as_usize()];

        let in_edge_ids = self.inp.in_edge_ids();
        let n = self.g.num_edges().as_usize();
        let mut num_degeneracies = 0;
        let mut inp = 0usize;

        for out in 0..n {
            let out_edge = self.g.edge(EdgeId(out as i32));
            if out_edge.0 == out_edge.1 {
                // Self-loop: always degenerate.
                self.is_edge_degeneracy[out] = true;
                num_degeneracies += 1;
            } else {
                // Check for matching sibling using sorted incoming edges.
                while inp < n && Graph::reverse(self.g.edge(in_edge_ids[inp])) < out_edge {
                    inp += 1;
                }
                if inp < n && Graph::reverse(self.g.edge(in_edge_ids[inp])) == out_edge {
                    self.is_edge_degeneracy[out] = true;
                    num_degeneracies += 1;
                } else {
                    self.is_vertex_unbalanced[out_edge.0.as_usize()] = true;
                }
            }
        }
        num_degeneracies
    }

    /// BFS from root vertex through degenerate edges, building a component.
    fn build_component(&mut self, root: VertexId) -> Component {
        let mut result = Component {
            root,
            root_sign: 0,
            degeneracies: Vec::new(),
        };

        // frontier: (vertex, same_inside_as_root)
        let mut frontier: Vec<(VertexId, bool)> = vec![(root, true)];
        self.is_vertex_used[root.as_usize()] = true;

        while let Some((v0, v0_same_inside)) = frontier.pop() {
            // If this vertex is unbalanced, we can determine sign directly.
            if result.root_sign == 0 && self.is_vertex_unbalanced[v0.as_usize()] {
                let v0_sign = self.contains_vertex_sign(v0);
                result.root_sign = if v0_same_inside { v0_sign } else { -v0_sign };
            }

            for &e in self.out.edge_ids(v0) {
                let v1 = self.g.edge(e).1;
                let mut same_inside = v0_same_inside ^ self.crossing_parity(v0, v1, false);
                if self.is_edge_degeneracy[e.as_usize()] {
                    result.degeneracies.push(PolygonDegeneracy {
                        edge_id: e,
                        is_hole: same_inside,
                    });
                }
                if self.is_vertex_used[v1.as_usize()] {
                    continue;
                }
                same_inside ^= self.crossing_parity(v1, v0, true);
                frontier.push((v1, same_inside));
                self.is_vertex_used[v1.as_usize()] = true;
            }
        }
        result
    }

    /// Counts edges around v0 between `reference_dir(v0)` and v1,
    /// returning the parity of the count.
    fn crossing_parity(&self, v0: VertexId, v1: VertexId, include_same: bool) -> bool {
        let mut crossings = 0i32;
        let p0 = self.g.vertex(v0);
        let p1 = self.g.vertex(v1);
        let p0_ref = p0.reference_dir();

        // Count outgoing edges from v0 that lie between p0_ref and p1.
        for &e in self.out.edge_ids(v0) {
            let target = self.g.edge(e).1;
            if target == v1 {
                if include_same {
                    crossings += 1;
                }
            } else if predicates::ordered_ccw(p0_ref, self.g.vertex(target), p1, p0) {
                crossings += 1;
            }
        }

        // Count incoming edges to v0 that lie between p0_ref and p1.
        for &e in self.inp.edge_ids(v0) {
            let source = self.g.edge(e).0;
            if source == v1 {
                if include_same {
                    crossings += 1;
                }
            } else if predicates::ordered_ccw(p0_ref, self.g.vertex(source), p1, p0) {
                crossings += 1;
            }
        }

        crossings & 1 != 0
    }

    /// Uses `ContainsVertexQuery` to determine if vertex v0 is inside the polygon.
    fn contains_vertex_sign(&self, v0: VertexId) -> i32 {
        let mut query = ContainsVertexQuery::new(self.g.vertex(v0));
        for &e in self.out.edge_ids(v0) {
            query.add_edge(self.g.vertex(self.g.edge(e).1), 1);
        }
        for &e in self.inp.edge_ids(v0) {
            query.add_edge(self.g.vertex(self.g.edge(e).0), -1);
        }
        query.contains_vertex()
    }

    /// Finds a vertex whose containment status is known.
    fn find_known_vertex(&self, components: &[Component]) -> (VertexId, i32) {
        // Check if any component already has a known sign.
        for c in components {
            if c.root_sign != 0 {
                return (c.root, c.root_sign);
            }
        }
        // Fall back to brute force: pick any non-degenerate vertex and check.
        for e in (0..self.g.num_edges().0).map(EdgeId) {
            if !self.is_edge_degeneracy[e.as_usize()] {
                let v = self.g.edge(e).0;
                let sign = self.contains_vertex_sign(v);
                if sign != 0 {
                    return (v, sign);
                }
            }
        }
        // Should not happen for a valid polygon graph with non-degenerate edges.
        (VertexId(0), -1)
    }

    /// Brute force: for each unknown component, count edge crossings
    /// from `known_vertex` to component root.
    fn compute_unknown_signs_brute_force(
        &self,
        known_vertex: VertexId,
        known_vertex_sign: i32,
        components: &mut [Component],
    ) {
        for component in components.iter_mut() {
            if component.root_sign != 0 {
                continue;
            }
            let mut inside = known_vertex_sign > 0;
            let mut crosser =
                EdgeCrosser::new(self.g.vertex(known_vertex), self.g.vertex(component.root));
            for e in (0..self.g.num_edges().0).map(EdgeId) {
                if self.is_edge_degeneracy[e.as_usize()] {
                    continue;
                }
                let edge = self.g.edge(e);
                inside ^=
                    crosser.edge_or_vertex_crossing(self.g.vertex(edge.0), self.g.vertex(edge.1));
            }
            component.root_sign = if inside { 1 } else { -1 };
        }
    }

    /// Indexed: build a `ShapeIndex` for efficient crossing queries.
    fn compute_unknown_signs_indexed(
        &self,
        known_vertex: VertexId,
        known_vertex_sign: i32,
        components: &mut [Component],
    ) {
        let mut index = ShapeIndex::new();
        let shape = GraphShape::from_graph(self.g);
        let shape_id = index.add(Box::new(shape));

        let mut query = CrossingEdgeQuery::new(&index);

        for component in components.iter_mut() {
            if component.root_sign != 0 {
                continue;
            }
            let mut inside = known_vertex_sign > 0;
            let a = self.g.vertex(known_vertex);
            let b = self.g.vertex(component.root);
            let mut crosser = EdgeCrosser::new(a, b);
            let Some(shape) = index.shape(shape_id) else {
                continue;
            };
            let candidates = query.candidates(a, b, shape, shape_id);
            for edge_id in candidates {
                let e = edge_id;
                if self.is_edge_degeneracy[e as usize] {
                    continue;
                }
                let edge = self.g.edge(e);
                inside ^=
                    crosser.edge_or_vertex_crossing(self.g.vertex(edge.0), self.g.vertex(edge.1));
            }
            component.root_sign = if inside { 1 } else { -1 };
        }
    }

    /// Converts relative `is_hole` (relative to component root) to absolute.
    #[expect(clippy::unused_self, reason = "matches C++ method signature")]
    fn merge_degeneracies(&self, components: &[Component]) -> Vec<PolygonDegeneracy> {
        let mut result = Vec::new();
        for component in components {
            debug_assert_ne!(component.root_sign, 0);
            let invert = component.root_sign < 0;
            for d in &component.degeneracies {
                result.push(PolygonDegeneracy {
                    edge_id: d.edge_id,
                    is_hole: d.is_hole ^ invert,
                });
            }
        }
        result.sort_unstable();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::builder::graph::{
        DegenerateEdges, DuplicateEdges, EdgeType, GraphOptions, SiblingPairs,
    };
    use crate::s2::builder::id_set_lexicon::IdSetLexicon;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point::from_coords(x, y, z).normalize()
    }

    fn polygon_graph_options() -> GraphOptions {
        GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Keep,
            SiblingPairs::DiscardExcess,
        )
    }

    fn build_graph(vertices: &[Point], edges: &[(i32, i32)]) -> Graph {
        let options = polygon_graph_options();
        let mut lexicon = IdSetLexicon::new();
        let input_ids: Vec<i32> = (0..edges.len() as i32)
            .map(|i| lexicon.add_set(&[i]))
            .collect();
        let label_ids: Vec<i32> = vec![lexicon.add_set(&[]); edges.len()];
        Graph::from_raw_parts(
            options,
            vertices.to_vec(),
            edges
                .iter()
                .map(|&(a, b)| (VertexId(a), VertexId(b)))
                .collect(),
            input_ids,
            lexicon.clone(),
            label_ids,
            lexicon,
            None,
        )
    }

    #[test]
    fn test_empty_polygon() {
        let g = build_graph(&[], &[]);
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert!(result.is_empty());
    }

    #[test]
    fn test_no_degeneracies() {
        // A simple triangle: no degenerate edges.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let g = build_graph(&[v0, v1, v2], &[(0, 1), (1, 2), (2, 0)]);
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert!(result.is_empty());
    }

    #[test]
    fn test_point_shell() {
        // A triangle plus a degenerate point (self-loop) outside.
        // The point should be classified as a shell.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let v3 = p(-1.0, 0.0, 0.0); // Outside the triangle
        // Edges: triangle (0→1, 1→2, 2→0) + self-loop at v3 (3→3)
        let g = build_graph(&[v0, v1, v2, v3], &[(0, 1), (1, 2), (2, 0), (3, 3)]);
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].edge_id, 3);
        assert!(
            !result[0].is_hole,
            "point outside polygon should be a shell"
        );
    }

    #[test]
    fn test_sibling_pair_shells() {
        // Two matching triangles forming a sibling pair. All edges degenerate.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        // Edges: 0→1, 1→2, 2→0, 1→0, 2→1, 0→2 (sorted lexicographically)
        // After sorting: (0,1), (0,2), (1,0), (1,2), (2,0), (2,1)
        let g = build_graph(
            &[v0, v1, v2],
            &[(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)],
        );
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert_eq!(result.len(), 6);
        // All should be shells (not inside the polygon since there's no real area)
        for d in &result {
            assert!(
                !d.is_hole,
                "sibling pair forming empty polygon should be shells"
            );
        }
    }

    #[test]
    fn test_attached_sibling_pair_holes() {
        // A triangle with a sibling pair attached inside.
        // Triangle: v0→v1→v2→v0 (large, covering the sibling pair)
        // Sibling pair: v1→v3, v3→v1 (inside the triangle)
        let v0 = p(1.0, 0.1, 0.0);
        let v1 = p(0.0, 1.0, 0.1);
        let v2 = p(0.1, 0.0, 1.0);
        // v3 is inside the triangle
        let v3 = p(0.4, 0.4, 0.4).normalize();
        // Sorted edges: (0,1), (1,2), (1,3), (2,0), (3,1)
        let g = build_graph(&[v0, v1, v2, v3], &[(0, 1), (1, 2), (1, 3), (2, 0), (3, 1)]);
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert_eq!(result.len(), 2);
        // The sibling pair (1,3) and (3,1) should be holes.
        for d in &result {
            assert!(d.is_hole, "sibling pair inside polygon should be holes");
        }
    }

    #[test]
    fn test_degenerate_shells_outside_loop() {
        // A triangle plus a self-loop outside → shell.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let v3 = p(-1.0, 0.0, 0.0);
        let v4 = p(0.0, -1.0, 0.0);
        // Triangle + two self-loops outside
        let g = build_graph(
            &[v0, v1, v2, v3, v4],
            &[(0, 1), (1, 2), (2, 0), (3, 3), (4, 4)],
        );
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert_eq!(result.len(), 2);
        for d in &result {
            assert!(
                !d.is_hole,
                "degenerate edges outside polygon should be shells"
            );
        }
    }

    #[test]
    fn test_degenerate_holes_within_loop() {
        // A triangle with self-loops inside → holes.
        let v0 = p(1.0, 0.1, 0.0);
        let v1 = p(0.0, 1.0, 0.1);
        let v2 = p(0.1, 0.0, 1.0);
        // v3 is inside the triangle
        let v3 = p(0.4, 0.4, 0.4).normalize();
        // Triangle + self-loop inside
        let g = build_graph(&[v0, v1, v2, v3], &[(0, 1), (1, 2), (2, 0), (3, 3)]);
        let mut error = S2Error::ok();
        let result = find_polygon_degeneracies(&g, &mut error);
        assert_eq!(result.len(), 1);
        assert!(
            result[0].is_hole,
            "degenerate edge inside polygon should be a hole"
        );
    }

    // ─── Batch 7: Full-pipeline degeneracy tests (from C++) ─────────────

    /// A layer that checks `find_polygon_degeneracies` results against expected
    /// degeneracies. Matches C++ `DegeneracyCheckingLayer`.
    #[derive(Debug)]
    struct DegeneracyCheckingLayer {
        expected: Vec<(String, bool)>, // (edge_str, is_hole)
    }

    impl DegeneracyCheckingLayer {
        fn new(expected: Vec<(String, bool)>) -> Self {
            DegeneracyCheckingLayer { expected }
        }
    }

    impl crate::s2::builder::layer::Layer for DegeneracyCheckingLayer {
        fn graph_options(&self) -> GraphOptions {
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::DiscardExcess,
                DuplicateEdges::Keep,
                SiblingPairs::DiscardExcess,
            )
        }

        fn build(&mut self, g: &Graph, error: &mut S2Error) {
            let degeneracies = find_polygon_degeneracies(g, error);

            let mut actual: Vec<(String, bool)> = degeneracies
                .iter()
                .map(|d| {
                    let (v0, v1) = g.edge(d.edge_id);
                    let edge_str =
                        crate::s2::text_format::points_to_string(&[g.vertex(v0), g.vertex(v1)]);
                    (edge_str, d.is_hole)
                })
                .collect();
            actual.sort_unstable();

            let mut expected = self.expected.clone();
            expected.sort_unstable();

            assert_eq!(
                expected, actual,
                "degeneracies mismatch\nExpected: {expected:?}\nActual: {actual:?}"
            );

            // Also verify is_fully_degenerate consistency.
            assert_eq!(
                is_fully_degenerate(g),
                degeneracies.len() == g.num_edges().as_usize(),
                "is_fully_degenerate mismatch"
            );
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    /// Builds a polygon through `S2Builder` with a `DegeneracyCheckingLayer`.
    fn expect_degeneracies(polygon_str: &str, expected: &[(&str, bool)]) {
        use crate::s2::builder::S2Builder;
        use crate::s2::shape::Shape;
        use crate::s2::text_format::make_lax_polygon;

        let expected_vec: Vec<(String, bool)> =
            expected.iter().map(|(s, h)| (s.to_string(), *h)).collect();

        let mut builder = S2Builder::new(crate::s2::builder::Options::default());
        builder.start_layer(Box::new(DegeneracyCheckingLayer::new(expected_vec)));

        let polygon = make_lax_polygon(polygon_str);
        let is_full = polygon.reference_point().contained;
        builder.add_is_full_polygon_predicate(S2Builder::is_full_polygon(is_full));
        builder.add_shape(&polygon);

        let result = builder.build();
        assert!(
            result.is_ok(),
            "build failed for {polygon_str:?}: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_attached_sibling_pair_shells() {
        // C++: FindPolygonDegeneracies::AttachedSiblingPairShells
        expect_degeneracies(
            "0:0, 0:1, 1:0; 1:0, 2:0",
            &[("1:0, 2:0", false), ("2:0, 1:0", false)],
        );
    }

    #[test]
    fn test_attached_sibling_pair_shells_and_holes() {
        // C++: FindPolygonDegeneracies::AttachedSiblingPairShellsAndHoles
        expect_degeneracies(
            "0:0, 0:3, 3:0; 3:0, 1:1; 3:0, 5:5",
            &[
                ("3:0, 1:1", true),
                ("1:1, 3:0", true),
                ("3:0, 5:5", false),
                ("5:5, 3:0", false),
            ],
        );
    }

    #[test]
    fn test_point_hole_within_full() {
        // C++: FindPolygonDegeneracies::PointHoleWithinFull
        expect_degeneracies("full; 0:0", &[("0:0, 0:0", true)]);
    }

    #[test]
    fn test_sibling_pair_holes_within_full() {
        // C++: FindPolygonDegeneracies::SiblingPairHolesWithinFull
        expect_degeneracies(
            "full; 0:0, 0:1, 1:0; 1:0, 0:1, 0:0",
            &[
                ("0:0, 0:1", true),
                ("0:1, 0:0", true),
                ("0:1, 1:0", true),
                ("1:0, 0:1", true),
                ("0:0, 1:0", true),
                ("1:0, 0:0", true),
            ],
        );
    }
}
