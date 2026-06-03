// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// S2PolylineVectorLayer: assembles edges into polylines.

use crate::s2::polyline::Polyline;

use super::graph::Graph;
use super::graph::{
    DegenerateEdges, DuplicateEdges, EdgeType, GraphOptions, LabelFetcher, PolylineType,
    SiblingPairs,
};
use super::id_set_lexicon::IdSetLexicon;
use super::layer::Layer;
use super::{LabelSetId, S2Error};

/// Per-polyline label set IDs.
///
/// `label_set_ids[i][j]` gives the `LabelSetId` for edge `j` of polyline `i`.
/// Decode individual labels via the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<Vec<LabelSetId>>;

/// Options for `S2PolylineVectorLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
    /// Whether polylines are paths (no repeated vertices) or walks.
    pub polyline_type: PolylineType,
    /// How duplicate edges are handled.
    pub duplicate_edges: DuplicateEdges,
    /// How sibling pairs (edge and its reverse) are handled.
    pub sibling_pairs: SiblingPairs,
    /// If true, the layer verifies that each output polyline is valid
    /// (unit-length vertices, no adjacent identical/antipodal vertices).
    /// Any validation errors are reported via the `S2Error` passed to `build()`.
    ///
    /// C++: `S2PolylineVectorLayer::Options::validate()`
    pub validate: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            edge_type: EdgeType::Directed,
            polyline_type: PolylineType::Path,
            duplicate_edges: DuplicateEdges::Keep,
            sibling_pairs: SiblingPairs::Keep,
            validate: false,
        }
    }
}

/// A layer that assembles edges into polylines.
#[derive(Debug)]
pub struct S2PolylineVectorLayer {
    polylines: Option<Vec<Polyline>>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Legacy shared output for backward-compatible test code.
    #[cfg(test)]
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<Vec<Polyline>>>>,
}

impl S2PolylineVectorLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        S2PolylineVectorLayer {
            polylines: None,
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
        S2PolylineVectorLayer {
            polylines: None,
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
        S2PolylineVectorLayer {
            polylines: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: true,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Consumes this layer and returns the built polylines.
    ///
    /// # Panics
    ///
    /// Panics if `build()` was not called or returned an error.
    #[expect(
        clippy::expect_used,
        reason = "panics are documented; caller must call build() first"
    )]
    pub fn into_output(self) -> Vec<Polyline> {
        self.polylines
            .expect("S2PolylineVectorLayer::build() was not called")
    }

    /// Returns a reference to the built polylines.
    pub fn output(&self) -> Option<&[Polyline]> {
        self.polylines.as_deref()
    }

    /// Takes the built polylines out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<Vec<Polyline>> {
        self.polylines.take()
    }

    /// Returns the per-polyline label set IDs.
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon.
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }
}

impl Default for S2PolylineVectorLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl S2PolylineVectorLayer {
    /// Legacy constructor for test backward compatibility.
    pub fn new_legacy(output: std::rc::Rc<std::cell::RefCell<Vec<Polyline>>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<Vec<Polyline>>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_options(options);
        s.legacy_output = Some(output);
        s
    }

    /// Syncs output to legacy Rc<RefCell> if present.
    fn sync_legacy(&self) {
        if let (Some(output), Some(legacy)) = (&self.polylines, &self.legacy_output) {
            *legacy.borrow_mut() = output.clone();
        }
    }
}

impl Layer for S2PolylineVectorLayer {
    fn graph_options(&self) -> GraphOptions {
        debug_assert!(
            self.options.sibling_pairs == SiblingPairs::Keep
                || self.options.sibling_pairs == SiblingPairs::Discard,
            "S2PolylineVectorLayer only supports SiblingPairs::Keep or Discard"
        );
        GraphOptions::new(
            self.options.edge_type,
            DegenerateEdges::Discard,
            self.options.duplicate_edges,
            self.options.sibling_pairs,
        )
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        let edge_polylines = graph.get_polylines(self.options.polyline_type);
        let mut polylines = Vec::with_capacity(edge_polylines.len());

        let tracking_labels = self.track_labels;
        let mut all_label_ids: LabelSetIds = Vec::new();
        let mut lex = if tracking_labels {
            self.label_set_lexicon.take().unwrap_or_default()
        } else {
            IdSetLexicon::new()
        };
        if tracking_labels {
            all_label_ids.reserve(edge_polylines.len());
        }

        for edge_path in &edge_polylines {
            let mut vertices = Vec::with_capacity(edge_path.len() + 1);
            vertices.push(graph.vertex(graph.edge(edge_path[0]).0));
            for &eid in edge_path {
                let (_, v1) = graph.edge(eid);
                vertices.push(graph.vertex(v1));
            }
            let polyline = Polyline::new(vertices);
            if self.options.validate
                && let Some(e) = polyline.find_validation_error()
            {
                *error = e;
            }
            polylines.push(polyline);

            if tracking_labels {
                let fetcher = LabelFetcher::new(graph, self.options.edge_type);
                let mut polyline_labels = Vec::with_capacity(edge_path.len());
                for &eid in edge_path {
                    let labels = fetcher.fetch(graph, eid);
                    polyline_labels.push(lex.add_set(&labels));
                }
                all_label_ids.push(polyline_labels);
            }
        }

        self.polylines = Some(polylines);
        if tracking_labels {
            self.label_set_ids = Some(all_label_ids);
            self.label_set_lexicon = Some(lex);
        }

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
    use super::super::graph::PolylineType;
    use super::*;
    use crate::s2::text_format::{make_polyline, polyline_to_string};

    fn test_polyline_vector_with_options(
        input_strs: &[&str],
        expected_strs: &[&str],
        opts: Options,
    ) {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(S2PolylineVectorLayer::with_options(opts)));
        for &s in input_strs {
            let polyline = make_polyline(s);
            builder.add_polyline(&polyline);
        }
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineVectorLayer>()
            .expect("wrong type");
        let output = layer.into_output();

        assert_eq!(output.len(), expected_strs.len(), "polyline count mismatch");
        for (i, &expected_str) in expected_strs.iter().enumerate() {
            let expected = make_polyline(expected_str);
            assert_eq!(
                polyline_to_string(&expected),
                polyline_to_string(&output[i]),
                "polyline {i} mismatch"
            );
        }
    }

    fn test_polyline_vector(input_strs: &[&str], expected_strs: &[&str]) {
        test_polyline_vector_with_options(input_strs, expected_strs, Options::default());
    }

    #[test]
    fn test_polyline_vector_empty() {
        test_polyline_vector(&[], &[]);
    }

    #[test]
    fn test_polyline_vector_one_polyline() {
        test_polyline_vector(&["0:0, 1:1, 2:2"], &["0:0, 1:1, 2:2"]);
    }

    #[test]
    fn test_polyline_vector_two_polylines() {
        test_polyline_vector(&["0:0, 1:1", "2:2, 3:3"], &["0:0, 1:1", "2:2, 3:3"]);
    }

    #[test]
    fn test_polyline_vector_walks() {
        let opts = Options {
            polyline_type: PolylineType::Walk,
            ..Default::default()
        };
        test_polyline_vector_with_options(&["0:0, 1:1, 0:0"], &["0:0, 1:1, 0:0"], opts);
    }

    #[test]
    fn test_polyline_vector_edge_labels() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(S2PolylineVectorLayer::with_labels(
            Options::default(),
        )));
        builder.set_label(5);
        builder.add_polyline(&make_polyline("0:0, 0:1, 0:2"));
        builder.set_label(7);
        builder.add_polyline(&make_polyline("1:0, 1:1"));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineVectorLayer>()
            .expect("wrong type");
        let ids = layer.label_set_ids().expect("labels should be present");
        let lex = layer
            .label_set_lexicon()
            .expect("lexicon should be present");

        assert_eq!(ids.len(), 2, "expected 2 polylines");
        assert_eq!(ids[0].len(), 2, "polyline 0 should have 2 edges");
        assert_eq!(ids[1].len(), 1, "polyline 1 should have 1 edge");
        // First polyline edges got label 5.
        for &id in &ids[0] {
            let labels = lex.id_set(id);
            assert!(labels.contains(&5), "expected label 5");
        }
        // Second polyline edge got label 7.
        let labels = lex.id_set(ids[1][0]);
        assert!(labels.contains(&7), "expected label 7");
    }
}
