// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Finding edges in a [`ShapeIndex`] that cross a given edge.
//!
//! [`CrossingEdgeQuery`] efficiently finds the edge IDs of shapes that are
//! crossed by a given edge. If you need to query many edges, reuse a single
//! `CrossingEdgeQuery` instance.
//!
//! Corresponds to C++ `s2crossing_edge_query.h`, Go `s2/crossing_edge_query.go`.

#![expect(clippy::cast_sign_loss, reason = "EdgeId (i32) used as Vec indices")]
#![expect(
    clippy::cast_possible_truncation,
    reason = "EdgeId (i32) -> usize for Vec indexing"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for EdgeId — always in range"
)]
use std::collections::HashMap;

use crate::r2;
use crate::s2::edge_clipping::{self, interpolate_float64};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::Crossing;
use crate::s2::padded_cell::PaddedCell;
use crate::s2::shape::{Shape, ShapeId};
use crate::s2::shape_index::{CellRelation, ShapeIndex, ShapeIndexCell, ShapeIndexIterator};
use crate::s2::{CellId, Point};

/// Specifies what types of edge crossings are reported.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CrossingType {
    /// Only report intersections at a point interior to both edges.
    #[default]
    Interior,
    /// Report all intersections, including edges that share a vertex.
    All,
}

/// Finds edges in a [`ShapeIndex`] that cross a given query edge.
///
/// For best performance, reuse a single `CrossingEdgeQuery` for multiple queries
/// against the same index.
#[derive(Debug)]
pub struct CrossingEdgeQuery<'a> {
    index: &'a ShapeIndex,
    // Temporary values for the current query edge in (u,v) coordinates.
    a: r2::Point,
    b: r2::Point,
    iter: ShapeIndexIterator<'a>,
    // Candidate cells accumulated during a query.
    cells: Vec<&'a ShapeIndexCell>,
}

impl<'a> CrossingEdgeQuery<'a> {
    /// Creates a new query for the given index.
    pub fn new(index: &'a ShapeIndex) -> Self {
        CrossingEdgeQuery {
            index,
            a: r2::Point::default(),
            b: r2::Point::default(),
            iter: index.iter(),
            cells: Vec::new(),
        }
    }

    /// Returns the edge IDs of `shape` that intersect the edge AB.
    ///
    /// If `cross_type` is [`CrossingType::Interior`], only intersections at a
    /// point interior to both edges are reported. If [`CrossingType::All`],
    /// edges sharing a vertex are also reported.
    pub fn crossings(
        &mut self,
        a: Point,
        b: Point,
        shape: &dyn Shape,
        shape_id: impl Into<ShapeId>,
        cross_type: CrossingType,
    ) -> Vec<i32> {
        let mut edges = self.candidates(a, b, shape, shape_id);
        if edges.is_empty() {
            return edges;
        }

        let mut crosser = EdgeCrosser::new(a, b);
        let mut out = 0;
        for i in 0..edges.len() {
            let e = shape.edge(edges[i] as usize);
            let sign = crosser.crossing_sign(e.v0, e.v1);
            let dominated = match cross_type {
                CrossingType::All => sign == Crossing::MaybeCross || sign == Crossing::Cross,
                CrossingType::Interior => sign == Crossing::Cross,
            };
            if dominated {
                edges[out] = edges[i];
                out += 1;
            }
        }
        edges.truncate(out);
        edges
    }

    /// Returns all edges in the index that intersect the edge AB, grouped by
    /// shape ID. Every returned shape has at least one crossing edge.
    pub fn crossings_edge_map(
        &mut self,
        a: Point,
        b: Point,
        cross_type: CrossingType,
    ) -> HashMap<ShapeId, Vec<i32>> {
        let mut edge_map = self.candidates_edge_map(a, b);
        if edge_map.is_empty() {
            return edge_map;
        }

        let mut crosser = EdgeCrosser::new(a, b);
        let mut to_remove = Vec::new();

        for (&shape_id, edges) in &mut edge_map {
            let Some(shape) = self.index.shape(shape_id) else {
                to_remove.push(shape_id);
                continue;
            };
            let mut out = 0;
            for i in 0..edges.len() {
                let e = shape.edge(edges[i] as usize);
                let sign = crosser.crossing_sign(e.v0, e.v1);
                let dominated = match cross_type {
                    CrossingType::All => sign == Crossing::MaybeCross || sign == Crossing::Cross,
                    CrossingType::Interior => sign == Crossing::Cross,
                };
                if dominated {
                    edges[out] = edges[i];
                    out += 1;
                }
            }
            if out == 0 {
                to_remove.push(shape_id);
            } else {
                edges.truncate(out);
            }
        }

        for id in to_remove {
            edge_map.remove(&id);
        }
        edge_map
    }

    /// Returns a superset of edge IDs of `shape` that may intersect the edge AB.
    pub fn candidates(
        &mut self,
        a: Point,
        b: Point,
        shape: &dyn Shape,
        shape_id: impl Into<ShapeId>,
    ) -> Vec<i32> {
        let shape_id = shape_id.into();
        // For small shapes, brute force is faster.
        const MAX_BRUTE_FORCE_EDGES: usize = 27;
        let max_edges = shape.num_edges();
        if max_edges <= MAX_BRUTE_FORCE_EDGES {
            return (0..max_edges as i32).collect();
        }

        // Compute the cells intersected by the query edge.
        self.get_cells_for_edge(a, b);
        if self.cells.is_empty() {
            return Vec::new();
        }

        // Gather all edges that intersect those cells.
        let mut edges = Vec::new();
        for cell in &self.cells {
            if let Some(clipped) = cell.find_by_shape_id(shape_id) {
                edges.extend_from_slice(&clipped.edges);
            }
        }

        if self.cells.len() > 1 {
            edges.sort_unstable();
            edges.dedup();
        }

        edges
    }

    /// Returns a map from shape IDs to candidate edge IDs that may intersect
    /// the edge AB.
    pub fn candidates_edge_map(&mut self, a: Point, b: Point) -> HashMap<ShapeId, Vec<i32>> {
        let mut edge_map = HashMap::new();

        // Optimization: for a single shape, use the candidates() method.
        if self.index.len() == 1
            && let Some(shape) = self.index.shape(0)
        {
            let candidates = self.candidates(a, b, shape, ShapeId(0));
            edge_map.insert(ShapeId(0), candidates);
            return edge_map;
        }

        // Compute the cells intersected by the query edge.
        self.get_cells_for_edge(a, b);
        if self.cells.is_empty() {
            return edge_map;
        }

        // Gather all edges from all shapes in all intersected cells.
        for cell in &self.cells {
            for clipped in &cell.shapes {
                let entry = edge_map.entry(clipped.shape_id).or_insert_with(Vec::new);
                for &edge_id in &clipped.edges {
                    entry.push(edge_id);
                }
            }
        }

        if self.cells.len() > 1 {
            for edges in edge_map.values_mut() {
                edges.sort_unstable();
                edges.dedup();
            }
        }

        edge_map
    }

    /// Returns `ShapeIndexCells` that might contain edges intersecting the edge
    /// AB within the given padded cell root.
    pub fn get_cells(
        &mut self,
        a: Point,
        b: Point,
        root: &mut PaddedCell,
    ) -> Vec<&'a ShapeIndexCell> {
        self.cells.clear();
        if let Some((a_uv, b_uv)) = edge_clipping::clip_to_face(a, b, root.cell_id().face()) {
            self.a = a_uv;
            self.b = b_uv;
            let edge_bound = r2::Rect::from_point_pair(self.a, self.b);
            if root.bound().intersects(edge_bound) {
                self.compute_cells_intersected(root, edge_bound);
            }
        }
        self.cells.clone()
    }

    /// Populates the cells field with index cells intersected by the edge AB.
    fn get_cells_for_edge(&mut self, a: Point, b: Point) {
        self.cells.clear();

        let segments = edge_clipping::face_segments(a, b);
        for segment in &segments {
            self.a = segment.a;
            self.b = segment.b;

            // Start at the smallest cell that contains the edge (edge root cell).
            let edge_bound = r2::Rect::from_point_pair(self.a, self.b);
            let mut pcell = PaddedCell::from_cell_id(CellId::from_face(segment.face), 0.0);
            let edge_root = pcell.shrink_to_fit(edge_bound);

            let relation = self.iter.locate_cell_id(edge_root);
            match relation {
                CellRelation::Indexed => {
                    // edge_root is an index cell or is contained by one.
                    debug_assert!(self.iter.cell_id().contains(edge_root));
                    if let Some(cell) = self.iter.index_cell() {
                        self.cells.push(cell);
                    }
                }
                CellRelation::Subdivided => {
                    // edge_root is subdivided into index cells.
                    if !edge_root.is_face() {
                        pcell = PaddedCell::from_cell_id(edge_root, 0.0);
                    }
                    self.compute_cells_intersected(&mut pcell, edge_bound);
                }
                CellRelation::Disjoint => {}
            }
        }
    }

    /// Recursively finds index cells intersected by the current edge that
    /// are descendants of `pcell`.
    fn compute_cells_intersected(&mut self, pcell: &mut PaddedCell, edge_bound: r2::Rect) {
        self.iter.seek(pcell.cell_id().range_min());
        if self.iter.done() || self.iter.cell_id() > pcell.cell_id().range_max() {
            return;
        }
        if self.iter.cell_id() == pcell.cell_id() {
            if let Some(cell) = self.iter.index_cell() {
                self.cells.push(cell);
            }
            return;
        }

        // Split the edge among the four children of pcell.
        let center = pcell.middle().lo();

        if edge_bound.x.hi < center.x {
            // Edge is entirely in the two left children.
            self.clip_v_axis(edge_bound, center.y, 0, pcell);
        } else if edge_bound.x.lo >= center.x {
            // Edge is entirely in the two right children.
            self.clip_v_axis(edge_bound, center.y, 1, pcell);
        } else {
            let child_bounds = self.split_u_bound(edge_bound, center.x);
            if edge_bound.y.hi < center.y {
                // Edge is in the two lower children.
                let mut child0 = PaddedCell::from_parent_ij(pcell, 0, 0);
                self.compute_cells_intersected(&mut child0, child_bounds[0]);
                let mut child1 = PaddedCell::from_parent_ij(pcell, 1, 0);
                self.compute_cells_intersected(&mut child1, child_bounds[1]);
            } else if edge_bound.y.lo >= center.y {
                // Edge is in the two upper children.
                let mut child0 = PaddedCell::from_parent_ij(pcell, 0, 1);
                self.compute_cells_intersected(&mut child0, child_bounds[0]);
                let mut child1 = PaddedCell::from_parent_ij(pcell, 1, 1);
                self.compute_cells_intersected(&mut child1, child_bounds[1]);
            } else {
                // Edge spans all four children.
                self.clip_v_axis(child_bounds[0], center.y, 0, pcell);
                self.clip_v_axis(child_bounds[1], center.y, 1, pcell);
            }
        }
    }

    /// Given either the left (i=0) or right (i=1) side of `pcell`, determines
    /// whether the current edge intersects the lower child, upper child, or
    /// both, and recurses.
    fn clip_v_axis(&mut self, edge_bound: r2::Rect, center: f64, i: i32, pcell: &mut PaddedCell) {
        if edge_bound.y.hi < center {
            let mut child = PaddedCell::from_parent_ij(pcell, i, 0);
            self.compute_cells_intersected(&mut child, edge_bound);
        } else if edge_bound.y.lo >= center {
            let mut child = PaddedCell::from_parent_ij(pcell, i, 1);
            self.compute_cells_intersected(&mut child, edge_bound);
        } else {
            let child_bounds = self.split_v_bound(edge_bound, center);
            let mut child0 = PaddedCell::from_parent_ij(pcell, i, 0);
            self.compute_cells_intersected(&mut child0, child_bounds[0]);
            let mut child1 = PaddedCell::from_parent_ij(pcell, i, 1);
            self.compute_cells_intersected(&mut child1, child_bounds[1]);
        }
    }

    /// Splits the edge bound along the u-axis at the given value.
    fn split_u_bound(&self, edge_bound: r2::Rect, u: f64) -> [r2::Rect; 2] {
        let v = edge_bound.y.project(interpolate_float64(
            u, self.a.x, self.b.x, self.a.y, self.b.y,
        ));
        let diag = if (self.a.x > self.b.x) == (self.a.y > self.b.y) {
            0
        } else {
            1
        };
        split_bound(edge_bound, 0, diag, u, v)
    }

    /// Splits the edge bound along the v-axis at the given value.
    fn split_v_bound(&self, edge_bound: r2::Rect, v: f64) -> [r2::Rect; 2] {
        let u = edge_bound.x.project(interpolate_float64(
            v, self.a.y, self.b.y, self.a.x, self.b.x,
        ));
        let diag = if (self.a.x > self.b.x) == (self.a.y > self.b.y) {
            0
        } else {
            1
        };
        split_bound(edge_bound, diag, 0, u, v)
    }
}

/// Splits `edge_bound` into two child bounds at the point (u, v).
/// `u_end` and `v_end` indicate which endpoint of the first child is updated.
fn split_bound(edge_bound: r2::Rect, u_end: usize, v_end: usize, u: f64, v: f64) -> [r2::Rect; 2] {
    let mut child0 = edge_bound;
    let mut child1 = edge_bound;

    if u_end == 1 {
        child0.x.lo = u;
        child1.x.hi = u;
    } else {
        child0.x.hi = u;
        child1.x.lo = u;
    }

    if v_end == 1 {
        child0.y.lo = v;
        child1.y.hi = v;
    } else {
        child0.y.hi = v;
        child1.y.lo = v;
    }

    debug_assert!(!child0.is_empty());
    debug_assert!(edge_bound.contains(child0));
    debug_assert!(!child1.is_empty());
    debug_assert!(edge_bound.contains(child1));
    [child0, child1]
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;
    use crate::s2::lax_loop::LaxLoop;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_empty_index() {
        let index = ShapeIndex::new();
        let mut query = CrossingEdgeQuery::new(&index);
        let result = query.crossings_edge_map(p(0.0, 0.0), p(0.0, 90.0), CrossingType::All);
        assert!(result.is_empty());
    }

    #[test]
    fn test_no_crossings() {
        // A small triangle near the equator
        let loop_pts = vec![p(1.0, 1.0), p(1.0, 2.0), p(2.0, 1.0)];
        let shape = LaxLoop::new(loop_pts);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let mut query = CrossingEdgeQuery::new(&index);
        // Query edge far away
        let result = query.crossings(
            p(80.0, 80.0),
            p(80.0, 81.0),
            index.shape(0).unwrap(),
            0,
            CrossingType::All,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_crossing_triangle() {
        // A triangle
        let v0 = p(0.0, 0.0);
        let v1 = p(0.0, 10.0);
        let v2 = p(10.0, 0.0);
        let shape = LaxLoop::new(vec![v0, v1, v2]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let mut query = CrossingEdgeQuery::new(&index);

        // Query edge that crosses the triangle
        let a = p(-1.0, 5.0);
        let b = p(1.0, 5.0);
        let result = query.crossings(a, b, index.shape(0).unwrap(), 0, CrossingType::All);
        assert!(
            !result.is_empty(),
            "expected at least one crossing edge, got none"
        );
    }

    #[test]
    fn test_candidates_brute_force() {
        // Shape with few edges -> brute force path
        let shape = LaxLoop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let mut query = CrossingEdgeQuery::new(&index);
        let candidates = query.candidates(p(0.0, 0.0), p(10.0, 10.0), index.shape(0).unwrap(), 0);
        // Should return all 3 edges
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates, vec![0, 1, 2]);
    }

    #[test]
    fn test_crossings_edge_map() {
        let v0 = p(0.0, 0.0);
        let v1 = p(0.0, 10.0);
        let v2 = p(10.0, 0.0);
        let shape = LaxLoop::new(vec![v0, v1, v2]);
        let mut index = ShapeIndex::new();
        index.add(Box::new(shape));
        index.build();

        let mut query = CrossingEdgeQuery::new(&index);

        // Query edge crossing the triangle
        let a = p(-1.0, 5.0);
        let b = p(1.0, 5.0);
        let result = query.crossings_edge_map(a, b, CrossingType::All);
        // Should have at least one shape with crossing edges
        assert!(!result.is_empty());
    }

    #[test]
    fn test_split_bound() {
        let rect =
            r2::Rect::from_point_pair(r2::Point { x: 0.0, y: 0.0 }, r2::Point { x: 1.0, y: 1.0 });
        let [c0, c1] = split_bound(rect, 0, 0, 0.5, 0.5);

        // child0: x.hi = 0.5, y.hi = 0.5
        assert!((c0.x.hi - 0.5).abs() < 1e-15);
        assert!((c0.y.hi - 0.5).abs() < 1e-15);
        // child1: x.lo = 0.5, y.lo = 0.5
        assert!((c1.x.lo - 0.5).abs() < 1e-15);
        assert!((c1.y.lo - 0.5).abs() < 1e-15);
    }

    #[test]
    fn test_crossing_edge_query_multi_shape() {
        // Two separate triangles, both near the equator but at different longitudes.
        // Shape 0: triangle around (0, 5)
        let shape0 = LaxLoop::new(vec![p(-2.0, 3.0), p(-2.0, 7.0), p(2.0, 5.0)]);
        // Shape 1: triangle around (0, 15)
        let shape1 = LaxLoop::new(vec![p(-2.0, 13.0), p(-2.0, 17.0), p(2.0, 15.0)]);

        let mut index = ShapeIndex::new();
        index.add(Box::new(shape0));
        index.add(Box::new(shape1));
        index.build();

        // Query edge that crosses through both triangles (a long horizontal edge).
        let a = p(0.0, 0.0);
        let b = p(0.0, 20.0);
        let mut query = CrossingEdgeQuery::new(&index);
        let result = query.crossings_edge_map(a, b, CrossingType::All);

        // Both shape 0 and shape 1 should have crossing edges.
        assert!(
            result.contains_key(&ShapeId(0)),
            "expected crossings for shape 0, got {result:?}"
        );
        assert!(
            result.contains_key(&ShapeId(1)),
            "expected crossings for shape 1, got {result:?}"
        );
        // Each shape should have at least one crossing edge.
        assert!(!result[&ShapeId(0)].is_empty());
        assert!(!result[&ShapeId(1)].is_empty());
    }

    #[test]
    fn test_crossing_edge_query_candidates_span_cells() {
        // Build a shape with many edges so that candidates() uses the
        // index-based path (>27 edges) rather than brute force.
        // Create a polygon with many small edges along the equator.
        let mut pts = Vec::new();
        for i in 0..40 {
            let lng = f64::from(i) * 0.5;
            let lat = if i % 2 == 0 { 0.5 } else { -0.5 };
            pts.push(p(lat, lng));
        }
        let shape = LaxLoop::new(pts);
        let mut index = ShapeIndex::new();
        let sid = index.add(Box::new(shape));
        index.build();

        // A long edge that spans most of the shape's extent.
        let a = p(0.0, -1.0);
        let b = p(0.0, 21.0);
        let mut query = CrossingEdgeQuery::new(&index);
        let candidates = query.candidates(a, b, index.shape(sid).unwrap(), sid);

        // We should have candidates (possibly many), and they should be deduplicated
        // (no duplicates).
        assert!(
            !candidates.is_empty(),
            "expected at least some candidates for a long spanning edge"
        );
        let mut sorted = candidates.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            candidates.len(),
            sorted.len(),
            "candidates should be deduplicated"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_crossing_type_roundtrip() {
        for v in [CrossingType::Interior, CrossingType::All] {
            let j = serde_json::to_string(&v).unwrap();
            assert_eq!(v, serde_json::from_str::<CrossingType>(&j).unwrap());
        }
    }
}
