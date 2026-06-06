// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// S2PolygonLayer: assembles edges into an S2Polygon.

use crate::s2::{Loop, Point, Polygon};

use super::LabelSetId;
use super::S2Error;
use super::graph::{DegenerateEdges, DuplicateEdges, GraphOptions, LoopType, SiblingPairs};
use super::graph::{EdgeId, EdgeType, Graph, LabelFetcher};
use super::id_set_lexicon::IdSetLexicon;
use super::layer::Layer;

/// Per-loop label set IDs: `label_set_ids[loop_index][edge_index]` gives
/// the `LabelSetId` for that edge. Decode via the accompanying `IdSetLexicon`.
pub type LabelSetIds = Vec<Vec<LabelSetId>>;

/// Maps the heap address of a loop's vertex data to `(original_index,
/// contains_origin)`. Mirrors C++ `LoopMap` which maps `S2Loop*`.
type LoopMap = std::collections::HashMap<usize, (usize, bool)>;

/// Options for `S2PolygonLayer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// Whether edges are directed or undirected.
    pub edge_type: EdgeType,
    /// Whether to validate the output polygon.
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

/// A layer that assembles edges into an `S2Polygon`.
///
/// `S2Polygon` doesn't support degeneracies, so degenerate edges are
/// discarded and sibling pairs are removed.
///
/// # Example
///
/// ```
/// use s2rst::s2::builder::{S2Builder, Options};
/// use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
/// use s2rst::s2::text_format::parse_point;
///
/// let mut builder = S2Builder::new(Options::default());
/// builder.start_layer(Box::new(S2PolygonLayer::new()));
/// builder.add_edge(parse_point("0:0"), parse_point("0:10"));
/// builder.add_edge(parse_point("0:10"), parse_point("10:0"));
/// builder.add_edge(parse_point("10:0"), parse_point("0:0"));
/// let mut layers = builder.build().unwrap();
/// let layer = layers.remove(0)
///     .into_any()
///     .downcast::<S2PolygonLayer>()
///     .expect("wrong layer type");
/// let polygon = layer.into_output();
/// assert_eq!(polygon.num_loops(), 1);
/// ```
#[derive(Debug)]
pub struct S2PolygonLayer {
    polygon: Option<Polygon>,
    options: Options,
    label_set_ids: Option<LabelSetIds>,
    label_set_lexicon: Option<IdSetLexicon>,
    track_labels: bool,
    /// Optional shared cell the output polygon is written to on `build`. Used by
    /// convenience wrappers (e.g. buffering) that drive an operation owning this
    /// layer and need to recover its result.
    legacy_output: Option<std::rc::Rc<std::cell::RefCell<Polygon>>>,
    #[cfg(test)]
    legacy_label_set_ids: Option<std::rc::Rc<std::cell::RefCell<LabelSetIds>>>,
    #[cfg(test)]
    legacy_label_set_lexicon: Option<std::rc::Rc<std::cell::RefCell<IdSetLexicon>>>,
}

impl S2PolygonLayer {
    /// Creates a new layer with default options.
    pub fn new() -> Self {
        S2PolygonLayer {
            polygon: None,
            options: Options::default(),
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: false,
            legacy_output: None,
            #[cfg(test)]
            legacy_label_set_ids: None,
            #[cfg(test)]
            legacy_label_set_lexicon: None,
        }
    }

    /// Creates a new layer with the given options.
    pub fn with_options(options: Options) -> Self {
        S2PolygonLayer {
            polygon: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: false,
            legacy_output: None,
            #[cfg(test)]
            legacy_label_set_ids: None,
            #[cfg(test)]
            legacy_label_set_lexicon: None,
        }
    }

    /// Creates a new layer that also collects per-edge label sets.
    ///
    /// After `build()`, retrieve labels via [`label_set_ids`](Self::label_set_ids)
    /// and [`label_set_lexicon`](Self::label_set_lexicon).
    pub fn with_labels(options: Options) -> Self {
        S2PolygonLayer {
            polygon: None,
            options,
            label_set_ids: None,
            label_set_lexicon: None,
            track_labels: true,
            legacy_output: None,
            #[cfg(test)]
            legacy_label_set_ids: None,
            #[cfg(test)]
            legacy_label_set_lexicon: None,
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
    pub fn into_output(self) -> Polygon {
        self.polygon
            .expect("S2PolygonLayer::build() was not called")
    }

    /// Returns a reference to the built polygon, or `None` if `build()` was
    /// not called.
    pub fn output(&self) -> Option<&Polygon> {
        self.polygon.as_ref()
    }

    /// Takes the built polygon out, leaving `None` in its place.
    pub fn take_output(&mut self) -> Option<Polygon> {
        self.polygon.take()
    }

    /// Returns the per-loop label set IDs (if label tracking was enabled).
    pub fn label_set_ids(&self) -> Option<&LabelSetIds> {
        self.label_set_ids.as_ref()
    }

    /// Returns the label set lexicon (if label tracking was enabled).
    pub fn label_set_lexicon(&self) -> Option<&IdSetLexicon> {
        self.label_set_lexicon.as_ref()
    }

    /// Consumes and returns `(polygon, label_set_ids, label_set_lexicon)`.
    ///
    /// # Panics
    ///
    /// Panics if `build()` was not called or returned an error.
    #[expect(
        clippy::expect_used,
        reason = "panics are documented; caller must call build() first"
    )]
    pub fn into_parts(self) -> (Polygon, Option<LabelSetIds>, Option<IdSetLexicon>) {
        let polygon = self
            .polygon
            .expect("S2PolygonLayer::build() was not called");
        (polygon, self.label_set_ids, self.label_set_lexicon)
    }

    /// Collects edge labels from the graph for each loop's edges.
    fn append_edge_labels(
        &self,
        graph: &Graph,
        edge_loops: &[&Vec<EdgeId>],
        out_ids: &mut LabelSetIds,
        out_lexicon: &mut IdSetLexicon,
    ) {
        // Use the layer's edge_type for the label fetcher. For undirected
        // layers, this merges labels from sibling edges, matching C++.
        let fetcher = LabelFetcher::new(graph, self.options.edge_type);
        for edge_loop in edge_loops {
            let mut loop_ids = Vec::with_capacity(edge_loop.len());
            for &edge_id in *edge_loop {
                let labels = fetcher.fetch(graph, edge_id);
                loop_ids.push(out_lexicon.add_set(&labels));
            }
            out_ids.push(loop_ids);
        }
    }

    /// Records the heap address of each loop's vertex data, its index,
    /// and its `contains_origin` state. O(n) in the number of loops.
    ///
    /// This works because `Vec<Point>` heap pointers are stable across
    /// moves, sorts, and in-place reversal (`Loop::invert`). The Polygon
    /// construction methods (`from_loops`, `from_oriented_loops`) never
    /// reallocate individual loop vertex buffers.
    fn init_loop_map(loops: &[Loop]) -> LoopMap {
        loops
            .iter()
            .enumerate()
            .map(|(i, lp)| {
                let key = lp.vertices().as_ptr() as usize;
                (key, (i, lp.contains_origin()))
            })
            .collect()
    }

    /// Reorders label sets to match the final polygon loop order using
    /// pointer-identity lookup — O(1) per loop, O(n) total.
    ///
    /// Matches C++ `S2PolygonLayer::ReorderEdgeLabels`.
    fn reorder_edge_labels(polygon: &Polygon, loop_map: &LoopMap, label_set_ids: &mut LabelSetIds) {
        if loop_map.is_empty() || label_set_ids.is_empty() {
            return;
        }
        let mut new_ids: LabelSetIds = Vec::with_capacity(polygon.num_loops());
        for i in 0..polygon.num_loops() {
            let lp = polygon.loop_at(i);
            let key = lp.vertices().as_ptr() as usize;
            let Some(&(orig_idx, old_contains_origin)) = loop_map.get(&key) else {
                debug_assert!(
                    false,
                    "loop not found in loop_map — vertex buffer was reallocated"
                );
                continue;
            };
            let mut ids = std::mem::take(&mut label_set_ids[orig_idx]);
            if lp.contains_origin() != old_contains_origin {
                // S2Loop::Invert() reverses vertices: ABCD → DCBA
                // Edges: AB,BC,CD,DA → DC,CB,BA,AD
                // The last edge is unchanged; reverse all but the last.
                let n = ids.len();
                if n > 1 {
                    ids[..n - 1].reverse();
                }
            }
            new_ids.push(ids);
        }
        *label_set_ids = new_ids;
    }
}

impl Default for S2PolygonLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl S2PolygonLayer {
    /// Creates a layer that also writes its output polygon into the given shared
    /// cell when `build` runs. Used by convenience wrappers (e.g. buffering) that
    /// drive an operation owning the layer and need its result back.
    pub(crate) fn new_legacy(output: std::rc::Rc<std::cell::RefCell<Polygon>>) -> Self {
        let mut s = Self::new();
        s.legacy_output = Some(output);
        s
    }

    /// Syncs the built output into the shared `Rc<RefCell>` cell(s) if present.
    fn sync_legacy(&self) {
        if let (Some(output), Some(legacy)) = (&self.polygon, &self.legacy_output) {
            *legacy.borrow_mut() = output.clone();
        }
        #[cfg(test)]
        {
            if let (Some(ids), Some(legacy)) = (&self.label_set_ids, &self.legacy_label_set_ids) {
                *legacy.borrow_mut() = ids.clone();
            }
            if let (Some(lex), Some(legacy)) =
                (&self.label_set_lexicon, &self.legacy_label_set_lexicon)
            {
                *legacy.borrow_mut() = lex.clone();
            }
        }
    }
}

#[cfg(test)]
impl S2PolygonLayer {
    /// Legacy constructor with options for test backward compatibility.
    pub fn with_options_legacy(
        output: std::rc::Rc<std::cell::RefCell<Polygon>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_options(options);
        s.legacy_output = Some(output);
        s
    }

    /// Legacy constructor with labels for test backward compatibility.
    pub fn with_labels_legacy(
        output: std::rc::Rc<std::cell::RefCell<Polygon>>,
        label_set_ids: std::rc::Rc<std::cell::RefCell<LabelSetIds>>,
        label_set_lexicon: std::rc::Rc<std::cell::RefCell<IdSetLexicon>>,
        options: Options,
    ) -> Self {
        let mut s = Self::with_labels(options);
        s.legacy_output = Some(output);
        s.legacy_label_set_ids = Some(label_set_ids);
        s.legacy_label_set_lexicon = Some(label_set_lexicon);
        s
    }
}

impl Layer for S2PolygonLayer {
    fn graph_options(&self) -> GraphOptions {
        // Prevent degenerate edges and sibling edge pairs.  There should not be
        // any duplicate edges if the input is valid, but if there are then we
        // keep them since this tends to produce more comprehensible errors.
        GraphOptions::new(
            self.options.edge_type,
            DegenerateEdges::Discard,
            DuplicateEdges::Keep,
            SiblingPairs::Discard,
        )
    }

    fn build(&mut self, graph: &Graph, error: &mut S2Error) {
        let tracking_labels = self.track_labels;
        let mut label_ids: LabelSetIds = Vec::new();
        let mut label_lexicon = if tracking_labels {
            self.label_set_lexicon.take().unwrap_or_default()
        } else {
            IdSetLexicon::new()
        };

        if graph.num_edges() == 0 {
            // The polygon is either full or empty.
            match graph.is_full_polygon() {
                Ok(true) => self.polygon = Some(Polygon::full()),
                Ok(false) => self.polygon = Some(Polygon::empty()),
                Err(e) => *error = e,
            }
            if tracking_labels {
                self.label_set_ids = Some(label_ids);
                self.label_set_lexicon = Some(label_lexicon);
            }
            self.sync_legacy();
            return;
        }

        if graph.options().edge_type == EdgeType::Directed {
            let edge_loops = graph.get_directed_loops(LoopType::Simple, error);
            if !error.is_ok() {
                self.sync_legacy();
                return;
            }

            let mut all_loops = Vec::new();
            for loop_edges in &edge_loops {
                if loop_edges.len() < 3 {
                    continue;
                }
                let mut vertices: Vec<Point> = Vec::with_capacity(loop_edges.len());
                for &eid in loop_edges {
                    let (v0, _) = graph.edge(eid);
                    vertices.push(graph.vertex(v0));
                }
                all_loops.push(Loop::new(vertices));
            }
            if tracking_labels {
                let non_degen: Vec<&Vec<EdgeId>> =
                    edge_loops.iter().filter(|el| el.len() >= 3).collect();
                self.append_edge_labels(graph, &non_degen, &mut label_ids, &mut label_lexicon);
            }
            let pre_loops = Self::init_loop_map(&all_loops);
            let polygon = Polygon::from_oriented_loops(all_loops);
            if tracking_labels {
                Self::reorder_edge_labels(&polygon, &pre_loops, &mut label_ids);
            }
            self.polygon = Some(polygon);
        } else {
            let components = graph.get_undirected_components(LoopType::Simple, error);
            if !error.is_ok() {
                self.sync_legacy();
                return;
            }

            let mut all_loops = Vec::new();
            for component in &components {
                for loop_edges in &component[0] {
                    if loop_edges.len() < 3 {
                        continue;
                    }
                    let mut vertices: Vec<Point> = Vec::with_capacity(loop_edges.len());
                    for &eid in loop_edges {
                        let (v0, _) = graph.edge(eid);
                        vertices.push(graph.vertex(v0));
                    }
                    all_loops.push(Loop::new(vertices));
                }
            }
            if tracking_labels {
                let non_degen: Vec<Vec<&Vec<EdgeId>>> = components
                    .iter()
                    .map(|c| c[0].iter().filter(|el| el.len() >= 3).collect())
                    .collect();
                let flat: Vec<&Vec<EdgeId>> = non_degen.into_iter().flatten().collect();
                self.append_edge_labels(graph, &flat, &mut label_ids, &mut label_lexicon);
            }
            let pre_loops = Self::init_loop_map(&all_loops);
            for lp in &mut all_loops {
                lp.normalize();
            }
            let polygon = Polygon::from_loops(all_loops);
            if tracking_labels {
                Self::reorder_edge_labels(&polygon, &pre_loops, &mut label_ids);
            }
            self.polygon = Some(polygon);
        }

        if tracking_labels {
            self.label_set_ids = Some(label_ids);
            self.label_set_lexicon = Some(label_lexicon);
        }

        if self.options.validate
            && let Some(poly) = &self.polygon
            && let Some(e) = poly.find_validation_error()
        {
            *error = e;
        }

        // Sync to legacy Rc<RefCell> output if present (test backward compat).
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
    use crate::s2::builder::S2ErrorCode;
    use crate::s2::builder::graph::VertexId;
    use crate::s2::text_format::{make_polygon, make_polyline, polygon_to_string};

    #[test]
    fn test_polygon_layer_triangle() {
        let mut layer = S2PolygonLayer::new();

        let p0 = Point::from_coords(1.0, 0.0, 0.0);
        let p1 = Point::from_coords(0.0, 1.0, 0.0);
        let p2 = Point::from_coords(0.0, 0.0, 1.0);

        let opts = layer.graph_options();

        let mut lexicon = IdSetLexicon::new();
        let ids: Vec<_> = (0..3).map(|i| lexicon.add_set(&[i])).collect();

        let graph = Graph::new(
            opts,
            vec![p0, p1, p2],
            vec![
                (VertexId(0), VertexId(1)),
                (VertexId(1), VertexId(2)),
                (VertexId(2), VertexId(0)),
            ],
            ids,
            lexicon,
            vec![],
            IdSetLexicon::new(),
            None,
        );

        let mut err = S2Error::ok();
        layer.build(&graph, &mut err);
        assert!(err.is_ok());

        let polygon = layer.into_output();
        assert_eq!(polygon.num_loops(), 1);
        assert_eq!(polygon.num_vertices(), 3);
    }

    // ─── Full-pipeline test helpers ─────────────────────────────────────

    fn test_s2_polygon_with_edge_type(
        input_strs: &[&str],
        expected_str: &str,
        edge_type: EdgeType,
    ) {
        use super::super::S2Builder;
        use crate::s2::text_format::parse_points;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type,
            validate: false,
        };
        builder.start_layer(Box::new(S2PolygonLayer::with_options(opts)));

        // Add loops verbatim (without polygon normalization), matching C++
        // MakeVerbatimPolygonOrDie. This avoids hole inversion which would
        // change the edge directions passed to the builder.
        let mut is_full = false;
        for &s in input_strs {
            if s == "full" {
                is_full = true;
                builder.add_polygon(&Polygon::full());
            } else if s.is_empty() {
                // empty polygon — nothing to add
            } else {
                for loop_str in s.split(';') {
                    let loop_str = loop_str.trim();
                    if loop_str.is_empty() {
                        continue;
                    }
                    let vertices = parse_points(loop_str);
                    builder.add_loop_from_points(&vertices);
                }
            }
        }
        builder.add_is_full_polygon_predicate(S2Builder::is_full_polygon(is_full));

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolygonLayer>()
            .expect("wrong layer type");
        let output = layer.into_output();

        let expected = make_polygon(expected_str);
        assert_eq!(
            polygon_to_string(&expected),
            polygon_to_string(&output),
            "edge_type={edge_type:?}, input={input_strs:?}"
        );
    }

    fn test_s2_polygon(input_strs: &[&str], expected_str: &str) {
        test_s2_polygon_with_edge_type(input_strs, expected_str, EdgeType::Directed);
    }

    fn test_s2_polygon_unchanged(input_str: &str) {
        test_s2_polygon(&[input_str], input_str);
    }

    fn test_s2_polygon_error_with_edge_type(
        input_strs: &[&str],
        expected_codes: &[S2ErrorCode],
        edge_type: EdgeType,
    ) {
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type,
            validate: true,
        };
        builder.start_layer(Box::new(S2PolygonLayer::with_options(opts)));

        for &s in input_strs {
            let polyline = make_polyline(s);
            builder.add_polyline(&polyline);
        }

        let result = builder.build();
        assert!(result.is_err(), "expected build to fail");
        let err = result.unwrap_err();
        assert!(
            expected_codes.contains(&err.code),
            "expected one of {expected_codes:?}, got {:?}",
            err.code
        );
    }

    fn test_s2_polygon_error(input_strs: &[&str], expected_codes: &[S2ErrorCode]) {
        test_s2_polygon_error_with_edge_type(input_strs, expected_codes, EdgeType::Directed);
    }

    // ─── Phase 1 tests ─────────────────────────────────────────────────

    #[test]
    fn test_polygon_layer_empty() {
        test_s2_polygon_unchanged("");
    }

    #[test]
    fn test_polygon_layer_full() {
        test_s2_polygon_unchanged("full");
    }

    #[test]
    fn test_polygon_layer_small_loop() {
        test_s2_polygon_unchanged("0:0, 0:1, 1:1");
    }

    #[test]
    fn test_polygon_layer_three_loops() {
        // The second two loops are nested.
        test_s2_polygon_unchanged(
            "0:1, 1:1, 0:0; \
             3:3, 3:6, 6:6, 6:3; \
             4:4, 4:5, 5:5, 5:4",
        );
    }

    #[test]
    fn test_polygon_layer_partial_loop() {
        test_s2_polygon_error(
            &["0:1, 2:3, 4:5"],
            &[S2ErrorCode::BuilderEdgesDoNotFormLoops],
        );
    }

    #[test]
    fn test_polygon_layer_invalid_polygon() {
        test_s2_polygon_error(
            &["0:0, 0:10, 10:0, 10:10, 0:0"],
            &[
                S2ErrorCode::LoopSelfIntersection,
                S2ErrorCode::OverlappingGeometry,
            ],
        );
    }

    #[test]
    fn test_polygon_layer_duplicate_input_edges() {
        // Check that S2PolygonLayer can assemble polygons even when there are
        // duplicate edges (after sibling pairs are removed), and report error.
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type: EdgeType::Directed,
            validate: true,
        };
        builder.start_layer(Box::new(S2PolygonLayer::with_options(opts)));
        let polyline = make_polyline("0:0, 0:2, 2:2, 1:1, 0:2, 2:2, 2:0, 0:0");
        builder.add_polyline(&polyline);

        let result = builder.build();
        assert!(result.is_err(), "expected build to fail");
        let err = result.unwrap_err();
        assert!(
            err.code == S2ErrorCode::PolygonLoopsShareEdge
                || err.code == S2ErrorCode::PolygonInconsistentLoopOrientations,
            "unexpected error: {:?}",
            err.code
        );
    }

    #[test]
    fn test_polygon_layer_three_loops_into_one() {
        // Three loops (two shells and one hole) that combine into one.
        test_s2_polygon(
            &[
                "10:0, 0:0, 0:10, 5:10, 10:10, 10:5",
                "0:10, 0:15, 5:15, 5:10",
                "10:10, 5:10, 5:5, 10:5",
            ],
            "10:5, 10:0, 0:0, 0:10, 0:15, 5:15, 5:10, 5:5",
        );
    }

    #[test]
    fn test_polygon_layer_triangle_pyramid() {
        // A big CCW triangle containing 3 CW triangular holes.  The whole thing
        // looks like a pyramid of nine triangles.  The output consists of 6
        // positive triangles with no holes.
        test_s2_polygon(
            &[
                "0:0, 0:2, 0:4, 0:6, 1:5, 2:4, 3:3, 2:2, 1:1",
                "0:2, 1:1, 1:3",
                "0:4, 1:3, 1:5",
                "1:3, 2:2, 2:4",
            ],
            "0:4, 0:6, 1:5; 2:4, 3:3, 2:2; 2:2, 1:1, 1:3; \
             1:1, 0:0, 0:2; 1:3, 0:2, 0:4; 1:3, 1:5, 2:4",
        );
    }

    #[test]
    fn test_polygon_layer_complex_nesting() {
        // A complex set of nested polygons, with the loops in random order.
        test_s2_polygon_unchanged(
            "47:15, 47:5, 5:5, 5:15; \
             35:12, 35:7, 27:7, 27:12; \
             1:50, 50:50, 50:1, 1:1; \
             42:22, 10:22, 10:25, 42:25; \
             47:30, 47:17, 5:17, 5:30; \
             7:27, 45:27, 45:20, 7:20; \
             37:7, 37:12, 45:12, 45:7; \
             47:47, 47:32, 5:32, 5:47; \
             50:60, 50:55, 1:55, 1:60; \
             25:7, 17:7, 17:12, 25:12; \
             7:7, 7:12, 15:12, 15:7",
        );
    }

    #[test]
    fn test_polygon_layer_five_loops_touching() {
        // Five nested loops that touch at one common point.
        test_s2_polygon_unchanged(
            "0:0, 0:10, 10:10, 10:0; \
             0:0, 1:9, 9:9, 9:1; \
             0:0, 2:8, 8:8, 8:2; \
             0:0, 3:7, 7:7, 7:3; \
             0:0, 4:6, 6:6, 6:4",
        );
    }

    #[test]
    fn test_polygon_layer_four_nested_diamonds() {
        // Four diamonds nested inside each other, where each diamond shares two
        // vertices with the diamond inside it.
        test_s2_polygon(
            &[
                "0:10, -10:0, 0:-10, 10:0",
                "0:-20, -10:0, 0:20, 10:0",
                "0:-10, -5:0, 0:10, 5:0",
                "0:5, -5:0, 0:-5, 5:0",
            ],
            "10:0, 0:10, -10:0, 0:20; \
             0:-20, -10:0, 0:-10, 10:0; \
             5:0, 0:-10, -5:0, 0:-5; \
             0:5, -5:0, 0:10, 5:0",
        );
    }

    // ─── Batch 6: Label tests (from C++) ────────────────────────────────

    /// Adds a polyline's edges with labels, optionally reversing alternating
    /// edges for undirected mode. Returns a map of edge keys to label sets.
    fn add_polyline_with_labels(
        polyline: &crate::s2::polyline::Polyline,
        edge_type: EdgeType,
        label_begin: i32,
        builder: &mut super::super::S2Builder,
    ) -> std::collections::HashMap<[u64; 3], std::collections::BTreeSet<i32>> {
        use std::collections::{BTreeSet, HashMap};

        let mut edge_label_map: HashMap<[u64; 3], BTreeSet<i32>> = HashMap::new();
        for i in 0..polyline.num_vertices() - 1 {
            let label = label_begin + i as i32;
            builder.set_label(label);
            // With undirected edges, reverse direction of every other edge.
            let dir = if edge_type == EdgeType::Directed {
                1
            } else {
                i & 1
            };
            let v0 = polyline.vertex(i + (1 - dir));
            let v1 = polyline.vertex(i + dir);
            builder.add_edge(v0, v1);
            // Key: sum of endpoint bit patterns (invariant under swap).
            let key = edge_key(polyline.vertex(i), polyline.vertex(i + 1));
            edge_label_map.entry(key).or_default().insert(label);
        }
        edge_label_map
    }

    /// Creates a symmetric key for an edge (invariant under endpoint swap).
    /// Uses the actual Point sum, matching C++ which uses `S2Point key = v0 + v1`.
    fn edge_key(a: Point, b: Point) -> [u64; 3] {
        let sum = Point::from_coords(a.x() + b.x(), a.y() + b.y(), a.z() + b.z());
        [sum.x().to_bits(), sum.y().to_bits(), sum.z().to_bits()]
    }

    fn test_edge_labels(edge_type: EdgeType) {
        // C++: S2PolygonLayer::TestEdgeLabels
        use super::super::S2Builder;

        let mut builder = S2Builder::new(super::super::Options::default());
        let opts = Options {
            edge_type,
            validate: false,
        };
        builder.start_layer(Box::new(S2PolygonLayer::with_labels(opts)));

        // A polygon with 3 loops: outer 4-gon, inner triangle, inner triangle.
        // The loops are reordered and some inverted during S2Polygon construction.
        let polyline = make_polyline("0:0, 9:1, 1:9, 0:0, 2:8, 8:2, 0:0, 0:10, 10:10, 10:0, 0:0");
        let edge_label_map = add_polyline_with_labels(&polyline, edge_type, 0, &mut builder);

        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolygonLayer>()
            .expect("wrong layer type");
        let (polygon, label_ids_opt, label_lex_opt) = layer.into_parts();
        let ids = label_ids_opt.expect("labels should be present");
        let lex = label_lex_opt.expect("lexicon should be present");

        let expected_loop_sizes = [4, 3, 3];
        assert_eq!(
            expected_loop_sizes.len(),
            ids.len(),
            "wrong number of loops: expected {}, got {}",
            expected_loop_sizes.len(),
            ids.len()
        );

        for (i, loop_ids) in ids.iter().enumerate() {
            assert_eq!(
                expected_loop_sizes[i],
                loop_ids.len(),
                "loop {i}: wrong edge count"
            );
            for (j, &edge_id) in loop_ids.iter().enumerate() {
                let key = edge_key(
                    polygon.loop_at(i).vertex(j),
                    polygon.loop_at(i).vertex(j + 1),
                );
                let expected_labels = edge_label_map.get(&key).cloned().unwrap_or_default();
                let actual_labels: std::collections::BTreeSet<i32> =
                    lex.id_set(edge_id).into_iter().collect();
                assert_eq!(
                    expected_labels, actual_labels,
                    "loop {i} edge {j}: labels mismatch (edge_type={edge_type:?})"
                );
            }
        }
    }

    #[test]
    fn test_polygon_layer_directed_edge_labels() {
        // C++: S2PolygonLayer::DirectedEdgeLabels
        test_edge_labels(EdgeType::Directed);
    }

    #[test]
    fn test_polygon_layer_undirected_edge_labels() {
        // C++: S2PolygonLayer::UndirectedEdgeLabels
        test_edge_labels(EdgeType::Undirected);
    }

    #[test]
    fn test_polygon_layer_labels_requested_but_not_provided() {
        // C++: S2PolygonLayer::LabelsRequestedButNotProvided
        // Labels requested but none added → all edges get empty label sets.
        use super::super::S2Builder;
        use super::super::id_set_lexicon::EMPTY_SET_ID;

        let mut builder = S2Builder::new(super::super::Options::default());
        builder.start_layer(Box::new(S2PolygonLayer::with_labels(Options::default())));
        builder.add_polyline(&make_polyline("0:0, 0:1, 1:0, 0:0"));
        let mut layers = builder.build().expect("build failed");
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolygonLayer>()
            .expect("wrong layer type");
        let (_polygon, label_ids_opt, _lex) = layer.into_parts();
        let ids = label_ids_opt.expect("labels should be present");
        assert_eq!(ids.len(), 1, "expected 1 loop");
        assert_eq!(ids[0].len(), 3, "expected 3 edges");
        for &label_set_id in &ids[0] {
            assert_eq!(label_set_id, EMPTY_SET_ID, "expected empty label set");
        }
    }

    #[test]
    fn test_polygon_layer_seven_diamonds_touching() {
        // Seven diamonds nested within each other touching at one
        // point between each nested pair.
        // C++: S2PolygonLayer::SevenDiamondsTouchingAtOnePointPerPair
        test_s2_polygon_unchanged(
            "0:-70, -70:0, 0:70, 70:0; \
             0:-70, -60:0, 0:60, 60:0; \
             0:-50, -60:0, 0:50, 50:0; \
             0:-40, -40:0, 0:50, 40:0; \
             0:-30, -30:0, 0:30, 40:0; \
             0:-20, -20:0, 0:30, 20:0; \
             0:-10, -20:0, 0:10, 10:0",
        );
    }
}
