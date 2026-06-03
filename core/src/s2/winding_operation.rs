// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! N-way boolean polygon operations using winding numbers.
//!
//! [`S2WindingOperation`] takes a set of possibly overlapping or
//! self-intersecting closed loops and partitions the sphere into regions of
//! constant winding number. A configurable [`WindingRule`] then selects which
//! regions to include in the output. Common rules include:
//!
//! - **Odd** — selects regions with odd winding number (equivalent to XOR).
//! - **Positive** — selects regions with winding number > 0 (equivalent to
//!   union for consistently-oriented input).
//! - **`NonZero`** — selects all regions whose winding number is not zero.
//!
//! Unlike [`S2BooleanOperation`](crate::s2::boolean_operation::S2BooleanOperation),
//! which operates on exactly two input regions, `S2WindingOperation` supports
//! any number of input loops and can handle self-intersections. It is used
//! internally by [`S2BufferOperation`](crate::s2::buffer_operation::S2BufferOperation).

#![expect(
    clippy::cast_sign_loss,
    reason = "EdgeId/VertexId (i32) used as Vec indices in winding number computation"
)]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId/VertexId (i32) <-> usize and edge counts — always small values"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId/edge counts — always in range"
)]
use std::cell::RefCell;
use std::rc::Rc;

use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::builder::get_snapped_winding_delta::{
    find_first_vertex_id, get_snapped_winding_delta,
};
use crate::s2::builder::graph::{
    DegenerateEdges, DuplicateEdges, Edge, EdgeId, EdgeType, Graph, GraphOptions, SiblingPairs,
    VertexId,
};
use crate::s2::builder::graph_shape::GraphShape;
use crate::s2::builder::layer::{IsFullPolygonPredicate, Layer};
use crate::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
use crate::s2::builder::{InputEdgeId, InputEdgeIdSetId, Options, S2Builder, S2Error};
use crate::s2::crossing_edge_query::CrossingEdgeQuery;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::angle_contains_vertex;
use crate::s2::shape_index::ShapeIndex;

/// Specifies the winding rule used to determine which regions belong to
/// the result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WindingRule {
    /// Winding number > 0 (N-way union).
    #[default]
    Positive,
    /// Winding number < 0.
    Negative,
    /// Winding number != 0.
    NonZero,
    /// Winding number is odd (N-way symmetric difference).
    Odd,
}

/// Options for `S2WindingOperation`.
#[derive(Debug)]
pub struct WindingOptions {
    snap_function: Box<dyn SnapFunction>,
    include_degeneracies: bool,
}

impl Default for WindingOptions {
    fn default() -> Self {
        WindingOptions {
            snap_function: Box::new(IdentitySnapFunction::new(Angle::default())),
            include_degeneracies: false,
        }
    }
}

impl WindingOptions {
    /// Creates default winding options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates winding options with the given snap function.
    pub fn with_snap_function(snap_function: Box<dyn SnapFunction>) -> Self {
        WindingOptions {
            snap_function,
            include_degeneracies: false,
        }
    }

    /// Returns the snap function used for snap rounding the output.
    pub fn snap_function(&self) -> &dyn SnapFunction {
        &*self.snap_function
    }

    /// Sets the snap function used for snap rounding the output.
    pub fn set_snap_function(&mut self, snap_function: Box<dyn SnapFunction>) {
        self.snap_function = snap_function;
    }

    /// Returns whether degeneracies (sibling edge pairs and isolated
    /// vertices) are included in the output.
    pub fn include_degeneracies(&self) -> bool {
        self.include_degeneracies
    }

    /// Sets whether degeneracies are included in the output.
    /// Default: false.
    pub fn set_include_degeneracies(&mut self, include_degeneracies: bool) {
        self.include_degeneracies = include_degeneracies;
    }
}

/// N-way boolean polygon operations via winding numbers.
///
/// Computes a partitioning of the sphere into regions of constant winding
/// number and returns the subset selected by the given [`WindingRule`].
///
/// # Examples
///
/// ```
/// use s2rst::s2::winding_operation::{S2WindingOperation, WindingOptions, WindingRule};
/// use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
/// use s2rst::s2::LatLng;
///
/// // Create a winding operation to compute a union via winding numbers.
/// let layer = S2PolygonLayer::new();
/// let mut op = S2WindingOperation::new(Box::new(layer), WindingOptions::new());
///
/// // Add a loop (closed polygon boundary).
/// let loop_pts: Vec<_> = vec![
///     LatLng::from_degrees(0.0, 0.0),
///     LatLng::from_degrees(0.0, 1.0),
///     LatLng::from_degrees(1.0, 1.0),
///     LatLng::from_degrees(1.0, 0.0),
/// ].into_iter().map(|ll| ll.to_point()).collect();
/// op.add_loop(&loop_pts);
///
/// // Build with a reference point outside the loop, winding = 0.
/// let ref_point = LatLng::from_degrees(45.0, 45.0).to_point();
/// op.build(ref_point, 0, WindingRule::Positive).unwrap();
/// ```
#[derive(Debug)]
pub struct S2WindingOperation {
    options: WindingOptions,
    builder: S2Builder,
    ref_input_edge_id: InputEdgeId,
    ref_winding_in: i32,
    rule: WindingRule,
    /// Shared storage for input edges (cloned before build).
    input_edges_shared: Rc<RefCell<Vec<(Point, Point)>>>,
    /// Shared storage for the rule (needed by `WindingLayer`).
    rule_shared: Rc<RefCell<WindingRule>>,
    /// Shared storage for `include_degeneracies` flag.
    include_degeneracies_shared: Rc<RefCell<bool>>,
    /// Shared storage for `ref_input_edge_id`.
    ref_input_edge_id_shared: Rc<RefCell<InputEdgeId>>,
    /// Shared storage for `ref_winding_in`.
    ref_winding_in_shared: Rc<RefCell<i32>>,
}

impl S2WindingOperation {
    /// Creates a new `S2WindingOperation` that sends output to the given layer.
    pub fn new(result_layer: Box<dyn Layer>, options: WindingOptions) -> Self {
        let mut builder_options = Options::new(options.snap_function.clone_snap());
        builder_options.split_crossing_edges = true;

        let mut builder = S2Builder::new(builder_options);

        let input_edges_shared = Rc::new(RefCell::new(Vec::new()));
        let rule_shared = Rc::new(RefCell::new(WindingRule::Positive));
        let include_degeneracies_shared = Rc::new(RefCell::new(options.include_degeneracies));
        let ref_input_edge_id_shared = Rc::new(RefCell::new(InputEdgeId(0)));
        let ref_winding_in_shared = Rc::new(RefCell::new(0i32));

        let winding_layer = WindingLayer {
            result_layer,
            input_edges: Rc::clone(&input_edges_shared),
            rule: Rc::clone(&rule_shared),
            include_degeneracies: Rc::clone(&include_degeneracies_shared),
            ref_input_edge_id: Rc::clone(&ref_input_edge_id_shared),
            ref_winding_in: Rc::clone(&ref_winding_in_shared),
            result_edges: Vec::new(),
            result_input_edge_ids: Vec::new(),
        };
        builder.start_layer(Box::new(winding_layer));

        S2WindingOperation {
            options,
            builder,
            ref_input_edge_id: InputEdgeId(0),
            ref_winding_in: 0,
            rule: WindingRule::Positive,
            input_edges_shared,
            rule_shared,
            include_degeneracies_shared,
            ref_input_edge_id_shared,
            ref_winding_in_shared,
        }
    }

    /// Adds a loop to the set of loops used to partition the sphere.
    pub fn add_loop(&mut self, loop_vertices: &[Point]) {
        self.builder.add_loop_from_points(loop_vertices);
    }

    /// Executes the operation.
    ///
    /// `ref_p` — a reference point with known winding number.
    /// `ref_winding` — the winding number at `ref_p`.
    /// `rule` — determines which regions belong to the result.
    /// # Errors
    ///
    /// Returns an error if the underlying `S2Builder::build` fails.
    pub fn build(
        &mut self,
        ref_p: Point,
        ref_winding: i32,
        rule: WindingRule,
    ) -> Result<(), S2Error> {
        // Add the reference point as a degenerate edge.
        self.ref_input_edge_id = InputEdgeId(self.builder.num_input_edges());
        self.builder.add_point(ref_p);
        self.ref_winding_in = ref_winding;
        self.rule = rule;

        // Share the parameters with the WindingLayer.
        *self.rule_shared.borrow_mut() = rule;
        *self.include_degeneracies_shared.borrow_mut() = self.options.include_degeneracies;
        *self.ref_input_edge_id_shared.borrow_mut() = self.ref_input_edge_id;
        *self.ref_winding_in_shared.borrow_mut() = self.ref_winding_in;

        // Clone input edges before build (needed by GetSnappedWindingDelta).
        let edges: Vec<(Point, Point)> = (0..self.builder.num_input_edges())
            .map(|i| self.builder.input_edge(i))
            .collect();
        *self.input_edges_shared.borrow_mut() = edges;

        self.builder.build().map(|_layers| ())
    }
}

// ─── WindingOracle ───────────────────────────────────────────────────────

/// Computes winding numbers at arbitrary points with respect to snapped loops.
struct WindingOracle {
    /// Current reference point (updated after each query).
    ref_p: Point,
    /// Winding number at current reference point.
    ref_winding: i32,
    /// Number of remaining brute-force tests before building an index.
    brute_force_tests_left: i32,
    /// Graph data for brute-force queries.
    graph_vertices: Vec<Point>,
    graph_edges: Vec<Edge>,
    /// Input edge counts per graph edge (for multiplicity).
    edge_input_counts: Vec<usize>,
    /// Shape index built lazily for fast queries.
    index: Option<ShapeIndex>,
}

impl WindingOracle {
    fn new(
        ref_input_edge_id: InputEdgeId,
        ref_winding_in: i32,
        input_edges: &[(Point, Point)],
        g: &Graph,
    ) -> Self {
        // Compute the winding number at the reference point after snapping.
        let ref_in = input_edges[ref_input_edge_id.as_usize()].0;
        let ref_v = find_first_vertex_id(ref_input_edge_id, g);
        debug_assert!(ref_v >= 0);
        let ref_p = g.vertex(ref_v);
        let mut error = S2Error::ok();
        let delta = get_snapped_winding_delta(ref_in, ref_v, None, input_edges, g, &mut error);
        debug_assert!(error.is_ok(), "GetSnappedWindingDelta error: {error}");
        let ref_winding = ref_winding_in + delta;

        // Cache graph data for brute-force and index queries.
        let graph_vertices = g.vertices().to_vec();
        let graph_edges = g.edges().to_vec();
        let edge_input_counts: Vec<usize> = (0..g.num_edges().0)
            .map(|e| g.input_edge_ids(e).len())
            .collect();

        WindingOracle {
            ref_p,
            ref_winding,
            brute_force_tests_left: 1,
            graph_vertices,
            graph_edges,
            edge_input_counts,
            index: None,
        }
    }

    fn current_ref_winding(&self) -> i32 {
        self.ref_winding
    }

    fn get_winding_number(&mut self, p: &Point) -> i32 {
        let mut crosser = EdgeCrosser::new(self.ref_p, *p);
        let mut winding = self.ref_winding;

        self.brute_force_tests_left -= 1;
        if self.brute_force_tests_left >= 0 {
            // Brute force: scan all edges.
            for (i, &(v0, v1)) in self.graph_edges.iter().enumerate() {
                let sign = crosser.signed_edge_or_vertex_crossing(
                    self.graph_vertices[v0.as_usize()],
                    self.graph_vertices[v1.as_usize()],
                );
                winding += sign * self.edge_input_counts[i] as i32;
            }
        } else {
            // Build index if needed.
            if self.index.is_none() {
                let mut index = ShapeIndex::new();
                let shape =
                    GraphShape::from_parts(self.graph_vertices.clone(), self.graph_edges.clone());
                index.add(Box::new(shape));
                index.build();
                self.index = Some(index);
            }
            let Some(index) = self.index.as_ref() else {
                return winding;
            };
            let mut query = CrossingEdgeQuery::new(index);
            let Some(shape) = index.shape(0) else {
                return winding;
            };
            let candidates = query.candidates(self.ref_p, *p, shape, 0);
            for edge_id in candidates {
                let (v0, v1) = self.graph_edges[edge_id as usize];
                let sign = crosser.signed_edge_or_vertex_crossing(
                    self.graph_vertices[v0.as_usize()],
                    self.graph_vertices[v1.as_usize()],
                );
                winding += sign * self.edge_input_counts[edge_id as usize] as i32;
            }
        }

        // Update reference for next query.
        self.ref_p = *p;
        self.ref_winding = winding;
        winding
    }
}

// ─── WindingLayer ────────────────────────────────────────────────────────

/// The layer that implements the actual winding number operation.
#[derive(Debug)]
struct WindingLayer {
    result_layer: Box<dyn Layer>,
    input_edges: Rc<RefCell<Vec<(Point, Point)>>>,
    rule: Rc<RefCell<WindingRule>>,
    include_degeneracies: Rc<RefCell<bool>>,
    ref_input_edge_id: Rc<RefCell<InputEdgeId>>,
    ref_winding_in: Rc<RefCell<i32>>,
    result_edges: Vec<Edge>,
    result_input_edge_ids: Vec<InputEdgeIdSetId>,
}

impl WindingLayer {
    fn matches_rule(rule: WindingRule, winding: i32) -> bool {
        match rule {
            WindingRule::Positive => winding > 0,
            WindingRule::Negative => winding < 0,
            WindingRule::NonZero => winding != 0,
            WindingRule::Odd => (winding & 1) != 0,
        }
    }

    fn matches_degeneracy(
        rule: WindingRule,
        include_degeneracies: bool,
        winding: i32,
        winding_minus: usize,
        winding_plus: usize,
    ) -> bool {
        if !include_degeneracies {
            return false;
        }
        if winding_minus != winding_plus {
            return false;
        }
        if rule == WindingRule::Odd {
            (winding_plus & 1) != 0
        } else {
            winding == 0
        }
    }

    /// Given an incoming edge `start` to vertex `v`, returns an edge of the
    /// loop that contains `v` (using semi-open boundary rules).
    fn get_containing_loop_edge(
        v: VertexId,
        start: EdgeId,
        g: &Graph,
        left_turn_map: &[EdgeId],
        sibling_map: &[EdgeId],
    ) -> EdgeId {
        let edge = g.edge(start);
        debug_assert_eq!(edge.1, v);
        if edge.0 == v {
            return start; // Degenerate (isolated vertex).
        }
        let mut e0 = start;
        loop {
            let e1 = left_turn_map[e0.as_usize()];
            debug_assert_eq!(g.edge(e0).1, v);
            debug_assert_eq!(g.edge(e1).0, v);
            if g.edge(e0).0 == g.edge(e1).1
                || angle_contains_vertex(
                    g.vertex(g.edge(e0).0),
                    g.vertex(v),
                    g.vertex(g.edge(e1).1),
                )
            {
                return e0;
            }
            e0 = sibling_map[e1.as_usize()];
            debug_assert_ne!(e0, start);
        }
    }

    fn compute_boundary(
        &mut self,
        g: &Graph,
        oracle: &mut WindingOracle,
        rule: WindingRule,
        include_degeneracies: bool,
        error: &mut S2Error,
    ) {
        let sibling_map = g.get_sibling_map();
        let left_turn_map = g.get_left_turn_map(&sibling_map, error);
        if !error.is_ok() {
            return;
        }

        let mut left_turn_map = left_turn_map;
        let mut edge_winding = vec![0i32; g.num_edges().as_usize()];
        let mut frontier: Vec<EdgeId> = Vec::new();

        for e_min in (0..g.num_edges().0).map(EdgeId) {
            if left_turn_map[e_min.as_usize()] < 0 {
                continue; // Already visited.
            }

            // New connected component.
            let v0 = g.edge(e_min).1;
            let e0 = Self::get_containing_loop_edge(v0, e_min, g, &left_turn_map, &sibling_map);
            edge_winding[e0.as_usize()] = oracle.get_winding_number(&g.vertex(v0));

            frontier.push(e0);
            while let Some(e) = frontier.pop() {
                if left_turn_map[e.as_usize()] < 0 {
                    continue; // Already visited.
                }

                let winding = edge_winding[e.as_usize()];
                let mut current = e;
                loop {
                    if left_turn_map[current.as_usize()] < 0 {
                        break;
                    }

                    let sibling = sibling_map[current.as_usize()];
                    let winding_minus = g.input_edge_ids(current).len();
                    let winding_plus = g.input_edge_ids(sibling).len();
                    let sibling_winding = winding - winding_minus as i32 + winding_plus as i32;

                    if (Self::matches_rule(rule, winding)
                        && !Self::matches_rule(rule, sibling_winding))
                        || Self::matches_degeneracy(
                            rule,
                            include_degeneracies,
                            winding,
                            winding_minus,
                            winding_plus,
                        )
                    {
                        self.result_edges.push(g.edge(current));
                        self.result_input_edge_ids
                            .push(g.input_edge_id_set_id(current));
                    }

                    let next = left_turn_map[current.as_usize()];
                    left_turn_map[current.as_usize()] = EdgeId(-1);

                    if left_turn_map[sibling.as_usize()] >= 0 {
                        edge_winding[sibling.as_usize()] = sibling_winding;
                        frontier.push(sibling);
                    }

                    current = next;
                    // Note: winding stays constant throughout this loop because
                    // all edges in the loop bound the same region (C++ sets it
                    // once from edge_winding[e] and never re-reads).
                }
            }
        }
    }
}

impl Layer for WindingLayer {
    fn graph_options(&self) -> GraphOptions {
        GraphOptions {
            edge_type: EdgeType::Directed,
            degenerate_edges: DegenerateEdges::Keep,
            duplicate_edges: DuplicateEdges::Keep,
            sibling_pairs: SiblingPairs::Keep,
            allow_vertex_filtering: true,
        }
    }

    fn build(&mut self, g: &Graph, error: &mut S2Error) {
        if !error.is_ok() {
            return;
        }

        let rule = *self.rule.borrow();
        let include_degeneracies = *self.include_degeneracies.borrow();
        let ref_input_edge_id = *self.ref_input_edge_id.borrow();
        let ref_winding_in = *self.ref_winding_in.borrow();

        // Clone input edges to avoid holding the Ref across mutable self borrows.
        let input_edges = self.input_edges.borrow().clone();

        // Create WindingOracle.
        let mut oracle = WindingOracle::new(ref_input_edge_id, ref_winding_in, &input_edges, g);

        // Build a new graph with the reference edge removed.
        let mut new_edges = Vec::with_capacity(g.num_edges().as_usize());
        let mut new_input_edge_ids = Vec::with_capacity(g.num_edges().as_usize());

        for e in (0..g.num_edges().0).map(EdgeId) {
            let ids = g.input_edge_ids(e);
            if !ids.is_empty() && ref_input_edge_id == ids[0] {
                continue;
            }
            new_edges.push(g.edge(e));
            new_input_edge_ids.push(g.input_edge_id_set_id(e));
        }

        // Create new graph with MERGE + CREATE for proper loop assembly.
        let new_options = GraphOptions::new(
            EdgeType::Directed,
            DegenerateEdges::DiscardExcess,
            DuplicateEdges::Merge,
            SiblingPairs::Create,
        );
        let mut new_lexicon = g.input_edge_id_set_lexicon().clone();
        let new_graph = g.make_subgraph(
            new_options,
            &mut new_edges,
            &mut new_input_edge_ids,
            &mut new_lexicon,
            None,
            error,
        );
        if !error.is_ok() {
            return;
        }

        // Compute the boundary — fills self.result_edges and self.result_input_edge_ids.
        self.result_edges.clear();
        self.result_input_edge_ids.clear();
        self.compute_boundary(&new_graph, &mut oracle, rule, include_degeneracies, error);
        if !error.is_ok() {
            return;
        }

        // Build final graph with the result layer's options.
        let oracle_winding = oracle.current_ref_winding();
        let is_full = Self::matches_rule(rule, oracle_winding);
        let is_full_predicate: IsFullPolygonPredicate =
            std::sync::Arc::new(move |_g: &Graph| Ok(is_full));

        let mut result_lexicon = new_graph.input_edge_id_set_lexicon().clone();
        let result_graph = new_graph.make_subgraph(
            self.result_layer.graph_options(),
            &mut self.result_edges,
            &mut self.result_input_edge_ids,
            &mut result_lexicon,
            Some(is_full_predicate),
            error,
        );
        if !error.is_ok() {
            return;
        }

        self.result_layer.build(&result_graph, error);
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::boolean_operation::{OpType, Options as BooleanOptions, S2BooleanOperation};
    use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
    use crate::s2::builder::snap::IdentitySnapFunction;
    use crate::s2::builder::snap::IntLatLngSnapFunction;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::shape::Shape;
    use crate::s2::shape_index::ShapeIndex;
    use crate::s2::text_format;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Runs `S2WindingOperation` and verifies the result by computing symmetric
    /// difference with expected using `S2BooleanOperation`.
    fn expect_winding_result(
        options: WindingOptions,
        loop_strs: &[&str],
        ref_point_str: &str,
        ref_winding: i32,
        rule: WindingRule,
        expected_str: &str,
    ) {
        // Build the expected result.
        let mut expected_index = ShapeIndex::new();
        let expected_polygon = text_format::make_lax_polygon(expected_str);
        expected_index.add(Box::new(expected_polygon));

        // Build the actual result.
        let actual_output = Rc::new(RefCell::new(LaxPolygon::empty()));
        let mut winding_op = S2WindingOperation::new(Box::new(LaxPolygonLayer::new()), options);

        for loop_str in loop_strs {
            if loop_str.is_empty() {
                continue;
            }
            let vertices = text_format::parse_points(loop_str);
            winding_op.add_loop(&vertices);
        }

        let ref_point = text_format::parse_point(ref_point_str);
        let result = winding_op.build(ref_point, ref_winding, rule);
        assert!(result.is_ok(), "Build failed: {:?}", result.err());

        // Verify by computing symmetric difference (should be empty).
        let mut actual_index = ShapeIndex::new();
        let actual = actual_output.borrow().clone();
        actual_index.add(Box::new(actual));

        let diff_output = Rc::new(RefCell::new(LaxPolygon::empty()));
        let mut diff_op = S2BooleanOperation::new(
            OpType::SymmetricDifference,
            Box::new(LaxPolygonLayer::new()),
            BooleanOptions::default(),
        );
        diff_op
            .build(&mut actual_index, &mut expected_index)
            .expect("Diff failed");
        assert!(
            diff_output.borrow().is_empty(),
            "Result differs from expected. Actual: {}",
            text_format::lax_polygon_to_string(&actual_output.borrow()),
        );
    }

    fn expect_degenerate_winding_result(
        mut options: WindingOptions,
        loop_strs: &[&str],
        ref_point_str: &str,
        ref_winding: i32,
        rule: WindingRule,
        expected_false: &str,
        expected_true: &str,
    ) {
        options.set_include_degeneracies(false);
        // Can't reuse options since S2WindingOperation consumes it in snap_fn clone.
        // Build with degeneracies=false.
        expect_winding_result(
            WindingOptions {
                snap_function: options.snap_function.clone_snap(),
                include_degeneracies: false,
            },
            loop_strs,
            ref_point_str,
            ref_winding,
            rule,
            expected_false,
        );
        expect_winding_result(
            WindingOptions {
                snap_function: options.snap_function.clone_snap(),
                include_degeneracies: true,
            },
            loop_strs,
            ref_point_str,
            ref_winding,
            rule,
            expected_true,
        );
    }

    #[test]
    fn test_empty() {
        expect_winding_result(
            WindingOptions::new(),
            &[""],
            "5:5",
            0,
            WindingRule::Positive,
            "",
        );
        expect_winding_result(
            WindingOptions::new(),
            &[""],
            "5:5",
            1,
            WindingRule::Positive,
            "full",
        );
    }

    #[test]
    fn test_point_loop() {
        expect_degenerate_winding_result(
            WindingOptions::new(),
            &["2:2"],
            "5:5",
            0,
            WindingRule::Positive,
            "",
            "2:2",
        );
    }

    #[test]
    fn test_sibling_pair_loop() {
        expect_degenerate_winding_result(
            WindingOptions::new(),
            &["2:2, 3:3"],
            "5:5",
            0,
            WindingRule::Positive,
            "",
            "2:2, 3:3",
        );
    }

    #[test]
    fn test_rectangle() {
        expect_winding_result(
            WindingOptions::new(),
            &["0:0, 0:10, 10:10, 10:0"],
            "5:5",
            1,
            WindingRule::Positive,
            "0:0, 0:10, 10:10, 10:0",
        );
        expect_winding_result(
            WindingOptions::new(),
            &["0:0, 0:10, 10:10, 10:0"],
            "5:5",
            1,
            WindingRule::Negative,
            "",
        );
        expect_winding_result(
            WindingOptions::new(),
            &["0:0, 0:10, 10:10, 10:0"],
            "5:5",
            1,
            WindingRule::NonZero,
            "0:0, 0:10, 10:10, 10:0",
        );
        expect_winding_result(
            WindingOptions::new(),
            &["0:0, 0:10, 10:10, 10:0"],
            "5:5",
            1,
            WindingRule::Odd,
            "0:0, 0:10, 10:10, 10:0",
        );
    }

    #[test]
    fn test_bowtie() {
        let opts = || {
            WindingOptions::with_snap_function(Box::new(IdentitySnapFunction::new(
                Angle::from_degrees(1.0),
            )))
        };
        expect_winding_result(
            opts(),
            &["5:-5, -5:5, 5:5, -5:-5"],
            "10:0",
            0,
            WindingRule::Positive,
            "0:0, -5:5, 5:5",
        );
        expect_winding_result(
            opts(),
            &["5:-5, -5:5, 5:5, -5:-5"],
            "10:0",
            0,
            WindingRule::Negative,
            "-5:-5, 0:0, 5:-5",
        );
        expect_winding_result(
            opts(),
            &["5:-5, -5:5, 5:5, -5:-5"],
            "10:0",
            0,
            WindingRule::NonZero,
            "0:0, -5:5, 5:5; -5:-5, 0:0, 5:-5",
        );
        expect_winding_result(
            opts(),
            &["5:-5, -5:5, 5:5, -5:-5"],
            "10:0",
            0,
            WindingRule::Odd,
            "0:0, -5:5, 5:5; -5:-5, 0:0, 5:-5",
        );
    }

    #[test]
    fn test_collapsing_shell() {
        let opts = || {
            WindingOptions::with_snap_function(Box::new(IdentitySnapFunction::new(
                Angle::from_degrees(5.0),
            )))
        };
        expect_degenerate_winding_result(
            opts(),
            &["0:0, 0:3, 3:3"],
            "10:0",
            0,
            WindingRule::Positive,
            "",
            "0:0",
        );
        expect_degenerate_winding_result(
            opts(),
            &["0:0, 0:3, 3:3"],
            "1:1",
            1,
            WindingRule::Positive,
            "",
            "0:0",
        );
        expect_winding_result(
            opts(),
            &["0:0, 3:3, 0:3"],
            "10:0",
            1,
            WindingRule::Positive,
            "full",
        );
        expect_winding_result(
            opts(),
            &["0:0, 3:3, 0:3"],
            "1:1",
            0,
            WindingRule::Positive,
            "full",
        );
    }

    #[test]
    fn test_touching_triangles() {
        expect_winding_result(
            WindingOptions::new(),
            &["0:0, 0:8, 8:8", "0:0, 8:8, 8:0"],
            "1:1",
            1,
            WindingRule::Positive,
            "0:0, 0:8, 8:8, 8:0",
        );
        expect_degenerate_winding_result(
            WindingOptions::new(),
            &["0:0, 0:8, 8:8", "0:0, 8:8, 8:0"],
            "2:2",
            1,
            WindingRule::Odd,
            "0:0, 0:8, 8:8, 8:0",
            "0:0, 0:8, 8:8; 0:0, 8:8, 8:0",
        );
    }

    #[test]
    fn test_touching_triangles_after_snapping() {
        let opts = || WindingOptions::with_snap_function(Box::new(IntLatLngSnapFunction::new(0)));
        expect_winding_result(
            opts(),
            &["0.1:0.2, 0:7.8, 7.6:8.2", "0.3:0.2, 8.1:7.8, 7.6:0.4"],
            "6:2",
            1,
            WindingRule::Positive,
            "0:0, 0:8, 8:8, 8:0",
        );
        expect_degenerate_winding_result(
            opts(),
            &["0.1:0.2, 0:7.8, 7.6:8.2", "0.3:0.2, 8.1:7.8, 7.6:0.4"],
            "2:6",
            1,
            WindingRule::Odd,
            "0:0, 0:8, 8:8, 8:0",
            "0:0, 0:8, 8:8; 0:0, 8:8, 8:0",
        );
    }

    #[test]
    fn test_union_of_squares() {
        let opts = || WindingOptions::with_snap_function(Box::new(IntLatLngSnapFunction::new(1)));
        let squares: &[&str] = &[
            "0:0, 0:4, 4:4, 4:0",
            "1:1, 1:5, 5:5, 5:1",
            "2:2, 2:6, 6:6, 6:2",
            "3:3, 3:7, 7:7, 7:3",
            "4:4, 4:8, 8:8, 8:4",
        ];

        // N-way union (at least 1 square).
        expect_winding_result(
            opts(),
            squares,
            "0.5:0.5",
            1,
            WindingRule::Positive,
            "7:4, 7:3, 6:3, 6:2, 5:2, 5:1, 4:1, 4:0, 0:0, 0:4, \
             1:4, 1:5, 2:5, 2:6, 3:6, 3:7, 4:7, 4:8, 8:8, 8:4",
        );

        // At least 2 squares.
        expect_winding_result(
            opts(),
            squares,
            "0.5:0.5",
            0,
            WindingRule::Positive,
            "6:4, 6:3, 5:3, 5:2, 4:2, 4:1, 1:1, 1:4, 2:4, 2:5, \
             3:5, 3:6, 4:6, 4:7, 7:7, 7:4",
        );

        // At least 3 squares.
        expect_winding_result(
            opts(),
            squares,
            "0.5:0.5",
            -1,
            WindingRule::Positive,
            "5:4, 5:3, 4:3, 4:2, 2:2, 2:4, 3:4, 3:5, 4:5, 4:6, 6:6, 6:4",
        );

        // At least 4 squares.
        expect_winding_result(
            opts(),
            squares,
            "0.5:0.5",
            -2,
            WindingRule::Positive,
            "3:3, 3:4, 4:4, 4:3; 4:4, 4:5, 5:5, 5:4",
        );
    }

    #[test]
    fn test_symmetric_difference_degeneracies() {
        let opts = || WindingOptions::with_snap_function(Box::new(IntLatLngSnapFunction::new(1)));
        expect_degenerate_winding_result(
            opts(),
            &[
                "0:0, 0:3, 3:3, 3:0",
                "1:1",
                "2:2",
                "4:4",
                // Geometry 2
                "0:0, 0:3, 3:3, 3:0",
                "1:1",
                "4:4",
                "5:5",
            ],
            "10:10",
            0,
            WindingRule::Odd,
            "",
            "2:2; 5:5",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_winding_rule_roundtrip() {
        for v in [
            WindingRule::Positive,
            WindingRule::Negative,
            WindingRule::NonZero,
            WindingRule::Odd,
        ] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<WindingRule>(&j).unwrap());
        }
    }
}
