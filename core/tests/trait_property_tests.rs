// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Type-parameterized property-based tests for the core `s2rst` traits.
//!
//! For every public trait in the library (`Region`, `Shape`, `S2Encode`,
//! `S2Decode`, `Projection`, `SnapFunction`, the `distance_target` /
//! `closest_edge_query` target traits), this file defines:
//!
//!  1. **Generic property functions** that encode the trait's *invariants*.
//!     They take a reference to any value implementing the trait plus
//!     some inputs and return a `bool`.
//!
//!  2. **Composable data generators** — plain functions that build
//!     concrete instances from small primitive seeds. They can be
//!     composed for composite types (e.g., `RegionIntersection` combines
//!     two arbitrary `Box<dyn Region>` values).
//!
//!  3. **`#[quickcheck]` wire-ups** that instantiate each generic
//!     property for each concrete implementing type. Adding a new
//!     concrete type only requires defining its generator and satisfying
//!     the trait — the invariant suite comes for free.
//!
//! The invariants themselves are documented in `CORE_TRAITS.md`.

#![allow(
    clippy::doc_markdown,
    reason = "test doc comments use unquoted API names"
)]
#![allow(
    clippy::missing_panics_doc,
    reason = "integration tests — panics document themselves"
)]
#![allow(
    trivial_casts,
    reason = "quickcheck_macros emits `fn as fn` casts for test registration"
)]
#![allow(
    unreachable_pub,
    reason = "internal helper modules re-expose items only to sibling code"
)]
#![allow(
    dead_code,
    reason = "generic generators and property functions are exercised by wire-ups below; a few extras are kept for completeness"
)]

use quickcheck_macros::quickcheck;

use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::builder::snap::{
    IdentitySnapFunction, IntLatLngSnapFunction, S2CellIdSnapFunction, SnapFunction,
};
use s2rst::s2::coords::Level;
use s2rst::s2::edge_vector_shape::EdgeVectorShape;
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::lax_loop::LaxLoop;
use s2rst::s2::lax_polygon::LaxPolygon;
use s2rst::s2::lax_polyline::LaxPolyline;
use s2rst::s2::point_region::PointRegion;
use s2rst::s2::point_vector::PointVector;
use s2rst::s2::polyline::Polyline;
use s2rst::s2::projections::{MercatorProjection, PlateCarreeProjection, Projection};
use s2rst::s2::region_intersection::RegionIntersection;
use s2rst::s2::region_union::RegionUnion;
use s2rst::s2::shape::{Dimension, Shape};
use s2rst::s2::{Cap, Cell, CellId, CellUnion, LatLng, Loop, Point, Polygon, Rect, Region};

// ════════════════════════════════════════════════════════════════════
// Section 1. Primitive helpers and safe `LatLng`/`Point` construction
// ════════════════════════════════════════════════════════════════════

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() <= eps
}

/// Integer-seeded lat/lng in `[-90, 90] × [-180, 180]`, avoiding exact poles/dateline.
fn make_latlng(lat_i: i32, lng_i: i32) -> LatLng {
    let lat = (lat_i.rem_euclid(179)) as f64 - 89.0;
    let lng = (lng_i.rem_euclid(359)) as f64 - 179.0;
    LatLng::from_degrees(lat, lng)
}

/// A latitude/longitude that avoids poles and dateline for maximum numerical stability.
fn make_latlng_safe(lat_i: i32, lng_i: i32) -> LatLng {
    let lat = ((lat_i.wrapping_mul(37)).rem_euclid(121)) as f64 - 60.0;
    let lng = ((lng_i.wrapping_mul(53)).rem_euclid(321)) as f64 - 160.0;
    LatLng::from_degrees(lat, lng)
}

fn make_point(lat_i: i32, lng_i: i32) -> Point {
    make_latlng(lat_i, lng_i).to_point()
}

fn make_safe_point(lat_i: i32, lng_i: i32) -> Point {
    make_latlng_safe(lat_i, lng_i).to_point()
}

fn point_is_unit(p: Point) -> bool {
    let n2 = p.0.x * p.0.x + p.0.y * p.0.y + p.0.z * p.0.z;
    (n2 - 1.0).abs() < 1e-9
}

// ════════════════════════════════════════════════════════════════════
// Section 2. Composable data generators
// ════════════════════════════════════════════════════════════════════
//
// Each generator function returns either `T` or `Option<T>` depending on
// whether degenerate inputs are possible. Composite generators take
// already-generated sub-values and combine them.

mod generators {
    use super::{
        Angle, Cap, Cell, CellId, CellUnion, EdgeVectorShape, IdentitySnapFunction,
        IntLatLngSnapFunction, LatLng, LaxLoop, LaxPolygon, LaxPolyline, Level, Loop,
        MercatorProjection, PlateCarreeProjection, Point, PointRegion, PointVector, Polygon,
        Polyline, Rect, Region, RegionIntersection, RegionUnion, S2CellIdSnapFunction, make_latlng,
        make_latlng_safe, make_point, make_safe_point,
    };

    // ---- Core primitives --------------------------------------------------

    pub fn point(lat_i: i32, lng_i: i32) -> Point {
        make_point(lat_i, lng_i)
    }

    pub fn safe_point(lat_i: i32, lng_i: i32) -> Point {
        make_safe_point(lat_i, lng_i)
    }

    pub fn latlng(lat_i: i32, lng_i: i32) -> LatLng {
        make_latlng(lat_i, lng_i)
    }

    pub fn level(l: u8) -> Level {
        Level::new(l % 31)
    }

    pub fn cell_id(lat_i: i32, lng_i: i32, lvl: u8) -> CellId {
        CellId::from_lat_lng(&make_latlng(lat_i, lng_i)).parent_at_level(level(lvl))
    }

    pub fn cell(lat_i: i32, lng_i: i32, lvl: u8) -> Cell {
        Cell::from_cell_id(cell_id(lat_i, lng_i, lvl))
    }

    // ---- Region generators ------------------------------------------------

    pub fn cap(lat_i: i32, lng_i: i32, radius_i: u16) -> Cap {
        // Radius in [0, 180] degrees.
        let radius = Angle::from_degrees((radius_i % 18001) as f64 / 100.0);
        Cap::from_center_angle(safe_point(lat_i, lng_i), radius)
    }

    /// A cap from arbitrary `f64`s, not whole degrees (spec 008 FR-011): a latitude, a longitude
    /// (degrees) and a radius (radians), each folded into range, so that inputs like spec 008's
    /// K5 cap (longitude 1e-14°, radius 1e-18 rad) are reachable. Non-finite inputs become 0.
    pub fn cap_f64(lat: f64, lng: f64, r: f64) -> Cap {
        let fin = |x: f64| if x.is_finite() { x } else { 0.0 };
        let (lat, lng, r) = (fin(lat), fin(lng), fin(r));
        let lat = if lat.abs() <= 90.0 { lat } else { lat % 90.0 };
        let lng = if lng.abs() <= 180.0 { lng } else { lng % 180.0 };
        let r = if r.abs() <= std::f64::consts::PI {
            r.abs()
        } else {
            r.abs() % std::f64::consts::PI
        };
        Cap::from_center_angle(
            LatLng::from_degrees(lat, lng).to_point(),
            Angle::from_radians(r),
        )
    }

    /// Small cap (≤ 10°) for numerical stability in sensitive tests.
    pub fn small_cap(lat_i: i32, lng_i: i32, radius_i: u8) -> Cap {
        let radius = Angle::from_degrees((radius_i % 100) as f64 / 10.0);
        Cap::from_center_angle(safe_point(lat_i, lng_i), radius)
    }

    pub fn rect(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> Rect {
        Rect::from_lat_lng(make_latlng_safe(lat0, lng0)).add_point(make_latlng_safe(lat1, lng1))
    }

    /// A rectangle from arbitrary `f64`s (spec 010 FR-011): two latitudes and two longitudes
    /// (radians), folded into range, and a family that moves the latitudes next to a pole
    /// (within 10⁻ᵏ of it), onto the north pole, or onto one latitude, so that spec 010's K4
    /// inputs (`UPSTREAM.md` U-016) are reachable. Non-finite inputs become 0.
    pub fn rect_f64(lat0: f64, lat1: f64, lng0: f64, lng1: f64, family: u8) -> Rect {
        use std::f64::consts::{FRAC_PI_2, PI};
        let fin = |x: f64, m: f64| if x.is_finite() { x % m } else { 0.0 };
        let (a, c) = (fin(lat0, FRAC_PI_2), fin(lat1, FRAC_PI_2));
        let (lo, hi) = (a.min(c), a.max(c));
        let k = i32::from(family / 6 % 17);
        let (lo, hi) = match family % 6 {
            1 => (
                FRAC_PI_2 - (a.abs() / FRAC_PI_2) * 10f64.powi(-k),
                FRAC_PI_2,
            ),
            2 => (
                -FRAC_PI_2,
                -FRAC_PI_2 + (a.abs() / FRAC_PI_2) * 10f64.powi(-k),
            ),
            3 => (FRAC_PI_2, FRAC_PI_2),
            4 => (lo, lo),
            _ => (lo, hi),
        };
        Rect::new(
            s2rst::r1::Interval::new(lo, hi),
            s2rst::s1::Interval::new(fin(lng0, PI), fin(lng1, PI)),
        )
    }

    /// A point on or next to `r`'s boundary: a corner, a point on either latitude bound or
    /// either longitude bound at the fraction `t`, or such a point moved `k` ulps inward
    /// (spec 010 research R0's points).
    pub fn boundary_point(r: Rect, which: u8, t: f64, k: u8) -> Point {
        use std::f64::consts::PI;
        let t = if t.is_finite() { t.abs().fract() } else { 0.5 };
        let step = |x: f64, up: bool| {
            (0..k % 17).fold(x, |y, _| if up { y.next_up() } else { y.next_down() })
        };
        let lng = if r.lng.is_inverted() {
            let v = r.lng.lo + t * r.lng.length();
            if v > PI { v - 2.0 * PI } else { v }
        } else {
            r.lng.lo + t * (r.lng.hi - r.lng.lo)
        };
        let lat = r.lat.lo + t * (r.lat.hi - r.lat.lo);
        let (la, ln) = match which % 8 {
            0 => (r.lat.lo, r.lng.lo),
            1 => (r.lat.hi, r.lng.hi),
            2 => (r.lat.lo, lng),
            3 => (r.lat.hi, lng),
            4 => (step(r.lat.lo, true), step(r.lng.lo, true)),
            5 => (step(r.lat.hi, false), step(r.lng.hi, false)),
            6 => (r.lat.lo, step(r.lng.hi, false)),
            _ => (lat, step(r.lng.lo, true)),
        };
        LatLng::from_radians(la, ln).to_point()
    }

    pub fn point_region(lat_i: i32, lng_i: i32) -> PointRegion {
        PointRegion::new(safe_point(lat_i, lng_i))
    }

    pub fn cell_union(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
        lvl: u8,
    ) -> CellUnion {
        let l = level(lvl.saturating_add(5)); // Bias away from face-level cells.
        let ids = vec![
            cell_id(lat0, lng0, l.as_u8()),
            cell_id(lat1, lng1, l.as_u8()),
            cell_id(lat2, lng2, l.as_u8()),
        ];
        let mut u = CellUnion::from_cell_ids(ids);
        u.normalize();
        u
    }

    /// CCW triangle loop (skips degenerate/near-degenerate triangles).
    pub fn triangle_loop(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<Loop> {
        let p0 = make_latlng_safe(lat0, lng0).to_point();
        let p1 = make_latlng_safe(lat1, lng1).to_point();
        let p2 = make_latlng_safe(lat2, lng2).to_point();
        let cross = p0.0.cross(p1.0).dot(p2.0);
        if cross.abs() < 1e-4 {
            return None;
        }
        let lp = Loop::new(vec![p0, p1, p2]);
        if lp.num_vertices() < 3 {
            return None;
        }
        Some(lp)
    }

    pub fn triangle_polygon(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<Polygon> {
        triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2).map(|l| Polygon::from_loops(vec![l]))
    }

    pub fn polyline_three(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<Polyline> {
        let p0 = make_latlng_safe(lat0, lng0).to_point();
        let p1 = make_latlng_safe(lat1, lng1).to_point();
        let p2 = make_latlng_safe(lat2, lng2).to_point();
        if p0.distance(p1).radians() < 1e-6 || p1.distance(p2).radians() < 1e-6 {
            return None;
        }
        Some(Polyline::new(vec![p0, p1, p2]))
    }

    pub fn lax_polyline_three(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<LaxPolyline> {
        let p0 = make_latlng_safe(lat0, lng0).to_point();
        let p1 = make_latlng_safe(lat1, lng1).to_point();
        let p2 = make_latlng_safe(lat2, lng2).to_point();
        if p0.distance(p1).radians() < 1e-6 || p1.distance(p2).radians() < 1e-6 {
            return None;
        }
        Some(LaxPolyline::new(vec![p0, p1, p2]))
    }

    pub fn lax_loop_triangle(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<LaxLoop> {
        let lp = triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2)?;
        let verts: Vec<Point> = (0..lp.num_vertices()).map(|i| lp.vertex(i)).collect();
        Some(LaxLoop::new(verts))
    }

    pub fn lax_polygon_triangle(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<LaxPolygon> {
        let lp = triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2)?;
        let verts: Vec<Point> = (0..lp.num_vertices()).map(|i| lp.vertex(i)).collect();
        Some(LaxPolygon::from_loops_owned(vec![verts]))
    }

    pub fn point_vector(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> PointVector {
        PointVector::new(vec![safe_point(lat0, lng0), safe_point(lat1, lng1)])
    }

    pub fn edge_vector_shape(
        lat0: i32,
        lng0: i32,
        lat1: i32,
        lng1: i32,
        lat2: i32,
        lng2: i32,
    ) -> Option<EdgeVectorShape> {
        let p0 = make_latlng_safe(lat0, lng0).to_point();
        let p1 = make_latlng_safe(lat1, lng1).to_point();
        let p2 = make_latlng_safe(lat2, lng2).to_point();
        if p0.distance(p1).radians() < 1e-6 || p0.distance(p2).radians() < 1e-6 {
            return None;
        }
        Some(EdgeVectorShape::from_edges(vec![(p0, p1), (p0, p2)]))
    }

    // ---- Composable region combinators ------------------------------------

    pub fn region_intersection(a: Box<dyn Region>, b: Box<dyn Region>) -> RegionIntersection {
        RegionIntersection::from_regions(vec![a, b])
    }

    pub fn region_union(a: Box<dyn Region>, b: Box<dyn Region>) -> RegionUnion {
        RegionUnion::from_regions(vec![a, b])
    }

    // ---- Projection / snap-function generators ----------------------------

    pub fn plate_carree() -> PlateCarreeProjection {
        PlateCarreeProjection::new(180.0)
    }

    pub fn mercator() -> MercatorProjection {
        MercatorProjection::new(180.0)
    }

    pub fn identity_snap(snap_deg_tenths: u8) -> IdentitySnapFunction {
        // 0.0 .. 6.9 degrees (well below the 70° max).
        let r = Angle::from_degrees((snap_deg_tenths % 70) as f64 / 10.0);
        IdentitySnapFunction::new(r)
    }

    pub fn cellid_snap(lvl: u8) -> S2CellIdSnapFunction {
        // S2CellIdSnapFunction works at levels 0..=30.
        S2CellIdSnapFunction::new(level(lvl))
    }

    pub fn intlatlng_snap(exponent: u8) -> IntLatLngSnapFunction {
        // Valid exponents are 0..=10.
        IntLatLngSnapFunction::new((exponent % 11) as i32)
    }
}

// ════════════════════════════════════════════════════════════════════
// Section 3. Generic property functions (the trait invariants)
// ════════════════════════════════════════════════════════════════════
//
// Each function is a generic `<T: Trait>` predicate. The wire-up in
// section 4 instantiates these for every concrete implementation.

mod prop {
    pub use s2rst::s2::distance_target::DistanceTarget;

    use super::{
        Angle, Cap, Cell, CellId, ChordAngle, Dimension, LatLng, Level, Point, Projection, Region,
        S2Decode, S2Encode, Shape, SnapFunction, approx_eq, point_is_unit,
    };
    use s2rst::s2::CellVertex;

    // ---- Region invariants -----------------------------------------------

    /// If the region contains a point, that point lies within the bounding cap.
    pub fn region_cap_bound_contains_point<R: Region>(r: &R, p: Point) -> bool {
        if r.contains_point(&p) {
            r.cap_bound().contains_point(p)
        } else {
            true
        }
    }

    /// If the region contains a point, that point lies within the bounding rect.
    pub fn region_rect_bound_contains_point<R: Region>(r: &R, p: Point) -> bool {
        if r.contains_point(&p) {
            r.rect_bound().contains_lat_lng(LatLng::from_point(p))
        } else {
            true
        }
    }

    /// `contains_cell` implies `intersects_cell`.
    pub fn region_contains_implies_intersects<R: Region>(r: &R, c: &Cell) -> bool {
        if r.contains_cell(c) {
            r.intersects_cell(c)
        } else {
            true
        }
    }

    /// `intersects_cell(c) == false` must be consistent with the cell's
    /// center not being contained. (Conservative regions may return true
    /// for a non-intersecting cell, but they must not claim
    /// non-intersection while containing one of its points.)
    pub fn region_non_intersect_implies_no_contained_center<R: Region>(r: &R, c: &Cell) -> bool {
        if r.intersects_cell(c) {
            true
        } else {
            !r.contains_point(&c.center())
        }
    }

    /// A cell the region does not intersect has no vertex the region contains. The Kani twin is
    /// `verify/src/laws.rs` (`laws::region::no_intersect_no_vertex`, spec 008 P-CPREG-02).
    pub fn region_no_intersect_no_vertex<R: Region>(r: &R, c: &Cell) -> bool {
        r.intersects_cell(c) || (0..4).all(|k| !r.contains_point(&c.vertex(CellVertex::ALL[k])))
    }

    /// `cell_union_bound()` covers the region's bounding cap center if non-empty.
    pub fn region_cell_union_bound_non_empty<R: Region>(r: &R) -> bool {
        let bound = r.cell_union_bound();
        // Either the cover is non-empty or the region is trivially empty.
        !bound.is_empty() || r.cap_bound().is_empty()
    }

    /// Every cell the region contains is covered by its covering bound: some id of
    /// `cell_union_bound()` contains the cell's id. The Kani twin is
    /// `verify/src/laws.rs` (`laws::region::covering_covers`, spec 004 P-CCNT-03).
    pub fn region_cell_union_bound_covers<R: Region>(r: &R, c: &Cell) -> bool {
        !r.contains_cell(c) || r.cell_union_bound().iter().any(|id| id.contains(c.id()))
    }

    /// Querying twice is deterministic.
    pub fn region_deterministic<R: Region>(r: &R, p: Point) -> bool {
        let a = r.contains_point(&p);
        let b = r.contains_point(&p);
        let c = r.cap_bound();
        let d = r.cap_bound();
        a == b && c == d
    }

    // ---- Shape invariants -----------------------------------------------

    /// Sum of chain lengths equals the total edge count.
    pub fn shape_chain_lengths_sum_to_num_edges<S: Shape>(s: &S) -> bool {
        let n: usize = (0..s.num_chains()).map(|i| s.chain(i).length).sum();
        n == s.num_edges()
    }

    /// For every edge id, `chain_position → chain_edge` is the original edge.
    pub fn shape_chain_position_roundtrip<S: Shape>(s: &S) -> bool {
        let n = s.num_edges();
        // Cap to avoid pathological shapes in quickcheck.
        let n = n.min(64);
        for id in 0..n {
            let pos = s.chain_position(id);
            let edge_via_chain = s.chain_edge(pos.chain_id, pos.offset);
            let edge_direct = s.edge(id);
            if edge_via_chain != edge_direct {
                return false;
            }
        }
        true
    }

    /// `is_empty()` and `num_edges() == 0` agree for dim-1+.
    pub fn shape_empty_consistency<S: Shape>(s: &S) -> bool {
        if s.num_edges() == 0 && s.num_chains() == 0 {
            s.is_empty()
        } else if !s.is_empty() && s.num_edges() == 0 {
            // Dim-2 shapes may have zero edges but represent the full sphere via a
            // chain.
            s.dimension() == Dimension::Polygon
        } else {
            true
        }
    }

    /// `has_interior() == (dimension() == Polygon)`.
    pub fn shape_has_interior_matches_dim<S: Shape>(s: &S) -> bool {
        s.has_interior() == (s.dimension() == Dimension::Polygon)
    }

    /// `is_full() ⇒ dim == Polygon && !is_empty()`.
    pub fn shape_full_implies_polygon<S: Shape>(s: &S) -> bool {
        if s.is_full() {
            s.dimension() == Dimension::Polygon && !s.is_empty()
        } else {
            true
        }
    }

    /// Reference point lies on the unit sphere (when the shape has any
    /// geometry).
    pub fn shape_reference_point_is_unit<S: Shape>(s: &S) -> bool {
        if s.num_edges() == 0 && s.num_chains() == 0 {
            return true;
        }
        let rp = s.reference_point();
        point_is_unit(rp.point)
    }

    // ---- S2Encode / S2Decode invariants ---------------------------------

    /// `encode` produces deterministic byte output.
    pub fn encode_deterministic<T: S2Encode>(v: &T) -> bool {
        let mut a = Vec::new();
        v.encode(&mut a).expect("encode a");
        let mut b = Vec::new();
        v.encode(&mut b).expect("encode b");
        a == b
    }

    /// `encode` produces non-empty byte output.
    pub fn encode_non_empty<T: S2Encode>(v: &T) -> bool {
        let mut buf = Vec::new();
        v.encode(&mut buf).expect("encode");
        !buf.is_empty()
    }

    /// Strict round-trip: decoded value equals original.
    pub fn encode_decode_roundtrip_eq<T: S2Encode + S2Decode + PartialEq>(v: &T) -> bool {
        let mut buf = Vec::new();
        v.encode(&mut buf).expect("encode");
        let decoded = T::decode(&mut &buf[..]).expect("decode");
        &decoded == v
    }

    /// Approximate round-trip via a user-supplied comparator.
    pub fn encode_decode_roundtrip_approx<T, F>(v: &T, equiv: F) -> bool
    where
        T: S2Encode + S2Decode,
        F: Fn(&T, &T) -> bool,
    {
        let mut buf = Vec::new();
        v.encode(&mut buf).expect("encode");
        let decoded = T::decode(&mut &buf[..]).expect("decode");
        equiv(v, &decoded)
    }

    // ---- Projection invariants ------------------------------------------

    /// `unproject(project(p))` is approximately `p`.
    pub fn projection_project_unproject_roundtrip<P: Projection>(proj: &P, p: Point) -> bool {
        let projected = proj.project(p);
        if !projected.x.is_finite() || !projected.y.is_finite() {
            return true;
        }
        let back = proj.unproject(projected);
        let d = p.distance(back).radians();
        d < 1e-9
    }

    /// `interpolate(0, a, b) ≈ a`.
    pub fn projection_interpolate_zero_is_start<P: Projection>(
        proj: &P,
        a: Point,
        b: Point,
    ) -> bool {
        let pa = proj.project(a);
        let pb = proj.project(b);
        let mid = proj.interpolate(0.0, pa, pb);
        approx_eq(mid.x, pa.x, 1e-9) && approx_eq(mid.y, pa.y, 1e-9)
    }

    /// `interpolate(1, a, b) ≈ b`.
    pub fn projection_interpolate_one_is_end<P: Projection>(proj: &P, a: Point, b: Point) -> bool {
        let pa = proj.project(a);
        let pb = proj.project(b);
        let mid = proj.interpolate(1.0, pa, pb);
        // For wrapping projections, interpolate might wrap b relative to a,
        // but the projected "destination" should still lie within one wrap.
        let wrap = proj.wrap_distance();
        let dx = (mid.x - pb.x).abs();
        let dy = (mid.y - pb.y).abs();
        let x_ok = dx < 1e-9 || (wrap.x > 0.0 && approx_eq(dx, wrap.x, 1e-9));
        let y_ok = dy < 1e-9 || (wrap.y > 0.0 && approx_eq(dy, wrap.y, 1e-9));
        x_ok && y_ok
    }

    /// `wrap_distance()` components are non-negative.
    pub fn projection_wrap_distance_non_negative<P: Projection>(proj: &P) -> bool {
        let w = proj.wrap_distance();
        w.x >= 0.0 && w.y >= 0.0
    }

    // ---- SnapFunction invariants ----------------------------------------

    /// The distance from `p` to `snap_point(p)` is ≤ `snap_radius()`.
    pub fn snap_respects_radius<S: SnapFunction>(s: &S, p: Point) -> bool {
        let snapped = s.snap_point(p);
        let radius = s.snap_radius().radians();
        // Add a tiny epsilon for numerical slack.
        p.distance(snapped).radians() <= radius + 1e-12
    }

    /// The snapped point is still a unit-length vector.
    pub fn snap_output_is_unit<S: SnapFunction>(s: &S, p: Point) -> bool {
        point_is_unit(s.snap_point(p))
    }

    /// Snapping an already-snapped point is a no-op (idempotence).
    pub fn snap_is_idempotent<S: SnapFunction>(s: &S, p: Point) -> bool {
        let a = s.snap_point(p);
        let b = s.snap_point(a);
        // Idempotence should hold exactly in theory, but allow a small tolerance
        // to account for repeated floating-point round-off through LatLng
        // conversions.
        a.distance(b).radians() < 1e-13
    }

    /// Separation bounds are consistent: `min_edge_vertex ≤ min_vertex ≤ 2*radius`.
    pub fn snap_separation_bounds_consistent<S: SnapFunction>(s: &S) -> bool {
        let r = s.snap_radius().radians();
        let mv = s.min_vertex_separation().radians();
        let mev = s.min_edge_vertex_separation().radians();
        mv >= 0.0 && mev >= 0.0 && mv <= 2.0 * r + 1e-12 && mev <= mv + 1e-12
    }

    /// A cloned snap function preserves radius, separations, and `snap_point`.
    pub fn snap_clone_fidelity<S: SnapFunction>(s: &S, p: Point) -> bool {
        let cloned = s.clone_snap();
        s.snap_radius() == cloned.snap_radius()
            && s.min_vertex_separation() == cloned.min_vertex_separation()
            && s.min_edge_vertex_separation() == cloned.min_edge_vertex_separation()
            && s.snap_point(p) == cloned.snap_point(p)
    }

    // ---- DistanceTarget invariants --------------------------------------

    /// `cap_bound()` is deterministic (repeat calls produce identical caps).
    pub fn distance_target_cap_bound_deterministic<T>(t: &T) -> bool
    where
        T: DistanceTarget,
    {
        let a = t.cap_bound();
        let b = t.cap_bound();
        a == b
    }

    /// `set_max_error` with a small non-negative value does not panic and
    /// leaves `cap_bound()` observable (the return bool is impl-defined).
    pub fn distance_target_set_max_error_safe<T>(mut t: T) -> bool
    where
        T: DistanceTarget,
    {
        let _ = t.set_max_error(ChordAngle::from_radians(1e-6));
        let _ = t.cap_bound();
        true
    }

    /// Closest-edge-query Target: passing `dist_limit = ChordAngle::ZERO` to
    /// `update_distance_to_point` must never "update" (any point is at distance
    /// ≥ 0, so no distance strictly less than 0 exists).
    pub fn closest_target_zero_limit_no_update<T>(t: &T, p: Point) -> bool
    where
        T: s2rst::s2::closest_edge_query::Target,
    {
        let (_d, updated) = t.update_distance_to_point(p, ChordAngle::ZERO);
        !updated
    }

    /// Closest-edge-query Target: `update_distance_to_point(p, ∞)` always
    /// updates (there is some finite distance from any point to the target).
    pub fn closest_target_infinite_limit_updates<T>(t: &T, p: Point) -> bool
    where
        T: s2rst::s2::closest_edge_query::Target,
    {
        let (d, updated) = t.update_distance_to_point(p, ChordAngle::INFINITY);
        updated && d <= ChordAngle::STRAIGHT
    }

    /// Furthest-edge-query Target: `update_distance_to_point(p, ∞)` must
    /// never update (nothing is strictly greater than infinity).
    pub fn furthest_target_infinite_limit_no_update<T>(t: &T, p: Point) -> bool
    where
        T: s2rst::s2::furthest_edge_query::Target,
    {
        let (_d, updated) = t.update_distance_to_point(p, ChordAngle::INFINITY);
        !updated
    }

    // Sentinel: keep type imports in scope even if only used in bounds /
    // generics. The `dead_code` allow is blanket at the file level.
    fn _keepalive(_: Angle, _: Cap, _: Cell, _: CellId, _: Level) {}
}

// ════════════════════════════════════════════════════════════════════
// Section 4. Wire-ups: quickcheck tests for every (trait × concrete type)
// ════════════════════════════════════════════════════════════════════

// ─── 4a. Region ─────────────────────────────────────────────────────

#[quickcheck]
fn region_cap_cap_contains(c_lat: i32, c_lng: i32, r_i: u16, p_lat: i32, p_lng: i32) -> bool {
    let cap = generators::cap(c_lat, c_lng, r_i);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&cap, p)
}

#[quickcheck]
fn region_cap_rect_contains(c_lat: i32, c_lng: i32, r_i: u16, p_lat: i32, p_lng: i32) -> bool {
    let cap = generators::cap(c_lat, c_lng, r_i);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_rect_bound_contains_point(&cap, p)
}

#[quickcheck]
fn region_cap_contains_implies_intersects(
    c_lat: i32,
    c_lng: i32,
    r_i: u16,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let cap = generators::cap(c_lat, c_lng, r_i);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_contains_implies_intersects(&cap, &cell)
}

/// Spec 008 FR-011: the cap generators from arbitrary `f64`s.
#[quickcheck]
fn region_cap_f64_cap_contains(lat: f64, lng: f64, r: f64, p_lat: i32, p_lng: i32) -> bool {
    let cap = generators::cap_f64(lat, lng, r);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&cap, p)
}

#[quickcheck]
fn region_cap_f64_rect_contains(lat: f64, lng: f64, r: f64) -> bool {
    let cap = generators::cap_f64(lat, lng, r);
    // The centre and points near it, where a rounded bound shows first (K5).
    prop::region_rect_bound_contains_point(&cap, cap.center())
}

#[quickcheck]
fn region_cap_f64_contains_implies_intersects(
    lat: f64,
    lng: f64,
    r: f64,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let cap = generators::cap_f64(lat, lng, r);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_contains_implies_intersects(&cap, &cell)
}

#[quickcheck]
fn region_cap_no_intersect_no_vertex(
    c_lat: i32,
    c_lng: i32,
    r_i: u16,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let cap = generators::cap(c_lat, c_lng, r_i);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_no_intersect_no_vertex(&cap, &cell)
}

// No `Cell` instance of `region_no_intersect_no_vertex`: it is not a law for cells. A cell's
// `intersects_cell` compares ids, so two face cells sharing an edge do not intersect, while its
// closed region contains the neighbour's shared vertices (quickcheck: face 0 against the face
// at (45°, 0°)). Upstream's `S2Cell::MayIntersect` and `Contains(S2Point)` behave the same.

#[quickcheck]
fn region_rect_no_intersect_no_vertex(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let rect = generators::rect(lat0, lng0, lat1, lng1);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_no_intersect_no_vertex(&rect, &cell)
}

/// Spec 008 K5 (FR-020, FR-011): the bounding rectangle of a cap at longitude 1e-14° with radius
/// 1e-18 rad contains the cap's centre (it was `[0, 0]` before the fix).
#[test]
fn region_cap_rect_contains_k5() {
    let cap = generators::cap_f64(0.0, 1e-14, 1e-18);
    assert!(prop::region_rect_bound_contains_point(&cap, cap.center()));
}

#[quickcheck]
fn region_cap_deterministic(c_lat: i32, c_lng: i32, r_i: u16, p_lat: i32, p_lng: i32) -> bool {
    let cap = generators::cap(c_lat, c_lng, r_i);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_deterministic(&cap, p)
}

#[quickcheck]
fn region_rect_cap_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let r = generators::rect(lat0, lng0, lat1, lng1);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&r, p)
}

/// Spec 010 FR-011: the bounding cap contains the points on and next to a rectangle's boundary,
/// for rectangles near and at the poles, where upstream's `GetCapBound` misses some (U-016).
#[quickcheck]
fn region_rect_cap_contains_boundary(
    lats: (f64, f64),
    lngs: (f64, f64),
    family: u8,
    which: u8,
    t: f64,
    k: u8,
) -> bool {
    let r = generators::rect_f64(lats.0, lats.1, lngs.0, lngs.1, family);
    let p = generators::boundary_point(r, which, t, k);
    prop::region_cap_bound_contains_point(&r, p)
}

/// Spec 010 K4's fixed examples (`UPSTREAM.md` U-016): the north pole as a rectangle and a point
/// one ulp past it, and a point five ulps inside a vertex of a small rectangle; upstream's bound
/// misses both.
#[test]
fn region_rect_cap_contains_k4() {
    use s2rst::{r1, s1};
    use std::f64::consts::{FRAC_PI_2, PI};
    let pole = Rect::new(
        r1::Interval::new(FRAC_PI_2, FRAC_PI_2),
        s1::Interval::new(-PI, PI),
    );
    let p = LatLng::from_radians(1.570_796_326_794_896_8, 4.653_912_984_578_174_6e-1).to_point();
    assert!(pole.contains_point(p));
    assert!(prop::region_cap_bound_contains_point(&pole, p));
    let mid = Rect::new(
        r1::Interval::new(1.071_221_527_018_078_5, 1.071_237_668_898_467_3),
        s1::Interval::new(-8.905_155_217_671_079e-1, -8.905_155_217_089_906e-1),
    );
    let p = LatLng::from_radians(1.071_221_527_018_078_5, -8.905_155_217_089_911e-1).to_point();
    assert!(mid.contains_point(p));
    assert!(prop::region_cap_bound_contains_point(&mid, p));
}

#[quickcheck]
fn region_rect_contains_implies_intersects_f64(
    lats: (f64, f64),
    lngs: (f64, f64),
    family: u8,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let r = generators::rect_f64(lats.0, lats.1, lngs.0, lngs.1, family);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_contains_implies_intersects(&r, &cell)
}

#[quickcheck]
fn region_rect_no_intersect_no_vertex_f64(
    lats: (f64, f64),
    lngs: (f64, f64),
    family: u8,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
) -> bool {
    let r = generators::rect_f64(lats.0, lats.1, lngs.0, lngs.1, family);
    let cell = generators::cell(ce_lat, ce_lng, lvl);
    prop::region_no_intersect_no_vertex(&r, &cell)
}

#[quickcheck]
fn region_rect_rect_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let r = generators::rect(lat0, lng0, lat1, lng1);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_rect_bound_contains_point(&r, p)
}

#[quickcheck]
fn region_rect_deterministic(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let r = generators::rect(lat0, lng0, lat1, lng1);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_deterministic(&r, p)
}

#[quickcheck]
fn region_cell_cap_contains(lat: i32, lng: i32, lvl: u8, p_lat: i32, p_lng: i32) -> bool {
    let c = generators::cell(lat, lng, lvl);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&c, p)
}

#[quickcheck]
fn region_cell_contains_self_center(lat: i32, lng: i32, lvl: u8) -> bool {
    let c = generators::cell(lat, lng, lvl);
    c.contains_point(c.center())
}

/// The cell argument of the two integer `Region` laws below: either a descendant of the region
/// cell (same point, a level up to three deeper), so that containment occurs, or an unrelated
/// cell. Spec 004 adds these `Cell` instances (research R15; the Kani twins are P-CCNT-03).
fn law_cell_arg(lat: i32, lng: i32, lvl: u8, near: bool, d: u8, o_lat: i32, o_lng: i32) -> Cell {
    if near {
        generators::cell(lat, lng, lvl.saturating_add(d % 4))
    } else {
        generators::cell(o_lat, o_lng, d)
    }
}

#[quickcheck]
fn region_cell_contains_implies_intersects(
    lat: i32,
    lng: i32,
    lvl: u8,
    near: bool,
    d: u8,
    o_lat: i32,
    o_lng: i32,
) -> bool {
    let r = generators::cell(lat, lng, lvl);
    let c = law_cell_arg(lat, lng, lvl, near, d, o_lat, o_lng);
    prop::region_contains_implies_intersects(&r, &c)
}

#[quickcheck]
fn region_cell_union_bound_covers(
    lat: i32,
    lng: i32,
    lvl: u8,
    near: bool,
    d: u8,
    o_lat: i32,
    o_lng: i32,
) -> bool {
    let r = generators::cell(lat, lng, lvl);
    let c = law_cell_arg(lat, lng, lvl, near, d, o_lat, o_lng);
    prop::region_cell_union_bound_covers(&r, &c)
}

#[quickcheck]
fn region_cell_deterministic(lat: i32, lng: i32, lvl: u8, p_lat: i32, p_lng: i32) -> bool {
    let c = generators::cell(lat, lng, lvl);
    prop::region_deterministic(&c, generators::safe_point(p_lat, p_lng))
}

#[quickcheck]
fn region_point_region_cap_contains(pr_lat: i32, pr_lng: i32, q_lat: i32, q_lng: i32) -> bool {
    let pr = generators::point_region(pr_lat, pr_lng);
    let q = generators::safe_point(q_lat, q_lng);
    prop::region_cap_bound_contains_point(&pr, q)
}

#[quickcheck]
fn region_point_region_contains_self(lat: i32, lng: i32) -> bool {
    let pr = generators::point_region(lat, lng);
    pr.contains_point(&generators::safe_point(lat, lng))
}

#[quickcheck]
fn region_cell_union_cap_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    q_lat: i32,
    q_lng: i32,
) -> bool {
    let u = generators::cell_union(lat0, lng0, lat1, lng1, lat2, lng2, 7);
    let q = generators::safe_point(q_lat, q_lng);
    prop::region_cap_bound_contains_point(&u, q)
}

#[quickcheck]
fn region_cell_union_contains_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    ce_lat: i32,
    ce_lng: i32,
    lvl: u8,
    ce_lvl: u8,
) -> bool {
    let u = generators::cell_union(lat0, lng0, lat1, lng1, lat0 ^ 0x55, lng1 ^ 0x7f, lvl);
    let c = generators::cell(ce_lat, ce_lng, ce_lvl);
    prop::region_contains_implies_intersects(&u, &c)
}

#[quickcheck]
fn region_loop_cap_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    match generators::triangle_loop(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(l) => {
            let p = generators::safe_point(p_lat, p_lng);
            prop::region_cap_bound_contains_point(&l, p)
        }
        None => true,
    }
}

#[quickcheck]
fn region_polygon_cap_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    match generators::triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(pg) => {
            let p = generators::safe_point(p_lat, p_lng);
            prop::region_cap_bound_contains_point(&pg, p)
        }
        None => true,
    }
}

#[quickcheck]
fn region_polygon_contains_implies_intersects(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    ce_lat: i32,
    ce_lng: i32,
) -> bool {
    match generators::triangle_polygon(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(pg) => {
            let c = generators::cell(ce_lat, ce_lng, 8);
            prop::region_contains_implies_intersects(&pg, &c)
        }
        None => true,
    }
}

#[quickcheck]
fn region_polyline_cap_contains(
    lat0: i32,
    lng0: i32,
    lat1: i32,
    lng1: i32,
    lat2: i32,
    lng2: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    match generators::polyline_three(lat0, lng0, lat1, lng1, lat2, lng2) {
        Some(pl) => {
            let p = generators::safe_point(p_lat, p_lng);
            prop::region_cap_bound_contains_point(&pl, p)
        }
        None => true,
    }
}

// Composable region generators: intersection and union of two small caps.

#[quickcheck]
fn region_intersection_cap_contains(
    a_lat: i32,
    a_lng: i32,
    a_r: u16,
    b_lat: i32,
    b_lng: i32,
    b_r: u16,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let a: Box<dyn Region> = Box::new(generators::cap(a_lat, a_lng, a_r));
    let b: Box<dyn Region> = Box::new(generators::cap(b_lat, b_lng, b_r));
    let ri = generators::region_intersection(a, b);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&ri, p)
}

#[quickcheck]
fn region_intersection_deterministic(
    a_lat: i32,
    a_lng: i32,
    b_lat0: i32,
    b_lng0: i32,
    b_lat1: i32,
    b_lng1: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let a: Box<dyn Region> = Box::new(generators::cap(a_lat, a_lng, 500));
    let b: Box<dyn Region> = Box::new(generators::rect(b_lat0, b_lng0, b_lat1, b_lng1));
    let ri = generators::region_intersection(a, b);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_deterministic(&ri, p)
}

#[quickcheck]
fn region_union_cap_contains(
    a_lat: i32,
    a_lng: i32,
    a_r: u16,
    b_lat: i32,
    b_lng: i32,
    b_r: u16,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let a: Box<dyn Region> = Box::new(generators::cap(a_lat, a_lng, a_r));
    let b: Box<dyn Region> = Box::new(generators::cap(b_lat, b_lng, b_r));
    let ru = generators::region_union(a, b);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_cap_bound_contains_point(&ru, p)
}

#[quickcheck]
fn region_union_mixed_deterministic(
    a_lat0: i32,
    a_lng0: i32,
    a_lat1: i32,
    a_lng1: i32,
    b_lat: i32,
    b_lng: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let a: Box<dyn Region> = Box::new(generators::rect(a_lat0, a_lng0, a_lat1, a_lng1));
    let b: Box<dyn Region> = Box::new(generators::point_region(b_lat, b_lng));
    let ru = generators::region_union(a, b);
    let p = generators::safe_point(p_lat, p_lng);
    prop::region_deterministic(&ru, p)
}

// ─── 4b. Shape ──────────────────────────────────────────────────────

macro_rules! shape_tests {
    ($mod:ident, $gen:expr) => {
        mod $mod {
            use super::*;

            #[quickcheck]
            fn chain_lengths_sum(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_chain_lengths_sum_to_num_edges(&s),
                    None => true,
                }
            }

            #[quickcheck]
            fn chain_position_roundtrip(
                l0: i32,
                g0: i32,
                l1: i32,
                g1: i32,
                l2: i32,
                g2: i32,
            ) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_chain_position_roundtrip(&s),
                    None => true,
                }
            }

            #[quickcheck]
            fn has_interior_matches_dim(
                l0: i32,
                g0: i32,
                l1: i32,
                g1: i32,
                l2: i32,
                g2: i32,
            ) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_has_interior_matches_dim(&s),
                    None => true,
                }
            }

            #[quickcheck]
            fn full_implies_polygon(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_full_implies_polygon(&s),
                    None => true,
                }
            }

            #[quickcheck]
            fn reference_point_unit(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_reference_point_is_unit(&s),
                    None => true,
                }
            }

            #[quickcheck]
            fn empty_consistency(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
                match $gen(l0, g0, l1, g1, l2, g2) {
                    Some(s) => prop::shape_empty_consistency(&s),
                    None => true,
                }
            }
        }
    };
}

shape_tests!(shape_polyline, generators::polyline_three);
shape_tests!(shape_lax_polyline, generators::lax_polyline_three);
shape_tests!(shape_lax_loop, generators::lax_loop_triangle);
shape_tests!(shape_lax_polygon, generators::lax_polygon_triangle);
shape_tests!(shape_loop, generators::triangle_loop);
shape_tests!(shape_polygon, generators::triangle_polygon);
shape_tests!(shape_edge_vector, generators::edge_vector_shape);

// `PointVector` takes only four seeds (two points).
mod shape_point_vector {
    use super::*;

    #[quickcheck]
    fn chain_lengths_sum(l0: i32, g0: i32, l1: i32, g1: i32) -> bool {
        let pv = generators::point_vector(l0, g0, l1, g1);
        prop::shape_chain_lengths_sum_to_num_edges(&pv)
    }

    #[quickcheck]
    fn chain_position_roundtrip(l0: i32, g0: i32, l1: i32, g1: i32) -> bool {
        let pv = generators::point_vector(l0, g0, l1, g1);
        prop::shape_chain_position_roundtrip(&pv)
    }

    #[quickcheck]
    fn has_interior_matches_dim(l0: i32, g0: i32, l1: i32, g1: i32) -> bool {
        let pv = generators::point_vector(l0, g0, l1, g1);
        prop::shape_has_interior_matches_dim(&pv)
    }

    #[quickcheck]
    fn reference_point_unit(l0: i32, g0: i32, l1: i32, g1: i32) -> bool {
        let pv = generators::point_vector(l0, g0, l1, g1);
        prop::shape_reference_point_is_unit(&pv)
    }
}

// ─── 4c. S2Encode / S2Decode ────────────────────────────────────────

#[quickcheck]
fn encode_cellid_roundtrip(lat: i32, lng: i32, lvl: u8) -> bool {
    let id = generators::cell_id(lat, lng, lvl);
    prop::encode_decode_roundtrip_eq(&id) && prop::encode_deterministic(&id)
}

#[quickcheck]
fn encode_cellid_non_empty(lat: i32, lng: i32, lvl: u8) -> bool {
    let id = generators::cell_id(lat, lng, lvl);
    prop::encode_non_empty(&id)
}

#[quickcheck]
fn encode_point_roundtrip(lat: i32, lng: i32) -> bool {
    let p = generators::safe_point(lat, lng);
    prop::encode_decode_roundtrip_approx(&p, |a, b| a.distance(*b).radians() < 1e-15)
        && prop::encode_deterministic(&p)
}

#[quickcheck]
fn encode_cap_roundtrip(lat: i32, lng: i32, r_i: u16) -> bool {
    let cap = generators::cap(lat, lng, r_i);
    prop::encode_decode_roundtrip_approx(&cap, |a, b| {
        if a.is_empty() {
            b.is_empty()
        } else if a.is_full() {
            b.is_full()
        } else {
            a.center().distance(b.center()).radians() < 1e-15
                && (a.angle_radius().radians() - b.angle_radius().radians()).abs() < 1e-15
        }
    }) && prop::encode_deterministic(&cap)
}

#[quickcheck]
fn encode_rect_roundtrip(lat0: i32, lng0: i32, lat1: i32, lng1: i32) -> bool {
    let r = generators::rect(lat0, lng0, lat1, lng1);
    prop::encode_decode_roundtrip_eq(&r) && prop::encode_deterministic(&r)
}

#[quickcheck]
fn encode_cell_roundtrip(lat: i32, lng: i32, lvl: u8) -> bool {
    let c = generators::cell(lat, lng, lvl);
    prop::encode_decode_roundtrip_eq(&c) && prop::encode_non_empty(&c)
}

#[quickcheck]
fn encode_cell_union_roundtrip(
    l0: i32,
    g0: i32,
    l1: i32,
    g1: i32,
    l2: i32,
    g2: i32,
    lvl: u8,
) -> bool {
    let u = generators::cell_union(l0, g0, l1, g1, l2, g2, lvl);
    prop::encode_decode_roundtrip_eq(&u) && prop::encode_deterministic(&u)
}

#[quickcheck]
fn encode_polyline_roundtrip_via_length(
    l0: i32,
    g0: i32,
    l1: i32,
    g1: i32,
    l2: i32,
    g2: i32,
) -> bool {
    match generators::polyline_three(l0, g0, l1, g1, l2, g2) {
        Some(pl) => {
            prop::encode_decode_roundtrip_approx(&pl, |a, b| a.num_vertices() == b.num_vertices())
                && prop::encode_deterministic(&pl)
        }
        None => true,
    }
}

#[quickcheck]
fn encode_loop_roundtrip_via_area(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
    match generators::triangle_loop(l0, g0, l1, g1, l2, g2) {
        Some(lp) => {
            prop::encode_decode_roundtrip_approx(&lp, |a, b| (a.area() - b.area()).abs() < 1e-10)
        }
        None => true,
    }
}

#[quickcheck]
fn encode_polygon_roundtrip_via_area(l0: i32, g0: i32, l1: i32, g1: i32, l2: i32, g2: i32) -> bool {
    match generators::triangle_polygon(l0, g0, l1, g1, l2, g2) {
        Some(pg) => {
            prop::encode_decode_roundtrip_approx(&pg, |a, b| (a.area() - b.area()).abs() < 1e-10)
        }
        None => true,
    }
}

// ─── 4d. Projection ─────────────────────────────────────────────────

macro_rules! projection_tests {
    ($mod:ident, $gen:expr) => {
        mod $mod {
            use super::*;

            #[quickcheck]
            fn project_unproject_roundtrip(lat: i32, lng: i32) -> bool {
                let proj = $gen;
                let p = generators::safe_point(lat, lng);
                prop::projection_project_unproject_roundtrip(&proj, p)
            }

            #[quickcheck]
            fn interpolate_zero_is_start(a_lat: i32, a_lng: i32, b_lat: i32, b_lng: i32) -> bool {
                let proj = $gen;
                let a = generators::safe_point(a_lat, a_lng);
                let b = generators::safe_point(b_lat, b_lng);
                prop::projection_interpolate_zero_is_start(&proj, a, b)
            }

            #[quickcheck]
            fn interpolate_one_is_end(a_lat: i32, a_lng: i32, b_lat: i32, b_lng: i32) -> bool {
                let proj = $gen;
                let a = generators::safe_point(a_lat, a_lng);
                let b = generators::safe_point(b_lat, b_lng);
                prop::projection_interpolate_one_is_end(&proj, a, b)
            }

            #[test]
            fn wrap_distance_non_negative() {
                let proj = $gen;
                assert!(prop::projection_wrap_distance_non_negative(&proj));
            }
        }
    };
}

projection_tests!(projection_plate_carree, generators::plate_carree());
projection_tests!(projection_mercator, generators::mercator());

// ─── 4e. SnapFunction ───────────────────────────────────────────────

macro_rules! snap_tests {
    ($mod:ident, $gen:expr, $seed_ty:ty) => {
        mod $mod {
            use super::*;

            #[quickcheck]
            fn respects_radius(seed: $seed_ty, p_lat: i32, p_lng: i32) -> bool {
                let s = $gen(seed);
                let p = generators::safe_point(p_lat, p_lng);
                prop::snap_respects_radius(&s, p)
            }

            #[quickcheck]
            fn output_is_unit(seed: $seed_ty, p_lat: i32, p_lng: i32) -> bool {
                let s = $gen(seed);
                let p = generators::safe_point(p_lat, p_lng);
                prop::snap_output_is_unit(&s, p)
            }

            #[quickcheck]
            fn idempotent(seed: $seed_ty, p_lat: i32, p_lng: i32) -> bool {
                let s = $gen(seed);
                let p = generators::safe_point(p_lat, p_lng);
                prop::snap_is_idempotent(&s, p)
            }

            #[quickcheck]
            fn separations_consistent(seed: $seed_ty) -> bool {
                let s = $gen(seed);
                prop::snap_separation_bounds_consistent(&s)
            }

            #[quickcheck]
            fn clone_fidelity(seed: $seed_ty, p_lat: i32, p_lng: i32) -> bool {
                let s = $gen(seed);
                let p = generators::safe_point(p_lat, p_lng);
                prop::snap_clone_fidelity(&s, p)
            }
        }
    };
}

snap_tests!(snap_identity, generators::identity_snap, u8);
snap_tests!(snap_cellid, generators::cellid_snap, u8);
snap_tests!(snap_intlatlng, generators::intlatlng_snap, u8);

// ─── 4f. DistanceTarget + closest-edge-query Targets ────────────────

// We test the `distance_target::DistanceTarget` supertrait contract
// through the closest-edge-query targets (one per variant).

#[quickcheck]
fn dt_point_cap_deterministic(lat: i32, lng: i32) -> bool {
    let t = s2rst::s2::closest_edge_query::PointTarget::new(generators::safe_point(lat, lng));
    prop::distance_target_cap_bound_deterministic(&t)
}

#[quickcheck]
fn dt_edge_cap_deterministic(a_lat: i32, a_lng: i32, b_lat: i32, b_lng: i32) -> bool {
    let a = generators::safe_point(a_lat, a_lng);
    let b = generators::safe_point(b_lat, b_lng);
    if a.distance(b).radians() < 1e-6 {
        return true;
    }
    let t = s2rst::s2::closest_edge_query::EdgeTarget::new(a, b);
    prop::distance_target_cap_bound_deterministic(&t)
}

#[quickcheck]
fn dt_cell_cap_deterministic(lat: i32, lng: i32, lvl: u8) -> bool {
    let t = s2rst::s2::closest_edge_query::CellTarget::new(generators::cell(lat, lng, lvl));
    prop::distance_target_cap_bound_deterministic(&t)
}

#[quickcheck]
fn dt_point_set_max_error_safe(lat: i32, lng: i32) -> bool {
    let t = s2rst::s2::closest_edge_query::PointTarget::new(generators::safe_point(lat, lng));
    prop::distance_target_set_max_error_safe(t)
}

// Monotone-refinement contract for the closest-edge-query PointTarget.

#[quickcheck]
fn closest_point_target_zero_limit(t_lat: i32, t_lng: i32, p_lat: i32, p_lng: i32) -> bool {
    let t = s2rst::s2::closest_edge_query::PointTarget::new(generators::safe_point(t_lat, t_lng));
    let p = generators::safe_point(p_lat, p_lng);
    // Property: a strictly-less-than-zero update is impossible.
    prop::closest_target_zero_limit_no_update(&t, p)
}

#[quickcheck]
fn closest_point_target_infinite_limit(t_lat: i32, t_lng: i32, p_lat: i32, p_lng: i32) -> bool {
    let t = s2rst::s2::closest_edge_query::PointTarget::new(generators::safe_point(t_lat, t_lng));
    let p = generators::safe_point(p_lat, p_lng);
    prop::closest_target_infinite_limit_updates(&t, p)
}

#[quickcheck]
fn closest_edge_target_zero_limit(
    a_lat: i32,
    a_lng: i32,
    b_lat: i32,
    b_lng: i32,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let a = generators::safe_point(a_lat, a_lng);
    let b = generators::safe_point(b_lat, b_lng);
    if a.distance(b).radians() < 1e-6 {
        return true;
    }
    let t = s2rst::s2::closest_edge_query::EdgeTarget::new(a, b);
    let p = generators::safe_point(p_lat, p_lng);
    prop::closest_target_zero_limit_no_update(&t, p)
}

#[quickcheck]
fn closest_cell_target_infinite_limit(
    c_lat: i32,
    c_lng: i32,
    lvl: u8,
    p_lat: i32,
    p_lng: i32,
) -> bool {
    let t = s2rst::s2::closest_edge_query::CellTarget::new(generators::cell(c_lat, c_lng, lvl));
    let p = generators::safe_point(p_lat, p_lng);
    prop::closest_target_infinite_limit_updates(&t, p)
}

// Furthest-edge-query Target: reverse monotone contract.

#[quickcheck]
fn furthest_point_target_infinite_limit(t_lat: i32, t_lng: i32, p_lat: i32, p_lng: i32) -> bool {
    let t = s2rst::s2::furthest_edge_query::PointTarget::new(generators::safe_point(t_lat, t_lng));
    let p = generators::safe_point(p_lat, p_lng);
    prop::furthest_target_infinite_limit_no_update(&t, p)
}

#[quickcheck]
fn furthest_edge_target_cap_deterministic(a_lat: i32, a_lng: i32, b_lat: i32, b_lng: i32) -> bool {
    let a = generators::safe_point(a_lat, a_lng);
    let b = generators::safe_point(b_lat, b_lng);
    if a.distance(b).radians() < 1e-6 {
        return true;
    }
    let t = s2rst::s2::furthest_edge_query::EdgeTarget::new(a, b);
    prop::distance_target_cap_bound_deterministic(&t)
}
