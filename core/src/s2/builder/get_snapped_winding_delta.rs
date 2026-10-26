// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

// GetSnappedWindingDelta: computes the change in winding number of a reference
// vertex due to snap rounding.
//
// C++ ref: s2builderutil_get_snapped_winding_delta.h/cc

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::s2::Point;
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_distances;

use super::S2Error;
use super::S2ErrorCode;
use super::graph::{EdgeId, Graph, VertexId};

use super::InputEdgeId;

/// The winding number returned when a usage error is detected.
const ERROR_RESULT: i32 = i32::MAX;

/// An input edge may snap to zero, one, or two non-degenerate output edges
/// incident to the reference vertex, consisting of at most one incoming and
/// one outgoing edge.
#[derive(Clone, Debug)]
struct EdgeSnap {
    /// The original input edge endpoints.
    input: (Point, Point),
    /// If >= 0, the source vertex of an incoming edge to the reference vertex.
    v_in: VertexId,
    /// If >= 0, the destination vertex of an outgoing edge from the reference vertex.
    v_out: VertexId,
}

impl EdgeSnap {
    fn new() -> Self {
        EdgeSnap {
            input: (Point::default(), Point::default()),
            v_in: VertexId(-1),
            v_out: VertexId(-1),
        }
    }
}

/// A map that allows finding all the input edges that start at a given point.
type InputVertexEdgeMap = BTreeMap<OrderedPoint, Vec<EdgeSnap>>;

/// Wrapper for Point that provides Ord for use in `BTreeMap`.
#[derive(Clone, Debug)]
struct OrderedPoint(Point);

impl PartialEq for OrderedPoint {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for OrderedPoint {}

impl PartialOrd for OrderedPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp_point(other.0)
    }
}

/// Assembles incident edges into an edge chain. Returns true on success.
fn build_chain(
    ref_v: VertexId,
    g: &Graph,
    input_vertex_edge_map: &mut InputVertexEdgeMap,
    chain_in: &mut Vec<Point>,
    chain_out: &mut Vec<Point>,
    error: &mut S2Error,
) -> bool {
    debug_assert!(chain_in.is_empty());
    debug_assert!(chain_out.is_empty());

    // First look for an incoming edge to the reference vertex.
    let mut start_key: Option<OrderedPoint> = None;
    let mut start_idx: Option<usize> = None;

    for (key, snaps) in input_vertex_edge_map.iter() {
        for (i, snap) in snaps.iter().enumerate() {
            if snap.v_in >= 0 {
                start_key = Some(key.clone());
                start_idx = Some(i);
                break;
            }
        }
        if start_key.is_some() {
            break;
        }
    }

    let snap = if let (Some(key), Some(idx)) = (start_key, start_idx) {
        let Some(snaps) = input_vertex_edge_map.get_mut(&key) else {
            return false;
        };
        let snap = snaps.remove(idx);
        if snaps.is_empty() {
            input_vertex_edge_map.remove(&key);
        }
        chain_out.push(g.vertex(snap.v_in));
        snap
    } else {
        // Pick an arbitrary edge to start a closed loop.
        let Some(first_key) = input_vertex_edge_map.keys().next().cloned() else {
            return false;
        };
        let Some(snaps) = input_vertex_edge_map.get_mut(&first_key) else {
            return false;
        };
        let snap = snaps.remove(0);
        if snaps.is_empty() {
            input_vertex_edge_map.remove(&first_key);
        }
        snap
    };

    chain_in.push(snap.input.0);
    chain_in.push(snap.input.1);
    chain_out.push(g.vertex(ref_v));
    if snap.v_out >= 0 {
        chain_out.push(g.vertex(snap.v_out));
        return true;
    }

    // Repeatedly add edges until the chain or loop is finished.
    while chain_in[chain_in.len() - 1] != chain_in[0] {
        let key = OrderedPoint(chain_in[chain_in.len() - 1]);
        let Some(snaps) = input_vertex_edge_map
            .get_mut(&key)
            .filter(|s| !s.is_empty())
        else {
            *error = S2Error::new(
                S2ErrorCode::InvalidArgument,
                "Input edges (after filtering) do not form loops",
            );
            return false;
        };
        let snap = snaps.remove(0);
        if snaps.is_empty() {
            input_vertex_edge_map.remove(&key);
        }
        chain_in.push(snap.input.1);
        if snap.v_out >= 0 {
            chain_out.push(g.vertex(snap.v_out));
            break;
        }
    }
    true
}

/// Returns the change in winding number along the edge AB with respect to
/// the given edge chain.
fn get_edge_winding_delta(a: &Point, b: &Point, chain: &[Point]) -> i32 {
    debug_assert!(!chain.is_empty());
    let mut delta = 0;
    let mut crosser = EdgeCrosser::new(*a, *b);
    crosser.restart_at(chain[0]);
    for &vertex in &chain[1..] {
        delta += crosser.signed_edge_or_vertex_chain_crossing(vertex);
    }
    delta
}

/// Returns a connecting vertex for an input edge that snaps to a long chain.
fn get_connector(b0: Point, b1: Point, b1_snapped: Point) -> Point {
    if b1_snapped.0.dot(b1.0) >= 0.0 {
        return b1;
    }
    let x = Point(b0.point_cross(b1).0.cross(b1_snapped.0).normalize());
    if x.0.dot(edge_distances::interpolate(0.5, b0, b1).0) >= 0.0 {
        x
    } else {
        Point(-x.0)
    }
}

/// Returns incident edges for a vertex by brute-force scan.
fn get_incident_edges_brute_force(v: VertexId, g: &Graph) -> Vec<EdgeId> {
    let mut result = Vec::new();
    for e in (0..g.num_edges().0).map(EdgeId) {
        let edge = g.edge(e);
        if edge.0 == v || edge.1 == v {
            result.push(e);
        }
    }
    result
}

/// Computes the change in winding number of a reference vertex due to snap
/// rounding.
///
/// `ref_in` — reference vertex position before snapping.
/// `ref_v` — `VertexId` of the reference vertex after snapping.
/// `input_edge_filter` — returns true for edges to skip (None = use all).
/// `input_edges` — builder input edges (cloned before build).
/// `g` — the `S2Builder` output graph.
/// `error` — set on failure.
pub(crate) fn get_snapped_winding_delta(
    ref_in: Point,
    ref_v: VertexId,
    input_edge_filter: Option<&dyn Fn(InputEdgeId) -> bool>,
    input_edges: &[(Point, Point)],
    g: &Graph,
    error: &mut S2Error,
) -> i32 {
    let incident_edges = get_incident_edges_brute_force(ref_v, g);
    get_snapped_winding_delta_with_incident(
        ref_in,
        ref_v,
        &incident_edges,
        input_edge_filter,
        input_edges,
        g,
        error,
    )
}

/// Like `get_snapped_winding_delta`, but takes pre-computed incident edges.
pub(crate) fn get_snapped_winding_delta_with_incident(
    ref_in: Point,
    ref_v: VertexId,
    incident_edges: &[EdgeId],
    input_edge_filter: Option<&dyn Fn(InputEdgeId) -> bool>,
    input_edges: &[(Point, Point)],
    g: &Graph,
    error: &mut S2Error,
) -> i32 {
    // Group incident edges by input edge id.
    let mut input_id_edge_map: HashMap<InputEdgeId, EdgeSnap> = HashMap::new();
    for &e in incident_edges {
        let edge = g.edge(e);
        for input_id_raw in g.input_edge_ids(e) {
            let input_id = InputEdgeId(input_id_raw);
            if let Some(ref filter) = input_edge_filter
                && filter(input_id)
            {
                continue;
            }
            let snap = input_id_edge_map.entry(input_id).or_insert_with(|| {
                let ie = input_edges[input_id.as_usize()];
                let mut s = EdgeSnap::new();
                s.input = ie;
                s
            });
            if edge.0 != ref_v {
                snap.v_in = edge.0;
            }
            if edge.1 != ref_v {
                snap.v_out = edge.1;
            }
        }
    }

    // Regroup by starting vertex of each input edge.
    let mut input_vertex_edge_map: InputVertexEdgeMap = BTreeMap::new();
    for snap in input_id_edge_map.values() {
        input_vertex_edge_map
            .entry(OrderedPoint(snap.input.0))
            .or_default()
            .push(snap.clone());
    }

    let ref_out = g.vertex(ref_v);

    let mut winding_delta = 0i32;
    while !input_vertex_edge_map.is_empty() {
        let mut chain_in = Vec::new();
        let mut chain_out = Vec::new();
        if !build_chain(
            ref_v,
            g,
            &mut input_vertex_edge_map,
            &mut chain_in,
            &mut chain_out,
            error,
        ) {
            return ERROR_RESULT;
        }

        if chain_out.len() == 1 {
            // Closed chain: all vertices snap to R'.
            debug_assert_eq!(chain_out[0], ref_out);
            debug_assert_eq!(chain_in[0], chain_in[chain_in.len() - 1]);
            let z = crate::s2::ortho(ref_out);
            winding_delta += 0 - get_edge_winding_delta(&z, &ref_in, &chain_in);
        } else {
            // Open chain: C = (A0, ..., B1) snaps to C' = (A0', R', B1')
            debug_assert_eq!(chain_out.len(), 3);
            debug_assert_eq!(chain_out[1], ref_out);

            let mut za = Point(chain_in[0].point_cross(chain_in[1]).0.normalize());
            let n = chain_in.len();
            let mut zb = Point(chain_in[n - 2].point_cross(chain_in[n - 1]).0.normalize());
            if za.0.dot(ref_out.0) > 0.0 {
                za = Point(-za.0);
            }
            if zb.0.dot(ref_out.0) > 0.0 {
                zb = Point(-zb.0);
            }

            let a0_connector = get_connector(chain_in[1], chain_in[0], chain_out[0]);
            let b1_connector = get_connector(chain_in[n - 2], chain_in[n - 1], chain_out[2]);

            // Change in winding for Zb.
            let chain_z = vec![
                chain_out[0],
                chain_out[1],
                chain_in[1],
                chain_in[0],
                a0_connector,
                chain_out[0],
            ];
            winding_delta += get_edge_winding_delta(&za, &zb, &chain_z);

            // Change in winding of ZbR due to snapping C to C'.
            let mut chain_diff = chain_out.clone();
            chain_diff.push(b1_connector);
            chain_diff.extend(chain_in.iter().rev());
            chain_diff.push(a0_connector);
            chain_diff.push(chain_out[0]);
            winding_delta += get_edge_winding_delta(&zb, &ref_in, &chain_diff);

            // Change in winding of RR' with respect to C'.
            winding_delta += get_edge_winding_delta(&ref_in, &ref_out, &chain_out);
        }
    }
    winding_delta
}

/// Returns the first vertex of the snapped edge chain for the given input
/// edge, or -1 if this input edge does not exist in the graph.
pub(crate) fn find_first_vertex_id(input_edge_id: InputEdgeId, g: &Graph) -> VertexId {
    let mut excess_degree_map: BTreeMap<VertexId, i32> = BTreeMap::new();
    for e in (0..g.num_edges().0).map(EdgeId) {
        let id_set = g.input_edge_ids(e);
        for id in &id_set {
            if input_edge_id == *id {
                let edge = g.edge(e);
                *excess_degree_map.entry(edge.0).or_insert(0) += 1;
                *excess_degree_map.entry(edge.1).or_insert(0) -= 1;
                break;
            }
        }
    }
    if excess_degree_map.is_empty() {
        return VertexId(-1);
    }

    for (&v, &degree) in &excess_degree_map {
        if degree == 1 {
            return v;
        }
    }
    debug_assert_eq!(excess_degree_map.len(), 1);
    excess_degree_map
        .keys()
        .next()
        .copied()
        .unwrap_or(VertexId(-1))
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::builder::graph::GraphOptions;
    use crate::s2::builder::graph::{DegenerateEdges, DuplicateEdges, EdgeType, SiblingPairs};
    use crate::s2::builder::layer::Layer;
    use crate::s2::builder::snap::IdentitySnapFunction;
    use crate::s2::builder::{Options, S2Builder};
    use crate::s2::text_format;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Layer that checks `get_snapped_winding_delta` against expected value.
    #[derive(Debug)]
    struct WindingCheckLayer {
        ref_input_edge_id: InputEdgeId,
        expected_winding_delta: i32,
        input_edges: Rc<RefCell<Vec<(Point, Point)>>>,
    }

    impl Layer for WindingCheckLayer {
        fn graph_options(&self) -> GraphOptions {
            GraphOptions {
                edge_type: EdgeType::Directed,
                degenerate_edges: DegenerateEdges::Keep,
                duplicate_edges: DuplicateEdges::Merge,
                sibling_pairs: SiblingPairs::Keep,
                allow_vertex_filtering: true,
            }
        }

        fn build(&mut self, g: &Graph, error: &mut S2Error) {
            let input_edges = self.input_edges.borrow();
            let ref_in = input_edges[self.ref_input_edge_id.as_usize()].0;
            let ref_v = find_first_vertex_id(self.ref_input_edge_id, g);
            assert!(ref_v >= 0, "Reference vertex not found in graph");
            let winding_delta =
                get_snapped_winding_delta(ref_in, ref_v, None, &input_edges, g, error);
            assert!(error.is_ok(), "Error: {error}");
            assert_eq!(
                winding_delta, self.expected_winding_delta,
                "Expected winding delta {}, got {}",
                self.expected_winding_delta, winding_delta
            );
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }
    }

    fn expect_winding_delta(
        loops_str: &str,
        forced_vertices_str: &str,
        snap_radius_degrees: f64,
        ref_input_edge_id: impl Into<InputEdgeId>,
        expected_winding_delta: i32,
    ) {
        let ref_input_edge_id = ref_input_edge_id.into();
        let input_edges_shared = Rc::new(RefCell::new(Vec::new()));

        let options = Options::new(Box::new(IdentitySnapFunction::new(Angle::from_degrees(
            snap_radius_degrees,
        ))));
        let mut builder = S2Builder::new(options);

        builder.start_layer(Box::new(WindingCheckLayer {
            ref_input_edge_id,
            expected_winding_delta,
            input_edges: Rc::clone(&input_edges_shared),
        }));

        if !forced_vertices_str.is_empty() {
            let forced = text_format::parse_points(forced_vertices_str);
            for v in &forced {
                builder.force_vertex(*v);
            }
        }

        let polygon = text_format::make_lax_polygon(loops_str);
        builder.add_shape(&polygon);

        // Verify the reference edge is degenerate.
        let ref_edge = builder.input_edge(ref_input_edge_id);
        assert_eq!(ref_edge.0, ref_edge.1, "Reference edge not degenerate");

        // Clone input edges before build.
        let edges: Vec<(Point, Point)> = (0..builder.num_input_edges())
            .map(|i| builder.input_edge(i))
            .collect();
        *input_edges_shared.borrow_mut() = edges;

        builder.build().expect("Build failed");
    }

    #[test]
    fn test_no_other_edges() {
        expect_winding_delta("0:0", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_degenerate_input_loops() {
        expect_winding_delta("0:0; 1:1; 2:2", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_duplicate_degenerate_input_loops() {
        expect_winding_delta("0:0; 0:0; 1:1; 1:1", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_collapsing_shell() {
        expect_winding_delta("0:0; 1:1, 1:-2, -2:1", "0:0", 10.0, 0, -1);
    }

    #[test]
    fn test_collapsing_hole() {
        expect_winding_delta("0:0; 1:1, -2:1, 1:-2", "0:0", 10.0, 0, 1);
    }

    #[test]
    fn test_collapsing_double_shell() {
        expect_winding_delta("0:0; 1:1, 1:-2, -2:1, 2:2, 2:-3, -3:2", "0:0", 10.0, 0, -2);
    }

    #[test]
    fn test_external_loop_ref_stays_outside() {
        expect_winding_delta("0:0; 20:0, 0:0, 0:20", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_external_loop_ref_stays_inside() {
        expect_winding_delta("0:0; 0:-20, 0:0, 20:0", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_external_loop_ref_moves_inside() {
        expect_winding_delta("1:1; 0:-20, 1:-1, 20:0", "0:0", 10.0, 0, 1);
    }

    #[test]
    fn test_crossing_edge_ref_stays_outside() {
        expect_winding_delta("-1:-1; 20:-20, -20:20, 20:20", "0:0", 10.0, 0, 0);
    }

    #[test]
    fn test_crossing_edge_ref_moves_outside() {
        expect_winding_delta("1:1; 20:-20, -20:20, 20:20", "0:0", 10.0, 0, -1);
    }

    #[test]
    fn test_double_hole_to_single_hole() {
        expect_winding_delta("4:4; 0:20, 3:3, 6:3, 2:7, 2:2, 2:20", "0:0", 10.0, 0, 1);
    }

    #[test]
    fn test_double_hole_to_single_shell() {
        expect_winding_delta(
            "4:4; 0:-20, 6:2, 2:6, 2:2, 6:2, 2:6, 2:2, 20:0",
            "0:0",
            10.0,
            0,
            3,
        );
    }

    #[test]
    fn test_edges_cross_snap_to_same_vertex() {
        expect_winding_delta("1:1; -5:30, 7:-3, -7:-3, 5:30", "0:0, 0:15", 10.0, 0, -1);
    }

    #[test]
    fn test_edges_cross_snap_to_different_vertices() {
        expect_winding_delta(
            "1:1; -5:40, 7:-3, -7:-3, 5:40",
            "0:0, 6:10, -6:10",
            10.0,
            0,
            -1,
        );
    }

    #[test]
    fn test_ref_winding_numbers_change() {
        // Za changes.
        expect_winding_delta(
            "1:1; 70:-179.99, 5:0, 0:5, -0.01:110",
            "0:0, 1:90",
            10.0,
            0,
            0,
        );
        // Zb changes.
        expect_winding_delta(
            "1:1; 70:-179.99, 5:0, 0:5, -0.01:110",
            "0:0, 89:90",
            10.0,
            0,
            0,
        );
        // Both change.
        expect_winding_delta(
            "1:1; 70:-179.99, 5:0, 0:5, -0.01:110",
            "0:0, 1:90, 89:90",
            10.0,
            0,
            0,
        );
        // Opposite direction.
        expect_winding_delta(
            "1:1; 70:179.99, 5:0, 0:5, 0:110",
            "0:0, -1:20, 1:90",
            10.0,
            0,
            0,
        );
    }

    #[test]
    fn test_ref_loops_topologically_consistent() {
        expect_winding_delta(
            "-45:24; 0:148, 0:0, -31:-48, 44:-39, -59:0",
            "-31:-48, 44:-39",
            60.0,
            0,
            -1,
        );
        expect_winding_delta(
            "-45:24;  -59:0, 44:-39, -31:-48, 0:0, 0:148",
            "-31:-48, 44:-39",
            60.0,
            0,
            1,
        );
    }

    #[test]
    fn test_complex_example() {
        expect_winding_delta(
            "1:1; \
             70:179.99, 5:0, 0:5, 0:110; \
             70:179.99, 0:0, 0:3, 3:0, 0:-1, 0:110; \
             10:-10, -10:10, 10:10; \
             2:2, 1:-2, -1:2, 2:2, 1:-2, -1:2 ",
            "0:0, -1:90, 1:90, 45:-5",
            10.0,
            0,
            -5,
        );
    }

    #[test]
    fn test_ensure_za_zb_not_in_voronoi_region() {
        expect_winding_delta("30:42, 30:42; -27:52, 66:131, 30:-93", "", 67.0, 0, -1);
    }

    #[test]
    fn test_ensure_chain_diff_loop_is_closed() {
        expect_winding_delta("8:26, 8:26; -36:70, -64:-35, -41:48", "", 66.0, 0, 0);
    }

    #[test]
    fn test_voronoi_exclusion_bug() {
        // C++: GetSnappedWindingDelta::VoronoiExclusionBug
        // Previously failed due to a bug in GetVoronoiSiteExclusion()
        // involving long edges (near 180 degrees) and large snap radii.
        expect_winding_delta(
            "24.97:102.02, 24.97:102.02; \
             25.84:131.46, -29.23:-166.58, 29.40:173.03, -18.02:-5.83",
            "",
            64.83,
            0,
            -1,
        );
    }
}
