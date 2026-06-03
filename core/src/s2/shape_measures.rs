// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Geometric measures (length, perimeter, area, centroid) for [`Shape`] objects.
//!
//! These functions work on the abstract [`Shape`] trait, allowing measures to
//! be computed for any geometry regardless of its underlying representation.
//!
//! Corresponds to C++ `s2shape_measures.h/cc`.

use std::f64::consts::PI;

use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::loop_measures;
use crate::s2::polyline_measures;
use crate::s2::shape::{Dimension, Shape};

/// Extracts the vertices of a chain from a shape.
///
/// For dimension 1 (polyline) chains, the result has `chain.length + 1` vertices.
/// For dimension 2 (polygon) chains, the result has `chain.length` vertices.
pub fn get_chain_vertices(shape: &dyn Shape, chain_id: usize) -> Vec<Point> {
    let chain = shape.chain(chain_id);
    let num_vertices = chain.length
        + if shape.dimension() == Dimension::Polyline {
            1
        } else {
            0
        };
    let mut vertices = Vec::with_capacity(num_vertices);
    let mut e = 0;
    if num_vertices & 1 != 0 {
        vertices.push(shape.chain_edge(chain_id, e).v0);
        e += 1;
    }
    while e < num_vertices {
        let edge = shape.chain_edge(chain_id, e);
        vertices.push(edge.v0);
        vertices.push(edge.v1);
        e += 2;
    }
    vertices
}

/// For shapes of dimension 1, returns the sum of all polyline lengths.
/// Otherwise returns zero.
pub fn get_length(shape: &dyn Shape) -> Angle {
    if shape.dimension() != Dimension::Polyline {
        return Angle::ZERO;
    }
    let mut length = Angle::ZERO;
    for chain_id in 0..shape.num_chains() {
        let vertices = get_chain_vertices(shape, chain_id);
        length = length + polyline_measures::get_length(&vertices);
    }
    length
}

/// For shapes of dimension 2, returns the sum of all loop perimeters.
/// Otherwise returns zero.
pub fn get_perimeter(shape: &dyn Shape) -> Angle {
    if shape.dimension() != Dimension::Polygon {
        return Angle::ZERO;
    }
    let mut perimeter = Angle::ZERO;
    for chain_id in 0..shape.num_chains() {
        let vertices = get_chain_vertices(shape, chain_id);
        perimeter = perimeter + loop_measures::get_perimeter(&vertices);
    }
    perimeter
}

/// For shapes of dimension 2, returns the area on the unit sphere.
/// The result is between 0 and 4*PI steradians. Otherwise returns zero.
pub fn get_area(shape: &dyn Shape) -> f64 {
    if shape.dimension() != Dimension::Polygon {
        return 0.0;
    }
    let mut area = 0.0;
    for chain_id in 0..shape.num_chains() {
        let vertices = get_chain_vertices(shape, chain_id);
        area += loop_measures::get_signed_area(&vertices);
    }
    // The signed area sum should be in [-4*pi, 4*pi] (with small error).
    debug_assert!(area.abs() <= 4.0 * PI + 1e-10);
    if area < 0.0 {
        area += 4.0 * PI;
    }
    area
}

/// Like [`get_area`], but faster with more error.
pub fn get_approx_area(shape: &dyn Shape) -> f64 {
    if shape.dimension() != Dimension::Polygon {
        return 0.0;
    }
    let mut area = 0.0;
    for chain_id in 0..shape.num_chains() {
        let vertices = get_chain_vertices(shape, chain_id);
        area += loop_measures::get_approx_area(&vertices);
    }
    // Handle full polygons.
    if area <= 4.0 * PI {
        area
    } else {
        area % (4.0 * PI)
    }
}

/// Returns the centroid of the shape multiplied by its measure.
///
/// - For dimension 0: measure = `num_edges` (count of points)
/// - For dimension 1: measure = length
/// - For dimension 2: measure = area
pub fn get_centroid(shape: &dyn Shape) -> Point {
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    let dim = shape.dimension();
    for chain_id in 0..shape.num_chains() {
        let c = match dim {
            Dimension::Point => shape.edge(chain_id).v0,
            Dimension::Polyline => {
                let vertices = get_chain_vertices(shape, chain_id);
                polyline_measures::get_centroid(&vertices)
            }
            Dimension::Polygon => {
                let vertices = get_chain_vertices(shape, chain_id);
                loop_measures::get_centroid(&vertices)
            }
        };
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
    use crate::s2::edge_vector_shape::EdgeVectorShape;
    use crate::s2::lax_polyline::LaxPolyline;
    use crate::s2::text_format;
    // Tests that GetLength returns zero for wrong dimensions.
    #[test]
    fn test_get_length_wrong_dimension() {
        // Dimension 0 (point) — should return zero.
        let index = text_format::make_index("0:0 # #");
        assert_eq!(get_length(index.shape(0).unwrap()).radians(), 0.0);

        // Dimension 2 (polygon) — should return zero.
        let poly = text_format::make_lax_polygon("0:0, 0:1, 1:0");
        assert_eq!(get_length(&poly).radians(), 0.0);
    }

    #[test]
    fn test_get_length_empty_polyline() {
        let shape = LaxPolyline::new(vec![]);
        assert_eq!(get_length(&shape).radians(), 0.0);
    }

    #[test]
    fn test_get_length_three_polylines() {
        let pts = text_format::parse_points("0:0, 1:0, 2:0, 3:0");
        let shape =
            EdgeVectorShape::from_edges(vec![(pts[0], pts[1]), (pts[0], pts[2]), (pts[0], pts[3])]);
        assert!((get_length(&shape).degrees() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_perimeter_wrong_dimension() {
        let index = text_format::make_index("0:0 # #");
        assert_eq!(get_perimeter(index.shape(0).unwrap()).radians(), 0.0);

        let shape = text_format::make_lax_polyline("0:0, 0:1, 1:0");
        assert_eq!(get_perimeter(&shape).radians(), 0.0);
    }

    #[test]
    fn test_get_perimeter_empty_polygon() {
        let shape = text_format::make_lax_polygon("empty");
        assert_eq!(get_perimeter(&shape).radians(), 0.0);
    }

    #[test]
    fn test_get_perimeter_full_polygon() {
        let shape = text_format::make_lax_polygon("full");
        assert_eq!(get_perimeter(&shape).radians(), 0.0);
    }

    #[test]
    fn test_get_perimeter_two_loop_polygon() {
        // Degenerate loops where all edges are 1 degree.
        let shape = text_format::make_lax_polygon("0:0, 1:0; 0:1, 0:2, 0:3");
        assert!((get_perimeter(&shape).degrees() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_area_wrong_dimension() {
        let index = text_format::make_index("0:0 # #");
        assert_eq!(get_area(index.shape(0).unwrap()), 0.0);

        let shape = text_format::make_lax_polyline("0:0, 0:1, 1:0");
        assert_eq!(get_area(&shape), 0.0);
    }

    #[test]
    fn test_get_area_empty_polygon() {
        let shape = text_format::make_lax_polygon("empty");
        assert_eq!(get_area(&shape), 0.0);
    }

    #[test]
    fn test_get_area_full_polygon() {
        let shape = text_format::make_lax_polygon("full");
        assert!((get_area(&shape) - 4.0 * PI).abs() < 1e-10);
    }

    #[test]
    fn test_get_area_two_tiny_shells() {
        let side = Angle::from_degrees(1e-10).radians();
        let shape = text_format::make_lax_polygon(
            "0:0, 0:1e-10, 1e-10:1e-10, 1e-10:0; \
             0:0, 0:-1e-10, -1e-10:-1e-10, -1e-10:0",
        );
        assert!((get_area(&shape) - 2.0 * side * side).abs() < 1e-30);
    }

    #[test]
    fn test_get_area_tiny_shell_and_hole() {
        let side = Angle::from_degrees(1e-10).radians();
        let shape = text_format::make_lax_polygon(
            "0:0, 0:2e-10, 2e-10:2e-10, 2e-10:0; \
             0.5e-10:0.5e-10, 1.5e-10:0.5e-10, 1.5e-10:1.5e-10, 0.5e-10:1.5e-10",
        );
        assert!((get_area(&shape) - 3.0 * side * side).abs() < 1e-30);
    }

    #[test]
    fn test_get_approx_area_large_polygon() {
        let shape = text_format::make_lax_polygon("0:0, 0:90, 90:0; 0:22.5, 90:0, 0:67.5");
        assert!((get_approx_area(&shape) - PI / 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_get_centroid_points() {
        let index = text_format::make_index("0:0 | 0:90 # #");
        let c = get_centroid(index.shape(0).unwrap());
        assert!((c.0.x - 1.0).abs() < 1e-15);
        assert!((c.0.y - 1.0).abs() < 1e-15);
        assert!(c.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_get_centroid_polyline() {
        let shape = text_format::make_lax_polyline("0:0, 0:90");
        let c = get_centroid(&shape);
        assert!((c.0.x - 1.0).abs() < 1e-15);
        assert!((c.0.y - 1.0).abs() < 1e-15);
        assert!(c.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_get_centroid_polygon() {
        let shape = text_format::make_lax_polygon("0:0, 0:90, 90:0");
        let c = get_centroid(&shape);
        assert!((c.0.x - PI / 4.0).abs() < 1e-15);
        assert!((c.0.y - PI / 4.0).abs() < 1e-15);
        assert!((c.0.z - PI / 4.0).abs() < 1e-15);
    }
}
