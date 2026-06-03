// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// LaxPolylineLayer: assembles graph edges into a single LaxPolyline.
//
// C++ ref: s2builderutil_lax_polyline_layer.h/cc

use crate::s2::builder::graph::{
    DegenerateEdges, DuplicateEdges, EdgeType, Graph, GraphOptions, LabelFetcher, PolylineType,
    SiblingPairs,
};
use crate::s2::builder::id_set_lexicon::IdSetLexicon;
use crate::s2::builder::layer::Layer;
use crate::s2::builder::{LabelSetId, S2Error, S2ErrorCode};
use crate::s2::lax_polyline::LaxPolyline;

/// Per-edge label set IDs for a polyline.
///
/// `label_set_ids[j]` gives the `LabelSetId` for edge `j` of the polyline
/// (the edge from vertex `j` to vertex `j+1`). Decode individual labels via
/// the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<LabelSetId>;

/// Options for `LaxPolylineLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            edge_type: EdgeType::Directed,
        }
    }
}

/// A Layer that assembles graph edges into a single `LaxPolyline`.
#[derive(Debug)]
pub struct LaxPolylineLayer {
    polyline: Option<LaxPolyline>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Legacy shared output for backward-compatible test code.
    #[cfg(test)]
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<LaxPolyline>>>,
}

impl LaxPolylineLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        LaxPolylineLayer {
            polyline: None,
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
        LaxPolylineLayer {
            polyline: None,
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
        LaxPolylineLayer {
            polyline: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: true,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Consumes this layer and returns the built polyline.
    ///
    /// # Panics
    ///
    /// Panics if `build()` was not called or returned an error.
    #[expect(
        clippy::expect_used,
        reason = "panics are documented; caller must call build() first"
    )]
    pub fn into_output(self) -> LaxPolyline {
        self.polyline
            .expect("LaxPolylineLayer::build() was not called")
    }

    /// Returns a reference to the built polyline.
    pub fn output(&self) -> Option<&LaxPolyline> {
        self.polyline.as_ref()
    }

    /// Takes the built polyline out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<LaxPolyline> {
        self.polyline.take()
    }

    /// Returns the per-edge label set IDs.
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon.
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }
}

impl Default for LaxPolylineLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl LaxPolylineLayer {
    /// Legacy constructor for test backward compatibility.
    pub fn new_legacy(output: std::rc::Rc<std::cell::RefCell<LaxPolyline>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<LaxPolyline>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_options(options);
        s.legacy_output = Some(output);
        s
    }

    /// Syncs output to legacy Rc<RefCell> if present.
    fn sync_legacy(&self) {
        if let (Some(output), Some(legacy)) = (&self.polyline, &self.legacy_output) {
            *legacy.borrow_mut() = output.clone();
        }
    }
}

impl Layer for LaxPolylineLayer {
    fn graph_options(&self) -> GraphOptions {
        GraphOptions::new(
            self.options.edge_type,
            DegenerateEdges::Keep,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        )
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        let edge_polylines = graph.get_polylines(PolylineType::Walk);

        if edge_polylines.is_empty() {
            self.polyline = Some(LaxPolyline::new(vec![]));
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        if edge_polylines.len() != 1 {
            *error = S2Error::new(
                S2ErrorCode::BuilderEdgesDoNotFormPolyline,
                format!(
                    "Input edges form {} polylines rather than 1",
                    edge_polylines.len()
                ),
            );
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        let edge_polyline = &edge_polylines[0];
        let mut vertices = Vec::with_capacity(edge_polyline.len() + 1);

        // First vertex from first edge's start.
        if let Some(&first_eid) = edge_polyline.first() {
            let (v0, _) = graph.edge(first_eid);
            vertices.push(graph.vertex(v0));
        }
        // All edge endpoints.
        for &eid in edge_polyline {
            let (_, v1) = graph.edge(eid);
            vertices.push(graph.vertex(v1));
        }

        // Collect labels if requested.
        if self.track_labels {
            let fetcher = LabelFetcher::new(graph, self.options.edge_type);
            let mut lex = self.label_set_lexicon.take().unwrap_or_default();
            let mut ids = Vec::with_capacity(edge_polyline.len());
            for &eid in edge_polyline {
                let labels = fetcher.fetch(graph, eid);
                ids.push(lex.add_set(&labels));
            }
            self.label_set_ids = Some(ids);
            self.label_set_lexicon = Some(lex);
        }

        self.polyline = Some(LaxPolyline::new(vertices));

        // Sync to legacy Rc<RefCell> output if present (test backward compat).
        #[cfg(test)]
        self.sync_legacy();
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::Point;
    use crate::s2::builder::graph::{Graph, VertexId};
    use crate::s2::builder::id_set_lexicon::IdSetLexicon;

    fn p(x: f64, y: f64, z: f64) -> Point {
        Point::from_coords(x, y, z).normalize()
    }

    fn build_graph(vertices: &[Point], edges: &[(i32, i32)], options: GraphOptions) -> Graph {
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

    fn test_lax_polyline_shape_with_options(
        input_strs: &[&str],
        expected_str: &str,
        edge_type: EdgeType,
        builder_opts: super::super::Options,
    ) {
        use super::super::S2Builder;
        use crate::s2::text_format::{lax_polyline_to_string, make_lax_polyline};

        let mut builder = S2Builder::new(builder_opts);
        let opts = Options { edge_type };
        builder.start_layer(Box::new(LaxPolylineLayer::with_options(opts)));
        for &s in input_strs {
            let polyline = make_lax_polyline(s);
            builder.add_shape(&polyline);
        }
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<LaxPolylineLayer>()
            .expect("wrong type");
        let output = layer.into_output();

        let expected = make_lax_polyline(expected_str);
        assert_eq!(
            lax_polyline_to_string(&expected),
            lax_polyline_to_string(&output),
            "edge_type={edge_type:?}, input={input_strs:?}"
        );
    }

    fn test_lax_polyline_shape(input_strs: &[&str], expected_str: &str) {
        test_lax_polyline_shape_with_options(
            input_strs,
            expected_str,
            EdgeType::Directed,
            super::super::Options::default(),
        );
    }

    fn test_lax_polyline_unchanged(input_str: &str) {
        test_lax_polyline_shape(&[input_str], input_str);
    }

    #[test]
    fn test_lax_polyline_layer_no_edges() {
        test_lax_polyline_unchanged("");
    }
    #[test]
    fn test_lax_polyline_layer_one_edge() {
        test_lax_polyline_unchanged("3:4, 1:1");
        test_lax_polyline_unchanged("1:1, 3:4");
    }
    #[test]
    fn test_lax_polyline_layer_straight_line_with_backtracking() {
        test_lax_polyline_unchanged("0:0, 1:0, 2:0, 3:0, 2:0, 1:0, 2:0, 3:0, 4:0");
    }
    #[test]
    fn test_lax_polyline_layer_simple_loop() {
        test_lax_polyline_unchanged("0:0, 0:5, 5:5, 5:0, 0:0");
    }

    #[test]
    fn test_lax_polyline_layer_direct_build_empty() {
        let mut layer = LaxPolylineLayer::new();
        let opts = layer.graph_options();
        let graph = build_graph(&[], &[], opts);
        let mut err = S2Error::ok();
        layer.build(&graph, &mut err);
        assert!(err.is_ok());
        let output = layer.into_output();
        assert_eq!(output.num_vertices(), 0);
    }

    #[test]
    fn test_lax_polyline_layer_direct_build_single_edge() {
        let a = p(1.0, 0.0, 0.0);
        let b = p(0.0, 1.0, 0.0);
        let mut layer = LaxPolylineLayer::new();
        let opts = layer.graph_options();
        let graph = build_graph(&[a, b], &[(0, 1)], opts);
        let mut err = S2Error::ok();
        layer.build(&graph, &mut err);
        assert!(err.is_ok());
        let output = layer.into_output();
        assert_eq!(output.num_vertices(), 2);
    }

    #[test]
    fn test_lax_polyline_layer_direct_build_multi_edge() {
        let a = p(1.0, 0.0, 0.0);
        let b = p(0.0, 1.0, 0.0);
        let c = p(0.0, 0.0, 1.0);
        let mut layer = LaxPolylineLayer::new();
        let opts = layer.graph_options();
        let graph = build_graph(&[a, b, c], &[(0, 1), (1, 2)], opts);
        let mut err = S2Error::ok();
        layer.build(&graph, &mut err);
        assert!(err.is_ok());
        let output = layer.into_output();
        assert_eq!(output.num_vertices(), 3);
    }

    #[test]
    fn test_lax_polyline_layer_error_on_multiple_polylines() {
        let a = p(1.0, 0.0, 0.0);
        let b = p(0.0, 1.0, 0.0);
        let c = p(0.0, 0.0, 1.0);
        let d = p(-1.0, 0.0, 0.0);
        let mut layer = LaxPolylineLayer::new();
        let opts = layer.graph_options();
        let graph = build_graph(&[a, b, c, d], &[(0, 1), (2, 3)], opts);
        let mut err = S2Error::ok();
        layer.build(&graph, &mut err);
        assert_eq!(err.code, S2ErrorCode::BuilderEdgesDoNotFormPolyline);
    }

    #[test]
    fn test_lax_polyline_layer_split_edges() {
        use super::super::snap::IntLatLngSnapFunction;
        let builder_opts = super::super::Options {
            snap_function: Box::new(IntLatLngSnapFunction::new(2)),
            ..super::super::Options::default()
        };
        test_lax_polyline_shape_with_options(
            &["0:0, 0:2, 0:1"],
            "0:0, 0:1, 0:2, 0:1",
            EdgeType::Directed,
            builder_opts,
        );
    }

    #[test]
    fn test_lax_polyline_layer_simple_edge_labels() {
        use super::super::S2Builder;
        use crate::s2::text_format::make_lax_polyline;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type: EdgeType::Undirected,
        };
        builder.start_layer(Box::new(LaxPolylineLayer::with_labels(opts)));
        builder.set_label(5);
        builder.add_shape(&make_lax_polyline("0:0, 0:1, 0:2"));
        builder.push_label(7);
        builder.add_shape(&make_lax_polyline("0:3, 0:2"));
        builder.clear_labels();
        builder.add_shape(&make_lax_polyline("0:3, 0:4, 0:5"));
        builder.set_label(11);
        builder.add_shape(&make_lax_polyline("0:6, 0:5"));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<LaxPolylineLayer>()
            .expect("wrong type");

        let expected: Vec<Vec<i32>> = vec![vec![5], vec![5], vec![5, 7], vec![], vec![], vec![11]];
        let ids = layer.label_set_ids().expect("labels should be present");
        let lex = layer
            .label_set_lexicon()
            .expect("lexicon should be present");
        assert_eq!(expected.len(), ids.len());
        for (i, exp) in expected.iter().enumerate() {
            let labels = lex.id_set(ids[i]);
            assert_eq!(exp, &labels, "edge {i}: expected {exp:?}, got {labels:?}");
        }
    }
}
