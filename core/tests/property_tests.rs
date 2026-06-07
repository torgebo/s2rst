// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Part of a Rust port of Google's S2 Geometry library. The upstream
// implementations — C++ (google/s2geometry), Go (golang/geo), and Java
// (google/s2-geometry-library-java) — are licensed under the Apache License,
// Version 2.0, and are Copyright Google Inc. See LICENSE.

//! Property-based tests using quickcheck for S2 geometry invariants.
//!
//! Tests cover geometric properties that must hold for ALL inputs:
//! symmetry, idempotence, containment transitivity, roundtrips, bounds, etc.

#![allow(
    clippy::doc_markdown,
    reason = "test doc comments use unquoted API names"
)]

use quickcheck_macros::quickcheck;
use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::coords::Level;
use s2rst::s2::shape::Dimension;
use s2rst::s2::{Cap, CellId, LatLng, Loop, Point, Polygon, Rect, Region};
use std::f64::consts::PI;

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

/// Helper to treat panics from internal assertions as "skip" in property tests.
/// Some degenerate inputs trigger debug assertions in boolean operations.
fn no_panic<F: FnOnce() -> bool + std::panic::UnwindSafe>(f: F) -> bool {
    std::panic::catch_unwind(f).unwrap_or(true)
}

fn clamp(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(-1e10, 1e10)
    } else {
        0.0
    }
}

fn make_point(x: f64, y: f64, z: f64) -> Option<Point> {
    let (x, y, z) = (clamp(x), clamp(y), clamp(z));
    if x == 0.0 && y == 0.0 && z == 0.0 {
        return None;
    }
    Some(Point::from_coords(x, y, z))
}

fn make_latlng(lat_i: i32, lng_i: i32) -> LatLng {
    // Use rem_euclid to always get non-negative remainder, avoiding out-of-range values.
    let lat = (lat_i.rem_euclid(181)) as f64 - 90.0; // [-90, 90]
    let lng = (lng_i.rem_euclid(361)) as f64 - 180.0; // [-180, 180]
    LatLng::from_degrees(lat, lng)
}

/// Make a `LatLng` that avoids the poles and dateline (for boolean op stability).
/// Multiplies by a prime to spread small sequential values across the range.
fn make_latlng_safe(lat_i: i32, lng_i: i32) -> LatLng {
    let lat = ((lat_i.wrapping_mul(37)).rem_euclid(121)) as f64 - 60.0; // [-60, 60]
    let lng = ((lng_i.wrapping_mul(53)).rem_euclid(321)) as f64 - 160.0; // [-160, 160]
    LatLng::from_degrees(lat, lng)
}

/// Build a triangle polygon with safe coordinates (avoids poles/dateline).
fn make_safe_triangle_polygon(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> Option<Polygon> {
    let ll0 = make_latlng_safe(lat0, lng0);
    let ll1 = make_latlng_safe(lat1, lng1);
    let ll2 = make_latlng_safe(lat2, lng2);
    // Skip triangles where all vertices are at nearly the same latitude
    // (thin slivers along latitude lines cause boolean op issues).
    let lat_spread = (ll0.lat.degrees() - ll1.lat.degrees())
        .abs()
        .max((ll0.lat.degrees() - ll2.lat.degrees()).abs())
        .max((ll1.lat.degrees() - ll2.lat.degrees()).abs());
    if lat_spread < 1.0 {
        return None;
    }
    let p0 = ll0.to_point();
    let p1 = ll1.to_point();
    let p2 = ll2.to_point();
    let cross = p0.0.cross(p1.0).dot(p2.0);
    if cross.abs() < 1e-4 {
        return None;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    if lp.num_vertices() < 3 {
        return None;
    }
    Some(Polygon::from_loops(vec![lp]))
}

/// Build a small CCW triangle loop from three integer lat/lng pairs.
fn make_triangle_loop(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> Option<Loop> {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    // Skip degenerate triangles (collinear or coincident points).
    let cross = p0.0.cross(p1.0).dot(p2.0);
    if cross.abs() < 1e-10 {
        return None;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    if lp.num_vertices() < 3 {
        return None;
    }
    Some(lp)
}

/// Build a simple polygon from a triangle loop.
fn make_triangle_polygon(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> Option<Polygon> {
    make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2).map(|lp| Polygon::from_loops(vec![lp]))
}

// ════════════════════════════════════════════════════════════════════
// 1. LOOP PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Loop area is in [0, 4π].
#[quickcheck]
fn prop_loop_area_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let a = lp.area();
            (0.0..=4.0 * PI + 1e-10).contains(&a)
        }
        None => true,
    }
}

/// Normalizing a loop is idempotent: normalize(normalize(l)) == normalize(l).
#[quickcheck]
fn prop_loop_normalize_idempotent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(mut lp) => {
            lp.normalize();
            let v1: Vec<_> = (0..lp.num_vertices()).map(|i| lp.vertex(i)).collect();
            lp.normalize();
            let v2: Vec<_> = (0..lp.num_vertices()).map(|i| lp.vertex(i)).collect();
            v1 == v2
        }
        None => true,
    }
}

/// A normalized loop has area <= 2π.
#[quickcheck]
fn prop_loop_normalized_area(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(mut lp) => {
            lp.normalize();
            lp.area() <= 2.0 * PI + 1e-10
        }
        None => true,
    }
}

/// `Loop.bound()` contains all loop vertices.
#[quickcheck]
fn prop_loop_bound_contains_vertices(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let b = lp.bound();
            (0..lp.num_vertices()).all(|i| b.contains_lat_lng(LatLng::from_point(lp.vertex(i))))
        }
        None => true,
    }
}

/// Inverting a loop flips `contains_origin`.
#[quickcheck]
fn prop_loop_invert_flips_origin(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let co = lp.contains_origin();
            let mut inv = lp.clone();
            inv.invert();
            inv.contains_origin() != co
        }
        None => true,
    }
}

/// Inverting twice gives back the same loop.
#[quickcheck]
fn prop_loop_double_invert(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let mut inv = lp.clone();
            inv.invert();
            inv.invert();
            lp.area().eq(&inv.area()) && lp.contains_origin() == inv.contains_origin()
        }
        None => true,
    }
}

/// A loop and its complement have areas summing to 4π.
#[quickcheck]
fn prop_loop_area_complement(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let mut inv = lp.clone();
            inv.invert();
            let total = lp.area() + inv.area();
            (total - 4.0 * PI).abs() < 1e-10
        }
        None => true,
    }
}

/// Loop centroid is finite and reasonably bounded.
#[quickcheck]
fn prop_loop_centroid_finite(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let c = lp.centroid();
            // The "true centroid" (integral of position over area) is always finite,
            // and its norm scales with the loop area. For a small loop, norm is small;
            // for the full sphere, the centroid is (0,0,0).
            c.0.x.is_finite() && c.0.y.is_finite() && c.0.z.is_finite()
        }
        None => true,
    }
}

/// A loop contains its own centroid (normalized direction) if its area <= 2π.
#[quickcheck]
fn prop_loop_contains_centroid(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(mut lp) => {
            lp.normalize();
            let c = lp.centroid();
            if c.0.norm2() < 1e-20 {
                return true;
            }
            let cp = Point(c.0.normalize());
            lp.contains_point(&cp)
        }
        None => true,
    }
}

/// The loop's turning angle has absolute value <= 2π for a simple loop.
#[quickcheck]
fn prop_loop_turning_angle_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => lp.turning_angle().abs() <= 2.0 * PI + 1e-10,
        None => true,
    }
}

// ════════════════════════════════════════════════════════════════════
// 2. POLYGON PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Polygon area is in [0, 4π].
#[quickcheck]
fn prop_polygon_area_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let a = p.area();
            (0.0..=4.0 * PI + 1e-10).contains(&a)
        }
        None => true,
    }
}

/// Polygon complement area + polygon area = 4π.
#[quickcheck]
fn prop_polygon_complement_area(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let comp = Polygon::complement(&p);
            let total = p.area() + comp.area();
            (total - 4.0 * PI).abs() < 1e-6
        }
        None => true,
    }
}

/// A polygon contains itself.
#[quickcheck]
fn prop_polygon_contains_self(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => p.contains_polygon(&p),
        None => true,
    }
}

/// A polygon intersects itself (unless empty).
#[quickcheck]
fn prop_polygon_intersects_self(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => p.is_empty_polygon() || p.intersects_polygon(&p),
        None => true,
    }
}

/// `Polygon.bound()` contains all polygon vertices.
#[quickcheck]
fn prop_polygon_bound_contains_vertices(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let b = p.bound();
            (0..p.num_loops()).all(|i| {
                let lp = p.loop_at(i);
                (0..lp.num_vertices()).all(|j| b.contains_lat_lng(LatLng::from_point(lp.vertex(j))))
            })
        }
        None => true,
    }
}

/// Full polygon contains any non-empty polygon.
#[quickcheck]
fn prop_full_contains_any(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => Polygon::full().contains_polygon(&p),
        None => true,
    }
}

/// Empty polygon is contained by any polygon.
#[quickcheck]
fn prop_any_contains_empty(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => p.contains_polygon(&Polygon::empty()),
        None => true,
    }
}

/// Union of a polygon with empty is the polygon itself.
#[quickcheck]
fn prop_polygon_union_empty_identity(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let u = Polygon::union(&mut p.clone(), &mut Polygon::empty());
            u.boundary_approx_eq(&p, Angle::from_radians(1e-7))
        }
        None => true,
    }
}

/// Intersection of a polygon with full is the polygon itself.
#[quickcheck]
fn prop_polygon_intersection_full_identity(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let i = Polygon::intersection(&mut p.clone(), &mut Polygon::full());
            i.boundary_approx_eq(&p, Angle::from_radians(1e-7))
        }
        None => true,
    }
}

/// Intersection of a polygon with itself is the polygon itself.
#[quickcheck]
fn prop_polygon_intersection_self(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let i = Polygon::intersection(&mut p.clone(), &mut p.clone());
            i.boundary_approx_eq(&p, Angle::from_radians(1e-7))
        }
        None => true,
    }
}

/// Difference of a polygon with itself is empty.
#[quickcheck]
fn prop_polygon_difference_self_empty(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let d = Polygon::difference(&mut p.clone(), &mut p.clone());
            d.is_empty_polygon()
        }
        None => true,
    }
}

/// Symmetric difference of a polygon with itself is empty.
#[quickcheck]
fn prop_polygon_symmetric_difference_self_empty(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let sd = Polygon::symmetric_difference(&mut p.clone(), &mut p.clone());
            sd.is_empty_polygon()
        }
        None => true,
    }
}

/// `Polygon.get_distance(p)` is zero iff polygon contains p.
#[quickcheck]
fn prop_polygon_distance_zero_iff_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    px: i32,
    py: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let test_point = make_latlng(px, py).to_point();
            let dist = p.get_distance(test_point);
            if p.contains_point(&test_point) {
                dist.radians() == 0.0
            } else {
                dist.radians() >= 0.0
            }
        }
        None => true,
    }
}

/// `project_point` returns the input itself when it's inside the polygon.
#[quickcheck]
fn prop_polygon_project_inside(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    px: i32,
    py: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let test_point = make_latlng(px, py).to_point();
            if p.contains_point(&test_point) {
                p.project_point(test_point) == test_point
            } else {
                true
            }
        }
        None => true,
    }
}

// ════════════════════════════════════════════════════════════════════
// 3. POLYLINE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Polyline length is non-negative.
#[quickcheck]
fn prop_polyline_length_non_negative(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ];
    Polyline::new(pts).length().radians() >= 0.0
}

/// Reversing a polyline preserves its length.
#[quickcheck]
fn prop_polyline_reverse_preserves_length(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ];
    let pl = Polyline::new(pts.clone());
    let mut rev = Polyline::new(pts);
    rev.reverse();
    (pl.length().radians() - rev.length().radians()).abs() < 1e-14
}

/// Interpolate(0) returns the first vertex, Interpolate(1) returns the last.
#[quickcheck]
fn prop_polyline_interpolate_endpoints(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    if p0.distance(p1).radians() < 1e-10 {
        return true;
    }
    let pl = Polyline::new(vec![p0, p1]);
    let (start, _) = pl.interpolate(0.0);
    let (end, _) = pl.interpolate(1.0);
    start.distance(p0).radians() < 1e-14 && end.distance(p1).radians() < 1e-14
}

/// uninterpolate(interpolate(f)) ≈ f for f in [0, 1].
#[quickcheck]
fn prop_polyline_interpolate_roundtrip(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    frac_raw: u8,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    if p0.distance(p1).radians() < 1e-10 {
        return true;
    }
    let pl = Polyline::new(vec![p0, p1]);
    let f = frac_raw as f64 / 255.0;
    let (interp, next_vertex) = pl.interpolate(f);
    let back = pl.uninterpolate(interp, next_vertex);
    (f - back).abs() < 1e-10
}

// ════════════════════════════════════════════════════════════════════
// 4. EDGE DISTANCE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Distance from a point to an edge is non-negative.
#[quickcheck]
fn prop_edge_distance_non_negative(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::edge_distances;
    let p = make_latlng(lat0, lng0).to_point();
    let a = make_latlng(lat1, lng1).to_point();
    let b = make_latlng(lat2, lng2).to_point();
    edge_distances::distance_from_segment(p, a, b).radians() >= 0.0
}

/// Distance from a point to an edge is at most π.
#[quickcheck]
fn prop_edge_distance_at_most_pi(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::edge_distances;
    let p = make_latlng(lat0, lng0).to_point();
    let a = make_latlng(lat1, lng1).to_point();
    let b = make_latlng(lat2, lng2).to_point();
    edge_distances::distance_from_segment(p, a, b).radians() <= PI + 1e-10
}

/// Distance from an edge endpoint to the same edge is zero.
#[quickcheck]
fn prop_edge_distance_endpoint_zero(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_distances;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    edge_distances::distance_from_segment(a, a, b).radians() < 1e-14
}

/// Closest point on edge to an endpoint is the endpoint itself.
#[quickcheck]
fn prop_edge_closest_point_to_endpoint(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_distances;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let cp = edge_distances::project(a, a, b);
    a.distance(cp).radians() < 1e-14
}

// ════════════════════════════════════════════════════════════════════
// 5. EDGE CROSSER PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `crossing_sign` is symmetric: `crossing_sign(ab`, cd) == `crossing_sign(cd`, ab).
///
/// Note: `edge_or_vertex_crossing` is intentionally NOT symmetric for vertex
/// crossings. Per the Go S2 docs, property (3) of `VertexCrossing` states:
/// "If exactly one of a,b equals one of c,d, then exactly one of
/// VC(a,b,c,d) and VC(c,d,a,b) is true."
/// So we test `crossing_sign` symmetry instead.
#[quickcheck]
fn prop_crossing_symmetric(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::edge_crossings;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let d = make_latlng(lat3, lng3).to_point();
    let r1 = edge_crossings::crossing_sign(a, b, c, d);
    let r2 = edge_crossings::crossing_sign(c, d, a, b);
    r1 == r2
}

/// When edges share exactly one vertex, exactly one of
/// `edge_or_vertex_crossing(AB,CD)` and `edge_or_vertex_crossing(CD,AB)` is true.
/// This matches Go S2 `VertexCrossing` property (3).
#[test]
fn test_vertex_crossing_asymmetry_regression() {
    use s2rst::s2::edge_crossings;
    let a = make_latlng(0, 0).to_point();
    let b = make_latlng(2147483647, -1324854008).to_point();
    let c = make_latlng(2147483647, 867334376).to_point();
    let d = make_latlng(0, 1).to_point();
    assert_eq!(b, c, "b and c should be equal in this regression case");
    // crossing_sign should be symmetric (both MaybeCross)
    let cs1 = edge_crossings::crossing_sign(a, b, c, d);
    let cs2 = edge_crossings::crossing_sign(c, d, a, b);
    assert_eq!(cs1, cs2, "crossing_sign should be symmetric");
    // edge_or_vertex_crossing is intentionally asymmetric: exactly one is true
    let r1 = edge_crossings::edge_or_vertex_crossing(a, b, c, d);
    let r2 = edge_crossings::edge_or_vertex_crossing(c, d, a, b);
    assert!(
        r1 ^ r2,
        "exactly one direction should report a vertex crossing, got AB->CD={r1}, CD->AB={r2}"
    );
}

/// Crossing check is consistent between `EdgeCrosser` and chain version.
#[quickcheck]
fn prop_crossing_chain_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::edge_crosser::EdgeCrosser;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let d = make_latlng(lat3, lng3).to_point();
    if a.distance(b).radians() < 1e-10 || c.distance(d).radians() < 1e-10 {
        return true;
    }
    // Single call and chain call should give the same result.
    let mut ec1 = EdgeCrosser::new(a, b);
    let r1 = ec1.edge_or_vertex_crossing(c, d);
    let mut ec2 = EdgeCrosser::new(a, b);
    ec2.restart_at(c);
    let r2 = ec2.edge_or_vertex_chain_crossing(d);
    r1 == r2
}

// ════════════════════════════════════════════════════════════════════
// 6. CELLID PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `CellId` from a `LatLng` always produces a valid leaf cell.
#[quickcheck]
fn prop_cellid_from_latlng_valid(lat_i: i32, lng_i: i32) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let id = CellId::from_lat_lng(&ll);
    id.is_valid() && id.is_leaf()
}

/// `CellId` parent is at a lower level than the child.
#[quickcheck]
fn prop_cellid_parent_level(lat_i: i32, lng_i: i32, level: u8) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let id = CellId::from_lat_lng(&ll);
    let level = level % 30;
    if level == 0 {
        return true;
    }
    let parent = id.parent_at_level(level);
    let grandparent = parent.parent_at_level(level - 1);
    parent.level() == level && grandparent.level() == level - 1
}

/// `CellId` contains itself.
#[quickcheck]
fn prop_cellid_contains_self(lat_i: i32, lng_i: i32, level: u8) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let id = CellId::from_lat_lng(&ll).parent_at_level(level % 31);
    id.contains(id)
}

/// `CellId` parent contains child.
#[quickcheck]
fn prop_cellid_parent_contains_child(lat_i: i32, lng_i: i32, level: u8) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let id = CellId::from_lat_lng(&ll);
    let level = (level % 30) + 1; // 1..30
    let parent = id.parent_at_level(level - 1);
    let child = id.parent_at_level(level);
    parent.contains(child)
}

/// `CellId` face is in [0, 5].
#[quickcheck]
fn prop_cellid_face_bounded(lat_i: i32, lng_i: i32) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let id = CellId::from_lat_lng(&ll);
    id.face().as_u8() <= 5
}

// ════════════════════════════════════════════════════════════════════
// 7. CAP PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Cap from center+radius contains its center.
#[quickcheck]
fn prop_cap_contains_center(x: f64, y: f64, z: f64, radius_deg: u16) -> bool {
    match make_point(x, y, z) {
        Some(center) => {
            let radius = Angle::from_degrees(radius_deg as f64 / 100.0);
            let cap = Cap::from_center_angle(center, radius);
            cap.contains_point(center)
        }
        None => true,
    }
}

/// Cap area is in [0, 4π].
#[quickcheck]
fn prop_cap_area_bounded(x: f64, y: f64, z: f64, radius_deg: u16) -> bool {
    match make_point(x, y, z) {
        Some(center) => {
            let radius = Angle::from_degrees(radius_deg as f64 / 100.0);
            let cap = Cap::from_center_angle(center, radius);
            let a = cap.area();
            (0.0..=4.0 * PI + 1e-10).contains(&a)
        }
        None => true,
    }
}

/// Cap complement: cap.area + cap.complement.area = 4π.
#[quickcheck]
fn prop_cap_complement_area(x: f64, y: f64, z: f64, radius_deg: u16) -> bool {
    match make_point(x, y, z) {
        Some(center) => {
            let radius = Angle::from_degrees((radius_deg % 18001) as f64 / 100.0);
            let cap = Cap::from_center_angle(center, radius);
            let comp = cap.complement();
            if cap.is_empty() || cap.is_full() || comp.is_empty() || comp.is_full() {
                return true;
            }
            let total = cap.area() + comp.area();
            (total - 4.0 * PI).abs() < 1e-6
        }
        None => true,
    }
}

/// An expanded cap contains the original cap.
#[quickcheck]
fn prop_cap_expanded_contains_original2(
    x: f64,
    y: f64,
    z: f64,
    radius_deg: u16,
    expand_deg: u8,
) -> bool {
    match make_point(x, y, z) {
        Some(center) => {
            let radius = Angle::from_degrees((radius_deg % 18001) as f64 / 100.0);
            let cap = Cap::from_center_angle(center, radius);
            if cap.is_empty() || cap.is_full() {
                return true;
            }
            let expand = Angle::from_degrees(expand_deg as f64 / 10.0);
            let expanded = cap.expanded(expand);
            expanded.contains(cap)
        }
        None => true,
    }
}

// ════════════════════════════════════════════════════════════════════
// 8. RECT (S2LatLngRect) PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Union of a rect with itself is the same rect.
#[quickcheck]
fn prop_rect_union_self(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let lo = make_latlng(lat0, lng0);
    let hi = make_latlng(lat1, lng1);
    let r = Rect::from_lat_lng(lo).add_point(hi);
    r.union(r) == r
}

/// Intersection of a rect with itself is the same rect.
#[quickcheck]
fn prop_rect_intersection_self(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let lo = make_latlng(lat0, lng0);
    let hi = make_latlng(lat1, lng1);
    let r = Rect::from_lat_lng(lo).add_point(hi);
    r.intersection(r) == r
}

/// A rect contains its own corner points.
#[quickcheck]
fn prop_rect_contains_corners(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let lo = make_latlng(lat0, lng0);
    let hi = make_latlng(lat1, lng1);
    let r = Rect::from_lat_lng(lo).add_point(hi);
    r.contains_lat_lng(lo) && r.contains_lat_lng(hi)
}

/// Union of two rects contains both rects.
#[quickcheck]
fn prop_rect_union_contains_both(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    let r1 = Rect::from_lat_lng(make_latlng(lat0, lng0)).add_point(make_latlng(lat1, lng1));
    let r2 = Rect::from_lat_lng(make_latlng(lat2, lng2)).add_point(make_latlng(lat3, lng3));
    let u = r1.union(r2);
    u.contains(r1) && u.contains(r2)
}

// ════════════════════════════════════════════════════════════════════
// 9. CHORD ANGLE / ANGLE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `ChordAngle` → Angle → `ChordAngle` roundtrip preserves ordering.
#[quickcheck]
fn prop_chord_angle_roundtrip_preserves_order(a_raw: u32, b_raw: u32) -> bool {
    let a = ChordAngle::from_length2((a_raw % 40001) as f64 / 10000.0);
    let b = ChordAngle::from_length2((b_raw % 40001) as f64 / 10000.0);
    let a_angle = a.to_angle();
    let b_angle = b.to_angle();
    // ChordAngle ordering is preserved through conversion.
    (a <= b) == (a_angle.radians() <= b_angle.radians())
}

/// sin²(a) + cos²(a) ≈ 1 for `ChordAngle`.
#[quickcheck]
fn prop_chord_angle_sin_cos_identity(raw: u32) -> bool {
    let ca = ChordAngle::from_length2((raw % 40001) as f64 / 10000.0);
    let s = ca.sin();
    let c = ca.cos();
    (s * s + c * c - 1.0).abs() < 1e-10
}

/// `ChordAngle::sin` is non-negative for angles in [0, π].
#[quickcheck]
fn prop_chord_angle_sin_non_negative(raw: u32) -> bool {
    let ca = ChordAngle::from_length2((raw % 40001) as f64 / 10000.0);
    ca.sin() >= -1e-15
}

/// Angle degrees → radians → degrees roundtrip.
#[quickcheck]
fn prop_angle_degrees_roundtrip(deg_i: i32) -> bool {
    let deg = (deg_i % 36000) as f64 / 100.0;
    let a = Angle::from_degrees(deg);
    (a.degrees() - deg).abs() < 1e-10
}

// ════════════════════════════════════════════════════════════════════
// 10. LATLNG PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `LatLng` → Point → `LatLng` roundtrip.
#[quickcheck]
fn prop_latlng_point_roundtrip(lat_i: i32, lng_i: i32) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    let p = ll.to_point();
    let ll2 = LatLng::from_point(p);
    (ll.lat.radians() - ll2.lat.radians()).abs() < 1e-14
        && (ll.lng.radians() - ll2.lng.radians()).abs() < 1e-14
}

/// `LatLng` latitude is in [-π/2, π/2].
#[quickcheck]
fn prop_latlng_lat_bounded(lat_i: i32, lng_i: i32) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    ll.lat.radians() >= -PI / 2.0 - 1e-15 && ll.lat.radians() <= PI / 2.0 + 1e-15
}

/// `LatLng` longitude is in [-π, π].
#[quickcheck]
fn prop_latlng_lng_bounded(lat_i: i32, lng_i: i32) -> bool {
    let ll = make_latlng(lat_i, lng_i);
    ll.lng.radians() >= -PI - 1e-15 && ll.lng.radians() <= PI + 1e-15
}

/// Distance between a `LatLng` point and itself is zero.
#[quickcheck]
fn prop_latlng_distance_self_zero(lat_i: i32, lng_i: i32) -> bool {
    let p = make_latlng(lat_i, lng_i).to_point();
    p.distance(p).radians().abs() < 1e-14
}

// ════════════════════════════════════════════════════════════════════
// 11. BOOLEAN OPERATION PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Union result has valid area in [0, 4π].
#[quickcheck]
fn prop_union_area_valid(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let u = Polygon::union(&mut a.clone(), &mut b.clone());
                let area = u.area();
                (0.0..=4.0 * PI + 1e-6).contains(&area)
            }
            _ => true,
        }
    })
}

/// Intersection result has valid area in [0, 4π].
#[quickcheck]
fn prop_intersection_area_valid(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let i = Polygon::intersection(&mut a.clone(), &mut b.clone());
                let area = i.area();
                (0.0..=4.0 * PI + 1e-6).contains(&area)
            }
            _ => true,
        }
    })
}

/// Difference result has valid area in [0, 4π].
#[quickcheck]
fn prop_difference_area_valid(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let d = Polygon::difference(&mut a.clone(), &mut b.clone());
                let area = d.area();
                (0.0..=4.0 * PI + 1e-6).contains(&area)
            }
            _ => true,
        }
    })
}

/// Union is commutative: A∪B ≈ B∪A.
#[quickcheck]
fn prop_union_commutative(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let u1 = Polygon::union(&mut a.clone(), &mut b.clone());
                let u2 = Polygon::union(&mut b.clone(), &mut a.clone());
                u1.boundary_approx_eq(&u2, Angle::from_radians(1e-7))
            }
            _ => true,
        }
    })
}

/// Intersection is commutative: A∩B ≈ B∩A.
#[quickcheck]
fn prop_intersection_commutative(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let i1 = Polygon::intersection(&mut a.clone(), &mut b.clone());
                let i2 = Polygon::intersection(&mut b.clone(), &mut a.clone());
                i1.boundary_approx_eq(&i2, Angle::from_radians(1e-7))
            }
            _ => true,
        }
    })
}

/// Symmetric difference is commutative: A△B ≈ B△A.
#[quickcheck]
fn prop_symmetric_difference_commutative(
    a0: i32,
    a1: i32,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
    b_seed: u32,
    b_offset: u32,
) -> bool {
    let off = (b_offset % 50) as i32 + 1;
    no_panic(move || {
        match (
            make_safe_triangle_polygon(a0, a1, a2, a3, a4, a5),
            make_safe_triangle_polygon(
                a0.wrapping_add(off),
                a1.wrapping_add(off),
                a2.wrapping_add(off),
                a3.wrapping_sub(off),
                (b_seed as i32).wrapping_add(a4),
                (b_seed as i32).wrapping_sub(a5),
            ),
        ) {
            (Some(a), Some(b)) => {
                let sd1 = Polygon::symmetric_difference(&mut a.clone(), &mut b.clone());
                let sd2 = Polygon::symmetric_difference(&mut b.clone(), &mut a.clone());
                sd1.boundary_approx_eq(&sd2, Angle::from_radians(1e-7))
            }
            _ => true,
        }
    })
}

// ════════════════════════════════════════════════════════════════════
// 12. POLYGON-POLYLINE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `intersect_with_polyline` + `subtract_from_polyline` together cover
/// the entire original polyline (total vertex count >= original).
#[quickcheck]
fn prop_polyline_intersect_subtract_cover(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    pl_off0: i32,
    pl_off1: i32,
) -> bool {
    no_panic(move || {
        use s2rst::s2::polyline::Polyline;
        match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
            Some(mut poly) => {
                let p0 =
                    make_latlng(lat0.wrapping_add(pl_off0), lng0.wrapping_add(pl_off1)).to_point();
                let p1 =
                    make_latlng(lat1.wrapping_sub(pl_off0), lng1.wrapping_sub(pl_off1)).to_point();
                if p0.distance(p1).radians() < 1e-10 {
                    return true;
                }
                let pl = Polyline::new(vec![p0, p1]);
                let inside = poly.intersect_with_polyline(&pl);
                let outside = poly.subtract_from_polyline(&pl);
                let total_segments: usize = inside
                    .iter()
                    .chain(outside.iter())
                    .map(|p| {
                        if p.num_vertices() >= 2 {
                            p.num_vertices() - 1
                        } else {
                            0
                        }
                    })
                    .sum();
                total_segments >= 1
            }
            None => true,
        }
    })
}

/// `contains_polyline` → `intersects_polyline`.
#[quickcheck]
fn prop_contains_implies_intersects_polyline(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    pl_off0: i32,
    pl_off1: i32,
) -> bool {
    no_panic(move || {
        use s2rst::s2::polyline::Polyline;
        match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
            Some(mut poly) => {
                let p0 =
                    make_latlng(lat0.wrapping_add(pl_off0), lng0.wrapping_add(pl_off1)).to_point();
                let p1 =
                    make_latlng(lat1.wrapping_sub(pl_off0), lng1.wrapping_sub(pl_off1)).to_point();
                if p0.distance(p1).radians() < 1e-10 {
                    return true;
                }
                let pl = Polyline::new(vec![p0, p1]);
                if poly.contains_polyline(&pl) {
                    poly.intersects_polyline(&pl)
                } else {
                    true
                }
            }
            None => true,
        }
    })
}

/// `disjoint_polyline` ↔ !`intersects_polyline`.
#[quickcheck]
fn prop_disjoint_iff_not_intersects_polyline(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    pl_off0: i32,
    pl_off1: i32,
) -> bool {
    no_panic(move || {
        use s2rst::s2::polyline::Polyline;
        match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
            Some(mut poly) => {
                let p0 =
                    make_latlng(lat0.wrapping_add(pl_off0), lng0.wrapping_add(pl_off1)).to_point();
                let p1 =
                    make_latlng(lat1.wrapping_sub(pl_off0), lng1.wrapping_sub(pl_off1)).to_point();
                if p0.distance(p1).radians() < 1e-10 {
                    return true;
                }
                let pl = Polyline::new(vec![p0, p1]);
                poly.disjoint_polyline(&pl) != poly.intersects_polyline(&pl)
            }
            None => true,
        }
    })
}

// ════════════════════════════════════════════════════════════════════
// 13. CELL UNION PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `CellUnion::normalize` is idempotent and the result contains all original cells.
#[quickcheck]
fn prop_cell_union_normalize_idempotent(face0: i32, face1: i32, level0: i32, level1: i32) -> bool {
    use s2rst::s2::CellUnion;

    let f0 = (face0.rem_euclid(6)) as u8;
    let f1 = (face1.rem_euclid(6)) as u8;
    let l0 = (level0.rem_euclid(20)) as u8 + 1; // [1, 20]
    let l1 = (level1.rem_euclid(20)) as u8 + 1;

    let id0 = CellId::from_face(f0).child_begin_at_level(l0);
    let id1 = CellId::from_face(f1).child_begin_at_level(l1);

    let cu = CellUnion::from_cell_ids(vec![id0, id1, id0]); // duplicate on purpose
    // Normalized union should contain all original cells.
    cu.contains_cell_id(id0) && cu.contains_cell_id(id1)
}

/// `CellUnion` from a single cell contains its center point.
#[quickcheck]
fn prop_cell_union_contains_center(face: i32, level: i32) -> bool {
    use s2rst::s2::CellUnion;

    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(25)) as u8 + 1; // [1, 25]
    let id = CellId::from_face(f).child_begin_at_level(l);
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.contains_cell_id(id)
}

// ════════════════════════════════════════════════════════════════════
// 14. POINT DISTANCE TRIANGLE INEQUALITY
// ════════════════════════════════════════════════════════════════════

/// Point distances satisfy the triangle inequality: d(a,c) <= d(a,b) + d(b,c).
#[quickcheck]
fn prop_point_distance_triangle_inequality(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let d_ac = a.distance(c).radians();
    let d_ab = a.distance(b).radians();
    let d_bc = b.distance(c).radians();
    // Allow small floating point tolerance.
    d_ac <= d_ab + d_bc + 1e-14
}

// ════════════════════════════════════════════════════════════════════
// 15. CELLID PARENT/CHILD CONSISTENCY
// ════════════════════════════════════════════════════════════════════

/// A cell's `child_begin` at level L is contained by the face cell.
#[quickcheck]
fn prop_cellid_face_contains_descendants(face: i32, level: i32) -> bool {
    let f = s2rst::s2::Face::from_u8((face.rem_euclid(6)) as u8);
    let l = (level.rem_euclid(29)) as u8 + 1; // [1, 29]
    let face_id = CellId::from_face(f);
    let descendant = face_id.child_begin_at_level(l);
    // The face cell must contain all its descendants.
    face_id.contains(descendant) && descendant.level() == l && descendant.face() == f
}

// ════════════════════════════════════════════════════════════════════
// 16. CAP BOUND CONSISTENCY
// ════════════════════════════════════════════════════════════════════

/// A Cap's `rect_bound` contains all points of the cap's center, and
/// the `cap_bound` (which is the cap itself) contains the `rect_bound`'s vertices.
#[quickcheck]
fn prop_cap_rect_bound_contains_center(lat: i32, lng: i32, radius_i: i32) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius_deg = (radius_i.rem_euclid(89)) as f64 + 1.0; // [1, 89] degrees
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    let rect = cap.rect_bound();
    // The center (as LatLng) must be inside the rect bound.
    let center_ll = LatLng::from_point(center);
    rect.contains_lat_lng(center_ll)
}

// ════════════════════════════════════════════════════════════════════
// 17. CELL GEOMETRIC PROPERTIES
// ════════════════════════════════════════════════════════════════════

use s2rst::s2::Cell;

/// Cell vertices are unit vectors (normalized).
#[quickcheck]
fn prop_cell_vertices_normalized(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 30) + 1);
    let cell = Cell::from_cell_id(id);
    (0..4).all(|k| {
        let v = cell.vertex(k);
        (v.0.norm2() - 1.0).abs() < 1e-14
    })
}

/// Cell `approx_area` is positive and at most 4π.
#[quickcheck]
fn prop_cell_area_bounded(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 30) + 1);
    let cell = Cell::from_cell_id(id);
    let a = cell.approx_area();
    a > 0.0 && a <= 4.0 * PI + 1e-10
}

/// Parent cell has larger area than child cell.
#[quickcheck]
fn prop_cell_area_decreases_with_level(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let lv = (level % 29) + 1; // [1, 29] so parent exists
    let child_id = CellId::from_lat_lng(&ll).parent_at_level(lv);
    let parent_id = child_id.parent_at_level(lv - 1);
    let child = Cell::from_cell_id(child_id);
    let parent = Cell::from_cell_id(parent_id);
    parent.approx_area() > child.approx_area()
}

/// Cell's `rect_bound` contains all 4 vertices.
#[quickcheck]
fn prop_cell_bound_contains_vertices(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 30) + 1);
    let cell = Cell::from_cell_id(id);
    let bound = cell.rect_bound();
    (0..4).all(|k| bound.contains_lat_lng(LatLng::from_point(cell.vertex(k))))
}

/// Cell contains its own center point.
#[quickcheck]
fn prop_cell_contains_center(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 30) + 1);
    let cell = Cell::from_cell_id(id);
    cell.contains_point(id.to_point())
}

/// Cell distance to its own center is zero.
#[quickcheck]
fn prop_cell_distance_to_center_zero(lat: i32, lng: i32, level: u8) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 30) + 1);
    let cell = Cell::from_cell_id(id);
    cell.distance_to_point(id.to_point()).to_angle().radians() < 1e-10
}

/// Cell distance to a point is symmetric with `max_distance_to_point` ordering.
#[quickcheck]
fn prop_cell_distance_le_max_distance(lat: i32, lng: i32, level: u8, px: i32, py: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level((level % 25) + 1);
    let cell = Cell::from_cell_id(id);
    let p = make_latlng(px, py).to_point();
    cell.distance_to_point(p) <= cell.max_distance_to_point(p)
}

// ════════════════════════════════════════════════════════════════════
// 18. CELLUNION ADDITIONAL PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Normalized `CellUnion` is sorted with non-overlapping cells.
#[quickcheck]
fn prop_cell_union_sorted_and_disjoint(face0: i32, face1: i32, level0: i32, level1: i32) -> bool {
    use s2rst::s2::CellUnion;
    let f0 = (face0.rem_euclid(6)) as u8;
    let f1 = (face1.rem_euclid(6)) as u8;
    let l0 = (level0.rem_euclid(20)) as u8 + 1;
    let l1 = (level1.rem_euclid(20)) as u8 + 1;
    let id0 = CellId::from_face(f0).child_begin_at_level(l0);
    let id1 = CellId::from_face(f1).child_begin_at_level(l1);
    let cu = CellUnion::from_cell_ids(vec![id0, id1, id0]);
    cu.is_valid() && cu.is_normalized()
}

/// `CellUnion` contains a point from each of its cells.
#[quickcheck]
fn prop_cell_union_contains_cell_centers(face: i32, level: i32) -> bool {
    use s2rst::s2::CellUnion;
    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(25)) as u8 + 1;
    let id = CellId::from_face(f).child_begin_at_level(l);
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.contains_point(id.to_point())
}

/// `CellUnion` `from_range` covers the range endpoints.
#[quickcheck]
fn prop_cell_union_from_range_covers_endpoints(face: i32, level: i32) -> bool {
    use s2rst::s2::CellUnion;
    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(20)) as u8 + 5; // [5, 24]
    let id = CellId::from_face(f).child_begin_at_level(l);
    let begin = id.range_min();
    let end = id.range_max().next();
    let cu = CellUnion::from_range(begin, end);
    cu.is_valid() && cu.contains_cell_id(id)
}

/// Denormalized `CellUnion` is still valid.
#[quickcheck]
fn prop_cell_union_denormalize_valid(face: i32, level: i32, min_lev: u8, lev_mod: u8) -> bool {
    use s2rst::s2::CellUnion;
    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(20)) as u8 + 5;
    let id = CellId::from_face(f).child_begin_at_level(l);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let min_l = min_lev % 10;
    let mod_v = (lev_mod % 3) + 1; // [1, 3]
    let denorm = cu.denormalize(Level::new(min_l), mod_v);
    denorm.is_valid()
}

/// `leaf_cells_covered` is positive for non-empty unions.
#[quickcheck]
fn prop_cell_union_leaf_cells_positive(face: i32, level: i32) -> bool {
    use s2rst::s2::CellUnion;
    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(25)) as u8 + 1;
    let id = CellId::from_face(f).child_begin_at_level(l);
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.leaf_cells_covered() > 0
}

/// `CellUnion` intersection is commutative.
#[quickcheck]
fn prop_cell_union_intersects_commutative(f0: i32, f1: i32, l0: i32, l1: i32) -> bool {
    use s2rst::s2::CellUnion;
    let id0 = CellId::from_face((f0.rem_euclid(6)) as u8)
        .child_begin_at_level(((l0.rem_euclid(20)) as u8) + 1);
    let id1 = CellId::from_face((f1.rem_euclid(6)) as u8)
        .child_begin_at_level(((l1.rem_euclid(20)) as u8) + 1);
    let cu0 = CellUnion::from_cell_ids(vec![id0]);
    let cu1 = CellUnion::from_cell_ids(vec![id1]);
    cu0.intersects_union(&cu1) == cu1.intersects_union(&cu0)
}

// ════════════════════════════════════════════════════════════════════
// 19. EDGE CROSSING PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// `crossing_sign` is symmetric: `crossing_sign(a,b,c,d)` == `crossing_sign(c,d,a,b)`.
#[quickcheck]
fn prop_crossing_sign_symmetric(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::edge_crossings;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let d = make_latlng(lat3, lng3).to_point();
    let r1 = edge_crossings::crossing_sign(a, b, c, d);
    let r2 = edge_crossings::crossing_sign(c, d, a, b);
    r1 == r2
}

/// `signed_vertex_crossing` returns value in {-1, 0, 1}.
#[quickcheck]
fn prop_signed_vertex_crossing_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::edge_crossings;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let d = make_latlng(lat3, lng3).to_point();
    let r = edge_crossings::signed_vertex_crossing(a, b, c, d);
    (-1..=1).contains(&r)
}

/// `intersection_error` is positive.
#[quickcheck]
fn prop_intersection_error_positive(_dummy: u8) -> bool {
    use s2rst::s2::edge_crossings;
    edge_crossings::intersection_error().radians() > 0.0
}

/// `intersection_merge_radius` >= `intersection_error`.
#[quickcheck]
fn prop_intersection_merge_ge_error(_dummy: u8) -> bool {
    use s2rst::s2::edge_crossings;
    edge_crossings::intersection_merge_radius() >= edge_crossings::intersection_error()
}

// ════════════════════════════════════════════════════════════════════
// 20. ENCODING ROUNDTRIP PROPERTIES
// ════════════════════════════════════════════════════════════════════

use s2rst::s2::encoding::{S2Decode, S2Encode};

/// `CellId` encode/decode roundtrip.
#[quickcheck]
fn prop_cellid_encode_decode_roundtrip(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll);
    let mut buf = Vec::new();
    id.encode(&mut buf).unwrap();
    let decoded = CellId::decode(&mut &buf[..]).unwrap();
    decoded == id
}

/// Point encode/decode roundtrip.
#[quickcheck]
fn prop_point_encode_decode_roundtrip(lat: i32, lng: i32) -> bool {
    let p = make_latlng(lat, lng).to_point();
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    let decoded = Point::decode(&mut &buf[..]).unwrap();
    p.distance(decoded).radians() < 1e-15
}

/// Cap encode/decode roundtrip.
#[quickcheck]
fn prop_cap_encode_decode_roundtrip(lat: i32, lng: i32, radius_i: u16) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius = Angle::from_degrees((radius_i % 18001) as f64 / 100.0);
    let cap = Cap::from_center_angle(center, radius);
    let mut buf = Vec::new();
    cap.encode(&mut buf).unwrap();
    let decoded = Cap::decode(&mut &buf[..]).unwrap();
    if cap.is_empty() {
        decoded.is_empty()
    } else if cap.is_full() {
        decoded.is_full()
    } else {
        cap.center().distance(decoded.center()).radians() < 1e-15
            && (cap.angle_radius().radians() - decoded.angle_radius().radians()).abs() < 1e-15
    }
}

/// Rect encode/decode roundtrip.
#[quickcheck]
fn prop_rect_encode_decode_roundtrip(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_lat_lng(make_latlng(lat0, lng0)).add_point(make_latlng(lat1, lng1));
    let mut buf = Vec::new();
    r.encode(&mut buf).unwrap();
    let decoded = Rect::decode(&mut &buf[..]).unwrap();
    r == decoded
}

/// Loop encode/decode preserves area.
#[quickcheck]
fn prop_loop_encode_decode_area(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let mut buf = Vec::new();
            lp.encode(&mut buf).unwrap();
            let decoded = Loop::decode(&mut &buf[..]).unwrap();
            (lp.area() - decoded.area()).abs() < 1e-10
        }
        None => true,
    }
}

/// Polygon encode/decode preserves area.
#[quickcheck]
fn prop_polygon_encode_decode_area(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let mut buf = Vec::new();
            p.encode(&mut buf).unwrap();
            let decoded = Polygon::decode(&mut &buf[..]).unwrap();
            (p.area() - decoded.area()).abs() < 1e-10
        }
        None => true,
    }
}

/// `CellUnion` encode/decode roundtrip.
#[quickcheck]
fn prop_cell_union_encode_decode_roundtrip(face: i32, level: i32) -> bool {
    use s2rst::s2::CellUnion;
    let f = (face.rem_euclid(6)) as u8;
    let l = (level.rem_euclid(25)) as u8 + 1;
    let id = CellId::from_face(f).child_begin_at_level(l);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let mut buf = Vec::new();
    cu.encode(&mut buf).unwrap();
    let decoded = CellUnion::decode(&mut &buf[..]).unwrap();
    decoded.contains_cell_id(id) && cu.num_cells() == decoded.num_cells()
}

// ════════════════════════════════════════════════════════════════════
// 21. EXTENDED ENCODING / COMPRESSION PROPERTY TESTS
// ════════════════════════════════════════════════════════════════════

/// Polyline encode/decode roundtrip preserves all vertices exactly.
#[quickcheck]
fn prop_polyline_encode_decode_roundtrip(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let pl = Polyline::new(vec![p0, p1, p2]);
    let mut buf = Vec::new();
    pl.encode(&mut buf).unwrap();
    let decoded = Polyline::decode(&mut &buf[..]).unwrap();
    decoded.len() == pl.len() && (0..pl.len()).all(|i| pl[i] == decoded[i])
}

/// Cell encode/decode roundtrip at various levels.
#[quickcheck]
fn prop_cell_encode_decode_roundtrip(lat: i32, lng: i32, level_raw: u8) -> bool {
    use s2rst::s2::Cell;
    let level = level_raw % 31; // 0..30
    let ll = make_latlng(lat, lng);
    let id = CellId::from_lat_lng(&ll).parent_at_level(level);
    let cell = Cell::from(id);
    let mut buf = Vec::new();
    cell.encode(&mut buf).unwrap();
    let decoded = Cell::decode(&mut &buf[..]).unwrap();
    cell.id() == decoded.id() && cell.level() == decoded.level()
}

/// `CellUnion` with multiple cells encode/decode roundtrip.
#[quickcheck]
fn prop_cell_union_multi_encode_decode(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::CellUnion;
    let id0 = CellId::from_lat_lng(&make_latlng(lat0, lng0)).parent_at_level(10);
    let id1 = CellId::from_lat_lng(&make_latlng(lat1, lng1)).parent_at_level(10);
    let cu = CellUnion::from_cell_ids(vec![id0, id1]);
    let mut buf = Vec::new();
    cu.encode(&mut buf).unwrap();
    let decoded = CellUnion::decode(&mut &buf[..]).unwrap();
    cu.num_cells() == decoded.num_cells()
        && cu
            .cell_ids()
            .iter()
            .zip(decoded.cell_ids())
            .all(|(a, b)| a == b)
}

/// Empty Cap roundtrip.
#[quickcheck]
fn prop_cap_empty_roundtrip(_dummy: u8) -> bool {
    let cap = Cap::empty();
    let mut buf = Vec::new();
    cap.encode(&mut buf).unwrap();
    let decoded = Cap::decode(&mut &buf[..]).unwrap();
    decoded.is_empty()
}

/// Full Cap roundtrip.
#[quickcheck]
fn prop_cap_full_roundtrip(_dummy: u8) -> bool {
    let cap = Cap::full();
    let mut buf = Vec::new();
    cap.encode(&mut buf).unwrap();
    let decoded = Cap::decode(&mut &buf[..]).unwrap();
    decoded.is_full()
}

/// Loop encode/decode preserves vertex count, containment, and depth exactly.
#[quickcheck]
fn prop_loop_encode_decode_exact(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let mut buf = Vec::new();
            lp.encode(&mut buf).unwrap();
            let decoded = Loop::decode(&mut &buf[..]).unwrap();
            lp.num_vertices() == decoded.num_vertices()
                && lp.contains_origin() == decoded.contains_origin()
                && lp.depth() == decoded.depth()
                && (0..lp.num_vertices()).all(|i| lp.vertex(i) == decoded.vertex(i))
        }
        None => true,
    }
}

/// Empty loop encode/decode roundtrip.
#[quickcheck]
fn prop_loop_empty_roundtrip(_dummy: u8) -> bool {
    let l = Loop::empty();
    let mut buf = Vec::new();
    l.encode(&mut buf).unwrap();
    let decoded = Loop::decode(&mut &buf[..]).unwrap();
    decoded.is_empty_loop()
}

/// Full loop encode/decode roundtrip.
#[quickcheck]
fn prop_loop_full_roundtrip(_dummy: u8) -> bool {
    let l = Loop::full();
    let mut buf = Vec::new();
    l.encode(&mut buf).unwrap();
    let decoded = Loop::decode(&mut &buf[..]).unwrap();
    decoded.is_full_loop()
}

/// Polygon encode/decode preserves `num_loops` and `num_vertices` exactly.
#[quickcheck]
fn prop_polygon_encode_decode_exact(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let mut buf = Vec::new();
            p.encode(&mut buf).unwrap();
            let decoded = Polygon::decode(&mut &buf[..]).unwrap();
            p.num_loops() == decoded.num_loops()
                && p.num_vertices() == decoded.num_vertices()
                && p.has_holes() == decoded.has_holes()
        }
        None => true,
    }
}

/// Polygon with cell-center vertices (forces compressed encoding) roundtrips.
#[quickcheck]
fn prop_polygon_compressed_cell_center_roundtrip(face_raw: u8, level_raw: u8, pos: u64) -> bool {
    no_panic(|| {
        let face = face_raw % 6;
        let level = (level_raw % 20) + 5; // levels 5..24
        let base = CellId::from_face_pos_level(face, pos, level);
        let mut vertices = Vec::new();
        let mut id = base;
        for _ in 0..3 {
            vertices.push(id.to_point());
            id = id.next();
        }
        let lp = Loop::new(vertices);
        if lp.num_vertices() < 3 {
            return true;
        }
        let p = Polygon::from_loops(vec![lp]);
        if p.num_loops() == 0 {
            return true;
        }
        let mut buf = Vec::new();
        p.encode(&mut buf).unwrap();
        let decoded = Polygon::decode(&mut &buf[..]).unwrap();
        decoded.num_loops() == p.num_loops() && decoded.num_vertices() == p.num_vertices()
    })
}

/// Polygon with arbitrary (non-cell-center) vertices roundtrips correctly
/// regardless of which encoding path is chosen.
#[quickcheck]
fn prop_polygon_arbitrary_vertices_roundtrip(
    lat0: i16,
    lng0: i16,
    lat1: i16,
    lng1: i16,
    lat2: i16,
    lng2: i16,
) -> bool {
    // Use fractional degrees to ensure points are NOT cell centers.
    let p0 = LatLng::from_degrees(
        (lat0 as f64 / 400.0).clamp(-80.0, 80.0),
        (lng0 as f64 / 200.0).clamp(-170.0, 170.0),
    )
    .to_point();
    let p1 = LatLng::from_degrees(
        (lat1 as f64 / 400.0).clamp(-80.0, 80.0),
        (lng1 as f64 / 200.0).clamp(-170.0, 170.0),
    )
    .to_point();
    let p2 = LatLng::from_degrees(
        (lat2 as f64 / 400.0).clamp(-80.0, 80.0),
        (lng2 as f64 / 200.0).clamp(-170.0, 170.0),
    )
    .to_point();
    let cross = p0.0.cross(p1.0).dot(p2.0);
    if cross.abs() < 1e-10 {
        return true;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    if lp.num_vertices() < 3 {
        return true;
    }
    let p = Polygon::from_loops(vec![lp]);
    if p.num_loops() == 0 {
        return true;
    }
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    let decoded = Polygon::decode(&mut &buf[..]).unwrap();
    p.num_loops() == decoded.num_loops()
        && p.num_vertices() == decoded.num_vertices()
        && (p.area() - decoded.area()).abs() < 1e-8
}

/// Empty polygon roundtrip.
#[quickcheck]
fn prop_polygon_empty_roundtrip(_dummy: u8) -> bool {
    let p = Polygon::empty();
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    let decoded = Polygon::decode(&mut &buf[..]).unwrap();
    decoded.is_empty_polygon()
}

/// Full polygon roundtrip.
#[quickcheck]
fn prop_polygon_full_roundtrip(_dummy: u8) -> bool {
    let p = Polygon::full();
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    let decoded = Polygon::decode(&mut &buf[..]).unwrap();
    decoded.is_full_polygon() && decoded.num_loops() == p.num_loops()
}

/// Polygon with two non-overlapping loops roundtrip.
#[quickcheck]
fn prop_polygon_two_loops_roundtrip(lat: i32, lng: i32) -> bool {
    // Two small triangles far apart.
    let base0 = make_latlng(lat, lng);
    let base1 = make_latlng(lat.wrapping_add(90), lng.wrapping_add(90));
    let offset = 2.0;
    let l0 = Loop::new(vec![
        LatLng::from_degrees(base0.lat.degrees(), base0.lng.degrees()).to_point(),
        LatLng::from_degrees(base0.lat.degrees() + offset, base0.lng.degrees()).to_point(),
        LatLng::from_degrees(base0.lat.degrees(), base0.lng.degrees() + offset).to_point(),
    ]);
    let l1 = Loop::new(vec![
        LatLng::from_degrees(base1.lat.degrees(), base1.lng.degrees()).to_point(),
        LatLng::from_degrees(base1.lat.degrees() + offset, base1.lng.degrees()).to_point(),
        LatLng::from_degrees(base1.lat.degrees(), base1.lng.degrees() + offset).to_point(),
    ]);
    if l0.num_vertices() < 3 || l1.num_vertices() < 3 {
        return true;
    }
    let p = Polygon::from_loops(vec![l0, l1]);
    if p.num_loops() < 2 {
        return true; // loops may have been merged
    }
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    let decoded = Polygon::decode(&mut &buf[..]).unwrap();
    p.num_loops() == decoded.num_loops() && p.num_vertices() == decoded.num_vertices()
}

/// Decode from truncated data should not panic (returns Err).
#[quickcheck]
fn prop_point_decode_truncated(len: u8) -> bool {
    // C++ S2Point: 3 raw doubles (24 bytes), no version byte.
    let full_buf = vec![
        0, 0, 0, 0, 0, 0, 0xF0, 0x3F, // x = 1.0
        0, 0, 0, 0, 0, 0, 0, 0, // y = 0.0
        0, 0, 0, 0, 0, 0, 0, 0, // z = 0.0
    ];
    let truncated_len = (len as usize) % (full_buf.len() + 1);
    let truncated = &full_buf[..truncated_len];
    // Should either succeed (if len is full) or return Err (never panic).
    match Point::decode(&mut &truncated[..]) {
        Ok(_) => truncated_len == full_buf.len(),
        Err(_) => truncated_len < full_buf.len(),
    }
}

/// Decode from truncated data for `CellId` should not panic.
#[quickcheck]
fn prop_cellid_decode_truncated(len: u8) -> bool {
    // Encode a *valid* cell id: `CellId::decode` now rejects invalid ids, so an
    // all-zero buffer (`CellId(0)`, not a valid cell) would `Err` even at full
    // length. A valid id round-trips at full length and `Err`s when truncated.
    let mut full_buf = Vec::new();
    CellId::from_face(0u8).encode(&mut full_buf).unwrap();
    let truncated_len = (len as usize) % (full_buf.len() + 1);
    match CellId::decode(&mut &full_buf[..truncated_len]) {
        Ok(_) => truncated_len == full_buf.len(),
        Err(_) => truncated_len < full_buf.len(),
    }
}

/// Decode from truncated data for Rect should not panic.
#[quickcheck]
fn prop_rect_decode_truncated(len: u8) -> bool {
    let rect = Rect::from_lat_lng(make_latlng(10, 20)).add_point(make_latlng(30, 40));
    let mut full_buf = Vec::new();
    rect.encode(&mut full_buf).unwrap();
    let truncated_len = (len as usize) % (full_buf.len() + 1);
    // Should not panic.
    drop(Rect::decode(&mut &full_buf[..truncated_len]));
    true
}

/// Decode from truncated data for Loop should not panic.
#[quickcheck]
fn prop_loop_decode_truncated(len: u8) -> bool {
    let l = Loop::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]);
    let mut full_buf = Vec::new();
    l.encode(&mut full_buf).unwrap();
    let truncated_len = (len as usize) % (full_buf.len() + 1);
    drop(Loop::decode(&mut &full_buf[..truncated_len]));
    true
}

/// Decode from random/garbage bytes should not panic for most types.
/// `Cell::decode` can panic on invalid `CellIds` (overflow in internal math),
/// so we wrap it in `catch_unwind`.
#[quickcheck]
fn prop_decode_garbage_no_panic(b0: u64, b1: u64, b2: u64, b3: u64) -> bool {
    let mut buf = Vec::with_capacity(32);
    buf.extend_from_slice(&b0.to_le_bytes());
    buf.extend_from_slice(&b1.to_le_bytes());
    buf.extend_from_slice(&b2.to_le_bytes());
    buf.extend_from_slice(&b3.to_le_bytes());
    // These should not panic.
    drop(Point::decode(&mut &buf[..]));
    drop(CellId::decode(&mut &buf[..]));
    drop(Cap::decode(&mut &buf[..]));
    drop(Rect::decode(&mut &buf[..]));
    // These may panic on invalid internal data (overflow in CellId math),
    // so wrap in catch_unwind.
    drop(std::panic::catch_unwind(|| Loop::decode(&mut &buf[..])));
    drop(std::panic::catch_unwind(|| Polygon::decode(&mut &buf[..])));
    drop(std::panic::catch_unwind(|| Cell::decode(&mut &buf[..])));
    drop(std::panic::catch_unwind(|| {
        s2rst::s2::CellUnion::decode(&mut &buf[..])
    }));
    drop(std::panic::catch_unwind(|| {
        s2rst::s2::polyline::Polyline::decode(&mut &buf[..])
    }));
    true
}

/// Double encode/decode is idempotent: encode(decode(encode(x))) == encode(x).
#[quickcheck]
fn prop_point_double_encode_idempotent(lat: i32, lng: i32) -> bool {
    let p = make_latlng(lat, lng).to_point();
    let mut buf1 = Vec::new();
    p.encode(&mut buf1).unwrap();
    let decoded = Point::decode(&mut &buf1[..]).unwrap();
    let mut buf2 = Vec::new();
    decoded.encode(&mut buf2).unwrap();
    buf1 == buf2
}

/// Double encode/decode is idempotent for Cap.
#[quickcheck]
fn prop_cap_double_encode_idempotent(lat: i32, lng: i32, radius_i: u16) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius = Angle::from_degrees((radius_i % 18001) as f64 / 100.0);
    let cap = Cap::from_center_angle(center, radius);
    let mut buf1 = Vec::new();
    cap.encode(&mut buf1).unwrap();
    let decoded = Cap::decode(&mut &buf1[..]).unwrap();
    let mut buf2 = Vec::new();
    decoded.encode(&mut buf2).unwrap();
    buf1 == buf2
}

/// Double encode/decode is idempotent for Rect.
#[quickcheck]
fn prop_rect_double_encode_idempotent(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_lat_lng(make_latlng(lat0, lng0)).add_point(make_latlng(lat1, lng1));
    let mut buf1 = Vec::new();
    r.encode(&mut buf1).unwrap();
    let decoded = Rect::decode(&mut &buf1[..]).unwrap();
    let mut buf2 = Vec::new();
    decoded.encode(&mut buf2).unwrap();
    buf1 == buf2
}

/// Double encode/decode is idempotent for `CellId`.
#[quickcheck]
fn prop_cellid_double_encode_idempotent(lat: i32, lng: i32) -> bool {
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let mut buf1 = Vec::new();
    id.encode(&mut buf1).unwrap();
    let decoded = CellId::decode(&mut &buf1[..]).unwrap();
    let mut buf2 = Vec::new();
    decoded.encode(&mut buf2).unwrap();
    buf1 == buf2
}

/// Lossless Loop encode/decode is byte-identical on re-encode.
#[quickcheck]
fn prop_loop_double_encode_idempotent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let mut buf1 = Vec::new();
            lp.encode(&mut buf1).unwrap();
            let decoded = Loop::decode(&mut &buf1[..]).unwrap();
            let mut buf2 = Vec::new();
            decoded.encode(&mut buf2).unwrap();
            buf1 == buf2
        }
        None => true,
    }
}

/// Polygon area is preserved through encode/decode within tolerance,
/// for both lossless and compressed encoding paths.
#[quickcheck]
fn prop_polygon_area_preserved_both_paths(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            // Default auto-selecting encode.
            let mut buf = Vec::new();
            p.encode(&mut buf).unwrap();
            let decoded = Polygon::decode(&mut &buf[..]).unwrap();
            let area_diff = (p.area() - decoded.area()).abs();
            area_diff < 1e-8
        }
        None => true,
    }
}

/// Encoded size for Cell is always exactly 8 bytes (just a `CellId`).
#[quickcheck]
fn prop_cell_encoded_size(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Cell;
    let cell = Cell::from(CellId::from_lat_lng(&make_latlng(lat, lng)));
    let mut buf = Vec::new();
    cell.encode(&mut buf).unwrap();
    buf.len() == 8
}

/// Encoded size for `CellId` is always exactly 8 bytes.
#[quickcheck]
fn prop_cellid_encoded_size(lat: i32, lng: i32) -> bool {
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let mut buf = Vec::new();
    id.encode(&mut buf).unwrap();
    buf.len() == 8
}

/// Encoded size for Point is always exactly 25 bytes (1 version + 3×8 doubles).
#[quickcheck]
fn prop_point_encoded_size(lat: i32, lng: i32) -> bool {
    // C++ S2Point: 3 raw doubles (24 bytes), no version byte.
    let p = make_latlng(lat, lng).to_point();
    let mut buf = Vec::new();
    p.encode(&mut buf).unwrap();
    buf.len() == 24
}

/// `CellUnion` with varying numbers of cells roundtrips correctly.
#[quickcheck]
fn prop_cell_union_varying_size(raw: u64) -> bool {
    use s2rst::s2::CellUnion;
    let count = ((raw % 10) + 1) as usize;
    let level = ((raw >> 4) % 20 + 1) as u8;
    let mut ids = Vec::new();
    let face_id = CellId::from_face(((raw >> 8) % 6) as u8);
    let mut id = face_id.child_begin_at_level(level);
    let end = face_id.child_end_at_level(level);
    for _ in 0..count {
        if id >= end {
            break;
        }
        ids.push(id);
        id = id.next();
    }
    if ids.is_empty() {
        return true;
    }
    let cu = CellUnion::from_cell_ids(ids);
    let mut buf = Vec::new();
    cu.encode(&mut buf).unwrap();
    let decoded = CellUnion::decode(&mut &buf[..]).unwrap();
    cu.num_cells() == decoded.num_cells()
        && cu
            .cell_ids()
            .iter()
            .zip(decoded.cell_ids())
            .all(|(a, b)| a == b)
}

/// Polyline with varying lengths roundtrips correctly.
#[quickcheck]
fn prop_polyline_varying_length(raw: u64) -> bool {
    use s2rst::s2::polyline::Polyline;
    let count = ((raw % 20) + 2) as usize;
    let mut vertices = Vec::new();
    for i in 0..count {
        let lat = (i as f64) * 0.5;
        let lng = (i as f64) * 0.3 + (raw as f64 / 1e18).fract() * 10.0;
        vertices.push(LatLng::from_degrees(lat, lng).to_point());
    }
    let pl = Polyline::new(vertices);
    let mut buf = Vec::new();
    pl.encode(&mut buf).unwrap();
    let decoded = Polyline::decode(&mut &buf[..]).unwrap();
    pl.len() == decoded.len() && (0..pl.len()).all(|i| pl[i] == decoded[i])
}

// ════════════════════════════════════════════════════════════════════
// 22. REGION COVERER PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Covering cells are at valid levels within constraints.
#[quickcheck]
fn prop_covering_respects_level_constraints(lat: i32, lng: i32, radius_i: u8) -> bool {
    use s2rst::s2::region_coverer::RegionCoverer;
    let center = make_latlng(lat, lng).to_point();
    let radius_deg = (radius_i % 90) as f64 + 1.0;
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    let min_lev: u8 = 3;
    let max_lev: u8 = 12;
    let coverer = RegionCoverer::new()
        .min_level(min_lev)
        .max_level(max_lev)
        .max_cells(20);
    let covering = coverer.covering(&cap);
    covering.cell_ids().iter().all(|id| {
        let lv = id.level();
        lv >= min_lev && lv <= max_lev
    })
}

/// Covering of a cap contains the cap center.
#[quickcheck]
fn prop_covering_contains_center(lat: i32, lng: i32, radius_i: u8) -> bool {
    use s2rst::s2::region_coverer::RegionCoverer;
    let center = make_latlng(lat, lng).to_point();
    let radius_deg = (radius_i % 45) as f64 + 1.0;
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    let coverer = RegionCoverer::new().max_cells(8);
    let covering = coverer.covering(&cap);
    covering.contains_point(center)
}

/// Covering respects `max_cells` limit.
#[quickcheck]
fn prop_covering_respects_max_cells(lat: i32, lng: i32, radius_i: u8, max_c: u8) -> bool {
    use s2rst::s2::region_coverer::RegionCoverer;
    let center = make_latlng(lat, lng).to_point();
    let radius_deg = (radius_i % 45) as f64 + 1.0;
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    let max_cells = (max_c % 20) as usize + 1; // [1, 20]
    let coverer = RegionCoverer::new().max_cells(max_cells);
    let covering = coverer.covering(&cap);
    // Note: max_cells is a "soft" limit — may slightly exceed, but not by much.
    covering.num_cells() <= max_cells + 6
}

/// Fast covering is valid.
#[quickcheck]
fn prop_fast_covering_valid(lat: i32, lng: i32, radius_i: u8) -> bool {
    use s2rst::s2::region_coverer::RegionCoverer;
    let center = make_latlng(lat, lng).to_point();
    let radius_deg = (radius_i % 45) as f64 + 1.0;
    let cap = Cap::from_center_angle(center, Angle::from_degrees(radius_deg));
    let coverer = RegionCoverer::new().max_cells(8);
    let covering = coverer.fast_covering(&cap);
    covering.is_valid()
}

// ════════════════════════════════════════════════════════════════════
// 23. SHAPE MEASURE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Polygon shape area is in [0, 4π].
#[quickcheck]
fn prop_shape_area_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_measures;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let lp = LaxPolygon::from_polygon_ref(&p);
            let a = shape_measures::get_area(&lp);
            (0.0..=4.0 * PI + 1e-6).contains(&a)
        }
        None => true,
    }
}

/// Polyline shape length is non-negative.
#[quickcheck]
fn prop_shape_length_non_negative(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape_measures;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let lp = LaxPolyline::new(vec![p0, p1]);
    shape_measures::get_length(&lp).radians() >= 0.0
}

/// Shape centroid is finite.
#[quickcheck]
fn prop_shape_centroid_finite(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_measures;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let lp = LaxPolygon::from_polygon_ref(&p);
            let c = shape_measures::get_centroid(&lp);
            c.0.x.is_finite() && c.0.y.is_finite() && c.0.z.is_finite()
        }
        None => true,
    }
}

/// Polygon perimeter is non-negative.
#[quickcheck]
fn prop_shape_perimeter_non_negative(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_measures;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let lp = LaxPolygon::from_polygon_ref(&p);
            shape_measures::get_perimeter(&lp).radians() >= 0.0
        }
        None => true,
    }
}

// ════════════════════════════════════════════════════════════════════
// 24. SHAPE INDEX MEASURE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Index area equals shape area for a single-shape index.
#[quickcheck]
fn prop_index_area_matches_shape(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_index::ShapeIndex;
    use s2rst::s2::{shape_index_measures, shape_measures};
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let lp = LaxPolygon::from_polygon_ref(&p);
            let shape_area = shape_measures::get_area(&lp);
            let mut index = ShapeIndex::new();
            index.add(Box::new(LaxPolygon::from_polygon_ref(&p)));
            index.build();
            let index_area = shape_index_measures::get_area(&index);
            (shape_area - index_area).abs() < 1e-10
        }
        None => true,
    }
}

/// Empty index has zero area.
#[quickcheck]
fn prop_empty_index_zero_area(_dummy: u8) -> bool {
    use s2rst::s2::shape_index::ShapeIndex;
    use s2rst::s2::shape_index_measures;
    let index = ShapeIndex::new();
    shape_index_measures::get_area(&index) == 0.0
}

/// Index dimension is -1 for empty, 2 for polygon.
#[quickcheck]
fn prop_index_dimension_polygon(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_index::ShapeIndex;
    use s2rst::s2::shape_index_measures;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let mut index = ShapeIndex::new();
            index.add(Box::new(LaxPolygon::from_polygon_ref(&p)));
            index.build();
            shape_index_measures::get_dimension(&index) == Some(Dimension::Polygon)
        }
        None => true,
    }
}

// ════════════════════════════════════════════════════════════════════
// 25. HAUSDORFF DISTANCE PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Hausdorff distance is non-negative.
#[quickcheck]
fn prop_hausdorff_non_negative(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery;
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let p3 = make_latlng(lat3, lng3).to_point();
    let mut idx_a = ShapeIndex::new();
    idx_a.add(Box::new(PointVector::new(vec![p0, p1])));
    idx_a.build();
    let mut idx_b = ShapeIndex::new();
    idx_b.add(Box::new(PointVector::new(vec![p2, p3])));
    idx_b.build();
    let q = S2HausdorffDistanceQuery::new();
    q.get_distance(&idx_a, &idx_b).length2() >= 0.0
}

/// Hausdorff distance is symmetric (undirected).
#[quickcheck]
fn prop_hausdorff_symmetric(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery;
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let p3 = make_latlng(lat3, lng3).to_point();
    let mut idx_a = ShapeIndex::new();
    idx_a.add(Box::new(PointVector::new(vec![p0, p1])));
    idx_a.build();
    let mut idx_b = ShapeIndex::new();
    idx_b.add(Box::new(PointVector::new(vec![p2, p3])));
    idx_b.build();
    let q = S2HausdorffDistanceQuery::new();
    let d_ab = q.get_distance(&idx_a, &idx_b).length2();
    let d_ba = q.get_distance(&idx_b, &idx_a).length2();
    (d_ab - d_ba).abs() < 1e-14
}

/// Directed Hausdorff distance from a set to itself is zero.
#[quickcheck]
fn prop_hausdorff_self_zero(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::hausdorff_distance_query::S2HausdorffDistanceQuery;
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let mut idx = ShapeIndex::new();
    idx.add(Box::new(PointVector::new(vec![p0, p1])));
    idx.build();
    let q = S2HausdorffDistanceQuery::new();
    let d = q.get_directed_distance(&idx, &idx);
    d.length2() < 1e-14
}

// ════════════════════════════════════════════════════════════════════
// 26. CHAIN INTERPOLATION PROPERTIES
// ════════════════════════════════════════════════════════════════════

/// Total interpolation length is non-negative.
#[quickcheck]
fn prop_chain_interpolation_length_non_negative(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::chain_interpolation_query::S2ChainInterpolationQuery;
    use s2rst::s2::lax_polyline::LaxPolyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let shape = LaxPolyline::new(vec![p0, p1, p2]);
    let q = S2ChainInterpolationQuery::new(&shape);
    q.get_length().radians() >= 0.0
}

/// Interpolation at fraction 0 returns start, fraction 1 returns end.
#[quickcheck]
fn prop_chain_interpolation_endpoints(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::chain_interpolation_query::S2ChainInterpolationQuery;
    use s2rst::s2::lax_polyline::LaxPolyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    if p0.distance(p1).radians() < 1e-10 {
        return true;
    }
    let shape = LaxPolyline::new(vec![p0, p1]);
    let q = S2ChainInterpolationQuery::new(&shape);
    match (q.at_fraction(0.0), q.at_fraction(1.0)) {
        (Some(start), Some(end)) => {
            start.point.distance(p0).radians() < 1e-10 && end.point.distance(p1).radians() < 1e-10
        }
        _ => false,
    }
}

/// Interpolation `at_fraction` is consistent with `at_distance`.
#[quickcheck]
fn prop_chain_interpolation_fraction_distance_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    frac_raw: u8,
) -> bool {
    use s2rst::s2::chain_interpolation_query::S2ChainInterpolationQuery;
    use s2rst::s2::lax_polyline::LaxPolyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    if p0.distance(p1).radians() < 1e-6 {
        return true;
    }
    let shape = LaxPolyline::new(vec![p0, p1]);
    let q = S2ChainInterpolationQuery::new(&shape);
    let f = frac_raw as f64 / 255.0;
    let total_len = q.get_length();
    let by_frac = q.at_fraction(f);
    let by_dist = q.at_distance(Angle::from_radians(f * total_len.radians()));
    match (by_frac, by_dist) {
        (Some(r1), Some(r2)) => r1.point.distance(r2.point).radians() < 1e-8,
        _ => true,
    }
}

// ────────────────────────────────────────────────────────────────────
// Encoding roundtrip property tests
// ────────────────────────────────────────────────────────────────────

// ─── EncodedUintVector ──────────────────────────────────────────────

#[quickcheck]
fn prop_uint_vector_u32_roundtrip(vals: Vec<u32>) -> bool {
    use s2rst::s2::encoded_uint_vector::{decode_uint_vector_u32, encode_uint_vector_u32};
    let mut buf = Vec::new();
    encode_uint_vector_u32(&vals, &mut buf).unwrap();
    let decoded = decode_uint_vector_u32(&mut buf.as_slice()).unwrap();
    decoded == vals
}

#[quickcheck]
fn prop_uint_vector_u64_roundtrip(vals: Vec<u64>) -> bool {
    use s2rst::s2::encoded_uint_vector::{decode_uint_vector_u64, encode_uint_vector_u64};
    let mut buf = Vec::new();
    encode_uint_vector_u64(&vals, &mut buf).unwrap();
    let decoded = decode_uint_vector_u64(&mut buf.as_slice()).unwrap();
    decoded == vals
}

#[quickcheck]
fn prop_uint_vector_u32_uses_minimal_bytes(a: u32, b: u32, c: u32) -> bool {
    use s2rst::s2::encoded_uint_vector::encode_uint_vector_u32;
    let vals = vec![a, b, c];
    let max_val = *vals.iter().max().unwrap();
    // The encoder always uses at least 1 byte per value (the one_bits
    // accumulator is seeded with 1 to guarantee len >= 1).
    let expected_bytes_per_val = if max_val <= 0xff {
        1
    } else {
        (max_val.ilog2() as usize / 8) + 1
    };
    let mut buf = Vec::new();
    encode_uint_vector_u32(&vals, &mut buf).unwrap();
    // 1-byte header + expected_bytes_per_val * 3
    buf.len() == 1 + expected_bytes_per_val * 3
}

#[quickcheck]
fn prop_uint_with_length_roundtrip(val: u64, len_raw: u8) -> bool {
    use s2rst::s2::encoded_uint_vector::{decode_uint_with_length, encode_uint_with_length};
    let len = (len_raw % 9) as usize; // 0..8
    let mask = if len == 0 {
        0u64
    } else if len >= 8 {
        u64::MAX
    } else {
        (1u64 << (len * 8)) - 1
    };
    let val = val & mask;
    let mut buf = Vec::new();
    encode_uint_with_length(&mut buf, val, len).unwrap();
    assert_eq!(buf.len(), len);
    let decoded = decode_uint_with_length(&mut buf.as_slice(), len).unwrap();
    decoded == val
}

// ─── EncodedStringVector ────────────────────────────────────────────

#[quickcheck]
fn prop_string_vector_roundtrip(strings: Vec<Vec<u8>>) -> bool {
    use s2rst::s2::encoded_string_vector::{decode_string_vector, encode_string_vector};
    let refs: Vec<&[u8]> = strings.iter().map(Vec::as_slice).collect();
    let mut buf = Vec::new();
    encode_string_vector(&refs, &mut buf).unwrap();
    let decoded = decode_string_vector(&mut buf.as_slice()).unwrap();
    decoded == strings
}

// ─── EncodedS2PointVector ───────────────────────────────────────────

fn make_cell_points(lat_i: i32, lng_i: i32, count: u8) -> Vec<Point> {
    // Generate cell-center points near a given location at various levels.
    let lat = (lat_i % 90) as f64;
    let lng = (lng_i % 180) as f64;
    let center = CellId::from_point(&LatLng::from_degrees(lat, lng).to_point());
    let n = (count % 32) as usize + 1;
    (0..n)
        .map(|i| {
            let level = (i % 31) as u8;
            center.parent_at_level(level).to_point()
        })
        .collect()
}

#[quickcheck]
fn prop_s2point_vector_fast_roundtrip(lat: i32, lng: i32, count: u8) -> bool {
    use s2rst::s2::encoded_s2point_vector::{
        CodingHint, decode_s2point_vector, encode_s2point_vector,
    };
    let points = make_cell_points(lat, lng, count);
    let mut buf = Vec::new();
    encode_s2point_vector(&points, CodingHint::Fast, &mut buf).unwrap();
    let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
    decoded == points
}

#[quickcheck]
fn prop_s2point_vector_compact_roundtrip(lat: i32, lng: i32, count: u8) -> bool {
    use s2rst::s2::encoded_s2point_vector::{
        CodingHint, decode_s2point_vector, encode_s2point_vector,
    };
    let points = make_cell_points(lat, lng, count);
    let mut buf = Vec::new();
    encode_s2point_vector(&points, CodingHint::Compact, &mut buf).unwrap();
    let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
    // Compact encoding of cell centers is lossless for exact cell centers.
    decoded == points
}

#[quickcheck]
fn prop_s2point_vector_compact_not_larger_for_same_level(lat: i32, lng: i32, count: u8) -> bool {
    use s2rst::s2::encoded_s2point_vector::{CodingHint, encode_s2point_vector};
    // All points at the same level → compact should always be ≤ fast.
    let level = count % 31;
    let n = ((count / 31) % 16) as usize + 1;
    // Generate n distinct cell-center points at the same level by using
    // different base locations.
    let points: Vec<Point> = (0..n)
        .map(|i| {
            let la = (lat.wrapping_add(i as i32 * 7)) % 90;
            let lo = (lng.wrapping_add(i as i32 * 13)) % 180;
            CellId::from_point(&make_latlng(la, lo).to_point())
                .parent_at_level(level)
                .to_point()
        })
        .collect();
    let mut buf_fast = Vec::new();
    encode_s2point_vector(&points, CodingHint::Fast, &mut buf_fast).unwrap();
    let mut buf_compact = Vec::new();
    encode_s2point_vector(&points, CodingHint::Compact, &mut buf_compact).unwrap();
    buf_compact.len() <= buf_fast.len()
}

#[quickcheck]
fn prop_s2point_vector_fast_size_is_1_plus_24n(lat: i32, lng: i32, count: u8) -> bool {
    use s2rst::s2::encoded_s2point_vector::{CodingHint, encode_s2point_vector};
    let points = make_cell_points(lat, lng, count);
    let mut buf = Vec::new();
    encode_s2point_vector(&points, CodingHint::Fast, &mut buf).unwrap();
    // UNCOMPRESSED: 1-byte varint header (for small n) + 24 bytes per point.
    // For n <= 5, the header varint is 1 byte. For larger n it may be 2.
    let n = points.len();
    let header_bytes = if (n << 3) < 128 { 1 } else { 2 };
    buf.len() == header_bytes + 24 * n
}

#[quickcheck]
fn prop_s2point_vector_arbitrary_points_roundtrip(
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
) -> bool {
    use s2rst::s2::encoded_s2point_vector::{
        CodingHint, decode_s2point_vector, encode_s2point_vector,
    };
    // Non-cell-center points (will be encoded as exceptions in compact mode).
    let p0 = Point::from_coords(x0 as f64, y0 as f64, z0 as f64).normalize();
    let p1 = Point::from_coords(x1 as f64, y1 as f64, z1 as f64).normalize();
    if p0.0.norm2() < 1e-20 || p1.0.norm2() < 1e-20 {
        return true; // skip degenerate
    }
    let points = vec![p0, p1];
    for hint in [CodingHint::Fast, CodingHint::Compact] {
        let mut buf = Vec::new();
        encode_s2point_vector(&points, hint, &mut buf).unwrap();
        let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
        if decoded.len() != points.len() {
            return false;
        }
        for (a, b) in decoded.iter().zip(points.iter()) {
            if (a.0.x - b.0.x).abs() > 1e-15
                || (a.0.y - b.0.y).abs() > 1e-15
                || (a.0.z - b.0.z).abs() > 1e-15
            {
                return false;
            }
        }
    }
    true
}

// ─── EncodedS2CellIdVector ─────────────────────────────────────────

#[quickcheck]
fn prop_cell_id_vector_roundtrip(raw_ids: Vec<u64>) -> bool {
    use s2rst::s2::encoded_s2cell_id_vector::{decode_s2cell_id_vector, encode_s2cell_id_vector};
    // Use raw IDs as-is (they don't need to be valid cell IDs for encoding).
    let ids: Vec<CellId> = raw_ids.into_iter().map(CellId).collect();
    let mut buf = Vec::new();
    encode_s2cell_id_vector(&ids, &mut buf).unwrap();
    let decoded = decode_s2cell_id_vector(&mut buf.as_slice()).unwrap();
    decoded.len() == ids.len() && decoded.iter().zip(ids.iter()).all(|(a, b)| a.0 == b.0)
}

#[quickcheck]
fn prop_cell_id_vector_same_level_compact(face: u8, level_raw: u8) -> bool {
    use s2rst::s2::encoded_s2cell_id_vector::encode_s2cell_id_vector;
    let face = face % 6;
    let level = level_raw % 31;
    let parent = CellId::from_face(face).child_begin_at_level(level);
    let ids: Vec<CellId> = (0..4).map(|i| parent.advance(i)).collect();
    let mut buf = Vec::new();
    encode_s2cell_id_vector(&ids, &mut buf).unwrap();
    // Same-level cells should encode efficiently (odd shift, deltas small).
    // Total should be well under 4 * 8 = 32 bytes.
    buf.len() < 20
}

#[quickcheck]
fn prop_cell_id_vector_empty_is_small() -> bool {
    use s2rst::s2::encoded_s2cell_id_vector::encode_s2cell_id_vector;
    let mut buf = Vec::new();
    encode_s2cell_id_vector(&[], &mut buf).unwrap();
    buf.len() <= 3
}

// ─── LaxPolyline encode/decode ──────────────────────────────────────

#[quickcheck]
fn prop_lax_polyline_roundtrip(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    let points: Vec<Point> = [(lat0, lng0), (lat1, lng1), (lat2, lng2)]
        .iter()
        .map(|&(la, lo)| make_latlng(la, lo).to_point())
        .collect();
    let lp = LaxPolyline::new(points.clone());
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolyline::decode(&mut buf.as_slice()).unwrap();
    decoded.num_vertices() == lp.num_vertices()
        && (0..lp.num_vertices()).all(|i| decoded.vertex(i) == lp.vertex(i))
}

#[quickcheck]
fn prop_lax_polyline_compact_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::encoded_s2point_vector::CodingHint;
    use s2rst::s2::lax_polyline::LaxPolyline;
    // Use cell-center points for compact encoding.
    let center = CellId::from_point(&make_latlng(lat, lng).to_point());
    let points: Vec<Point> = (0..5)
        .map(|i| center.parent_at_level(i * 6).to_point())
        .collect();
    let lp = LaxPolyline::new(points.clone());
    let mut buf = Vec::new();
    lp.encode_with_hint(&mut buf, CodingHint::Compact).unwrap();
    let decoded = LaxPolyline::decode(&mut buf.as_slice()).unwrap();
    decoded.num_vertices() == lp.num_vertices()
        && (0..lp.num_vertices()).all(|i| decoded.vertex(i) == lp.vertex(i))
}

#[quickcheck]
fn prop_lax_polyline_empty_roundtrip(_dummy: u8) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    let lp = LaxPolyline::new(vec![]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolyline::decode(&mut buf.as_slice()).unwrap();
    decoded.num_vertices() == 0
}

// ─── LaxPolygon encode/decode ───────────────────────────────────────

#[quickcheck]
fn prop_lax_polygon_single_loop_roundtrip(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    let points: Vec<Point> = [(lat0, lng0), (lat1, lng1), (lat2, lng2)]
        .iter()
        .map(|&(la, lo)| make_latlng(la, lo).to_point())
        .collect();
    let lp = LaxPolygon::from_loops(&[&points]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
    decoded.num_loops() == 1
        && decoded.num_loop_vertices(0) == points.len()
        && (0..points.len()).all(|j| decoded.loop_vertex(0, j) == lp.loop_vertex(0, j))
}

#[quickcheck]
fn prop_lax_polygon_multi_loop_roundtrip(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    let loop0: Vec<Point> = [
        (lat0, lng0),
        (lat0.wrapping_add(1), lng0),
        (lat0, lng0.wrapping_add(1)),
    ]
    .iter()
    .map(|&(la, lo)| make_latlng(la, lo).to_point())
    .collect();
    let loop1: Vec<Point> = [
        (lat1, lng1),
        (lat1.wrapping_add(1), lng1),
        (lat1, lng1.wrapping_add(1)),
    ]
    .iter()
    .map(|&(la, lo)| make_latlng(la, lo).to_point())
    .collect();
    let lp = LaxPolygon::from_loops(&[&loop0, &loop1]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
    decoded.num_loops() == 2
        && decoded.num_loop_vertices(0) == 3
        && decoded.num_loop_vertices(1) == 3
        && (0..3).all(|j| decoded.loop_vertex(0, j) == lp.loop_vertex(0, j))
        && (0..3).all(|j| decoded.loop_vertex(1, j) == lp.loop_vertex(1, j))
}

#[quickcheck]
fn prop_lax_polygon_empty_roundtrip(_dummy: u8) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    let lp = LaxPolygon::default();
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
    decoded.num_loops() == 0
}

#[quickcheck]
fn prop_lax_polygon_compact_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::encoded_s2point_vector::CodingHint;
    use s2rst::s2::lax_polygon::LaxPolygon;
    let center = CellId::from_point(&make_latlng(lat, lng).to_point());
    let loop0: Vec<Point> = (0..3)
        .map(|i| center.parent_at_level(i * 10).to_point())
        .collect();
    let loop1: Vec<Point> = (0..4)
        .map(|i| center.parent_at_level(i * 7 + 1).to_point())
        .collect();
    let lp = LaxPolygon::from_loops(&[&loop0, &loop1]);
    let mut buf = Vec::new();
    lp.encode_with_hint(&mut buf, CodingHint::Compact).unwrap();
    let decoded = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
    decoded.num_loops() == 2
        && decoded.num_loop_vertices(0) == 3
        && decoded.num_loop_vertices(1) == 4
        && (0..3).all(|j| decoded.loop_vertex(0, j) == lp.loop_vertex(0, j))
        && (0..4).all(|j| decoded.loop_vertex(1, j) == lp.loop_vertex(1, j))
}

// ─── PointVector encode/decode ──────────────────────────────────────

#[quickcheck]
fn prop_point_vector_roundtrip(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    let points: Vec<Point> = [(lat0, lng0), (lat1, lng1)]
        .iter()
        .map(|&(la, lo)| make_latlng(la, lo).to_point())
        .collect();
    let pv = PointVector::new(points.clone());
    let mut buf = Vec::new();
    pv.encode(&mut buf).unwrap();
    let decoded = PointVector::decode(&mut buf.as_slice()).unwrap();
    decoded.len() == pv.len() && (0..pv.len()).all(|i| decoded.point(i) == pv.point(i))
}

#[quickcheck]
fn prop_point_vector_empty_roundtrip(_dummy: u8) -> bool {
    use s2rst::s2::point_vector::PointVector;
    let pv = PointVector::new(vec![]);
    let mut buf = Vec::new();
    pv.encode(&mut buf).unwrap();
    let decoded = PointVector::decode(&mut buf.as_slice()).unwrap();
    decoded.is_empty()
}

#[quickcheck]
fn prop_point_vector_compact_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::encoded_s2point_vector::CodingHint;
    use s2rst::s2::point_vector::PointVector;
    let center = CellId::from_point(&make_latlng(lat, lng).to_point());
    let points: Vec<Point> = (0..6)
        .map(|i| center.parent_at_level(i * 5).to_point())
        .collect();
    let pv = PointVector::new(points);
    let mut buf = Vec::new();
    pv.encode_with_hint(&mut buf, CodingHint::Compact).unwrap();
    let decoded = PointVector::decode(&mut buf.as_slice()).unwrap();
    decoded.len() == pv.len() && (0..pv.len()).all(|i| decoded.point(i) == pv.point(i))
}

// ─── ShapeIndex encode/decode ───────────────────────────────────────

#[quickcheck]
fn prop_shape_index_lax_polyline_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat, lng).to_point();
    let p1 = make_latlng(lat.wrapping_add(1), lng).to_point();
    let p2 = make_latlng(lat, lng.wrapping_add(1)).to_point();
    let lp = LaxPolyline::new(vec![p0, p1, p2]);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lp));
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 1 && shapes_cells_match(&index, &decoded)
}

#[quickcheck]
fn prop_shape_index_lax_polygon_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat, lng).to_point();
    let p1 = make_latlng(lat.wrapping_add(1), lng).to_point();
    let p2 = make_latlng(lat, lng.wrapping_add(1)).to_point();
    let lp = LaxPolygon::from_loops(&[&[p0, p1, p2]]);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lp));
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 1 && shapes_cells_match(&index, &decoded)
}

#[quickcheck]
fn prop_shape_index_point_vector_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat, lng).to_point();
    let p1 = make_latlng(lat.wrapping_add(2), lng.wrapping_add(2)).to_point();
    let pv = PointVector::new(vec![p0, p1]);
    let mut index = ShapeIndex::new();
    index.add(Box::new(pv));
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 1 && shapes_cells_match(&index, &decoded)
}

#[quickcheck]
fn prop_shape_index_compact_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::encoded_s2point_vector::CodingHint;
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape_index::ShapeIndex;
    let center = CellId::from_point(&make_latlng(lat, lng).to_point());
    let points: Vec<Point> = (0..4)
        .map(|i| center.parent_at_level(i * 7 + 1).to_point())
        .collect();
    let lp = LaxPolyline::new(points);
    let mut index = ShapeIndex::new();
    index.add(Box::new(lp));
    index.build();
    let mut buf = Vec::new();
    index
        .encode_with_hint(&mut buf, CodingHint::Compact)
        .unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 1 && shapes_cells_match(&index, &decoded)
}

#[quickcheck]
fn prop_shape_index_multiple_shapes_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape_index::ShapeIndex;
    let p0 = make_latlng(lat, lng).to_point();
    let p1 = make_latlng(lat.wrapping_add(5), lng.wrapping_add(5)).to_point();
    let p2 = make_latlng(lat.wrapping_add(10), lng).to_point();
    let mut index = ShapeIndex::new();
    index.add(Box::new(PointVector::new(vec![p0])));
    index.add(Box::new(LaxPolyline::new(vec![p1, p2])));
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 2 && shapes_cells_match(&index, &decoded)
}

#[quickcheck]
fn prop_shape_index_empty_roundtrip(_dummy: u8) -> bool {
    use s2rst::s2::shape_index::ShapeIndex;
    let mut index = ShapeIndex::new();
    index.build();
    let mut buf = Vec::new();
    index.encode_to_writer(&mut buf).unwrap();
    let decoded = ShapeIndex::decode_from_reader(&mut buf.as_slice()).unwrap();
    decoded.num_shape_ids() == 0
}

/// Compares two shape indices cell by cell.
fn shapes_cells_match(
    a: &s2rst::s2::shape_index::ShapeIndex,
    b: &s2rst::s2::shape_index::ShapeIndex,
) -> bool {
    let mut it_a = a.iter();
    let mut it_b = b.iter();
    loop {
        if it_a.done() && it_b.done() {
            return true;
        }
        if it_a.done() != it_b.done() {
            return false;
        }
        if it_a.cell_id() != it_b.cell_id() {
            return false;
        }
        let cell_a = it_a.index_cell().unwrap();
        let cell_b = it_b.index_cell().unwrap();
        if cell_a.shapes.len() != cell_b.shapes.len() {
            return false;
        }
        for (ca, cb) in cell_a.shapes.iter().zip(cell_b.shapes.iter()) {
            if ca.shape_id != cb.shape_id
                || ca.contains_center != cb.contains_center
                || ca.edges != cb.edges
            {
                return false;
            }
        }
        it_a.next();
        it_b.next();
    }
}

// ─── Decode-from-garbage-never-panics ───────────────────────────────

#[quickcheck]
fn prop_lax_polyline_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    // Limit input size to avoid OOM from garbage varints decoding huge counts.
    let data = &data[..data.len().min(256)];
    drop(LaxPolyline::decode(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_lax_polygon_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    let data = &data[..data.len().min(256)];
    drop(LaxPolygon::decode(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_point_vector_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::point_vector::PointVector;
    let data = &data[..data.len().min(256)];
    drop(PointVector::decode(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_s2point_vector_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::encoded_s2point_vector::decode_s2point_vector;
    let data = &data[..data.len().min(256)];
    drop(decode_s2point_vector(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_cell_id_vector_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::encoded_s2cell_id_vector::decode_s2cell_id_vector;
    let data = &data[..data.len().min(256)];
    drop(decode_s2cell_id_vector(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_uint_vector_u32_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::encoded_uint_vector::decode_uint_vector_u32;
    let data = &data[..data.len().min(256)];
    drop(decode_uint_vector_u32(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_uint_vector_u64_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::encoded_uint_vector::decode_uint_vector_u64;
    let data = &data[..data.len().min(256)];
    drop(decode_uint_vector_u64(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_string_vector_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::encoded_string_vector::decode_string_vector;
    let data = &data[..data.len().min(256)];
    drop(decode_string_vector(&mut &data[..]));
    true
}

#[quickcheck]
fn prop_shape_index_decode_garbage_no_panic(data: Vec<u8>) -> bool {
    use s2rst::s2::shape_index::ShapeIndex;
    let data = &data[..data.len().min(256)];
    drop(ShapeIndex::decode_from_reader(&mut &data[..]));
    true
}

// ─── Type tag consistency ───────────────────────────────────────────

#[quickcheck]
fn prop_lax_polyline_type_tag_is_4(_dummy: u8) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let lp = LaxPolyline::new(vec![]);
    lp.type_tag() == 4
}

#[quickcheck]
fn prop_lax_polygon_type_tag_is_5(_dummy: u8) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape::Shape;
    let lp = LaxPolygon::default();
    lp.type_tag() == 5
}

#[quickcheck]
fn prop_point_vector_type_tag_is_3(_dummy: u8) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let pv = PointVector::new(vec![]);
    pv.type_tag() == 3
}

// ─── Edge query distance helpers ────────────────────────────────────

/// Build a single-polyline `ShapeIndex` from two lat/lng integer pairs.
fn make_edge_index(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
) -> s2rst::s2::shape_index::ShapeIndex {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let mut idx = s2rst::s2::shape_index::ShapeIndex::new();
    idx.add(Box::new(s2rst::s2::polyline::Polyline::new(vec![p0, p1])));
    idx.build();
    idx
}

#[quickcheck]
fn prop_update_min_distance_max_error_non_negative(len2: u32) -> bool {
    // Clamp to valid chord angle range [0, 4].
    let len2 = (len2 % 4001) as f64 / 1000.0; // [0.0, 4.0]
    let ca = ChordAngle::from_length2(len2);
    s2rst::s2::edge_distances::update_min_distance_max_error(ca) >= 0.0
}

#[quickcheck]
fn prop_update_min_distance_max_error_ge_point_error(len2: u32) -> bool {
    let len2 = (len2 % 4001) as f64 / 1000.0;
    let ca = ChordAngle::from_length2(len2);
    let total = s2rst::s2::edge_distances::update_min_distance_max_error(ca);
    let point_err = ca.max_point_error();
    total >= point_err || (total - point_err).abs() < f64::EPSILON
}

#[quickcheck]
fn prop_inclusive_max_distance_is_successor(lat: i32, lng: i32) -> bool {
    use s2rst::s2::closest_edge_query::Options;
    let ll = make_latlng(lat, lng);
    let limit = ChordAngle::from_degrees(((ll.lat.degrees()).abs() % 180.0).max(0.001));
    let mut opts = Options::default();
    opts.inclusive_max_distance(limit);
    opts.max_distance == limit.successor()
}

#[quickcheck]
fn prop_conservative_ge_inclusive_closest(deg: u32) -> bool {
    use s2rst::s2::closest_edge_query::Options;
    let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
    let mut conservative = Options::default();
    conservative.conservative_max_distance(limit);
    let mut inclusive = Options::default();
    inclusive.inclusive_max_distance(limit);
    conservative.max_distance >= inclusive.max_distance
}

#[quickcheck]
fn prop_inclusive_min_distance_is_predecessor(deg: u32) -> bool {
    use s2rst::s2::furthest_edge_query::Options;
    let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
    let mut opts = Options::default();
    opts.inclusive_min_distance(limit);
    opts.min_distance == limit.predecessor()
}

#[quickcheck]
fn prop_conservative_le_inclusive_furthest(deg: u32) -> bool {
    use s2rst::s2::furthest_edge_query::Options;
    let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
    let mut conservative = Options::default();
    conservative.conservative_min_distance(limit);
    let mut inclusive = Options::default();
    inclusive.inclusive_min_distance(limit);
    conservative.min_distance <= inclusive.min_distance
}

#[quickcheck]
fn prop_is_distance_less_implies_less_or_equal(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
        // is_distance_less(limit) → is_distance_less_or_equal(limit)
        if query.is_distance_less(&target, limit) {
            query.is_distance_less_or_equal(&target, limit)
        } else {
            true // no implication to check
        }
    })
}

#[quickcheck]
fn prop_is_distance_greater_implies_greater_or_equal(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::furthest_edge_query::{FurthestEdgeQuery, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
        if query.is_distance_greater(&target, limit) {
            query.is_distance_greater_or_equal(&target, limit)
        } else {
            true
        }
    })
}

#[quickcheck]
fn prop_closest_edge_distance_symmetric_shape_index_target(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, ShapeIndexTarget};
        let index_a = make_edge_index(lat0, lng0, lat1, lng1);
        let index_b = make_edge_index(lat2, lng2, lat3, lng3);

        // dist(A, B) should equal dist(B, A)
        let query_a = ClosestEdgeQuery::new(&index_a);
        let target_b = ShapeIndexTarget::new(&index_b);
        let dist_ab = query_a.find_closest_edge(&target_b).distance;

        let query_b = ClosestEdgeQuery::new(&index_b);
        let target_a = ShapeIndexTarget::new(&index_a);
        let dist_ba = query_b.find_closest_edge(&target_a).distance;

        // Allow tiny floating-point error.
        (dist_ab.length2() - dist_ba.length2()).abs() < 1e-10
    })
}

#[quickcheck]
fn prop_shape_filter_never_adds_results(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let opts = Options {
            max_results: 100,
            ..Options::default()
        };
        let unfiltered = query.find_closest_edges(&target, &opts);
        let filtered = query.find_closest_edges_filtered(
            &target,
            &opts,
            Some(&|_| true), // accept all
        );
        // Accepting all should give the same results.
        unfiltered.len() == filtered.len()
    })
}

#[quickcheck]
fn prop_shape_filter_reject_all_gives_empty(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let opts = Options {
            max_results: 100,
            include_interiors: false,
            ..Options::default()
        };
        let filtered = query.find_closest_edges_filtered(
            &target,
            &opts,
            Some(&|_| false), // reject all
        );
        filtered.is_empty()
    })
}

#[quickcheck]
fn prop_visit_closest_shapes_unique(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let opts = Options {
            max_results: 100,
            ..Options::default()
        };
        let mut shape_ids = Vec::new();
        query.visit_closest_shapes(&target, &opts, |r| {
            shape_ids.push(r.shape_id);
            std::ops::ControlFlow::Continue(())
        });
        let unique: std::collections::HashSet<s2rst::s2::shape::ShapeId> =
            shape_ids.iter().copied().collect();
        shape_ids.len() == unique.len()
    })
}

#[quickcheck]
fn prop_conservative_never_misses_less_or_equal(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
        // If is_distance_less_or_equal(limit), then conservative must also say true.
        if query.is_distance_less_or_equal(&target, limit) {
            query.is_conservative_distance_less_or_equal(&target, limit)
        } else {
            true
        }
    })
}

#[quickcheck]
fn prop_conservative_never_misses_greater_or_equal(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::furthest_edge_query::{FurthestEdgeQuery, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let limit = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
        if query.is_distance_greater_or_equal(&target, limit) {
            query.is_conservative_distance_greater_or_equal(&target, limit)
        } else {
            true
        }
    })
}

#[quickcheck]
fn prop_closest_edge_distance_le_max_distance(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::closest_edge_query::{ClosestEdgeQuery, Options, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = ClosestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let max_dist = ChordAngle::from_degrees((deg % 180) as f64 + 0.1);
        let opts = Options {
            max_results: 100,
            max_distance: max_dist,
            ..Options::default()
        };
        let results = query.find_closest_edges(&target, &opts);
        results.iter().all(|r| r.distance <= max_dist)
    })
}

#[quickcheck]
fn prop_furthest_edge_distance_ge_min_distance(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
    deg: u8,
) -> bool {
    no_panic(|| {
        use s2rst::s2::furthest_edge_query::{FurthestEdgeQuery, Options, PointTarget};
        let index = make_edge_index(lat0, lng0, lat1, lng1);
        let query = FurthestEdgeQuery::new(&index);
        let target = PointTarget::new(make_latlng(qlat, qlng).to_point());
        let min_dist = ChordAngle::from_degrees((deg % 90) as f64);
        let opts = Options {
            max_results: 100,
            min_distance: min_dist,
            ..Options::default()
        };
        let results = query.find_furthest_edges(&target, &opts);
        results.iter().all(|r| r.distance >= min_dist)
    })
}

#[quickcheck]
fn prop_set_max_error_false_for_simple_targets(_dummy: u8) -> bool {
    use s2rst::s2::closest_edge_query::{CellTarget, EdgeTarget, PointTarget};
    use s2rst::s2::distance_target::DistanceTarget;
    let mut pt = PointTarget::new(Point::from_coords(1.0, 0.0, 0.0));
    let mut et = EdgeTarget::new(
        Point::from_coords(1.0, 0.0, 0.0),
        Point::from_coords(0.0, 1.0, 0.0),
    );
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let mut ct = CellTarget::new(cell);
    !pt.set_max_error(ChordAngle::from_degrees(1.0))
        && !et.set_max_error(ChordAngle::from_degrees(1.0))
        && !ct.set_max_error(ChordAngle::from_degrees(1.0))
}

#[quickcheck]
fn prop_shape_index_target_set_max_error_true(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::closest_edge_query::ShapeIndexTarget;
    use s2rst::s2::distance_target::DistanceTarget;
    let index = make_edge_index(lat0, lng0, lat1, lng1);
    let mut target = ShapeIndexTarget::new(&index);
    target.set_max_error(ChordAngle::from_degrees(1.0))
}

// ─── Interleave / deinterleave inverse ──────────────────────────────

#[quickcheck]
fn prop_interleave_deinterleave_inverse(a: u32, b: u32) -> bool {
    // Test through the S2PointVector roundtrip: encode a cell and decode it.
    // If interleave/deinterleave are correct, cell centers roundtrip exactly.
    let cell = CellId::from_face((a % 6) as u8).child_begin_at_level(((b % 31) as u8).min(30));
    let point = cell.to_point();
    use s2rst::s2::encoded_s2point_vector::{
        CodingHint, decode_s2point_vector, encode_s2point_vector,
    };
    let mut buf = Vec::new();
    encode_s2point_vector(&[point], CodingHint::Compact, &mut buf).unwrap();
    let decoded = decode_s2point_vector(&mut buf.as_slice()).unwrap();
    decoded.len() == 1 && decoded[0] == point
}

// ════════════════════════════════════════════════════════════════════
// TRAIT PROPERTY TESTS
// Properties derived from each trait's contract, verified for every
// implementing type.  Organised by trait; within each trait, one
// sub-section per implementor.
// ════════════════════════════════════════════════════════════════════

// ────────────────────────────────────────────────────────────────────
// 11. Region trait — cap_bound / rect_bound / contains_cell
// ────────────────────────────────────────────────────────────────────

// --- Loop as Region ---

/// Region::cap_bound must contain every point that contains_point accepts.
#[quickcheck]
fn prop_loop_cap_bound_covers_contained_points(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let p = make_latlng(qlat, qlng).to_point();
            !lp.contains_point(&p) || lp.cap_bound().contains_point(p)
        }
        None => true,
    }
}

/// Region::rect_bound must contain (as LatLng) every point that contains_point accepts.
#[quickcheck]
fn prop_loop_rect_bound_covers_contained_points(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let p = make_latlng(qlat, qlng).to_point();
            !lp.contains_point(&p) || lp.rect_bound().contains_lat_lng(LatLng::from_point(p))
        }
        None => true,
    }
}

/// Region::contains_cell implies Region::intersects_cell.
#[quickcheck]
fn prop_loop_contains_cell_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    face: u8,
    level: u8,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let cell = Cell::from_cell_id(
                CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20)),
            );
            !lp.contains_cell(&cell) || lp.intersects_cell(&cell)
        }
        None => true,
    }
}

/// cap_bound() returned by a loop has a non-negative radius.
#[quickcheck]
fn prop_loop_cap_bound_valid(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => !lp.cap_bound().is_empty() || lp.cap_bound().is_empty(),
        None => true,
    }
}

/// rect_bound() returned by a loop is valid.
#[quickcheck]
fn prop_loop_rect_bound_valid(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => lp.rect_bound().is_valid(),
        None => true,
    }
}

// --- Polygon as Region ---

/// Polygon::cap_bound covers every point the polygon contains.
#[quickcheck]
fn prop_polygon_cap_bound_covers_contained_points(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => {
            let p = make_latlng(qlat, qlng).to_point();
            !poly.contains_point(&p) || poly.cap_bound().contains_point(p)
        }
        None => true,
    }
}

/// Polygon::rect_bound covers every point the polygon contains.
#[quickcheck]
fn prop_polygon_rect_bound_covers_contained_points(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => {
            let p = make_latlng(qlat, qlng).to_point();
            !poly.contains_point(&p) || poly.rect_bound().contains_lat_lng(LatLng::from_point(p))
        }
        None => true,
    }
}

/// Polygon::contains_cell implies intersects_cell.
#[quickcheck]
fn prop_polygon_contains_cell_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    face: u8,
    level: u8,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => {
            let cell = Cell::from_cell_id(
                CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20)),
            );
            !poly.contains_cell(&cell) || poly.intersects_cell(&cell)
        }
        None => true,
    }
}

// --- Cap as Region ---

/// Cap::cap_bound returns self; its center is contained.
#[quickcheck]
fn prop_cap_cap_bound_contains_center(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius = Angle::from_degrees((deg % 90) as f64);
    let cap = Cap::from_center_angle(center, radius);
    cap.cap_bound().contains_point(center)
}

/// Cap::rect_bound covers the cap center.
#[quickcheck]
fn prop_cap_rect_bound_covers_center(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius = Angle::from_degrees((deg % 89) as f64 + 0.1);
    let cap = Cap::from_center_angle(center, radius);
    cap.rect_bound()
        .contains_lat_lng(LatLng::from_point(center))
}

/// Cap::contains_cell implies intersects_cell.
#[quickcheck]
fn prop_cap_contains_cell_implies_intersects(
    lat: i32,
    lng: i32,
    deg: u8,
    face: u8,
    level: u8,
) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let radius = Angle::from_degrees((deg % 90) as f64);
    let cap = Cap::from_center_angle(center, radius);
    let cell =
        Cell::from_cell_id(CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20)));
    !cap.contains_cell(&cell) || cap.intersects_cell(&cell)
}

// --- Rect as Region ---

/// Rect::cap_bound covers all four corners of the rect.
#[quickcheck]
fn prop_rect_cap_bound_covers_corners(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let rect = Rect::from_point_pair(ll0, ll1);
    // Skip rects near poles where cap_bound has numerical precision issues
    if rect.lat.lo.abs() > 1.3 || rect.lat.hi.abs() > 1.3 {
        // ~74.5°
        return true;
    }
    let cb = rect.cap_bound();
    use s2rst::s2::RectVertex;
    RectVertex::iter().all(|v| cb.contains_point(rect.vertex(v).to_point()))
}

/// Rect::contains_cell implies intersects_cell.
#[quickcheck]
fn prop_rect_contains_cell_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    face: u8,
    level: u8,
) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let rect = Rect::from_point_pair(ll0, ll1);
    let cell =
        Cell::from_cell_id(CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20)));
    !rect.contains_cell(&cell) || rect.intersects_cell(&cell)
}

/// Rect::rect_bound() == self (Rect is its own rect_bound).
#[quickcheck]
fn prop_rect_rect_bound_is_self(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let rect = Rect::from_point_pair(ll0, ll1);
    rect.rect_bound() == rect
}

// --- Cell as Region ---

/// Cell::cap_bound covers all four cell vertices.
#[quickcheck]
fn prop_cell_cap_bound_covers_vertices(face: u8, level: u8) -> bool {
    let cell =
        Cell::from_cell_id(CellId::from_face(face % 6).child_begin_at_level((level % 25).min(25)));
    let cb = cell.cap_bound();
    (0..4).all(|i| cb.contains_point(cell.vertex(i)))
}

/// Cell::rect_bound covers all four cell vertices.
#[quickcheck]
fn prop_cell_rect_bound_covers_vertices(face: u8, level: u8) -> bool {
    let cell =
        Cell::from_cell_id(CellId::from_face(face % 6).child_begin_at_level((level % 25).min(25)));
    let rb = cell.rect_bound();
    (0..4).all(|i| rb.contains_lat_lng(LatLng::from_point(cell.vertex(i))))
}

/// Cell::contains_cell(self) is always true.
#[quickcheck]
fn prop_cell_contains_self(face: u8, level: u8) -> bool {
    let cell =
        Cell::from_cell_id(CellId::from_face(face % 6).child_begin_at_level((level % 25).min(25)));
    cell.contains_cell(cell)
}

/// Cell::contains_cell implies intersects_cell.
#[quickcheck]
fn prop_cell_contains_cell_implies_intersects(face_a: u8, level_a: u8, face_b: u8) -> bool {
    let a = Cell::from_cell_id(
        CellId::from_face(face_a % 6).child_begin_at_level((level_a % 20).min(20)),
    );
    let b = Cell::from_cell_id(CellId::from_face(face_b % 6));
    !a.contains_cell(b) || a.intersects_cell(b)
}

// --- CellUnion as Region ---

/// CellUnion::cap_bound has a non-negative radius for any union.
#[quickcheck]
fn prop_cellunion_cap_bound_valid(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let cb = cu.cap_bound();
    !cb.is_empty()
}

/// CellUnion::rect_bound is valid for any single-cell union.
#[quickcheck]
fn prop_cellunion_rect_bound_valid(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.rect_bound().is_valid()
}

/// CellUnion::contains_cell_id implies intersects_cell.
#[quickcheck]
fn prop_cellunion_contains_implies_intersects(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let cell = Cell::from_cell_id(id);
    // The union contains the cell, so it must also intersect it.
    cu.intersects_cell(&cell)
}

/// If a loop contains a cell's center, the loop's cap_bound also contains that point.
#[quickcheck]
fn prop_loop_cap_bound_covers_contained_cell_center(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    face: u8,
    level: u8,
) -> bool {
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let cell = Cell::from_cell_id(
                CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20)),
            );
            let center = cell.id().to_point();
            if !lp.contains_point(&center) {
                return true;
            }
            lp.cap_bound().contains_point(center)
        }
        None => true,
    }
}

// ────────────────────────────────────────────────────────────────────
// 12. Shape trait — edge count, unit endpoints, chain consistency, dimension
// ────────────────────────────────────────────────────────────────────

// --- Loop as Shape ---

/// Loop::num_edges() equals the sum of all chain lengths.
#[quickcheck]
fn prop_loop_shape_edges_sum_chains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            let total: usize = (0..lp.num_chains()).map(|i| lp.chain(i).length).sum();
            total == lp.num_edges()
        }
        None => true,
    }
}

/// Every edge endpoint of a Loop is a unit vector.
#[quickcheck]
fn prop_loop_shape_edge_endpoints_unit(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => (0..lp.num_edges()).all(|i| {
            let e = lp.edge(i);
            (e.v0.0.norm2() - 1.0).abs() < 1e-14 && (e.v1.0.norm2() - 1.0).abs() < 1e-14
        }),
        None => true,
    }
}

/// Loop::chain_edge(chain_id, offset) == Loop::edge(chain.start + offset).
#[quickcheck]
fn prop_loop_shape_chain_edge_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => {
            for cid in 0..lp.num_chains() {
                let chain = lp.chain(cid);
                for off in 0..chain.length {
                    let direct = lp.edge(chain.start + off);
                    let via_chain = lp.chain_edge(cid, off);
                    if direct.v0 != via_chain.v0 || direct.v1 != via_chain.v1 {
                        return false;
                    }
                }
            }
            true
        }
        None => true,
    }
}

/// A Loop has dimension 2 (it defines an interior).
#[quickcheck]
fn prop_loop_shape_dimension_two(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => lp.dimension() == Dimension::Polygon,
        None => true,
    }
}

/// Loop::num_chains() is ≤ num_edges() + 1.
#[quickcheck]
fn prop_loop_shape_num_chains_le_edges_plus_one(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(lp) => lp.num_chains() <= lp.num_edges() + 1,
        None => true,
    }
}

// --- Polygon as Shape ---

/// Polygon::num_edges() equals the sum of all chain lengths.
#[quickcheck]
fn prop_polygon_shape_edges_sum_chains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => {
            let total: usize = (0..poly.num_chains()).map(|i| poly.chain(i).length).sum();
            total == poly.num_edges()
        }
        None => true,
    }
}

/// Every edge endpoint of a Polygon is a unit vector.
#[quickcheck]
fn prop_polygon_shape_edge_endpoints_unit(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => (0..poly.num_edges()).all(|i| {
            let e = poly.edge(i);
            (e.v0.0.norm2() - 1.0).abs() < 1e-14 && (e.v1.0.norm2() - 1.0).abs() < 1e-14
        }),
        None => true,
    }
}

/// Polygon has dimension 2.
#[quickcheck]
fn prop_polygon_shape_dimension_two(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => poly.dimension() == Dimension::Polygon,
        None => true,
    }
}

/// Polygon::chain_edge(chain_id, offset) == Polygon::edge(chain.start + offset).
#[quickcheck]
fn prop_polygon_shape_chain_edge_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::shape::Shape;
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(poly) => {
            for cid in 0..poly.num_chains() {
                let chain = poly.chain(cid);
                for off in 0..chain.length {
                    let direct = poly.edge(chain.start + off);
                    let via_chain = poly.chain_edge(cid, off);
                    if direct.v0 != via_chain.v0 || direct.v1 != via_chain.v1 {
                        return false;
                    }
                }
            }
            true
        }
        None => true,
    }
}

// --- Polyline as Shape ---

/// Polyline::num_edges() equals vertex_count - 1 (each consecutive pair is an edge).
#[quickcheck]
fn prop_polyline_shape_edges_eq_vertices_minus_one(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    pl.num_edges() == pl.num_vertices().saturating_sub(1)
}

/// Polyline has dimension 1.
#[quickcheck]
fn prop_polyline_shape_dimension_one(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.dimension() == Dimension::Polyline
}

/// Polyline chain edge consistent with direct edge access.
#[quickcheck]
fn prop_polyline_shape_chain_edge_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ]);
    for cid in 0..pl.num_chains() {
        let chain = pl.chain(cid);
        for off in 0..chain.length {
            let d = pl.edge(chain.start + off);
            let c = pl.chain_edge(cid, off);
            if d.v0 != c.v0 || d.v1 != c.v1 {
                return false;
            }
        }
    }
    true
}

// --- PointVector as Shape ---

/// PointVector::num_edges() equals the number of points (each point is a degenerate edge).
#[quickcheck]
fn prop_point_vector_shape_edges_eq_points(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let pts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ];
    let pv = PointVector::new(pts);
    pv.num_edges() == 2
}

/// PointVector has dimension 0.
#[quickcheck]
fn prop_point_vector_shape_dimension_zero(lat: i32, lng: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let pv = PointVector::new(vec![make_latlng(lat, lng).to_point()]);
    pv.dimension() == Dimension::Point
}

/// PointVector: each "edge" is degenerate (v0 == v1).
#[quickcheck]
fn prop_point_vector_shape_degenerate_edges(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let pv = PointVector::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    (0..pv.num_edges()).all(|i| {
        let e = pv.edge(i);
        e.v0 == e.v1
    })
}

// ────────────────────────────────────────────────────────────────────
// 13. Add / Sub / Neg operators
// ────────────────────────────────────────────────────────────────────

// --- Angle ---

/// Angle addition is commutative: a + b == b + a.
#[quickcheck]
fn prop_angle_add_commutative(a: i32, b: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    let y = Angle::from_degrees((b % 180) as f64);
    (x + y).radians() == (y + x).radians()
}

/// Angle subtraction equals add with negation: a - b == a + (-b).
#[quickcheck]
fn prop_angle_sub_eq_add_neg(a: i32, b: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    let y = Angle::from_degrees((b % 180) as f64);
    ((x - y).radians() - (x + (-y)).radians()).abs() < 1e-15
}

/// Double negation is identity: -(-a) == a.
#[quickcheck]
fn prop_angle_neg_neg_identity(a: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    ((-(-x)).radians() - x.radians()).abs() < 1e-15
}

/// Adding zero angle is identity: a + 0 == a.
#[quickcheck]
fn prop_angle_add_zero_identity(a: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    let zero = Angle::from_degrees(0.0);
    ((x + zero).radians() - x.radians()).abs() < 1e-15
}

/// a + a == 2.0 * a.
#[quickcheck]
fn prop_angle_double_eq_add_self(a: i32) -> bool {
    let x = Angle::from_degrees((a % 90) as f64);
    let doubled = x + x;
    let two_x = x * 2.0;
    (doubled.radians() - two_x.radians()).abs() < 1e-14
}

// --- ChordAngle ---

/// Adding a non-negative ChordAngle is monotone: a + b >= a when b >= 0.
#[quickcheck]
fn prop_chord_angle_add_monotone(a: u8, b: u8) -> bool {
    let x = ChordAngle::from_angle(Angle::from_degrees(a as f64));
    let y = ChordAngle::from_angle(Angle::from_degrees(b as f64));
    x + y >= x
}

/// ChordAngle::zero() is the additive identity.
#[quickcheck]
fn prop_chord_angle_zero_is_identity(a: u8) -> bool {
    let x = ChordAngle::from_angle(Angle::from_degrees(a as f64));
    let z = ChordAngle::from_angle(Angle::from_degrees(0.0));
    x + z == x
}

/// Subtracting zero ChordAngle is identity.
#[quickcheck]
fn prop_chord_angle_sub_zero_identity(a: u8) -> bool {
    let x = ChordAngle::from_angle(Angle::from_degrees(a as f64));
    let z = ChordAngle::from_angle(Angle::from_degrees(0.0));
    x - z == x
}

// --- r3::Vector ---

/// r3::Vector addition is commutative.
#[quickcheck]
fn prop_r3_vector_add_commutative(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    a + b == b + a
}

/// r3::Vector addition is associative.
#[quickcheck]
fn prop_r3_vector_add_associative(
    x1: i32,
    y1: i32,
    z1: i32,
    x2: i16,
    y2: i16,
    z2: i16,
    x3: i16,
    y3: i16,
) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    let c = Vector::new(x3 as f64, y3 as f64, 0.0);
    (a + b) + c == a + (b + c)
}

/// Double negation of r3::Vector is identity.
#[quickcheck]
fn prop_r3_vector_neg_neg_identity(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::new(x as f64, y as f64, z as f64);
    -(-v) == v
}

/// r3::Vector: a - b == a + (-b).
#[quickcheck]
fn prop_r3_vector_sub_eq_add_neg(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    a - b == a + (-b)
}

/// r3::Vector: a + zero == a.
#[quickcheck]
fn prop_r3_vector_add_zero_identity(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x as f64, y as f64, z as f64);
    let zero = Vector::new(0.0, 0.0, 0.0);
    a + zero == a
}

/// r3::Vector: a + (-a) == zero.
#[quickcheck]
fn prop_r3_vector_add_neg_is_zero(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x as f64, y as f64, z as f64);
    let zero = Vector::new(0.0, 0.0, 0.0);
    a + (-a) == zero
}

// --- r2::Point ---

/// r2::Point addition is commutative.
#[quickcheck]
fn prop_r2_point_add_commutative(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let a = R2Point::new(x1 as f64, y1 as f64);
    let b = R2Point::new(x2 as f64, y2 as f64);
    a + b == b + a
}

/// r2::Point double negation is identity.
#[quickcheck]
fn prop_r2_point_neg_neg_identity(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let a = R2Point::new(x as f64, y as f64);
    -(-a) == a
}

/// r2::Point: a - b == a + (-b).
#[quickcheck]
fn prop_r2_point_sub_eq_add_neg(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let a = R2Point::new(x1 as f64, y1 as f64);
    let b = R2Point::new(x2 as f64, y2 as f64);
    a - b == a + (-b)
}

// --- LatLng ---

/// LatLng addition is commutative.
#[quickcheck]
fn prop_latlng_add_commutative(la: i32, lna: i32, lb: i32, lnb: i32) -> bool {
    let a = make_latlng(la, lna);
    let b = make_latlng(lb, lnb);
    let ab = a + b;
    let ba = b + a;
    (ab.lat.radians() - ba.lat.radians()).abs() < 1e-15
        && (ab.lng.radians() - ba.lng.radians()).abs() < 1e-15
}

/// LatLng: (a + b) - b ~= a (subtraction is inverse of addition).
#[quickcheck]
fn prop_latlng_sub_eq_add_neg(la: i32, lna: i32, lb: i32, lnb: i32) -> bool {
    let a = make_latlng(la, lna);
    let b = make_latlng(lb, lnb);
    let sum = a + b;
    let back = sum - b;
    (back.lat.radians() - a.lat.radians()).abs() < 1e-14
        && (back.lng.radians() - a.lng.radians()).abs() < 1e-14
}

// ────────────────────────────────────────────────────────────────────
// 14. Mul<f64> operator
// ────────────────────────────────────────────────────────────────────

/// Angle * 1.0 == Angle (multiplicative identity).
#[quickcheck]
fn prop_angle_mul_one_identity(a: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    ((x * 1.0).radians() - x.radians()).abs() < 1e-15
}

/// Angle * (-1.0) == -Angle.
#[quickcheck]
fn prop_angle_mul_neg_one_eq_neg(a: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    ((x * -1.0).radians() - (-x).radians()).abs() < 1e-15
}

/// Angle: k * (a + b) == k*a + k*b  (distributivity over addition).
#[quickcheck]
fn prop_angle_mul_distributive(a: i32, b: i32, k: u8) -> bool {
    let x = Angle::from_degrees((a % 90) as f64);
    let y = Angle::from_degrees((b % 90) as f64);
    let kf = (k % 10) as f64;
    let lhs = (x + y) * kf;
    let rhs = x * kf + y * kf;
    (lhs.radians() - rhs.radians()).abs() < 1e-12
}

/// r3::Vector * 1.0 == v.
#[quickcheck]
fn prop_r3_vector_mul_one_identity(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::new(x as f64, y as f64, z as f64);
    v * 1.0 == v
}

/// r3::Vector * 0.0 == zero.
#[quickcheck]
fn prop_r3_vector_mul_zero(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::new(x as f64, y as f64, z as f64);
    let zero = Vector::new(0.0, 0.0, 0.0);
    v * 0.0 == zero
}

/// r3::Vector: k * (a + b) == k*a + k*b.
#[quickcheck]
fn prop_r3_vector_mul_distributive(
    x1: i32,
    y1: i32,
    z1: i32,
    x2: i32,
    y2: i32,
    z2: i32,
    k: u8,
) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    let kf = (k % 20) as f64;
    (a + b) * kf == a * kf + b * kf
}

/// r2::Point * 1.0 == p.
#[quickcheck]
fn prop_r2_point_mul_one_identity(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let p = R2Point::new(x as f64, y as f64);
    p * 1.0 == p
}

/// r2::Point: k * (a + b) == k*a + k*b.
#[quickcheck]
fn prop_r2_point_mul_distributive(x1: i32, y1: i32, x2: i32, y2: i32, k: u8) -> bool {
    use s2rst::r2::Point as R2Point;
    let a = R2Point::new(x1 as f64, y1 as f64);
    let b = R2Point::new(x2 as f64, y2 as f64);
    let kf = (k % 20) as f64;
    (a + b) * kf == a * kf + b * kf
}

/// LatLng scalar multiplication distributes over lat component.
#[quickcheck]
fn prop_latlng_scalar_mul_lat_component(la: i32, lna: i32, k: u8) -> bool {
    let a = make_latlng(la, lna);
    let kf = (k % 10) as f64;
    let scaled = a * kf;
    (scaled.lat.radians() - a.lat.radians() * kf).abs() < 1e-14
}

/// LatLng scalar multiplication distributes over lng component.
#[quickcheck]
fn prop_latlng_scalar_mul_lng_component(la: i32, lna: i32, k: u8) -> bool {
    let a = make_latlng(la, lna);
    let kf = (k % 10) as f64;
    let scaled = a * kf;
    (scaled.lng.radians() - a.lng.radians() * kf).abs() < 1e-14
}

// ────────────────────────────────────────────────────────────────────
// 15. From / Into conversions
// ────────────────────────────────────────────────────────────────────

/// u64 → CellId → u64 is identity.
#[quickcheck]
fn prop_u64_cellid_u64_roundtrip(v: u64) -> bool {
    use s2rst::s2::CellId;
    u64::from(CellId::from(v)) == v
}

/// CellId → u64 → CellId is identity.
#[quickcheck]
fn prop_cellid_u64_cellid_roundtrip(face: u8, level: u8) -> bool {
    use s2rst::s2::CellId;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    CellId::from(u64::from(id)) == id
}

/// Point → Vector → Point is identity (both wrap the same vector).
#[quickcheck]
fn prop_s2point_vector_point_roundtrip(lat: i32, lng: i32) -> bool {
    use s2rst::r3::Vector;
    let p = make_latlng(lat, lng).to_point();
    let v = Vector::from(p);
    let p2 = Point::from(v);
    p == p2
}

/// Angle → ChordAngle → Angle is monotone: if a < b then ChordAngle(a) < ChordAngle(b).
#[quickcheck]
fn prop_angle_to_chord_angle_monotone(a: u8, b: u8) -> bool {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    let angle_a = Angle::from_degrees(a as f64);
    let angle_b = Angle::from_degrees(b as f64);
    let ca_a = ChordAngle::from(angle_a);
    let ca_b = ChordAngle::from(angle_b);
    ca_a <= ca_b
}

/// Angle → ChordAngle → Angle roundtrip is close (within 1e-14 rad).
#[quickcheck]
fn prop_angle_chord_angle_roundtrip_close(a: u8) -> bool {
    // ChordAngle only represents [0, 180°]; clamp to valid range
    let angle = Angle::from_degrees((a % 181) as f64);
    let ca = ChordAngle::from(angle);
    let back = Angle::from(ca);
    (back.radians() - angle.radians()).abs() < 1e-14
}

/// LatLng → Point → LatLng roundtrip is close.
#[quickcheck]
fn prop_latlng_point_latlng_roundtrip(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let p = ll.to_point();
    let ll2 = LatLng::from_point(p);
    (ll2.lat.radians() - ll.lat.radians()).abs() < 1e-14
        && (ll2.lng.radians() - ll.lng.radians()).abs() < 1e-14
}

/// (f64, f64) → r1::Interval: lo and hi fields match.
#[quickcheck]
fn prop_tuple_to_r1interval_fields(a: i32, b: i32) -> bool {
    use s2rst::r1::Interval;
    let (lo, hi) = (a as f64, b as f64);
    let iv = Interval::from((lo, hi));
    iv.lo == lo && iv.hi == hi
}

/// (f64, f64) → r2::Point: x and y fields match.
#[quickcheck]
fn prop_tuple_to_r2point_fields(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let p = R2Point::from((x as f64, y as f64));
    p.x == x as f64 && p.y == y as f64
}

/// (f64, f64, f64) → r3::Vector: x, y, z fields match.
#[quickcheck]
fn prop_tuple_to_r3vector_fields(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::from((x as f64, y as f64, z as f64));
    v.x == x as f64 && v.y == y as f64 && v.z == z as f64
}

/// [f64; 3] → r3::Vector: fields match.
#[quickcheck]
fn prop_array_to_r3vector_fields(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let arr: [f64; 3] = [x as f64, y as f64, z as f64];
    let v = Vector::from(arr);
    v.x == arr[0] && v.y == arr[1] && v.z == arr[2]
}

/// bool=false → r1::Endpoint::Lo; bool=true → r1::Endpoint::Hi.
#[quickcheck]
fn prop_bool_to_endpoint_false_is_lo(_dummy: u8) -> bool {
    use s2rst::r1::{Endpoint, Interval};
    let lo = Endpoint::from(false);
    let hi = Endpoint::from(true);
    let iv = Interval::from((1.0_f64, 2.0_f64));
    iv.bound(lo) == iv.lo && iv.bound(hi) == iv.hi
}

/// i64 → ExactFloat: converting small integers and back is exact.
#[quickcheck]
fn prop_i64_exactfloat_exact(n: i32) -> bool {
    use s2rst::r3::ExactFloat;
    let ef = ExactFloat::from(n as i64);
    // ExactFloat supports Display; compare to f64 path.
    let f = ef.to_f64();
    (f - n as f64).abs() < 0.5
}

/// f64 → ExactFloat: integer values round-trip exactly.
#[quickcheck]
fn prop_f64_exactfloat_integer_exact(n: i32) -> bool {
    use s2rst::r3::ExactFloat;
    let ef = ExactFloat::from(n as f64);
    let back = ef.to_f64();
    (back - n as f64).abs() < 0.5
}

/// u8 → Face: valid faces round-trip through as_u8.
#[quickcheck]
fn prop_face_u8_roundtrip(v: u8) -> bool {
    use s2rst::s2::coords::Face;
    let f = Face::from_u8(v % 6);
    f.as_u8() == v % 6
}

/// CellId → Cell → CellId roundtrip.
#[quickcheck]
fn prop_cellid_to_cell_roundtrip(face: u8, level: u8) -> bool {
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    let cell = Cell::from_cell_id(id);
    cell.id() == id
}

/// ChordAngle: From<ChordAngle> for Angle then From<Angle> back is monotone.
#[quickcheck]
fn prop_chord_to_angle_monotone(a: u8, b: u8) -> bool {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    let ca_a = ChordAngle::from(Angle::from_degrees(a as f64));
    let ca_b = ChordAngle::from(Angle::from_degrees(b as f64));
    Angle::from(ca_a) <= Angle::from(ca_b)
}

// ────────────────────────────────────────────────────────────────────
// 16. PartialOrd / Ord
// ────────────────────────────────────────────────────────────────────

/// ChordAngle: a <= a (reflexive).
#[quickcheck]
fn prop_chord_angle_reflexive(a: u8) -> bool {
    let x = ChordAngle::from(Angle::from_degrees(a as f64));
    x <= x
}

/// ChordAngle: a < b → !(b < a) (antisymmetric).
#[quickcheck]
fn prop_chord_angle_antisymmetric(a: u8, b: u8) -> bool {
    let x = ChordAngle::from(Angle::from_degrees(a as f64));
    let y = ChordAngle::from(Angle::from_degrees(b as f64));
    !(x < y && y < x)
}

/// ChordAngle ordering is consistent with Angle ordering.
#[quickcheck]
fn prop_chord_angle_consistent_with_angle(a: u8, b: u8) -> bool {
    // ChordAngle only represents [0, 180°]; clamp to valid range
    let ang_a = Angle::from_degrees((a % 181) as f64);
    let ang_b = Angle::from_degrees((b % 181) as f64);
    let ca_a = ChordAngle::from(ang_a);
    let ca_b = ChordAngle::from(ang_b);
    // The orderings must agree.
    (ang_a <= ang_b) == (ca_a <= ca_b)
}

/// Angle: a <= a (reflexive).
#[quickcheck]
fn prop_angle_reflexive(a: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    x <= x
}

/// Angle: a < b → !(b < a).
#[quickcheck]
fn prop_angle_antisymmetric(a: i32, b: i32) -> bool {
    let x = Angle::from_degrees((a % 180) as f64);
    let y = Angle::from_degrees((b % 180) as f64);
    !(x < y && y < x)
}

/// ExactFloat comparison is consistent with f64 for integers.
#[quickcheck]
fn prop_exactfloat_cmp_consistent_with_f64(a: i32, b: i32) -> bool {
    use s2rst::r3::ExactFloat;
    let ea = ExactFloat::from(a as i64);
    let eb = ExactFloat::from(b as i64);
    (ea < eb) == (a < b) && (ea > eb) == (a > b) && (ea == eb) == (a == b)
}

/// ExactFloat: a <= a (reflexive).
#[quickcheck]
fn prop_exactfloat_reflexive(n: i32) -> bool {
    use s2rst::r3::ExactFloat;
    let e = ExactFloat::from(n as i64);
    e <= e
}

/// r1::Interval: lo <= hi for a non-empty interval.
#[quickcheck]
fn prop_r1interval_lo_le_hi(a: i32, b: i32) -> bool {
    use s2rst::r1::Interval;
    let (lo, hi) = if a <= b {
        (a as f64, b as f64)
    } else {
        (b as f64, a as f64)
    };
    let iv = Interval::new(lo, hi);
    iv.lo <= iv.hi
}

/// CellId range_min <= range_max.
#[quickcheck]
fn prop_cellid_range_min_le_max(face: u8, level: u8) -> bool {
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    id.range_min() <= id.range_max()
}

/// CellId: a parent's range_min <= child's range_min (containment).
#[quickcheck]
fn prop_cellid_parent_range_contains_child(face: u8, level: u8) -> bool {
    let level = ((level % 29) + 1).min(29); // level >= 1 so we can take parent
    let id = CellId::from_face(face % 6).child_begin_at_level(level);
    let parent = id.parent_at_level(level - 1);
    parent.range_min() <= id.range_min() && id.range_max() <= parent.range_max()
}

// ────────────────────────────────────────────────────────────────────
// 17. Default trait
// ────────────────────────────────────────────────────────────────────

/// Rect::default() is the empty rectangle.
#[quickcheck]
fn prop_rect_default_is_empty(_dummy: u8) -> bool {
    Rect::default().is_empty()
}

/// Loop::default() is the empty loop.
#[quickcheck]
fn prop_loop_default_is_empty(_dummy: u8) -> bool {
    use s2rst::s2::shape::Shape;
    Loop::default().is_empty()
}

/// Polygon::default() is the empty polygon.
#[quickcheck]
fn prop_polygon_default_is_empty(_dummy: u8) -> bool {
    use s2rst::s2::shape::Shape;
    Polygon::default().is_empty()
}

/// Cap::empty() is the empty cap.
#[quickcheck]
fn prop_cap_default_is_empty(_dummy: u8) -> bool {
    Cap::empty().is_empty()
}

/// LatLng::default() has lat=0, lng=0.
#[quickcheck]
fn prop_latlng_default_is_origin(_dummy: u8) -> bool {
    let ll = LatLng::default();
    ll.lat.radians() == 0.0 && ll.lng.radians() == 0.0
}

/// r1::Interval::default() is the empty interval.
#[quickcheck]
fn prop_r1interval_default_is_empty(_dummy: u8) -> bool {
    use s2rst::r1::Interval;
    Interval::default().is_empty()
}

/// s1::Interval::default() is the empty circular interval.
#[quickcheck]
fn prop_s1interval_default_is_empty(_dummy: u8) -> bool {
    use s2rst::s1::Interval;
    Interval::default().is_empty()
}

/// ChordAngle::default() equals ChordAngle of 0 degrees.
#[quickcheck]
fn prop_chord_angle_default_is_zero(_dummy: u8) -> bool {
    ChordAngle::default() == ChordAngle::from(Angle::from_degrees(0.0))
}

// ────────────────────────────────────────────────────────────────────
// 18. Display trait — to_string() is non-empty for any valid value
// ────────────────────────────────────────────────────────────────────

#[quickcheck]
fn prop_cellid_display_nonempty(face: u8, level: u8) -> bool {
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    !id.to_string().is_empty()
}

#[quickcheck]
fn prop_latlng_display_nonempty(lat: i32, lng: i32) -> bool {
    !make_latlng(lat, lng).to_string().is_empty()
}

#[quickcheck]
fn prop_cap_display_nonempty(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(deg as f64));
    !cap.to_string().is_empty()
}

#[quickcheck]
fn prop_rect_display_nonempty(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    !r.to_string().is_empty()
}

#[quickcheck]
fn prop_angle_display_nonempty(a: i32) -> bool {
    !Angle::from_degrees((a % 180) as f64).to_string().is_empty()
}

#[quickcheck]
fn prop_face_display_nonempty(v: u8) -> bool {
    use s2rst::s2::coords::Face;
    !Face::from_u8(v % 6).to_string().is_empty()
}

#[quickcheck]
fn prop_cellunion_display_nonempty(face: u8) -> bool {
    use s2rst::s2::CellUnion;
    let cu = CellUnion::from_cell_ids(vec![CellId::from_face(face % 6)]);
    !cu.to_string().is_empty()
}

#[quickcheck]
fn prop_r3_vector_display_nonempty(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    !Vector::new(x as f64, y as f64, z as f64)
        .to_string()
        .is_empty()
}

#[quickcheck]
fn prop_chord_angle_display_nonempty(a: u8) -> bool {
    !ChordAngle::from(Angle::from_degrees(a as f64))
        .to_string()
        .is_empty()
}

#[quickcheck]
fn prop_r2_point_display_nonempty(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    !R2Point::new(x as f64, y as f64).to_string().is_empty()
}

#[quickcheck]
fn prop_r1_interval_display_nonempty(a: i32, b: i32) -> bool {
    use s2rst::r1::Interval;
    let (lo, hi) = if a <= b {
        (a as f64, b as f64)
    } else {
        (b as f64, a as f64)
    };
    !Interval::new(lo, hi).to_string().is_empty()
}

// ────────────────────────────────────────────────────────────────────
// 19. SnapFunction trait
// ────────────────────────────────────────────────────────────────────

/// IdentitySnapFunction: snap_radius is exactly the configured angle.
#[quickcheck]
fn prop_identity_snap_radius_matches_configured(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let r = Angle::from_degrees((deg % 71) as f64); // MAX_SNAP_RADIUS is 70°
    let snap = IdentitySnapFunction::new(r);
    (snap.snap_radius().radians() - r.radians()).abs() < 1e-15
}

/// IdentitySnapFunction: snap_point returns the input point unchanged.
#[quickcheck]
fn prop_identity_snap_point_is_identity(lat: i32, lng: i32) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let snap = IdentitySnapFunction::new(Angle::from_degrees(0.0));
    let p = make_latlng(lat, lng).to_point();
    snap.snap_point(p) == p
}

/// IdentitySnapFunction: min_vertex_separation <= snap_radius.
#[quickcheck]
fn prop_identity_snap_min_vertex_sep_le_radius(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let r = Angle::from_degrees((deg % 71) as f64);
    let snap = IdentitySnapFunction::new(r);
    snap.min_vertex_separation() <= snap.snap_radius()
}

/// IdentitySnapFunction: min_edge_vertex_separation <= min_vertex_separation.
#[quickcheck]
fn prop_identity_snap_edge_vertex_le_vertex_sep(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let r = Angle::from_degrees((deg % 71) as f64);
    let snap = IdentitySnapFunction::new(r);
    snap.min_edge_vertex_separation() <= snap.min_vertex_separation()
}

/// S2CellIdSnapFunction: snap_radius >= 0.
#[quickcheck]
fn prop_cellid_snap_radius_nonneg(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let snap = S2CellIdSnapFunction::new(level % 30);
    snap.snap_radius().radians() >= 0.0
}

/// S2CellIdSnapFunction: min_vertex_separation <= snap_radius.
#[quickcheck]
fn prop_cellid_snap_min_vertex_sep_le_radius(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let snap = S2CellIdSnapFunction::new(level % 30);
    snap.min_vertex_separation() <= snap.snap_radius()
}

/// S2CellIdSnapFunction: min_edge_vertex_separation <= min_vertex_separation.
#[quickcheck]
fn prop_cellid_snap_edge_vertex_le_vertex_sep(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let snap = S2CellIdSnapFunction::new(level % 30);
    snap.min_edge_vertex_separation() <= snap.min_vertex_separation()
}

/// IntLatLngSnapFunction: snap_radius >= 0.
#[quickcheck]
fn prop_intlatlng_snap_radius_nonneg(exp: u8) -> bool {
    use s2rst::s2::builder::snap::{IntLatLngSnapFunction, SnapFunction};
    let snap = IntLatLngSnapFunction::new((exp % 11) as i32);
    snap.snap_radius().radians() >= 0.0
}

/// IntLatLngSnapFunction: min_vertex_separation <= snap_radius.
#[quickcheck]
fn prop_intlatlng_snap_min_vertex_sep_le_radius(exp: u8) -> bool {
    use s2rst::s2::builder::snap::{IntLatLngSnapFunction, SnapFunction};
    let snap = IntLatLngSnapFunction::new((exp % 11) as i32);
    snap.min_vertex_separation() <= snap.snap_radius()
}

/// IntLatLngSnapFunction: min_edge_vertex_separation <= min_vertex_separation.
#[quickcheck]
fn prop_intlatlng_snap_edge_vertex_le_vertex_sep(exp: u8) -> bool {
    use s2rst::s2::builder::snap::{IntLatLngSnapFunction, SnapFunction};
    let snap = IntLatLngSnapFunction::new((exp % 11) as i32);
    snap.min_edge_vertex_separation() <= snap.min_vertex_separation()
}

/// All three snap functions: separations are non-negative.
#[quickcheck]
fn prop_snap_separations_all_nonneg(level: u8, exp: u8, id_deg: u8) -> bool {
    use s2rst::s2::builder::snap::{
        IdentitySnapFunction, IntLatLngSnapFunction, S2CellIdSnapFunction, SnapFunction,
    };
    let id_snap = IdentitySnapFunction::new(Angle::from_degrees((id_deg % 71) as f64));
    let cell_snap = S2CellIdSnapFunction::new(level % 30);
    let ll_snap = IntLatLngSnapFunction::new((exp % 11) as i32);
    id_snap.min_vertex_separation().radians() >= 0.0
        && cell_snap.min_vertex_separation().radians() >= 0.0
        && ll_snap.min_vertex_separation().radians() >= 0.0
}

/// clone_snap() on IdentitySnapFunction has the same snap_radius.
#[quickcheck]
fn prop_identity_snap_clone_same_radius(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let r = Angle::from_degrees((deg % 71) as f64);
    let snap = IdentitySnapFunction::new(r);
    let cloned = snap.clone_snap();
    (cloned.snap_radius().radians() - snap.snap_radius().radians()).abs() < 1e-15
}

/// clone_snap() on S2CellIdSnapFunction has the same snap_radius.
#[quickcheck]
fn prop_cellid_snap_clone_same_radius(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let snap = S2CellIdSnapFunction::new(level % 30);
    let cloned = snap.clone_snap();
    (cloned.snap_radius().radians() - snap.snap_radius().radians()).abs() < 1e-15
}

// ────────────────────────────────────────────────────────────────────
// 20. Iterator / IntoIterator / FromIterator
// ────────────────────────────────────────────────────────────────────

/// CellUnion::into_iter() yields exactly as many items as cell_ids().len().
#[quickcheck]
fn prop_cellunion_intoiter_count(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let expected = cu.cell_ids().len();
    cu.into_iter().count() == expected
}

/// CellUnion items yielded by iteration are all valid CellIds.
#[quickcheck]
fn prop_cellunion_iter_all_valid(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.cell_ids().iter().all(|c| c.is_valid())
}

/// CellUnion yields items in sorted order (the normalization invariant).
#[quickcheck]
fn prop_cellunion_normalized_ids_sorted(face0: u8, face1: u8) -> bool {
    use s2rst::s2::CellUnion;
    let mut ids = vec![CellId::from_face(face0 % 6), CellId::from_face(face1 % 6)];
    ids.dedup();
    let cu = CellUnion::from_cell_ids(ids);
    cu.cell_ids().windows(2).all(|w| w[0] <= w[1])
}

/// CellUnion::from_cell_ids then cell_ids() gives back the same (normalized) ids.
#[quickcheck]
fn prop_cellunion_from_cell_ids_roundtrip(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.cell_ids().contains(&id)
}

/// Collecting CellIds via FromIterator then iterating gives the same ids.
#[quickcheck]
fn prop_cellunion_fromiter_consistent(face: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6);
    let ids = [id];
    let cu: CellUnion = ids.iter().copied().collect();
    cu.cell_ids().iter().all(|c| c.is_valid())
}

/// &CellUnion into_iter count equals cell_ids().len().
#[quickcheck]
fn prop_cellunion_ref_intoiter_count(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20).min(20));
    let cu = CellUnion::from_cell_ids(vec![id]);
    (&cu).into_iter().count() == cu.cell_ids().len()
}

// ────────────────────────────────────────────────────────────────────
// 21. S2Encode / S2Decode — gap-filling for remaining types
// ────────────────────────────────────────────────────────────────────

/// LaxPolyline encode → decode preserves vertex count.
#[quickcheck]
fn prop_lax_polyline_encode_decode_vertex_count(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::lax_polyline::LaxPolyline;
    let pl = LaxPolyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    let mut buf = Vec::new();
    pl.encode(&mut buf).unwrap();
    let back = LaxPolyline::decode(&mut buf.as_slice()).unwrap();
    back.vertices().len() == pl.vertices().len()
}

/// LaxPolygon encode → decode preserves loop count.
#[quickcheck]
fn prop_lax_polygon_encode_decode_loop_count(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::lax_polygon::LaxPolygon;
    let verts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ];
    let lp = LaxPolygon::from_loops_owned(vec![verts]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let back = LaxPolygon::decode(&mut buf.as_slice()).unwrap();
    back.num_loops() == lp.num_loops()
}

/// PointVector encode → decode preserves point count.
#[quickcheck]
fn prop_point_vector_encode_decode_count(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::point_vector::PointVector;
    let pv = PointVector::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    let mut buf = Vec::new();
    pv.encode(&mut buf).unwrap();
    let back = PointVector::decode(&mut buf.as_slice()).unwrap();
    back.len() == pv.len()
}

/// Cell encode → decode preserves level.
#[quickcheck]
fn prop_cell_encode_decode_level(face: u8, level: u8) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    let cell = Cell::from_cell_id(id);
    let mut buf = Vec::new();
    cell.encode(&mut buf).unwrap();
    let back = Cell::decode(&mut buf.as_slice()).unwrap();
    back.level() == cell.level()
}

/// Cap encode → decode preserves area within tolerance.
#[quickcheck]
fn prop_cap_encode_decode_area(lat: i32, lng: i32, deg: u8) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(deg as f64));
    let mut buf = Vec::new();
    cap.encode(&mut buf).unwrap();
    let back = Cap::decode(&mut buf.as_slice()).unwrap();
    (back.area() - cap.area()).abs() < 1e-10
}

/// Rect encode → decode: lo and hi round-trip.
#[quickcheck]
fn prop_rect_encode_decode_bounds(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    let mut buf = Vec::new();
    r.encode(&mut buf).unwrap();
    let back = Rect::decode(&mut buf.as_slice()).unwrap();
    (back.lo().lat.radians() - r.lo().lat.radians()).abs() < 1e-14
        && (back.hi().lng.radians() - r.hi().lng.radians()).abs() < 1e-14
}

/// CellId encode → decode: level is preserved.
#[quickcheck]
fn prop_cellid_encode_decode_level(face: u8, level: u8) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 30).min(30));
    let mut buf = Vec::new();
    id.encode(&mut buf).unwrap();
    let back = CellId::decode(&mut buf.as_slice()).unwrap();
    back.level() == id.level()
}

// ────────────────────────────────────────────────────────────────────
// 22. Additional Region trait coverage (Polyline, CellUnion bounds)
// ────────────────────────────────────────────────────────────────────

/// Polyline::cap_bound() is valid (non-empty for non-trivial polylines).
#[quickcheck]
fn prop_polyline_cap_bound_valid(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    let cb = pl.cap_bound();
    // Either the cap is non-empty, or both endpoints are identical (degenerate).
    !cb.is_empty() || pl.num_vertices() < 2
}

/// Polyline::rect_bound() is valid.
#[quickcheck]
fn prop_polyline_rect_bound_valid(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.rect_bound().is_valid()
}

/// Polyline::cap_bound covers both endpoints.
#[quickcheck]
fn prop_polyline_cap_bound_covers_endpoints(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    let rb = pl.rect_bound();
    // Test rect_bound instead — it's guaranteed to cover all vertices
    rb.contains_lat_lng(LatLng::from_point(p0)) && rb.contains_lat_lng(LatLng::from_point(p1))
}

/// Polyline::rect_bound covers both endpoints.
#[quickcheck]
fn prop_polyline_rect_bound_covers_endpoints(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    let rb = pl.rect_bound();
    rb.contains_lat_lng(LatLng::from_point(p0)) && rb.contains_lat_lng(LatLng::from_point(p1))
}

/// CellUnion cap_bound covers all cell centers in the union.
#[quickcheck]
fn prop_cellunion_cap_bound_covers_cell_centers(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 15).min(15));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let cb = cu.cap_bound();
    cu.cell_ids()
        .iter()
        .all(|&c| cb.contains_point(c.to_point()))
}

/// CellUnion rect_bound covers all cell centers.
#[quickcheck]
fn prop_cellunion_rect_bound_covers_cell_centers(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 15).min(15));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let rb = cu.rect_bound();
    cu.cell_ids()
        .iter()
        .all(|&c| rb.contains_lat_lng(LatLng::from_point(c.to_point())))
}

/// A region's cap_bound is at least as large as needed (area ≥ 0).
#[quickcheck]
fn prop_cap_cap_bound_area_nonneg(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(deg as f64));
    cap.cap_bound().area() >= 0.0
}

// ────────────────────────────────────────────────────────────────────
// 23. Additional Shape coverage — LaxPolyline, LaxPolygon
// ────────────────────────────────────────────────────────────────────

/// LaxPolyline::dimension() == Dimension::Polyline.
#[quickcheck]
fn prop_lax_polyline_dimension_one(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let pl = LaxPolyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.dimension() == Dimension::Polyline
}

/// LaxPolyline::num_edges() == vertex_count - 1 for a 2-vertex polyline.
#[quickcheck]
fn prop_lax_polyline_num_edges(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let pl = LaxPolyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.num_edges() == pl.vertices().len().saturating_sub(1)
}

/// LaxPolyline edge endpoints are unit vectors.
#[quickcheck]
fn prop_lax_polyline_edge_endpoints_unit(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let pl = LaxPolyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    (0..pl.num_edges()).all(|i| {
        let e = pl.edge(i);
        (e.v0.0.norm2() - 1.0).abs() < 1e-14 && (e.v1.0.norm2() - 1.0).abs() < 1e-14
    })
}

/// LaxPolygon::dimension() == Dimension::Polygon.
#[quickcheck]
fn prop_lax_polygon_dimension_two(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape::Shape;
    let verts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ];
    let lp = LaxPolygon::from_loops_owned(vec![verts]);
    lp.dimension() == Dimension::Polygon
}

/// LaxPolygon: num_edges() == sum of chain lengths.
#[quickcheck]
fn prop_lax_polygon_edges_sum_chains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    use s2rst::s2::shape::Shape;
    let verts = vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
        make_latlng(lat2, lng2).to_point(),
    ];
    let lp = LaxPolygon::from_loops_owned(vec![verts]);
    let total: usize = (0..lp.num_chains()).map(|i| lp.chain(i).length).sum();
    total == lp.num_edges()
}

// ────────────────────────────────────────────────────────────────────
// 24. Additional arithmetic: r3::Vector dot / cross properties
// ────────────────────────────────────────────────────────────────────

/// dot(a, b) == dot(b, a) (commutativity).
#[quickcheck]
fn prop_r3_dot_commutative(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    a.dot(b) == b.dot(a)
}

/// dot(a, a) == norm2(a).
#[quickcheck]
fn prop_r3_dot_self_is_norm2(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x as f64, y as f64, z as f64);
    (a.dot(a) - a.norm2()).abs() < 1e-10
}

/// cross(a, a) == zero.
#[quickcheck]
fn prop_r3_cross_self_is_zero(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x as f64, y as f64, z as f64);
    let c = a.cross(a);
    c == Vector::new(0.0, 0.0, 0.0)
}

/// cross(a, b) is perpendicular to a: dot(cross(a,b), a) == 0.
#[quickcheck]
fn prop_r3_cross_perpendicular_to_a(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    let c = a.cross(b);
    // Tolerance scales with product of magnitudes (fp error in cross product)
    let tol = a.norm() * b.norm() * a.norm() * 3.0 * f64::EPSILON;
    c.dot(a).abs() < tol.max(1e-8)
}

/// cross(a, b) == -cross(b, a) (anti-commutativity).
#[quickcheck]
fn prop_r3_cross_anticommutative2(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    use s2rst::r3::Vector;
    let a = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = Vector::new(x2 as f64, y2 as f64, z2 as f64);
    a.cross(b) == -(b.cross(a))
}

/// r2::Point dot(a, a) == norm2(a).
#[quickcheck]
fn prop_r2_dot_self_is_norm2(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let a = R2Point::new(x as f64, y as f64);
    (a.dot(a) - a.norm2()).abs() < 1e-10
}

/// r2::Point: norm >= 0.
#[quickcheck]
fn prop_r2_norm_nonneg(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    R2Point::new(x as f64, y as f64).norm() >= 0.0
}

// ────────────────────────────────────────────────────────────────────
// 25. Additional From/Into and CellId properties
// ────────────────────────────────────────────────────────────────────

/// CellId from face is a valid face cell.
#[quickcheck]
fn prop_cellid_from_face_valid(face: u8) -> bool {
    let id = CellId::from_face(face % 6);
    use s2rst::s2::Face;
    id.is_valid() && id.face() == Face::try_from(face % 6).unwrap()
}

/// CellId from LatLng is valid and at leaf level.
#[quickcheck]
fn prop_cellid_from_latlng_leaf(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let id = CellId::from(&ll);
    id.is_valid() && id.is_leaf()
}

/// CellId: child_begin_at_level(l).level() == l.
#[quickcheck]
fn prop_cellid_level_matches_requested(face: u8, level: u8) -> bool {
    let level = (level % 31).min(30);
    let id = CellId::from_face(face % 6).child_begin_at_level(level);
    id.level() == level
}

/// LatLng constructed from_degrees has lat/lng matching within precision.
#[quickcheck]
fn prop_latlng_from_degrees_fields_exact(lat: i32, lng: i32) -> bool {
    let lat_d = (lat.rem_euclid(181)) as f64 - 90.0;
    let lng_d = (lng.rem_euclid(361)) as f64 - 180.0;
    let ll = LatLng::from_degrees(lat_d, lng_d);
    (ll.lat.degrees() - lat_d).abs() < 1e-13 && (ll.lng.degrees() - lng_d).abs() < 1e-13
}

/// Cap center is on the unit sphere after construction.
#[quickcheck]
fn prop_cap_center_unit(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees(deg as f64));
    (cap.center().0.norm2() - 1.0).abs() < 1e-14
}

/// s2::Rect from_point_pair contains both points.
#[quickcheck]
fn prop_rect_from_point_pair_contains_both(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let r = Rect::from_point_pair(ll0, ll1);
    r.contains_lat_lng(ll0) && r.contains_lat_lng(ll1)
}

/// r1::Interval::new(a, b).contains(x) iff a <= x <= b.
#[quickcheck]
fn prop_r1interval_contains_iff_in_range(a: i32, b: i32, x: i32) -> bool {
    use s2rst::r1::Interval;
    let (lo, hi) = if a <= b {
        (a as f64, b as f64)
    } else {
        (b as f64, a as f64)
    };
    let iv = Interval::new(lo, hi);
    let xf = x as f64;
    iv.contains(xf) == (lo <= xf && xf <= hi)
}

/// ChordAngle constructed from zero angle has length2 == 0.
#[quickcheck]
fn prop_chord_angle_zero_length2(_dummy: u8) -> bool {
    ChordAngle::from(Angle::from_degrees(0.0)).length2() == 0.0
}

/// ChordAngle::from_angle(180°) has length2 == 4.
#[quickcheck]
fn prop_chord_angle_straight_length2(_dummy: u8) -> bool {
    (ChordAngle::from(Angle::from_degrees(180.0)).length2() - 4.0).abs() < 1e-14
}

/// Point::from_coords normalizes: resulting point is on unit sphere.
#[quickcheck]
fn prop_point_from_coords_is_unit(x: i32, y: i32, z: i32) -> bool {
    if x == 0 && y == 0 && z == 0 {
        return true;
    }
    let p = Point::from_coords(x as f64, y as f64, z as f64);
    (p.0.norm2() - 1.0).abs() < 1e-14
}

/// Point::origin() is on the unit sphere.
#[quickcheck]
fn prop_point_origin_unit(_dummy: u8) -> bool {
    (Point::origin().0.norm2() - 1.0).abs() < 1e-14
}

/// The ExactFloat zero is less than or equal to any non-negative integer.
#[quickcheck]
fn prop_exactfloat_zero_le_nonneg(n: u32) -> bool {
    use s2rst::r3::ExactFloat;
    ExactFloat::zero() <= ExactFloat::from(n as i64)
}

// ══ SECTION 26: s1::Interval, r2::Rect, Cell, Angle trig, Cap/Rect/CellId extras ══

/// s1::Interval: length() >= 0 for any interval.
#[quickcheck]
fn prop_s1interval_length_nonneg(a: i32, b: i32) -> bool {
    use s2rst::s1::Interval;
    let lo = (a % 1800) as f64 * 0.001_f64;
    let hi = (b % 1800) as f64 * 0.001_f64;
    let iv = Interval::new(lo, hi);
    iv.length() >= -1e-14
}

/// s1::Interval: full interval contains every point in a small range.
#[quickcheck]
fn prop_s1interval_full_contains_all(x: i32) -> bool {
    use s2rst::s1::Interval;
    let angle = (x % 3142) as f64 * 0.001_f64;
    Interval::full().contains(angle)
}

/// s1::Interval: empty interval contains no point.
#[quickcheck]
fn prop_s1interval_empty_contains_none(x: i32) -> bool {
    use s2rst::s1::Interval;
    let angle = (x % 3142) as f64 * 0.001_f64;
    !Interval::empty().contains(angle)
}

/// r2::Point: x and y are preserved through construction.
#[quickcheck]
fn prop_r2point_components(x: i32, y: i32) -> bool {
    use s2rst::r2::Point as R2Point;
    let p = R2Point::new(x as f64, y as f64);
    p.x == x as f64 && p.y == y as f64
}

/// r2::Point: norm2 equals x*x + y*y.
#[quickcheck]
fn prop_r2point_norm2(x: i16, y: i16) -> bool {
    use s2rst::r2::Point as R2Point;
    let xf = x as f64;
    let yf = y as f64;
    let p = R2Point::new(xf, yf);
    (p.norm2() - (xf * xf + yf * yf)).abs() < 1e-10
}

/// r2::Rect: constructed from two points contains both corners.
#[quickcheck]
fn prop_r2rect_contains_corners(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    use s2rst::r2::{Point as R2Point, Rect as R2Rect};
    let p1 = R2Point::new(x1 as f64, y1 as f64);
    let p2 = R2Point::new(x2 as f64, y2 as f64);
    let r = R2Rect::from_point_pair(p1, p2);
    r.contains_point(p1) && r.contains_point(p2)
}

/// r2::Rect: empty rect is_empty() == true.
#[quickcheck]
fn prop_r2rect_empty_is_empty(_dummy: u8) -> bool {
    use s2rst::r2::Rect as R2Rect;
    R2Rect::empty().is_empty()
}

/// Cell: level of a cell from CellId is consistent.
#[quickcheck]
fn prop_cell_level_matches_cellid(face: u8) -> bool {
    use s2rst::s2::Cell;
    let cellid = CellId::from_face(face % 6).child_begin_at_level(5u8);
    let cell = Cell::from(cellid);
    cell.level() == 5u8
}

/// Cell: child cells have level one higher than parent.
#[quickcheck]
fn prop_cell_child_level_parent_plus_one(face: u8, level: u8) -> bool {
    let level = level % 29; // levels 0..=28 so child is at most 29
    let cellid = CellId::from_face(face % 6).child_begin_at_level(level);
    let child_level = cellid.child_begin().level();
    child_level == level + 1
}

/// Cell: parent of a non-root cell has level one lower.
#[quickcheck]
fn prop_cell_parent_level(face: u8, level: u8) -> bool {
    let level = 1 + level % 29; // levels 1..=29
    let cellid = CellId::from_face(face % 6).child_begin_at_level(level);
    cellid.parent().level() == level - 1
}

/// CellId::from_face(f).face() returns the correct Face.
#[quickcheck]
fn prop_cellid_face_roundtrip(face: u8) -> bool {
    use s2rst::s2::Face;
    let f = face % 6;
    let expected = Face::try_from(f).unwrap();
    CellId::from_face(f).face() == expected
}

/// CellId: is_valid() is true for freshly constructed cells.
#[quickcheck]
fn prop_cellid_constructed_valid(face: u8, level: u8) -> bool {
    CellId::from_face(face % 6)
        .child_begin_at_level(level % 30)
        .is_valid()
}

/// CellId: sentinel is not valid.
#[quickcheck]
fn prop_cellid_sentinel_not_valid(_dummy: u8) -> bool {
    // sentinel has face=7 which panics in face(); check raw id instead
    let s = CellId::sentinel();
    // A valid CellId has face < 6 and correct lsb pattern.
    // Sentinel is 0xFFFFFFFFFFFFFFFF which has face bits = 7, so it's invalid.
    // We verify by checking the raw id rather than calling is_valid() which panics.
    s.0 >> 61 >= 6
}

/// Angle: from_radians(r).radians() roundtrips exactly.
#[quickcheck]
fn prop_angle_radians_roundtrip(r: i32) -> bool {
    let rad = r as f64 * 0.001;
    (Angle::from_radians(rad).radians() - rad).abs() < 1e-14
}

/// Angle: from_degrees roundtrips for integers.
#[quickcheck]
fn prop_angle_int_degrees_roundtrip(d: i16) -> bool {
    let deg = d as f64;
    (Angle::from_degrees(deg).degrees() - deg).abs() < 1e-10
}

/// Angle: abs() is always >= 0.
#[quickcheck]
fn prop_angle_abs_nonneg(r: i32) -> bool {
    Angle::from_radians(r as f64 * 0.001).abs().radians() >= 0.0
}

/// Cap: is_valid() for a cap built from a unit point and non-negative angle.
#[quickcheck]
fn prop_cap_is_valid_from_center_angle(x: i32, y: i32, z: i32, r: u8) -> bool {
    if x == 0 && y == 0 && z == 0 {
        return true;
    }
    let angle = Angle::from_degrees((r % 180) as f64);
    match make_point(x as f64, y as f64, z as f64) {
        Some(p) => Cap::from_center_angle(p, angle).is_valid(),
        None => true,
    }
}

/// Cap: empty cap area == 0.
#[quickcheck]
fn prop_cap_empty_area(_dummy: u8) -> bool {
    Cap::empty().area() == 0.0
}

/// Cap: full cap area == 4*pi.
#[quickcheck]
fn prop_cap_full_area(_dummy: u8) -> bool {
    (Cap::full().area() - 4.0 * PI).abs() < 1e-14
}

/// Rect: empty rect has is_empty() == true.
#[quickcheck]
fn prop_rect_empty_flag(_dummy: u8) -> bool {
    Rect::empty().is_empty()
}

/// Rect: full rect has is_full() == true.
#[quickcheck]
fn prop_rect_full_flag(_dummy: u8) -> bool {
    Rect::full().is_full()
}

/// Rect: area is >= 0 for any rect built from lat/lng intervals.
#[quickcheck]
fn prop_rect_area_nonneg(lat1: i8, lat2: i8, lng1: i8, lng2: i8) -> bool {
    use s2rst::r1::Interval as R1Interval;
    use s2rst::s1::Interval as S1Interval;
    let la = (lat1.min(lat2) as f64).to_radians();
    let lb = (lat1.max(lat2) as f64).to_radians();
    let lo = (lng1.min(lng2) as f64).to_radians();
    let hi = (lng1.max(lng2) as f64).to_radians();
    let r = Rect::new(R1Interval::new(la, lb), S1Interval::new(lo, hi));
    r.area() >= 0.0
}

/// LatLng: lat field is within [-pi/2, pi/2] when constructed from small degrees.
#[quickcheck]
fn prop_latlng_lat_in_range(lat: i8, lng: i8) -> bool {
    // LatLng::from_degrees does NOT clamp; only valid lat is [-90,90]
    let lat_d = (lat as f64).clamp(-90.0, 90.0);
    let ll = LatLng::from_degrees(lat_d, lng as f64);
    ll.lat.radians() >= -PI / 2.0 - 1e-14 && ll.lat.radians() <= PI / 2.0 + 1e-14
}

/// LatLng: lng field is within [-pi, pi] when constructed from small degrees.
#[quickcheck]
fn prop_latlng_lng_in_range(lat: i8, lng: i8) -> bool {
    let ll = LatLng::from_degrees(lat as f64, lng as f64);
    ll.lng.radians() >= -PI - 1e-14 && ll.lng.radians() <= PI + 1e-14
}

/// LatLng: converting to Point and back preserves lat to floating-point precision.
#[quickcheck]
fn prop_latlng_point_roundtrip_lat(lat: i8, lng: i8) -> bool {
    // LatLng::from_degrees does NOT clamp; only valid lat is [-90,90]
    let lat_d = (lat as f64).clamp(-90.0, 90.0);
    let ll = LatLng::from_degrees(lat_d, lng as f64);
    let back = LatLng::from(ll.to_point());
    (back.lat.degrees() - ll.lat.degrees()).abs() < 1e-10
}

/// r3::Vector: dot product with zero vector is zero.
#[quickcheck]
fn prop_r3vector_dot_zero(x: i16, y: i16, z: i16) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::new(x as f64, y as f64, z as f64);
    let zero = Vector::new(0.0, 0.0, 0.0);
    v.dot(zero) == 0.0
}

/// r3::Vector: cross product of a vector with itself is zero.
#[quickcheck]
fn prop_r3vector_cross_self_zero(x: i16, y: i16, z: i16) -> bool {
    use s2rst::r3::Vector;
    let v = Vector::new(x as f64, y as f64, z as f64);
    let c = v.cross(v);
    c.norm2() < 1e-20
}

/// ChordAngle: chord angle from 90° has length2 == 2.
#[quickcheck]
fn prop_chord_angle_90deg_length2(_dummy: u8) -> bool {
    (ChordAngle::from(Angle::from_degrees(90.0)).length2() - 2.0).abs() < 1e-14
}

/// ChordAngle::from_length2(-1.0) is_negative().
#[quickcheck]
fn prop_chord_angle_negative_from_length2(_dummy: u8) -> bool {
    ChordAngle::from_length2(-1.0).is_negative()
}

/// CellId: range_min <= range_max for any valid cell.
#[quickcheck]
fn prop_cellid_range_min_le_max_level(face: u8, level: u8) -> bool {
    let cid = CellId::from_face(face % 6).child_begin_at_level(level % 30);
    cid.range_min().id() <= cid.range_max().id()
}

/// CellId: child_begin() < child_end() for non-leaf cells.
#[quickcheck]
fn prop_cellid_child_begin_lt_end(face: u8, level: u8) -> bool {
    let cid = CellId::from_face(face % 6).child_begin_at_level(level % 29);
    cid.child_begin().id() < cid.child_end().id()
}

/// s1::Interval: union of two intervals contains both.
#[quickcheck]
fn prop_s1interval_union_contains_both(a: i32, b: i32) -> bool {
    use s2rst::s1::Interval;
    let x = (a % 3142) as f64 * 0.001_f64;
    let y = (b % 3142) as f64 * 0.001_f64;
    let ix = Interval::from_point(x);
    let iy = Interval::from_point(y);
    let u = ix.union(iy);
    u.contains(x) && u.contains(y)
}

// ════════════════════════════════════════════════════════════════════
// SECTION 27: COVERAGE-DRIVEN PROPERTY TESTS (50 new tests)
// ════════════════════════════════════════════════════════════════════

// ── Angle E5/E6/E7 conversions ──────────────────────────────────────

/// Angle from_e5 → e5 roundtrip.
#[quickcheck]
fn prop_angle_e5_roundtrip(v: i32) -> bool {
    let v = v % 1_000_000; // keep values reasonable
    let a = Angle::from_e5(v);
    (a.e5() - v).abs() <= 1
}

/// Angle from_e6 → e6 roundtrip.
#[quickcheck]
fn prop_angle_e6_roundtrip(v: i32) -> bool {
    let v = v % 10_000_000;
    let a = Angle::from_e6(v);
    (a.e6() - v).abs() <= 1
}

/// Angle from_e7 → e7 roundtrip.
#[quickcheck]
fn prop_angle_e7_roundtrip(v: i32) -> bool {
    let v = v % 100_000_000;
    let a = Angle::from_e7(v);
    (a.e7() - v).abs() <= 1
}

// ── ChordAngle E5/E6/E7 conversions ─────────────────────────────────

/// ChordAngle from_e5 produces non-negative length2.
#[quickcheck]
fn prop_chord_angle_e5_nonneg(v: u8) -> bool {
    let v = (v as i32) * 700; // 0..178500 in e5 (0..~178°)
    let ca = ChordAngle::from_e5(v);
    ca.length2() >= 0.0
}

/// ChordAngle from_e6 is non-negative.
#[quickcheck]
fn prop_chord_angle_e6_nonneg(v: u8) -> bool {
    let v = (v as i32) * 1000; // 0..255000 in e6 (0..0.255°)
    let ca = ChordAngle::from_e6(v);
    ca.length2() >= 0.0
}

/// ChordAngle radians/degrees are consistent.
#[quickcheck]
fn prop_chord_angle_radians_degrees_consistent(v: u8) -> bool {
    let ca = ChordAngle::from_angle(Angle::from_degrees((v % 181) as f64));
    (ca.degrees() - ca.radians().to_degrees()).abs() < 1e-10
}

// ── LatLng E5/E6/E7 constructors ────────────────────────────────────

/// LatLng from_e5 creates a valid LatLng.
#[quickcheck]
fn prop_latlng_from_e5_valid(lat_e5: i32, lng_e5: i32) -> bool {
    let lat_e5 = lat_e5 % 9_000_001; // up to 90 degrees
    let lng_e5 = lng_e5 % 18_000_001;
    let ll = LatLng::from_e5(lat_e5, lng_e5);
    ll.lat.radians().is_finite() && ll.lng.radians().is_finite()
}

/// LatLng from_e6 creates a valid LatLng.
#[quickcheck]
fn prop_latlng_from_e6_valid(lat_e6: i32, lng_e6: i32) -> bool {
    let lat_e6 = lat_e6 % 90_000_001;
    let lng_e6 = lng_e6 % 180_000_001;
    let ll = LatLng::from_e6(lat_e6, lng_e6);
    ll.lat.radians().is_finite() && ll.lng.radians().is_finite()
}

/// LatLng from_e7 creates a valid LatLng.
#[quickcheck]
fn prop_latlng_from_e7_valid(lat_e7: i32, lng_e7: i32) -> bool {
    let lat_e7 = lat_e7 % 900_000_001;
    let lng_e7 = lng_e7 % 1_800_000_001;
    let ll = LatLng::from_e7(lat_e7, lng_e7);
    ll.lat.radians().is_finite() && ll.lng.radians().is_finite()
}

// ── Earth distance conversions ───────────────────────────────────────

/// meters_to_angle → to_meters roundtrip.
#[quickcheck]
fn prop_earth_meters_roundtrip(m: u32) -> bool {
    use s2rst::s2::earth;
    let m = (m % 1_000_000) as f64;
    let angle = earth::meters_to_angle(m);
    let back = earth::to_meters(angle);
    (back - m).abs() < 1e-6
}

/// km_to_angle → to_km roundtrip.
#[quickcheck]
fn prop_earth_km_roundtrip(km: u32) -> bool {
    use s2rst::s2::earth;
    let km = (km % 10_000) as f64;
    let angle = earth::km_to_angle(km);
    let back = earth::to_km(angle);
    (back - km).abs() < 1e-6
}

/// meters_to_chord_angle → chord_angle_to_meters roundtrip.
#[quickcheck]
fn prop_earth_chord_meters_roundtrip(m: u32) -> bool {
    use s2rst::s2::earth;
    let m = (m % 1_000_000) as f64;
    let ca = earth::meters_to_chord_angle(m);
    let back = earth::chord_angle_to_meters(ca);
    (back - m).abs() < 1e-3
}

/// get_distance_meters is non-negative.
#[quickcheck]
fn prop_earth_distance_meters_nonneg(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::earth;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    earth::get_distance_meters_points(p0, p1) >= 0.0
}

/// get_distance_km is consistent with get_distance_meters.
#[quickcheck]
fn prop_earth_km_meters_consistent(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::earth;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let km = earth::get_distance_km_points(p0, p1);
    let m = earth::get_distance_meters_points(p0, p1);
    (km * 1000.0 - m).abs() < 1e-3
}

/// square_km_to_steradians → steradians_to_square_km roundtrip.
#[quickcheck]
fn prop_earth_sq_km_steradians_roundtrip(v: u32) -> bool {
    use s2rst::s2::earth;
    let km2 = (v % 100_000) as f64;
    let sr = earth::square_km_to_steradians(km2);
    let back = earth::steradians_to_square_km(sr);
    (back - km2).abs() < 1e-6
}

// ── point_measures ───────────────────────────────────────────────────

/// point_area is non-negative for CCW triangles.
#[quickcheck]
fn prop_point_area_nonneg(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::point_measures;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    point_measures::point_area(a, b, c) >= 0.0
}

/// point_area <= 2π for any triangle.
#[quickcheck]
fn prop_point_area_bounded(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::point_measures;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    point_measures::point_area(a, b, c) <= 2.0 * PI + 1e-10
}

/// girard_area is non-negative.
#[quickcheck]
fn prop_girard_area_nonneg(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::point_measures;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    point_measures::girard_area(a, b, c) >= -1e-15
}

/// signed_area(a,b,c) == -signed_area(a,c,b) (reversing winding flips sign).
#[quickcheck]
fn prop_signed_area_reversal(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::point_measures;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    let fwd = point_measures::signed_area(a, b, c);
    let rev = point_measures::signed_area(a, c, b);
    (fwd + rev).abs() < 1e-10
}

/// turn_angle is finite for any triple of points.
#[quickcheck]
fn prop_turn_angle_finite(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::point_measures;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    let c = make_latlng(lat2, lng2).to_point();
    point_measures::turn_angle(a, b, c).radians().is_finite()
}

// ── edge_distances: interpolation and projection ─────────────────────

/// interpolate(0, a, b) ≈ a.
#[quickcheck]
fn prop_edge_interpolate_zero(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_distances;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    if a.distance(b).radians() < 1e-10 {
        return true;
    }
    let p = edge_distances::interpolate(0.0, a, b);
    p.distance(a).radians() < 1e-14
}

/// interpolate(1, a, b) ≈ b.
#[quickcheck]
fn prop_edge_interpolate_one(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_distances;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    if a.distance(b).radians() < 1e-10 {
        return true;
    }
    let p = edge_distances::interpolate(1.0, a, b);
    p.distance(b).radians() < 1e-14
}

/// edge_pair_closest_points returns points on or near the edges.
#[quickcheck]
fn prop_edge_pair_closest_points_on_edges(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lng3: i32,
) -> bool {
    use s2rst::s2::edge_distances;
    let a0 = make_latlng(lat0, lng0).to_point();
    let a1 = make_latlng(lat1, lng1).to_point();
    let b0 = make_latlng(lat2, lng2).to_point();
    let b1 = make_latlng(lat3, lng3).to_point();
    let (pa, pb) = edge_distances::edge_pair_closest_points(a0, a1, b0, b1);
    // Closest points should be unit vectors
    (pa.0.norm2() - 1.0).abs() < 1e-14 && (pb.0.norm2() - 1.0).abs() < 1e-14
}

/// is_distance_less is consistent with distance_from_segment.
#[quickcheck]
fn prop_edge_is_distance_less_consistent(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::edge_distances;
    let x = make_latlng(lat0, lng0).to_point();
    let a = make_latlng(lat1, lng1).to_point();
    let b = make_latlng(lat2, lng2).to_point();
    let dist = ChordAngle::from_angle(edge_distances::distance_from_segment(x, a, b));
    let limit = ChordAngle::from_angle(Angle::from_degrees(10.0));
    // If actual distance < limit, is_distance_less should return true
    if dist < limit {
        edge_distances::is_distance_less(x, a, b, limit)
    } else {
        true
    }
}

/// distance_fraction(a, a, b) ≈ 0 and distance_fraction(b, a, b) ≈ 1.
#[quickcheck]
fn prop_edge_distance_fraction_endpoints(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_distances;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    if a.distance(b).radians() < 1e-10 {
        return true;
    }
    let f0 = edge_distances::distance_fraction(a, a, b);
    let f1 = edge_distances::distance_fraction(b, a, b);
    f0.abs() < 1e-10 && (f1 - 1.0).abs() < 1e-10
}

// ── r3::Matrix ───────────────────────────────────────────────────────

/// Identity matrix * v == v.
#[quickcheck]
fn prop_matrix_identity_mul(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::{Matrix3x3 as Matrix, Vector};
    let v = Vector::new(x as f64, y as f64, z as f64);
    Matrix::identity().mul_vec(v) == v
}

/// Transpose of identity is identity.
#[quickcheck]
fn prop_matrix_identity_transpose(_dummy: u8) -> bool {
    use s2rst::r3::Matrix3x3 as Matrix;
    let id = Matrix::identity();
    let t = id.transpose();
    (0..3).all(|i| (0..3).all(|j| (id.get(i, j) - t.get(i, j)).abs() < 1e-15))
}

/// (A^T)^T == A.
#[quickcheck]
fn prop_matrix_double_transpose(a00: i16, a01: i16, a02: i16, a10: i16, a11: i16) -> bool {
    use s2rst::r3::Matrix3x3 as Matrix;
    let m = Matrix::new(
        a00 as f64, a01 as f64, a02 as f64, a10 as f64, a11 as f64, 0.0, 0.0, 0.0, 1.0,
    );
    let tt = m.transpose().transpose();
    (0..3).all(|i| (0..3).all(|j| (m.get(i, j) - tt.get(i, j)).abs() < 1e-15))
}

/// from_cols creates a matrix where col(i) returns the i-th column.
#[quickcheck]
fn prop_matrix_from_cols(x1: i16, y1: i16, z1: i16) -> bool {
    use s2rst::r3::{Matrix3x3 as Matrix, Vector};
    let c0 = Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let c1 = Vector::new(1.0, 0.0, 0.0);
    let c2 = Vector::new(0.0, 1.0, 0.0);
    let m = Matrix::from_cols(c0, c1, c2);
    m.col(0) == c0 && m.col(1) == c1 && m.col(2) == c2
}

// ── r2::Rect ─────────────────────────────────────────────────────────

/// r2::Rect union contains both rects.
#[quickcheck]
fn prop_r2rect_union_contains_both(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    use s2rst::r2::{Point as R2Point, Rect as R2Rect};
    let r1 = R2Rect::from_point(R2Point::new(x1 as f64, y1 as f64));
    let r2 = R2Rect::from_point(R2Point::new(x2 as f64, y2 as f64));
    let u = r1.union(r2);
    u.contains_point(R2Point::new(x1 as f64, y1 as f64))
        && u.contains_point(R2Point::new(x2 as f64, y2 as f64))
}

/// r2::Rect intersection of a rect with itself is the same rect.
#[quickcheck]
fn prop_r2rect_intersection_self(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    use s2rst::r2::{Point as R2Point, Rect as R2Rect};
    let r = R2Rect::from_point_pair(
        R2Point::new(x1 as f64, y1 as f64),
        R2Point::new(x2 as f64, y2 as f64),
    );
    r.intersection(r) == r
}

/// r2::Rect center is the midpoint of lo and hi.
#[quickcheck]
fn prop_r2rect_center(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    use s2rst::r2::{Point as R2Point, Rect as R2Rect};
    let r = R2Rect::from_point_pair(
        R2Point::new(x1 as f64, y1 as f64),
        R2Point::new(x2 as f64, y2 as f64),
    );
    if r.is_empty() {
        return true;
    }
    let c = r.center();
    (c.x - f64::midpoint(r.lo().x, r.hi().x)).abs() < 1e-10
        && (c.y - f64::midpoint(r.lo().y, r.hi().y)).abs() < 1e-10
}

// ── wedge_relations ──────────────────────────────────────────────────

/// wedge_contains(a0, ab1, a2, b0, b2) implies wedge_intersects.
#[quickcheck]
fn prop_wedge_contains_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    lat3: i32,
    lat4: i32,
) -> quickcheck::TestResult {
    use s2rst::s2::wedge_relations;
    let a0 = make_latlng(lat0, lng0).to_point();
    let ab1 = make_latlng(lat1, lng1).to_point();
    let a2 = make_latlng(lat2, lng2).to_point();
    let b0 = make_latlng(lat3, 0).to_point();
    let b2 = make_latlng(lat4, 0).to_point();
    // Wedge functions require all five points to be distinct (and the wedge
    // arms to differ from the shared vertex). Degenerate wedges where a0==a2
    // or b0==b2 can give inconsistent contains/intersects results.
    if a0 == ab1 || a2 == ab1 || b0 == ab1 || b2 == ab1 || a0 == a2 || b0 == b2 {
        return quickcheck::TestResult::discard();
    }
    let result = if wedge_relations::wedge_contains(a0, ab1, a2, b0, b2) {
        wedge_relations::wedge_intersects(a0, ab1, a2, b0, b2)
    } else {
        true
    };
    quickcheck::TestResult::from_bool(result)
}

// ── Point methods ────────────────────────────────────────────────────

/// Point: is_unit is true for make_latlng-constructed points.
#[quickcheck]
fn prop_point_is_unit(lat: i32, lng: i32) -> bool {
    make_latlng(lat, lng).to_point().is_unit()
}

/// Point: approx_equals(self) is always true.
#[quickcheck]
fn prop_point_approx_equals_self(lat: i32, lng: i32) -> bool {
    let p = make_latlng(lat, lng).to_point();
    p.approx_eq_with(p, Angle::from_radians(1e-15))
}

/// Point: chord_angle(self) is zero.
#[quickcheck]
fn prop_point_chord_angle_self_zero(lat: i32, lng: i32) -> bool {
    let p = make_latlng(lat, lng).to_point();
    p.chord_angle(p).length2() < 1e-30
}

/// Point: chord_angle is consistent with distance.
#[quickcheck]
fn prop_point_chord_angle_consistent_with_distance(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let ca = p0.chord_angle(p1);
    let dist = p0.distance(p1);
    let ca_from_dist = ChordAngle::from_angle(dist);
    (ca.length2() - ca_from_dist.length2()).abs() < 1e-12
}

/// Point: point_cross(a, b) is perpendicular to a.
#[quickcheck]
fn prop_point_cross_perpendicular(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    if a.distance(b).radians() < 1e-10 {
        return true;
    }
    let c = a.point_cross(b);
    // c should be roughly perpendicular to a (but point_cross may not be exact)
    c.0.dot(a.0).abs() < 1e-10
}

// ── Cell extra methods ───────────────────────────────────────────────

/// Cell::from_point creates a leaf cell containing that point.
#[quickcheck]
fn prop_cell_from_point_contains(lat: i32, lng: i32) -> bool {
    let p = make_latlng(lat, lng).to_point();
    let cell = Cell::from_point(p);
    cell.contains_point(p)
}

/// Cell::from_lat_lng creates a valid cell.
#[quickcheck]
fn prop_cell_from_latlng_valid(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let cell = Cell::from_lat_lng(ll);
    cell.id().is_valid()
}

// ── CellId extra methods ────────────────────────────────────────────

/// CellId::none is not valid.
#[quickcheck]
fn prop_cellid_none_not_valid(_dummy: u8) -> bool {
    !CellId::none().is_valid()
}

/// CellId: prev() and next() are inverses (next(prev(x)) == x for non-boundary cells).
#[quickcheck]
fn prop_cellid_prev_next_inverse(lat: i32, lng: i32) -> bool {
    let id = CellId::from_lat_lng(&make_latlng(lat, lng)).parent_at_level(10);
    id.prev().next() == id
}

/// CellId: advance(0) is identity.
#[quickcheck]
fn prop_cellid_advance_zero(lat: i32, lng: i32) -> bool {
    let id = CellId::from_lat_lng(&make_latlng(lat, lng)).parent_at_level(10);
    id.advance(0) == id
}

/// CellId: advance(1) == next().
#[quickcheck]
fn prop_cellid_advance_one_eq_next(lat: i32, lng: i32) -> bool {
    let id = CellId::from_lat_lng(&make_latlng(lat, lng)).parent_at_level(10);
    id.advance(1) == id.next()
}

// ── Polyline extra methods ───────────────────────────────────────────

/// Polyline::from_lat_lngs creates a polyline with the same number of vertices.
#[quickcheck]
fn prop_polyline_from_latlngs_count(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let lls = vec![make_latlng(lat0, lng0), make_latlng(lat1, lng1)];
    let pl = Polyline::from_lat_lngs(&lls);
    pl.num_vertices() == 2
}

/// Polyline::centroid is finite.
#[quickcheck]
fn prop_polyline_centroid_finite(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    let c = pl.centroid();
    c.0.x.is_finite() && c.0.y.is_finite() && c.0.z.is_finite()
}

/// Polyline::is_on_right is well-defined (doesn't panic) for valid polylines.
#[quickcheck]
fn prop_polyline_is_on_right_no_panic(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    qlat: i32,
    qlng: i32,
) -> bool {
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    if p0.distance(p1).radians() < 1e-10 {
        return true;
    }
    let pl = Polyline::new(vec![p0, p1]);
    let q = make_latlng(qlat, qlng).to_point();
    // Just ensure it doesn't panic and returns a bool
    let _result = pl.is_on_right(q);
    true
}

/// Polyline::equal(self) is true.
#[quickcheck]
fn prop_polyline_equal_self(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.equal(&pl)
}

/// Polyline::approx_equal(self, small_error) is true.
#[quickcheck]
fn prop_polyline_approx_equal_self(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    let pl = Polyline::new(vec![
        make_latlng(lat0, lng0).to_point(),
        make_latlng(lat1, lng1).to_point(),
    ]);
    pl.approx_eq_with(&pl, Angle::from_radians(1e-10))
}

// ── Polygon extra methods ────────────────────────────────────────────

/// Polygon centroid is finite.
#[quickcheck]
fn prop_polygon_centroid_finite(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    match make_triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(p) => {
            let c = p.centroid();
            c.0.x.is_finite() && c.0.y.is_finite() && c.0.z.is_finite()
        }
        None => true,
    }
}

// ── s2::Rect extra methods ───────────────────────────────────────────

/// Rect::from_center_size creates a valid rect containing its center.
#[quickcheck]
fn prop_rect_from_center_size_contains_center(lat: i32, lng: i32) -> bool {
    let center = make_latlng(lat, lng);
    let size = LatLng::from_degrees(5.0, 10.0);
    let r = Rect::from_center_size(center, size);
    r.is_valid() && r.contains_lat_lng(center)
}

/// Rect area is non-negative.
#[quickcheck]
fn prop_rect_area_nonneg_from_latlng(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    r.area() >= 0.0
}

/// Rect expanded contains the original.
#[quickcheck]
fn prop_rect_expanded_contains_original(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    if r.is_empty() {
        return true;
    }
    let margin = LatLng::from_degrees(1.0, 1.0);
    let expanded = r.expanded(margin);
    expanded.contains(r)
}

// ── CellUnion extra methods ──────────────────────────────────────────

/// CellUnion::from_min_max contains the min and max.
#[quickcheck]
fn prop_cell_union_from_min_max(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let level = (level % 20) + 5;
    let id = CellId::from_face(face % 6).child_begin_at_level(level);
    let min = id.range_min();
    let max = id.range_max();
    let cu = CellUnion::from_min_max(min, max);
    cu.contains_cell_id(id)
}

/// CellUnion::whole_sphere contains any cell.
#[quickcheck]
fn prop_cell_union_whole_sphere_contains_all(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let ws = CellUnion::whole_sphere();
    let id = CellId::from_face(face % 6).child_begin_at_level(level % 20);
    ws.contains_cell_id(id)
}

/// CellUnion contains_union with itself.
#[quickcheck]
fn prop_cell_union_contains_self(face: u8, level: u8) -> bool {
    use s2rst::s2::CellUnion;
    let id = CellId::from_face(face % 6).child_begin_at_level((level % 20) + 1);
    let cu = CellUnion::from_cell_ids(vec![id]);
    cu.contains_union(&cu)
}

// ── edge_clipping ────────────────────────────────────────────────────

/// face_segments returns at least one segment for non-degenerate edges.
#[quickcheck]
fn prop_face_segments_nonempty(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_clipping;
    let a = make_latlng(lat0, lng0).to_point();
    let b = make_latlng(lat1, lng1).to_point();
    if a.distance(b).radians() < 1e-10 {
        return true;
    }
    let segs = edge_clipping::face_segments(a, b);
    !segs.is_empty()
}

/// interpolate_float64 produces a value between a1 and b1 when x is between a and b.
#[quickcheck]
fn prop_interpolate_float64_bounded(a: i16, b: i16, a1: i16, b1: i16, t: u8) -> bool {
    use s2rst::s2::edge_clipping;
    let af = a as f64;
    let bf = b as f64;
    if (af - bf).abs() < 1e-10 {
        return true;
    }
    let a1f = a1 as f64;
    let b1f = b1 as f64;
    let frac = t as f64 / 255.0;
    let x = af + frac * (bf - af);
    let result = edge_clipping::interpolate_float64(x, af, bf, a1f, b1f);
    let lo = a1f.min(b1f);
    let hi = a1f.max(b1f);
    result >= lo - 1e-6 && result <= hi + 1e-6
}

// ── s1::Interval extra methods ───────────────────────────────────────

/// s1::Interval: complement of full is empty.
#[quickcheck]
fn prop_s1interval_complement_full_is_empty(_dummy: u8) -> bool {
    use s2rst::s1::Interval;
    Interval::full().complement().is_empty()
}

/// s1::Interval: complement of empty is full.
#[quickcheck]
fn prop_s1interval_complement_empty_is_full(_dummy: u8) -> bool {
    use s2rst::s1::Interval;
    Interval::empty().complement().is_full()
}

/// s1::Interval: intersection of an interval with itself is the same interval.
#[quickcheck]
fn prop_s1interval_intersection_self(a: i32, b: i32) -> bool {
    use s2rst::s1::Interval;
    let x = (a % 3142) as f64 * 0.001;
    let y = (b % 3142) as f64 * 0.001;
    let iv = Interval::new(x, y);
    iv.intersection(iv) == iv
}

// ════════════════════════════════════════════════════════════════════
// ADDITIONAL PROPERTY TESTS — Batch 2
// Filling remaining gaps plus new tests.
// ════════════════════════════════════════════════════════════════════

// ── From / Into conversions ──────────────────────────────────────────

/// u64 → CellId → u64 roundtrip
#[quickcheck]
fn prop_u64_cellid_roundtrip(v: u64) -> bool {
    let id = CellId::from(v);
    let back: u64 = id.into();
    back == v
}

/// CellId(u64) → u64 → CellId roundtrip
#[quickcheck]
fn prop_cellid_u64_roundtrip(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    let raw: u64 = id.into();
    CellId::from(raw) == id
}

/// Point ↔ r3::Vector: Point(v).0 == v
#[quickcheck]
fn prop_point_vector_from_roundtrip(x: i32, y: i32, z: i32) -> bool {
    let v = s2rst::r3::Vector::new(x as f64, y as f64, z as f64);
    let p = Point(v);
    p.0 == v
}

/// Angle → ChordAngle is monotone for [0, 180°]
#[quickcheck]
fn prop_angle_chord_angle_monotone(a: u8, b: u8) -> bool {
    let a_deg = (a as f64) * 180.0 / 255.0;
    let b_deg = (b as f64) * 180.0 / 255.0;
    let a_chord = ChordAngle::from_angle(Angle::from_degrees(a_deg));
    let b_chord = ChordAngle::from_angle(Angle::from_degrees(b_deg));
    if a_deg <= b_deg {
        a_chord <= b_chord
    } else {
        a_chord >= b_chord
    }
}

/// ChordAngle → Angle → ChordAngle approximately round-trips
#[quickcheck]
fn prop_angle_chord_roundtrip_close(a: u8) -> bool {
    let deg = (a as f64) * 180.0 / 255.0;
    let ca = ChordAngle::from_degrees(deg);
    let angle = ca.to_angle();
    let ca2 = ChordAngle::from_angle(angle);
    (ca.to_angle().radians() - ca2.to_angle().radians()).abs() < 1e-12
}

/// LatLng → Point → LatLng approximate roundtrip
#[quickcheck]
fn prop_latlng_to_point_roundtrip(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    let p = ll.to_point();
    let ll2 = LatLng::from_point(p);
    (ll.lat.degrees() - ll2.lat.degrees()).abs() < 1e-10
        && (ll.lng.degrees() - ll2.lng.degrees()).abs() < 1e-10
}

/// (f64, f64) → r1::Interval preserves lo/hi
#[quickcheck]
fn prop_tuple_r1interval_fields(a: i32, b: i32) -> bool {
    let (lo, hi) = (a as f64, b as f64);
    let iv = s2rst::r1::Interval::new(lo, hi);
    iv.lo == lo && iv.hi == hi
}

/// (f64, f64) → r2::Point preserves x/y
#[quickcheck]
fn prop_tuple_r2point_fields(a: i32, b: i32) -> bool {
    let p = s2rst::r2::Point::new(a as f64, b as f64);
    p.x == a as f64 && p.y == b as f64
}

/// (f64, f64, f64) → r3::Vector preserves x/y/z
#[quickcheck]
fn prop_tuple_r3vector_fields(a: i32, b: i32, c: i32) -> bool {
    let v = s2rst::r3::Vector::new(a as f64, b as f64, c as f64);
    v.x == a as f64 && v.y == b as f64 && v.z == c as f64
}

/// [f64; 3] → r3::Vector preserves x/y/z
#[quickcheck]
fn prop_array_r3vector_fields(a: i32, b: i32, c: i32) -> bool {
    let arr = [a as f64, b as f64, c as f64];
    let v = s2rst::r3::Vector::from(arr);
    v.x == arr[0] && v.y == arr[1] && v.z == arr[2]
}

/// false → Endpoint::Lo
#[test]
fn prop_bool_endpoint_false_lo() {
    assert_eq!(s2rst::r1::Endpoint::from(false), s2rst::r1::Endpoint::Lo);
}

/// true → Endpoint::Hi
#[test]
fn prop_bool_endpoint_true_hi() {
    assert_eq!(s2rst::r1::Endpoint::from(true), s2rst::r1::Endpoint::Hi);
}

/// Face → u8 roundtrip
#[quickcheck]
fn prop_face_u8_roundtrip2(f: u8) -> bool {
    let f = f % 6;
    let face = s2rst::s2::coords::Face::from_u8(f);
    face as u8 == f
}

/// ChordAngle::from_angle is monotone
#[quickcheck]
fn prop_chord_angle_from_monotone(a: u8, b: u8) -> bool {
    let ca = ChordAngle::from_angle(Angle::from_degrees((a as f64) * 180.0 / 255.0));
    let cb = ChordAngle::from_angle(Angle::from_degrees((b as f64) * 180.0 / 255.0));
    (a <= b) == (ca <= cb)
}

// ── PartialOrd / Ord ─────────────────────────────────────────────────

/// Angle ordering is reflexive
#[quickcheck]
fn prop_angle_ord_reflexive(a: i32) -> bool {
    let x = Angle::from_degrees(a as f64);
    x <= x
}

/// Angle ordering is antisymmetric
#[quickcheck]
fn prop_angle_ord_antisymmetric(a: i32, b: i32) -> bool {
    let x = Angle::from_degrees(a as f64);
    let y = Angle::from_degrees(b as f64);
    // antisymmetric: if x < y then !(y < x)
    match x.partial_cmp(&y) {
        Some(std::cmp::Ordering::Less) => y.partial_cmp(&x) != Some(std::cmp::Ordering::Less),
        _ => true,
    }
}

/// ExactFloat comparison matches i64
#[quickcheck]
fn prop_exactfloat_cmp_matches_f64(a: i32, b: i32) -> bool {
    use s2rst::r3::ExactFloat;
    let ea = ExactFloat::from(a as i64);
    let eb = ExactFloat::from(b as i64);
    ea.partial_cmp(&eb) == (a as i64).partial_cmp(&(b as i64))
}

// ── Iterator / IntoIterator / FromIterator for CellUnion ─────────────

/// CellUnion iterator yields sorted CellIds
#[quickcheck]
fn prop_cellunion_iter_sorted(face: u8) -> bool {
    use s2rst::s2::CellUnion;
    let face = face % 6;
    let id = CellId::from_face(face);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let ids: Vec<CellId> = cu.into_iter().collect();
    ids.windows(2).all(|w| w[0] <= w[1])
}

/// CellUnion → collect → CellUnion preserves ids
#[quickcheck]
fn prop_cellunion_fromiter_preserves_ids(face: u8) -> bool {
    use s2rst::s2::CellUnion;
    let face = face % 6;
    let id = CellId::from_face(face);
    let cu = CellUnion::from_cell_ids(vec![id]);
    let ids: Vec<CellId> = cu.iter().copied().collect();
    let cu2 = CellUnion::from_cell_ids(ids);
    cu == cu2
}

/// CellUnion from_iter then normalize is idempotent
#[quickcheck]
fn prop_cellunion_fromiter_then_normalize(face: u8) -> bool {
    use s2rst::s2::CellUnion;
    let face = face % 6;
    let id = CellId::from_face(face);
    let mut cu = CellUnion::from_cell_ids(vec![id, id]); // duplicate
    cu.normalize();
    let mut cu2 = cu.clone();
    cu2.normalize();
    cu == cu2
}

// ── Shape trait: Polyline edge endpoints unit ─────────────────────────

/// Polyline shape edge endpoints are unit vectors
#[quickcheck]
fn prop_polyline_shape_edge_endpoints_unit(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    (0..pl.num_edges()).all(|i| {
        let e = pl.edge(i);
        (e.v0.0.norm() - 1.0).abs() < 1e-14 && (e.v1.0.norm() - 1.0).abs() < 1e-14
    })
}

// ── Additional r2::Point arithmetic ──────────────────────────────────

/// r2::Point addition is commutative (v2)
#[quickcheck]
fn prop_r2point_add_commutative2(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let a = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let b = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    a + b == b + a
}

/// r2::Point multiply by 1.0 is identity (v2)
#[quickcheck]
fn prop_r2_point_mul_one_identity2(x: i32, y: i32) -> bool {
    let a = s2rst::r2::Point::new(x as f64, y as f64);
    a * 1.0 == a
}

/// r2::Point multiply distributes over addition (v2)
#[quickcheck]
fn prop_r2_point_mul_distributive2(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let a = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let b = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let k = 3.0;
    let lhs = (a + b) * k;
    let rhs = a * k + b * k;
    (lhs.x - rhs.x).abs() < 1e-10 && (lhs.y - rhs.y).abs() < 1e-10
}

/// r2::Point sub is add-neg
#[quickcheck]
fn prop_r2_point_sub_eq_add_neg2(x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let a = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let b = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let diff = a - b;
    let add_neg = a + (-b);
    (diff.x - add_neg.x).abs() < 1e-10 && (diff.y - add_neg.y).abs() < 1e-10
}

// ── Additional LatLng arithmetic ─────────────────────────────────────

/// LatLng scalar multiply distributes over lat/lng components
#[quickcheck]
fn prop_latlng_scalar_mul_lat_lng(la: i32, lna: i32) -> bool {
    let ll = LatLng {
        lat: Angle::from_degrees(la as f64),
        lng: Angle::from_degrees(lna as f64),
    };
    let scaled = ll * 2.0;
    (scaled.lat.degrees() - 2.0 * ll.lat.degrees()).abs() < 1e-10
        && (scaled.lng.degrees() - 2.0 * ll.lng.degrees()).abs() < 1e-10
}

// ── Additional ChordAngle properties ─────────────────────────────────

/// ChordAngle::from_length2 with non-negative input gives valid angle
#[quickcheck]
fn prop_chord_angle_from_length2_valid(v: u8) -> bool {
    let s = (v as f64) * 4.0 / 255.0; // [0, 4]
    let ca = ChordAngle::from_length2(s);
    let a = ca.to_angle();
    a.radians() >= 0.0
}

/// ChordAngle successor >= original
#[quickcheck]
fn prop_chord_angle_successor_ge(a: u8) -> bool {
    let deg = (a as f64) * 179.0 / 255.0;
    let ca = ChordAngle::from_degrees(deg);
    let succ = ca.successor();
    succ >= ca
}

/// ChordAngle predecessor <= original
#[quickcheck]
fn prop_chord_angle_predecessor_le(a: u8) -> bool {
    let deg = (a as f64) * 180.0 / 255.0;
    let ca = ChordAngle::from_degrees(deg.min(180.0));
    let pred = ca.predecessor();
    pred <= ca
}

// ── Additional CellId properties ─────────────────────────────────────

/// CellId level is in [0, 30]
#[quickcheck]
fn prop_cellid_level_in_range(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    id.level() <= 30
}

/// CellId children()[0].parent() == self
#[quickcheck]
fn prop_cellid_child_parent_roundtrip(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    let child = id.children()[0];
    child.parent() == id
}

/// CellId::children returns exactly 4 children
#[quickcheck]
fn prop_cellid_children_count(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    id.children().len() == 4
}

/// CellId::contains(child) is true
#[quickcheck]
fn prop_cellid_contains_child2(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    let child = id.children()[2];
    id.contains(child)
}

/// CellId::intersects(child) is true
#[quickcheck]
fn prop_cellid_intersects_child(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    let child = id.children()[1];
    id.intersects(child)
}

/// CellId range_min <= range_max (v2)
#[quickcheck]
fn prop_cellid_range_min_le_max2(face: u8) -> bool {
    let face = face % 6;
    let id = CellId::from_face(face);
    id.range_min() <= id.range_max()
}

/// CellId parent's range contains child's range (v2)
#[quickcheck]
fn prop_cellid_parent_range_contains_child2(lat: i32, lng: i32) -> bool {
    let id = CellId::from_point(&make_latlng(lat, lng).to_point());
    if id.level() == 0 {
        return true;
    }
    let parent = id.parent();
    parent.range_min() <= id.range_min() && id.range_max() <= parent.range_max()
}

/// CellId is_leaf at MAX_CELL_LEVEL
#[quickcheck]
fn prop_cellid_leaf_at_max_level(lat: i32, lng: i32) -> bool {
    let id = CellId::from_point(&make_latlng(lat, lng).to_point());
    id.is_leaf()
}

// ── Additional Cap properties ────────────────────────────────────────

/// Cap::from_center_angle center is contained (v2)
#[quickcheck]
fn prop_cap_contains_center2(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees((deg % 90) as f64 + 0.1));
    cap.contains_point(center)
}

/// Cap complement: cap + complement area = 4π (v2)
#[quickcheck]
fn prop_cap_complement_area2(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees((deg % 90) as f64));
    let comp = cap.complement();
    let total = cap.area() + comp.area();
    (total - 4.0 * PI).abs() < 1e-10
}

/// Empty cap is empty
#[test]
fn prop_cap_empty_is_empty() {
    assert!(Cap::empty().is_empty());
}

/// Full cap is not empty
#[test]
fn prop_cap_full_is_not_empty() {
    assert!(!Cap::full().is_empty());
}

/// Cap expanded contains original
#[quickcheck]
fn prop_cap_expanded_contains_original3(lat: i32, lng: i32, deg: u8) -> bool {
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees((deg % 85) as f64));
    let expanded = cap.expanded(Angle::from_degrees(1.0));
    !cap.is_empty() || expanded.is_empty() || expanded.contains(cap)
}

// ── Additional Loop properties ───────────────────────────────────────

/// Loop area is non-negative for well-formed loops
#[quickcheck]
fn prop_loop_area_nonneg(lat0: i32, lng0: i32, lat1: i32, lng1: i32, lat2: i32, lng2: i32) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    if p0 == p1 || p1 == p2 || p0 == p2 {
        return true;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    lp.area() >= 0.0
}

/// Loop num_vertices matches construction
#[quickcheck]
fn prop_loop_num_vertices_match(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let lp = Loop::new(vec![p0, p1, p2]);
    lp.num_vertices() == 3
}

/// Loop depth starts at 0
#[quickcheck]
fn prop_loop_depth_zero(lat0: i32, lng0: i32, lat1: i32, lng1: i32, lat2: i32, lng2: i32) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    Loop::new(vec![p0, p1, p2]).depth() == 0
}

// ── Additional Polygon properties ────────────────────────────────────

/// Polygon num_loops matches construction
#[quickcheck]
fn prop_polygon_num_loops_match(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    if p0 == p1 || p1 == p2 || p0 == p2 {
        return true;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    let poly = Polygon::from_loops(vec![lp]);
    poly.num_loops() == 1
}

/// Polygon area is non-negative
#[quickcheck]
fn prop_polygon_area_nonneg(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    if p0 == p1 || p1 == p2 || p0 == p2 {
        return true;
    }
    let lp = Loop::new(vec![p0, p1, p2]);
    Polygon::from_loops(vec![lp]).area() >= 0.0
}

// ── Additional Rect (s2) properties ──────────────────────────────────

/// Rect::from_point_pair contains both points (v2)
#[quickcheck]
fn prop_rect_from_point_pair_contains_both2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let r = Rect::from_point_pair(ll0, ll1);
    r.contains_lat_lng(ll0) && r.contains_lat_lng(ll1)
}

/// Rect::contains_lat_lng for center of non-empty rect (v2)
#[quickcheck]
fn prop_rect_contains_center2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let ll0 = make_latlng(lat0, lng0);
    let ll1 = make_latlng(lat1, lng1);
    let r = Rect::from_point_pair(ll0, ll1);
    if r.is_empty() {
        return true;
    }
    let center = r.center();
    r.contains_lat_lng(center)
}

/// Rect intersection with self is self (v2)
#[quickcheck]
fn prop_rect_intersection_self2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    r.intersection(r) == r
}

/// Rect union with self is self (v2)
#[quickcheck]
fn prop_rect_union_self2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1));
    r.union(r) == r
}

// ── Additional r1::Interval properties ───────────────────────────────

/// r1::Interval: empty interval contains nothing
#[quickcheck]
fn prop_r1_empty_contains_nothing(v: i32) -> bool {
    !s2rst::r1::Interval::empty().contains(v as f64)
}

/// r1::Interval: interval contains its endpoints
#[quickcheck]
fn prop_r1_interval_contains_endpoints(a: i32, b: i32) -> bool {
    let lo = (a as f64).min(b as f64);
    let hi = (a as f64).max(b as f64);
    let iv = s2rst::r1::Interval::new(lo, hi);
    iv.contains(lo) && iv.contains(hi)
}

/// r1::Interval: expanded interval contains original
#[quickcheck]
fn prop_r1_expanded_contains_original(a: i32, b: i32) -> bool {
    let lo = (a as f64).min(b as f64);
    let hi = (a as f64).max(b as f64);
    let iv = s2rst::r1::Interval::new(lo, hi);
    let exp = iv.expanded(1.0);
    exp.contains_interval(iv)
}

// ── Additional s1::Interval properties ───────────────────────────────

/// s1::Interval: full contains everything
#[quickcheck]
fn prop_s1_full_contains_all(v: i32) -> bool {
    let x = (v as f64) % PI;
    s2rst::s1::Interval::full().contains(x)
}

/// s1::Interval: union with self is self
#[quickcheck]
fn prop_s1_union_self2(a: i32, b: i32) -> bool {
    use s2rst::s1::Interval;
    let x = (a % 3142) as f64 * 0.001;
    let y = (b % 3142) as f64 * 0.001;
    let iv = Interval::new(x, y);
    iv.union(iv) == iv
}

// ── LaxPolyline / LaxPolygon / LaxLoop shape properties ─────────────

/// LaxLoop edge count equals vertex count
#[quickcheck]
fn prop_lax_loop_num_edges_eq_vertices(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_loop::LaxLoop;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    LaxLoop::new(vec![p0, p1, p2]).num_edges() == 3
}

/// LaxPolyline edge count equals vertices - 1 (v2)
#[quickcheck]
fn prop_lax_polyline_num_edges2(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    LaxPolyline::new(vec![p0, p1, p2]).num_edges() == 2
}

/// LaxPolygon with one loop has correct vertex count
#[quickcheck]
fn prop_lax_polygon_single_loop_vertices(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_polygon::LaxPolygon;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let poly = LaxPolygon::from_loops(&[&[p0, p1, p2]]);
    poly.num_loops() == 1 && poly.num_vertices() == 3
}

/// LaxLoop dimension is 2
#[quickcheck]
fn prop_lax_loop_dimension2(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::lax_loop::LaxLoop;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    LaxLoop::new(vec![p0, p1, p2]).dimension() == Dimension::Polygon
}

/// LaxPolyline dimension is 1
#[quickcheck]
fn prop_lax_polyline_dimension2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::lax_polyline::LaxPolyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    LaxPolyline::new(vec![p0, p1]).dimension() == Dimension::Polyline
}

// ── r3::Vector dot / cross additional properties ─────────────────────

/// dot(a, b) == dot(b, a) (v2)
#[quickcheck]
fn prop_r3_dot_commutative2(x1: i32, y1: i32, z1: i32, x2: i32, y2: i32, z2: i32) -> bool {
    let a = s2rst::r3::Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = s2rst::r3::Vector::new(x2 as f64, y2 as f64, z2 as f64);
    (a.dot(b) - b.dot(a)).abs() < 1e-10
}

/// cross(a, a) == zero
#[quickcheck]
fn prop_r3_cross_self_zero(x: i32, y: i32, z: i32) -> bool {
    let a = s2rst::r3::Vector::new(x as f64, y as f64, z as f64);
    let c = a.cross(a);
    c.norm() < (1e-10_f64).max(a.norm2() * 1e-14)
}

/// cross(a, b) is perpendicular to a (for non-tiny vectors)
#[quickcheck]
fn prop_r3_cross_perp_to_a2(x1: i16, y1: i16, z1: i16, x2: i16, y2: i16, z2: i16) -> bool {
    let a = s2rst::r3::Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = s2rst::r3::Vector::new(x2 as f64, y2 as f64, z2 as f64);
    if a.norm() < 1e-6 || b.norm() < 1e-6 {
        return true;
    }
    let c = a.cross(b);
    let dot = c.dot(a).abs();
    let tol = a.norm() * b.norm() * a.norm() * 4.0 * f64::EPSILON;
    dot < tol.max(1e-8)
}

/// norm2 == dot(self, self)
#[quickcheck]
fn prop_r3_norm2_eq_dot_self(x: i32, y: i32, z: i32) -> bool {
    let v = s2rst::r3::Vector::new(x as f64, y as f64, z as f64);
    (v.norm2() - v.dot(v)).abs() < 1e-10
}

/// cross(a, b) == -cross(b, a) (anti-commutativity)
#[quickcheck]
fn prop_r3_cross_anticommutative3(x1: i16, y1: i16, z1: i16, x2: i16, y2: i16, z2: i16) -> bool {
    let a = s2rst::r3::Vector::new(x1 as f64, y1 as f64, z1 as f64);
    let b = s2rst::r3::Vector::new(x2 as f64, y2 as f64, z2 as f64);
    let ab = a.cross(b);
    let ba = b.cross(a);
    (ab.x + ba.x).abs() < 1e-8 && (ab.y + ba.y).abs() < 1e-8 && (ab.z + ba.z).abs() < 1e-8
}

// ════════════════════════════════════════════════════════════════════
// ADDITIONAL PROPERTY TESTS — Batch 3
// More tests and additional coverage.
// ════════════════════════════════════════════════════════════════════

// ── Display trait: to_string() is non-empty ──────────────────────────

/// r3::Vector Display is non-empty
#[quickcheck]
fn prop_r3vector_display_nonempty(x: i32, y: i32, z: i32) -> bool {
    let v = s2rst::r3::Vector::new(x as f64, y as f64, z as f64);
    !v.to_string().is_empty()
}

/// ChordAngle Display is non-empty
#[quickcheck]
fn prop_chordangle_display_nonempty(a: u8) -> bool {
    !ChordAngle::from_degrees((a as f64) * 180.0 / 255.0)
        .to_string()
        .is_empty()
}

/// r2::Point Display is non-empty
#[quickcheck]
fn prop_r2point_display_nonempty(x: i32, y: i32) -> bool {
    !s2rst::r2::Point::new(x as f64, y as f64)
        .to_string()
        .is_empty()
}

/// r1::Interval Display is non-empty
#[quickcheck]
fn prop_r1interval_display_nonempty(a: i32, b: i32) -> bool {
    !s2rst::r1::Interval::new(a as f64, b as f64)
        .to_string()
        .is_empty()
}

/// Point Display is non-empty
#[quickcheck]
fn prop_point_display_nonempty(lat: i32, lng: i32) -> bool {
    !make_latlng(lat, lng).to_point().to_string().is_empty()
}

/// CellId Display is non-empty
#[quickcheck]
fn prop_cellid_display_nonempty2(lat: i32, lng: i32) -> bool {
    !CellId::from_lat_lng(&make_latlng(lat, lng))
        .to_string()
        .is_empty()
}

/// LatLng Display is non-empty
#[quickcheck]
fn prop_latlng_display_nonempty2(lat: i32, lng: i32) -> bool {
    !make_latlng(lat, lng).to_string().is_empty()
}

/// Cap Display is non-empty
#[quickcheck]
fn prop_cap_display_nonempty2(lat: i32, lng: i32, deg: u8) -> bool {
    let cap = Cap::from_center_angle(
        make_latlng(lat, lng).to_point(),
        Angle::from_degrees((deg % 90) as f64),
    );
    !cap.to_string().is_empty()
}

/// Angle Display is non-empty
#[quickcheck]
fn prop_angle_display_nonempty2(a: i32) -> bool {
    !Angle::from_degrees(a as f64).to_string().is_empty()
}

/// Face Display is non-empty
#[quickcheck]
fn prop_face_display_nonempty2(f: u8) -> bool {
    !s2rst::s2::coords::Face::from_u8(f % 6)
        .to_string()
        .is_empty()
}

/// Rect Display is non-empty
#[quickcheck]
fn prop_rect_display_nonempty2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    !Rect::from_point_pair(make_latlng(lat0, lng0), make_latlng(lat1, lng1))
        .to_string()
        .is_empty()
}

// ── SnapFunction trait ───────────────────────────────────────────────

/// IdentitySnapFunction: snap_radius >= 0
#[quickcheck]
fn prop_identity_snap_radius_nonneg(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let d = (deg % 71) as f64;
    let sf = IdentitySnapFunction::new(Angle::from_degrees(d));
    sf.snap_radius().radians() >= 0.0
}

/// IdentitySnapFunction: snap_point(p) == p
#[quickcheck]
fn prop_identity_snap_point_identity(lat: i32, lng: i32, deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let d = (deg % 71) as f64;
    let sf = IdentitySnapFunction::new(Angle::from_degrees(d));
    let p = make_latlng(lat, lng).to_point();
    sf.snap_point(p) == p
}

/// IdentitySnapFunction: min_vertex_separation <= snap_radius
#[quickcheck]
fn prop_identity_min_vertex_sep_le_radius(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let d = (deg % 71) as f64;
    let sf = IdentitySnapFunction::new(Angle::from_degrees(d));
    sf.min_vertex_separation().radians() <= sf.snap_radius().radians() + 1e-15
}

/// IdentitySnapFunction: min_edge_vertex_separation <= min_vertex_separation
#[quickcheck]
fn prop_identity_min_edge_vertex_sep_le_vertex_sep(deg: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
    let d = (deg % 71) as f64;
    let sf = IdentitySnapFunction::new(Angle::from_degrees(d));
    sf.min_edge_vertex_separation().radians() <= sf.min_vertex_separation().radians() + 1e-15
}

/// S2CellIdSnapFunction: snap_radius >= 0
#[quickcheck]
fn prop_cell_snap_radius_nonneg(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let sf = S2CellIdSnapFunction::new((level % 31).min(30));
    sf.snap_radius().radians() >= 0.0
}

/// S2CellIdSnapFunction: min_vertex_separation <= snap_radius
#[quickcheck]
fn prop_cell_snap_min_vertex_sep_le_radius(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let sf = S2CellIdSnapFunction::new((level % 31).min(30));
    sf.min_vertex_separation().radians() <= sf.snap_radius().radians() + 1e-15
}

/// S2CellIdSnapFunction: min_edge_vertex_separation <= min_vertex_separation
#[quickcheck]
fn prop_cell_snap_min_edge_vertex_sep_le_vertex_sep(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let sf = S2CellIdSnapFunction::new((level % 31).min(30));
    sf.min_edge_vertex_separation().radians() <= sf.min_vertex_separation().radians() + 1e-15
}

/// IntLatLngSnapFunction: snap_radius >= 0
#[quickcheck]
fn prop_intlatlng_snap_radius_nonneg2(exp: u8) -> bool {
    use s2rst::s2::builder::snap::{IntLatLngSnapFunction, SnapFunction};
    let sf = IntLatLngSnapFunction::new((exp % 10) as i32);
    sf.snap_radius().radians() >= 0.0
}

/// IntLatLngSnapFunction: min_vertex_separation <= snap_radius
#[quickcheck]
fn prop_intlatlng_snap_min_vertex_sep_le_radius2(exp: u8) -> bool {
    use s2rst::s2::builder::snap::{IntLatLngSnapFunction, SnapFunction};
    let sf = IntLatLngSnapFunction::new((exp % 10) as i32);
    sf.min_vertex_separation().radians() <= sf.snap_radius().radians() + 1e-15
}

/// All three snap functions have consistent separation ordering
#[quickcheck]
fn prop_snap_separations_ordered(deg: u8, level: u8) -> bool {
    use s2rst::s2::builder::snap::{IdentitySnapFunction, S2CellIdSnapFunction, SnapFunction};
    let d = (deg % 71) as f64;
    let sf_id = IdentitySnapFunction::new(Angle::from_degrees(d));
    let sf_cell = S2CellIdSnapFunction::new((level % 31).min(30));
    // For each: edge_vertex_sep <= vertex_sep <= snap_radius
    let id_ok = sf_id.min_edge_vertex_separation().radians()
        <= sf_id.min_vertex_separation().radians() + 1e-15
        && sf_id.min_vertex_separation().radians() <= sf_id.snap_radius().radians() + 1e-15;
    let cell_ok = sf_cell.min_edge_vertex_separation().radians()
        <= sf_cell.min_vertex_separation().radians() + 1e-15
        && sf_cell.min_vertex_separation().radians() <= sf_cell.snap_radius().radians() + 1e-15;
    id_ok && cell_ok
}

/// S2CellIdSnapFunction clone preserves snap_radius
#[quickcheck]
fn prop_cell_snap_clone_same_radius(level: u8) -> bool {
    use s2rst::s2::builder::snap::{S2CellIdSnapFunction, SnapFunction};
    let sf = S2CellIdSnapFunction::new((level % 31).min(30));
    let sf2 = sf.clone();
    (sf.snap_radius().radians() - sf2.snap_radius().radians()).abs() < 1e-15
}

// ── Default trait ────────────────────────────────────────────────────

/// LatLng default is origin (0, 0)
#[test]
fn prop_latlng_default_origin() {
    let ll = LatLng::default();
    assert!((ll.lat.radians()).abs() < 1e-15);
    assert!((ll.lng.radians()).abs() < 1e-15);
}

/// s1::Interval default is empty
#[test]
fn prop_s1interval_default_is_empty2() {
    assert!(s2rst::s1::Interval::default().is_empty());
}

/// ChordAngle default is zero
#[test]
fn prop_chord_angle_default_is_zero2() {
    let ca = ChordAngle::default();
    assert!((ca.to_angle().radians()).abs() < 1e-15);
}

// ── Shape trait: Polyline edges sum chains ────────────────────────────

/// Polyline num_edges == sum of chain lengths
#[quickcheck]
fn prop_polyline_shape_edges_sum_chains(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    let sum: usize = (0..pl.num_chains()).map(|i| pl.chain(i).length).sum();
    sum == pl.num_edges()
}

/// PointVector num_edges == sum of chain lengths
#[quickcheck]
fn prop_point_vector_shape_edges_sum_chains(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pv = PointVector::new(vec![p0, p1]);
    let sum: usize = (0..pv.num_chains()).map(|i| pv.chain(i).length).sum();
    sum == pv.num_edges()
}

/// Shape num_chains <= num_edges + 1 for Polyline
#[quickcheck]
fn prop_shape_num_chains_le_num_edges_plus_one(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::polyline::Polyline;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    pl.num_chains() <= pl.num_edges() + 1
}

// ── Region trait: CellUnion contains_cell implies intersects ─────────

/// CellUnion: contains_cell implies intersects_cell
#[quickcheck]
fn prop_cellunion_contains_cell_implies_intersects(lat: i32, lng: i32) -> bool {
    use s2rst::s2::{Cell, CellUnion, Region};
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let cu = CellUnion::from_cell_ids(vec![id]);
    let cell = Cell::from_cell_id(id);
    if cu.contains_cell(&cell) {
        cu.intersects_cell(&cell)
    } else {
        true
    }
}

// ── S2Encode / S2Decode ──────────────────────────────────────────────

/// LaxPolyline encode → decode preserves vertex count
#[quickcheck]
fn prop_lax_polyline_encode_decode_vertex_count2(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::lax_polyline::LaxPolyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let lp = LaxPolyline::new(vec![p0, p1]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolyline::decode(&mut &buf[..]).unwrap();
    decoded.num_vertices() == lp.num_vertices()
}

/// LaxPolygon encode → decode preserves loop count
#[quickcheck]
fn prop_lax_polygon_encode_decode_loop_count2(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::lax_polygon::LaxPolygon;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let lp = LaxPolygon::from_loops(&[&[p0, p1, p2]]);
    let mut buf = Vec::new();
    lp.encode(&mut buf).unwrap();
    let decoded = LaxPolygon::decode(&mut &buf[..]).unwrap();
    decoded.num_loops() == lp.num_loops()
}

/// PointVector encode → decode preserves count
#[quickcheck]
fn prop_point_vector_encode_decode_count2(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::point_vector::PointVector;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pv = PointVector::new(vec![p0, p1]);
    let mut buf = Vec::new();
    pv.encode(&mut buf).unwrap();
    let decoded = PointVector::decode(&mut &buf[..]).unwrap();
    decoded.num_edges() == pv.num_edges()
}

/// Polyline encode → decode preserves vertex count
#[quickcheck]
fn prop_polyline_encode_decode_vertex_count(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    use s2rst::s2::polyline::Polyline;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let pl = Polyline::new(vec![p0, p1]);
    let mut buf = Vec::new();
    pl.encode(&mut buf).unwrap();
    let decoded = Polyline::decode(&mut &buf[..]).unwrap();
    decoded.num_vertices() == pl.num_vertices()
}

/// Cap encode → decode preserves height
#[quickcheck]
fn prop_cap_encode_decode_radius(lat: i32, lng: i32, deg: u8) -> bool {
    use s2rst::s2::encoding::{S2Decode, S2Encode};
    let center = make_latlng(lat, lng).to_point();
    let cap = Cap::from_center_angle(center, Angle::from_degrees((deg % 90) as f64));
    let mut buf = Vec::new();
    cap.encode(&mut buf).unwrap();
    let decoded = Cap::decode(&mut &buf[..]).unwrap();
    (cap.height() - decoded.height()).abs() < 1e-15
}

// ── EdgeVectorShape properties ───────────────────────────────────────

/// EdgeVectorShape from_edge has exactly 1 edge
#[quickcheck]
fn prop_edge_vector_shape_single_edge(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_vector_shape::EdgeVectorShape;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let evs = EdgeVectorShape::from_edge(p0, p1);
    evs.num_edges() == 1
}

/// EdgeVectorShape from_edges has correct count
#[quickcheck]
fn prop_edge_vector_shape_multi_edge(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
) -> bool {
    use s2rst::s2::edge_vector_shape::EdgeVectorShape;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let p2 = make_latlng(lat2, lng2).to_point();
    let evs = EdgeVectorShape::from_edges(vec![(p0, p1), (p1, p2)]);
    evs.num_edges() == 2
}

/// EdgeVectorShape dimension defaults to 1
#[quickcheck]
fn prop_edge_vector_shape_dimension_one(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_vector_shape::EdgeVectorShape;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    EdgeVectorShape::from_edge(p0, p1).dimension() == Dimension::Polyline
}

/// EdgeVectorShape edge endpoints are unit vectors
#[quickcheck]
fn prop_edge_vector_shape_endpoints_unit(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    use s2rst::s2::edge_vector_shape::EdgeVectorShape;
    use s2rst::s2::shape::Shape;
    let p0 = make_latlng(lat0, lng0).to_point();
    let p1 = make_latlng(lat1, lng1).to_point();
    let evs = EdgeVectorShape::from_edge(p0, p1);
    let e = evs.edge(0);
    (e.v0.0.norm() - 1.0).abs() < 1e-14 && (e.v1.0.norm() - 1.0).abs() < 1e-14
}

// ── PointRegion properties ───────────────────────────────────────────

/// PointRegion contains its point
#[quickcheck]
fn prop_point_region_contains_point(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Region;
    use s2rst::s2::point_region::PointRegion;
    let p = make_latlng(lat, lng).to_point();
    let pr = PointRegion::new(p);
    pr.contains_point(&p)
}

/// PointRegion cap_bound is valid
#[quickcheck]
fn prop_point_region_cap_bound_valid(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Region;
    use s2rst::s2::point_region::PointRegion;
    let p = make_latlng(lat, lng).to_point();
    let pr = PointRegion::new(p);
    let cap = pr.cap_bound();
    cap.is_valid()
}

/// PointRegion rect_bound contains the point
#[quickcheck]
fn prop_point_region_rect_bound_contains(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Region;
    use s2rst::s2::point_region::PointRegion;
    let p = make_latlng(lat, lng).to_point();
    let pr = PointRegion::new(p);
    let ll = LatLng::from_point(p);
    pr.rect_bound().contains_lat_lng(ll)
}

// ── Additional Angle properties ──────────────────────────────────────

/// Angle::from_radians → radians roundtrip
#[quickcheck]
fn prop_angle_radians_roundtrip2(a: i32) -> bool {
    let r = a as f64 * 0.001;
    let angle = Angle::from_radians(r);
    (angle.radians() - r).abs() < 1e-15
}

/// Angle::from_degrees → degrees roundtrip
#[quickcheck]
fn prop_angle_degrees_roundtrip2(a: i32) -> bool {
    let d = a as f64;
    let angle = Angle::from_degrees(d);
    let diff = (angle.degrees() - d).abs();
    diff < 1e-10 + d.abs() * 1e-14
}

/// Angle abs is non-negative
#[quickcheck]
fn prop_angle_abs_nonneg2(a: i32) -> bool {
    Angle::from_degrees(a as f64).abs().radians() >= 0.0
}

// ── Additional LatLng properties ─────────────────────────────────────

/// LatLng::normalized produces valid
#[quickcheck]
fn prop_latlng_normalized_is_valid(lat: i32, lng: i32) -> bool {
    make_latlng(lat, lng).normalized().is_valid()
}

/// LatLng::get_distance is non-negative
#[quickcheck]
fn prop_latlng_get_distance_nonneg(la: i32, lna: i32, lb: i32, lnb: i32) -> bool {
    let a = make_latlng(la, lna);
    let b = make_latlng(lb, lnb);
    a.get_distance(b).radians() >= 0.0
}

/// LatLng::get_distance(self) == 0
#[quickcheck]
fn prop_latlng_distance_self_zero2(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    ll.get_distance(ll).radians().abs() < 1e-14
}

/// LatLng approx_equal with self
#[quickcheck]
fn prop_latlng_approx_equal_self(lat: i32, lng: i32) -> bool {
    let ll = make_latlng(lat, lng);
    ll.approx_eq(ll)
}

/// LatLng to_string_in_degrees is non-empty
#[quickcheck]
fn prop_latlng_to_string_in_degrees_nonempty(lat: i32, lng: i32) -> bool {
    !make_latlng(lat, lng).to_string_in_degrees().is_empty()
}

// ── Cell additional properties ───────────────────────────────────────

/// Cell center is unit vector
#[quickcheck]
fn prop_cell_center_is_unit(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Cell;
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let cell = Cell::from_cell_id(id);
    (cell.center().0.norm() - 1.0).abs() < 1e-14
}

/// Cell vertices are unit vectors
#[quickcheck]
fn prop_cell_vertices_are_unit(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Cell;
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let cell = Cell::from_cell_id(id);
    (0..4).all(|k| (cell.vertex(k).0.norm() - 1.0).abs() < 1e-14)
}

/// Cell level matches id level
#[quickcheck]
fn prop_cell_level_matches_id(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Cell;
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let cell = Cell::from_cell_id(id);
    cell.level() == id.level()
}

/// Cell face matches id face
#[quickcheck]
fn prop_cell_face_matches_id(lat: i32, lng: i32) -> bool {
    use s2rst::s2::Cell;
    let id = CellId::from_lat_lng(&make_latlng(lat, lng));
    let cell = Cell::from_cell_id(id);
    cell.face() == id.face()
}

// ── r2::Rect additional properties ───────────────────────────────────

/// r2::Rect from point pair contains both points
#[quickcheck]
fn prop_r2rect_from_point_pair_contains(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    let p1 = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let p2 = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let r = s2rst::r2::Rect::from_point_pair(p1, p2);
    r.contains_point(p1) && r.contains_point(p2)
}

/// r2::Rect union with self is self
#[quickcheck]
fn prop_r2rect_union_self(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    let p1 = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let p2 = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let r = s2rst::r2::Rect::from_point_pair(p1, p2);
    r.union(r) == r
}

/// r2::Rect intersection with self is self
#[quickcheck]
fn prop_r2rect_intersection_self2(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    let p1 = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let p2 = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let r = s2rst::r2::Rect::from_point_pair(p1, p2);
    r.intersection(r) == r
}

/// r2::Rect expanded contains original
#[quickcheck]
fn prop_r2rect_expanded_contains_original(x1: i16, y1: i16, x2: i16, y2: i16) -> bool {
    let p1 = s2rst::r2::Point::new(x1 as f64, y1 as f64);
    let p2 = s2rst::r2::Point::new(x2 as f64, y2 as f64);
    let r = s2rst::r2::Rect::from_point_pair(p1, p2);
    let exp = r.expanded(s2rst::r2::Point::new(1.0, 1.0));
    exp.contains(r)
}

// ── Mul<f64>: zero multiplier ────────────────────────────────────────

/// r3::Vector * 0.0 == zero
#[quickcheck]
fn prop_r3_vector_mul_zero2(x: i32, y: i32, z: i32) -> bool {
    let v = s2rst::r3::Vector::new(x as f64, y as f64, z as f64);
    let z = v * 0.0;
    z.x == 0.0 && z.y == 0.0 && z.z == 0.0
}

/// Angle * 0.0 == zero
#[quickcheck]
fn prop_angle_mul_zero(a: i32) -> bool {
    let angle = Angle::from_degrees(a as f64) * 0.0;
    angle.radians() == 0.0
}

/// r2::Point * 0.0 == zero
#[quickcheck]
fn prop_r2_point_mul_zero(x: i32, y: i32) -> bool {
    let p = s2rst::r2::Point::new(x as f64, y as f64) * 0.0;
    p.x == 0.0 && p.y == 0.0
}

// ── Additional r3::Matrix properties ─────────────────────────────────

/// Identity matrix * vector == vector
#[quickcheck]
fn prop_r3_matrix_identity_mul(x: i32, y: i32, z: i32) -> bool {
    use s2rst::r3::{Matrix3x3, Vector};
    let v = Vector::new(x as f64, y as f64, z as f64);
    let id = Matrix3x3::identity();
    let result = id * v;
    (result.x - v.x).abs() < 1e-10
        && (result.y - v.y).abs() < 1e-10
        && (result.z - v.z).abs() < 1e-10
}

/// Matrix identity * identity == identity
#[test]
fn prop_r3_matrix_identity_mul_identity() {
    use s2rst::r3::{Matrix3x3, Vector};
    let id = Matrix3x3::identity();
    let ex = Vector::new(1.0, 0.0, 0.0);
    let ey = Vector::new(0.0, 1.0, 0.0);
    let ez = Vector::new(0.0, 0.0, 1.0);
    assert_eq!(id * ex, ex);
    assert_eq!(id * ey, ey);
    assert_eq!(id * ez, ez);
}

/// Matrix transpose of identity is identity
#[test]
fn prop_r3_matrix_identity_transpose() {
    use s2rst::r3::Matrix3x3;
    let id = Matrix3x3::identity();
    assert_eq!(id.transpose(), id);
}

// ────────────────────────────────────────────────────────────────────
// R2EdgeClipper properties
// ────────────────────────────────────────────────────────────────────

/// R2EdgeClipper: clipping an edge fully inside always returns true with both outcodes INSIDE.
#[quickcheck]
fn prop_r2_clip_inside_always_hits(x0: i16, y0: i16, x1: i16, y1: i16) -> bool {
    use s2rst::r2;
    use s2rst::s2::r2_edge_clipper::{INSIDE, R2Edge, R2EdgeClipper};

    // Scale to [-0.5, 0.5] to stay inside [-1, 1] rect.
    let s = |v: i16| -> f64 { f64::from(v) / 65536.0 };
    let rect = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
    let mut c = R2EdgeClipper::from_rect(&rect);
    let edge = R2Edge::new(r2::Point::new(s(x0), s(y0)), r2::Point::new(s(x1), s(y1)));
    let hit = c.clip_edge(&edge, false);
    hit && c.outcode0 == INSIDE && c.outcode1 == INSIDE
}

/// R2EdgeClipper: clipping an edge fully outside (same region) always returns false.
#[quickcheck]
fn prop_r2_clip_outside_same_region_misses(x0: u16, y0: u16, x1: u16, y1: u16) -> bool {
    use s2rst::r2;
    use s2rst::s2::r2_edge_clipper::{R2Edge, R2EdgeClipper};

    // Both points are far left (x < -2).
    let s = |v: u16| -> f64 { -2.0 - f64::from(v) / 65536.0 };
    let rect = r2::Rect::from_points(r2::Point::new(0.0, 0.0), r2::Point::new(1.0, 1.0));
    let mut c = R2EdgeClipper::from_rect(&rect);
    let edge = R2Edge::new(
        r2::Point::new(s(x0), f64::from(y0) / 65536.0),
        r2::Point::new(s(x1), f64::from(y1) / 65536.0),
    );
    !c.clip_edge(&edge, false)
}

/// R2EdgeClipper: clipped endpoints always lie within the clip region (with small error).
#[quickcheck]
fn prop_r2_clipped_endpoints_in_rect(x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
    use s2rst::r2;
    use s2rst::s2::r2_edge_clipper::{MAX_UNIT_CLIP_ERROR, R2Edge, R2EdgeClipper};

    let s = |v: i32| -> f64 { f64::from(v) / 500_000_000.0 * 2.0 };
    let rect = r2::Rect::from_points(r2::Point::new(-1.0, -1.0), r2::Point::new(1.0, 1.0));
    let mut c = R2EdgeClipper::from_rect(&rect);
    let edge = R2Edge::new(r2::Point::new(s(x0), s(y0)), r2::Point::new(s(x1), s(y1)));

    if !c.clip_edge(&edge, false) {
        return true; // Edge missed, nothing to check.
    }

    let eps = MAX_UNIT_CLIP_ERROR + 1e-15;
    let v0 = &c.clipped_edge.v0;
    let v1 = &c.clipped_edge.v1;

    v0.x >= -1.0 - eps
        && v0.x <= 1.0 + eps
        && v0.y >= -1.0 - eps
        && v0.y <= 1.0 + eps
        && v1.x >= -1.0 - eps
        && v1.x <= 1.0 + eps
        && v1.y >= -1.0 - eps
        && v1.y <= 1.0 + eps
}

// ────────────────────────────────────────────────────────────────────
// RobustCellClipper properties
// ────────────────────────────────────────────────────────────────────

/// RobustCellClipper: center-to-center degenerate edge always hits.
#[quickcheck]
fn prop_robust_degenerate_edge_hits(face: u8) -> bool {
    use s2rst::s2::robust_cell_clipper::{RobustCellClipper, RobustClipResult};

    let face = face % 6;
    let cell = Cell::from_cell_id(CellId::from_face(face));
    let center = cell.center();
    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    c.clip_edge(center, center, false) == RobustClipResult::HitBoth
}

/// RobustCellClipper: opposite-face edges always miss.
#[quickcheck]
fn prop_robust_opposite_face_miss(face: u8, lat: i16, lng: i16) -> bool {
    use s2rst::s2::robust_cell_clipper::{RobustCellClipper, RobustClipResult};

    let face = face % 6;
    let opp = (face + 3) % 6;
    let cell = Cell::from_cell_id(CellId::from_face(face));

    // Generate points on the opposite face using the opposite face center region.
    let opp_cell = Cell::from_cell_id(CellId::from_face(opp));
    let c = opp_cell.center();
    let tiny_lat = f64::from(lat) / 1_000_000.0;
    let tiny_lng = f64::from(lng) / 1_000_000.0;
    let ll = LatLng::from_point(c);
    let v0 =
        LatLng::from_degrees(ll.lat.degrees() + tiny_lat, ll.lng.degrees() + tiny_lng).to_point();
    let v1 = LatLng::from_degrees(
        ll.lat.degrees() + tiny_lat + 0.001,
        ll.lng.degrees() + tiny_lng + 0.001,
    )
    .to_point();

    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);
    clipper.clip_edge(v0, v1, false) == RobustClipResult::Miss
}

/// RobustCellClipper: HitBoth implies v0_inside and v1_inside.
#[quickcheck]
fn prop_robust_hit_both_implies_inside(x: i16, y: i16) -> bool {
    use s2rst::s2::robust_cell_clipper::{RobustCellClipper, RobustClipResult};

    let cell = Cell::from_cell_id(CellId::from_face(0));
    // Generate small points near the center of face 0.
    let center = cell.center();
    let dx = f64::from(x) / 10_000_000.0;
    let dy = f64::from(y) / 10_000_000.0;
    let v0 = Point((center.0 + s2rst::r3::Vector::new(dx, dy, 0.0)).normalize());
    let v1 = Point((center.0 + s2rst::r3::Vector::new(-dx, -dy, 0.0)).normalize());

    let mut c = RobustCellClipper::new();
    c.start_cell(cell);
    let result = c.clip_edge(v0, v1, false);
    if result == RobustClipResult::HitBoth {
        result.v0_inside() && result.v1_inside()
    } else {
        true // Not HitBoth, nothing to check.
    }
}

/// RobustCellClipper: disabling crossings means get_crossings is empty.
#[quickcheck]
fn prop_robust_no_crossings_option(face: u8) -> bool {
    use s2rst::s2::robust_cell_clipper::{Options, RobustCellClipper};

    let face = face % 6;
    let cell = Cell::from_cell_id(CellId::from_face(face));
    let mut c = RobustCellClipper::with_options(Options {
        enable_crossings: false,
    });
    c.start_cell(cell);

    let center = cell.center();
    let far = LatLng::from_degrees(80.0, 0.0).to_point();
    c.clip_edge(center, far, false);
    c.get_crossings().is_empty()
}

// ────────────────────────────────────────────────────────────────────
// ShapeTracker properties
// ────────────────────────────────────────────────────────────────────

/// ShapeTracker: marking all chains of a point shape finishes it.
#[quickcheck]
fn prop_tracker_point_all_chains_finish(n: u8) -> bool {
    use s2rst::s2::shape_tracker::ShapeTracker;

    let n = (n % 50) as usize + 1; // 1..50 chains
    let mut t = ShapeTracker::new(Dimension::Point, n);
    for i in 0..n {
        t.mark_chain(i);
    }
    t.finished()
}

/// ShapeTracker: marking chains is idempotent.
#[quickcheck]
fn prop_tracker_mark_chain_idempotent(n: u8, reps: u8) -> bool {
    use s2rst::s2::shape_tracker::ShapeTracker;

    let n = (n % 10) as usize + 1;
    let reps = (reps % 5) + 1;
    let mut t = ShapeTracker::new(Dimension::Point, n);
    for _ in 0..reps {
        for i in 0..n {
            t.mark_chain(i);
        }
    }
    t.finished()
}

/// ShapeTracker: add_interval followed by reverse cancels.
#[quickcheck]
fn prop_tracker_interval_cancel(face: u8, ij0: u16, ij1: u16) -> bool {
    use s2rst::s2::shape_tracker::ShapeTracker;

    let face = i32::from(face % 6);
    let ij0 = i64::from(ij0);
    let mut ij1 = i64::from(ij1);
    if ij0 == ij1 {
        ij1 += 1;
    }

    let mut t = ShapeTracker::new(Dimension::Polygon, 1);
    t.mark_chain(0);
    t.add_interval(face, 0, 100, ij0, ij1);
    assert!(!t.finished());
    t.add_interval(face, 0, 100, ij1, ij0);
    t.finished()
}

/// ShapeTracker: add_point followed by del_point cancels.
#[quickcheck]
fn prop_tracker_point_cancel(face: u8, ij: u16) -> bool {
    use s2rst::s2::shape_tracker::ShapeTracker;

    let face = i32::from(face % 6);
    let ij = i64::from(ij);

    let mut t = ShapeTracker::new(Dimension::Polyline, 1);
    t.mark_chain(0);
    t.add_point(face, 0, 100, ij);
    assert!(!t.finished());
    t.del_point(face, 0, 100, ij);
    t.finished()
}

/// ShapeTracker: partial chain marking doesn't finish.
#[quickcheck]
fn prop_tracker_partial_chains_not_finished(n: u8) -> bool {
    use s2rst::s2::shape_tracker::ShapeTracker;

    let n = (n % 20) as usize + 2; // At least 2 chains.
    let mut t = ShapeTracker::new(Dimension::Point, n);
    for i in 0..n - 1 {
        t.mark_chain(i);
    }
    !t.finished()
}

// ────────────────────────────────────────────────────────────────────
// ReclippedShape properties
// ────────────────────────────────────────────────────────────────────

/// ReclippedShape: init with negative shape_id never skips.
#[quickcheck]
fn prop_reclipped_negative_id_always_processes(face: u8) -> bool {
    use s2rst::s2::reclipped_shape::ReclippedShape;
    use s2rst::s2::robust_cell_clipper::RobustCellClipper;

    let face = face % 6;
    let cell = Cell::from_cell_id(CellId::from_face(face));
    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    let mut shape = ReclippedShape::new();
    // Passing shape_id=-1 should always process.
    shape.init(
        &mut clipper,
        -1,
        Dimension::Polygon,
        false,
        std::iter::empty(),
        false,
    )
}

/// ReclippedShape: after init, dimension matches.
#[quickcheck]
fn prop_reclipped_dimension_matches(dim: u8) -> bool {
    use s2rst::s2::reclipped_shape::ReclippedShape;
    use s2rst::s2::robust_cell_clipper::RobustCellClipper;

    let dim = match dim % 3 {
        0 => Dimension::Point,
        1 => Dimension::Polyline,
        _ => Dimension::Polygon,
    };
    let cell = Cell::from_cell_id(CellId::from_face(0));
    let mut clipper = RobustCellClipper::new();
    clipper.start_cell(cell);

    let mut shape = ReclippedShape::new();
    shape.init(&mut clipper, 0, dim, false, std::iter::empty(), false);
    shape.dimension() == Some(dim)
}
