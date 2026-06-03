// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Measures for polylines on the unit sphere.
//!
//! These are low-level methods that work directly with slices of [`Point`]s.
//! They are used to implement methods in [`shape_measures`](super::shape_measures)
//! and [`polyline`](super::polyline).
//!
//! Corresponds to C++ `s2polyline_measures.h/cc`.

use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::centroids;

/// Returns the length of the polyline. Returns zero for polylines with
/// fewer than two vertices.
pub fn get_length(polyline: &[Point]) -> Angle {
    let mut length = 0.0;
    for i in 1..polyline.len() {
        length += polyline[i - 1].distance(polyline[i]).radians();
    }
    Angle::from_radians(length)
}

/// Returns the true centroid of the polyline multiplied by the length.
///
/// The result is not unit length. Scaling by the polyline length makes it
/// easy to compute centroids of multiple polylines (by adding centroids).
///
/// Returns the zero point for degenerate polylines (e.g., AA).
pub fn get_centroid(polyline: &[Point]) -> Point {
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for i in 1..polyline.len() {
        let c = centroids::edge_true_centroid(polyline[i - 1], polyline[i]);
        cx += c.0.x;
        cy += c.0.y;
        cz += c.0.z;
    }
    Point(crate::r3::Vector {
        x: cx,
        y: cy,
        z: cz,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::text_format;

    #[test]
    fn test_get_length_empty() {
        assert_eq!(get_length(&[]).radians(), 0.0);
    }

    #[test]
    fn test_get_length_single_point() {
        let pts = text_format::parse_points("0:0");
        assert_eq!(get_length(&pts).radians(), 0.0);
    }

    #[test]
    fn test_get_length_equator_90() {
        let pts = text_format::parse_points("0:0, 0:90");
        assert!((get_length(&pts).radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-14);
    }

    #[test]
    fn test_get_length_three_segments() {
        let pts = text_format::parse_points("0:0, 0:1, 0:2, 0:3");
        let expected = Angle::from_degrees(3.0);
        assert!((get_length(&pts).radians() - expected.radians()).abs() < 1e-14);
    }

    #[test]
    fn test_get_centroid_empty() {
        let c = get_centroid(&[]);
        assert_eq!(c.0.x, 0.0);
        assert_eq!(c.0.y, 0.0);
        assert_eq!(c.0.z, 0.0);
    }

    #[test]
    fn test_get_centroid_single_edge() {
        let pts = text_format::parse_points("0:0, 0:90");
        let c = get_centroid(&pts);
        // Centroid of edge from (1,0,0) to (0,1,0) should be in the direction (1,1,0).
        assert!(c.0.x > 0.0);
        assert!(c.0.y > 0.0);
        assert!(c.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_get_centroid_symmetric() {
        // A symmetric polyline should have centroid at the midpoint direction.
        let pts = text_format::parse_points("0:-10, 0:0, 0:10");
        let c = get_centroid(&pts);
        // Should be along (1,0,0) direction.
        assert!(c.0.x > 0.0);
        assert!(c.0.y.abs() < 1e-15);
        assert!(c.0.z.abs() < 1e-15);
    }
}
