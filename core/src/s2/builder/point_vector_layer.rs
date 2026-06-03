// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// S2PointVectorLayer: extracts degenerate edges (points) from a Graph.

use crate::s2::Point;

use super::graph::Graph;
use super::graph::{
    DegenerateEdges, DuplicateEdges, EdgeId, EdgeType, GraphOptions, LabelFetcher, SiblingPairs,
};
use super::id_set_lexicon::IdSetLexicon;
use super::layer::Layer;
use super::{LabelSetId, S2Error};

/// Per-point label set IDs.
///
/// `label_set_ids[j]` gives the `LabelSetId` for point `j`.
/// Decode individual labels via the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<LabelSetId>;

/// Options for `S2PointVectorLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// How duplicate points are handled.
    pub duplicate_edges: DuplicateEdges,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            duplicate_edges: DuplicateEdges::Merge,
        }
    }
}

/// A layer that extracts degenerate edges (v0 == v1) as points.
#[derive(Debug)]
pub struct S2PointVectorLayer {
    points: Option<Vec<Point>>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Legacy shared output for backward-compatible test code.
    #[cfg(test)]
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<Vec<Point>>>>,
}

impl S2PointVectorLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        S2PointVectorLayer {
            points: None,
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
        S2PointVectorLayer {
            points: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: false,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Creates a new layer that also collects per-point label sets.
    pub fn with_labels(options: Options) -> Self {
        S2PointVectorLayer {
            points: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: true,
            #[cfg(test)]
            legacy_output: None,
        }
    }

    /// Consumes this layer and returns the built point vector.
    ///
    /// # Panics
    ///
    /// Panics if `build()` was not called or returned an error.
    #[expect(
        clippy::expect_used,
        reason = "panics are documented; caller must call build() first"
    )]
    pub fn into_output(self) -> Vec<Point> {
        self.points
            .expect("S2PointVectorLayer::build() was not called")
    }

    /// Returns a reference to the built points.
    pub fn output(&self) -> Option<&[Point]> {
        self.points.as_deref()
    }

    /// Takes the built points out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<Vec<Point>> {
        self.points.take()
    }

    /// Returns the per-point label set IDs.
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon.
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }
}

impl Default for S2PointVectorLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl S2PointVectorLayer {
    /// Legacy constructor for test backward compatibility.
    pub fn new_legacy(output: std::rc::Rc<std::cell::RefCell<Vec<Point>>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<Vec<Point>>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_options(options);
        s.legacy_output = Some(output);
        s
    }

    /// Syncs output to legacy Rc<RefCell> if present.
    fn sync_legacy(&self) {
        if let (Some(output), Some(legacy)) = (&self.points, &self.legacy_output) {
            *legacy.borrow_mut() = output.clone();
        }
    }
}

impl Layer for S2PointVectorLayer {
    fn graph_options(&self) -> GraphOptions {
        GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::Keep,
            self.options.duplicate_edges,
            SiblingPairs::Keep,
        )
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        let fetcher = LabelFetcher::new(graph, EdgeType::Directed);
        let mut points = Vec::new();
        let mut label_ids: Vec<LabelSetId> = Vec::new();
        let mut lex = if self.track_labels {
            self.label_set_lexicon.take().unwrap_or_default()
        } else {
            IdSetLexicon::new()
        };

        for eid in (0..graph.num_edges().0).map(EdgeId) {
            let (v0, v1) = graph.edge(eid);
            if v0 != v1 {
                *error = S2Error::new(
                    super::S2ErrorCode::InvalidArgument,
                    "Found non-degenerate edges".to_string(),
                );
                continue;
            }
            points.push(graph.vertex(v0));
            if self.track_labels {
                let labels = fetcher.fetch(graph, eid);
                label_ids.push(lex.add_set(&labels));
            }
        }

        self.points = Some(points);
        if self.track_labels {
            self.label_set_ids = Some(label_ids);
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
    use super::*;
    use crate::s2::text_format::parse_point;

    #[test]
    fn test_point_vector_merge_duplicates() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            duplicate_edges: DuplicateEdges::Merge,
        };
        builder.start_layer(Box::new(S2PointVectorLayer::with_options(opts)));

        builder.add_point(parse_point("0:1"));
        builder.add_point(parse_point("0:2"));
        builder.add_point(parse_point("0:1")); // duplicate
        builder.add_point(parse_point("0:4"));
        builder.add_point(parse_point("0:5"));
        builder.add_point(parse_point("0:5")); // duplicate
        builder.add_point(parse_point("0:6"));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PointVectorLayer>()
            .expect("wrong layer type");
        // Merge deduplicates: 5 unique points.
        assert_eq!(layer.into_output().len(), 5);
    }

    #[test]
    fn test_point_vector_keep_duplicates() {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            duplicate_edges: DuplicateEdges::Keep,
        };
        builder.start_layer(Box::new(S2PointVectorLayer::with_options(opts)));

        builder.add_point(parse_point("0:1"));
        builder.add_point(parse_point("0:2"));
        builder.add_point(parse_point("0:1"));
        builder.add_point(parse_point("0:4"));
        builder.add_point(parse_point("0:5"));
        builder.add_point(parse_point("0:5"));
        builder.add_point(parse_point("0:6"));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PointVectorLayer>()
            .expect("wrong layer type");
        // Keep preserves all: 7 points.
        assert_eq!(layer.into_output().len(), 7);
    }

    #[test]
    fn test_point_vector_error_non_degenerate() {
        use super::super::{S2Builder, S2ErrorCode};

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            duplicate_edges: DuplicateEdges::Keep,
        };
        builder.start_layer(Box::new(S2PointVectorLayer::with_options(opts)));

        builder.add_point(parse_point("0:1"));
        builder.add_edge(parse_point("0:3"), parse_point("0:4"));
        builder.add_point(parse_point("0:5"));

        let result = builder.build();
        assert!(result.is_err(), "expected build to fail");
        let err = result.unwrap_err();
        assert_eq!(err.code, S2ErrorCode::InvalidArgument);
    }
}

#[cfg(test)]
#[path = "point_vector_layer_tests.rs"]
mod point_vector_layer_tests;
