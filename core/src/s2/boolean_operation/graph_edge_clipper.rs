// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! `GraphEdgeClipper`: post-snap clipping of graph edges.
//!
//! Given a set of clipping instructions encoded as `InputEdgeCrossings`,
//! determines which graph edges correspond to clipped portions of input edges
//! and removes them.

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/ShapeId (i32) used as Vec indices — mirrors C++ graph clipping"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId (i32) <-> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId — always in range"
)]
use std::cmp::{max, min};

use crate::s2::builder::graph::{Edge, EdgeId, Graph, VertexId};
use crate::s2::builder::{InputEdgeId, InputEdgeIdSetId};
use crate::s2::point_measures;
use crate::s2::predicates;
use crate::s2::shape::Dimension;

use super::{CrossingInputEdge, InputEdgeCrossings, K_SET_INSIDE, K_SET_INVERT_B, K_SET_REVERSE_A};

/// Represents an edge from chain B that shares a vertex with chain A.
#[derive(Clone, Debug)]
struct CrossingGraphEdge {
    id: EdgeId,
    a_index: usize,
    outgoing: bool,
    dst: VertexId,
}

/// Returns a vector of `EdgeIds` sorted by input edge id. When more than one
/// output edge has the same input edge id, the edges are sorted to form a
/// directed edge chain.
#[expect(
    clippy::needless_range_loop,
    reason = "index needed for parallel array access"
)]
pub(super) fn get_input_edge_chain_order(g: &Graph, input_ids: &[InputEdgeId]) -> Vec<EdgeId> {
    debug_assert_eq!(
        g.options().edge_type,
        crate::s2::builder::graph::EdgeType::Directed
    );
    debug_assert_eq!(
        g.options().duplicate_edges,
        crate::s2::builder::graph::DuplicateEdges::Keep
    );
    debug_assert_eq!(
        g.options().sibling_pairs,
        crate::s2::builder::graph::SiblingPairs::Keep
    );

    let mut order = g.get_input_edge_order(input_ids);

    // Sort the group of edges corresponding to each input edge in chain order.
    let mut vmap: Vec<(VertexId, EdgeId)> = Vec::new();
    let mut indegree: Vec<i32> = vec![0; g.num_vertices().as_usize()];

    let mut begin = 0;
    while begin < order.len() {
        let input_id = input_ids[order[begin].as_usize()];
        let mut end = begin;
        while end < order.len() && input_ids[order[end].as_usize()] == input_id {
            end += 1;
        }
        if end - begin == 1 {
            begin = end;
            continue;
        }

        // Build vertex -> edge map and compute indegree.
        for i in begin..end {
            let e = order[i];
            let edge = g.edge(e);
            vmap.push((edge.0, e));
            indegree[edge.1.as_usize()] += 1;
        }
        vmap.sort_unstable();

        // Find starting edge (one with indegree 0 at source).
        let mut next = g.num_edges();
        for i in begin..end {
            let e = order[i];
            if indegree[g.edge(e).0.as_usize()] == 0 {
                next = e;
            }
        }

        // Build the chain.
        let mut i = begin;
        loop {
            order[i] = next;
            let v = g.edge(next).1;
            indegree[v.as_usize()] = 0;
            i += 1;
            if i == end {
                break;
            }
            let key = (v, EdgeId(0));
            let pos = vmap.partition_point(|x| *x < key);
            debug_assert!(pos < vmap.len() && vmap[pos].0 == v);
            next = vmap[pos].1;
        }
        vmap.clear();
        begin = end;
    }
    order
}

/// Processes `InputEdgeCrossings` to determine which graph edges correspond
/// to clipped portions of input edges and removes them.
pub(super) struct GraphEdgeClipper<'a> {
    g: &'a Graph,
    in_map: crate::s2::builder::graph::VertexInMap,
    out_map: crate::s2::builder::graph::VertexOutMap,
    input_dimensions: &'a [Dimension],
    input_crossings: &'a InputEdgeCrossings,
    new_edges: &'a mut Vec<Edge>,
    new_input_edge_ids: &'a mut Vec<InputEdgeIdSetId>,

    /// `input_ids`: maps each graph edge to its singleton input edge id.
    input_ids: Vec<InputEdgeId>,
    /// Graph edges sorted in input edge id order.
    order: Vec<EdgeId>,
    /// Rank of each graph edge within order.
    rank: Vec<usize>,
}

impl<'a> GraphEdgeClipper<'a> {
    pub(super) fn new(
        g: &'a Graph,
        input_dimensions: &'a [Dimension],
        input_crossings: &'a InputEdgeCrossings,
        new_edges: &'a mut Vec<Edge>,
        new_input_edge_ids: &'a mut Vec<InputEdgeIdSetId>,
    ) -> Self {
        let input_ids: Vec<InputEdgeId> = (0..g.num_edges().0)
            .map(EdgeId)
            .map(|e| g.min_input_edge_id(e))
            .collect();
        let in_map = g.get_vertex_in_map();
        let out_map = g.get_vertex_out_map();
        let order = get_input_edge_chain_order(g, &input_ids);
        let mut rank = vec![0usize; order.len()];
        for (i, &e) in order.iter().enumerate() {
            rank[e.as_usize()] = i;
        }
        new_edges.reserve(g.num_edges().as_usize());
        new_input_edge_ids.reserve(g.num_edges().as_usize());

        GraphEdgeClipper {
            g,
            in_map,
            out_map,
            input_dimensions,
            input_crossings,
            new_edges,
            new_input_edge_ids,
            input_ids,
            order,
            rank,
        }
    }

    fn add_edge(&mut self, edge: Edge, input_edge_id: InputEdgeId) {
        self.new_edges.push(edge);
        self.new_input_edge_ids.push(input_edge_id.0);
    }

    pub(super) fn run(&mut self) {
        let mut a_vertices: Vec<VertexId> = Vec::new();
        let mut a_num_crossings: Vec<i32> = Vec::new();
        let mut a_isolated: Vec<bool> = Vec::new();
        let mut b_input_edges: Vec<CrossingInputEdge> = Vec::new();
        let mut b_edges: Vec<Vec<CrossingGraphEdge>> = Vec::new();

        let mut inside = false;
        let mut invert_b = false;
        let mut reverse_a = false;
        let mut next_idx = 0;

        let mut i = 0;
        while i < self.order.len() {
            let a_input_id = self.input_ids[self.order[i].as_usize()];
            let edge0 = self.g.edge(self.order[i]);

            // Gather B input edges and state modifications.
            b_input_edges.clear();
            while next_idx < self.input_crossings.len() {
                let (ref_id, ref_crossing) = &self.input_crossings[next_idx];
                if *ref_id != a_input_id {
                    break;
                }
                if ref_crossing.input_id() >= 0 {
                    b_input_edges.push(*ref_crossing);
                } else if ref_crossing.input_id() == K_SET_INSIDE {
                    inside = ref_crossing.left_to_right();
                } else if ref_crossing.input_id() == K_SET_INVERT_B {
                    invert_b = ref_crossing.left_to_right();
                } else {
                    debug_assert_eq!(ref_crossing.input_id(), K_SET_REVERSE_A);
                    reverse_a = ref_crossing.left_to_right();
                }
                next_idx += 1;
            }

            // Optimization for degenerate edges.
            if edge0.0 == edge0.1 {
                inside ^= (b_input_edges.len() & 1) != 0;
                self.add_edge(edge0, a_input_id);
                i += 1;
                continue;
            }

            // Optimization: no crossings.
            if b_input_edges.is_empty() {
                if inside {
                    let e = if reverse_a {
                        Graph::reverse(edge0)
                    } else {
                        edge0
                    };
                    self.add_edge(e, a_input_id);
                }
                i += 1;
                continue;
            }

            // Walk the snapped edge chain for input edge A.
            a_vertices.clear();
            a_vertices.push(edge0.0);
            b_edges.clear();
            b_edges.resize(b_input_edges.len(), Vec::new());
            self.gather_incident_edges(&a_vertices, 0, &b_input_edges, &mut b_edges);

            while i < self.order.len() && self.input_ids[self.order[i].as_usize()] == a_input_id {
                a_vertices.push(self.g.edge(self.order[i]).1);
                let ai = a_vertices.len() - 1;
                self.gather_incident_edges(&a_vertices, ai, &b_input_edges, &mut b_edges);
                i += 1;
            }
            i -= 1; // Will be incremented at the end of the loop.

            // Determine crossing vertices and compute signed crossing counts.
            a_num_crossings.clear();
            a_num_crossings.resize(a_vertices.len(), 0);
            a_isolated.clear();
            a_isolated.resize(a_vertices.len(), false);

            for bi in 0..b_input_edges.len() {
                let left_to_right = b_input_edges[bi].left_to_right();
                let a_index =
                    self.get_crossed_vertex_index(&a_vertices, &b_edges[bi], left_to_right);
                if a_index >= 0 {
                    let a_idx = a_index as usize;
                    let is_line = self.input_dimensions[b_input_edges[bi].input_id().as_usize()]
                        == Dimension::Polyline;
                    let sign = if is_line {
                        0
                    } else if left_to_right == invert_b {
                        -1
                    } else {
                        1
                    };
                    a_num_crossings[a_idx] += sign;
                    a_isolated[a_idx] = true;
                }
            }

            // Walk the A chain, tracking multiplicity.
            let mut multiplicity = i32::from(inside) + a_num_crossings[0];
            for ai in 1..a_vertices.len() {
                if multiplicity != 0 {
                    a_isolated[ai - 1] = false;
                    a_isolated[ai] = false;
                }
                let edge_count = if reverse_a {
                    -multiplicity
                } else {
                    multiplicity
                };
                // Output forward edges.
                for _ in 0..edge_count.max(0) {
                    self.add_edge((a_vertices[ai - 1], a_vertices[ai]), a_input_id);
                }
                // Output reverse edges.
                for _ in edge_count..0 {
                    self.add_edge((a_vertices[ai], a_vertices[ai - 1]), a_input_id);
                }
                multiplicity += a_num_crossings[ai];
            }
            debug_assert!(multiplicity == 0 || multiplicity == 1);
            inside = multiplicity != 0;

            // Output isolated polyline vertices.
            if self.input_dimensions[a_input_id.as_usize()] != Dimension::Point {
                for ai in 0..a_vertices.len() {
                    if a_isolated[ai] {
                        self.add_edge((a_vertices[ai], a_vertices[ai]), a_input_id);
                    }
                }
            }
            i += 1;
        }
    }

    /// Gathers all snapped edges of B that are incident to a given vertex of A.
    fn gather_incident_edges(
        &self,
        a: &[VertexId],
        ai: usize,
        b_input_edges: &[CrossingInputEdge],
        b_edges: &mut [Vec<CrossingGraphEdge>],
    ) {
        debug_assert_eq!(b_input_edges.len(), b_edges.len());
        // Incoming edges to a[ai].
        for &e in self.in_map.edge_ids(a[ai]) {
            let id = self.input_ids[e.as_usize()];
            if let Ok(pos) = b_input_edges.binary_search_by(|x| x.input_id().cmp(&id)) {
                b_edges[pos].push(CrossingGraphEdge {
                    id: e,
                    a_index: ai,
                    outgoing: false,
                    dst: self.g.edge(e).0,
                });
            }
        }
        // Outgoing edges from a[ai].
        for &e in self.out_map.edge_ids(a[ai]) {
            let id = self.input_ids[e.as_usize()];
            if let Ok(pos) = b_input_edges.binary_search_by(|x| x.input_id().cmp(&id)) {
                b_edges[pos].push(CrossingGraphEdge {
                    id: e,
                    a_index: ai,
                    outgoing: true,
                    dst: self.g.edge(e).1,
                });
            }
        }
    }

    fn get_vertex_rank(&self, e: &CrossingGraphEdge) -> usize {
        self.rank[e.id.as_usize()] + if e.outgoing { 0 } else { 1 }
    }

    /// Determines which vertex of the A chain the crossing takes place at.
    fn get_crossed_vertex_index(
        &self,
        a: &[VertexId],
        b: &[CrossingGraphEdge],
        left_to_right: bool,
    ) -> i32 {
        if a.is_empty() || b.is_empty() {
            return -1;
        }
        let n = a.len();
        if n == 1 {
            return 0;
        }
        if b[0].a_index == b[b.len() - 1].a_index {
            return b[0].a_index as i32;
        }

        let b_reversed = self.get_vertex_rank(&b[0]) > self.get_vertex_rank(&b[b.len() - 1]);

        let mut lo: i64 = -1;
        let mut hi: i64 = self.order.len() as i64;
        let mut b_first = EdgeId(-1);
        let mut b_last = EdgeId(-1);

        for e in b {
            let ai = e.a_index;
            if ai == 0 {
                if e.outgoing != b_reversed && e.dst != a[1] {
                    b_first = e.id;
                }
            } else if ai == n - 1 {
                if e.outgoing == b_reversed && e.dst != a[n - 2] {
                    b_last = e.id;
                }
            } else {
                // Interior vertex of A chain.
                if e.dst == a[ai - 1] || e.dst == a[ai + 1] {
                    continue;
                }
                let on_left = predicates::ordered_ccw(
                    self.g.vertex(a[ai + 1]),
                    self.g.vertex(e.dst),
                    self.g.vertex(a[ai - 1]),
                    self.g.vertex(a[ai]),
                );
                if left_to_right == on_left {
                    lo = max(lo, self.rank[e.id.as_usize()] as i64 + 1);
                } else {
                    hi = min(hi, self.rank[e.id.as_usize()] as i64);
                }
            }
        }

        // Special case: B subchain connects first and last vertices of A.
        if b_first >= 0 && b_last >= 0 {
            let (b_first, b_last) = if b_reversed {
                (b_last, b_first)
            } else {
                (b_first, b_last)
            };

            let mut has_interior_vertex = false;
            for e in b {
                if e.a_index > 0
                    && e.a_index < n - 1
                    && self.rank[e.id.as_usize()] >= self.rank[b_first.as_usize()]
                    && self.rank[e.id.as_usize()] <= self.rank[b_last.as_usize()]
                {
                    has_interior_vertex = true;
                    break;
                }
            }
            if !has_interior_vertex {
                let on_left = self.edge_chain_on_left(a, b_first, b_last);
                if left_to_right == on_left {
                    lo = max(lo, self.rank[b_last.as_usize()] as i64 + 1);
                } else {
                    hi = min(hi, self.rank[b_first.as_usize()] as i64);
                }
            }
        }

        // Choose the smallest shared VertexId in the acceptable range.
        let mut best: i32 = -1;
        debug_assert!(lo <= hi);
        for e in b {
            let ai = e.a_index;
            let vrank = self.get_vertex_rank(e) as i64;
            if vrank >= lo && vrank <= hi && (best < 0 || a[ai] < a[best as usize]) {
                best = ai as i32;
            }
        }
        best
    }

    /// Returns true if chain B is to the left of chain A, where together they
    /// form a loop.
    fn edge_chain_on_left(&self, a: &[VertexId], b_first: EdgeId, b_last: EdgeId) -> bool {
        let mut loop_vertices: Vec<VertexId> = Vec::new();
        let r_first = self.rank[b_first.as_usize()];
        let r_last = self.rank[b_last.as_usize()];
        for i in r_first..r_last {
            loop_vertices.push(self.g.edge(self.order[i]).1);
        }
        // Possibly reverse so it forms a loop when a is appended.
        if self.g.edge(b_last).1 != a[0] {
            loop_vertices.reverse();
        }
        loop_vertices.extend_from_slice(a);
        // Duplicate first two vertices.
        if loop_vertices.len() >= 2 {
            let v0 = loop_vertices[0];
            let v1 = loop_vertices[1];
            loop_vertices.push(v0);
            loop_vertices.push(v1);
        }
        let mut sum = 0.0;
        for i in 2..loop_vertices.len() {
            sum += point_measures::turn_angle(
                self.g.vertex(loop_vertices[i - 2]),
                self.g.vertex(loop_vertices[i - 1]),
                self.g.vertex(loop_vertices[i]),
            )
            .radians();
        }
        sum > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── CrossingGraphEdge ───────────────────────────────────────────

    #[test]
    fn test_crossing_graph_edge_fields() {
        let e = CrossingGraphEdge {
            id: EdgeId(5),
            a_index: 2,
            outgoing: true,
            dst: VertexId(7),
        };
        assert_eq!(e.id, 5);
        assert_eq!(e.a_index, 2);
        assert!(e.outgoing);
        assert_eq!(e.dst, 7);
    }

    #[test]
    fn test_crossing_graph_edge_clone() {
        let e = CrossingGraphEdge {
            id: EdgeId(3),
            a_index: 1,
            outgoing: false,
            dst: VertexId(4),
        };
        let e2 = e.clone();
        assert_eq!(e.id, e2.id);
        assert_eq!(e.a_index, e2.a_index);
        assert_eq!(e.outgoing, e2.outgoing);
        assert_eq!(e.dst, e2.dst);
    }

    // ─── CrossingInputEdge ordering ──────────────────────────────────

    #[test]
    fn test_crossing_input_edge_ordering() {
        let a = CrossingInputEdge::new(1, true);
        let b = CrossingInputEdge::new(2, false);
        let c = CrossingInputEdge::new(1, false);
        assert!(a < b);
        assert_eq!(a, c); // Equality ignores left_to_right.
        assert!(b > a);
    }

    #[test]
    fn test_crossing_input_edge_accessors() {
        let e = CrossingInputEdge::new(42, true);
        assert_eq!(e.input_id(), 42);
        assert!(e.left_to_right());

        let e2 = CrossingInputEdge::new(-3, false);
        assert_eq!(e2.input_id(), -3);
        assert!(!e2.left_to_right());
    }

    // ─── get_input_edge_chain_order ──────────────────────────────────
    // These tests exercise the chain-building logic indirectly through
    // the full S2BooleanOperation integration tests. Here we test the
    // specific logic of CrossingInputEdge binary_search used in
    // gather_incident_edges.

    #[test]
    fn test_crossing_input_edge_binary_search() {
        let edges = [
            CrossingInputEdge::new(1, false),
            CrossingInputEdge::new(3, true),
            CrossingInputEdge::new(5, false),
            CrossingInputEdge::new(7, true),
        ];

        // Found cases.
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(1)))
                .is_ok()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(3)))
                .is_ok()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(5)))
                .is_ok()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(7)))
                .is_ok()
        );

        // Not found cases.
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(0)))
                .is_err()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(2)))
                .is_err()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(4)))
                .is_err()
        );
        assert!(
            edges
                .binary_search_by(|x| x.input_id().cmp(&InputEdgeId(8)))
                .is_err()
        );
    }

    #[test]
    fn test_crossing_input_edge_special_values() {
        // Test that special K_SET_* values work correctly with CrossingInputEdge.
        let inside = CrossingInputEdge::new(K_SET_INSIDE, true);
        let invert = CrossingInputEdge::new(K_SET_INVERT_B, false);
        let reverse = CrossingInputEdge::new(K_SET_REVERSE_A, true);

        assert_eq!(inside.input_id(), K_SET_INSIDE);
        assert_eq!(invert.input_id(), K_SET_INVERT_B);
        assert_eq!(reverse.input_id(), K_SET_REVERSE_A);

        // Special values are negative — ordering should still work.
        assert!(reverse < invert);
        assert!(invert < inside);
        assert!(inside < CrossingInputEdge::new(0, false));
    }

    // ─── Input edge crossings vector operations ─────────────────────

    #[test]
    fn test_input_edge_crossings_grouping() {
        // Verify that crossings can be grouped by input edge id.
        let crossings: InputEdgeCrossings = vec![
            (InputEdgeId(0), CrossingInputEdge::new(K_SET_INSIDE, true)),
            (InputEdgeId(0), CrossingInputEdge::new(5, true)),
            (InputEdgeId(0), CrossingInputEdge::new(7, false)),
            (InputEdgeId(1), CrossingInputEdge::new(K_SET_INSIDE, false)),
            (InputEdgeId(1), CrossingInputEdge::new(3, true)),
            (
                InputEdgeId(2),
                CrossingInputEdge::new(K_SET_REVERSE_A, true),
            ),
        ];

        // Count crossings for input edge 0.
        let count_0 = crossings.iter().filter(|(id, _)| *id == 0).count();
        assert_eq!(count_0, 3);

        // Count crossings for input edge 1.
        let count_1 = crossings.iter().filter(|(id, _)| *id == 1).count();
        assert_eq!(count_1, 2);

        // Count crossings for input edge 2.
        let count_2 = crossings.iter().filter(|(id, _)| *id == 2).count();
        assert_eq!(count_2, 1);
    }

    #[test]
    fn test_input_edge_crossings_empty() {
        let crossings: InputEdgeCrossings = vec![];
        assert!(crossings.is_empty());
    }
}
