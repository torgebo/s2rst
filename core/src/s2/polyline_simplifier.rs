// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Polyline simplification using angular constraints.
//!
//! [`PolylineSimplifier`] finds maximal edges that pass through a sequence of
//! target discs while avoiding other discs, using exact arithmetic for
//! containment predicates. Use [`S2Builder`](super::builder::S2Builder) for more full-featured
//! simplification (e.g., with snapping); this class is for cases where a
//! simpler API is needed.
//!
//! Corresponds to C++ `s2polyline_simplifier.h/cc`.

use std::f64::consts::{FRAC_PI_2, PI};

use crate::s1;
use crate::s1::Interval as S1Interval;
use crate::s2::Point;

/// Half-epsilon for floating-point error bounds.
const DBL_ERR: f64 = 0.5 * f64::EPSILON;

/// Simplifies polylines by tracking an allowable range of output edge
/// directions. Target discs constrain the edge to pass through them; avoid
/// discs constrain the edge to stay clear. Results are conservative with
/// respect to floating-point error.
#[derive(Debug)]
pub struct PolylineSimplifier {
    src: Point,
    x_dir: Point,
    y_dir: Point,
    window: S1Interval,
    ranges_to_avoid: Vec<RangeToAvoid>,
}

#[derive(Debug)]
struct RangeToAvoid {
    interval: S1Interval,
    on_left: bool,
}

impl PolylineSimplifier {
    /// Creates a new simplifier (must call [`init`](Self::init) before use).
    pub fn new() -> Self {
        PolylineSimplifier {
            src: Point::from_coords(1.0, 0.0, 0.0),
            x_dir: Point::from_coords(0.0, 0.0, 0.0),
            y_dir: Point::from_coords(0.0, 0.0, 0.0),
            window: S1Interval::full(),
            ranges_to_avoid: Vec::new(),
        }
    }

    /// Starts a new simplified edge at `src`.
    pub fn init(&mut self, src: Point) {
        self.src = src;
        self.window = S1Interval::full();
        self.ranges_to_avoid.clear();

        // Precompute tangent-space basis vectors at src. Find the component
        // with smallest absolute magnitude, then compute the cross products.
        let abs_x = src.0.x.abs();
        let abs_y = src.0.y.abs();
        let abs_z = src.0.z.abs();

        // i = index of smallest absolute component
        let (i, j, k) = if abs_x < abs_y {
            if abs_x < abs_z { (0, 1, 2) } else { (2, 0, 1) }
        } else if abs_y < abs_z {
            (1, 2, 0)
        } else {
            (2, 0, 1)
        };

        let s = [src.0.x, src.0.y, src.0.z];
        let mut y = [0.0, 0.0, 0.0];
        y[i] = 0.0;
        y[j] = s[k];
        y[k] = -s[j];
        self.y_dir = Point(crate::r3::Vector {
            x: y[0],
            y: y[1],
            z: y[2],
        });

        // x_dir = y_dir x src (expanded to avoid multiplies by zero)
        let mut x = [0.0, 0.0, 0.0];
        x[i] = s[j] * s[j] + s[k] * s[k];
        x[j] = -s[j] * s[i];
        x[k] = -s[k] * s[i];
        self.x_dir = Point(crate::r3::Vector {
            x: x[0],
            y: x[1],
            z: x[2],
        });
    }

    /// Returns the source vertex of the output edge.
    pub fn src(&self) -> Point {
        self.src
    }

    /// Returns true if the edge `(src, dst)` satisfies all targeting
    /// requirements so far. Returns false if the edge would be longer than
    /// 90 degrees (such edges are not supported).
    pub fn extend(&self, dst: Point) -> bool {
        if self.src.chord_angle(dst) > s1::ChordAngle::RIGHT {
            return false;
        }
        let dir = self.get_direction(dst);
        if !self.window.contains(dir) {
            return false;
        }
        for range in &self.ranges_to_avoid {
            if range.interval.contains(dir) {
                return false;
            }
        }
        true
    }

    /// Requires that the output edge must pass through the given disc.
    /// Returns true if it is possible to do so given previous constraints.
    pub fn target_disc(&mut self, point: Point, radius: s1::ChordAngle) -> bool {
        let semiwidth = self.get_semiwidth(point, radius, -1);
        if semiwidth >= PI {
            // The target disc contains src, nothing to do.
            return true;
        }
        if semiwidth < 0.0 {
            self.window = S1Interval::empty();
            return false;
        }
        let center = self.get_direction(point);
        let target = S1Interval::from_point(center).expanded(semiwidth);
        self.window = self.window.intersection(target);

        // Process any pending avoid ranges.
        let ranges: Vec<RangeToAvoid> = self.ranges_to_avoid.drain(..).collect();
        for range in &ranges {
            self.avoid_range(range.interval, range.on_left);
        }

        !self.window.is_empty()
    }

    /// Requires that the output edge must avoid the given disc.
    /// `disc_on_left` specifies whether the disc must be to the left or
    /// right of the output edge.
    pub fn avoid_disc(&mut self, point: Point, radius: s1::ChordAngle, disc_on_left: bool) -> bool {
        let semiwidth = self.get_semiwidth(point, radius, 1);
        if semiwidth >= PI {
            self.window = S1Interval::empty();
            return false;
        }
        let center = self.get_direction(point);
        let dleft = if disc_on_left { FRAC_PI_2 } else { semiwidth };
        let dright = if disc_on_left { semiwidth } else { FRAC_PI_2 };
        let avoid_interval = S1Interval::new(
            f64::rem_euclid(center - dright + PI, 2.0 * PI) - PI,
            f64::rem_euclid(center + dleft + PI, 2.0 * PI) - PI,
        );

        if self.window.is_full() {
            self.ranges_to_avoid.push(RangeToAvoid {
                interval: avoid_interval,
                on_left: disc_on_left,
            });
            return true;
        }
        self.avoid_range(avoid_interval, disc_on_left);
        !self.window.is_empty()
    }

    fn avoid_range(&mut self, avoid_interval: S1Interval, disc_on_left: bool) {
        debug_assert!(!self.window.is_full());
        if self.window.contains_interval(avoid_interval) {
            if disc_on_left {
                self.window = S1Interval::new(self.window.lo, avoid_interval.lo);
            } else {
                self.window = S1Interval::new(avoid_interval.hi, self.window.hi);
            }
        } else {
            self.window = self.window.intersection(avoid_interval.complement());
        }
    }

    fn get_direction(&self, p: Point) -> f64 {
        let y = p.0.dot(self.y_dir.0);
        let x = p.0.dot(self.x_dir.0);
        y.atan2(x)
    }

    fn get_semiwidth(&self, p: Point, r: s1::ChordAngle, round_direction: i32) -> f64 {
        let r2 = r.length2();
        let mut a2 = self.src.chord_angle(p).length2();
        a2 -= 64.0 * DBL_ERR * DBL_ERR * f64::from(round_direction);
        if a2 <= r2 {
            return PI; // The given disc contains src.
        }

        let sin2_r = r2 * (1.0 - 0.25 * r2);
        let sin2_a = a2 * (1.0 - 0.25 * a2);
        let semiwidth = (sin2_r / sin2_a).sqrt().asin();

        let error = (2.0 * 10.0 + 4.0) * DBL_ERR + 17.0 * DBL_ERR * semiwidth;
        semiwidth + f64::from(round_direction) * error
    }
}

impl Default for PolylineSimplifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s1::Angle;
    use crate::s2::text_format;

    fn check_simplify(
        src_str: &str,
        dst_str: &str,
        target_str: &str,
        avoid_str: &str,
        disc_on_left: &[bool],
        radius_degrees: f64,
        expected: bool,
    ) {
        let radius = s1::ChordAngle::from_angle(Angle::from_degrees(radius_degrees));
        let mut s = PolylineSimplifier::new();
        s.init(text_format::parse_point(src_str));

        if !target_str.is_empty() {
            for p in text_format::parse_points(target_str) {
                s.target_disc(p, radius);
            }
        }
        if !avoid_str.is_empty() {
            for (i, p) in text_format::parse_points(avoid_str).iter().enumerate() {
                s.avoid_disc(*p, radius, disc_on_left[i]);
            }
        }
        let dst = text_format::parse_point(dst_str);
        assert_eq!(
            expected,
            s.extend(dst),
            "src={src_str}, dst={dst_str}, target={target_str}, avoid={avoid_str}"
        );
    }

    #[test]
    fn test_src() {
        let mut s = PolylineSimplifier::new();
        s.init(Point::from_coords(1.0, 0.0, 0.0));
        assert_eq!(s.src(), Point::from_coords(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_reuse() {
        let mut s = PolylineSimplifier::new();
        let radius = s1::ChordAngle::from_angle(Angle::from_degrees(10.0));

        s.init(Point::from_coords(1.0, 0.0, 0.0));
        assert!(
            s.target_disc(
                Point(
                    crate::r3::Vector {
                        x: 1.0,
                        y: 1.0,
                        z: 0.0
                    }
                    .normalize()
                ),
                radius
            )
        );
        assert!(
            s.target_disc(
                Point(
                    crate::r3::Vector {
                        x: 1.0,
                        y: 1.0,
                        z: 0.1
                    }
                    .normalize()
                ),
                radius
            )
        );
        assert!(
            !s.extend(Point(
                crate::r3::Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 0.4
                }
                .normalize()
            ))
        );

        s.init(Point::from_coords(0.0, 1.0, 0.0));
        assert!(
            s.target_disc(
                Point(
                    crate::r3::Vector {
                        x: 1.0,
                        y: 1.0,
                        z: 0.3
                    }
                    .normalize()
                ),
                radius
            )
        );
        assert!(
            s.target_disc(
                Point(
                    crate::r3::Vector {
                        x: 1.0,
                        y: 1.0,
                        z: 0.2
                    }
                    .normalize()
                ),
                radius
            )
        );
        assert!(
            !s.extend(Point(
                crate::r3::Vector {
                    x: 1.0,
                    y: 1.0,
                    z: 0.0
                }
                .normalize()
            ))
        );
    }

    #[test]
    fn test_no_constraints() {
        check_simplify("0:1", "0:1", "", "", &[], 0.0, true);
        check_simplify("0:1", "1:0", "", "", &[], 0.0, true);
        // Edge longer than 90 degrees is not supported.
        check_simplify("0:0", "0:91", "", "", &[], 0.0, false);
    }

    #[test]
    fn test_target_one_point() {
        check_simplify("0:0", "0:2", "0:1", "", &[], 1e-10, true);
        check_simplify("0:0", "0:2", "1:1", "", &[], 0.9, false);
        // Target disc contains src vertex.
        check_simplify("0:0", "0:2", "0:0.1", "", &[], 1.0, true);
        // Target disc contains dst vertex.
        check_simplify("0:0", "0:2", "0:2.1", "", &[], 1.0, true);
    }

    #[test]
    fn test_avoid_one_point() {
        check_simplify("0:0", "0:2", "", "0:1", &[true], 1e-10, false);
        check_simplify("0:0", "0:2", "", "1:1", &[true], 0.9, true);
        // Disc on wrong side.
        check_simplify("0:0", "0:2", "", "1:1", &[false], 1e-10, false);
        // Point behind src — disc_on_left should not affect result.
        check_simplify("0:0", "0:2", "", "1:-1", &[false], 1.4, true);
        check_simplify("0:0", "0:2", "", "1:-1", &[true], 1.4, true);
        check_simplify("0:0", "0:2", "", "-1:-1", &[false], 1.4, true);
        check_simplify("0:0", "0:2", "", "-1:-1", &[true], 1.4, true);
    }

    #[test]
    fn test_avoid_several_points() {
        for dst in &["0:2", "1.732:-1", "-1.732:-1"] {
            check_simplify(
                "0:0",
                dst,
                "",
                "0.01:2, 1.732:-1.01, -1.732:-0.99",
                &[true, true, true],
                0.00001,
                true,
            );
            check_simplify(
                "0:0",
                dst,
                "",
                "0.01:2, 1.732:-1.01, -1.732:-0.99",
                &[false, false, false],
                0.00001,
                false,
            );
        }
    }

    #[test]
    fn test_target_and_avoid() {
        check_simplify(
            "0:0",
            "10:10",
            "2:3, 4:3, 7:8",
            "4:2, 7:5, 7:9",
            &[true, true, false],
            1.0,
            true,
        );
        // One target point too far away.
        check_simplify(
            "0:0",
            "10:10",
            "2:3, 4:6, 7:8",
            "4:2, 7:5, 7:9",
            &[true, true, false],
            1.0,
            false,
        );
        // One avoid point too close.
        check_simplify(
            "0:0",
            "10:10",
            "2:3, 4:3, 7:8",
            "4:2, 6:5, 7:9",
            &[true, true, false],
            1.0,
            false,
        );
    }
}
