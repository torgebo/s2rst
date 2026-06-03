// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Original Rust code (not ported from upstream S2): `geo-types` interop for this
// crate. Part of the S2 Rust port; licensed Apache-2.0. See LICENSE.

//! Interoperability with the [`geo-types`](https://docs.rs/geo-types) crate.
//!
//! This module is only present when the `geo-types` feature is enabled.
//! It provides [`From`] implementations in both directions between the
//! `geo-types` coordinate and geometry types (the de-facto standard in
//! the Rust geospatial ecosystem) and the corresponding `s2rst` types.
//!
//! # Coordinate convention
//!
//! `geo-types` uses `Coord { x, y }` where **x = longitude (degrees)** and
//! **y = latitude (degrees)**.  All conversions in this module follow that
//! convention: `Coord.x ↔ lng`, `Coord.y ↔ lat`.
//!
//! # Type mapping
//!
//! | `geo-types` | `s2rst` |
//! |---|---|
//! | `geo_types::Coord<f64>` | [`LatLng`] |
//! | `geo_types::Point<f64>` | [`LatLng`] **and** [`Point`] |
//! | `geo_types::Line<f64>` | [`Polyline`] (2 vertices) |
//! | `geo_types::LineString<f64>` | [`Polyline`] |
//! | `geo_types::MultiLineString<f64>` | `Vec<`[`Polyline`]`>` |
//! | `geo_types::MultiPoint<f64>` | [`PointVector`] |
//! | `geo_types::Polygon<f64>` | [`Polygon`] |
//! | `geo_types::MultiPolygon<f64>` | `Vec<`[`Polygon`]`>` |
//! | `geo_types::Rect<f64>` | [`Rect`] |
//! | `geo_types::Triangle<f64>` | [`Loop`] (3 vertices) |
//!
//! # Polygon ring semantics
//!
//! `geo-types` rings are **closed**: the last coordinate equals the first.
//! `s2rst` [`Loop`]s are **open**: vertices are not repeated.
//! Conversions strip or add the closing vertex as needed.
//!
//! `geo-types` convention (`GeoJSON`) says the exterior ring is
//! counter-clockwise and holes are clockwise.  [`Polygon::from_loops`] runs
//! `init_nesting` automatically, so orientation is inferred from containment
//! and you do not need to pre-orient the rings.
//!
//! # Converting `s2::Polygon` to `geo-types`
//!
//! `s2::Polygon` can have multiple shells (even-depth loops) with nested holes.
//! Because a `geo_types::Polygon` holds exactly one exterior ring, the safe
//! conversion target is always [`geo_types::MultiPolygon<f64>`], which handles
//! all nesting levels correctly.

use geo_types::{
    Coord, Line, LineString, MultiLineString, MultiPoint, MultiPolygon, Point as GeoPoint,
    Polygon as GeoPoly, Rect as GeoRect, Triangle,
};

use crate::s2::point_vector::PointVector;
use crate::s2::polyline::Polyline;
use crate::s2::{LatLng, Loop, Point, Polygon, Rect};

// ─── Internal helpers ────────────────────────────────────────────────────────

/// Converts an `s2::Point` to a `Coord<f64>` (x = lng°, y = lat°).
fn point_to_coord(p: Point) -> Coord<f64> {
    let ll = LatLng::from_point(p);
    Coord {
        x: ll.lng.degrees(),
        y: ll.lat.degrees(),
    }
}

/// Converts a `Coord<f64>` (x = lng°, y = lat°) to an `s2::Point`.
fn coord_to_point(c: Coord<f64>) -> Point {
    LatLng::from_degrees(c.y, c.x).to_point()
}

/// Converts an `s2::Loop` to a closed `LineString<f64>` (last coord == first).
fn loop_to_ring(lp: &Loop) -> LineString<f64> {
    let mut coords: Vec<Coord<f64>> = lp.vertices().iter().map(|&p| point_to_coord(p)).collect();
    if let Some(&first) = coords.first() {
        coords.push(first);
    }
    LineString(coords)
}

/// Converts a closed `LineString<f64>` ring to an `s2::Loop`.
///
/// If the ring is closed (first coord == last coord), the closing vertex is
/// stripped before constructing the loop.
fn ring_to_loop(ls: LineString<f64>) -> Loop {
    let mut coords = ls.into_inner();
    if coords.len() > 1 && coords.first() == coords.last() {
        coords.pop();
    }
    let vertices: Vec<Point> = coords.into_iter().map(coord_to_point).collect();
    Loop::new(vertices)
}

// ─── geo-types → s2rst ───────────────────────────────────────────────────────

/// Converts a `Coord<f64>` (x = longitude°, y = latitude°) to a [`LatLng`].
///
/// # Examples
///
/// ```
/// use geo_types::Coord;
/// use s2rst::s2::LatLng;
///
/// let coord = Coord { x: -0.1278, y: 51.5074 };
/// let ll = LatLng::from(coord);
/// assert!((ll.lat.degrees() - 51.5074).abs() < 1e-10);
/// assert!((ll.lng.degrees() - -0.1278).abs() < 1e-10);
/// ```
impl From<Coord<f64>> for LatLng {
    fn from(c: Coord<f64>) -> Self {
        LatLng::from_degrees(c.y, c.x)
    }
}

/// Converts a [`LatLng`] to a `Coord<f64>` (x = longitude°, y = latitude°).
///
/// # Examples
///
/// ```
/// use geo_types::Coord;
/// use s2rst::s2::LatLng;
///
/// let ll = LatLng::from_degrees(51.5074, -0.1278);
/// let coord = Coord::from(ll);
/// assert!((coord.x - -0.1278).abs() < 1e-10);
/// assert!((coord.y - 51.5074).abs() < 1e-10);
/// ```
impl From<LatLng> for Coord<f64> {
    fn from(ll: LatLng) -> Self {
        Coord {
            x: ll.lng.degrees(),
            y: ll.lat.degrees(),
        }
    }
}

/// Converts a `Point<f64>` (x = longitude°, y = latitude°) to a [`LatLng`].
impl From<GeoPoint<f64>> for LatLng {
    fn from(p: GeoPoint<f64>) -> Self {
        LatLng::from_degrees(p.y(), p.x())
    }
}

/// Converts a [`LatLng`] to a `Point<f64>` (x = longitude°, y = latitude°).
impl From<LatLng> for GeoPoint<f64> {
    fn from(ll: LatLng) -> Self {
        GeoPoint::new(ll.lng.degrees(), ll.lat.degrees())
    }
}

/// Converts a `Point<f64>` (x = longitude°, y = latitude°) to an [`Point`].
///
/// The geographic coordinates are interpreted as (longitude, latitude) in
/// degrees and converted to a unit 3-D vector on the sphere.
impl From<GeoPoint<f64>> for Point {
    fn from(p: GeoPoint<f64>) -> Self {
        LatLng::from_degrees(p.y(), p.x()).to_point()
    }
}

/// Converts an [`Point`] to a `Point<f64>` (x = longitude°, y = latitude°).
impl From<Point> for GeoPoint<f64> {
    fn from(p: Point) -> Self {
        let ll = LatLng::from_point(p);
        GeoPoint::new(ll.lng.degrees(), ll.lat.degrees())
    }
}

/// Converts a `Line<f64>` (two-point segment) to a 2-vertex [`Polyline`].
///
/// The line's `start` and `end` coordinates are interpreted as
/// (longitude°, latitude°).
impl From<Line<f64>> for Polyline {
    fn from(line: Line<f64>) -> Self {
        Polyline::new(vec![coord_to_point(line.start), coord_to_point(line.end)])
    }
}

/// Converts a [`Polyline`] to a `Line<f64>`.
///
/// # Panics
///
/// Panics if the polyline has fewer than 2 vertices.  Use the `From<LineString>`
/// conversion for polylines of arbitrary length.
impl From<Polyline> for Line<f64> {
    fn from(pl: Polyline) -> Self {
        let v = pl.vertices_vec();
        assert!(
            v.len() >= 2,
            "Polyline must have at least 2 vertices to convert to geo_types::Line"
        );
        Line {
            start: point_to_coord(v[0]),
            end: point_to_coord(v[1]),
        }
    }
}

/// Converts a `LineString<f64>` to a [`Polyline`].
///
/// If the line-string is closed (first coord == last coord), the duplicate
/// closing vertex is stripped so that the resulting polyline ends at a
/// different point than it starts.  This is the expected shape when a
/// `LineString` ring is accidentally passed here; for intentional ring
/// conversion see [`From<geo_types::Polygon<f64>> for s2::Polygon`].
impl From<LineString<f64>> for Polyline {
    fn from(ls: LineString<f64>) -> Self {
        let vertices: Vec<Point> = ls.into_inner().into_iter().map(coord_to_point).collect();
        Polyline::new(vertices)
    }
}

/// Converts a [`Polyline`] to a `LineString<f64>`.
impl From<Polyline> for LineString<f64> {
    fn from(pl: Polyline) -> Self {
        LineString(
            pl.vertices_vec()
                .iter()
                .map(|&p| point_to_coord(p))
                .collect(),
        )
    }
}

/// Converts a [`MultiLineString<f64>`] to a `Vec<`[`Polyline`]`>`.
///
/// A free function is used instead of a [`From`] impl because the orphan rule
/// forbids `impl From<MultiLineString> for Vec<_>` when both types are foreign.
///
/// # Examples
///
/// ```
/// use geo_types::{Coord, LineString, MultiLineString};
/// use s2rst::geo_types_interop::multi_linestring_to_polylines;
///
/// let mls = MultiLineString::new(vec![
///     LineString::new(vec![Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 0.0 }]),
/// ]);
/// let pls = multi_linestring_to_polylines(mls);
/// assert_eq!(pls.len(), 1);
/// ```
pub fn multi_linestring_to_polylines(mls: MultiLineString<f64>) -> Vec<Polyline> {
    mls.into_iter().map(Polyline::from).collect()
}

/// Converts a `Vec<`[`Polyline`]`>` to a [`MultiLineString<f64>`].
///
/// A free function is used instead of a [`From`] impl because the orphan rule
/// forbids `impl From<Vec<_>> for MultiLineString` when both types are foreign.
///
/// # Examples
///
/// ```
/// use s2rst::s2::LatLng;
/// use s2rst::s2::polyline::Polyline;
/// use s2rst::geo_types_interop::polylines_to_multi_linestring;
///
/// let pl = Polyline::new(vec![
///     LatLng::from_degrees(0.0, 0.0).to_point(),
///     LatLng::from_degrees(0.0, 1.0).to_point(),
/// ]);
/// let mls = polylines_to_multi_linestring(vec![pl]);
/// assert_eq!(mls.0.len(), 1);
/// ```
pub fn polylines_to_multi_linestring(polylines: Vec<Polyline>) -> MultiLineString<f64> {
    MultiLineString(polylines.into_iter().map(LineString::from).collect())
}

/// Converts a `MultiPoint<f64>` to a [`PointVector`].
impl From<MultiPoint<f64>> for PointVector {
    fn from(mp: MultiPoint<f64>) -> Self {
        PointVector::new(mp.into_iter().map(Point::from).collect())
    }
}

/// Converts a [`PointVector`] to a `MultiPoint<f64>`.
impl From<PointVector> for MultiPoint<f64> {
    fn from(pv: PointVector) -> Self {
        MultiPoint(pv.points().iter().map(|&p| GeoPoint::from(p)).collect())
    }
}

/// Converts a `Triangle<f64>` to a 3-vertex [`Loop`].
///
/// The triangle's three corners are interpreted as (longitude°, latitude°)
/// and converted to geodesic vertices on the unit sphere.
impl From<Triangle<f64>> for Loop {
    fn from(t: Triangle<f64>) -> Self {
        Loop::new(vec![
            coord_to_point(t.v1()),
            coord_to_point(t.v2()),
            coord_to_point(t.v3()),
        ])
    }
}

/// Converts a 3-vertex [`Loop`] to a `Triangle<f64>`.
///
/// # Panics
///
/// Panics if the loop does not have exactly 3 vertices.
impl From<Loop> for Triangle<f64> {
    fn from(lp: Loop) -> Self {
        let v = lp.vertices();
        assert_eq!(
            v.len(),
            3,
            "Loop must have exactly 3 vertices to convert to geo_types::Triangle"
        );
        Triangle(
            point_to_coord(v[0]),
            point_to_coord(v[1]),
            point_to_coord(v[2]),
        )
    }
}

/// Converts a `Rect<f64>` (min = lower-left, max = upper-right) to an
/// [`Rect`] (latitude-longitude rectangle).
///
/// The `Rect` coordinates are interpreted as (x = longitude°, y = latitude°).
impl From<GeoRect<f64>> for Rect {
    fn from(r: GeoRect<f64>) -> Self {
        let min = r.min();
        let max = r.max();
        let lo = LatLng::from_degrees(min.y, min.x);
        let hi = LatLng::from_degrees(max.y, max.x);
        Rect::from_point_pair(lo, hi)
    }
}

/// Converts an [`Rect`] to a `geo_types::Rect<f64>`.
///
/// The output corners use (x = longitude°, y = latitude°).
impl From<Rect> for GeoRect<f64> {
    fn from(r: Rect) -> Self {
        let lo = r.lo();
        let hi = r.hi();
        GeoRect::new(
            Coord {
                x: lo.lng.degrees(),
                y: lo.lat.degrees(),
            },
            Coord {
                x: hi.lng.degrees(),
                y: hi.lat.degrees(),
            },
        )
    }
}

/// Converts a `geo_types::Polygon<f64>` to an [`Polygon`].
///
/// The exterior ring and each interior (hole) ring are each converted to an
/// [`Loop`]. Closing vertices (where first coord == last coord) are stripped
/// automatically. [`Polygon::from_loops`] determines nesting via geometric
/// containment, so you do not need to pre-orient the rings.
///
/// # Examples
///
/// ```
/// use geo_types::{Coord, LineString, Polygon as GeoPoly};
/// use s2rst::s2::{LatLng, Polygon, Region};
///
/// let ring = LineString::new(vec![
///     Coord { x: -10.0, y: -10.0 },
///     Coord { x:  10.0, y: -10.0 },
///     Coord { x:  10.0, y:  10.0 },
///     Coord { x: -10.0, y:  10.0 },
///     Coord { x: -10.0, y: -10.0 }, // closed
/// ]);
/// let geo_poly = GeoPoly::new(ring, vec![]);
/// let s2_poly = Polygon::from(geo_poly);
/// assert!(s2_poly.contains_point(&LatLng::from_degrees(0.0, 0.0).to_point()));
/// ```
impl From<GeoPoly<f64>> for Polygon {
    fn from(poly: GeoPoly<f64>) -> Self {
        let (exterior, interiors) = poly.into_inner();
        let mut loops: Vec<Loop> = Vec::with_capacity(1 + interiors.len());
        loops.push(ring_to_loop(exterior));
        for ring in interiors {
            loops.push(ring_to_loop(ring));
        }
        Polygon::from_loops(loops)
    }
}

/// Converts a [`MultiPolygon<f64>`] to a `Vec<`[`Polygon`]`>`.
///
/// A free function is used instead of a [`From`] impl because the orphan rule
/// forbids `impl From<MultiPolygon> for Vec<_>` when both types are foreign.
///
/// # Examples
///
/// ```
/// use geo_types::{Coord, LineString, MultiPolygon, Polygon as GeoPoly};
/// use s2rst::geo_types_interop::multi_polygon_to_polygons;
///
/// let ring = LineString::new(vec![
///     Coord { x: -5.0, y: -5.0 }, Coord { x: 5.0, y: -5.0 },
///     Coord { x: 5.0, y: 5.0 }, Coord { x: -5.0, y: 5.0 },
///     Coord { x: -5.0, y: -5.0 },
/// ]);
/// let mp = MultiPolygon::new(vec![GeoPoly::new(ring, vec![])]);
/// let polys = multi_polygon_to_polygons(mp);
/// assert_eq!(polys.len(), 1);
/// ```
pub fn multi_polygon_to_polygons(mp: MultiPolygon<f64>) -> Vec<Polygon> {
    mp.into_iter().map(Polygon::from).collect()
}

/// Converts an [`Polygon`] to a `geo_types::MultiPolygon<f64>`.
///
/// `s2::Polygon` loops are arranged in a depth-first nesting hierarchy:
/// even-depth loops are shells and odd-depth loops are holes.  This conversion
/// reconstructs that hierarchy into a `MultiPolygon` where each even-depth
/// shell, together with its immediate odd-depth holes, becomes one
/// `geo_types::Polygon`.  Deeper even-depth loops (islands inside holes) each
/// become independent `geo_types::Polygon` entries.
///
/// Rings are closed (first coord repeated as last coord) to satisfy the
/// `geo_types` / `GeoJSON` contract.
///
/// # Examples
///
/// ```
/// use geo_types::{Coord, LineString, MultiPolygon, Polygon as GeoPoly};
/// use s2rst::s2::{LatLng, Loop, Polygon};
///
/// let shell = Loop::new(vec![
///     LatLng::from_degrees(-10.0, -10.0).to_point(),
///     LatLng::from_degrees(-10.0,  10.0).to_point(),
///     LatLng::from_degrees( 10.0,  10.0).to_point(),
///     LatLng::from_degrees( 10.0, -10.0).to_point(),
/// ]);
/// let s2_poly = Polygon::from_loops(vec![shell]);
/// let multi: MultiPolygon<f64> = MultiPolygon::from(s2_poly);
/// assert_eq!(multi.0.len(), 1);
/// ```
impl From<Polygon> for MultiPolygon<f64> {
    fn from(polygon: Polygon) -> Self {
        /// Accumulates exterior + holes before `GeoPoly::new` is called.
        struct ShellBuilder {
            exterior: LineString<f64>,
            holes: Vec<LineString<f64>>,
            depth: i32,
        }

        let mut result: Vec<GeoPoly<f64>> = Vec::new();
        let mut stack: Vec<ShellBuilder> = Vec::new();

        /// Flush `ShellBuilder`s from the top of `stack` whose depth is ≥
        /// `min_depth`, turning each into a finished `GeoPoly`.
        fn flush_until(
            stack: &mut Vec<ShellBuilder>,
            result: &mut Vec<GeoPoly<f64>>,
            min_depth: i32,
        ) {
            while stack.last().is_some_and(|s| s.depth >= min_depth) {
                if let Some(sb) = stack.pop() {
                    result.push(GeoPoly::new(sb.exterior, sb.holes));
                }
            }
        }

        for lp in polygon.loops() {
            let depth = lp.depth();
            let ring = loop_to_ring(lp);

            if depth % 2 == 0 {
                // Even depth → shell: flush all shells whose depth ≥ this one,
                // then open a new shell.
                flush_until(&mut stack, &mut result, depth);
                stack.push(ShellBuilder {
                    exterior: ring,
                    holes: Vec::new(),
                    depth,
                });
            } else {
                // Odd depth → hole: flush deeper shells first, then add as
                // interior ring of the shell on top.
                flush_until(&mut stack, &mut result, depth);
                if let Some(top) = stack.last_mut() {
                    top.holes.push(ring);
                }
            }
        }

        // Flush any shells that were never closed by a shallower sibling.
        while let Some(sb) = stack.pop() {
            result.push(GeoPoly::new(sb.exterior, sb.holes));
        }

        MultiPolygon(result)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::point_vector::PointVector;
    use crate::s2::polyline::Polyline;
    use crate::s2::{LatLng, Loop, Polygon, Rect, Region};
    use geo_types::{
        Coord, Line, LineString, MultiLineString, MultiPoint, MultiPolygon, Point as GeoPoint,
        Polygon as GeoPoly, Rect as GeoRect, Triangle,
    };
    use quickcheck_macros::quickcheck;

    // Helpers
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-10
    }

    fn ll(lat: f64, lng: f64) -> LatLng {
        LatLng::from_degrees(lat, lng)
    }

    /// Map an arbitrary `f64` (quickcheck may emit `NaN`/±inf) into a finite
    /// value — mirrors the `clamp_finite` helper in the `latlng` property tests.
    fn clamp_finite(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(-1e10, 1e10)
        } else {
            0.0
        }
    }

    /// Squash an arbitrary `f64` into the open interval `(-max, max)` via the
    /// monotonic map `max·v / (1 + |v|)`. Distinct finite inputs stay distinct,
    /// and the result keeps clear of the endpoints — for coordinates that means
    /// clear of the poles (lat = ±90°) and the ±180° antimeridian, where the
    /// latitude/longitude round-trip is singular.
    fn squash(v: f64, max: f64) -> f64 {
        let v = clamp_finite(v);
        max * v / (1.0 + v.abs())
    }

    /// Build a `Coord` (x = lng°, y = lat°) inside the valid ranges and clear of
    /// the round-trip singularities, from two arbitrary `f64`s.
    fn safe_coord(lat: f64, lng: f64) -> Coord<f64> {
        Coord {
            x: squash(lng, 179.0),
            y: squash(lat, 89.0),
        }
    }

    // ── Coord / LatLng ──────────────────────────────────────────────────────

    #[test]
    fn coord_latlng_roundtrip() {
        let c = Coord {
            x: 2.3522_f64,
            y: 48.8566_f64,
        }; // Paris (lng, lat)
        let ll = LatLng::from(c);
        assert!(approx_eq(ll.lat.degrees(), 48.8566));
        assert!(approx_eq(ll.lng.degrees(), 2.3522));
        let back = Coord::from(ll);
        assert!(approx_eq(back.x, c.x));
        assert!(approx_eq(back.y, c.y));
    }

    // ── GeoPoint / LatLng ───────────────────────────────────────────────────

    #[test]
    fn geopoint_latlng_roundtrip() {
        let p = GeoPoint::new(-0.1278_f64, 51.5074_f64); // London (lng, lat)
        let ll = LatLng::from(p);
        assert!(approx_eq(ll.lat.degrees(), 51.5074));
        assert!(approx_eq(ll.lng.degrees(), -0.1278));
        let back = GeoPoint::from(ll);
        assert!(approx_eq(back.x(), p.x()));
        assert!(approx_eq(back.y(), p.y()));
    }

    // ── GeoPoint / s2::Point ────────────────────────────────────────────────

    #[test]
    fn geopoint_s2point_roundtrip() {
        let p = GeoPoint::new(139.6917_f64, 35.6895_f64); // Tokyo
        let s2p = Point::from(p);
        let back = GeoPoint::from(s2p);
        // Allow 1e-13 for the double round-trip through 3D normalisation.
        assert!((back.x() - p.x()).abs() < 1e-13);
        assert!((back.y() - p.y()).abs() < 1e-13);
    }

    // ── Line / Polyline ─────────────────────────────────────────────────────

    #[test]
    fn line_to_polyline() {
        let line = Line {
            start: Coord { x: 0.0, y: 0.0 },
            end: Coord { x: 90.0, y: 0.0 },
        };
        let pl = Polyline::from(line);
        assert_eq!(pl.vertices_vec().len(), 2);
    }

    // ── LineString / Polyline ───────────────────────────────────────────────

    #[test]
    fn linestring_to_polyline_vertex_count() {
        let ls = LineString::new(vec![
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 90.0, y: 0.0 },
            Coord { x: 0.0, y: 90.0 },
        ]);
        let pl = Polyline::from(ls);
        assert_eq!(pl.vertices_vec().len(), 3);
    }

    #[test]
    fn polyline_to_linestring_vertex_count() {
        let pl = Polyline::new(vec![
            ll(0.0, 0.0).to_point(),
            ll(0.0, 90.0).to_point(),
            ll(90.0, 0.0).to_point(),
        ]);
        let ls = LineString::from(pl);
        assert_eq!(ls.0.len(), 3);
    }

    // ── MultiLineString / Vec<Polyline> ─────────────────────────────────────

    #[test]
    fn multi_linestring_roundtrip_count() {
        let mls = MultiLineString(vec![
            LineString::new(vec![Coord { x: 0.0, y: 0.0 }, Coord { x: 1.0, y: 0.0 }]),
            LineString::new(vec![Coord { x: 2.0, y: 0.0 }, Coord { x: 3.0, y: 0.0 }]),
        ]);
        let pls = multi_linestring_to_polylines(mls);
        assert_eq!(pls.len(), 2);
        let back = polylines_to_multi_linestring(pls);
        assert_eq!(back.0.len(), 2);
    }

    // ── MultiPoint / PointVector ─────────────────────────────────────────────

    #[test]
    fn multipoint_pointvector_roundtrip() {
        let mp = MultiPoint(vec![GeoPoint::new(0.0, 0.0), GeoPoint::new(90.0, 45.0)]);
        let pv = PointVector::from(mp);
        assert_eq!(pv.points().len(), 2);
        let back = MultiPoint::from(pv);
        assert_eq!(back.0.len(), 2);
        assert!(approx_eq(back.0[1].x(), 90.0));
        assert!(approx_eq(back.0[1].y(), 45.0));
    }

    // ── Triangle / Loop ──────────────────────────────────────────────────────

    #[test]
    fn triangle_to_loop_vertex_count() {
        let t = Triangle(
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
            Coord { x: 5.0, y: 10.0 },
        );
        let lp = Loop::from(t);
        assert_eq!(lp.vertices().len(), 3);
    }

    // ── Rect ─────────────────────────────────────────────────────────────────

    #[test]
    fn rect_roundtrip() {
        let geo_rect = GeoRect::new(Coord { x: -10.0, y: -20.0 }, Coord { x: 10.0, y: 20.0 });
        let s2_rect = Rect::from(geo_rect);
        assert!(approx_eq(s2_rect.lo().lat.degrees(), -20.0));
        assert!(approx_eq(s2_rect.lo().lng.degrees(), -10.0));
        assert!(approx_eq(s2_rect.hi().lat.degrees(), 20.0));
        assert!(approx_eq(s2_rect.hi().lng.degrees(), 10.0));

        let back = GeoRect::from(s2_rect);
        assert!(approx_eq(back.min().x, -10.0));
        assert!(approx_eq(back.min().y, -20.0));
        assert!(approx_eq(back.max().x, 10.0));
        assert!(approx_eq(back.max().y, 20.0));
    }

    // ── Polygon ──────────────────────────────────────────────────────────────

    /// A square ring centred on the origin, CCW (exterior convention).
    fn square_ring(size: f64) -> LineString<f64> {
        let s = size;
        LineString::new(vec![
            Coord { x: -s, y: -s },
            Coord { x: s, y: -s },
            Coord { x: s, y: s },
            Coord { x: -s, y: s },
            Coord { x: -s, y: -s }, // closing vertex
        ])
    }

    #[test]
    fn geo_polygon_to_s2_polygon_contains_center() {
        let geo_poly = GeoPoly::new(square_ring(10.0), vec![]);
        let s2_poly = Polygon::from(geo_poly);
        let center = ll(0.0, 0.0).to_point();
        assert!(s2_poly.contains_point(&center));
        let outside = ll(20.0, 0.0).to_point();
        assert!(!s2_poly.contains_point(&outside));
    }

    #[test]
    fn geo_polygon_with_hole() {
        // Outer square [-10, 10]², inner square [-2, 2]² (hole).
        let outer = square_ring(10.0);
        let inner = square_ring(2.0);
        let geo_poly = GeoPoly::new(outer, vec![inner]);
        let s2_poly = Polygon::from(geo_poly);

        // A point in the ring (between inner and outer) is contained.
        assert!(s2_poly.contains_point(&ll(5.0, 5.0).to_point()));
        // A point inside the hole is NOT contained.
        assert!(!s2_poly.contains_point(&ll(0.0, 0.0).to_point()));
    }

    #[test]
    fn s2_polygon_to_multi_polygon_single_shell() {
        let shell = Loop::new(vec![
            ll(-10.0, -10.0).to_point(),
            ll(-10.0, 10.0).to_point(),
            ll(10.0, 10.0).to_point(),
            ll(10.0, -10.0).to_point(),
        ]);
        let s2_poly = Polygon::from_loops(vec![shell]);
        let multi = MultiPolygon::from(s2_poly);
        assert_eq!(multi.0.len(), 1);
        // Exterior ring should be closed (len = 4 vertices + 1 closing = 5).
        assert_eq!(multi.0[0].exterior().0.len(), 5);
    }

    #[test]
    fn s2_polygon_to_multi_polygon_with_hole() {
        let outer = Loop::new(vec![
            ll(-10.0, -10.0).to_point(),
            ll(-10.0, 10.0).to_point(),
            ll(10.0, 10.0).to_point(),
            ll(10.0, -10.0).to_point(),
        ]);
        let inner = Loop::new(vec![
            ll(-2.0, -2.0).to_point(),
            ll(-2.0, 2.0).to_point(),
            ll(2.0, 2.0).to_point(),
            ll(2.0, -2.0).to_point(),
        ]);
        let s2_poly = Polygon::from_loops(vec![outer, inner]);
        let multi = MultiPolygon::from(s2_poly);
        assert_eq!(multi.0.len(), 1);
        assert_eq!(multi.0[0].interiors().len(), 1);
    }

    #[test]
    fn s2_polygon_to_multi_polygon_two_shells() {
        // Two disjoint shells (both depth 0). Converting the second shell makes
        // `flush_until` pop the first, exercising the multi-shell branch that
        // the single-shell and with-hole cases never reach.
        let west = Loop::new(vec![
            ll(-10.0, -40.0).to_point(),
            ll(-10.0, -20.0).to_point(),
            ll(10.0, -20.0).to_point(),
            ll(10.0, -40.0).to_point(),
        ]);
        let east = Loop::new(vec![
            ll(-10.0, 20.0).to_point(),
            ll(-10.0, 40.0).to_point(),
            ll(10.0, 40.0).to_point(),
            ll(10.0, 20.0).to_point(),
        ]);
        let s2_poly = Polygon::from_loops(vec![west, east]);
        let multi = MultiPolygon::from(s2_poly);
        // Each disjoint shell becomes its own single-ring polygon.
        assert_eq!(multi.0.len(), 2);
        assert!(multi.0.iter().all(|p| p.interiors().is_empty()));
    }

    // ── Polyline → Line (reverse) ───────────────────────────────────────────

    #[test]
    fn polyline_to_line_roundtrip() {
        let line = Line {
            start: Coord { x: 10.0, y: 20.0 },
            end: Coord { x: 30.0, y: 40.0 },
        };
        let pl = Polyline::from(line);
        let back = Line::from(pl);
        assert!(approx_eq(back.start.x, 10.0));
        assert!(approx_eq(back.start.y, 20.0));
        assert!(approx_eq(back.end.x, 30.0));
        assert!(approx_eq(back.end.y, 40.0));
    }

    // ── LineString ↔ Polyline coordinate roundtrip ──────────────────────────

    #[test]
    fn linestring_polyline_coordinate_roundtrip() {
        let coords = vec![
            Coord {
                x: -73.9857,
                y: 40.7484,
            }, // NYC
            Coord {
                x: 2.2945,
                y: 48.8584,
            }, // Paris
            Coord {
                x: 139.6917,
                y: 35.6895,
            }, // Tokyo
        ];
        let ls = LineString::new(coords.clone());
        let pl = Polyline::from(ls);
        let back = LineString::from(pl);
        assert_eq!(back.0.len(), coords.len());
        for (got, want) in back.0.iter().zip(&coords) {
            assert!(approx_eq(got.x, want.x));
            assert!(approx_eq(got.y, want.y));
        }
    }

    // ── Loop → Triangle (reverse) ───────────────────────────────────────────

    #[test]
    fn loop_triangle_roundtrip() {
        let t = Triangle(
            Coord { x: 0.0, y: 0.0 },
            Coord { x: 10.0, y: 0.0 },
            Coord { x: 5.0, y: 10.0 },
        );
        let lp = Loop::from(t);
        let back = Triangle::from(lp);
        assert!(approx_eq(back.v1().x, 0.0));
        assert!(approx_eq(back.v1().y, 0.0));
        assert!(approx_eq(back.v2().x, 10.0));
        assert!(approx_eq(back.v2().y, 0.0));
        assert!(approx_eq(back.v3().x, 5.0));
        assert!(approx_eq(back.v3().y, 10.0));
    }

    /// Property: any triangle whose vertices avoid the poles and the ±180°
    /// antimeridian survives a `Triangle → Loop → Triangle` round-trip, vertex
    /// for vertex. Complements the fixed-example `loop_triangle_roundtrip` above
    /// with the crate's quickcheck style.
    #[quickcheck]
    fn prop_triangle_loop_roundtrip(
        lat1: f64,
        lng1: f64,
        lat2: f64,
        lng2: f64,
        lat3: f64,
        lng3: f64,
    ) -> bool {
        let c = [
            safe_coord(lat1, lng1),
            safe_coord(lat2, lng2),
            safe_coord(lat3, lng3),
        ];
        let t = Triangle(c[0], c[1], c[2]);

        // `Loop::new` keeps all three vertices, so `From<Loop> for Triangle`
        // never trips its 3-vertex assertion — and if that ever regresses, this
        // property panics and fails, which is exactly the signal we want.
        let back = Triangle::from(Loop::from(t));

        // 1e-9° is well above the ~1e-13 rad error of the round-trip through the
        // unit-vector representation.
        let close =
            |a: Coord<f64>, b: Coord<f64>| (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9;
        close(back.v1(), c[0]) && close(back.v2(), c[1]) && close(back.v3(), c[2])
    }

    // ── MultiPolygon / Vec<Polygon> ──────────────────────────────────────────

    #[test]
    fn multi_polygon_to_vec_s2_polygon() {
        let mp = MultiPolygon(vec![
            GeoPoly::new(square_ring(5.0), vec![]),
            GeoPoly::new(square_ring(3.0), vec![]),
        ]);
        let polys = multi_polygon_to_polygons(mp);
        assert_eq!(polys.len(), 2);
    }
}
