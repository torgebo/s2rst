// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Map projections between the sphere (S2) and the plane (R2).
//!
//! Provides a [`Projection`] trait and implementations for common
//! projections (Plate Carree, Mercator).
//!
//! Corresponds to Go `s2/projections.go`, C++ `s2edge_tessellator.h`.

use crate::r2;
use crate::s1::Angle;
use crate::s2::{LatLng, Point};
use std::f64::consts::PI;

/// Defines how points on the sphere are mapped to 2D coordinates and back.
pub trait Projection: std::fmt::Debug {
    /// Converts a point on the sphere to a projected 2D point.
    fn project(&self, p: Point) -> r2::Point;

    /// Converts a projected 2D point to a point on the sphere.
    fn unproject(&self, p: r2::Point) -> Point;

    /// Convenience: project from `LatLng` (may be more efficient than
    /// `project(ll.to_point())`).
    #[expect(
        clippy::wrong_self_convention,
        reason = "named to match C++ API convention"
    )]
    fn from_lat_lng(&self, ll: LatLng) -> r2::Point;

    /// Convenience: unproject to `LatLng` (may be more efficient than
    /// `LatLng::from_point(unproject(p))`).
    fn to_lat_lng(&self, p: r2::Point) -> LatLng;

    /// Interpolates the given fraction of the distance along the line from
    /// `a` to `b`. Fractions outside [0, 1] result in extrapolation.
    fn interpolate(&self, f: f64, a: r2::Point, b: r2::Point) -> r2::Point;

    /// Reports the coordinate wrapping distance along each axis. A value of
    /// zero means no wrapping on that axis. For example, if
    /// `wrap_distance().x == 360`, then `(x, y)` and `(x+360, y)` map to the
    /// same point.
    fn wrap_distance(&self) -> r2::Point;

    /// Wraps the coordinates of `b` if necessary to obtain the shortest
    /// edge from `a` to `b`.
    fn wrap_destination(&self, a: r2::Point, b: r2::Point) -> r2::Point {
        let wrap = self.wrap_distance();
        let mut x = b.x;
        let mut y = b.y;
        if wrap.x > 0.0 && (x - a.x).abs() > 0.5 * wrap.x {
            x = a.x + remainder(x - a.x, wrap.x);
        }
        if wrap.y > 0.0 && (y - a.y).abs() > 0.5 * wrap.y {
            y = a.y + remainder(y - a.y, wrap.y);
        }
        r2::Point::new(x, y)
    }
}

/// IEEE 754 remainder (equivalent to Go `math.Remainder`).
fn remainder(x: f64, y: f64) -> f64 {
    // Rust doesn't have a direct equivalent in std, but f64::rem_euclid isn't
    // the same. We implement using the formula: x - round(x/y) * y.
    x - (x / y).round() * y
}

// ─── Plate Carree ──────────────────────────────────────────────────────

/// The "plate carree" projection: maps (longitude, latitude) linearly to
/// (x, y). Coordinates can be scaled to represent radians, degrees, etc.
///
/// By default (`x_scale` = π), coordinates are in radians with x in [-π, π]
/// and y in [-π/2, π/2].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlateCarreeProjection {
    x_wrap: f64,
    to_radians: f64,
    from_radians: f64,
}

impl PlateCarreeProjection {
    /// Creates a new plate carree projection where x-coordinates span
    /// [-`x_scale`, `x_scale`] and y-coordinates span [-`x_scale/2`, `x_scale/2`].
    ///
    /// For example, `x_scale = 180` gives degrees.
    pub fn new(x_scale: f64) -> Self {
        PlateCarreeProjection {
            x_wrap: 2.0 * x_scale,
            to_radians: PI / x_scale,
            from_radians: x_scale / PI,
        }
    }
}

impl Projection for PlateCarreeProjection {
    fn project(&self, p: Point) -> r2::Point {
        self.from_lat_lng(LatLng::from_point(p))
    }

    fn unproject(&self, p: r2::Point) -> Point {
        self.to_lat_lng(p).to_point()
    }

    fn from_lat_lng(&self, ll: LatLng) -> r2::Point {
        r2::Point::new(
            self.from_radians * ll.lng.radians(),
            self.from_radians * ll.lat.radians(),
        )
    }

    fn to_lat_lng(&self, p: r2::Point) -> LatLng {
        LatLng::new(
            Angle::from_radians(self.to_radians * p.y),
            Angle::from_radians(self.to_radians * remainder(p.x, self.x_wrap)),
        )
    }

    fn interpolate(&self, f: f64, a: r2::Point, b: r2::Point) -> r2::Point {
        a * (1.0 - f) + b * f
    }

    fn wrap_distance(&self) -> r2::Point {
        r2::Point::new(self.x_wrap, 0.0)
    }
}

// ─── Mercator ──────────────────────────────────────────────────────────

/// The spherical Mercator projection. Maps longitude linearly to x, and
/// latitude non-linearly to y using the Mercator formula.
///
/// The x-axis is finite and wraps, while the y-axis is infinite (poles
/// have y = ±∞).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MercatorProjection {
    x_wrap: f64,
    to_radians: f64,
    from_radians: f64,
}

impl MercatorProjection {
    /// Creates a new Mercator projection where the longitude axis spans
    /// [-`max_lng`, `max_lng`]. For degrees, use `max_lng = 180`.
    pub fn new(max_lng: f64) -> Self {
        MercatorProjection {
            x_wrap: 2.0 * max_lng,
            to_radians: PI / max_lng,
            from_radians: max_lng / PI,
        }
    }
}

impl Projection for MercatorProjection {
    fn project(&self, p: Point) -> r2::Point {
        self.from_lat_lng(LatLng::from_point(p))
    }

    fn unproject(&self, p: r2::Point) -> Point {
        self.to_lat_lng(p).to_point()
    }

    fn from_lat_lng(&self, ll: LatLng) -> r2::Point {
        let sin_phi = ll.lat.radians().sin();
        let y = 0.5 * ((1.0 + sin_phi) / (1.0 - sin_phi)).ln();
        r2::Point::new(self.from_radians * ll.lng.radians(), self.from_radians * y)
    }

    fn to_lat_lng(&self, p: r2::Point) -> LatLng {
        let x = self.to_radians * remainder(p.x, self.x_wrap);
        let k = (2.0 * self.to_radians * p.y).exp();
        let y = if k.is_infinite() {
            PI / 2.0
        } else {
            ((k - 1.0) / (k + 1.0)).asin()
        };
        LatLng::new(Angle::from_radians(y), Angle::from_radians(x))
    }

    fn interpolate(&self, f: f64, a: r2::Point, b: r2::Point) -> r2::Point {
        a * (1.0 - f) + b * f
    }

    fn wrap_distance(&self) -> r2::Point {
        r2::Point::new(self.x_wrap, 0.0)
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn float64_near(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn test_plate_carree_roundtrip() {
        let proj = PlateCarreeProjection::new(180.0);
        let p = Point::from_coords(1.0, 0.0, 0.0);
        let r2 = proj.project(p);
        let back = proj.unproject(r2);
        assert!(
            p.approx_eq_angle(back, Angle::from_radians(1e-14)),
            "roundtrip: {p} -> {r2} -> {back}",
        );
    }

    #[test]
    fn test_plate_carree_from_lat_lng() {
        let proj = PlateCarreeProjection::new(180.0);
        let ll = LatLng::from_degrees(45.0, 90.0);
        let pt = proj.from_lat_lng(ll);
        assert!(float64_near(pt.x, 90.0, 1e-10), "x = {}, want 90", pt.x,);
        assert!(float64_near(pt.y, 45.0, 1e-10), "y = {}, want 45", pt.y,);
    }

    #[test]
    fn test_plate_carree_interpolate() {
        let proj = PlateCarreeProjection::new(180.0);
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(10.0, 20.0);
        let mid = proj.interpolate(0.5, a, b);
        assert!(float64_near(mid.x, 5.0, 1e-10));
        assert!(float64_near(mid.y, 10.0, 1e-10));
    }

    #[test]
    fn test_plate_carree_wrap() {
        let proj = PlateCarreeProjection::new(180.0);
        let a = r2::Point::new(170.0, 0.0);
        let b = r2::Point::new(-170.0, 0.0);
        let wrapped = proj.wrap_destination(a, b);
        assert!(
            float64_near(wrapped.x, 190.0, 1e-10),
            "wrapped x = {}, want 190",
            wrapped.x,
        );
    }

    #[test]
    fn test_mercator_roundtrip() {
        let proj = MercatorProjection::new(180.0);
        let p = Point::from_coords(1.0, 0.5, 0.3);
        let r2 = proj.project(p);
        let back = proj.unproject(r2);
        assert!(
            p.approx_eq_angle(back, Angle::from_radians(1e-14)),
            "roundtrip: {p} -> {r2} -> {back}",
        );
    }

    #[test]
    fn test_mercator_equator() {
        let proj = MercatorProjection::new(180.0);
        let ll = LatLng::from_degrees(0.0, 0.0);
        let pt = proj.from_lat_lng(ll);
        assert!(float64_near(pt.x, 0.0, 1e-10));
        assert!(float64_near(pt.y, 0.0, 1e-10));
    }

    #[test]
    fn test_remainder() {
        assert!(float64_near(remainder(5.0, 3.0), -1.0, 1e-15));
        assert!(float64_near(remainder(4.0, 3.0), 1.0, 1e-15));
        assert!(float64_near(remainder(-4.0, 3.0), -1.0, 1e-15));
    }

    // ─── C++ parity tests ──────────────────────────────────────────────

    fn test_project_unproject(proj: &dyn Projection, px: r2::Point, x: Point) {
        let projected = proj.project(x);
        assert!(
            (projected.x == px.x || float64_near(projected.x, px.x, 1e-10))
                && (projected.y == px.y || float64_near(projected.y, px.y, 1e-10)),
            "project({x}) = {projected}, want {px}",
        );
        let back = proj.unproject(px);
        assert!(
            x.approx_eq_angle(back, Angle::from_radians(1e-10)),
            "unproject({px}) = {back}, want ≈{x}",
        );
    }

    #[test]
    fn test_plate_carree_project_unproject() {
        // Matches C++ TEST(PlateCarreeProjection, ProjectUnproject).
        let proj = PlateCarreeProjection::new(180.0);
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, 0.0),
            Point::from_coords(1.0, 0.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(180.0, 0.0),
            Point::from_coords(-1.0, 0.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(90.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(-90.0, 0.0),
            Point::from_coords(0.0, -1.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, 90.0),
            Point::from_coords(0.0, 0.0, 1.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, -90.0),
            Point::from_coords(0.0, 0.0, -1.0),
        );
    }

    #[test]
    fn test_mercator_project_unproject() {
        // Matches C++ TEST(MercatorProjection, ProjectUnproject).
        let proj = MercatorProjection::new(180.0);
        let inf = f64::INFINITY;
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, 0.0),
            Point::from_coords(1.0, 0.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(180.0, 0.0),
            Point::from_coords(-1.0, 0.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(90.0, 0.0),
            Point::from_coords(0.0, 1.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(-90.0, 0.0),
            Point::from_coords(0.0, -1.0, 0.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, inf),
            Point::from_coords(0.0, 0.0, 1.0),
        );
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, -inf),
            Point::from_coords(0.0, 0.0, -1.0),
        );
        // Arbitrary point sanity check.
        test_project_unproject(
            &proj,
            r2::Point::new(0.0, 70.255578967830246),
            LatLng::from_radians(1.0, 0.0).to_point(),
        );
    }

    #[test]
    fn test_plate_carree_interpolate_exact() {
        // Matches C++ TEST(PlateCarreeProjection, Interpolate).
        let proj = PlateCarreeProjection::new(180.0);
        let result = proj.interpolate(0.25, r2::Point::new(1.0, 5.0), r2::Point::new(3.0, 9.0));
        assert!(float64_near(result.x, 1.5, 1e-15));
        assert!(float64_near(result.y, 6.0, 1e-15));

        // Extrapolation.
        let result = proj.interpolate(-2.0, r2::Point::new(1.0, 0.0), r2::Point::new(3.0, 0.0));
        assert!(float64_near(result.x, -3.0, 1e-15));
        assert!(float64_near(result.y, 0.0, 1e-15));

        // Exact at endpoints.
        let a = r2::Point::new(1.234, -5.456e-20);
        let b = r2::Point::new(2.1234e-20, 7.456);
        let r0 = proj.interpolate(0.0, a, b);
        let r1 = proj.interpolate(1.0, a, b);
        assert_eq!(r0.x, a.x);
        assert_eq!(r0.y, a.y);
        assert_eq!(r1.x, b.x);
        assert_eq!(r1.y, b.y);
    }
}
