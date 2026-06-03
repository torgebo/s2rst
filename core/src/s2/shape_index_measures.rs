// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Geometric measures (dimension, length, perimeter, area, centroid) for
//! [`ShapeIndex`] objects.
//!
//! These functions return the sum of the corresponding measure for all shapes
//! in the index. For centroid computation, only shapes of the maximal
//! dimension contribute.
//!
//! Corresponds to C++ `s2shape_index_measures.h/cc`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "shape iteration (usize->i32) — count in i32 range"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i32 for ShapeId iteration — always in range"
)]
use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::shape::Dimension;
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_measures;

/// Returns the maximum dimension of any shape in the index, or `None` if empty.
pub fn get_dimension(index: &ShapeIndex) -> Option<Dimension> {
    let mut dim: Option<Dimension> = None;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32) {
            let d = shape.dimension();
            dim = Some(match dim {
                Some(prev) if prev >= d => prev,
                _ => d,
            });
        }
    }
    dim
}

/// Returns the number of points (dimension-0 edges) in the index.
pub fn get_num_points(index: &ShapeIndex) -> usize {
    let mut count = 0;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32)
            && shape.dimension() == Dimension::Point
        {
            count += shape.num_edges();
        }
    }
    count
}

/// Returns the total length of all polylines in the index.
pub fn get_length(index: &ShapeIndex) -> Angle {
    let mut length = Angle::ZERO;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32) {
            length = length + shape_measures::get_length(shape);
        }
    }
    length
}

/// Returns the total perimeter of all polygons in the index.
pub fn get_perimeter(index: &ShapeIndex) -> Angle {
    let mut perimeter = Angle::ZERO;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32) {
            perimeter = perimeter + shape_measures::get_perimeter(shape);
        }
    }
    perimeter
}

/// Returns the total area of all polygons in the index.
pub fn get_area(index: &ShapeIndex) -> f64 {
    let mut area = 0.0;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32) {
            area += shape_measures::get_area(shape);
        }
    }
    area
}

/// Like [`get_area`], but faster with more error.
pub fn get_approx_area(index: &ShapeIndex) -> f64 {
    let mut area = 0.0;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32) {
            area += shape_measures::get_approx_area(shape);
        }
    }
    area
}

/// Returns the centroid of shapes with maximal dimension, scaled by measure.
///
/// Only shapes whose dimension equals the maximum dimension in the index
/// contribute. Points and polylines are ignored if polygons are present.
pub fn get_centroid(index: &ShapeIndex) -> Point {
    let Some(dim) = get_dimension(index) else {
        return Point(crate::r3::Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    };
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut cz = 0.0;
    for i in 0..index.num_shape_ids() {
        if let Some(shape) = index.shape(i as i32)
            && shape.dimension() == dim
        {
            let c = shape_measures::get_centroid(shape);
            cx += c.0.x;
            cy += c.0.y;
            cz += c.0.z;
        }
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
    use std::f64::consts::PI;

    #[test]
    fn test_get_dimension_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_dimension(&index), None);
    }

    #[test]
    fn test_get_dimension_points() {
        let index = text_format::make_index("0:0 # #");
        assert_eq!(get_dimension(&index), Some(Dimension::Point));
    }

    #[test]
    fn test_get_dimension_points_and_lines() {
        let index = text_format::make_index("0:0 # 1:1, 1:2 #");
        assert_eq!(get_dimension(&index), Some(Dimension::Polyline));
    }

    #[test]
    fn test_get_dimension_points_lines_and_polygons() {
        let index = text_format::make_index("0:0 # 1:1, 2:2 # 3:3, 3:4, 4:3");
        assert_eq!(get_dimension(&index), Some(Dimension::Polygon));

        let index = text_format::make_index("# # empty");
        assert_eq!(get_dimension(&index), Some(Dimension::Polygon));
    }

    #[test]
    fn test_get_num_points_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_num_points(&index), 0);
    }

    #[test]
    fn test_get_num_points_two_points() {
        let index = text_format::make_index("0:0 | 1:0 # #");
        assert_eq!(get_num_points(&index), 2);
    }

    #[test]
    fn test_get_num_points_line_and_polygon() {
        let index = text_format::make_index("# 1:1, 1:2 # 0:3, 0:5, 2:5");
        assert_eq!(get_num_points(&index), 0);
    }

    #[test]
    fn test_get_length_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_length(&index).radians(), 0.0);
    }

    #[test]
    fn test_get_length_two_lines() {
        let index = text_format::make_index("4:4 # 0:0, 1:0 | 1:0, 2:0 # 5:5, 5:6, 6:5");
        assert!((get_length(&index).degrees() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_perimeter_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_perimeter(&index).radians(), 0.0);
    }

    #[test]
    fn test_get_perimeter_degenerate_polygon() {
        let index = text_format::make_index("4:4 # 0:0, 1:0 | 2:0, 3:0 # 0:1, 0:2, 0:3");
        assert!((get_perimeter(&index).degrees() - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_area_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_area(&index), 0.0);
    }

    #[test]
    fn test_get_area_two_full_polygons() {
        let index = text_format::make_index("# # full | full");
        assert!((get_area(&index) - 8.0 * PI).abs() < 1e-10);
    }

    #[test]
    fn test_get_approx_area_empty() {
        let index = text_format::make_index("# #");
        assert_eq!(get_approx_area(&index), 0.0);
    }

    #[test]
    fn test_get_approx_area_two_full_polygons() {
        let index = text_format::make_index("# # full | full");
        assert!((get_approx_area(&index) - 8.0 * PI).abs() < 1e-10);
    }

    #[test]
    fn test_get_centroid_empty() {
        let index = text_format::make_index("# #");
        let c = get_centroid(&index);
        assert_eq!(c.0.x, 0.0);
        assert_eq!(c.0.y, 0.0);
        assert_eq!(c.0.z, 0.0);
    }

    #[test]
    fn test_get_centroid_points() {
        let index = text_format::make_index("0:0 | 0:90 # #");
        let c = get_centroid(&index);
        assert!((c.0.x - 1.0).abs() < 1e-15);
        assert!((c.0.y - 1.0).abs() < 1e-15);
        assert!(c.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_get_centroid_polyline() {
        // Points are ignored when polylines are present.
        let index = text_format::make_index("5:5 | 6:6 # 0:0, 0:90 #");
        let c = get_centroid(&index);
        assert!((c.0.x - 1.0).abs() < 1e-15);
        assert!((c.0.y - 1.0).abs() < 1e-15);
        assert!(c.0.z.abs() < 1e-15);
    }

    #[test]
    fn test_get_centroid_polygon() {
        // Points and polylines are ignored when polygons are present.
        let index = text_format::make_index("5:5 # 6:6, 7:7 # 0:0, 0:90, 90:0");
        let c = get_centroid(&index);
        assert!((c.0.x - PI / 4.0).abs() < 1e-15);
        assert!((c.0.y - PI / 4.0).abs() < 1e-15);
        assert!((c.0.z - PI / 4.0).abs() < 1e-15);
    }
}
