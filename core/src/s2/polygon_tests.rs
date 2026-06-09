// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for [`super::Polygon`] paths not exercised by the
//! in-file test module: the `Region`/`Shape` trait plumbing (`intersects_cell`,
//! `cell_union_bound`, `subregion_bound`, `type_tag`/`encode_tagged`), the
//! nested-loop body of `is_normalized`, the many-loop decode path, the
//! `compare_loops`/`loops_approx_eq` helpers, the `Debug`/`Display`/`PartialEq`/
//! `Default` impls, and several boundary-comparison early returns. Written for
//! this crate, not ported from upstream S2.

use super::*;
use crate::s2::LatLng;

fn p(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

/// A single-loop axis-aligned lat/lng rectangle.
fn rect_polygon(lat_lo: f64, lng_lo: f64, lat_hi: f64, lng_hi: f64) -> Polygon {
    Polygon::from_loops(vec![Loop::new(vec![
        p(lat_lo, lng_lo),
        p(lat_lo, lng_hi),
        p(lat_hi, lng_hi),
        p(lat_hi, lng_lo),
    ])])
}

fn cell_at(lat: f64, lng: f64, level: u8) -> Cell {
    Cell::from_cell_id(CellId::from_point(&p(lat, lng)).parent_at_level(level))
}

// ─── Region: intersects_cell ─────────────────────────────────────────────

#[test]
fn intersects_cell_false_when_bounds_disjoint() {
    // The polygon's bound does not intersect the cell's bound: short-circuit.
    let poly = rect_polygon(0.0, 0.0, 5.0, 5.0);
    let cell = cell_at(0.0, 170.0, 5);
    assert!(!poly.intersects_cell(&cell));
}

#[test]
fn intersects_cell_true_when_cell_vertex_inside_polygon() {
    // A tiny cell deep inside a large polygon: all four cell vertices are
    // contained, so the first detection strategy fires.
    let poly = rect_polygon(-40.0, -40.0, 40.0, 40.0);
    let cell = cell_at(0.0, 0.0, 12);
    assert!(poly.intersects_cell(&cell));
}

#[test]
fn intersects_cell_true_when_polygon_vertex_inside_cell() {
    // A tiny polygon inside a huge (face-level) cell. No cell vertex is inside
    // the polygon, but the polygon's vertices are inside the cell, so the
    // second detection strategy fires.
    let poly = rect_polygon(-0.5, -0.5, 0.5, 0.5);
    let cell = Cell::from_cell_id(CellId::from_face(0));
    assert!(poly.intersects_cell(&cell));
}

#[test]
fn intersects_cell_true_on_edge_crossing_only() {
    // A thin, tall bar passes vertically through a small cell: the bar's side
    // edges enter and exit the cell. The bar's longitude band is derived from
    // the cell's own geometry so it stays strictly between the cell's corners
    // (no corner inside the bar) while spanning far past it in latitude (no bar
    // vertex inside the cell). The asserts below confirm those preconditions,
    // so the only way `intersects_cell` can return true is the edge crossing.
    let cell_id = CellId::from_point(&p(0.0, 0.0)).parent_at_level(6);
    let cell = Cell::from_cell_id(cell_id);
    let center = LatLng::from_point(cell_id.to_point());
    let (clat, clng) = (center.lat.degrees(), center.lng.degrees());
    // Quarter of the distance from the center to the nearest corner longitude:
    // narrow enough to keep every corner outside the bar.
    let half_width = (0..4)
        .map(|i| (LatLng::from_point(cell.vertex(i)).lng.degrees() - clng).abs())
        .fold(f64::INFINITY, f64::min)
        * 0.25;

    let bar_verts = [
        p(clat - 30.0, clng - half_width),
        p(clat - 30.0, clng + half_width),
        p(clat + 30.0, clng + half_width),
        p(clat + 30.0, clng - half_width),
    ];
    let bar = Polygon::from_loops(vec![Loop::new(bar_verts.to_vec())]);

    for i in 0..4 {
        assert!(
            !bar.contains_point(&cell.vertex(i)),
            "precondition: cell vertex {i} must be outside the bar"
        );
    }
    for v in bar_verts {
        assert!(
            !cell.contains_point(v),
            "precondition: bar vertices must be outside the cell"
        );
    }

    assert!(bar.intersects_cell(&cell));
}

#[test]
fn intersects_cell_false_when_disjoint_within_overlapping_bounds() {
    // The SW triangle and a small cell in the NE corner share an overlapping
    // lat/lng bounding rectangle, but are geometrically disjoint. This forces
    // `intersects_cell` past the bound check, through both vertex-containment
    // scans and the full edge-crossing scan, to the final `false`.
    let tri = Polygon::from_loops(vec![Loop::new(vec![
        p(0.0, 0.0),
        p(0.0, 10.0),
        p(10.0, 0.0),
    ])]);
    let cell = cell_at(9.0, 9.0, 8);
    // Bounds overlap (the cell sits inside the triangle's bounding rectangle)…
    assert!(tri.bound().intersects(cell.rect_bound()));
    // …but the cell lies beyond the hypotenuse, so they do not intersect.
    assert!(!tri.intersects_cell(&cell));
}

// ─── Region: cell_union_bound and subregion_bound ────────────────────────

#[test]
fn cell_union_bound_is_nonempty() {
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let cells = poly.cell_union_bound();
    assert!(!cells.is_empty());
}

#[test]
fn subregion_bound_contains_interior_point() {
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let sub = poly.subregion_bound();
    assert!(sub.contains_point(p(5.0, 5.0)));
}

// ─── Shape: type_tag and encode_tagged ───────────────────────────────────

#[test]
fn shape_type_tag_is_one() {
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    assert_eq!(poly.type_tag(), 1);
}

#[test]
fn encode_tagged_roundtrips_as_polygon() {
    use crate::s2::encoded_s2point_vector::CodingHint;
    use crate::s2::encoding::S2Decode;
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let mut buf = Vec::new();
    poly.encode_tagged(&mut buf, CodingHint::Fast)
        .expect("encode_tagged");
    // encode_tagged delegates to encode(), so the bytes decode as a polygon.
    let back = Polygon::decode(&mut buf.as_slice()).expect("decode");
    assert_eq!(back.num_loops(), poly.num_loops());
    assert_eq!(back.num_vertices(), poly.num_vertices());
}

// ─── is_normalized: nested-loop body ─────────────────────────────────────

#[test]
fn is_normalized_true_for_shell_with_disjoint_hole() {
    // Outer CCW shell + inner CW hole that shares no vertices with the shell.
    let outer = Loop::new(vec![p(0.0, 0.0), p(0.0, 20.0), p(20.0, 20.0), p(20.0, 0.0)]);
    let mut inner = Loop::new(vec![p(5.0, 5.0), p(15.0, 5.0), p(15.0, 15.0), p(5.0, 15.0)]);
    inner.invert();
    let poly = Polygon::from_oriented_loops(vec![outer, inner]);
    assert_eq!(poly.num_loops(), 2);
    assert_eq!(poly.loop_at(1).depth(), 1);
    // The hole shares no vertex with its parent shell → normalized.
    assert!(poly.is_normalized());
}

#[test]
fn is_normalized_false_when_hole_shares_two_shell_vertices() {
    // A hole that reuses two of the shell's vertices is not normalized: a loop
    // and its parent sharing more than one vertex should be merged.
    let a = p(0.0, 0.0);
    let b = p(0.0, 20.0);
    let outer = Loop::new(vec![a, b, p(20.0, 20.0), p(20.0, 0.0)]);
    let mut inner = Loop::new(vec![a, b, p(10.0, 10.0)]);
    inner.invert();
    let poly = Polygon::from_oriented_loops(vec![outer, inner]);
    // Precondition: the inner loop nested as a depth-1 hole.
    assert_eq!(poly.num_loops(), 2);
    assert_eq!(poly.loop_at(1).depth(), 1);
    assert!(!poly.is_normalized());
}

// ─── from_oriented_loops / from_decoded_loops ────────────────────────────

#[test]
fn from_oriented_loops_empty_input_is_empty() {
    let poly = Polygon::from_oriented_loops(vec![]);
    assert!(poly.is_empty_polygon());
}

#[test]
fn decode_many_loops_populates_cumulative_edges() {
    use crate::s2::encoding::{S2Decode, S2Encode};
    // 13 disjoint triangles → more than 12 loops → exercises the
    // cumulative_edges fast-path in from_decoded_loops.
    let loops: Vec<Loop> = (0..13)
        .map(|i| {
            let lng = f64::from(i) * 12.0 - 70.0;
            Loop::new(vec![p(0.0, lng), p(0.0, lng + 3.0), p(3.0, lng)])
        })
        .collect();
    let poly = Polygon::from_loops(loops);
    assert_eq!(poly.num_loops(), 13);
    let mut buf = Vec::new();
    poly.encode(&mut buf).expect("encode");
    let back = Polygon::decode(&mut buf.as_slice()).expect("decode");
    assert_eq!(back.num_loops(), 13);
    assert_eq!(back.num_vertices(), poly.num_vertices());
}

// ─── compare_loops / loops_approx_eq ─────────────────────────────────────

#[test]
fn compare_loops_covers_all_orderings() {
    use std::cmp::Ordering;

    // Two fixed filler vertices; only the first vertex drives the comparisons.
    let mk = |first: Point| Loop::new(vec![first, p(40.0, 0.0), p(-40.0, 0.0)]);

    let tri = mk(p(0.0, 0.0));
    let quad = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
    // Different vertex counts: ordered by count.
    assert_eq!(compare_loops(&tri, &quad), Ordering::Less);
    assert_eq!(compare_loops(&quad, &tri), Ordering::Greater);

    // Identical loops compare Equal.
    assert_eq!(compare_loops(&tri, &mk(p(0.0, 0.0))), Ordering::Equal);

    // x differs (1 vs 2/sqrt(5) ≈ 0.894).
    let hi_x = mk(Point::from_coords(1.0, 0.0, 0.0));
    let lo_x = mk(Point::from_coords(2.0, 1.0, 0.0));
    assert_eq!(compare_loops(&hi_x, &lo_x), Ordering::Greater);
    assert_eq!(compare_loops(&lo_x, &hi_x), Ordering::Less);

    // Equal x, y differs (±1/sqrt(2)).
    let pos_y = mk(Point::from_coords(1.0, 1.0, 0.0));
    let neg_y = mk(Point::from_coords(1.0, -1.0, 0.0));
    assert_eq!(compare_loops(&pos_y, &neg_y), Ordering::Greater);
    assert_eq!(compare_loops(&neg_y, &pos_y), Ordering::Less);

    // Equal x and y, z differs (±1/sqrt(3)).
    let pos_z = mk(Point::from_coords(1.0, 1.0, 1.0));
    let neg_z = mk(Point::from_coords(1.0, 1.0, -1.0));
    assert_eq!(compare_loops(&pos_z, &neg_z), Ordering::Greater);
    assert_eq!(compare_loops(&neg_z, &pos_z), Ordering::Less);
}

#[test]
fn loops_approx_eq_false_for_mismatched_vertex_counts() {
    let tri = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 0.0)]);
    let quad = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
    assert!(!loops_approx_eq(&tri, &quad, Angle::from_degrees(1.0)));
}

#[test]
fn loops_approx_eq_matches_reversed_loop() {
    // The same quad traced in the opposite direction: no forward rotation
    // matches, so the comparison falls through to the reversed-loop pass.
    let fwd = Loop::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]);
    let rev = Loop::new(vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)]);
    assert!(loops_approx_eq(&fwd, &rev, Angle::from_degrees(0.001)));
}

// ─── Debug / Display / PartialEq / Default ───────────────────────────────

#[test]
fn debug_format_reports_loop_and_vertex_counts() {
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let s = format!("{poly:?}");
    assert!(s.contains("Polygon"));
    assert!(s.contains("num_loops"));
    assert!(s.contains("num_vertices"));
}

#[test]
fn display_renders_nonempty_text() {
    let poly = rect_polygon(0.0, 0.0, 10.0, 10.0);
    assert!(!poly.to_string().is_empty());
}

#[test]
fn partial_eq_matches_equal_method() {
    let a = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let b = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let c = rect_polygon(0.0, 0.0, 20.0, 20.0);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn default_polygon_is_empty() {
    let poly = Polygon::default();
    assert!(poly.is_empty_polygon());
    assert_eq!(poly, Polygon::empty());
}

// ─── Boundary / equality early returns for mismatched loop counts ─────────

#[test]
fn boundary_and_equality_false_for_different_loop_counts() {
    let one = rect_polygon(0.0, 0.0, 10.0, 10.0);
    let two = Polygon::from_loops(vec![
        Loop::new(vec![p(0.0, 0.0), p(0.0, 5.0), p(5.0, 0.0)]),
        Loop::new(vec![p(20.0, 20.0), p(20.0, 25.0), p(25.0, 20.0)]),
    ]);
    let tol = Angle::from_degrees(1.0);
    assert!(!one.boundary_approx_eq(&two, tol));
    assert!(!one.boundary_near(&two, tol));
    assert!(!one.equal(&two));
    assert!(!one.boundary_equals(&two));
}

// ─── Shape contract: interior on the left of every edge ───────────────────
//
// Generalized guards for the hole-orientation fix (BUG.md §2, Phase 5): the
// polygon Shape views must present hole-loop edges reversed (C++
// `oriented_vertex` semantics) so the polygon interior is on the left of
// every edge, and the two Shape impls (`Shape for Polygon` and the internal
// indexed `PolygonShape`) must agree exactly.

fn rect_loop(lat_lo: f64, lng_lo: f64, lat_hi: f64, lng_hi: f64) -> Loop {
    Loop::new(vec![
        p(lat_lo, lng_lo),
        p(lat_lo, lng_hi),
        p(lat_hi, lng_hi),
        p(lat_hi, lng_lo),
    ])
}

/// A set of polygons exercising every loop role: plain shell, shell+hole,
/// shell with two holes, and a nested shell/hole/island (depths 0/1/2).
fn shape_contract_polygons() -> Vec<Polygon> {
    vec![
        rect_polygon(0.0, 0.0, 20.0, 20.0),
        Polygon::from_loops(vec![
            rect_loop(0.0, 0.0, 20.0, 20.0),
            rect_loop(4.0, 4.0, 16.0, 16.0),
        ]),
        Polygon::from_loops(vec![
            rect_loop(0.0, 0.0, 20.0, 20.0),
            rect_loop(2.0, 2.0, 8.0, 8.0),
            rect_loop(11.0, 11.0, 18.0, 17.0),
        ]),
        Polygon::from_loops(vec![
            rect_loop(0.0, 0.0, 20.0, 20.0),
            rect_loop(3.0, 3.0, 17.0, 17.0),
            rect_loop(7.0, 7.0, 13.0, 13.0),
        ]),
    ]
}

/// Asserts the Shape contract for one polygon view: every edge has the
/// polygon interior strictly on its left, each chain is a closed cycle, and
/// `chain_edge` is consistent with `edge`.
fn check_shape_contract(poly: &Polygon, shape: &dyn Shape) {
    assert_eq!(shape.num_edges(), poly.num_vertices());
    assert_eq!(shape.num_chains(), poly.num_loops());

    for chain_id in 0..shape.num_chains() {
        let chain = shape.chain(chain_id);
        for offset in 0..chain.length {
            let e = shape.edge(chain.start + offset);

            // chain_edge agrees with the flat edge numbering.
            let ce = shape.chain_edge(chain_id, offset);
            assert_eq!(e.v0, ce.v0);
            assert_eq!(e.v1, ce.v1);

            // The chain is a closed cycle: this edge ends where the next
            // (cyclically) begins.
            let next = shape.chain_edge(chain_id, (offset + 1) % chain.length);
            assert_eq!(e.v1, next.v0, "chain {chain_id} broken at {offset}");

            // Interior-on-left: displace the edge midpoint slightly toward
            // the left of the directed edge (n = v0 × v1 points left) and
            // slightly toward the right; only the left probe is contained.
            // Test geometry keeps all boundaries >= 1 degree apart, so a
            // ~0.06 degree displacement cannot reach another boundary.
            let n = e.v0.0.cross(e.v1.0).normalize();
            let m = (e.v0.0 + e.v1.0).normalize();
            let eps = 1e-3;
            let left = Point::from_coords(m.x + eps * n.x, m.y + eps * n.y, m.z + eps * n.z);
            let right = Point::from_coords(m.x - eps * n.x, m.y - eps * n.y, m.z - eps * n.z);
            assert!(
                poly.contains_point(&left),
                "interior must be on the LEFT of chain {chain_id} edge {offset}"
            );
            assert!(
                !poly.contains_point(&right),
                "exterior must be on the RIGHT of chain {chain_id} edge {offset}"
            );
        }
    }
}

#[test]
fn shape_edges_keep_interior_on_left_for_holes_and_islands() {
    for poly in shape_contract_polygons() {
        check_shape_contract(&poly, &poly);
    }
}

#[test]
fn indexed_polygon_shape_satisfies_the_same_contract() {
    for poly in shape_contract_polygons() {
        let indexed = PolygonShape::from_polygon(&poly);
        check_shape_contract(&poly, &indexed);
    }
}

#[test]
fn polygon_shape_impls_agree_edge_for_edge() {
    for poly in shape_contract_polygons() {
        let indexed = PolygonShape::from_polygon(&poly);
        assert_eq!(poly.num_edges(), indexed.num_edges());
        assert_eq!(poly.num_chains(), indexed.num_chains());
        assert_eq!(poly.dimension(), indexed.dimension());
        assert_eq!(
            poly.reference_point().contained,
            indexed.reference_point().contained
        );
        for e in 0..poly.num_edges() {
            let a = poly.edge(e);
            let b = indexed.edge(e);
            assert_eq!(a.v0, b.v0, "edge {e} v0 differs between Shape impls");
            assert_eq!(a.v1, b.v1, "edge {e} v1 differs between Shape impls");
            assert_eq!(
                poly.chain_position(e).chain_id,
                indexed.chain_position(e).chain_id
            );
            assert_eq!(
                poly.chain_position(e).offset,
                indexed.chain_position(e).offset
            );
        }
    }
}

/// Randomized version: grid-aligned shells with 0–2 disjoint holes. All
/// coordinates are >= 1 degree apart, keeping the left/right probes valid.
#[test]
fn shape_contract_holds_for_randomized_polygons_with_holes() {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(0x5333_7001);
    for _ in 0..50 {
        let lat0 = f64::from(rng.gen_range(-40..20i32));
        let lng0 = f64::from(rng.gen_range(-60..40i32));
        let h = f64::from(rng.gen_range(12..28i32));
        let w = f64::from(rng.gen_range(12..28i32));
        let mut loops = vec![rect_loop(lat0, lng0, lat0 + h, lng0 + w)];

        let num_holes = rng.gen_range(0..=2u32);
        if num_holes >= 1 {
            // First hole in the SW quadrant of the shell interior.
            let s = f64::from(rng.gen_range(2..5i32));
            loops.push(rect_loop(
                lat0 + 1.0,
                lng0 + 1.0,
                lat0 + 1.0 + s,
                lng0 + 1.0 + s,
            ));
        }
        if num_holes == 2 {
            // Second hole in the NE quadrant, disjoint from the first.
            let s = f64::from(rng.gen_range(2..5i32));
            loops.push(rect_loop(
                lat0 + h - 1.0 - s,
                lng0 + w - 1.0 - s,
                lat0 + h - 1.0,
                lng0 + w - 1.0,
            ));
        }

        let poly = Polygon::from_loops(loops);
        check_shape_contract(&poly, &poly);
        let indexed = PolygonShape::from_polygon(&poly);
        check_shape_contract(&poly, &indexed);
    }
}

// ─── Boolean-op generalizations (BUG.md §2 fixes) ─────────────────────────

/// Generalizes C++ `IntersectionPreservesLoopOrder` (and the lexicon
/// singleton-encoding fix behind it): intersecting a polygon with a container
/// that strictly contains it must reproduce the polygon *exactly* — same
/// loops, same order, same starting vertex per loop. Exactness relies on the
/// boolean-op output carrying input-edge attribution so that
/// `Graph::canonicalize_loop_order` restores the input rotation.
#[test]
fn intersection_with_container_preserves_polygon_exactly() {
    use crate::s2::text_format::polygon_to_string;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(0x1337_c0de);

    let container = rect_polygon(-40.0, -60.0, 40.0, 60.0);
    for _ in 0..40 {
        let lat0 = f64::from(rng.gen_range(-30..0i32));
        let lng0 = f64::from(rng.gen_range(-50..20i32));
        let h = f64::from(rng.gen_range(10..25i32));
        let w = f64::from(rng.gen_range(10..25i32));
        let mut loops = vec![rect_loop(lat0, lng0, lat0 + h, lng0 + w)];
        // Up to two holes with distinct sizes (keeps loop ordering by area
        // deterministic).
        if rng.gen_bool(0.8) {
            loops.push(rect_loop(lat0 + 1.0, lng0 + 1.0, lat0 + 4.0, lng0 + 5.0));
        }
        if rng.gen_bool(0.5) {
            loops.push(rect_loop(
                lat0 + h - 3.0,
                lng0 + w - 3.0,
                lat0 + h - 1.0,
                lng0 + w - 1.0,
            ));
        }
        let poly = Polygon::from_loops(loops);

        let result = Polygon::intersection(&mut poly.clone(), &mut container.clone());
        assert_eq!(
            polygon_to_string(&poly),
            polygon_to_string(&result),
            "intersection with a container must preserve the polygon exactly"
        );
    }
}

/// Generalizes the robust-predicate fix: boolean ops on randomized
/// near-degenerate sliver pairs (vertex separations down to ~1e-15, like the
/// `test_bug*` inputs) must uphold the GraphEdgeClipper invariants — in debug
/// builds the `multiplicity`/`lo <= hi` debug_asserts are active, so simply
/// completing without panicking is the check — and any non-empty output must
/// be a valid polygon.
#[test]
fn boolean_ops_on_near_degenerate_slivers_uphold_invariants() {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(0xde9e_4e7a);

    for iter in 0..150 {
        // Random base point and tangent frame.
        let base = LatLng::from_degrees(
            rng.gen_range(-80.0..80.0f64),
            rng.gen_range(-179.0..179.0f64),
        )
        .to_point();
        let e1 = crate::s2::ortho(base).0.normalize();
        let e2 = base.0.cross(e1).normalize();

        // A thin sliver triangle: long direction s, thin direction t.
        let s = 10f64.powf(rng.gen_range(-7.0..-4.0f64));
        let t = s * 10f64.powf(rng.gen_range(-8.0..-2.0f64));
        let sliver = |dx: f64, dy: f64| {
            let mk = |a: f64, b: f64| {
                let v = base.0 + e1 * (a + dx) + e2 * (b + dy);
                Point::from_coords(v.x, v.y, v.z)
            };
            Loop::new(vec![mk(0.0, 0.0), mk(s, 0.0), mk(s * 0.5, t)])
        };

        // Second sliver: nearly coincident with the first — offset by a few
        // ULP-scale increments, mimicking the regression inputs.
        let jitter = 10f64.powf(rng.gen_range(-15.5..-13.0f64));
        let a = Polygon::from_loops(vec![sliver(0.0, 0.0)]);
        let b = Polygon::from_loops(vec![sliver(jitter, -jitter)]);

        for op in 0..4 {
            let mut x = a.clone();
            let mut y = b.clone();
            let out = match op {
                0 => Polygon::union(&mut x, &mut y),
                1 => Polygon::intersection(&mut x, &mut y),
                2 => Polygon::difference(&mut x, &mut y),
                _ => Polygon::symmetric_difference(&mut x, &mut y),
            };
            if !out.is_empty_polygon() {
                assert!(
                    out.find_validation_error().is_none(),
                    "iter {iter} op {op}: boolean op produced an invalid polygon"
                );
            }
        }
    }
}
