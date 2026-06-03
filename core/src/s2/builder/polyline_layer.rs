// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// S2PolylineLayer: assembles edges into a single S2Polyline.

use crate::s2::polyline::Polyline;

use super::graph::Graph;
use super::graph::{
    DegenerateEdges, DuplicateEdges, EdgeType, GraphOptions, LabelFetcher, PolylineType,
    SiblingPairs,
};
use super::id_set_lexicon::IdSetLexicon;
use super::layer::Layer;
use super::{LabelSetId, S2Error};

/// Per-edge label set IDs for a polyline.
///
/// `label_set_ids[j]` gives the `LabelSetId` for edge `j` of the polyline
/// (the edge from vertex `j` to vertex `j+1`). Decode individual labels via
/// the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<LabelSetId>;

/// Options for `S2PolylineLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
    /// If true, the layer verifies that the output polyline is valid
    /// (unit-length vertices, no adjacent identical/antipodal vertices).
    /// Any validation errors are reported via the `S2Error` passed to `build()`.
    ///
    /// C++: `S2PolylineLayer::Options::validate()`
    pub validate: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            edge_type: EdgeType::Directed,
            validate: false,
        }
    }
}

/// A layer that assembles edges into a single polyline.
///
/// If the edges cannot be assembled into a single connected polyline,
/// only the first polyline is kept.
#[derive(Debug)]
pub struct S2PolylineLayer {
    polyline: Option<Polyline>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Legacy shared output for backward-compatible test code.
    #[cfg(test)]
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<Polyline>>>,
}

impl S2PolylineLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        S2PolylineLayer {
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
        S2PolylineLayer {
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
        S2PolylineLayer {
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
    pub fn into_output(self) -> Polyline {
        self.polyline
            .expect("S2PolylineLayer::build() was not called")
    }

    /// Returns a reference to the built polyline.
    pub fn output(&self) -> Option<&Polyline> {
        self.polyline.as_ref()
    }

    /// Takes the built polyline out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<Polyline> {
        self.polyline.take()
    }

    /// Returns the per-edge label set IDs (if label tracking was enabled).
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon (if label tracking was enabled).
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }
}

impl Default for S2PolylineLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl S2PolylineLayer {
    /// Legacy constructor for test backward compatibility.
    pub fn new_legacy(output: std::rc::Rc<std::cell::RefCell<Polyline>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<Polyline>>,
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

impl Layer for S2PolylineLayer {
    fn graph_options(&self) -> GraphOptions {
        GraphOptions::new(
            self.options.edge_type,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Keep,
        )
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        if graph.num_edges() == 0 {
            self.polyline = Some(Polyline::new(vec![]));
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        let edge_polylines = graph.get_polylines(PolylineType::Walk);

        if edge_polylines.len() != 1 {
            *error = S2Error {
                code: super::S2ErrorCode::BuilderEdgesDoNotFormPolyline,
                message: "Input edges cannot be assembled into polyline".to_string(),
            };
            #[cfg(test)]
            self.sync_legacy();
            return;
        }

        let edge_path = &edge_polylines[0];
        let mut vertices = Vec::with_capacity(edge_path.len() + 1);
        vertices.push(graph.vertex(graph.edge(edge_path[0]).0));
        for &eid in edge_path {
            let (_, v1) = graph.edge(eid);
            vertices.push(graph.vertex(v1));
        }

        // Collect labels if requested.
        if self.track_labels {
            let fetcher = LabelFetcher::new(graph, self.options.edge_type);
            let mut lex = self.label_set_lexicon.take().unwrap_or_default();
            let mut ids = Vec::with_capacity(edge_path.len());
            for &eid in edge_path {
                let labels = fetcher.fetch(graph, eid);
                ids.push(lex.add_set(&labels));
            }
            self.label_set_ids = Some(ids);
            self.label_set_lexicon = Some(lex);
        }

        let polyline = Polyline::new(vertices);
        if self.options.validate
            && let Some(e) = polyline.find_validation_error()
        {
            *error = e;
        }
        self.polyline = Some(polyline);

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
    use crate::s2::text_format::{make_polyline, polyline_to_string};

    fn test_s2_polyline_with_edge_type(
        input_strs: &[&str],
        expected_str: &str,
        edge_type: EdgeType,
        builder_opts: super::super::Options,
    ) {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(builder_opts);
        let opts = Options {
            edge_type,
            ..Options::default()
        };
        builder.start_layer(Box::new(S2PolylineLayer::with_options(opts)));

        for &s in input_strs {
            let polyline = make_polyline(s);
            builder.add_polyline(&polyline);
        }

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineLayer>()
            .expect("wrong layer type");
        let output = layer.into_output();

        let expected = make_polyline(expected_str);
        assert_eq!(
            polyline_to_string(&expected),
            polyline_to_string(&output),
            "edge_type={edge_type:?}, input={input_strs:?}"
        );
    }

    fn test_s2_polyline(input_strs: &[&str], expected_str: &str) {
        test_s2_polyline_with_edge_type(
            input_strs,
            expected_str,
            EdgeType::Directed,
            super::super::Options::default(),
        );
    }

    fn test_s2_polyline_unchanged(input_str: &str) {
        test_s2_polyline(&[input_str], input_str);
    }

    // ─── Tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_polyline_layer_no_edges() {
        test_s2_polyline_unchanged("");
    }

    #[test]
    fn test_polyline_layer_one_edge() {
        test_s2_polyline_unchanged("3:4, 1:1");
        test_s2_polyline_unchanged("1:1, 3:4");
    }

    #[test]
    fn test_polyline_layer_straight_line_with_backtracking() {
        test_s2_polyline_unchanged("0:0, 1:0, 2:0, 3:0, 2:0, 1:0, 2:0, 3:0, 4:0");
    }

    #[test]
    fn test_polyline_layer_early_walk_termination_1() {
        use super::super::snap::IntLatLngSnapFunction;

        let builder_opts = super::super::Options {
            snap_function: Box::new(IntLatLngSnapFunction::new(2)),
            ..super::super::Options::default()
        };
        test_s2_polyline_with_edge_type(
            &["0:0, 0:2, 0:1"],
            "0:0, 0:1, 0:2, 0:1",
            EdgeType::Directed,
            builder_opts,
        );
    }

    #[test]
    fn test_polyline_layer_early_walk_termination_2() {
        test_s2_polyline(&["0:0, 0:1", "0:2, 0:1", "0:1, 0:2"], "0:0, 0:1, 0:2, 0:1");
    }

    #[test]
    fn test_polyline_layer_simple_loop() {
        test_s2_polyline_unchanged("0:0, 0:5, 5:5, 5:0, 0:0");
    }

    #[test]
    fn test_polyline_layer_many_loops() {
        test_s2_polyline_unchanged(
            "0:0, 2:2, 2:4, 2:2, 2:4, 4:4, 4:2, 2:2, 4:4, 4:2, 2:2, 2:0, \
             2:2, 2:0, 4:0, 2:2, 4:2, 2:2, 0:2, 0:4, 2:2, 0:4, 0:2, 2:2, \
             0:4, 2:2, 0:2, 2:2, 0:0, 0:2, 2:2, 0:0",
        );
    }

    #[test]
    fn test_polyline_layer_unordered_loops() {
        test_s2_polyline(
            &[
                "3:3, 3:2, 2:2, 2:3, 3:3",
                "1:0, 0:0, 0:1, 1:1, 1:0",
                "3:1, 3:0, 2:0, 2:1, 3:1",
                "1:3, 1:2, 0:2, 0:1, 1:3",
                "1:1, 1:2, 2:2, 2:1, 1:1",
            ],
            "3:3, 3:2, 2:2, 2:1, 3:1, 3:0, 2:0, 2:1, 1:1, 1:0, 0:0, 0:1, \
             1:1, 1:2, 0:2, 0:1, 1:3, 1:2, 2:2, 2:3, 3:3",
        );
    }

    #[test]
    fn test_polyline_layer_split_edges() {
        use super::super::snap::IntLatLngSnapFunction;

        let builder_opts = super::super::Options {
            snap_function: Box::new(IntLatLngSnapFunction::new(7)),
            split_crossing_edges: true,
            ..super::super::Options::default()
        };
        test_s2_polyline_with_edge_type(
            &["0:10, 0:0, 1:0, -1:2, 1:4, -1:6, 1:8, -1:10, -5:0, 0:0, 0:10"],
            "0:10, 0:9, 0:7, 0:5, 0:3, 0:1, 0:0, 1:0, 0:1, -1:2, 0:3, 1:4, \
             0:5, -1:6, 0:7, 1:8, 0:9, -1:10, -5:0, 0:0, 0:1, 0:3, 0:5, 0:7, \
             0:9, 0:10",
            EdgeType::Directed,
            builder_opts,
        );
    }

    #[test]
    fn test_polyline_layer_validate_ok() {
        // A valid polyline should not produce a validation error.
        let mut builder = super::super::S2Builder::new(super::super::Options::default());
        let opts = Options {
            validate: true,
            ..Options::default()
        };
        builder.start_layer(Box::new(S2PolylineLayer::with_options(opts)));
        builder.add_polyline(&make_polyline("0:0, 1:1, 2:2"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineLayer>()
            .expect("wrong layer type");
        let output = layer.into_output();
        assert_eq!(
            polyline_to_string(&output),
            polyline_to_string(&make_polyline("0:0, 1:1, 2:2"))
        );
    }

    #[test]
    fn test_polyline_layer_invalid_polyline() {
        // Antipodal vertices with validate: true should fail.
        // C++: S2PolylineLayer::InvalidPolyline
        let mut builder = super::super::S2Builder::new(super::super::Options::default());
        let opts = Options {
            validate: true,
            ..Options::default()
        };
        builder.start_layer(Box::new(S2PolylineLayer::with_options(opts)));
        let p0 = Point::from_coords(1.0, 0.0, 0.0);
        let p1 = Point::from_coords(-1.0, 0.0, 0.0);
        builder.add_edge(p0, p1);
        let result = builder.build();
        assert!(
            result.is_err(),
            "expected build to fail for antipodal vertices"
        );
        let err = result.unwrap_err();
        assert_eq!(err.code, super::super::S2ErrorCode::AntipodalVertices);
    }

    #[test]
    fn test_polyline_layer_simple_edge_labels() {
        // C++: S2PolylineLayer::SimpleEdgeLabels
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type: EdgeType::Undirected,
            ..Options::default()
        };
        builder.start_layer(Box::new(S2PolylineLayer::with_labels(opts)));
        builder.set_label(5);
        builder.add_polyline(&make_polyline("0:0, 0:1, 0:2"));
        builder.push_label(7);
        builder.add_polyline(&make_polyline("0:3, 0:2"));
        builder.clear_labels();
        builder.add_polyline(&make_polyline("0:3, 0:4, 0:5"));
        builder.set_label(11);
        builder.add_polyline(&make_polyline("0:6, 0:5"));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineLayer>()
            .expect("wrong layer type");

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
