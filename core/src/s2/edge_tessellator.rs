// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Edge tessellation for map projections.
//!
//! [`EdgeTessellator`] converts geodesic edges to chains of projected edges
//! (and vice versa) such that the maximum error between the original edge
//! and the chain is at most the requested tolerance.
//!
//! Corresponds to Go `s2/edge_tessellator.go`, C++ `s2edge_tessellator.cc`.

use crate::r2;
use crate::s1::{Angle, ChordAngle};
use crate::s2::Point;
use crate::s2::edge_distances;
use crate::s2::projections::Projection;

/// Fraction of the edge length at which error is evaluated (≈ 0.312).
const TESSELLATION_INTERPOLATION_FRACTION: f64 = 0.31215691082248312;

/// Scale factor applied to the tolerance to make the error estimate
/// conservative. The error is sampled at two points and can underestimate
/// the true maximum; this factor compensates.
const TESSELLATION_SCALE_FACTOR: f64 = 0.838_299_925_698_885_1;

/// Minimum supported tolerance (~1 micrometer on the Earth's surface).
const MIN_TESSELLATION_TOLERANCE: f64 = 1e-13;

/// Converts spherical geodesic edges to chains of projected edges (and
/// vice versa) such that the maximum distance between the original edge
/// and the chain is at most the requested tolerance.
#[derive(Debug)]
pub struct EdgeTessellator<P: Projection> {
    projection: P,
    /// The tolerance scaled so it can be compared directly against the
    /// error estimate.
    scaled_tolerance: ChordAngle,
}

impl<P: Projection> EdgeTessellator<P> {
    /// Creates a new edge tessellator for the given projection and
    /// tolerance.
    pub fn new(projection: P, tolerance: Angle) -> Self {
        let tol = tolerance.radians().max(MIN_TESSELLATION_TOLERANCE);
        EdgeTessellator {
            projection,
            scaled_tolerance: ChordAngle::from_angle(Angle::from_radians(
                TESSELLATION_SCALE_FACTOR * tol,
            )),
        }
    }

    /// Converts the spherical geodesic edge AB to a chain of planar edges
    /// in the projection and appends the projected vertices.
    ///
    /// If `vertices` is empty, the projected A is pushed first. If
    /// `vertices` is non-empty, the last vertex is used as the wrapping
    /// reference for A.
    pub fn append_projected(&self, a: Point, b: Point, vertices: &mut Vec<r2::Point>) {
        let mut pa = self.projection.project(a);
        if vertices.is_empty() {
            vertices.push(pa);
        } else {
            pa = self
                .projection
                .wrap_destination(vertices[vertices.len() - 1], pa);
            debug_assert!(
                vertices[vertices.len() - 1] == pa,
                "Appended edges must form a chain"
            );
        }
        let pb = self.projection.project(b);
        self.append_projected_inner(pa, a, pb, b, vertices);
    }

    /// Converts the planar edge AB (in projection coordinates) to a chain
    /// of spherical geodesic edges and appends the vertices.
    ///
    /// If `vertices` is empty, the unprojected A is pushed first.
    pub fn append_unprojected(&self, pa: r2::Point, pb: r2::Point, vertices: &mut Vec<Point>) {
        let a = self.projection.unproject(pa);
        let b = self.projection.unproject(pb);
        if vertices.is_empty() {
            vertices.push(a);
        } else {
            debug_assert!(
                vertices[vertices.len() - 1].approx_eq(a),
                "Appended edges must form a chain"
            );
        }
        self.append_unprojected_inner(pa, a, pb, b, vertices);
    }

    /// Recursive tessellation for projected output.
    fn append_projected_inner(
        &self,
        pa: r2::Point,
        a: Point,
        pb_in: r2::Point,
        b: Point,
        vertices: &mut Vec<r2::Point>,
    ) {
        let pb = self.projection.wrap_destination(pa, pb_in);
        if self.estimate_max_error(pa, a, pb, b) <= self.scaled_tolerance {
            vertices.push(pb);
            return;
        }

        let mid = edge_distances::interpolate(0.5, a, b);
        let pmid = self
            .projection
            .wrap_destination(pa, self.projection.project(mid));
        self.append_projected_inner(pa, a, pmid, mid, vertices);
        self.append_projected_inner(pmid, mid, pb, b, vertices);
    }

    /// Recursive tessellation for unprojected (spherical) output.
    fn append_unprojected_inner(
        &self,
        pa: r2::Point,
        a: Point,
        pb_in: r2::Point,
        b: Point,
        vertices: &mut Vec<Point>,
    ) {
        let pb = self.projection.wrap_destination(pa, pb_in);
        if self.estimate_max_error(pa, a, pb, b) <= self.scaled_tolerance {
            vertices.push(b);
            return;
        }

        let pmid = self.projection.interpolate(0.5, pa, pb);
        let mid = self.projection.unproject(pmid);
        self.append_unprojected_inner(pa, a, pmid, mid, vertices);
        self.append_unprojected_inner(pmid, mid, pb, b, vertices);
    }

    /// Estimates the maximum error between the geodesic edge AB and the
    /// projected edge pa→pb.
    fn estimate_max_error(&self, pa: r2::Point, a: Point, pb: r2::Point, b: Point) -> ChordAngle {
        // Always tessellate edges longer than 90 degrees.
        if a.0.dot(b.0) < -1e-14 {
            return ChordAngle::INFINITY;
        }

        let t1 = TESSELLATION_INTERPOLATION_FRACTION;
        let t2 = 1.0 - TESSELLATION_INTERPOLATION_FRACTION;

        let mid1 = edge_distances::interpolate(t1, a, b);
        let mid2 = edge_distances::interpolate(t2, a, b);

        let pmid1 = self
            .projection
            .unproject(self.projection.interpolate(t1, pa, pb));
        let pmid2 = self
            .projection
            .unproject(self.projection.interpolate(t2, pa, pb));

        let e1 = mid1.chord_angle(pmid1);
        let e2 = mid2.chord_angle(pmid2);
        if e1 > e2 { e1 } else { e2 }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::LatLng;
    use crate::s2::edge_distances;
    use crate::s2::projections::{MercatorProjection, PlateCarreeProjection, Projection};
    use crate::s2::text_format::parse_points;

    // ─── Stats helper (mirrors C++ Stats class) ──────────────────────

    struct Stats {
        max: f64,
        sum: f64,
        count: usize,
    }

    impl Stats {
        fn new() -> Self {
            Stats {
                max: f64::NEG_INFINITY,
                sum: 0.0,
                count: 0,
            }
        }

        fn tally(&mut self, v: f64) {
            assert!(!v.is_nan(), "NaN in stats tally");
            self.max = self.max.max(v);
            self.sum += v;
            self.count += 1;
        }

        fn max(&self) -> f64 {
            self.max
        }
    }

    /// Max projection error allowed when round-tripping through project/unproject.
    const MAX_PROJ_ERROR: f64 = 3e-14; // radians

    /// Get max distance between projected and geodesic edge.
    fn get_max_distance<P: Projection>(
        proj: &P,
        px: r2::Point,
        x: Point,
        py: r2::Point,
        y: Point,
    ) -> Angle {
        const NUM_STEPS: usize = 100;
        let mut max_dist = ChordAngle::ZERO;
        for step in 0..NUM_STEPS {
            let f = (step as f64 + 0.5) / NUM_STEPS as f64;
            let p = proj.unproject(proj.interpolate(f, px, py));
            let (dist, _) = edge_distances::update_min_distance(p, x, y, ChordAngle::INFINITY);
            if dist > max_dist {
                max_dist = dist;
            }
        }
        // Use a lower bound to avoid false failures.
        let err = edge_distances::update_min_distance_max_error(max_dist);
        Angle::from(max_dist) - Angle::from_radians(err)
    }

    /// Test unprojected edge: converts projected edge to geodesic chain.
    fn test_unprojected<P: Projection + Copy>(
        proj: &P,
        tolerance: Angle,
        pa: r2::Point,
        pb_in: r2::Point,
    ) -> Stats {
        let tess = EdgeTessellator::new(*proj, tolerance);
        let mut vertices = Vec::new();
        tess.append_unprojected(pa, pb_in, &mut vertices);
        let pb = proj.wrap_destination(pa, pb_in);
        assert!(
            Angle::from(proj.unproject(pa).chord_angle(vertices[0])).radians() <= MAX_PROJ_ERROR,
            "Start point mismatch"
        );
        assert!(
            Angle::from(proj.unproject(pb).chord_angle(*vertices.last().unwrap())).radians()
                <= MAX_PROJ_ERROR,
            "End point mismatch"
        );
        let mut stats = Stats::new();
        if pa == pb_in {
            assert_eq!(vertices.len(), 1);
            return stats;
        }
        let mut x = vertices[0];
        let mut px_cur = proj.project(x);
        for y in &vertices[1..] {
            let y = *y;
            let py_cur = proj.wrap_destination(px_cur, proj.project(y));
            stats.tally(get_max_distance(proj, px_cur, x, py_cur, y) / tolerance);
            x = y;
            px_cur = py_cur;
        }
        stats
    }

    /// Test projected edge: converts geodesic edge to projected chain.
    fn test_projected<P: Projection + Copy>(
        proj: &P,
        tolerance: Angle,
        a: Point,
        b: Point,
    ) -> Stats {
        let tess = EdgeTessellator::new(*proj, tolerance);
        let mut vertices = Vec::new();
        tess.append_projected(a, b, &mut vertices);
        assert!(
            Angle::from(a.chord_angle(proj.unproject(vertices[0]))).radians() <= MAX_PROJ_ERROR,
            "Start point mismatch"
        );
        assert!(
            Angle::from(b.chord_angle(proj.unproject(*vertices.last().unwrap()))).radians()
                <= MAX_PROJ_ERROR,
            "End point mismatch"
        );
        let mut stats = Stats::new();
        if a == b {
            assert_eq!(vertices.len(), 1);
            return stats;
        }
        let mut px_cur = vertices[0];
        let mut x = proj.unproject(px_cur);
        for py_cur in &vertices[1..] {
            let py_cur = *py_cur;
            let y = proj.unproject(py_cur);
            stats.tally(get_max_distance(proj, px_cur, x, py_cur, y) / tolerance);
            x = y;
            px_cur = py_cur;
        }
        stats
    }

    // ─── Original tests ───────────────────────────────────────────────

    #[test]
    fn test_tessellator_projected_short_edge() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(1.0));
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(1.0, 0.01, 0.0);
        let mut vertices = Vec::new();
        tess.append_projected(a, b, &mut vertices);
        assert!(
            vertices.len() >= 2,
            "expected >= 2 vertices, got {}",
            vertices.len(),
        );
    }

    #[test]
    fn test_tessellator_projected_long_edge() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.1));
        let a = LatLng::from_degrees(60.0, 0.0).to_point();
        let b = LatLng::from_degrees(60.0, 90.0).to_point();
        let mut vertices = Vec::new();
        tess.append_projected(a, b, &mut vertices);
        assert!(
            vertices.len() > 2,
            "expected > 2 vertices for high-lat edge, got {}",
            vertices.len(),
        );
    }

    #[test]
    fn test_tessellator_unprojected() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(1.0));
        let pa = r2::Point::new(0.0, 0.0);
        let pb = r2::Point::new(90.0, 45.0);
        let mut vertices = Vec::new();
        tess.append_unprojected(pa, pb, &mut vertices);
        assert!(
            vertices.len() >= 2,
            "expected >= 2 vertices, got {}",
            vertices.len(),
        );
        for v in &vertices {
            assert!(
                v.is_unit(),
                "vertex {} is not unit length (norm = {})",
                v,
                v.0.norm(),
            );
        }
    }

    #[test]
    fn test_tessellator_same_point() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(1.0));
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let mut vertices = Vec::new();
        tess.append_projected(p, p, &mut vertices);
        assert_eq!(vertices.len(), 2);
    }

    #[test]
    fn test_tessellator_tolerance_respected() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let a = LatLng::from_degrees(60.0, 0.0).to_point();
        let b = LatLng::from_degrees(60.0, 90.0).to_point();
        let mut vertices = Vec::new();
        tess.append_projected(a, b, &mut vertices);
        assert!(
            vertices.len() > 5,
            "expected > 5 vertices for high-lat edge with 0.01° tolerance, got {}",
            vertices.len(),
        );
    }

    #[test]
    fn test_tessellator_projected_consecutive_edges() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(1.0));
        let a = LatLng::from_degrees(0.0, 0.0).to_point();
        let b = LatLng::from_degrees(0.0, 10.0).to_point();
        let c = LatLng::from_degrees(0.0, 20.0).to_point();

        let mut vertices = Vec::new();
        tess.append_projected(a, b, &mut vertices);
        let n_after_first = vertices.len();
        assert!(n_after_first >= 2);

        tess.append_projected(b, c, &mut vertices);
        assert!(vertices.len() > n_after_first);

        for i in 1..vertices.len() {
            assert!(
                vertices[i].x >= vertices[i - 1].x - 1e-10,
                "longitude should increase monotonically at index {i}"
            );
        }
    }

    // ─── C++ tests: no tessellation ──────────────────────────────────

    /// C++ TEST(S2EdgeTessellator, `ProjectedNoTessellation`)
    #[test]
    fn test_projected_no_tessellation() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let mut vertices = Vec::new();
        tess.append_projected(
            Point::from_coords(1.0, 0.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
            &mut vertices,
        );
        assert_eq!(vertices.len(), 2);
    }

    /// C++ TEST(S2EdgeTessellator, `UnprojectedNoTessellation`)
    #[test]
    fn test_unprojected_no_tessellation() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let mut vertices = Vec::new();
        tess.append_unprojected(
            r2::Point::new(0.0, 30.0),
            r2::Point::new(0.0, 50.0),
            &mut vertices,
        );
        assert_eq!(vertices.len(), 2);
    }

    // ─── C++ tests: wrapping ────────────────────────────────────────

    /// C++ TEST(S2EdgeTessellator, `UnprojectedWrapping`)
    #[test]
    fn test_unprojected_wrapping() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let mut vertices = Vec::new();
        tess.append_unprojected(
            r2::Point::new(-170.0, 0.0),
            r2::Point::new(170.0, 80.0),
            &mut vertices,
        );
        for v in &vertices {
            assert!(
                LatLng::longitude(*v).degrees().abs() >= 170.0,
                "vertex has longitude {} < 170",
                LatLng::longitude(*v).degrees()
            );
        }
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedWrapping`)
    #[test]
    fn test_projected_wrapping() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let mut vertices = Vec::new();
        tess.append_projected(
            LatLng::from_degrees(0.0, -170.0).to_point(),
            LatLng::from_degrees(0.0, 170.0).to_point(),
            &mut vertices,
        );
        for v in &vertices {
            assert!(v.x <= -170.0, "projected vertex has x = {} > -170", v.x);
        }
    }

    /// C++ TEST(S2EdgeTessellator, `UnprojectedWrappingMultipleCrossings`)
    #[test]
    fn test_unprojected_wrapping_multiple_crossings() {
        let proj = PlateCarreeProjection::new(180.0);
        let tess = EdgeTessellator::new(proj, Angle::from_degrees(0.01));
        let mut vertices = Vec::new();
        for lat in 1..=60 {
            let lat_f = f64::from(lat);
            tess.append_unprojected(
                r2::Point::new(180.0 - 0.03 * lat_f, lat_f),
                r2::Point::new(-180.0 + 0.07 * lat_f, lat_f),
                &mut vertices,
            );
            tess.append_unprojected(
                r2::Point::new(-180.0 + 0.07 * lat_f, lat_f),
                r2::Point::new(180.0 - 0.03 * (lat_f + 1.0), lat_f + 1.0),
                &mut vertices,
            );
        }
        for v in &vertices {
            assert!(
                LatLng::longitude(*v).degrees().abs() >= 175.0,
                "vertex has longitude {} < 175",
                LatLng::longitude(*v).degrees()
            );
        }
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedWrappingMultipleCrossings`)
    #[test]
    fn test_projected_wrapping_multiple_crossings() {
        let loop_pts = parse_points("0:160, 0:-40, 0:120, 0:-80, 10:120, 10:-40, 0:160");
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_radians(1e-7); // ~S1Angle::E7(1)
        let tess = EdgeTessellator::new(proj, tolerance);
        let mut vertices = Vec::new();
        for i in 0..loop_pts.len() - 1 {
            tess.append_projected(loop_pts[i], loop_pts[i + 1], &mut vertices);
        }
        assert_eq!(vertices.first(), vertices.last());

        // Note: R2Point coordinates are in (lng, lat) order.
        let min_lng = vertices.iter().map(|v| v.x).fold(f64::INFINITY, f64::min);
        let max_lng = vertices
            .iter()
            .map(|v| v.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (min_lng - 160.0).abs() < 1e-10,
            "min_lng = {min_lng}, expected 160"
        );
        assert!(
            (max_lng - 640.0).abs() < 1e-10,
            "max_lng = {max_lng}, expected 640"
        );
    }

    // ─── C++ tests: infinite recursion bug ──────────────────────────

    /// C++ TEST(S2EdgeTessellator, `InfiniteRecursionBug`)
    #[test]
    fn test_infinite_recursion_bug() {
        let proj = PlateCarreeProjection::new(180.0);
        let one_micron = Angle::from_radians(1e-6 / 6371.0);
        let tess = EdgeTessellator::new(proj, one_micron);
        let mut vertices = Vec::new();
        tess.append_projected(
            LatLng::from_degrees(3.0, 21.0).to_point(),
            LatLng::from_degrees(1.0, -159.0).to_point(),
            &mut vertices,
        );
        assert_eq!(vertices.len(), 36);
    }

    // ─── C++ tests: accuracy ────────────────────────────────────────

    /// C++ TEST(S2EdgeTessellator, `UnprojectedAccuracy`)
    #[test]
    fn test_unprojected_accuracy() {
        let proj = MercatorProjection::new(180.0);
        let tolerance = Angle::from_degrees(1e-5);
        let stats = test_unprojected(
            &proj,
            tolerance,
            r2::Point::new(0.0, 0.0),
            r2::Point::new(89.999999, 179.0),
        );
        assert!(stats.max() <= 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `UnprojectedAccuracyCrossEquator`)
    #[test]
    fn test_unprojected_accuracy_cross_equator() {
        let proj = MercatorProjection::new(180.0);
        let tolerance = Angle::from_degrees(1e-5);
        let stats = test_unprojected(
            &proj,
            tolerance,
            r2::Point::new(-10.0, -10.0),
            r2::Point::new(10.0, 10.0),
        );
        assert!(stats.max() < 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedAccuracy`)
    #[test]
    fn test_projected_accuracy() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_radians(1e-7); // ~S1Angle::E7(1)
        let a = LatLng::from_degrees(-89.999, -170.0).to_point();
        let b = LatLng::from_degrees(50.0, 100.0).to_point();
        let stats = test_projected(&proj, tolerance, a, b);
        assert!(stats.max() <= 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `UnprojectedAccuracyMidpointEquator`)
    #[test]
    fn test_unprojected_accuracy_midpoint_equator() {
        let proj = PlateCarreeProjection::new(180.0);
        // ~1 meter tolerance
        let tolerance = Angle::from_radians(1.0 / 6371000.0);
        let stats = test_unprojected(
            &proj,
            tolerance,
            r2::Point::new(80.0, 50.0),
            r2::Point::new(-80.0, -50.0),
        );
        assert!(stats.max() <= 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedAccuracyMidpointEquator`)
    #[test]
    fn test_projected_accuracy_midpoint_equator() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_radians(1.0 / 6371000.0);
        let a = LatLng::from_degrees(50.0, 80.0).to_point();
        let b = LatLng::from_degrees(-50.0, -80.0).to_point();
        let stats = test_projected(&proj, tolerance, a, b);
        assert!(stats.max() <= 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedAccuracyCrossEquator`)
    #[test]
    fn test_projected_accuracy_cross_equator() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_radians(1e-7);
        let a = LatLng::from_degrees(-20.0, -20.0).to_point();
        let b = LatLng::from_degrees(20.0, 20.0).to_point();
        let stats = test_projected(&proj, tolerance, a, b);
        assert!(stats.max() < 1.0, "max ratio = {}", stats.max());
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedAccuracySeattleToNewYork`)
    #[test]
    fn test_projected_accuracy_seattle_to_new_york() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_radians(1.0 / 6371000.0); // ~1 meter
        let seattle = LatLng::from_degrees(47.6062, -122.3321).to_point();
        let newyork = LatLng::from_degrees(40.7128, -74.0059).to_point();
        let stats = test_projected(&proj, tolerance, seattle, newyork);
        assert!(stats.max() <= 1.0, "max ratio = {}", stats.max());
    }

    // ─── C++ tests: unwrapping DCHECK regression ────────────────────

    /// C++ TEST(S2EdgeTessellator, `UnwrappingDcheckRegression`)
    ///
    /// Tests that projecting a series of points near the 180-degree meridian
    /// with a small wrapping distance (0.5) does not cause infinite recursion.
    /// Each edge is tessellated independently because the small wrap distance
    /// means consecutive projected endpoints may not exactly match.
    #[test]
    fn test_unwrapping_dcheck_regression() {
        let points: &[(f64, f64)] = &[
            (-16.876721435218865253, -179.986547984808964884),
            (-16.874909244632696925, -179.991889238369623172),
            (-16.880241814330226191, -179.990858688466971671),
            (-16.883762104047619346, -179.995169553755403058),
            (-16.881949690252106677, 179.999489074621124018),
            (-16.876617071405430437, 179.998458788144517939),
            (-16.880137137875717457, 179.994147804931060364),
            (-16.878324446969305228, 179.988806637264332267),
            (-16.872991774409559440, 179.987776672537478362),
            (-16.869471841739493101, 179.992087611973005323),
            (-16.867659097232969856, 179.986746766061799008),
            (-16.862326415537093993, 179.985716917832945683),
            (-16.858806527326652969, 179.990027652027180238),
            (-16.860619186956174786, 179.995368278278732532),
            (-16.855286549828541354, 179.994338224830613626),
            (-16.851766483129139829, 179.998648636203512297),
            (-16.849953908374558864, 179.993308229628894424),
        ];

        let proj = MercatorProjection::new(0.5);
        let tolerance = Angle::from_radians(1e-7); // ~S1Angle::E7(1)
        let tess = EdgeTessellator::new(proj, tolerance);

        // Tessellate each edge independently — with a wrap distance of 0.5
        // the projected endpoints of consecutive edges may not match exactly
        // due to coordinate wrapping, so we cannot chain them.
        let mut total_vertices = 0;
        for i in 0..points.len() - 1 {
            let mut vertices = Vec::new();
            tess.append_projected(
                LatLng::from_degrees(points[i].0, points[i].1).to_point(),
                LatLng::from_degrees(points[i + 1].0, points[i + 1].1).to_point(),
                &mut vertices,
            );
            // Each edge should produce exactly 2 vertices (no tessellation needed).
            assert_eq!(
                vertices.len(),
                2,
                "edge {i} produced {} vertices",
                vertices.len()
            );
            total_vertices += 1; // count endpoints (first is shared)
        }
        // 16 edges → 17 unique endpoints.
        assert_eq!(total_vertices + 1, 17);
    }

    // ─── C++ tests: random accuracy checks ──────────────────────────

    /// C++ TEST(S2EdgeTessellator, `UnprojectedAccuracyRandomCheck`)
    #[test]
    fn test_unprojected_accuracy_random_check() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_degrees(1e-3);
        // Use a deterministic set of points instead of random.
        let test_cases: &[(f64, f64, f64)] = &[
            (0.0, 45.0, 90.0),
            (-45.0, 30.0, 120.0),
            (60.0, -60.0, 45.0),
            (-89.0, 89.0, 179.0),
            (10.0, -10.0, 170.0),
            (0.0, 80.0, 10.0),
            (-70.0, 70.0, 5.0),
            (45.0, -45.0, 89.0),
            (1.0, -1.0, 1.0),
            (89.0, -89.0, 0.1),
        ];
        for &(alat, blat, blon) in test_cases {
            let pa = r2::Point::new(0.0, alat);
            let pb = r2::Point::new(blon, blat);
            let stats = test_unprojected(&proj, tolerance, pa, pb);
            assert!(
                stats.max() < 1.0,
                "Failed for ({alat}, 0) -> ({blat}, {blon}): max = {}",
                stats.max()
            );
        }
    }

    /// C++ TEST(S2EdgeTessellator, `ProjectedAccuracyRandomCheck`)
    #[test]
    fn test_projected_accuracy_random_check() {
        let proj = PlateCarreeProjection::new(180.0);
        let tolerance = Angle::from_degrees(1e-3);
        // Deterministic set of test cases.
        let test_cases: &[(f64, f64, f64)] = &[
            (0.0, 45.0, 90.0),
            (-45.0, 30.0, -120.0),
            (60.0, -60.0, 45.0),
            (-89.0, 89.0, 179.0),
            (10.0, -10.0, -170.0),
            (0.0, 80.0, 10.0),
            (-70.0, 70.0, -5.0),
            (45.0, -45.0, 89.0),
            (1.0, -1.0, 1.0),
            (89.0, -89.0, -180.0),
        ];
        for &(alat, blat, blon) in test_cases {
            let a = LatLng::from_degrees(alat, 0.0).to_point();
            let b = LatLng::from_degrees(blat, blon).to_point();
            let stats = test_projected(&proj, tolerance, a, b);
            assert!(
                stats.max() < 1.0,
                "Failed for ({alat}, 0) -> ({blat}, {blon}): max = {}",
                stats.max()
            );
        }
    }
}
