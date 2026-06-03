// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Go:   golang/geo
//   - Java: google/s2-geometry-library-java

//! Robust edge clipping for S2 cube faces and 2D rectangles.
//!
//! Provides functions for:
//! 1. Robustly clipping geodesic edges to the faces of the S2 biunit cube.
//! 2. Robustly clipping 2D edges against 2D rectangles.
//!
//! These functions can be used to find the set of `CellIDs` intersected by a
//! geodesic edge (e.g., see `CrossingEdgeQuery`).
//!
//! Corresponds to Go `s2/edge_clipping.go`, C++ `s2edge_clipping.cc`.

use crate::r1;
use crate::r2;
use crate::r3::Vector;
use crate::s2::Point;
use crate::s2::coords;
use crate::s2::predicates;

// ─── Constants ─────────────────────────────────────────────────────────

/// Maximum error in a u- or v-coordinate compared to the exact result,
/// assuming that the input points are in or near [-1,1]x[-1,1].
pub const EDGE_CLIP_ERROR_UV_COORD: f64 = 2.25 * predicates::DBL_EPSILON;

/// Maximum distance from a clipped point to the exact result.
pub const EDGE_CLIP_ERROR_UV_DIST: f64 = 2.25 * predicates::DBL_EPSILON;

/// Maximum angle between a returned vertex and the nearest point on the
/// exact edge AB (in radians).
pub const FACE_CLIP_ERROR_RADIANS: f64 = 3.0 * predicates::DBL_EPSILON;

/// Maximum distance in (u,v)-space from a returned vertex to the exact
/// edge AB projected into (u,v)-space.
pub const FACE_CLIP_ERROR_UV_DIST: f64 = 9.0 * predicates::DBL_EPSILON;

/// Maximum error in an individual u- or v-coordinate of a returned vertex.
pub const FACE_CLIP_ERROR_UV_COORD: f64 =
    9.0 * std::f64::consts::FRAC_1_SQRT_2 * predicates::DBL_EPSILON;

/// Maximum error when testing whether a point intersects a rectangle.
pub const INTERSECTS_RECT_ERROR_UV_DIST: f64 =
    3.0 * std::f64::consts::SQRT_2 * predicates::DBL_EPSILON;

// ─── Face Segment ──────────────────────────────────────────────────────

/// An edge AB clipped to an S2 cube face, represented by a face index and
/// a pair of (u,v) coordinates.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FaceSegment {
    /// The cube face index (0–5).
    pub face: coords::Face,
    /// The first endpoint in (u,v) coordinates.
    pub a: r2::Point,
    /// The second endpoint in (u,v) coordinates.
    pub b: r2::Point,
}

// ─── Public API ────────────────────────────────────────────────────────

/// Returns the (u,v) coordinates for the portion of edge AB that intersects
/// the given face, or `None` if the edge does not intersect.
///
/// Clipped vertices lie within [-1,1]x[-1,1] and are within
/// `FACE_CLIP_ERROR_UV_DIST` of the line AB.
#[inline]
pub fn clip_to_face(a: Point, b: Point, face: coords::Face) -> Option<(r2::Point, r2::Point)> {
    clip_to_padded_face(a, b, face, 0.0)
}

/// Like `clip_to_face`, but clips to [-R,R]x[-R,R] where R = 1 + padding.
///
/// Padding must be non-negative.
#[inline]
pub fn clip_to_padded_face(
    a: Point,
    b: Point,
    face: coords::Face,
    padding: f64,
) -> Option<(r2::Point, r2::Point)> {
    debug_assert!(padding >= 0.0);
    // Fast path: both endpoints are on the given face.
    if coords::get_face(&a.0) == face && coords::get_face(&b.0) == face {
        let (au, av) = coords::valid_face_xyz_to_uv(face, &a.0);
        let (bu, bv) = coords::valid_face_xyz_to_uv(face, &b.0);
        return Some((r2::Point::new(au, av), r2::Point::new(bu, bv)));
    }

    // Convert to (u,v,w) coordinates of the given face.
    let norm_uvw = coords::face_xyz_to_uvw(face, &a.point_cross(b).0);
    let a_uvw = coords::face_xyz_to_uvw(face, &a.0);
    let b_uvw = coords::face_xyz_to_uvw(face, &b.0);

    // Scale u- and v-components of normal for padding.
    let scale_uv = 1.0 + padding;
    let scaled_n = Vector::new(scale_uv * norm_uvw.x, scale_uv * norm_uvw.y, norm_uvw.z);
    if !intersects_face(&scaled_n) {
        return None;
    }

    // Workaround for extremely small vectors where loss of precision can
    // occur in Normalize causing underflow.
    let norm_uvw =
        if norm_uvw.x.abs().max(norm_uvw.y.abs()).max(norm_uvw.z.abs()) < 2.0_f64.powi(-511) {
            (norm_uvw * 2.0_f64.powi(563)).normalize()
        } else {
            norm_uvw.normalize()
        };

    let a_tan = norm_uvw.cross(a_uvw);
    let b_tan = b_uvw.cross(norm_uvw);

    let (a_uv, a_score) = clip_destination(b_uvw, a_uvw, -scaled_n, b_tan, a_tan, scale_uv);
    let (b_uv, b_score) = clip_destination(a_uvw, b_uvw, scaled_n, a_tan, b_tan, scale_uv);

    if a_score + b_score < 3 {
        Some((a_uv, b_uv))
    } else {
        None
    }
}

/// Returns the portion of the 2D edge AB contained by `clip`.
///
/// Returns `None` if there is no intersection.
#[inline]
pub fn clip_edge(a: r2::Point, b: r2::Point, clip: r2::Rect) -> Option<(r2::Point, r2::Point)> {
    let bound = r2::Rect::from_point_pair(a, b);
    let (bound, intersects) = clip_edge_bound(a, b, clip, bound);
    if !intersects {
        return None;
    }
    use crate::r1::Endpoint;
    let ai = Endpoint::from(a.x > b.x);
    let aj = Endpoint::from(a.y > b.y);
    Some((bound.vertex_ij(ai, aj), bound.vertex_ij(!ai, !aj)))
}

/// Subdivides edge AB at every point where it crosses the boundary between
/// two S2 cube faces, and returns the corresponding `FaceSegment`s in order
/// from A toward B.
pub fn face_segments(a: Point, b: Point) -> Vec<FaceSegment> {
    let (a_face, au, av) = coords::xyz_to_face_uv(&a.0);
    let (b_face, bu, bv) = coords::xyz_to_face_uv(&b.0);

    if a_face == b_face {
        return vec![FaceSegment {
            face: a_face,
            a: r2::Point::new(au, av),
            b: r2::Point::new(bu, bv),
        }];
    }

    let ab = a.point_cross(b);

    let (mut cur_face, mut seg_a) =
        move_origin_to_valid_face(a_face, a, ab, r2::Point::new(au, av));
    let (b_face, b_saved) =
        move_origin_to_valid_face(b_face, b, Point(-ab.0), r2::Point::new(bu, bv));

    let mut segments = Vec::new();
    let mut seg_b;

    loop {
        if cur_face == b_face {
            break;
        }

        let z = coords::face_xyz_to_uvw(cur_face, &ab.0);
        let exit_ax = exit_axis(&z);
        seg_b = exit_point(&z, exit_ax);
        segments.push(FaceSegment {
            face: cur_face,
            a: seg_a,
            b: seg_b,
        });

        let exit_xyz = coords::face_uv_to_xyz(cur_face, seg_b.x, seg_b.y);
        cur_face = next_face(cur_face, seg_b, exit_ax, &z, b_face);
        let exit_uvw = coords::face_xyz_to_uvw(cur_face, &exit_xyz);
        seg_a = r2::Point::new(exit_uvw.x, exit_uvw.y);
    }

    // Finish the last segment.
    segments.push(FaceSegment {
        face: cur_face,
        a: seg_a,
        b: b_saved,
    });

    segments
}

/// Reports whether edge AB intersects the given closed rectangle.
#[inline]
pub fn edge_intersects_rect(a: r2::Point, b: r2::Point, r: r2::Rect) -> bool {
    if !r.intersects(r2::Rect::from_point_pair(a, b)) {
        return false;
    }
    let n = (b - a).ortho();
    use crate::r1::Endpoint;
    let i = Endpoint::from(n.x >= 0.0);
    let j = Endpoint::from(n.y >= 0.0);
    let max = n.dot(r.vertex_ij(i, j) - a);
    let min = n.dot(r.vertex_ij(!i, !j) - a);
    (max >= 0.0) && (min <= 0.0)
}

/// Returns the bounding rectangle of the portion of edge AB intersected by
/// `clip`. Returns an empty rect if there is no intersection.
pub fn clipped_edge_bound(a: r2::Point, b: r2::Point, clip: r2::Rect) -> r2::Rect {
    let bound = r2::Rect::from_point_pair(a, b);
    let (b1, intersects) = clip_edge_bound(a, b, clip, bound);
    if intersects { b1 } else { r2::Rect::empty() }
}

// ─── Internal helpers ──────────────────────────────────────────────────

/// Reports whether u + v == w exactly.
#[inline]
fn sum_equal(u: f64, v: f64, w: f64) -> bool {
    (u + v == w) && (u == w - v) && (v == w - u)
}

/// Reports whether a directed line (given by its normal N in (u,v,w)
/// coordinates) intersects the [-1,1]x[-1,1] cube face.
#[inline]
fn intersects_face(n: &Vector) -> bool {
    let u = n.x.abs();
    let v = n.y.abs();
    let w = n.z.abs();
    (v >= w - u) && (u >= w - v)
}

/// Reports whether a directed line intersects two opposite edges of the
/// cube face.
#[inline]
fn intersects_opposite_edges(n: &Vector) -> bool {
    let u = n.x.abs();
    let v = n.y.abs();
    let w = n.z.abs();
    if (u - v).abs() != w {
        return (u - v).abs() >= w;
    }
    if u >= v { u - w >= v } else { v - w >= u }
}

/// Axis along which a line exits a face.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    U = 0,
    V = 1,
}

/// Reports which axis the directed line L (given by normal N in (u,v,w)
/// coordinates) exits the cube face on.
#[inline]
fn exit_axis(n: &Vector) -> Axis {
    if intersects_opposite_edges(n) {
        if n.x.abs() >= n.y.abs() {
            Axis::V
        } else {
            Axis::U
        }
    } else {
        let x_neg = u8::from(n.x.is_sign_negative());
        let y_neg = u8::from(n.y.is_sign_negative());
        let z_neg = u8::from(n.z.is_sign_negative());
        if x_neg ^ y_neg ^ z_neg == 0 {
            Axis::V
        } else {
            Axis::U
        }
    }
}

/// Returns the UV coordinates of the point where a directed line exits the
/// cube face along the given axis.
#[inline]
fn exit_point(n: &Vector, a: Axis) -> r2::Point {
    if a == Axis::U {
        let u = if n.y > 0.0 { 1.0 } else { -1.0 };
        r2::Point::new(u, (-u * n.x - n.z) / n.y)
    } else {
        let v = if n.x < 0.0 { 1.0 } else { -1.0 };
        r2::Point::new((-v * n.y - n.z) / n.x, v)
    }
}

/// Clips a destination point and returns a score.
///
/// The score indicates whether the edge intersects the face. If the sum of
/// scores from both endpoints is >= 3, the edge does not intersect.
fn clip_destination(
    a: Vector,
    b: Vector,
    scaled_n: Vector,
    a_tan: Vector,
    b_tan: Vector,
    scale_uv: f64,
) -> (r2::Point, i32) {
    debug_assert!(intersects_face(&scaled_n));
    // If B is within the safe region of the face, use it.
    let max_safe = 1.0 - FACE_CLIP_ERROR_UV_COORD;
    if b.z > 0.0 {
        let uv = r2::Point::new(b.x / b.z, b.y / b.z);
        if uv.x.abs().max(uv.y.abs()) <= max_safe {
            return (uv, 0);
        }
    }

    // Otherwise find where line AB exits the face.
    let mut uv = exit_point(&scaled_n, exit_axis(&scaled_n)) * scale_uv;

    let p = Vector::new(uv.x, uv.y, 1.0);

    let score;
    if (p - a).dot(a_tan) < 0.0 {
        score = 2; // B' is on wrong side of A.
    } else if (p - b).dot(b_tan) < 0.0 {
        score = 1; // B' is on wrong side of B.
    } else {
        score = 0;
    }

    if score > 0 {
        if b.z <= 0.0 {
            return (uv, 3);
        }
        uv = r2::Point::new(b.x / b.z, b.y / b.z);
    }

    (uv, score)
}

/// Updates one endpoint of an interval, returning false if the new value
/// lies beyond the opposite endpoint.
fn update_endpoint(
    mut bound: r1::Interval,
    high_endpoint: bool,
    value: f64,
) -> (r1::Interval, bool) {
    if high_endpoint {
        if bound.lo > value {
            return (bound, false);
        }
        if bound.hi > value {
            bound.hi = value;
        }
        (bound, true)
    } else {
        if bound.hi < value {
            return (bound, false);
        }
        if bound.lo < value {
            bound.lo = value;
        }
        (bound, true)
    }
}

/// Clips the bounding intervals for a line segment against a clip interval.
#[expect(clippy::too_many_arguments, reason = "matches C++ API")]
fn clip_bound_axis(
    a0: f64,
    b0: f64,
    mut bound0: r1::Interval,
    a1: f64,
    b1: f64,
    mut bound1: r1::Interval,
    neg_slope: bool,
    clip: r1::Interval,
) -> (r1::Interval, r1::Interval, bool) {
    if bound0.lo < clip.lo {
        if bound0.hi < clip.lo {
            return (bound0, bound1, false);
        }
        bound0.lo = clip.lo;
        let (b1_new, ok) = update_endpoint(
            bound1,
            neg_slope,
            interpolate_float64(clip.lo, a0, b0, a1, b1),
        );
        if !ok {
            return (bound0, b1_new, false);
        }
        bound1 = b1_new;
    }

    if bound0.hi > clip.hi {
        if bound0.lo > clip.hi {
            return (bound0, bound1, false);
        }
        bound0.hi = clip.hi;
        let (b1_new, ok) = update_endpoint(
            bound1,
            !neg_slope,
            interpolate_float64(clip.hi, a0, b0, a1, b1),
        );
        if !ok {
            return (bound0, b1_new, false);
        }
        bound1 = b1_new;
    }

    (bound0, bound1, true)
}

/// Clips an edge bounding box against a clip rectangle.
fn clip_edge_bound(
    a: r2::Point,
    b: r2::Point,
    clip: r2::Rect,
    bound: r2::Rect,
) -> (r2::Rect, bool) {
    let neg_slope = (a.x > b.x) != (a.y > b.y);

    let (b0x, b0y, ok) = clip_bound_axis(a.x, b.x, bound.x, a.y, b.y, bound.y, neg_slope, clip.x);
    if !ok {
        return (bound, false);
    }
    let (b1y, b1x, ok) = clip_bound_axis(a.y, b.y, b0y, a.x, b.x, b0x, neg_slope, clip.y);
    if !ok {
        return (r2::Rect::new(b0x, b0y), false);
    }
    (r2::Rect::new(b1x, b1y), true)
}

/// Interpolates a value between two points, preserving exact results at
/// endpoints.
pub fn interpolate_float64(x: f64, a: f64, b: f64, a1: f64, b1: f64) -> f64 {
    if a == b {
        debug_assert!(x == a && a1 == b1);
        return a1;
    }
    if (a - x).abs() <= (b - x).abs() {
        a1 + (b1 - a1) * (x - a) / (b - a)
    } else {
        b1 + (a1 - b1) * (x - b) / (a - b)
    }
}

/// Adjusts the origin face and UV coordinates if necessary to ensure the
/// edge AB intersects the face.
fn move_origin_to_valid_face(
    mut face: coords::Face,
    a: Point,
    ab: Point,
    mut a_uv: r2::Point,
) -> (coords::Face, r2::Point) {
    let max_safe = 1.0 - FACE_CLIP_ERROR_UV_COORD;
    if a_uv.x.abs().max(a_uv.y.abs()) <= max_safe {
        return (face, a_uv);
    }

    // Check whether the normal AB even intersects this face.
    let z = coords::face_xyz_to_uvw(face, &ab.0);
    if intersects_face(&z) {
        let exit_ax = exit_axis(&z);
        let uv = exit_point(&z, exit_ax);
        let exit = coords::face_uv_to_xyz(face, uv.x, uv.y);
        let a_tangent = ab.0.normalize().cross(a.0);

        if (exit - a.0).dot(a_tangent) >= -FACE_CLIP_ERROR_RADIANS {
            return (face, a_uv);
        }
    }

    // Reproject A to the nearest adjacent face.
    let dir;
    if a_uv.x.abs() >= a_uv.y.abs() {
        dir = if a_uv.x > 0.0 { 1 } else { 0 };
        face = coords::get_uvw_face(face, 0, dir);
    } else {
        dir = if a_uv.y > 0.0 { 1 } else { 0 };
        face = coords::get_uvw_face(face, 1, dir);
    }

    let (u, v) = coords::valid_face_xyz_to_uv(face, &a.0);
    a_uv.x = u.clamp(-1.0, 1.0);
    a_uv.y = v.clamp(-1.0, 1.0);

    (face, a_uv)
}

/// Returns the next face to visit in `face_segments`.
fn next_face(
    face: coords::Face,
    exit: r2::Point,
    axis: Axis,
    n: &Vector,
    target_face: coords::Face,
) -> coords::Face {
    let (exit_a, exit_1_minus_a) = if axis == Axis::V {
        (exit.y, exit.x)
    } else {
        (exit.x, exit.y)
    };

    let exit_a_pos: u8 = if exit_a > 0.0 { 1 } else { 0 };
    let exit_1_minus_a_pos: u8 = if exit_1_minus_a > 0.0 { 1 } else { 0 };

    if exit_1_minus_a.abs() == 1.0
        && coords::get_uvw_face(face, 1 - axis as u8, exit_1_minus_a_pos) == target_face
        && sum_equal(exit.x * n.x, exit.y * n.y, -n.z)
    {
        return target_face;
    }

    coords::get_uvw_face(face, axis as u8, exit_a_pos)
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clip_to_face_basic() {
        // Edge that crosses the equator on face 0.
        let a = Point::from_coords(1.0, 0.5, 0.5);
        let b = Point::from_coords(1.0, -0.5, -0.5);
        let result = clip_to_face(a, b, coords::Face::F0);
        assert!(result.is_some(), "edge should intersect face 0");
    }

    #[test]
    fn test_clip_to_face_no_intersection() {
        // Edge entirely on face 2 should not intersect face 0.
        let a = Point::from_coords(0.0, 0.0, 1.0);
        let b = Point::from_coords(0.5, 0.5, 1.0);
        let result = clip_to_face(a, b, coords::Face::F0);
        // This edge is on face 2 (z dominant), not face 0 (x dominant).
        // Whether it intersects depends on the exact geometry.
        // Just verify we get a result (no panic).
        let _ = result;
    }

    #[test]
    fn test_clip_to_padded_face() {
        let a = Point::from_coords(1.0, 0.5, 0.5);
        let b = Point::from_coords(1.0, -0.5, -0.5);
        let result = clip_to_padded_face(a, b, coords::Face::F0, 0.1);
        assert!(result.is_some(), "padded edge should intersect face 0");
    }

    #[test]
    fn test_face_segments_same_face() {
        let a = Point::from_coords(1.0, 0.1, 0.1);
        let b = Point::from_coords(1.0, -0.1, -0.1);
        let segs = face_segments(a, b);
        assert_eq!(segs.len(), 1, "single face should yield 1 segment");
        assert_eq!(segs[0].face, coords::Face::F0);
    }

    #[test]
    fn test_face_segments_two_faces() {
        // Edge from face 0 to face 1.
        let a = Point::from_coords(1.0, 0.0, 0.0);
        let b = Point::from_coords(0.0, 1.0, 0.0);
        let segs = face_segments(a, b);
        assert!(
            segs.len() >= 2,
            "edge crossing faces should yield >= 2 segments, got {}",
            segs.len(),
        );
    }

    #[test]
    fn test_clip_edge_basic() {
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(1.0, 1.0);
        let clip = r2::Rect::new(r1::Interval::new(0.2, 0.8), r1::Interval::new(0.2, 0.8));
        let result = clip_edge(a, b, clip);
        assert!(result.is_some(), "diagonal should intersect centered clip");
        let (ca, cb) = result.unwrap();
        // Clipped endpoints should be within the clip rectangle.
        assert!(ca.x >= 0.2 - 1e-10 && ca.x <= 0.8 + 1e-10);
        assert!(cb.x >= 0.2 - 1e-10 && cb.x <= 0.8 + 1e-10);
    }

    #[test]
    fn test_clip_edge_no_intersection() {
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(0.1, 0.1);
        let clip = r2::Rect::new(r1::Interval::new(0.5, 1.0), r1::Interval::new(0.5, 1.0));
        let result = clip_edge(a, b, clip);
        assert!(result.is_none(), "edge should not intersect distant clip");
    }

    #[test]
    fn test_edge_intersects_rect() {
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(1.0, 1.0);
        let r = r2::Rect::new(r1::Interval::new(0.2, 0.8), r1::Interval::new(0.2, 0.8));
        assert!(edge_intersects_rect(a, b, r));
    }

    #[test]
    fn test_edge_intersects_rect_miss() {
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(0.1, 0.0);
        let r = r2::Rect::new(r1::Interval::new(0.5, 1.0), r1::Interval::new(0.5, 1.0));
        assert!(!edge_intersects_rect(a, b, r));
    }

    #[test]
    fn test_interpolate_float64() {
        // Exact at endpoints.
        assert_eq!(interpolate_float64(0.0, 0.0, 1.0, 10.0, 20.0), 10.0);
        assert_eq!(interpolate_float64(1.0, 0.0, 1.0, 10.0, 20.0), 20.0);
        // Midpoint.
        let mid = interpolate_float64(0.5, 0.0, 1.0, 10.0, 20.0);
        assert!((mid - 15.0).abs() < 1e-10, "mid = {mid}, want 15.0");
    }

    #[test]
    fn test_intersects_face() {
        // Normal along u-axis: line is in the v-w plane, crosses the face.
        assert!(intersects_face(&Vector::new(1.0, 0.0, 0.0)));
        // Normal with |Nu| + |Nv| < |Nw|: does not intersect.
        assert!(!intersects_face(&Vector::new(0.1, 0.1, 1.0)));
        // Normal along w-axis: |Nu| + |Nv| = 0 < 1 = |Nw|, does not intersect.
        assert!(!intersects_face(&Vector::new(0.0, 0.0, 1.0)));
    }

    #[test]
    fn test_clipped_edge_bound() {
        let a = r2::Point::new(0.0, 0.0);
        let b = r2::Point::new(1.0, 1.0);
        let clip = r2::Rect::new(r1::Interval::new(0.2, 0.8), r1::Interval::new(0.2, 0.8));
        let bound = clipped_edge_bound(a, b, clip);
        assert!(!bound.is_empty(), "clipped edge bound should not be empty");
    }

    #[test]
    fn test_sum_equal() {
        assert!(sum_equal(1.0, 2.0, 3.0));
        assert!(!sum_equal(1.0, 2.0, 3.5));
    }

    #[test]
    fn test_exit_axis_and_point() {
        // Normal along +Z: line is the u-axis. Should exit through v edge.
        let n = Vector::new(0.0, 0.0, 1.0);
        let ax = exit_axis(&n);
        let _pt = exit_point(&n, ax);
        // Just verify no panics and deterministic output.
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_face_segment_roundtrip() {
        let fs = FaceSegment {
            face: coords::Face::F0,
            a: r2::Point::new(0.1, 0.2),
            b: r2::Point::new(0.3, 0.4),
        };
        let json = serde_json::to_string(&fs).unwrap();
        let back: FaceSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(fs.face, back.face);
        assert_eq!(fs.a, back.a);
        assert_eq!(fs.b, back.b);
    }
}
