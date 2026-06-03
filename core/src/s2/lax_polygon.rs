// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! A lightweight polygon shape supporting degeneracies.
//!
//! [`LaxPolygon`] is similar to [`Polygon`](super::Polygon) except that it
//! supports polygons with degeneracies (degenerate edges and sibling edge
//! pairs). It is faster to initialize and more compact, but does not have
//! built-in operations — use [`ShapeIndex`](super::shape_index::ShapeIndex)-based operations instead.
//!
//! Loops with fewer than three vertices are interpreted as follows:
//! - Two vertices: defines two edges (in opposite directions).
//! - One vertex: defines a single degenerate edge.
//! - Zero vertices: interpreted as the "full loop" containing all points.
//!
//! Corresponds to C++ `s2lax_polygon_shape.h`, Go `s2/lax_polygon.go`.

use crate::s2::Point;
use crate::s2::shape::{
    Chain, ChainPosition, Dimension, Edge, ReferencePoint, Shape, reference_point_for_shape,
};

/// A region defined by a collection of closed loops (dimension 2).
///
/// The interior is the region to the left of all loops. Unlike
/// [`Polygon`](super::Polygon), this type supports degeneracies
/// and does not validate or normalize its input.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LaxPolygon {
    num_loops: usize,
    vertices: Vec<Point>,
    /// For a single loop, stores the number of vertices directly.
    /// For multiple loops, this field is unused (`cumulative_vertices` is used).
    num_verts: usize,
    /// For 2+ loops: `cumulative_vertices`[i] is the index of the first vertex
    /// of loop i. `cumulative_vertices`[`num_loops`] is the total vertex count.
    cumulative_vertices: Vec<usize>,
}

impl LaxPolygon {
    /// Creates a `LaxPolygon` from a list of loops, where each loop is a
    /// slice of vertices.
    pub fn from_loops(loops: &[&[Point]]) -> Self {
        let num_loops = loops.len();
        match num_loops {
            0 => LaxPolygon {
                num_loops: 0,
                vertices: Vec::new(),
                num_verts: 0,
                cumulative_vertices: Vec::new(),
            },
            1 => {
                let verts = loops[0].to_vec();
                let n = verts.len();
                LaxPolygon {
                    num_loops: 1,
                    vertices: verts,
                    num_verts: n,
                    cumulative_vertices: Vec::new(),
                }
            }
            _ => {
                let mut cumulative = Vec::with_capacity(num_loops + 1);
                let mut total = 0;
                for lp in loops {
                    cumulative.push(total);
                    total += lp.len();
                }
                cumulative.push(total);

                let mut vertices = Vec::with_capacity(total);
                for lp in loops {
                    vertices.extend_from_slice(lp);
                }

                LaxPolygon {
                    num_loops,
                    vertices,
                    num_verts: 0,
                    cumulative_vertices: cumulative,
                }
            }
        }
    }

    /// Creates a `LaxPolygon` from owned loop vectors.
    pub fn from_loops_owned(loops: Vec<Vec<Point>>) -> Self {
        let refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
        LaxPolygon::from_loops(&refs)
    }

    /// Creates a `LaxPolygon` from a `Polygon` reference by copying its
    /// loop vertices. Full loops are represented as empty vertex lists.
    /// Hole loops are reversed because `S2Polygon` and `S2LaxPolygonShape` have
    /// opposite hole orientations.
    ///
    /// Corresponds to C++ `S2LaxPolygonShape::Init(const S2Polygon&)`.
    pub fn from_polygon_ref(polygon: &super::Polygon) -> Self {
        let mut loops = Vec::with_capacity(polygon.num_loops());
        for i in 0..polygon.num_loops() {
            let lp = polygon.loop_at(i);
            if lp.is_full_loop() {
                loops.push(Vec::new()); // empty span = full loop
            } else {
                let mut verts: Vec<Point> = (0..lp.num_vertices()).map(|j| lp.vertex(j)).collect();
                if lp.is_hole() {
                    verts.reverse();
                }
                loops.push(verts);
            }
        }
        LaxPolygon::from_loops_owned(loops)
    }

    /// Creates an empty `LaxPolygon` (no loops).
    pub fn empty() -> Self {
        LaxPolygon::default()
    }

    /// Creates a full `LaxPolygon` (a single empty loop = full sphere).
    pub fn full() -> Self {
        LaxPolygon::from_loops(&[&[]])
    }

    /// Returns the number of loops.
    pub fn num_loops(&self) -> usize {
        self.num_loops
    }

    /// Returns the total number of vertices across all loops.
    pub fn num_vertices(&self) -> usize {
        if self.num_loops <= 1 {
            self.num_verts
        } else {
            self.cumulative_vertices[self.num_loops]
        }
    }

    /// Returns the number of vertices in the given loop.
    pub fn num_loop_vertices(&self, i: usize) -> usize {
        debug_assert!(i < self.num_loops);
        if self.num_loops == 1 {
            self.num_verts
        } else {
            self.cumulative_vertices[i + 1] - self.cumulative_vertices[i]
        }
    }

    /// Returns a slice of all vertices (across all loops).
    pub fn all_vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Returns the vertex at index `j` within loop `i`.
    pub fn loop_vertex(&self, i: usize, j: usize) -> Point {
        debug_assert!(i < self.num_loops);
        debug_assert!(j < self.num_loop_vertices(i));
        if self.num_loops == 1 {
            self.vertices[j]
        } else {
            self.vertices[self.cumulative_vertices[i] + j]
        }
    }
}

impl Shape for LaxPolygon {
    fn num_edges(&self) -> usize {
        self.num_vertices()
    }

    fn edge(&self, id: usize) -> Edge {
        let next_id = id + 1;
        if self.num_loops == 1 {
            let next = if next_id == self.num_verts {
                0
            } else {
                next_id
            };
            return Edge::new(self.vertices[id], self.vertices[next]);
        }

        // Find which loop this edge belongs to.
        let mut next_loop = 0;
        while self.cumulative_vertices[next_loop] <= id {
            next_loop += 1;
        }
        // Wrap around to the first vertex of the loop if needed.
        let next = if next_id == self.cumulative_vertices[next_loop] {
            self.cumulative_vertices[next_loop - 1]
        } else {
            next_id
        };
        Edge::new(self.vertices[id], self.vertices[next])
    }

    fn reference_point(&self) -> ReferencePoint {
        reference_point_for_shape(self)
    }

    fn num_chains(&self) -> usize {
        self.num_loops
    }

    fn chain(&self, chain_id: usize) -> Chain {
        if self.num_loops == 1 {
            return Chain::new(0, self.num_vertices());
        }
        let start = self.cumulative_vertices[chain_id];
        let length = self.cumulative_vertices[chain_id + 1] - start;
        Chain::new(start, length)
    }

    fn chain_edge(&self, chain_id: usize, offset: usize) -> Edge {
        let n = self.num_loop_vertices(chain_id);
        let next = if offset + 1 == n { 0 } else { offset + 1 };
        if self.num_loops == 1 {
            return Edge::new(self.vertices[offset], self.vertices[next]);
        }
        let base = self.cumulative_vertices[chain_id];
        Edge::new(self.vertices[base + offset], self.vertices[base + next])
    }

    fn chain_position(&self, edge_id: usize) -> ChainPosition {
        if self.num_loops == 1 {
            return ChainPosition::new(0, edge_id);
        }

        // Find the loop containing this edge.
        let mut next_loop = 1;
        while self.cumulative_vertices[next_loop] <= edge_id {
            next_loop += 1;
        }
        let chain_id = next_loop - 1;
        let offset = edge_id - self.cumulative_vertices[chain_id];
        ChainPosition::new(chain_id, offset)
    }

    fn dimension(&self) -> Dimension {
        Dimension::Polygon
    }

    fn type_tag(&self) -> u32 {
        5 // S2LaxPolygonShape::kTypeTag
    }

    fn encode_tagged(
        &self,
        w: &mut dyn std::io::Write,
        hint: crate::s2::encoded_s2point_vector::CodingHint,
    ) -> std::io::Result<()> {
        self.encode_with_hint(w, hint)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    fn is_send_sync<T: Sized + Send + Sync + Unpin>() {}

    #[test]
    fn lax_polygon_is_send_sync() {
        is_send_sync::<LaxPolygon>();
    }

    #[test]
    fn test_empty() {
        let poly = LaxPolygon::empty();
        assert_eq!(poly.num_loops(), 0);
        assert_eq!(poly.num_edges(), 0);
        assert_eq!(poly.num_chains(), 0);
        assert_eq!(poly.dimension(), Dimension::Polygon);
        assert!(poly.is_empty());
        assert!(!poly.is_full());
    }

    #[test]
    fn test_full() {
        let poly = LaxPolygon::full();
        assert_eq!(poly.num_loops(), 1);
        assert_eq!(poly.num_edges(), 0);
        assert_eq!(poly.num_chains(), 1);
        assert!(!poly.is_empty());
        // Full polygon: no edges but has a chain (empty loop = full)
        assert!(poly.is_full());
    }

    #[test]
    fn test_single_triangle() {
        let v = vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)];
        let poly = LaxPolygon::from_loops(&[&v]);
        assert_eq!(poly.num_loops(), 1);
        assert_eq!(poly.num_edges(), 3);
        assert_eq!(poly.num_chains(), 1);
        assert_eq!(poly.num_vertices(), 3);

        let chain = poly.chain(0);
        assert_eq!(chain.start, 0);
        assert_eq!(chain.length, 3);

        // Last edge wraps around
        let last_edge = poly.edge(2);
        assert_eq!(last_edge.v0, poly.loop_vertex(0, 2));
        assert_eq!(last_edge.v1, poly.loop_vertex(0, 0));
    }

    #[test]
    fn test_two_loops() {
        let outer = vec![
            p(-10.0, -10.0),
            p(-10.0, 10.0),
            p(10.0, 10.0),
            p(10.0, -10.0),
        ];
        let inner = vec![p(-1.0, -1.0), p(1.0, -1.0), p(1.0, 1.0), p(-1.0, 1.0)];
        let poly = LaxPolygon::from_loops(&[&outer, &inner]);
        assert_eq!(poly.num_loops(), 2);
        assert_eq!(poly.num_edges(), 8);
        assert_eq!(poly.num_chains(), 2);
        assert_eq!(poly.num_vertices(), 8);
        assert_eq!(poly.num_loop_vertices(0), 4);
        assert_eq!(poly.num_loop_vertices(1), 4);

        // Chain 0: outer loop
        let c0 = poly.chain(0);
        assert_eq!(c0.start, 0);
        assert_eq!(c0.length, 4);

        // Chain 1: inner loop
        let c1 = poly.chain(1);
        assert_eq!(c1.start, 4);
        assert_eq!(c1.length, 4);

        // Edge 3 (last of outer) wraps to first vertex of outer
        let e3 = poly.edge(3);
        assert_eq!(e3.v0, poly.loop_vertex(0, 3));
        assert_eq!(e3.v1, poly.loop_vertex(0, 0));

        // Edge 7 (last of inner) wraps to first vertex of inner
        let e7 = poly.edge(7);
        assert_eq!(e7.v0, poly.loop_vertex(1, 3));
        assert_eq!(e7.v1, poly.loop_vertex(1, 0));
    }

    #[test]
    fn test_chain_position_single() {
        let v = vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)];
        let poly = LaxPolygon::from_loops(&[&v]);
        for i in 0..3 {
            let cp = poly.chain_position(i);
            assert_eq!(cp.chain_id, 0);
            assert_eq!(cp.offset, i);
        }
    }

    #[test]
    fn test_chain_position_multi() {
        let a = vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)];
        let b = vec![p(10.0, 10.0), p(10.0, 20.0)];
        let poly = LaxPolygon::from_loops(&[&a, &b]);

        // Edge 0-2 are in chain 0
        for i in 0..3 {
            let cp = poly.chain_position(i);
            assert_eq!(cp.chain_id, 0, "edge {i}");
            assert_eq!(cp.offset, i, "edge {i}");
        }
        // Edge 3-4 are in chain 1
        for i in 3..5 {
            let cp = poly.chain_position(i);
            assert_eq!(cp.chain_id, 1, "edge {i}");
            assert_eq!(cp.offset, i - 3, "edge {i}");
        }
    }

    #[test]
    fn test_chain_edge() {
        let v = vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)];
        let poly = LaxPolygon::from_loops(&[&v]);
        // chain_edge(0, 2) should wrap around
        let ce = poly.chain_edge(0, 2);
        assert_eq!(ce.v0, v[2]);
        assert_eq!(ce.v1, v[0]);
    }

    #[test]
    fn test_has_interior() {
        let v = vec![p(0.0, 0.0), p(0.0, 90.0), p(90.0, 0.0)];
        let poly = LaxPolygon::from_loops(&[&v]);
        assert!(poly.has_interior());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip() {
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let c = Point::from_coords(0.0, 0.0, 1.0);
        let poly = LaxPolygon::from_loops(&[&[a, b, c]]);
        let json = serde_json::to_string(&poly).unwrap();
        let back: LaxPolygon = serde_json::from_str(&json).unwrap();
        assert_eq!(poly.num_loops(), back.num_loops());
        assert_eq!(poly.num_vertices(), back.num_vertices());
        for i in 0..poly.num_vertices() {
            assert_eq!(poly.loop_vertex(0, i), back.loop_vertex(0, i));
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2lax_polygon_shape_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_single_vertex_polygon() {
        // C++ TEST(S2LaxPolygonShape, SingleVertexPolygon)
        let pt = crate::s2::text_format::parse_point("0:0");
        let shape = LaxPolygon::from_loops_owned(vec![vec![pt]]);
        assert_eq!(1, shape.num_loops());
        assert_eq!(1, shape.num_vertices());
        assert_eq!(1, shape.num_edges());
        assert_eq!(1, shape.num_chains());
        assert_eq!(0, shape.chain(0).start);
        assert_eq!(1, shape.chain(0).length);
        let edge = shape.edge(0);
        assert_eq!(pt, edge.v0);
        assert_eq!(pt, edge.v1);
        assert_eq!(Dimension::Polygon, shape.dimension());
        assert!(!shape.is_empty());
        assert!(!shape.is_full());
        assert!(!shape.reference_point().contained);
    }

    #[test]
    fn test_single_loop_polygon() {
        // C++ TEST(S2LaxPolygonShape, SingleLoopPolygon)
        let vertices = crate::s2::text_format::parse_points("0:0, 0:1, 1:1, 1:0");
        let shape = LaxPolygon::from_loops(&[&vertices]);
        assert_eq!(1, shape.num_loops());
        assert_eq!(vertices.len(), shape.num_vertices());
        assert_eq!(vertices.len(), shape.num_loop_vertices(0));
        assert_eq!(vertices.len(), shape.num_edges());
        assert_eq!(1, shape.num_chains());
        assert_eq!(0, shape.chain(0).start);
        assert_eq!(vertices.len(), shape.chain(0).length);
        for i in 0..vertices.len() {
            assert_eq!(vertices[i], shape.loop_vertex(0, i));
            let edge = shape.edge(i);
            assert_eq!(vertices[i], edge.v0);
            assert_eq!(vertices[(i + 1) % vertices.len()], edge.v1);
            assert_eq!(edge.v0, shape.chain_edge(0, i).v0);
            assert_eq!(edge.v1, shape.chain_edge(0, i).v1);
        }
        assert_eq!(Dimension::Polygon, shape.dimension());
        assert!(!shape.is_empty());
        assert!(!shape.is_full());
    }

    #[test]
    fn test_multi_loop_polygon() {
        // C++ TEST(S2LaxPolygonShape, MultiLoopPolygon)
        let loops = [
            crate::s2::text_format::parse_points("0:0, 0:3, 3:3"),
            crate::s2::text_format::parse_points("1:1, 2:2, 1:2"),
        ];
        let shape = LaxPolygon::from_loops(&[&loops[0], &loops[1]]);
        assert_eq!(2, shape.num_loops());
        assert_eq!(2, shape.num_chains());
        let mut num_vertices = 0;
        for i in 0..2 {
            assert_eq!(loops[i].len(), shape.num_loop_vertices(i));
            assert_eq!(num_vertices, shape.chain(i).start);
            assert_eq!(loops[i].len(), shape.chain(i).length);
            for j in 0..loops[i].len() {
                assert_eq!(loops[i][j], shape.loop_vertex(i, j));
                let edge = shape.edge(num_vertices + j);
                assert_eq!(loops[i][j], edge.v0);
                assert_eq!(loops[i][(j + 1) % loops[i].len()], edge.v1);
            }
            num_vertices += loops[i].len();
        }
        assert_eq!(num_vertices, shape.num_vertices());
        assert_eq!(num_vertices, shape.num_edges());
        assert_eq!(Dimension::Polygon, shape.dimension());
        assert!(!shape.is_empty());
        assert!(!shape.is_full());
    }

    #[test]
    fn test_multi_loop_s2_polygon() {
        // C++ TEST(S2LaxPolygonShape, MultiLoopS2Polygon)
        // Verify that oriented_vertex is used when converting from S2Polygon.
        let polygon = crate::s2::text_format::make_polygon("0:0, 0:3, 3:3; 1:1, 1:2, 2:2");
        let shape = LaxPolygon::from_polygon_ref(&polygon);
        for i in 0..polygon.num_loops() {
            let s2loop = polygon.loop_at(i);
            for j in 0..s2loop.num_vertices() {
                assert_eq!(
                    s2loop.oriented_vertex(j),
                    shape.loop_vertex(i, j),
                    "mismatch at loop {i}, vertex {j}"
                );
            }
        }
    }

    #[test]
    fn test_many_loop_polygon() {
        // C++ TEST(S2LaxPolygonShape, ManyLoopPolygon) — deterministic version
        // Test a polygon with enough loops so that binary search is used in chain_position.
        let mut loops: Vec<Vec<Point>> = Vec::new();
        let num_verts_options = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1];
        for i in 0..100 {
            let center = LatLng::from_degrees(0.0, i as f64).to_point();
            let nv = num_verts_options[i % num_verts_options.len()];
            if nv == 0 {
                loops.push(vec![]);
            } else {
                loops.push(crate::s2::testing::make_regular_points(
                    center,
                    crate::s1::Angle::from_degrees(0.1),
                    nv,
                ));
            }
        }
        let refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
        let shape = LaxPolygon::from_loops(&refs);
        assert_eq!(100, shape.num_loops());
        assert_eq!(100, shape.num_chains());

        let mut num_vertices = 0;
        for i in 0..100 {
            assert_eq!(loops[i].len(), shape.num_loop_vertices(i));
            assert_eq!(num_vertices, shape.chain(i).start);
            assert_eq!(loops[i].len(), shape.chain(i).length);
            for j in 0..loops[i].len() {
                assert_eq!(loops[i][j], shape.loop_vertex(i, j));
                let e = num_vertices + j;
                assert_eq!(
                    shape.chain_position(e),
                    ChainPosition {
                        chain_id: i,
                        offset: j
                    }
                );
                assert_eq!(loops[i][j], shape.edge(e).v0);
                assert_eq!(loops[i][(j + 1) % loops[i].len()], shape.edge(e).v1);
            }
            num_vertices += loops[i].len();
        }
        assert_eq!(num_vertices, shape.num_vertices());
        assert_eq!(num_vertices, shape.num_edges());
    }

    #[test]
    fn test_degenerate_loops() {
        // C++ TEST(S2LaxPolygonShape, DegenerateLoops)
        let loops = [
            crate::s2::text_format::parse_points("1:1, 1:2, 2:2, 1:2, 1:3, 1:2, 1:1"),
            crate::s2::text_format::parse_points("0:0, 0:3, 0:6, 0:9, 0:6, 0:3, 0:0"),
            crate::s2::text_format::parse_points("5:5, 6:6"),
        ];
        let refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
        let shape = LaxPolygon::from_loops(&refs);
        assert!(!shape.reference_point().contained);
    }

    #[test]
    fn test_inverted_loops() {
        // C++ TEST(S2LaxPolygonShape, InvertedLoops)
        // Two CW loops — together they cover most of the sphere.
        // C++ verifies ContainsBruteForce(shape, Origin()) == true.
        // In Rust, we verify via the shape_util brute-force containment check.
        let loops = [
            crate::s2::text_format::parse_points("1:2, 1:1, 2:2"),
            crate::s2::text_format::parse_points("3:4, 3:3, 4:4"),
        ];
        let refs: Vec<&[Point]> = loops.iter().map(Vec::as_slice).collect();
        let shape = LaxPolygon::from_loops(&refs);
        // Both loops are CW, which means in the interior-on-left convention,
        // they are inverted (the interior is the complement).
        assert_eq!(2, shape.num_loops());
        assert_eq!(6, shape.num_edges());
        assert!(!shape.is_empty());
        assert!(!shape.is_full());
        // The brute-force containment should report origin as contained.
        assert!(
            crate::s2::shape_util::contains_brute_force(&shape, Point::origin()),
            "origin should be contained by inverted loops"
        );
    }
}
