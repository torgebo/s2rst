// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! True centroid, surface centroid, and edge centroid computations.
//!
//! The *true centroid* (mass centroid) is the surface integral of (x,y,z)
//! divided by the area. Unlike the planar or surface centroid, it behaves
//! linearly when regions are added or subtracted.
//!
//! Corresponds to Go `s2/centroids.go`, C++ `s2centroids.cc`.

use crate::r3::Vector;
use crate::s2::Point;

/// Returns the true centroid of the spherical triangle ABC multiplied by
/// the signed area of the triangle.
///
/// Multiplying by the signed area makes it easy to compute the centroid
/// of a union or difference of triangles: just sum the results and
/// normalize.
///
/// Returns `Point(0, 0, 0)` for degenerate triangles.
/// All points must have unit length.
pub fn true_centroid(a: Point, b: Point, c: Point) -> Point {
    let mut ra = 1.0;
    let sa = b.distance(c).radians();
    if sa != 0.0 {
        ra = sa / sa.sin();
    }
    let mut rb = 1.0;
    let sb = c.distance(a).radians();
    if sb != 0.0 {
        rb = sb / sb.sin();
    }
    let mut rc = 1.0;
    let sc = a.distance(b).radians();
    if sc != 0.0 {
        rc = sc / sc.sin();
    }

    // Solve a 3×3 system via Cramer's rule. We subtract the first row (A)
    // from the other two to reduce cancellation error when A, B, C are
    // very close together.
    let x = Vector {
        x: a.0.x,
        y: b.0.x - a.0.x,
        z: c.0.x - a.0.x,
    };
    let y = Vector {
        x: a.0.y,
        y: b.0.y - a.0.y,
        z: c.0.y - a.0.y,
    };
    let z = Vector {
        x: a.0.z,
        y: b.0.z - a.0.z,
        z: c.0.z - a.0.z,
    };
    let r = Vector {
        x: ra,
        y: rb - ra,
        z: rc - ra,
    };

    Point(
        Vector {
            x: y.cross(z).dot(r),
            y: z.cross(x).dot(r),
            z: x.cross(y).dot(r),
        } * 0.5,
    )
}

/// Returns the true centroid of the spherical geodesic edge AB
/// multiplied by the length of the edge.
///
/// The true centroid of a collection of edges (e.g. a polyline) can be
/// computed by summing the result of this function for each edge.
///
/// Returns `Point(0, 0, 0)` if the edge is degenerate.
pub fn edge_true_centroid(a: Point, b: Point) -> Point {
    let v_diff = a.0 - b.0; // length == 2·sin(θ)
    let v_sum = a.0 + b.0; // length == 2·cos(θ)
    let sin2 = v_diff.norm2();
    let cos2 = v_sum.norm2();
    if cos2 == 0.0 {
        return Point(Vector::default()); // Antipodal edges.
    }
    Point(v_sum * (sin2 / cos2).sqrt()) // length == 2·sin(θ)
}

/// Returns the centroid of the planar triangle ABC (not the spherical
/// centroid).
///
/// This can be normalized to unit length to obtain the "surface centroid"
/// of the corresponding spherical triangle, i.e. the intersection of
/// the three medians. For large spherical triangles the surface centroid
/// may be nowhere near the intuitive "center".
pub fn planar_centroid(a: Point, b: Point, c: Point) -> Point {
    Point((a.0 + b.0 + c.0) * (1.0 / 3.0))
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::LatLng;

    fn p(lat: f64, lng: f64) -> Point {
        LatLng::from_degrees(lat, lng).to_point()
    }

    #[test]
    fn test_true_centroid_equilateral() {
        // An equilateral triangle on the sphere.
        let a = p(90.0, 0.0);
        let b = p(0.0, 0.0);
        let c = p(0.0, 90.0);
        let centroid = true_centroid(a, b, c);
        // The centroid should point roughly toward (1,1,1)/√3.
        assert!(centroid.0.x > 0.0 && centroid.0.y > 0.0 && centroid.0.z > 0.0);
    }

    #[test]
    fn test_true_centroid_degenerate() {
        // Degenerate triangle (all points the same).
        let a = p(0.0, 0.0);
        let centroid = true_centroid(a, a, a);
        assert!(
            centroid.0.norm() < 1e-14,
            "degenerate centroid should be near zero: {}",
            centroid.0.norm()
        );
    }

    #[test]
    fn test_true_centroid_additive() {
        // The true centroid should be additive: splitting a triangle into
        // two sub-triangles and summing centroids should give the original.
        let a = p(0.0, 0.0);
        let b = p(0.0, 90.0);
        let c = p(90.0, 0.0);
        let m = Point((b.0 + c.0).normalize()); // midpoint of BC

        let c_abc = true_centroid(a, b, c);
        let c_abm = true_centroid(a, b, m);
        let c_amc = true_centroid(a, m, c);
        let c_sum = Point(c_abm.0 + c_amc.0);

        let diff = (c_abc.0 - c_sum.0).norm();
        assert!(
            diff < 1e-10,
            "true_centroid should be additive: diff = {diff}"
        );
    }

    #[test]
    fn test_edge_true_centroid() {
        let a = p(0.0, 0.0);
        let b = p(0.0, 90.0);
        let centroid = edge_true_centroid(a, b);
        // Should point toward the midpoint of the edge.
        assert!(centroid.0.x > 0.0);
        assert!(centroid.0.y > 0.0);
        assert!(centroid.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_edge_true_centroid_degenerate() {
        let a = p(0.0, 0.0);
        let centroid = edge_true_centroid(a, a);
        // Zero-length edge should have zero centroid.
        assert!(centroid.0.norm() < 1e-14);
    }

    #[test]
    fn test_planar_centroid() {
        let a = p(0.0, 0.0);
        let b = p(0.0, 90.0);
        let c = p(90.0, 0.0);
        let centroid = planar_centroid(a, b, c);
        // Should be roughly (a+b+c)/3.
        let expected = (a.0 + b.0 + c.0) * (1.0 / 3.0);
        assert!((centroid.0 - expected).norm() < 1e-15);
    }

    #[test]
    fn test_edge_true_centroid_great_circles() {
        // C++ EdgeTrueCentroid::GreatCircles: random great circles divided into
        // random segments should have centroid near the origin.
        use crate::s2::testing::random_frame;
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let mut rng = StdRng::seed_from_u64(42);
        for _ in 0..100 {
            let (x, y, _z) = random_frame(&mut rng);
            let mut centroid = Vector::default();
            let mut v0 = x;
            let mut theta = 0.0;
            while theta < 2.0 * std::f64::consts::PI {
                let v1 = Point(x.0 * theta.cos() + y.0 * theta.sin());
                centroid = centroid + edge_true_centroid(v0, v1).0;
                v0 = v1;
                theta += rng.r#gen::<f64>().powf(10.0);
            }
            // Close the circle.
            centroid = centroid + edge_true_centroid(v0, x).0;
            assert!(
                centroid.norm() <= 2e-14,
                "great circle centroid should be near origin: norm={}",
                centroid.norm()
            );
        }
    }
}
