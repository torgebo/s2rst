// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Test utilities for generating random S2 geometry.
//!
//! Provides functions for generating random points, caps, cell ids, and
//! composite geometry for use in property-based and fuzz testing.
//!
//! Corresponds to C++ `s2random.{h,cc}` and parts of `s2testing.{h,cc}`.

#![expect(
    clippy::cast_possible_truncation,
    reason = "fractal dimension and level values — bounded by construction"
)]
#![cfg_attr(
    test,
    expect(
        clippy::cast_possible_wrap,
        reason = "usize -> i32/i64 for test helper values — always small"
    )
)]
use rand::Rng;

use crate::r3::Vector;
use crate::s2::coords::{Level, MAX_CELL_LEVEL};
use crate::s2::point::{from_frame, get_frame};
use crate::s2::{Cap, CellId, Loop, Point, Polygon};

/// Returns a random unit-length point on the sphere.
pub fn random_point(rng: &mut impl Rng) -> Point {
    let x: f64 = rng.gen_range(-1.0..=1.0);
    let y: f64 = rng.gen_range(-1.0..=1.0);
    let z: f64 = rng.gen_range(-1.0..=1.0);
    Point::from_coords(x, y, z)
}

/// Returns a right-handed coordinate frame (three orthonormal vectors).
pub fn random_frame(rng: &mut impl Rng) -> (Point, Point, Point) {
    let z = random_point(rng);
    frame_at(rng, z)
}

/// Given a unit-length z-axis, computes x- and y-axes such that (x, y, z)
/// is a right-handed coordinate frame.
pub fn frame_at(rng: &mut impl Rng, z: Point) -> (Point, Point, Point) {
    use crate::s2::edge_crossings::robust_cross_prod;
    let r = random_point(rng);
    let x = Point(robust_cross_prod(z, r).0.normalize());
    let y = Point(robust_cross_prod(z, x).0.normalize());
    (x, y, z)
}

/// Returns a cap with a random axis such that the log of its area is
/// uniformly distributed between the logs of `min_area` and `max_area`.
///
/// # Panics
/// Panics if `min_area <= 0`, `max_area > 4*PI`, or `min_area > max_area`.
pub fn random_cap(rng: &mut impl Rng, min_area: f64, max_area: f64) -> Cap {
    assert!(min_area > 0.0);
    assert!(max_area <= 4.0 * std::f64::consts::PI);
    assert!(min_area <= max_area);
    let exponent: f64 = rng.gen_range(0.0..=1.0);
    let cap_area = max_area * (min_area / max_area).powf(exponent);
    Cap::from_center_area(random_point(rng), cap_area)
}

/// Returns a point chosen uniformly at random (with respect to area) from
/// the given cap.
///
/// # Panics
/// Panics if the cap is empty.
pub fn sample_point_from_cap(rng: &mut impl Rng, cap: &Cap) -> Point {
    assert!(!cap.is_empty());

    let m = get_frame(cap.center());
    let h: f64 = rng.gen_range(0.0..=cap.height());
    let theta: f64 = rng.gen_range(0.0..(2.0 * std::f64::consts::PI));
    let r = (h * (2.0 - h)).sqrt();
    let p = Point(Vector::new(theta.cos() * r, theta.sin() * r, 1.0 - h).normalize());
    from_frame(&m, p)
}

/// Returns a random cell id at the given level. The distribution is uniform
/// over the space of cell ids, but only approximately uniform over the sphere.
pub fn random_cell_id_at_level(rng: &mut impl Rng, level: impl Into<Level>) -> CellId {
    let face: u8 = rng.gen_range(0..6);
    let pos: u64 = rng.gen_range(0..(1u64 << (2 * u32::from(MAX_CELL_LEVEL) + 1)));
    CellId::from_face_pos_level(face, pos, level)
}

/// Returns a random cell id at a randomly chosen level.
pub fn random_cell_id(rng: &mut impl Rng) -> CellId {
    let level: u8 = rng.gen_range(0..=MAX_CELL_LEVEL);
    random_cell_id_at_level(rng, level)
}

/// Returns a random number skewed towards smaller values. First a `base` is
/// picked uniformly from `[0, max_log]` and then `base` random bits are
/// returned.
///
/// # Panics
///
/// Panics if `max_log >= 32`.
pub fn skewed_int(rng: &mut impl Rng, max_log: u32) -> u32 {
    assert!(max_log < 32);
    let base: u32 = rng.gen_range(0..=max_log);
    rng.gen_range(0..(1u32 << base))
}

/// Creates a polygon with `num_loops` concentric loops centered at `center`,
/// each with `num_vertices_per_loop` vertices.
pub fn concentric_loops_polygon(
    center: Point,
    num_loops: usize,
    num_vertices_per_loop: usize,
) -> Polygon {
    let m = get_frame(center);
    let mut loops = Vec::with_capacity(num_loops);
    for li in 0..num_loops {
        let radius = 0.005 * (li + 1) as f64 / num_loops as f64;
        let radian_step = 2.0 * std::f64::consts::PI / num_vertices_per_loop as f64;
        let mut vertices = Vec::with_capacity(num_vertices_per_loop);
        for vi in 0..num_vertices_per_loop {
            let angle = vi as f64 * radian_step;
            let p = Point(Vector::new(radius * angle.cos(), radius * angle.sin(), 1.0).normalize());
            vertices.push(from_frame(&m, p));
        }
        loops.push(Loop::new(vertices));
    }
    Polygon::from_loops(loops)
}

/// Recursively checks that a covering of `region` is valid.
///
/// If `check_tight` is true, checks that the covering does not contain any
/// cells that don't intersect the region. If `id` is `None`, starts from
/// all 6 faces.
///
/// # Panics
///
/// Panics if a tight covering contains a cell that doesn't intersect the
/// region.
pub fn check_covering(
    region: &dyn crate::s2::Region,
    covering: &crate::s2::CellUnion,
    check_tight: bool,
    id: Option<CellId>,
) {
    let Some(id) = id else {
        for face in 0..6u8 {
            check_covering(region, covering, check_tight, Some(CellId::from_face(face)));
        }
        return;
    };

    if !region.intersects_cell(&crate::s2::Cell::from(id)) {
        if check_tight {
            assert!(
                !covering.intersects_cell_id(id),
                "%.covering intersects non-intersecting cell {id:?}"
            );
        }
    } else if !covering.contains_cell_id(id) {
        assert!(
            !region.contains_cell(&crate::s2::Cell::from(id)),
            "covering doesn't contain region-contained cell {id:?}"
        );
        assert!(!id.is_leaf(), "covering doesn't contain leaf cell {id:?}");
        let mut child = id.child_begin();
        let end = id.child_end();
        while child != end {
            check_covering(region, covering, check_tight, Some(child));
            child = child.next();
        }
    }
}

/// Generates `num_points` regularly spaced points on a circle of given
/// `radius` centered at `center`. Matches C++ `S2Testing::MakeRegularPoints`.
///
/// Uses the deterministic frame from [`get_frame`](crate::s2::point::get_frame)
/// (not a random frame), matching the C++ implementation.
pub fn make_regular_points(
    center: Point,
    radius: crate::s1::Angle,
    num_points: usize,
) -> Vec<Point> {
    use crate::s2::point::{from_frame, get_frame};
    use std::f64::consts::PI;
    let mat = get_frame(center);
    let r = radius.radians();
    let mut points = Vec::with_capacity(num_points);
    for i in 0..num_points {
        let angle = 2.0 * PI * (i as f64) / (num_points as f64);
        let p = Point::from_coords(r * angle.cos(), r * angle.sin(), 1.0).normalize();
        points.push(from_frame(&mat, p));
    }
    points
}

/// Adds approximately `num_edges` fractal loop edges to the given
/// `ShapeIndex`. Matches C++ `s2testing::FractalLoopShapeIndexFactory`.
pub fn add_fractal_loop_edges(
    cap: &Cap,
    num_edges: usize,
    rng: &mut impl Rng,
    index: &mut crate::s2::shape_index::ShapeIndex,
) {
    use crate::r3::matrix::Matrix3x3;
    use crate::s2::fractal::S2Fractal;
    use crate::s2::lax_polygon::LaxPolygon;
    let mut fractal = S2Fractal::new(rng.r#gen::<u64>());
    fractal.level_for_approx_max_edges(num_edges as i32);
    fractal.set_fractal_dimension(1.5);
    let (x, y, z) = frame_at(rng, cap.center());
    let mat = Matrix3x3::from_cols(x.0, y.0, z.0);
    let lp = fractal.make_loop(&mat, cap.angle_radius());
    let vertices: Vec<Point> = lp.vertices().to_vec();
    let vslice: Vec<&[Point]> = vec![&vertices];
    index.add(Box::new(LaxPolygon::from_loops(&vslice)));
}

/// Checks that `actual` distance results are consistent with `expected`
/// brute-force results given the query parameters. Returns `true` if they
/// match.
///
/// Works with any result type `(ChordAngle, D)` where `D: Eq`.
/// Matches C++ `s2testing::CheckDistanceResults`.
///
/// # Panics
///
/// Panics if `expected` or `actual` slices have internal inconsistencies
/// (should not happen with well-formed inputs).
#[expect(clippy::print_stderr, reason = "diagnostic output for test failures")]
pub fn check_distance_results<D: Eq + std::fmt::Debug>(
    expected: &[(crate::s1::ChordAngle, D)],
    actual: &[(crate::s1::ChordAngle, D)],
    max_results: i32,
    max_distance: crate::s1::ChordAngle,
    max_error: crate::s1::ChordAngle,
) -> bool {
    use crate::s1::ChordAngle;
    const MAX_PRUNING_ERROR: f64 = 1e-15;

    // Both should be sorted by distance.
    for (label, slice) in [("expected", expected), ("actual", actual)] {
        for w in slice.windows(2) {
            if w[1].0 < w[0].0 {
                eprintln!("{label} not sorted: {:?} > {:?}", w[0].0, w[1].0);
                return false;
            }
        }
    }

    let check_result_set = |x: &[(ChordAngle, D)], y: &[(ChordAngle, D)], label: &str| -> bool {
        let limit = if (x.len() as i32) < max_results {
            if max_distance == ChordAngle::INFINITY {
                ChordAngle::INFINITY
            } else {
                ChordAngle::from_length2((max_distance.length2() - MAX_PRUNING_ERROR).max(0.0))
            }
        } else if !x.is_empty() {
            let back = x.last().unwrap().0.length2();
            ChordAngle::from_length2((back - max_error.length2() - MAX_PRUNING_ERROR).max(0.0))
        } else {
            ChordAngle::ZERO
        };

        for yp in y {
            if yp.0 >= limit {
                break;
            }
            let found = x.iter().any(|xp| xp.1 == yp.1);
            if !found {
                eprintln!(
                    "{label}: missing result with data={:?} distance={:?} (limit={limit:?})",
                    yp.1, yp.0
                );
                return false;
            }
        }
        true
    };

    check_result_set(expected, actual, "expected⊇actual")
        && check_result_set(actual, expected, "actual⊇expected")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn make_rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(42)
    }

    #[test]
    fn test_random_point_unit_length() {
        let mut rng = make_rng();
        for _ in 0..100 {
            let p = random_point(&mut rng);
            let norm = p.0.norm();
            assert!(
                (norm - 1.0).abs() < 1e-14,
                "point norm = {norm}, expected ~1.0"
            );
        }
    }

    #[test]
    fn test_random_frame_orthonormal() {
        let mut rng = make_rng();
        for _ in 0..100 {
            let (x, y, z) = random_frame(&mut rng);
            assert!((x.0.dot(y.0)).abs() < 1e-14);
            assert!((x.0.dot(z.0)).abs() < 1e-14);
            assert!((y.0.dot(z.0)).abs() < 1e-14);
            assert!((x.0.norm() - 1.0).abs() < 1e-14);
            assert!((y.0.norm() - 1.0).abs() < 1e-14);
            assert!((z.0.norm() - 1.0).abs() < 1e-14);
        }
    }

    #[test]
    fn test_random_cap_area_in_range() {
        let mut rng = make_rng();
        let min_area = 1e-6;
        let max_area = 4.0 * std::f64::consts::PI;
        for _ in 0..100 {
            let cap = random_cap(&mut rng, min_area, max_area);
            let area = cap.area();
            assert!(area >= min_area * 0.9, "area {area} < min {min_area}");
            assert!(area <= max_area * 1.1, "area {area} > max {max_area}");
        }
    }

    #[test]
    fn test_sample_point_inside_cap() {
        let mut rng = make_rng();
        let cap = random_cap(&mut rng, 0.01, 1.0);
        for _ in 0..100 {
            let p = sample_point_from_cap(&mut rng, &cap);
            assert!(cap.contains_point(p), "sampled point not in cap");
        }
    }

    #[test]
    fn test_random_cell_id_valid() {
        let mut rng = make_rng();
        for _ in 0..100 {
            let id = random_cell_id(&mut rng);
            assert!(id.is_valid());
        }
    }

    #[test]
    fn test_random_cell_id_at_level() {
        let mut rng = make_rng();
        for level in 0..=MAX_CELL_LEVEL {
            let id = random_cell_id_at_level(&mut rng, level);
            assert!(id.is_valid());
            assert_eq!(id.level(), level);
        }
    }

    #[test]
    fn test_concentric_loops_polygon() {
        let center = Point::from_coords(1.0, 0.0, 0.0);
        let polygon = concentric_loops_polygon(center, 3, 20);
        assert_eq!(polygon.num_loops(), 3);
        for i in 0..3 {
            assert_eq!(polygon.loop_at(i).num_vertices(), 20);
        }
    }

    #[test]
    fn test_skewed_int() {
        let mut rng = make_rng();
        for _ in 0..100 {
            let val = skewed_int(&mut rng, 10);
            assert!(val < 1024);
        }
    }
}
