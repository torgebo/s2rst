// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Cohen-Sutherland 2D edge clipper.
//!
//! Clips edges to rectangular regions in 2D space. Ported from Java
//! `R2EdgeClipper`.
//!
//! The clipper treats the clip region as a closed set (points on the boundary
//! test as contained). It guarantees:
//! - Vertices inside the region are left unmodified.
//! - Clipped vertices land exactly on a boundary coordinate.
//! - Shared boundaries between adjacent regions produce identical results.

use crate::r2;

/// Outcode: vertex is inside the clip region.
pub const INSIDE: u8 = 0x00;
/// Outcode: vertex is below the clip region.
pub const BOTTOM: u8 = 0x01;
/// Outcode: vertex is right of the clip region.
pub const RIGHT: u8 = 0x02;
/// Outcode: vertex is above the clip region.
pub const TOP: u8 = 0x04;
/// Outcode: vertex is left of the clip region.
pub const LEFT: u8 = 0x08;
/// Outcode: vertex is outside (result of a failed clip).
pub const OUTSIDE: u8 = 0xFF;

/// Maximum absolute error in each clipped coordinate when the clip region
/// and points have coordinates with magnitude ≤ 1.
pub const MAX_UNIT_CLIP_ERROR: f64 = 2.0 * crate::s2::edge_clipping::EDGE_CLIP_ERROR_UV_COORD;

/// An edge in 2D space (two `r2::Point` endpoints).
#[derive(Clone, Debug, Default)]
pub struct R2Edge {
    /// First vertex.
    pub v0: r2::Point,
    /// Second vertex.
    pub v1: r2::Point,
}

impl R2Edge {
    /// Creates a new edge from two points.
    pub fn new(v0: r2::Point, v1: r2::Point) -> Self {
        Self { v0, v1 }
    }
}

/// Cohen-Sutherland rectangle edge clipper.
///
/// Call [`init`](Self::init) to set the clip rectangle, then
/// [`clip_edge`](Self::clip_edge) for each edge. The clipped result is in
/// [`clipped_edge`](Self::clipped_edge), and the outcodes for each vertex are
/// in [`outcode0`](Self::outcode0) and [`outcode1`](Self::outcode1).
#[derive(Debug)]
pub struct R2EdgeClipper {
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    last_outcode: u8,
    /// The clipped edge result (valid after a successful `clip_edge`).
    pub clipped_edge: R2Edge,
    /// Outcode for the first vertex of the clipped edge.
    pub outcode0: u8,
    /// Outcode for the second vertex of the clipped edge.
    pub outcode1: u8,
}

impl Default for R2EdgeClipper {
    fn default() -> Self {
        Self::new()
    }
}

impl R2EdgeClipper {
    /// Creates a new clipper with no clip rectangle set.
    pub fn new() -> Self {
        Self {
            x_min: 0.0,
            x_max: 0.0,
            y_min: 0.0,
            y_max: 0.0,
            last_outcode: OUTSIDE,
            clipped_edge: R2Edge::default(),
            outcode0: OUTSIDE,
            outcode1: OUTSIDE,
        }
    }

    /// Creates a new clipper for the given rectangle.
    pub fn from_rect(rect: &r2::Rect) -> Self {
        let mut c = Self::new();
        c.init(rect);
        c
    }

    /// Sets the clip rectangle.
    pub fn init(&mut self, rect: &r2::Rect) {
        self.x_min = rect.x.lo;
        self.x_max = rect.x.hi;
        self.y_min = rect.y.lo;
        self.y_max = rect.y.hi;
    }

    /// Returns the current clip rectangle.
    pub fn clip_rect(&self) -> r2::Rect {
        r2::Rect::from_points(
            r2::Point::new(self.x_min, self.y_min),
            r2::Point::new(self.x_max, self.y_max),
        )
    }

    /// Clips an edge to the current clip rectangle.
    ///
    /// Returns `true` when the edge intersected the clip region.
    /// If `connected` is true, the clipper reuses the outcode from the previous
    /// edge's second vertex (assumes `edge.v0 == prev_edge.v1`).
    pub fn clip_edge(&mut self, edge: &R2Edge, connected: bool) -> bool {
        self.outcode0 = OUTSIDE;
        self.outcode1 = OUTSIDE;

        let code0 = if connected {
            self.last_outcode
        } else {
            self.outcode_of(&edge.v0)
        };
        let code1 = self.outcode_of(&edge.v1);
        self.last_outcode = code1;

        // If both vertices are in the same outside region, the edge can't intersect.
        if (code0 & code1) != INSIDE {
            return false;
        }

        self.clipped_edge = edge.clone();

        let c0 = if code0 == INSIDE {
            code0
        } else {
            self.clip_vertex_0(edge, code0)
        };

        let c1 = if code1 == INSIDE {
            code1
        } else {
            self.clip_vertex_1(edge, code1)
        };

        self.outcode0 = c0;
        self.outcode1 = c1;

        c0 != OUTSIDE && c1 != OUTSIDE
    }

    /// Unconditionally clips to the boundary identified by a single-bit
    /// outcode, returning the intersection point.
    pub fn clip(&self, edge: &R2Edge, outcode: u8) -> r2::Point {
        debug_assert!(outcode > 0 && outcode.is_power_of_two());
        match outcode {
            BOTTOM => r2::Point::new(
                interpolate_double(self.y_min, edge.v0.y, edge.v1.y, edge.v0.x, edge.v1.x),
                self.y_min,
            ),
            RIGHT => r2::Point::new(
                self.x_max,
                interpolate_double(self.x_max, edge.v0.x, edge.v1.x, edge.v0.y, edge.v1.y),
            ),
            TOP => r2::Point::new(
                interpolate_double(self.y_max, edge.v0.y, edge.v1.y, edge.v0.x, edge.v1.x),
                self.y_max,
            ),
            LEFT => r2::Point::new(
                self.x_min,
                interpolate_double(self.x_min, edge.v0.x, edge.v1.x, edge.v0.y, edge.v1.y),
            ),
            _ => unreachable!("invalid outcode {outcode:#x}"),
        }
    }

    fn outcode_of(&self, p: &r2::Point) -> u8 {
        let mut code = 0u8;
        if p.x < self.x_min {
            code |= LEFT;
        } else if p.x > self.x_max {
            code |= RIGHT;
        }
        if p.y < self.y_min {
            code |= BOTTOM;
        } else if p.y > self.y_max {
            code |= TOP;
        }
        code
    }

    fn clip_vertex_0(&mut self, edge: &R2Edge, code: u8) -> u8 {
        self.clip_vertex(true, edge, code)
    }

    fn clip_vertex_1(&mut self, edge: &R2Edge, code: u8) -> u8 {
        self.clip_vertex(false, edge, code)
    }

    fn clip_vertex(&mut self, is_v0: bool, edge: &R2Edge, code: u8) -> u8 {
        debug_assert!(code != INSIDE && code != OUTSIDE);

        // Simple case: single boundary.
        if code.is_power_of_two() {
            let p = self.clip(edge, code);
            if self.outcode_of(&p) != INSIDE {
                return OUTSIDE;
            }
            if is_v0 {
                self.clipped_edge.v0 = p;
            } else {
                self.clipped_edge.v1 = p;
            }
            return code;
        }

        // Corner case: try both boundaries.
        let (out_a, out_b) = match code {
            c if c == TOP | LEFT => (TOP, LEFT),
            c if c == TOP | RIGHT => (TOP, RIGHT),
            c if c == BOTTOM | LEFT => (BOTTOM, LEFT),
            c if c == BOTTOM | RIGHT => (BOTTOM, RIGHT),
            _ => unreachable!("invalid outcode {code:#x}"),
        };

        let va = self.clip(edge, out_a);
        if self.outcode_of(&va) == INSIDE {
            if is_v0 {
                self.clipped_edge.v0 = va;
            } else {
                self.clipped_edge.v1 = va;
            }
            return out_a;
        }

        let vb = self.clip(edge, out_b);
        if self.outcode_of(&vb) == INSIDE {
            if is_v0 {
                self.clipped_edge.v0 = vb;
            } else {
                self.clipped_edge.v1 = vb;
            }
            return out_b;
        }

        OUTSIDE
    }
}

/// Linear interpolation: given `x` between `a` and `b`, interpolate `a1`/`b1`.
///
/// Interpolates from whichever of `a`/`b` is closer to `x` for better
/// numerical accuracy.
fn interpolate_double(x: f64, a: f64, b: f64, a1: f64, b1: f64) -> f64 {
    debug_assert!(a != b);
    if (a - x).abs() <= (b - x).abs() {
        a1 + (b1 - a1) * ((x - a) / (b - a))
    } else {
        b1 + (a1 - b1) * ((x - b) / (a - b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x_lo: f64, y_lo: f64, x_hi: f64, y_hi: f64) -> r2::Rect {
        r2::Rect::from_points(r2::Point::new(x_lo, y_lo), r2::Point::new(x_hi, y_hi))
    }

    fn edge(x0: f64, y0: f64, x1: f64, y1: f64) -> R2Edge {
        R2Edge::new(r2::Point::new(x0, y0), r2::Point::new(x1, y1))
    }

    #[test]
    fn test_fully_inside() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(0.2, 0.3, 0.7, 0.8), false));
        assert_eq!(c.outcode0, INSIDE);
        assert_eq!(c.outcode1, INSIDE);
    }

    #[test]
    fn test_fully_outside_same_region() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        // Both to the left.
        assert!(!c.clip_edge(&edge(-0.5, 0.3, -0.1, 0.8), false));
    }

    #[test]
    fn test_clip_left() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(-1.0, 0.5, 0.5, 0.5), false));
        assert_eq!(c.outcode0, LEFT);
        assert_eq!(c.outcode1, INSIDE);
        assert_eq!(c.clipped_edge.v0.x, 0.0);
    }

    #[test]
    fn test_clip_right() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(0.5, 0.5, 2.0, 0.5), false));
        assert_eq!(c.outcode0, INSIDE);
        assert_eq!(c.outcode1, RIGHT);
        assert_eq!(c.clipped_edge.v1.x, 1.0);
    }

    #[test]
    fn test_clip_top() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(0.5, 0.5, 0.5, 2.0), false));
        assert_eq!(c.outcode0, INSIDE);
        assert_eq!(c.outcode1, TOP);
        assert_eq!(c.clipped_edge.v1.y, 1.0);
    }

    #[test]
    fn test_clip_bottom() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(0.5, -1.0, 0.5, 0.5), false));
        assert_eq!(c.outcode0, BOTTOM);
        assert_eq!(c.outcode1, INSIDE);
        assert_eq!(c.clipped_edge.v0.y, 0.0);
    }

    #[test]
    fn test_clip_both_ends() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        assert!(c.clip_edge(&edge(-1.0, 0.5, 2.0, 0.5), false));
        assert_eq!(c.outcode0, LEFT);
        assert_eq!(c.outcode1, RIGHT);
        assert_eq!(c.clipped_edge.v0.x, 0.0);
        assert_eq!(c.clipped_edge.v1.x, 1.0);
    }

    #[test]
    fn test_diagonal_miss() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        // Entirely above and to the right — no intersection.
        assert!(!c.clip_edge(&edge(2.0, 3.0, 3.0, 2.0), false));
    }

    #[test]
    fn test_connected_reuses_outcode() {
        let mut c = R2EdgeClipper::from_rect(&rect(0.0, 0.0, 1.0, 1.0));
        // First edge: v1 is inside.
        assert!(c.clip_edge(&edge(-1.0, 0.5, 0.5, 0.5), false));
        // Connected edge: v0 reuses v1's outcode (INSIDE).
        assert!(c.clip_edge(&edge(0.5, 0.5, 0.8, 0.8), true));
        assert_eq!(c.outcode0, INSIDE);
        assert_eq!(c.outcode1, INSIDE);
    }

    #[test]
    fn test_interpolate_double_symmetry() {
        let v = interpolate_double(0.5, 0.0, 1.0, 10.0, 20.0);
        assert!((v - 15.0).abs() < 1e-10);
    }
}
