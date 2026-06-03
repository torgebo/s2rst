// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Area, angle, and turning-angle computations for spherical triangles.
//!
//! Corresponds to Go `s2/point_measures.go`, C++ `s2pointutil.cc`.

use crate::s1::Angle;
use crate::s2::Point;
use crate::s2::edge_crossings::robust_cross_prod;
use crate::s2::predicates;

/// Returns the area of the spherical triangle ABC.
///
/// This method combines l'Huilier's theorem (accurate for small triangles)
/// with Girard's formula (better for large, skinny triangles). The maximum
/// error is about 5e-15 (≈ 0.25 m² on the Earth's surface). All points
/// should be unit length and no two should be antipodal. The result is
/// always non-negative.
pub fn point_area(a: Point, b: Point, c: Point) -> f64 {
    let sa = b.stable_angle(c).radians();
    let sb = c.stable_angle(a).radians();
    let sc = a.stable_angle(b).radians();
    let s = 0.5 * (sa + sb + sc);
    if s >= 3e-4 {
        let dmin = s - sa.max(sb).max(sc);
        if dmin < 1e-2 * s * s * s * s * s {
            let area = girard_area(a, b, c);
            if dmin < s * 0.1 * (area + 5e-15) {
                return area;
            }
        }
    }
    // l'Huilier's formula.
    4.0 * (0.0f64.max(
        (0.5 * s).tan() * (0.5 * (s - sa)).tan() * (0.5 * (s - sb)).tan() * (0.5 * (s - sc)).tan(),
    ))
    .sqrt()
    .atan()
}

/// Returns the area of the triangle computed using Girard's formula.
///
/// About twice as fast as [`point_area`] but has poor relative accuracy
/// for small triangles. The maximum error is about 5e-15 (≈ 0.25 m²
/// on the Earth's surface). All points should be unit length and no two
/// should be antipodal.
pub fn girard_area(a: Point, b: Point, c: Point) -> f64 {
    // This is equivalent to the usual Girard's formula but is slightly more
    // accurate, faster to compute, and handles a == b == c without a special
    // case.  RobustCrossProd is necessary to get good accuracy when two of
    // the input points are very close together.
    let ab = robust_cross_prod(a, b);
    let bc = robust_cross_prod(b, c);
    let ac = robust_cross_prod(a, c);

    (ab.0.angle(ac.0) - ab.0.angle(bc.0) + bc.0.angle(ac.0)).max(0.0)
}

/// Returns a positive value for counterclockwise triangles and a
/// negative value otherwise.
pub fn signed_area(a: Point, b: Point, c: Point) -> f64 {
    f64::from(predicates::robust_sign(a, b, c) as i8) * point_area(a, b, c)
}

/// Returns the interior angle at vertex B in the triangle ABC.
///
/// The return value is always in the range [0, π]. All points should be
/// normalized. Ensures that `angle(a,b,c) == angle(c,b,a)` for all a,b,c.
///
/// The angle is undefined if A or C is diametrically opposite from B, and
/// becomes numerically unstable as the length of edge AB or BC approaches
/// 180 degrees.
pub fn angle(a: Point, b: Point, c: Point) -> Angle {
    // RobustCrossProd is necessary to get good accuracy when two of the input
    // points are very close together.
    Angle::from_radians(robust_cross_prod(a, b).0.angle(robust_cross_prod(c, b).0))
}

/// Returns the exterior angle at vertex B in the triangle ABC.
///
/// The return value is positive if ABC is counterclockwise and negative
/// otherwise. This quantity is also known as the "geodesic curvature" at B.
///
/// Ensures that `turn_angle(a,b,c) == -turn_angle(c,b,a)` for all distinct
/// a,b,c. The result is undefined if `a == b` or `b == c`, but is either
/// −π or π if `a == c`. All points should be normalized.
pub fn turn_angle(a: Point, b: Point, c: Point) -> Angle {
    // We use RobustCrossProd to get good accuracy when two points are very
    // close together, and Sign to ensure the sign is correct for turns
    // close to 180 degrees.
    let ang = robust_cross_prod(a, b).0.angle(robust_cross_prod(b, c).0);
    // Don't return Sign() * angle because it is legal to have (a == c).
    if predicates::robust_sign(a, b, c) == predicates::Direction::CounterClockwise {
        Angle::from_radians(ang)
    } else {
        Angle::from_radians(-ang)
    }
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
    fn test_point_area_right_triangle() {
        // A right triangle with two sides of π/2 at the north pole.
        let a = p(90.0, 0.0);
        let b = p(0.0, 0.0);
        let c = p(0.0, 90.0);
        let area = point_area(a, b, c);
        // Expected area = π/2 (one-eighth of the sphere).
        assert!(
            (area - std::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "area = {area}, expected π/2 ≈ {}",
            std::f64::consts::FRAC_PI_2,
        );
    }

    #[test]
    fn test_point_area_degenerate() {
        let a = p(0.0, 0.0);
        let b = p(0.0, 1.0);
        let c = p(0.0, 2.0);
        let area = point_area(a, b, c);
        assert!(area < 1e-14, "degenerate triangle area = {area}");
    }

    #[test]
    fn test_point_area_small() {
        // A very small triangle near the equator.
        let a = p(0.0, 0.0);
        let b = p(0.001, 0.0);
        let c = p(0.0, 0.001);
        let area = point_area(a, b, c);
        assert!(area > 0.0, "small triangle should have positive area");
        assert!(area < 1e-6, "small triangle area = {area} seems too large");
    }

    #[test]
    fn test_girard_area_right_triangle() {
        let a = p(90.0, 0.0);
        let b = p(0.0, 0.0);
        let c = p(0.0, 90.0);
        let area = girard_area(a, b, c);
        assert!(
            (area - std::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "girard area = {area}"
        );
    }

    #[test]
    fn test_signed_area() {
        let a = p(90.0, 0.0);
        let b = p(0.0, 0.0);
        let c = p(0.0, 90.0);
        let area_ccw = signed_area(a, b, c);
        let area_cw = signed_area(a, c, b);
        assert!(
            area_ccw > 0.0 || area_cw > 0.0,
            "one order should be positive"
        );
        assert!(
            (area_ccw.abs() - area_cw.abs()).abs() < 1e-10,
            "magnitudes should match"
        );
        // Signs should be opposite.
        assert!(area_ccw * area_cw <= 0.0, "signs should be opposite");
    }

    #[test]
    fn test_angle_right_angle() {
        let a = p(0.0, 0.0);
        let b = p(90.0, 0.0);
        let c = p(0.0, 90.0);
        let ang = angle(a, b, c);
        assert!(
            (ang.radians() - std::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "angle = {} radians",
            ang.radians(),
        );
    }

    #[test]
    fn test_angle_symmetric() {
        let a = p(10.0, 20.0);
        let b = p(30.0, 40.0);
        let c = p(50.0, 60.0);
        let ang1 = angle(a, b, c);
        let ang2 = angle(c, b, a);
        assert!(
            (ang1.radians() - ang2.radians()).abs() < 1e-15,
            "angle should be symmetric"
        );
    }

    #[test]
    fn test_turn_angle_antisymmetric() {
        let a = p(10.0, 20.0);
        let b = p(30.0, 40.0);
        let c = p(50.0, 60.0);
        let t1 = turn_angle(a, b, c);
        let t2 = turn_angle(c, b, a);
        assert!(
            (t1.radians() + t2.radians()).abs() < 1e-12,
            "turn_angle should be antisymmetric: {} + {} = {}",
            t1.radians(),
            t2.radians(),
            t1.radians() + t2.radians(),
        );
    }

    #[test]
    fn test_turn_angle_straight() {
        // Points along the equator should give turn_angle ≈ 0.
        let a = p(0.0, 0.0);
        let b = p(0.0, 1.0);
        let c = p(0.0, 2.0);
        let t = turn_angle(a, b, c);
        assert!(
            t.radians().abs() < 1e-12,
            "straight line turn_angle = {}",
            t.radians()
        );
    }

    #[test]
    fn test_area_regression_b229644268() {
        // C++ regression test: collinear triangle should have zero area.
        use crate::r3::Vector;
        let a = Point(Vector::new(
            -1.705_424_004_316_021_258e-01,
            -8.242_696_197_922_716_461e-01,
            5.399_026_611_737_816_062e-01,
        ));
        let b = Point(Vector::new(
            -1.706_078_905_422_188_652e-01,
            -8.246_067_119_418_969_416e-01,
            5.393_669_607_095_969_987e-01,
        ));
        let c = Point(Vector::new(
            -1.705_800_600_596_222_294e-01,
            -8.244_634_596_153_025_408e-01,
            5.395_947_061_167_500_891e-01,
        ));
        assert_eq!(point_area(a, b, c), 0.0);
    }

    // ═══════════════════════════════════════════════════════════════════
    // C++ s2measures_test.cc ports
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_angle_methods_cpp() {
        // C++ TEST(S2, AngleMethods)
        use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let pz = Point::from_coords(0.0, 0.0, 1.0);
        let p000 = Point::from_coords(1.0, 0.0, 0.0);
        let p045 = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let p180 = Point::from_coords(-1.0, 0.0, 0.0);

        let eps = 1e-12;
        assert!((angle(p000, pz, p045).radians() - FRAC_PI_4).abs() < eps);
        assert!((turn_angle(p000, pz, p045).radians() - (-3.0 * FRAC_PI_4)).abs() < eps);

        assert!((angle(p045, pz, p180).radians() - 3.0 * FRAC_PI_4).abs() < eps);
        assert!((turn_angle(p045, pz, p180).radians() - (-FRAC_PI_4)).abs() < eps);

        assert!((angle(p000, pz, p180).radians() - PI).abs() < eps);
        assert!(turn_angle(p000, pz, p180).radians().abs() < eps);

        assert!((angle(pz, p000, p045).radians() - FRAC_PI_2).abs() < eps);
        assert!((turn_angle(pz, p000, p045).radians() - FRAC_PI_2).abs() < eps);

        assert!(angle(pz, p000, pz).radians().abs() < eps);
        assert!((turn_angle(pz, p000, pz).radians().abs() - PI).abs() < eps);
    }

    #[test]
    fn test_area_methods_cpp() {
        // C++ TEST(S2, AreaMethods) — deterministic portions
        use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
        let pz = Point::from_coords(0.0, 0.0, 1.0);
        let p000 = Point::from_coords(1.0, 0.0, 0.0);
        let p045 = Point::from_coords(1.0, 1.0, 0.0).normalize();
        let p090 = Point::from_coords(0.0, 1.0, 0.0);
        let p180 = Point::from_coords(-1.0, 0.0, 0.0);

        assert!((point_area(p000, p090, pz) - FRAC_PI_2).abs() < 1e-12);
        assert!((point_area(p045, pz, p180) - 3.0 * FRAC_PI_4).abs() < 1e-12);

        // Very small area.
        let small_eps = 1e-10;
        let pepsx = Point::from_coords(small_eps, 0.0, 1.0).normalize();
        let pepsy = Point::from_coords(0.0, small_eps, 1.0).normalize();
        let expected1 = 0.5 * small_eps * small_eps;
        assert!(
            (point_area(pepsx, pepsy, pz) - expected1).abs() < 1e-14 * expected1,
            "small area: {} vs {}",
            point_area(pepsx, pepsy, pz),
            expected1
        );

        // Degenerate triangles.
        let pr = Point::from_coords(0.257, -0.5723, 0.112).normalize();
        let pq = Point::from_coords(-0.747, 0.401, 0.2235).normalize();
        assert_eq!(point_area(pr, pr, pr), 0.0);
        assert!(point_area(pr, pq, pr).abs() < 1e-15);
        assert_eq!(point_area(p000, p045, p090), 0.0);

        // Long and skinny triangle.
        let p045eps = Point::from_coords(1.0, 1.0, small_eps).normalize();
        let expected2 = 5.857_864_376_269_049_5e-11;
        assert!(
            (point_area(p000, p045eps, p090) - expected2).abs() < 1e-9 * expected2,
            "skinny: {} vs {}",
            point_area(p000, p045eps, p090),
            expected2
        );

        // Triangles with near-180 degree edges that sum to a quarter-sphere.
        let eps2 = 1e-14;
        let p000eps2 = Point::from_coords(1.0, 0.1 * eps2, eps2).normalize();
        let quarter_area1 = point_area(p000eps2, p000, p045)
            + point_area(p000eps2, p045, p180)
            + point_area(p000eps2, p180, pz)
            + point_area(p000eps2, pz, p000);
        assert!((quarter_area1 - PI).abs() < 1e-12);

        let p045eps2 = Point::from_coords(1.0, 1.0, eps2).normalize();
        let quarter_area2 = point_area(p045eps2, p000, p045)
            + point_area(p045eps2, p045, p180)
            + point_area(p045eps2, p180, pz)
            + point_area(p045eps2, pz, p000);
        assert!((quarter_area2 - PI).abs() < 1e-12);

        // Zero area due to collinear points.
        assert_eq!(
            0.0,
            point_area(
                LatLng::from_degrees(-45.0, -170.0).to_point(),
                LatLng::from_degrees(45.0, -170.0).to_point(),
                LatLng::from_degrees(0.0, -170.0).to_point(),
            )
        );
    }
}
