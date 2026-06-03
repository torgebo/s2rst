// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Computes a bounding latitude-longitude rectangle for an edge chain.
//!
//! [`LatLngRectBounder`] accumulates vertices of a chain v0, v1, v2, ...
//! and computes a conservative bounding rectangle that contains all edges.
//! The bound accounts for the fact that an edge's bounding box can be larger
//! than the box of its endpoints (e.g., an edge passing through a pole).
//!
//! Corresponds to C++ `s2latlng_rect_bounder.h/cc`.

use std::f64::consts::{FRAC_PI_2, PI};

use crate::r1;
use crate::s1;
use crate::s2::{LatLng, Point, Rect};

/// Computes a bounding latitude-longitude rectangle for a vertex chain.
///
/// Call [`add_point`](Self::add_point) for each vertex, then
/// [`get_bound`](Self::get_bound) to retrieve the conservative bound.
#[derive(Debug)]
pub struct LatLngRectBounder {
    a: Point,
    a_latlng: LatLng,
    bound: Rect,
}

impl LatLngRectBounder {
    /// Creates a new bounder with an empty initial bound.
    pub fn new() -> Self {
        LatLngRectBounder {
            a: Point::from_coords(0.0, 0.0, 0.0),
            a_latlng: LatLng::from_radians(0.0, 0.0),
            bound: Rect::empty(),
        }
    }

    /// Adds a vertex to the chain given as a Point (must be unit-length).
    pub fn add_point(&mut self, b: Point) {
        self.add_internal(b, LatLng::from_point(b));
    }

    /// Adds a vertex to the chain given as a `LatLng`.
    pub fn add_latlng(&mut self, b_latlng: LatLng) {
        self.add_internal(b_latlng.to_point(), b_latlng);
    }

    fn add_internal(&mut self, b: Point, b_latlng: LatLng) {
        if self.bound.is_empty() {
            self.bound = self.bound.add_point(b_latlng);
        } else {
            // Compute the cross product N = A x B robustly.
            let n = (self.a.0 - b.0).cross(self.a.0 + b.0); // N = 2*(A x B)

            let n_norm = n.norm();
            if n_norm < 1.91346e-15 {
                // A and B are nearly identical or nearly antipodal.
                if self.a.0.dot(b.0) < 0.0 {
                    // Nearly antipodal — bound could go anywhere.
                    self.bound = Rect::full();
                } else {
                    // Nearly identical — just use bounding rect of both points.
                    let pair = Rect::new(
                        r1::Interval::from_point_pair(
                            self.a_latlng.lat.radians(),
                            b_latlng.lat.radians(),
                        ),
                        s1::Interval::from_point_pair(
                            self.a_latlng.lng.radians(),
                            b_latlng.lng.radians(),
                        ),
                    );
                    self.bound = self.bound.union(pair);
                }
            } else {
                // Compute the longitude range spanned by AB.
                let mut lng_ab = s1::Interval::from_point_pair(
                    self.a_latlng.lng.radians(),
                    b_latlng.lng.radians(),
                );
                if lng_ab.length() >= PI - 2.0 * f64::EPSILON {
                    lng_ab = s1::Interval::full();
                }

                // Compute the latitude range spanned by the endpoints.
                let mut lat_ab = r1::Interval::from_point_pair(
                    self.a_latlng.lat.radians(),
                    b_latlng.lat.radians(),
                );

                // Check whether the edge crosses the plane through N and the
                // Z-axis (where the great circle attains min/max latitude).
                let z_axis = crate::r3::Vector {
                    x: 0.0,
                    y: 0.0,
                    z: 1.0,
                };
                let m = n.cross(z_axis);
                let m_a = m.dot(self.a.0);
                let m_b = m.dot(b.0);

                let m_error = 6.06638e-16 * n_norm + 6.83174e-31;
                if m_a * m_b < 0.0 || m_a.abs() <= m_error || m_b.abs() <= m_error {
                    let max_lat = (n.x.hypot(n.y)).atan2(n.z.abs()) + 3.0 * f64::EPSILON;
                    let max_lat = max_lat.min(FRAC_PI_2);

                    // Bound the latitude budget for nearby points.
                    let lat_budget_z = 0.5 * (self.a.0 - b.0).norm() * max_lat.sin();
                    let lat_budget =
                        2.0 * ((1.0 + 4.0 * f64::EPSILON) * lat_budget_z).min(1.0).asin();
                    let max_delta = 0.5 * (lat_budget - lat_ab.length()) + f64::EPSILON;

                    if m_a <= m_error && m_b >= -m_error {
                        lat_ab = r1::Interval::new(lat_ab.lo, max_lat.min(lat_ab.hi + max_delta));
                    }
                    if m_b <= m_error && m_a >= -m_error {
                        lat_ab =
                            r1::Interval::new((-max_lat).max(lat_ab.lo - max_delta), lat_ab.hi);
                    }
                }
                self.bound = self.bound.union(Rect::new(lat_ab, lng_ab));
            }
        }
        self.a = b;
        self.a_latlng = b_latlng;
    }

    /// Returns the bounding rectangle, expanded to account for numerical errors.
    pub fn get_bound(&self) -> Rect {
        let expansion = LatLng::from_radians(2.0 * f64::EPSILON, 0.0);
        self.bound.expanded(expansion).polar_closure()
    }

    /// Expands a bound so that it is guaranteed to contain the bounds of any
    /// sub-region whose bounds are computed using this class.
    pub fn expand_for_subregions(bound: Rect) -> Rect {
        bound.expand_for_subregions()
    }

    /// Returns the maximum error in `get_bound()` (for testing).
    pub fn max_error_for_tests() -> LatLng {
        LatLng::from_radians(10.0 * f64::EPSILON, f64::EPSILON)
    }
}

impl Default for LatLngRectBounder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::text_format;

    #[test]
    fn test_empty() {
        let bounder = LatLngRectBounder::new();
        assert!(bounder.get_bound().is_empty());
    }

    #[test]
    fn test_single_point() {
        let mut bounder = LatLngRectBounder::new();
        bounder.add_point(text_format::parse_point("0:0"));
        let bound = bounder.get_bound();
        assert!(!bound.is_empty());
        assert!(bound.contains_point(text_format::parse_point("0:0")));
    }

    #[test]
    fn test_two_points_equator() {
        let mut bounder = LatLngRectBounder::new();
        bounder.add_point(text_format::parse_point("0:0"));
        bounder.add_point(text_format::parse_point("0:90"));
        let bound = bounder.get_bound();
        let max_err = LatLngRectBounder::max_error_for_tests();
        // The edge goes along the equator, so lat should be near 0.
        assert!(bound.lat.lo >= -max_err.lat.radians());
        assert!(bound.lat.hi <= max_err.lat.radians());
        // Longitude should span from 0 to 90 degrees.
        assert!(bound.lng.lo <= max_err.lng.radians());
        assert!((bound.lng.hi - FRAC_PI_2).abs() <= max_err.lng.radians());
    }

    #[test]
    fn test_edge_through_pole() {
        let mut bounder = LatLngRectBounder::new();
        bounder.add_point(text_format::parse_point("0:0"));
        bounder.add_point(text_format::parse_point("0:180"));
        let bound = bounder.get_bound();
        // Edge from (0,0) to (0,180) passes through the north pole,
        // so latitude should reach PI/2.
        assert!(bound.lat.hi >= FRAC_PI_2 - 1e-14);
    }

    #[test]
    fn test_antipodal_points() {
        let mut bounder = LatLngRectBounder::new();
        bounder.add_point(text_format::parse_point("0:0"));
        bounder.add_point(Point::from_coords(-1.0, 0.0, 0.0));
        let bound = bounder.get_bound();
        // Nearly-antipodal points should result in full bound.
        assert!(bound.is_full());
    }

    #[test]
    fn test_loop_around_equator() {
        let mut bounder = LatLngRectBounder::new();
        let vertices = text_format::parse_points("0:0, 0:90, 0:180, 0:-90");
        for v in &vertices {
            bounder.add_point(*v);
        }
        bounder.add_point(vertices[0]);
        let bound = bounder.get_bound();
        let max_err = LatLngRectBounder::max_error_for_tests();
        // All edges are along the equator, so latitude should be near 0.
        assert!(bound.lat.lo >= -max_err.lat.radians());
        assert!(bound.lat.hi <= max_err.lat.radians());
        // Longitude should span the full circle.
        assert!(bound.lng.is_full());
    }

    #[test]
    fn test_triangle() {
        let mut bounder = LatLngRectBounder::new();
        let vertices = text_format::parse_points("10:0, 10:60, 10:120");
        for v in &vertices {
            bounder.add_point(*v);
        }
        bounder.add_point(vertices[0]);
        let bound = bounder.get_bound();
        let max_err = LatLngRectBounder::max_error_for_tests();
        assert!(bound.lat.lo < Angle::from_degrees(10.0).radians() + max_err.lat.radians());
        assert!(bound.lat.hi > Angle::from_degrees(10.0).radians() - max_err.lat.radians());
    }

    #[test]
    fn test_expand_for_subregions() {
        let bound = Rect::from_lat_lng(LatLng::from_degrees(10.0, 20.0));
        let expanded = LatLngRectBounder::expand_for_subregions(bound);
        assert!(expanded.contains(bound));
    }
}
