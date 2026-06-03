// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// LaxPolygonLayer: assembles edges into a LaxPolygon.

use crate::s2::Point;
use crate::s2::lax_polygon::LaxPolygon;

use super::S2Error;
use super::graph::{
    DegenerateEdges, DuplicateEdges, EdgeType, Graph, GraphOptions, LabelFetcher, LoopType,
    SiblingPairs,
};
use super::graph::{Edge, EdgeId};
use super::id_set_lexicon::IdSetLexicon;
use super::layer::Layer;
use super::{InputEdgeIdSetId, LabelSetId, S2ErrorCode};

/// Per-loop label set IDs: `label_set_ids[i][j]` gives the `LabelSetId`
/// for edge `j` of loop `i`. Decode via the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<Vec<LabelSetId>>;

/// Controls how degenerate boundaries are handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DegenerateBoundaries {
    /// Discard all degenerate boundaries (loops with < 3 vertices).
    Discard,
    /// Discard degenerate holes but keep degenerate shells.
    DiscardHoles,
    /// Discard degenerate shells but keep degenerate holes.
    DiscardShells,
    /// Keep all degenerate boundaries.
    #[default]
    Keep,
}

/// Options for `LaxPolygonLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
    /// How degenerate boundaries are handled.
    pub degenerate_boundaries: DegenerateBoundaries,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            edge_type: EdgeType::Directed,
            degenerate_boundaries: DegenerateBoundaries::Keep,
        }
    }
}

/// A layer that assembles edges into a `LaxPolygon`.
///
/// Unlike `S2PolygonLayer`, `LaxPolygon` supports degeneracies (empty loops,
/// sibling pairs as degenerate edges).
#[derive(Debug)]
pub struct LaxPolygonLayer {
    polygon: Option<LaxPolygon>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Legacy shared output for backward-compatible test code.
    #[cfg(test)]
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<LaxPolygon>>>,
}

impl LaxPolygonLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        LaxPolygonLayer {
            polygon: None,
            options: Options::default(),
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: false,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Creates a new layer with the given options.
    pub fn with_options(options: Options) -> Self {
        LaxPolygonLayer {
            polygon: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: false,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Creates a new layer that also collects per-edge label sets.
    pub fn with_labels(options: Options) -> Self {
        LaxPolygonLayer {
            polygon: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: true,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Consumes this layer and returns the built polygon.
    ///
    /// # Panics
    ///
    /// Panics if `build()` was not called or returned an error.
    #[expect(
        clippy::expect_used,
        reason = "panics are documented; caller must call build() first"
    )]
    pub fn into_output(self) -> LaxPolygon {
        self.polygon
            .expect("LaxPolygonLayer::build() was not called")
    }

    /// Returns a reference to the built polygon.
    pub fn output(&self) -> Option<&LaxPolygon> {
        self.polygon.as_ref()
    }

    /// Takes the built polygon out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<LaxPolygon> {
        self.polygon.take()
    }

    /// Returns the per-loop label set IDs.
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon.
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }

    /// Collects edge labels from the graph for each loop's edges.
    fn append_edge_labels(
        &self,
        graph: &Graph,
        edge_loops: &[Vec<EdgeId>],
        out_ids: &mut LabelSetIds,
        out_lexicon: &mut IdSetLexicon,
    ) {
        let fetcher = LabelFetcher::new(graph, self.options.edge_type);
        for edge_loop in edge_loops {
            let mut loop_ids = Vec::with_capacity(edge_loop.len());
            for &edge_id in edge_loop {
                let labels = fetcher.fetch(graph, edge_id);
                loop_ids.push(out_lexicon.add_set(&labels));
            }
            out_ids.push(loop_ids);
        }
    }
}

impl Default for LaxPolygonLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl LaxPolygonLayer {
    /// Legacy constructor for test backward compatibility.
    pub fn new_legacy(output: std::rc::Rc<std::cell::RefCell<LaxPolygon>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<LaxPolygon>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_options(options);
        s.legacy_output = Some(output);
        s
    }

    /// Syncs output to legacy Rc<RefCell> if present.
    fn sync_legacy(&self) {
        if let (Some(output), Some(legacy)) = (&self.polygon, &self.legacy_output) {
            *legacy.borrow_mut() = output.clone();
        }
    }
}

impl Layer for LaxPolygonLayer {
    fn graph_options(&self) -> GraphOptions {
        if self.options.degenerate_boundaries == DegenerateBoundaries::Discard {
            GraphOptions::new(
                self.options.edge_type,
                DegenerateEdges::Discard,
                DuplicateEdges::Keep,
                SiblingPairs::Discard,
            )
        } else {
            GraphOptions::new(
                self.options.edge_type,
                DegenerateEdges::DiscardExcess,
                DuplicateEdges::Keep,
                SiblingPairs::DiscardExcess,
            )
        }
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        use super::find_polygon_degeneracies::{find_polygon_degeneracies, is_fully_degenerate};

        if graph.options().edge_type == EdgeType::Undirected {
            *error = S2Error::new(
                S2ErrorCode::Unimplemented,
                "Undirected edges not supported yet",
            );
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        // Some cases are implemented by constructing a new graph with certain
        // degenerate edges removed.
        let mut new_edges: Vec<Edge> = Vec::new();
        let mut new_input_edge_id_set_ids: Vec<InputEdgeIdSetId> = Vec::new();
        let mut loops: Vec<Vec<Point>> = Vec::new();
        let db = self.options.degenerate_boundaries;

        // Determine which graph to use (original or filtered).
        let work_graph;
        let g: &Graph = if db == DegenerateBoundaries::Discard {
            // Easiest case: no degeneracies to handle.
            if graph.num_edges() == 0 {
                maybe_add_full_loop(graph, &mut loops, error);
            }
            graph
        } else if db == DegenerateBoundaries::Keep {
            // S2LaxPolygonShape doesn't need to distinguish degenerate shells
            // from holes except when the entire graph is degenerate.
            if is_fully_degenerate(graph) {
                maybe_add_full_loop(graph, &mut loops, error);
            }
            graph
        } else {
            // For DISCARD_SHELLS and DISCARD_HOLES we first determine whether
            // any degenerate loops of the given type exist, and if so construct
            // a new graph with those edges removed.
            let discard_holes = db == DegenerateBoundaries::DiscardHoles;
            let degeneracies = find_polygon_degeneracies(graph, error);
            if !error.is_ok() {
                #[cfg(test)]
                self.sync_legacy();
                return;
            }
            if degeneracies.len() == graph.num_edges().as_usize() {
                if degeneracies.is_empty() {
                    maybe_add_full_loop(graph, &mut loops, error);
                } else if degeneracies[0].is_hole {
                    loops.push(Vec::new()); // Full loop.
                }
            }
            let mut edges_to_discard: Vec<EdgeId> = Vec::new();
            for deg in &degeneracies {
                if deg.is_hole == discard_holes {
                    edges_to_discard.push(deg.edge_id);
                }
            }
            if edges_to_discard.is_empty() {
                graph
            } else {
                edges_to_discard.sort_unstable();
                discard_edges(
                    graph,
                    &edges_to_discard,
                    &mut new_edges,
                    &mut new_input_edge_id_set_ids,
                );
                let mut lexicon = graph.input_edge_id_set_lexicon().clone();
                work_graph = graph.make_subgraph(
                    graph.options().clone(),
                    &mut new_edges,
                    &mut new_input_edge_id_set_ids,
                    &mut lexicon,
                    graph.is_full_polygon_predicate_clone(),
                    error,
                );
                &work_graph
            }
        };

        if !error.is_ok() {
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        // Extract loops from the (possibly filtered) graph.
        let edge_loops = g.get_directed_loops(LoopType::Circuit, error);
        if !error.is_ok() {
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        for loop_edges in &edge_loops {
            let mut vertices: Vec<Point> = Vec::with_capacity(loop_edges.len());
            for &eid in loop_edges {
                let (v0, _) = g.edge(eid);
                vertices.push(g.vertex(v0));
            }
            loops.push(vertices);
        }

        // Collect labels if requested.
        if self.track_labels {
            let mut ids = self.label_set_ids.take().unwrap_or_default();
            let mut lex = self.label_set_lexicon.take().unwrap_or_default();
            self.append_edge_labels(g, &edge_loops, &mut ids, &mut lex);
            self.label_set_ids = Some(ids);
            self.label_set_lexicon = Some(lex);
        }

        let loop_refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
        self.polygon = Some(LaxPolygon::from_loops(&loop_refs));

        // Sync to legacy Rc<RefCell> output if present (test backward compat).
        #[cfg(test)]
        self.sync_legacy();
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

/// If the graph represents a full polygon, adds an empty loop (the
/// canonical representation of a full loop in `LaxPolygon`).
fn maybe_add_full_loop(g: &Graph, loops: &mut Vec<Vec<Point>>, error: &mut S2Error) {
    match g.is_full_polygon() {
        Ok(true) => loops.push(Vec::new()), // Full loop.
        Ok(false) => {}
        Err(e) => *error = e,
    }
}

/// Returns all edges of `g` except for those identified by `edges_to_discard`.
/// `edges_to_discard` must be sorted.
fn discard_edges(
    g: &Graph,
    edges_to_discard: &[EdgeId],
    new_edges: &mut Vec<Edge>,
    new_input_edge_id_set_ids: &mut Vec<InputEdgeIdSetId>,
) {
    debug_assert!(edges_to_discard.windows(2).all(|w| w[0] < w[1]));
    new_edges.clear();
    new_input_edge_id_set_ids.clear();
    new_edges.reserve(g.num_edges().as_usize());
    new_input_edge_id_set_ids.reserve(g.num_edges().as_usize());
    let mut it = edges_to_discard.iter().peekable();
    for e in (0..g.num_edges().0).map(EdgeId) {
        if it.peek() == Some(&&e) {
            it.next();
        } else {
            new_edges.push(g.edge(e));
            new_input_edge_id_set_ids.push(g.input_edge_id_set_id(e));
        }
    }
    debug_assert!(it.peek().is_none());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format::{
        lax_polygon_to_string, make_lax_polygon, make_polyline, parse_point,
    };

    fn test_lax_polygon_with_options(
        input_str: &str,
        expected_str: &str,
        edge_type: EdgeType,
        degenerate_boundaries: DegenerateBoundaries,
    ) {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type,
            degenerate_boundaries,
        };
        builder.start_layer(Box::new(LaxPolygonLayer::with_options(opts)));

        let input = make_lax_polygon(input_str);
        let mut is_full = false;
        for i in 0..input.num_loops() {
            if input.num_loop_vertices(i) == 0 {
                is_full = true;
            }
        }
        builder.add_shape(&input);
        builder.add_is_full_polygon_predicate(S2Builder::is_full_polygon(is_full));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<LaxPolygonLayer>()
            .expect("wrong layer type");
        let output = layer.into_output();

        let expected = make_lax_polygon(expected_str);
        assert_eq!(
            lax_polygon_to_string(&expected),
            lax_polygon_to_string(&output),
            "edge_type={edge_type:?}, db={degenerate_boundaries:?}, input={input_str:?}"
        );
    }

    fn test_lax_polygon(input_str: &str, expected_str: &str, db: DegenerateBoundaries) {
        test_lax_polygon_with_options(input_str, expected_str, EdgeType::Directed, db);
    }

    fn test_lax_polygon_unchanged(input_str: &str, db: DegenerateBoundaries) {
        test_lax_polygon(input_str, input_str, db);
    }

    #[test]
    fn test_lax_polygon_empty() {
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            test_lax_polygon_unchanged("", db);
        }
    }

    #[test]
    fn test_lax_polygon_full() {
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            test_lax_polygon_unchanged("full", db);
        }
    }

    #[test]
    fn test_lax_polygon_one_normal_shell() {
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            test_lax_polygon_unchanged("0:0, 0:1, 1:1", db);
        }
    }

    #[test]
    fn test_lax_polygon_two_shells_one_hole() {
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            test_lax_polygon_unchanged("0:1, 1:1, 0:0; 3:3, 3:6, 6:6, 6:3; 4:4, 5:4, 5:5, 4:5", db);
        }
    }

    #[test]
    fn test_lax_polygon_partial_loop() {
        use super::super::{S2Builder, S2ErrorCode};

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(LaxPolygonLayer::new()));
        let polyline = make_polyline("0:1, 2:3, 4:5");
        builder.add_polyline(&polyline);

        let result = builder.build();
        assert!(result.is_err(), "expected build to fail");
        let err = result.unwrap_err();
        assert_eq!(err.code, S2ErrorCode::BuilderEdgesDoNotFormLoops);
    }

    #[test]
    fn test_lax_polygon_is_full_polygon_predicate_not_called() {
        use super::super::S2Builder;

        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            let mut builder = S2Builder::new(super::super::Options::default());
            let opts = Options {
                edge_type: EdgeType::Directed,
                degenerate_boundaries: db,
            };
            builder.start_layer(Box::new(LaxPolygonLayer::with_options(opts)));
            let polygon = make_lax_polygon("0:0, 0:1, 1:1");
            builder.add_shape(&polygon);
            builder.add_is_full_polygon_predicate(S2Builder::is_full_polygon_unspecified());
            let result = builder.build();
            assert!(
                result.is_ok(),
                "build failed with db={db:?}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn test_lax_polygon_all_degenerate_shells() {
        for &db in &[
            DegenerateBoundaries::Keep,
            DegenerateBoundaries::DiscardHoles,
        ] {
            test_lax_polygon_unchanged("1:1; 2:2, 3:3", db);
        }
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardShells,
        ] {
            test_lax_polygon("1:1; 2:2, 3:3", "", db);
        }
    }

    #[test]
    fn test_lax_polygon_all_degenerate_holes() {
        for &db in &[
            DegenerateBoundaries::Keep,
            DegenerateBoundaries::DiscardShells,
        ] {
            test_lax_polygon_unchanged("full; 1:1; 2:2, 3:3", db);
        }
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
        ] {
            test_lax_polygon("full; 1:1; 2:2, 3:3", "full", db);
        }
    }

    #[test]
    fn test_lax_polygon_some_degenerate_shells() {
        let normal = "0:0, 0:9, 9:0; 1:1, 7:1, 1:7";
        let input = &format!("{normal}; 3:2; 2:2, 2:3");
        test_lax_polygon_unchanged(input, DegenerateBoundaries::Keep);
        test_lax_polygon_unchanged(input, DegenerateBoundaries::DiscardHoles);
        test_lax_polygon(input, normal, DegenerateBoundaries::Discard);
        test_lax_polygon(input, normal, DegenerateBoundaries::DiscardShells);
    }

    #[test]
    fn test_lax_polygon_some_degenerate_holes() {
        for &db in &[
            DegenerateBoundaries::Keep,
            DegenerateBoundaries::DiscardShells,
        ] {
            test_lax_polygon_unchanged("0:0, 0:9, 9:0; 1:1; 2:2, 3:3", db);
        }
        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
        ] {
            test_lax_polygon("0:0, 0:9, 9:0; 1:1; 2:2, 3:3", "0:0, 0:9, 9:0", db);
        }
    }

    #[test]
    fn test_lax_polygon_normal_and_degenerate_shells_and_holes() {
        let normal = "0:0, 0:9, 9:9, 9:0; \
                      0:10, 0:19, 9:19, 9:10; 1:11, 8:11, 8:18, 1:18";
        let normal_with_degen_holes = "0:0, 0:9, 1:8, 1:7, 1:8, 0:9, 9:9, 9:0; \
             0:10, 0:19, 9:19, 9:10, 0:10, 1:11, 8:11, 8:18, 1:18, 1:11";
        let degen_shells = "0:9, 0:10; 2:12; 3:13, 3:14; 20:20; 10:0, 10:1";
        let degen_holes = "2:5; 3:6, 3:7; 8:8";
        let input = format!("{normal_with_degen_holes}; {degen_shells}; {degen_holes}");

        test_lax_polygon(&input, normal, DegenerateBoundaries::Discard);
        test_lax_polygon(
            &input,
            &format!("{normal}; {degen_shells}"),
            DegenerateBoundaries::DiscardHoles,
        );
        test_lax_polygon(
            &input,
            &format!("{normal_with_degen_holes}; {degen_holes}"),
            DegenerateBoundaries::DiscardShells,
        );
        test_lax_polygon_unchanged(&input, DegenerateBoundaries::Keep);
    }

    #[test]
    fn test_lax_polygon_layer_undirected_edges_error() {
        use super::super::{S2Builder, S2ErrorCode};

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type: EdgeType::Undirected,
            ..Options::default()
        };
        builder.start_layer(Box::new(LaxPolygonLayer::with_options(opts)));
        builder.add_edge(parse_point("0:0"), parse_point("1:1"));
        let result = builder.build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, S2ErrorCode::Unimplemented);
    }

    #[test]
    fn test_lax_polygon_invalid_polygon() {
        use crate::s2::builder::S2Builder;

        let mut builder = S2Builder::new(crate::s2::builder::Options::default());
        builder.start_layer(Box::new(LaxPolygonLayer::new()));
        builder.add_polyline(&make_polyline("0:0, 0:10, 10:0, 10:10, 0:0"));
        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_lax_polygon_duplicate_input_edges() {
        use crate::s2::builder::S2Builder;

        let opts = Options {
            degenerate_boundaries: DegenerateBoundaries::Keep,
            ..Options::default()
        };
        let mut builder = S2Builder::new(crate::s2::builder::Options::default());
        builder.start_layer(Box::new(LaxPolygonLayer::with_options(opts)));
        builder.add_shape(&make_lax_polygon("0:0, 0:5, 5:5, 5:0"));
        builder.add_point(parse_point("0:0"));
        builder.add_point(parse_point("1:1"));
        builder.add_point(parse_point("1:1"));
        builder.add_shape(&make_lax_polygon("2:2, 2:3"));
        builder.add_shape(&make_lax_polygon("2:2, 2:3"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<LaxPolygonLayer>()
            .expect("wrong layer type");
        let result = layer.into_output();
        assert_eq!(
            lax_polygon_to_string(&result),
            "0:0, 0:5, 5:5, 5:0; 1:1; 2:2, 2:3"
        );
    }

    #[test]
    fn test_lax_polygon_edge_labels() {
        use super::super::S2Builder;
        use crate::s2::shape::Shape;

        for &db in &[
            DegenerateBoundaries::Discard,
            DegenerateBoundaries::DiscardHoles,
            DegenerateBoundaries::DiscardShells,
            DegenerateBoundaries::Keep,
        ] {
            let input_str = "1:1, 1:2; 0:0, 0:9, 9:9, 9:0; 1:2, 1:1; \
                             3:3, 8:3, 8:8, 3:8; 4:4; 4:5, 5:5; 4:4";

            let mut builder = S2Builder::new(super::super::Options::default());
            let opts = Options {
                edge_type: EdgeType::Directed,
                degenerate_boundaries: db,
            };
            builder.start_layer(Box::new(LaxPolygonLayer::with_labels(opts)));

            let input = make_lax_polygon(input_str);
            let mut label = 0i32;
            for i in 0..input.num_chains() {
                let chain = input.chain(i);
                for j in 0..chain.length {
                    let edge = input.chain_edge(i, j);
                    builder.set_label(label);
                    builder.add_edge(edge.v0, edge.v1);
                    label += 1;
                }
            }
            builder.add_is_full_polygon_predicate(S2Builder::is_full_polygon(false));

            let mut layers = builder
                .build()
                .unwrap_or_else(|e| panic!("build failed with db={db:?}: {e}"));
            let layer = layers
                .remove(0)
                .into_any()
                .downcast::<LaxPolygonLayer>()
                .expect("wrong layer type");

            let out = layer.output().expect("output should be present");
            let ids = layer.label_set_ids().expect("labels should be present");
            let lex = layer
                .label_set_lexicon()
                .expect("lexicon should be present");

            assert_eq!(
                out.num_chains(),
                ids.len(),
                "db={db:?}: loop count mismatch"
            );
            for (i, loop_ids) in ids.iter().enumerate() {
                let chain = out.chain(i);
                assert_eq!(
                    chain.length,
                    loop_ids.len(),
                    "db={db:?}, loop {i}: edge count mismatch"
                );
                for (j, &edge_id) in loop_ids.iter().enumerate() {
                    let labels = lex.id_set(edge_id);
                    assert!(
                        !labels.is_empty(),
                        "db={db:?}, loop {i}, edge {j}: expected non-empty labels"
                    );
                }
            }
        }
    }
}
