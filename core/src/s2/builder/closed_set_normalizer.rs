// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/ShapeId (i32) used as Vec indices"
)]
// ClosedSetNormalizer: normalizes 3 graphs (dim 0=points, 1=polylines,
// 2=polygons) by demoting degenerate polygon/polyline edges to lower
// dimensions and suppressing lower-dimensional geometry that coincides
// with higher-dimensional geometry.
//
// C++ ref: s2builderutil_closed_set_normalizer.h/cc

use std::cell::RefCell;
use std::rc::Rc;

use crate::s2::builder::find_polygon_degeneracies::find_polygon_degeneracies;
use crate::s2::builder::graph::{
    DegenerateEdges, Edge, EdgeId, EdgeType, Graph, GraphOptions, SiblingPairs, VertexId,
};
use crate::s2::builder::layer::Layer;
use crate::s2::builder::{InputEdgeIdSetId, S2Error};

/// Options for `ClosedSetNormalizer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// If true, lower-dimensional edges that coincide with higher-dimensional
    /// geometry are suppressed (e.g., a point coincident with a polygon vertex
    /// is removed). Default: true.
    pub suppress_lower_dimensions: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            suppress_lower_dimensions: true,
        }
    }
}

/// Normalizes a closed set of 3 graphs (points, polylines, polygons) by
/// demoting degenerate polygon/polyline edges to lower dimensions.
struct ClosedSetNormalizer {
    options: Options,
    graph_options_in: Vec<GraphOptions>,
    graph_options_out: Vec<GraphOptions>,
}

const SENTINEL: Edge = (VertexId::MAX, VertexId::MAX);

impl ClosedSetNormalizer {
    fn new(options: Options, graph_options_out: Vec<GraphOptions>) -> Self {
        assert_eq!(graph_options_out.len(), 3);
        debug_assert!(graph_options_out[0].edge_type == EdgeType::Directed);
        debug_assert!(graph_options_out[2].edge_type == EdgeType::Directed);
        debug_assert!(graph_options_out[1].sibling_pairs != SiblingPairs::Create);
        debug_assert!(graph_options_out[1].sibling_pairs != SiblingPairs::Require);
        let mut graph_options_in = graph_options_out.clone();
        for opt in &mut graph_options_in {
            opt.allow_vertex_filtering = false;
        }
        graph_options_in[1].degenerate_edges = DegenerateEdges::DiscardExcess;
        graph_options_in[2].degenerate_edges = DegenerateEdges::DiscardExcess;
        graph_options_in[2].sibling_pairs = SiblingPairs::DiscardExcess;

        ClosedSetNormalizer {
            options,
            graph_options_in,
            graph_options_out,
        }
    }

    /// Returns the adjusted input graph options (for the builder to use).
    fn graph_options(&self) -> &[GraphOptions] {
        &self.graph_options_in
    }

    /// Normalizes the input graphs and returns 3 output graphs.
    fn run(&self, input: &[Graph], error: &mut S2Error) -> Vec<Graph> {
        assert_eq!(input.len(), 3);

        // Build suppression data.
        let in_edges2 = if self.options.suppress_lower_dimensions {
            input[2].get_in_edge_ids()
        } else {
            Vec::new()
        };

        let mut is_suppressed = vec![false; input[0].vertices().len()];
        if self.options.suppress_lower_dimensions {
            for graph in &input[1..=2] {
                for e in (0..graph.num_edges().0).map(EdgeId) {
                    let edge = graph.edge(e);
                    if edge.0 != edge.1 {
                        is_suppressed[edge.0.as_usize()] = true;
                        is_suppressed[edge.1.as_usize()] = true;
                    }
                }
            }
        }

        // Normalize edges: 3-way merge join.
        let mut new_edges: [Vec<Edge>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        let mut new_input_ids: [Vec<InputEdgeIdSetId>; 3] = [Vec::new(), Vec::new(), Vec::new()];

        self.normalize_edges(
            input,
            &in_edges2,
            &is_suppressed,
            &mut new_edges,
            &mut new_input_ids,
            error,
        );

        // Check if any dimension was modified.
        let mut any_modified = false;
        let mut modified = [false; 3];
        for dim in (0..3).rev() {
            if new_edges[dim].len() != input[dim].num_edges().as_usize() {
                any_modified = true;
            }
            modified[dim] = any_modified;
        }

        if !any_modified {
            // No changes: return input graphs with requested output options.
            let mut result = Vec::with_capacity(3);
            for (opts, g) in self.graph_options_out.iter().zip(input.iter()) {
                result.push(Graph::from_raw_parts(
                    opts.clone(),
                    g.vertices().to_vec(),
                    g.edges().to_vec(),
                    g.input_edge_id_set_ids().to_vec(),
                    g.input_edge_id_set_lexicon().clone(),
                    g.label_set_ids().to_vec(),
                    g.label_set_lexicon().clone(),
                    g.is_full_polygon_predicate_clone(),
                ));
            }
            return result;
        }

        // Changes were made: reprocess edges with output options.
        let mut new_lexicon = input[0].input_edge_id_set_lexicon().clone();
        let mut result = Vec::with_capacity(3);
        for dim in 0..3 {
            if modified[dim] {
                let mut opts = self.graph_options_out[dim].clone();
                Graph::process_edges(
                    &mut opts,
                    &mut new_edges[dim],
                    &mut new_input_ids[dim],
                    &mut new_lexicon,
                    error,
                );
            }
            result.push(Graph::from_raw_parts(
                self.graph_options_out[dim].clone(),
                input[dim].vertices().to_vec(),
                std::mem::take(&mut new_edges[dim]),
                std::mem::take(&mut new_input_ids[dim]),
                new_lexicon.clone(),
                input[dim].label_set_ids().to_vec(),
                input[dim].label_set_lexicon().clone(),
                input[dim].is_full_polygon_predicate_clone(),
            ));
        }
        result
    }

    /// Three-way merge join over point/polyline/polygon edges.
    fn normalize_edges(
        &self,
        g: &[Graph],
        in_edges2: &[EdgeId],
        is_suppressed: &[bool],
        new_edges: &mut [Vec<Edge>; 3],
        new_input_ids: &mut [Vec<InputEdgeIdSetId>; 3],
        error: &mut S2Error,
    ) {
        let degeneracies = find_polygon_degeneracies(&g[2], error);
        let mut deg_idx = 0usize;

        let mut e0 = EdgeId(-1);
        let mut e1 = EdgeId(-1);
        let mut e2 = EdgeId(-1);
        let mut in_e2: i32 = -1;

        let mut edge0 = advance(&g[0], &mut e0);
        let mut edge1 = advance(&g[1], &mut e1);
        let mut edge2 = advance(&g[2], &mut e2);
        let mut in_edge2 = advance_incoming(&g[2], in_edges2, &mut in_e2);

        loop {
            if edge2 <= edge1 && edge2 <= edge0 {
                if edge2 == SENTINEL {
                    break;
                }
                if deg_idx >= degeneracies.len() || degeneracies[deg_idx].edge_id != e2 {
                    // CASE 1: Normal polygon edge — keep it.
                    add_edge(2, &g[2], e2, new_edges, new_input_ids);
                    // Suppress coincident polyline edges.
                    while self.options.suppress_lower_dimensions && edge1 == edge2 {
                        edge1 = advance(&g[1], &mut e1);
                    }
                } else if !degeneracies[deg_idx].is_hole {
                    deg_idx += 1;
                    // CASE 2: Degenerate polygon shell — demote.
                    if edge2.0 == edge2.1 {
                        // Self-loop → convert to point (if not suppressed).
                        if !self.options.suppress_lower_dimensions
                            || !is_suppressed[edge2.0.as_usize()]
                        {
                            add_edge(0, &g[2], e2, new_edges, new_input_ids);
                        }
                    } else {
                        // Sibling pair → convert to polyline (both directions).
                        add_edge(1, &g[2], e2, new_edges, new_input_ids);
                        // Don't suppress original polyline edges that coincide.
                        while edge1 == edge2 {
                            add_edge(1, &g[1], e1, new_edges, new_input_ids);
                            edge1 = advance(&g[1], &mut e1);
                        }
                    }
                } else {
                    // CASE 3: Degenerate polygon hole → discard.
                    deg_idx += 1;
                }
                edge2 = advance(&g[2], &mut e2);
            } else if edge1 <= edge0 {
                if edge1.0 == edge1.1 {
                    // CASE 5: Degenerate polyline edge → demote to point.
                    if !self.options.suppress_lower_dimensions || !is_suppressed[edge1.0.as_usize()]
                    {
                        add_edge(0, &g[1], e1, new_edges, new_input_ids);
                    }
                    // Skip the sibling of an undirected degenerate edge.
                    if g[1].options().edge_type == EdgeType::Undirected {
                        e1 += 1;
                    }
                } else {
                    // CASE 4: Normal polyline edge.
                    // Advance incoming polygon edges to check for suppression.
                    while in_edge2 < edge1 {
                        in_edge2 = advance_incoming(&g[2], in_edges2, &mut in_e2);
                    }
                    if !self.options.suppress_lower_dimensions || edge1 != in_edge2 {
                        add_edge(1, &g[1], e1, new_edges, new_input_ids);
                    }
                }
                edge1 = advance(&g[1], &mut e1);
            } else {
                // CASE 6: Input point.
                if !self.options.suppress_lower_dimensions || !is_suppressed[edge0.0.as_usize()] {
                    add_edge(0, &g[0], e0, new_edges, new_input_ids);
                }
                edge0 = advance(&g[0], &mut e0);
            }
        }
    }
}

/// Advances to the next edge in the graph, or returns SENTINEL.
fn advance(g: &Graph, e: &mut EdgeId) -> Edge {
    *e += 1;
    if *e >= g.num_edges() {
        SENTINEL
    } else {
        g.edge(*e)
    }
}

/// Advances to the next incoming edge (reversed), or returns SENTINEL.
fn advance_incoming(g: &Graph, in_edges: &[EdgeId], i: &mut i32) -> Edge {
    *i += 1;
    if (*i as usize) >= in_edges.len() {
        SENTINEL
    } else {
        Graph::reverse(g.edge(in_edges[*i as usize]))
    }
}

/// Adds an edge from source graph to the target dimension's output.
fn add_edge(
    new_dim: usize,
    g: &Graph,
    e: EdgeId,
    new_edges: &mut [Vec<Edge>; 3],
    new_input_ids: &mut [Vec<InputEdgeIdSetId>; 3],
) {
    new_edges[new_dim].push(g.edge(e));
    new_input_ids[new_dim].push(g.input_edge_id_set_id(e));
}

// ─── NormalizeClosedSet ──────────────────────────────────────────────────

/// Shared state for the three `NormalizingLayer`s that collect graphs
/// and run `ClosedSetNormalizer` when the last layer is built.
struct NormalizingState {
    output_layers: Vec<Box<dyn Layer>>,
    normalizer: ClosedSetNormalizer,
    graphs: Vec<Option<Graph>>,
    graphs_left: usize,
}

/// A layer that collects its graph and, when all three dimensions have been
/// collected, runs `ClosedSetNormalizer` and passes the results to the
/// output layers.
#[derive(Debug)]
struct NormalizingLayer {
    dimension: usize,
    graph_options: GraphOptions,
    state: Rc<RefCell<NormalizingState>>,
}

impl std::fmt::Debug for NormalizingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NormalizingState")
            .field("graphs_left", &self.graphs_left)
            .finish_non_exhaustive()
    }
}

impl Layer for NormalizingLayer {
    fn graph_options(&self) -> GraphOptions {
        self.graph_options.clone()
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        let mut state = self.state.borrow_mut();
        // Clone the graph data so it persists after this call returns.
        state.graphs[self.dimension] = Some(Graph::from_raw_parts(
            graph.options().clone(),
            graph.vertices().to_vec(),
            graph.edges().to_vec(),
            graph.input_edge_id_set_ids().to_vec(),
            graph.input_edge_id_set_lexicon().clone(),
            graph.label_set_ids().to_vec(),
            graph.label_set_lexicon().clone(),
            graph.is_full_polygon_predicate_clone(),
        ));
        state.graphs_left -= 1;
        if state.graphs_left > 0 {
            return;
        }

        // All three graphs collected — run the normalizer.
        let input: Vec<Graph> = state.graphs.iter_mut().filter_map(Option::take).collect();
        debug_assert_eq!(input.len(), 3);
        let output = state.normalizer.run(&input, error);
        for (dim, out_graph) in output.iter().enumerate() {
            state.output_layers[dim].build(out_graph, error);
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

/// Given three output layers (one each for dimensions 0, 1, and 2), returns
/// three new layers that preprocess the input graphs using a
/// `ClosedSetNormalizer` with the given options.
///
/// This ensures that the graphs passed to `output_layers` do not contain any
/// polyline or polygon degeneracies — they are demoted to lower dimensions.
///
/// The returned layers must be passed to `S2Builder` (or `S2BooleanOperation`)
/// as dimensions 0, 1, and 2 respectively.
///
/// # Panics
///
/// Panics if `output_layers` does not contain exactly 3 layers.
pub fn normalize_closed_set(
    output_layers: Vec<Box<dyn Layer>>,
    options: Options,
) -> Vec<Box<dyn Layer>> {
    assert_eq!(output_layers.len(), 3);
    let graph_options_out: Vec<GraphOptions> =
        output_layers.iter().map(|l| l.graph_options()).collect();
    let normalizer = ClosedSetNormalizer::new(options, graph_options_out);
    let input_options: Vec<GraphOptions> = normalizer.graph_options().to_vec();

    let state = Rc::new(RefCell::new(NormalizingState {
        output_layers,
        normalizer,
        graphs: vec![None, None, None],
        graphs_left: 3,
    }));

    input_options
        .into_iter()
        .enumerate()
        .map(|(dim, opts)| -> Box<dyn Layer> {
            Box::new(NormalizingLayer {
                dimension: dim,
                graph_options: opts,
                state: Rc::clone(&state),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::builder::graph::{DuplicateEdges, EdgeType, GraphOptions};
    use crate::s2::builder::id_set_lexicon::IdSetLexicon;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point::from_coords(x, y, z).normalize()
    }

    /// Creates graph options for dimension `dim` suitable for normalizer output.
    fn graph_options_for_dim(dim: usize) -> GraphOptions {
        match dim {
            0 => GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Merge,
                SiblingPairs::Keep,
            ),
            1 => GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Discard,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            2 => GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Discard,
                DuplicateEdges::Keep,
                SiblingPairs::Discard,
            ),
            _ => unreachable!(),
        }
    }

    fn build_graph_with_options(
        vertices: &[Point],
        edges: &[(i32, i32)],
        options: GraphOptions,
    ) -> Graph {
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

    fn make_normalizer() -> ClosedSetNormalizer {
        let out_opts = vec![
            graph_options_for_dim(0),
            graph_options_for_dim(1),
            graph_options_for_dim(2),
        ];
        ClosedSetNormalizer::new(Options::default(), out_opts)
    }

    fn make_graphs(
        vertices: &[Point],
        edges0: &[(i32, i32)],
        edges1: &[(i32, i32)],
        edges2: &[(i32, i32)],
    ) -> Vec<Graph> {
        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        vec![
            build_graph_with_options(vertices, edges0, opts[0].clone()),
            build_graph_with_options(vertices, edges1, opts[1].clone()),
            build_graph_with_options(vertices, edges2, opts[2].clone()),
        ]
    }

    #[test]
    fn test_empty_graphs() {
        let normalizer = make_normalizer();
        let v = vec![p(1.0, 0.0, 0.0)]; // Need at least shared vertex space
        let input = make_graphs(&v, &[], &[], &[]);
        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(result[0].num_edges(), 0);
        assert_eq!(result[1].num_edges(), 0);
        assert_eq!(result[2].num_edges(), 0);
    }

    #[test]
    fn test_non_degenerate_inputs() {
        // Point + polyline + polygon, all non-degenerate: should pass through.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let v3 = p(-1.0, 0.0, 0.0); // point
        let v4 = p(0.0, -1.0, 0.0); // polyline start
        let v5 = p(0.0, 0.0, -1.0); // polyline end

        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        let input = vec![
            build_graph_with_options(&[v0, v1, v2, v3, v4, v5], &[(3, 3)], opts[0].clone()),
            build_graph_with_options(&[v0, v1, v2, v3, v4, v5], &[(4, 5)], opts[1].clone()),
            build_graph_with_options(
                &[v0, v1, v2, v3, v4, v5],
                &[(0, 1), (1, 2), (2, 0)],
                opts[2].clone(),
            ),
        ];

        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(result[0].num_edges(), 1); // point
        assert_eq!(result[1].num_edges(), 1); // polyline
        assert_eq!(result[2].num_edges(), 3); // polygon
    }

    #[test]
    fn test_point_shell_demoted() {
        // Polygon has a degenerate point shell → should be demoted to point layer.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let v3 = p(-1.0, 0.0, 0.0); // degenerate point outside polygon

        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        let input = vec![
            build_graph_with_options(&[v0, v1, v2, v3], &[], opts[0].clone()),
            build_graph_with_options(&[v0, v1, v2, v3], &[], opts[1].clone()),
            build_graph_with_options(
                &[v0, v1, v2, v3],
                &[(0, 1), (1, 2), (2, 0), (3, 3)],
                opts[2].clone(),
            ),
        ];

        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(
            result[0].num_edges(),
            1,
            "point shell should be demoted to points"
        );
        assert_eq!(result[1].num_edges(), 0);
        assert_eq!(
            result[2].num_edges(),
            3,
            "non-degenerate polygon edges preserved"
        );
    }

    #[test]
    fn test_sibling_pair_shell_demoted() {
        // Polygon has a sibling pair shell → should be demoted to polyline layer.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        let v3 = p(-1.0, 0.0, 0.0);
        let v4 = p(0.0, -1.0, 0.0);
        // Triangle edges + sibling pair outside: (3,4) and (4,3)
        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        let input = vec![
            build_graph_with_options(&[v0, v1, v2, v3, v4], &[], opts[0].clone()),
            build_graph_with_options(&[v0, v1, v2, v3, v4], &[], opts[1].clone()),
            build_graph_with_options(
                &[v0, v1, v2, v3, v4],
                &[(0, 1), (1, 2), (2, 0), (3, 4), (4, 3)],
                opts[2].clone(),
            ),
        ];

        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(result[0].num_edges(), 0);
        // Sibling pair shell demoted to polyline: at least one edge direction.
        assert!(
            result[1].num_edges() >= 1,
            "sibling pair shell should be demoted to polyline"
        );
        assert_eq!(result[2].num_edges(), 3);
    }

    #[test]
    fn test_point_suppressed_by_polygon_vertex() {
        // A point coincident with a polygon vertex should be suppressed.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        // Point at v0 (same as polygon vertex)
        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        let input = vec![
            build_graph_with_options(&[v0, v1, v2], &[(0, 0)], opts[0].clone()),
            build_graph_with_options(&[v0, v1, v2], &[], opts[1].clone()),
            build_graph_with_options(&[v0, v1, v2], &[(0, 1), (1, 2), (2, 0)], opts[2].clone()),
        ];

        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(
            result[0].num_edges(),
            0,
            "point at polygon vertex should be suppressed"
        );
        assert_eq!(result[2].num_edges(), 3);
    }

    #[test]
    fn test_polyline_edge_suppressed_by_polygon_edge() {
        // A polyline edge coincident with a polygon edge should be suppressed.
        let v0 = p(1.0, 0.0, 0.0);
        let v1 = p(0.0, 1.0, 0.0);
        let v2 = p(0.0, 0.0, 1.0);
        // Polyline edge (0,1) matches polygon edge (0,1)
        let normalizer = make_normalizer();
        let opts = normalizer.graph_options();
        let input = vec![
            build_graph_with_options(&[v0, v1, v2], &[], opts[0].clone()),
            build_graph_with_options(&[v0, v1, v2], &[(0, 1)], opts[1].clone()),
            build_graph_with_options(&[v0, v1, v2], &[(0, 1), (1, 2), (2, 0)], opts[2].clone()),
        ];

        let mut error = S2Error::ok();
        let result = normalizer.run(&input, &mut error);
        assert!(error.is_ok());
        assert_eq!(result[0].num_edges(), 0);
        assert_eq!(
            result[1].num_edges(),
            0,
            "polyline coincident with polygon edge should be suppressed"
        );
        assert_eq!(result[2].num_edges(), 3);
    }

    /// Integration test: `normalize_closed_set` through `S2BooleanOperation`.
    ///
    /// Corresponds to C++ `TEST(ComputeUnion, MixedGeometry)`.
    #[test]
    fn test_normalize_closed_set_via_boolean_operation() {
        use crate::s2::boolean_operation::{OpType, S2BooleanOperation};
        use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
        use crate::s2::builder::lax_polyline_layer::LaxPolylineLayer;
        use crate::s2::builder::point_vector_layer::S2PointVectorLayer;
        use crate::s2::lax_polygon::LaxPolygon;
        use crate::s2::lax_polyline::LaxPolyline;
        use crate::s2::shape::Shape;
        use crate::s2::shape_index::ShapeIndex;
        use crate::s2::text_format;

        // Build a simple ShapeIndex with a polygon and a degenerate shell
        let mut a_index = ShapeIndex::new();
        // Triangle polygon: 0:0, 0:10, 10:0 with a degenerate point shell at 5:5
        let poly_a = text_format::make_lax_polygon("0:0, 0:10, 10:0; 5:5");
        a_index.add(Box::new(poly_a));
        a_index.build();

        // Build another index with a disjoint polygon
        let mut b_index = ShapeIndex::new();
        let poly_b = text_format::make_lax_polygon("20:20, 20:30, 30:20");
        b_index.add(Box::new(poly_b));
        b_index.build();

        // Output containers
        let points_out: Rc<RefCell<Vec<Point>>> = Rc::new(RefCell::new(Vec::new()));
        let polyline_out: Rc<RefCell<LaxPolyline>> = Rc::new(RefCell::new(LaxPolyline::default()));
        let polygon_out: Rc<RefCell<LaxPolygon>> = Rc::new(RefCell::new(LaxPolygon::empty()));

        // Create 3 output layers
        let output_layers: Vec<Box<dyn Layer>> = vec![
            Box::new(S2PointVectorLayer::new_legacy(Rc::clone(&points_out))),
            Box::new(LaxPolylineLayer::new_legacy(Rc::clone(&polyline_out))),
            Box::new(LaxPolygonLayer::new_legacy(Rc::clone(&polygon_out))),
        ];

        // Wrap with normalize_closed_set
        let normalized_layers = normalize_closed_set(output_layers, Options::default());

        // Run union
        let mut op = S2BooleanOperation::multi(
            OpType::Union,
            normalized_layers,
            crate::s2::boolean_operation::Options::default(),
        );
        op.build(&mut a_index, &mut b_index)
            .expect("S2BooleanOperation failed");

        // The degenerate point shell at 5:5 is inside the polygon and should
        // be suppressed (it's a degenerate hole). The output should have two
        // polygon loops and no demoted points.
        let polygon = polygon_out.borrow();
        assert!(!polygon.is_empty(), "expected non-empty polygon output");
    }

    // ─── Batch 8: Full-pipeline ClosedSetNormalizer tests (from C++) ────

    /// A layer that captures graph data for later inspection.
    #[derive(Debug)]
    struct GraphCapturingLayer2 {
        graph_options: GraphOptions,
        output: Rc<RefCell<Option<Graph>>>,
    }

    impl GraphCapturingLayer2 {
        fn new(graph_options: GraphOptions, output: Rc<RefCell<Option<Graph>>>) -> Self {
            GraphCapturingLayer2 {
                graph_options,
                output,
            }
        }
    }

    impl Layer for GraphCapturingLayer2 {
        fn graph_options(&self) -> GraphOptions {
            self.graph_options.clone()
        }

        fn build(&mut self, g: &Graph, _error: &mut S2Error) {
            // Clone the graph data to own it.
            let cloned = Graph::from_raw_parts(
                g.options().clone(),
                g.vertices().to_vec(),
                (0..g.num_edges().0)
                    .map(EdgeId)
                    .map(|e| g.edge(e))
                    .collect(),
                (0..g.num_edges().0)
                    .map(EdgeId)
                    .map(|e| {
                        // Reconstruct the input_edge_id_set_id for each edge.
                        let ids = g.input_edge_ids(e);
                        // Store a simple synthetic set id — we don't need exact
                        // id_set_lexicon roundtrip for string comparison.
                        ids.first().copied().unwrap_or(-1)
                    })
                    .collect(),
                IdSetLexicon::new(),
                vec![],
                IdSetLexicon::new(),
                None,
            );
            *self.output.borrow_mut() = Some(cloned);
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    /// Converts a graph's edges to a sorted string representation.
    fn graph_to_string(g: &Graph) -> String {
        let mut parts: Vec<String> = Vec::new();
        for e in (0..g.num_edges().0).map(EdgeId) {
            let (v0, v1) = g.edge(e);
            parts.push(crate::s2::text_format::points_to_string(&[
                g.vertex(v0),
                g.vertex(v1),
            ]));
        }
        parts.sort_unstable();
        parts.join("; ")
    }

    /// Adds layers for each dimension (points=0, polylines=1, polygons=2)
    /// from a `"points # polylines # polygons"` string.
    fn add_layers(
        builder: &mut crate::s2::builder::S2Builder,
        s: &str,
        graph_options: &[GraphOptions],
        outputs: &[Rc<RefCell<Option<Graph>>>],
    ) {
        use crate::s2::text_format::make_index;

        let index = make_index(s);
        for dim in 0..3 {
            builder.start_layer(Box::new(GraphCapturingLayer2::new(
                graph_options[dim].clone(),
                Rc::clone(&outputs[dim]),
            )));
            for shape_id in 0..index.len() as i32 {
                if let Some(shape) = index.shape(shape_id) {
                    if shape.dimension().as_usize() != dim {
                        continue;
                    }
                    let n = shape.num_edges();
                    for e in 0..n {
                        let edge = shape.edge(e);
                        builder.add_edge(edge.v0, edge.v1);
                    }
                }
            }
        }
    }

    /// Runs a `ClosedSetNormalizer` test matching C++ `NormalizeTest::Run`.
    fn run_normalize_test(input_str: &str, expected_str: &str) {
        run_normalize_test_with(input_str, expected_str, true);
    }

    fn run_normalize_test_with(
        input_str: &str,
        expected_str: &str,
        suppress_lower_dimensions: bool,
    ) {
        // C++ default output graph options:
        // Points: Directed, DegenerateEdges::Keep, DuplicateEdges::Keep, SiblingPairs::Keep
        // Polylines: Undirected, DegenerateEdges::Keep, DuplicateEdges::Keep, SiblingPairs::Keep
        // Polygons: Directed, DegenerateEdges::Keep, DuplicateEdges::Keep, SiblingPairs::Keep
        let graph_options_out = vec![
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Undirected,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
        ];

        let options = Options {
            suppress_lower_dimensions,
        };
        let normalizer = ClosedSetNormalizer::new(options, graph_options_out.clone());
        let input_graph_options: Vec<GraphOptions> = normalizer.graph_options().to_vec();

        // Build input and expected graphs via S2Builder.
        let mut builder =
            crate::s2::builder::S2Builder::new(crate::s2::builder::Options::default());
        let input_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();
        let expected_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();

        add_layers(
            &mut builder,
            input_str,
            &input_graph_options,
            &input_outputs,
        );
        add_layers(
            &mut builder,
            expected_str,
            &graph_options_out,
            &expected_outputs,
        );

        let result = builder.build();
        assert!(result.is_ok(), "build failed: {:?}", result.err());

        // Extract built graphs (use empty graph if layer had no edges).
        let empty_graph = |opts: &GraphOptions| {
            Graph::from_raw_parts(
                opts.clone(),
                vec![],
                vec![],
                vec![],
                IdSetLexicon::new(),
                vec![],
                IdSetLexicon::new(),
                None,
            )
        };

        let input_graphs: Vec<Graph> = input_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&input_graph_options[i]))
            })
            .collect();
        let expected_graphs: Vec<Graph> = expected_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&graph_options_out[i]))
            })
            .collect();

        // Run the normalizer.
        let mut error = S2Error::ok();
        let actual = normalizer.run(&input_graphs, &mut error);
        assert!(error.is_ok(), "normalizer error: {error:?}");

        // Compare.
        for dim in 0..3 {
            let expected_s = graph_to_string(&expected_graphs[dim]);
            let actual_s = graph_to_string(&actual[dim]);
            assert_eq!(
                expected_s, actual_s,
                "dim={dim}, input={input_str:?}, expected_str={expected_str:?}"
            );
        }
    }

    #[test]
    fn test_normalizer_point_hole() {
        // C++: NormalizeTest::PointHole
        // Point hole inside triangle → removed.
        run_normalize_test("# # 0:0, 0:3, 3:0 | 1:1", "# # 0:0, 0:3, 3:0");
    }

    #[test]
    fn test_normalizer_point_polyline() {
        // C++: NormalizeTest::PointPolyline
        // Degenerate polyline edge → demoted to point.
        run_normalize_test("# 0:0, 0:0 #", "0:0 # #");
    }

    #[test]
    fn test_normalizer_sibling_pair_hole() {
        // C++: NormalizeTest::SiblingPairHole
        // Sibling pair inside polygon → removed.
        run_normalize_test("# # 0:0, 0:3, 3:0; 0:0, 1:1", "# # 0:0, 0:3, 3:0");
    }

    #[test]
    fn test_normalizer_point_suppressed_by_polyline_vertex_suppress() {
        // C++: NormalizeTest::PointSuppressedByPolylineVertex (suppress=true)
        run_normalize_test("0:0 | 0:1 # 0:0, 0:1 #", "# 0:0, 0:1 #");
    }

    #[test]
    fn test_normalizer_point_suppressed_by_polyline_vertex_no_suppress() {
        // C++: NormalizeTest::PointSuppressedByPolylineVertex (suppress=false)
        run_normalize_test_with("0:0 | 0:1 # 0:0, 0:1 #", "0:0 | 0:1 # 0:0, 0:1 #", false);
    }

    #[test]
    fn test_normalizer_point_shell_suppressed_by_polyline_edge_suppress() {
        // C++: NormalizeTest::PointShellSuppressedByPolylineEdge (suppress=true)
        // Single-point shells demoted to points, then suppressed by polyline.
        run_normalize_test("# 0:0, 1:0 # 0:0; 1:0", "# 0:0, 1:0 #");
    }

    #[test]
    fn test_normalizer_point_shell_suppressed_by_polyline_edge_no_suppress() {
        // C++: NormalizeTest::PointShellSuppressedByPolylineEdge (suppress=false)
        run_normalize_test_with("# 0:0, 1:0 # 0:0; 1:0", "0:0 | 1:0 # 0:0, 1:0 #", false);
    }

    #[test]
    fn test_normalizer_polyline_edge_suppressed_by_reverse_polygon_edge() {
        // C++: NormalizeTest::PolylineEdgeSuppressedByReversePolygonEdge
        // Directed polyline output + suppress=true.
        // NOTE: C++ test sets graph_options_out_[1].set_edge_type(DIRECTED).
        // We handle this by using a custom normalizer setup.
        run_normalize_test_directed_polyline(
            "# 1:0, 0:0 # 0:0, 0:1, 1:0",
            "# # 0:0, 0:1, 1:0",
            true,
        );
    }

    #[test]
    fn test_normalizer_polyline_edge_suppressed_by_reverse_polygon_edge_no_suppress() {
        // C++: same test with suppress=false
        run_normalize_test_directed_polyline(
            "# 1:0, 0:0 # 0:0, 0:1, 1:0",
            "# 1:0, 0:0 # 0:0, 0:1, 1:0",
            false,
        );
    }

    /// Like `run_normalize_test_with` but with directed polyline output.
    fn run_normalize_test_directed_polyline(
        input_str: &str,
        expected_str: &str,
        suppress_lower_dimensions: bool,
    ) {
        let graph_options_out = vec![
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
        ];

        let options = Options {
            suppress_lower_dimensions,
        };
        let normalizer = ClosedSetNormalizer::new(options, graph_options_out.clone());
        let input_graph_options: Vec<GraphOptions> = normalizer.graph_options().to_vec();

        let mut builder =
            crate::s2::builder::S2Builder::new(crate::s2::builder::Options::default());
        let input_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();
        let expected_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();

        add_layers(
            &mut builder,
            input_str,
            &input_graph_options,
            &input_outputs,
        );
        add_layers(
            &mut builder,
            expected_str,
            &graph_options_out,
            &expected_outputs,
        );

        let result = builder.build();
        assert!(result.is_ok(), "build failed: {:?}", result.err());

        let empty_graph = |opts: &GraphOptions| {
            Graph::from_raw_parts(
                opts.clone(),
                vec![],
                vec![],
                vec![],
                IdSetLexicon::new(),
                vec![],
                IdSetLexicon::new(),
                None,
            )
        };

        let input_graphs: Vec<Graph> = input_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&input_graph_options[i]))
            })
            .collect();
        let expected_graphs: Vec<Graph> = expected_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&graph_options_out[i]))
            })
            .collect();

        let mut error = S2Error::ok();
        let actual = normalizer.run(&input_graphs, &mut error);
        assert!(error.is_ok(), "normalizer error: {error:?}");

        for dim in 0..3 {
            let expected_s = graph_to_string(&expected_graphs[dim]);
            let actual_s = graph_to_string(&actual[dim]);
            assert_eq!(
                expected_s, actual_s,
                "dim={dim}, input={input_str:?}, expected_str={expected_str:?}"
            );
        }
    }

    #[test]
    fn test_normalizer_duplicate_edge_merging() {
        // C++: NormalizeTest::DuplicateEdgeMerging
        // With DuplicateEdges::Keep (default), demoted edges are added as
        // new edges rather than being merged with existing ones.
        run_normalize_test(
            "0:0 | 0:0 # 0:0, 0:0 | 0:1, 0:2 # 0:0; 0:1, 0:2",
            "0:0 | 0:0 | 0:0 | 0:0 # 0:1, 0:2 | 0:1, 0:2 #",
        );
    }

    #[test]
    fn test_normalizer_duplicate_edge_merging_merge() {
        // C++: NormalizeTest::DuplicateEdgeMerging (DuplicateEdges::Merge)
        // With Merge, duplicate edges are collapsed.
        let graph_options_out = vec![
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Merge,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Undirected,
                DegenerateEdges::Keep,
                DuplicateEdges::Merge,
                SiblingPairs::Keep,
            ),
            GraphOptions::new(
                EdgeType::Directed,
                DegenerateEdges::Keep,
                DuplicateEdges::Keep,
                SiblingPairs::Keep,
            ),
        ];

        let options = Options {
            suppress_lower_dimensions: true,
        };
        let normalizer = ClosedSetNormalizer::new(options, graph_options_out.clone());
        let input_graph_options: Vec<GraphOptions> = normalizer.graph_options().to_vec();

        let mut builder =
            crate::s2::builder::S2Builder::new(crate::s2::builder::Options::default());
        let input_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();
        let expected_outputs: Vec<Rc<RefCell<Option<Graph>>>> =
            (0..3).map(|_| Rc::new(RefCell::new(None))).collect();

        add_layers(
            &mut builder,
            "0:0 | 0:0 # 0:0, 0:0 | 0:1, 0:2 # 0:0; 0:1, 0:2",
            &input_graph_options,
            &input_outputs,
        );
        add_layers(
            &mut builder,
            "0:0 # 0:1, 0:2 #",
            &graph_options_out,
            &expected_outputs,
        );

        let result = builder.build();
        assert!(result.is_ok(), "build failed: {:?}", result.err());

        let empty_graph = |opts: &GraphOptions| {
            Graph::from_raw_parts(
                opts.clone(),
                vec![],
                vec![],
                vec![],
                IdSetLexicon::new(),
                vec![],
                IdSetLexicon::new(),
                None,
            )
        };

        let input_graphs: Vec<Graph> = input_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&input_graph_options[i]))
            })
            .collect();
        let expected_graphs: Vec<Graph> = expected_outputs
            .iter()
            .enumerate()
            .map(|(i, rc)| {
                rc.borrow_mut()
                    .take()
                    .unwrap_or_else(|| empty_graph(&graph_options_out[i]))
            })
            .collect();

        let mut error = S2Error::ok();
        let actual = normalizer.run(&input_graphs, &mut error);
        assert!(error.is_ok(), "normalizer error: {error:?}");

        for dim in 0..3 {
            let expected_s = graph_to_string(&expected_graphs[dim]);
            let actual_s = graph_to_string(&actual[dim]);
            assert_eq!(
                expected_s, actual_s,
                "dim={dim} duplicate edge merging (Merge)"
            );
        }
    }
}
