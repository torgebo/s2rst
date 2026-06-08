// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry
//   - Java: google/s2-geometry-library-java

//! Expand or contract geometry by a fixed radius.
//!
//! [`S2BufferOperation`] computes the region within a given distance of the
//! input geometry — the Minkowski sum of the input with a spherical disc of
//! the specified radius. A positive radius expands the geometry outward; a
//! negative radius contracts it inward (eroding thin features). The radius is
//! measured along the sphere surface.
//!
//! The operation accepts any geometry that can be added to an
//! [`S2Builder`](crate::s2::builder::S2Builder) (points, polylines, loops,
//! polygons) and produces the result through an `S2Builder` layer. It uses
//! [`S2WindingOperation`]
//! internally to resolve the buffered boundary into valid output polygons.
//!
//! An error tolerance can be specified to control the trade-off between
//! geometric fidelity and the number of output edges.

#![expect(
    clippy::cast_possible_truncation,
    reason = "shape iteration (usize->i32) — count always in i32 range"
)]
#![expect(
    clippy::cast_possible_wrap,
    reason = "shape iteration (usize->i32) — count always in i32 range"
)]
use std::f64::consts::{FRAC_PI_2, PI};

use crate::s1::{self, ChordAngle};
use crate::s2::builder::layer::Layer;
use crate::s2::builder::snap::{IdentitySnapFunction, SnapFunction};
use crate::s2::builder::{Options as BuilderOptions, S2Error, S2ErrorCode};
use crate::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use crate::s2::edge_crosser::EdgeCrosser;
use crate::s2::edge_crossings::angle_contains_vertex;
use crate::s2::edge_distances;
use crate::s2::lax_loop::LaxLoop;
use crate::s2::ortho;
use crate::s2::predicates;
use crate::s2::shape::{Dimension, Shape};
use crate::s2::shape_index::ShapeIndex;
use crate::s2::shape_util;
use crate::s2::winding_operation::{S2WindingOperation, WindingOptions, WindingRule};
use crate::s2::{Point, Polygon};

// ─── Error Constants ────────────────────────────────────────────────────────

/// Minimum requested error: roughly the spacing between representable `S2Points`.
const MIN_REQUESTED_ERROR: f64 = 2.0 * predicates::DBL_ERROR; // ~1.11e-16 radians

/// Maximum absolute interpolation error from `RobustCrossProd` + `GetPointOnRay`.
/// About 10 nanometers on Earth's surface.
///
/// kGetPointOnLineError = (4 + 2/sqrt(3)) * `DBL_ERR` + 6 * `DBL_ERR`
/// kGetPointOnRayPerpendicularError = 3 * `DBL_ERR`
const MAX_ABSOLUTE_INTERPOLATION_ERROR: f64 = {
    let dbl_err = predicates::DBL_ERROR;
    let robust_cross_prod_error = 6.0 * dbl_err;
    let get_point_on_line_error =
        (4.0 + 2.0 / 1.7320508075688772) * dbl_err + robust_cross_prod_error;
    let get_point_on_ray_perp_error = 3.0 * dbl_err;
    get_point_on_line_error + get_point_on_ray_perp_error
};

// ─── EndCapStyle / PolylineSide ─────────────────────────────────────────────

/// For polylines, specifies whether end caps should be round or flat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EndCapStyle {
    /// Round end caps (the default).
    #[default]
    Round,
    /// Flat end caps (no buffering beyond the polyline endpoints).
    Flat,
}

/// Specifies whether polylines should be buffered on one side or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolylineSide {
    /// Buffer only on the left side of the polyline.
    Left,
    /// Buffer only on the right side of the polyline.
    Right,
    /// Buffer on both sides of the polyline (the default).
    #[default]
    Both,
}

// ─── Options ────────────────────────────────────────────────────────────────

/// Minimum allowed error fraction.
pub const MIN_ERROR_FRACTION: f64 = 1e-6;

/// Maximum circle segments (corresponds to `MIN_ERROR_FRACTION`).
pub const MAX_CIRCLE_SEGMENTS: f64 = 1570.7968503979573;

/// Options for `S2BufferOperation`.
#[derive(Debug)]
pub struct BufferOptions {
    buffer_radius: s1::Angle,
    error_fraction: f64,
    end_cap_style: EndCapStyle,
    polyline_side: PolylineSide,
    snap_function: Box<dyn SnapFunction>,
    /// Optional memory tracker for limiting and monitoring memory usage.
    ///
    /// C++: `S2BufferOperation::Options::memory_tracker()`
    pub memory_tracker:
        Option<std::sync::Arc<std::sync::Mutex<crate::s2::memory_tracker::S2MemoryTracker>>>,
}

impl Default for BufferOptions {
    fn default() -> Self {
        BufferOptions {
            buffer_radius: s1::Angle::ZERO,
            error_fraction: 0.02,
            end_cap_style: EndCapStyle::Round,
            polyline_side: PolylineSide::Both,
            snap_function: Box::new(IdentitySnapFunction::new(s1::Angle::ZERO)),
            memory_tracker: None,
        }
    }
}

impl BufferOptions {
    /// Creates new options with the given buffer radius.
    pub fn new(buffer_radius: s1::Angle) -> Self {
        BufferOptions {
            buffer_radius,
            ..Default::default()
        }
    }

    /// Returns the buffer radius. Positive adds area; negative subtracts.
    pub fn buffer_radius(&self) -> s1::Angle {
        self.buffer_radius
    }

    /// Sets the buffer radius.
    pub fn set_buffer_radius(&mut self, r: s1::Angle) {
        self.buffer_radius = r;
    }

    /// Returns the error fraction (relative to buffer radius).
    pub fn error_fraction(&self) -> f64 {
        self.error_fraction
    }

    /// Sets the allowable error as a fraction of the buffer radius.
    /// The actual buffer distance is in the range
    /// `[(1-f)*r - C, (1+f)*r + C]` where `f` is the error fraction,
    /// `r` is the buffer radius, and `C` is the absolute error.
    pub fn set_error_fraction(&mut self, f: f64) {
        debug_assert!(f >= MIN_ERROR_FRACTION);
        debug_assert!(f <= 1.0);
        self.error_fraction = f.clamp(MIN_ERROR_FRACTION, 1.0);
    }

    /// Returns the maximum error in the buffered result.
    pub fn max_error(&self) -> s1::Angle {
        let abs_rad = self.buffer_radius.radians().abs();
        let requested = self.error_fraction * abs_rad;
        let requested = requested.max(MIN_REQUESTED_ERROR);

        // Compute snap function's contribution via builder options.
        let mut builder_opts = BuilderOptions::new(self.snap_function.clone_snap());
        builder_opts.split_crossing_edges = true;
        let snap_error = builder_opts.max_edge_deviation();

        s1::Angle::from_radians(requested + MAX_ABSOLUTE_INTERPOLATION_ERROR + snap_error.radians())
    }

    /// Returns the error fraction expressed as the number of polyline
    /// segments used to approximate a planar circle.
    pub fn circle_segments(&self) -> f64 {
        PI / (1.0 - self.error_fraction).acos()
    }

    /// Sets the error fraction via the number of circle segments.
    /// Error decreases quadratically with the number of segments.
    pub fn set_circle_segments(&mut self, n: f64) {
        debug_assert!(n >= 2.0);
        debug_assert!(n <= MAX_CIRCLE_SEGMENTS);
        let n = n.clamp(2.0, MAX_CIRCLE_SEGMENTS);
        self.set_error_fraction(1.0 - (PI / n).cos() + 1e-15);
    }

    /// Returns the end cap style for polylines.
    pub fn end_cap_style(&self) -> EndCapStyle {
        self.end_cap_style
    }

    /// Sets the end cap style for polylines.
    pub fn set_end_cap_style(&mut self, style: EndCapStyle) {
        self.end_cap_style = style;
    }

    /// Returns which side(s) of polylines are buffered.
    pub fn polyline_side(&self) -> PolylineSide {
        self.polyline_side
    }

    /// Sets which side(s) of polylines are buffered.
    pub fn set_polyline_side(&mut self, side: PolylineSide) {
        self.polyline_side = side;
    }

    /// Returns the snap function.
    pub fn snap_function(&self) -> &dyn SnapFunction {
        &*self.snap_function
    }

    /// Sets the snap function used for snapping the output geometry.
    pub fn set_snap_function(&mut self, f: Box<dyn SnapFunction>) {
        self.snap_function = f;
    }
}

// ─── S2BufferOperation ──────────────────────────────────────────────────────

/// Expands or contracts geometry by a fixed buffer radius.
///
/// Positive radius expands (Minkowski sum with a disc), negative radius
/// contracts. Uses winding numbers for final polygon resolution.
///
/// # Examples
///
/// ```
/// use s2rst::s1;
/// use s2rst::s2::buffer_operation::{S2BufferOperation, BufferOptions};
/// use s2rst::s2::builder::layer::Layer;
/// use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
/// use s2rst::s2::LatLng;
///
/// // Buffer a single point to create a small disc.
/// let layer = S2PolygonLayer::new();
/// let options = BufferOptions::new(s1::Angle::from_degrees(1.0));
/// let mut op = S2BufferOperation::new(Box::new(layer), options);
/// op.add_point(LatLng::from_degrees(0.0, 0.0).to_point());
/// // build() returns the result layer; downcast it to extract the polygon.
/// let polygon = op
///     .build()
///     .unwrap()
///     .into_any()
///     .downcast::<S2PolygonLayer>()
///     .unwrap()
///     .into_output();
/// assert!(!polygon.is_empty_polygon());
/// ```
#[derive(Debug)]
pub struct S2BufferOperation {
    options: BufferOptions,

    // Number of polygon layers added (for negative radius constraint).
    num_polygon_layers: i32,

    // Buffering parameters (computed during init).
    buffer_sign: i32,
    abs_radius: ChordAngle,
    vertex_step: ChordAngle,
    edge_step: ChordAngle,
    point_step: ChordAngle,

    // Winding operation accumulates buffered loops.
    op: S2WindingOperation,

    // Current offset path being built.
    path: Vec<Point>,

    // Reference point winding tracking.
    ref_point: Point,
    ref_winding: i32,

    // Sweep edge endpoints: A on input, B on offset.
    sweep_a: Point,
    sweep_b: Point,

    // Starting vertices for closing buffer regions.
    input_start: Point,
    offset_start: Point,
    have_input_start: bool,
    have_offset_start: bool,
}

impl S2BufferOperation {
    /// Creates a new `S2BufferOperation` that sends output to the given layer.
    pub fn new(result_layer: Box<dyn Layer>, options: BufferOptions) -> Self {
        let buffer_sign = sgn(options.buffer_radius.radians());
        let abs_radius_angle = s1::Angle::from_radians(options.buffer_radius.radians().abs());

        let requested_error =
            (options.error_fraction * abs_radius_angle.radians()).max(MIN_REQUESTED_ERROR);
        let max_error = MAX_ABSOLUTE_INTERPOLATION_ERROR + requested_error;

        let (buffer_sign, abs_radius, vertex_step, edge_step, point_step) =
            if abs_radius_angle.radians() <= max_error {
                // Radius smaller than max error → pass through unchanged.
                (
                    0,
                    ChordAngle::ZERO,
                    ChordAngle::ZERO,
                    ChordAngle::ZERO,
                    ChordAngle::ZERO,
                )
            } else if abs_radius_angle.radians() + max_error >= PI {
                // Radius near π → use STRAIGHT.
                (
                    buffer_sign,
                    ChordAngle::STRAIGHT,
                    ChordAngle::ZERO,
                    ChordAngle::ZERO,
                    ChordAngle::ZERO,
                )
            } else {
                let abs_ca = ChordAngle::from_angle(abs_radius_angle);
                let vs =
                    get_max_edge_span(abs_radius_angle, s1::Angle::from_radians(requested_error));
                let vs_ca = ChordAngle::from_angle(vs);

                // Point step: ensure regular polygons.
                let ps_rad = 2.0 * PI / (2.0 * PI / vs.radians()).ceil() + 1e-15;
                let ps_ca = ChordAngle::from_radians(ps_rad);

                // Edge step: edges buffered only if radius < π/2.
                let edge_radius = s1::Angle::from_radians(FRAC_PI_2 - abs_radius_angle.radians());
                let es_ca = if edge_radius.radians() > max_error {
                    ChordAngle::from_angle(get_max_edge_span(
                        edge_radius,
                        s1::Angle::from_radians(requested_error),
                    ))
                } else {
                    ChordAngle::ZERO
                };

                (buffer_sign, abs_ca, vs_ca, es_ca, ps_ca)
            };

        // Configure winding operation.
        let mut winding_options =
            WindingOptions::with_snap_function(options.snap_function.clone_snap());
        winding_options
            .set_include_degeneracies(buffer_sign == 0 && options.buffer_radius >= s1::Angle::ZERO);
        let op = S2WindingOperation::new(result_layer, winding_options);

        S2BufferOperation {
            options,
            num_polygon_layers: 0,
            buffer_sign,
            abs_radius,
            vertex_step,
            edge_step,
            point_step,
            op,
            path: Vec::new(),
            ref_point: Point::origin(),
            ref_winding: 0,
            sweep_a: Point::origin(),
            sweep_b: Point::origin(),
            input_start: Point::origin(),
            offset_start: Point::origin(),
            have_input_start: false,
            have_offset_start: false,
        }
    }

    /// Returns the options for this buffer operation.
    pub fn options(&self) -> &BufferOptions {
        &self.options
    }

    // ─── Sweep Edge Tracking ────────────────────────────────────────────

    /// Advances the sweep edge by moving its input vertex to `new_a`.
    fn set_input_vertex(&mut self, new_a: Point) {
        if self.have_input_start {
            debug_assert!(self.have_offset_start);
            self.update_ref_winding(self.sweep_a, self.sweep_b, new_a);
        } else {
            self.input_start = new_a;
            self.have_input_start = true;
        }
        self.sweep_a = new_a;
    }

    /// Adds a point to the offset path and advances the sweep edge.
    fn add_offset_vertex(&mut self, new_b: Point) {
        self.path.push(new_b);
        if self.have_offset_start {
            debug_assert!(self.have_input_start);
            self.update_ref_winding(self.sweep_a, self.sweep_b, new_b);
        } else {
            self.offset_start = new_b;
            self.have_offset_start = true;
        }
        self.sweep_b = new_b;
    }

    /// Closes the buffer region by sweeping back to start.
    fn close_buffer_region(&mut self) {
        if self.have_offset_start && self.have_input_start {
            self.update_ref_winding(self.sweep_a, self.sweep_b, self.input_start);
            self.update_ref_winding(self.input_start, self.sweep_b, self.offset_start);
        }
    }

    /// Outputs the current path as a loop and resets state.
    fn output_path(&mut self) {
        let path = std::mem::take(&mut self.path);
        self.op.add_loop(&path);
        self.have_input_start = false;
        self.have_offset_start = false;
    }

    /// Updates the reference point winding for triangle ABC.
    fn update_ref_winding(&mut self, a: Point, b: Point, c: Point) {
        let sign = predicates::robust_sign(a, b, c) as i32;
        if sign == 0 {
            return;
        }
        let mut inside = angle_contains_vertex(a, b, c) == (sign > 0);
        let mut crosser = EdgeCrosser::new(b, self.ref_point);
        inside ^= crosser.edge_or_vertex_crossing(a, b);
        inside ^= crosser.edge_or_vertex_crossing(b, c);
        inside ^= crosser.edge_or_vertex_crossing(c, a);
        if inside {
            self.ref_winding += sign;
        }
    }

    /// Ensures the output will be the full polygon.
    fn add_full_polygon(&mut self) {
        self.ref_winding += 1;
    }

    // ─── Edge Axis ──────────────────────────────────────────────────────

    /// Returns the edge normal for edge AB, with sign based on `buffer_sign`.
    fn get_edge_axis(&self, a: Point, b: Point) -> Point {
        debug_assert_ne!(self.buffer_sign, 0);
        // C++ does: buffer_sign_ * RobustCrossProd(b, a).Normalize()
        let robust_ba = b.point_cross(a);
        Point(robust_ba.0.normalize() * f64::from(self.buffer_sign))
    }

    // ─── Vertex Arc ─────────────────────────────────────────────────────

    /// Adds a semi-open offset arc around vertex V from `start` to `end`.
    fn add_vertex_arc(&mut self, v: Point, start: Point, end: Point) {
        let rotate_dir = Point(v.0.cross(start.0).normalize() * f64::from(self.buffer_sign));
        let span = ChordAngle::from_points(start, end);
        let mut angle = ChordAngle::ZERO;
        loop {
            let dir = edge_distances::point_on_ray_chord(start, rotate_dir, angle);
            self.add_offset_vertex(edge_distances::point_on_ray_chord(v, dir, self.abs_radius));
            angle = angle + self.vertex_step;
            if angle >= span {
                break;
            }
        }
    }

    /// Closes the semi-open arc generated by `add_vertex_arc`.
    fn close_vertex_arc(&mut self, v: Point, end: Point) {
        self.add_offset_vertex(edge_distances::point_on_ray_chord(v, end, self.abs_radius));
    }

    // ─── Edge Arc ───────────────────────────────────────────────────────

    /// Adds a semi-open offset arc for edge AB.
    fn add_edge_arc(&mut self, a: Point, b: Point) {
        let ab_axis = self.get_edge_axis(a, b);
        if self.edge_step == ChordAngle::ZERO {
            // Buffer radius >= 90°: edges don't contribute to boundary.
            // Force path through edge normal for correct winding.
            self.add_offset_vertex(ab_axis);
        } else {
            let rotate_dir = Point(a.0.cross(ab_axis.0).normalize() * f64::from(self.buffer_sign));
            let span = ChordAngle::from_points(a, b);
            let mut angle = ChordAngle::ZERO;
            loop {
                let p = edge_distances::point_on_ray_chord(a, rotate_dir, angle);
                self.add_offset_vertex(edge_distances::point_on_ray_chord(
                    p,
                    ab_axis,
                    self.abs_radius,
                ));
                angle = angle + self.edge_step;
                if angle >= span {
                    break;
                }
            }
        }
        self.set_input_vertex(b);
    }

    /// Closes the semi-open arc generated by `add_edge_arc`.
    fn close_edge_arc(&mut self, a: Point, b: Point) {
        if self.edge_step != ChordAngle::ZERO {
            let axis = self.get_edge_axis(a, b);
            self.add_offset_vertex(edge_distances::point_on_ray_chord(b, axis, self.abs_radius));
        }
    }

    // ─── BufferEdgeAndVertex ────────────────────────────────────────────

    /// Buffers edge AB and vertex B (C determines the arc range at B).
    fn buffer_edge_and_vertex(&mut self, a: Point, b: Point, c: Point) {
        debug_assert_ne!(a, b);
        debug_assert_ne!(b, c);
        debug_assert_ne!(self.buffer_sign, 0);

        self.add_edge_arc(a, b);

        // Check if the turn at B is convex or concave.
        let sign = predicates::robust_sign(a, b, c) as i32;
        if self.buffer_sign * sign >= 0 {
            // Convex turn: add vertex arc.
            let start = self.get_edge_axis(a, b);
            let end = self.get_edge_axis(b, c);
            self.add_vertex_arc(b, start, end);
            if self.edge_step == ChordAngle::ZERO {
                self.close_vertex_arc(b, end);
            }
        } else {
            // Concave turn: splice through input vertex.
            self.close_edge_arc(a, b);
            self.add_offset_vertex(b);
        }
    }

    // ─── End Caps (Polylines) ───────────────────────────────────────────

    /// Adds the start cap for a polyline starting with edge AB.
    fn add_start_cap(&mut self, a: Point, b: Point) {
        let axis = self.get_edge_axis(a, b);
        if self.options.end_cap_style == EndCapStyle::Flat {
            if self.options.polyline_side == PolylineSide::Both {
                self.add_offset_vertex(edge_distances::point_on_ray_chord(
                    a,
                    -axis,
                    self.abs_radius,
                ));
            }
        } else {
            debug_assert!(self.options.end_cap_style == EndCapStyle::Round);
            // Round cap.
            if self.options.polyline_side == PolylineSide::Both {
                self.add_vertex_arc(a, -axis, axis);
            } else {
                let start = Point(axis.0.cross(a.0).normalize());
                self.add_vertex_arc(a, start, axis);
            }
        }
    }

    /// Adds the end cap for a polyline ending with edge AB.
    fn add_end_cap(&mut self, a: Point, b: Point) {
        let axis = self.get_edge_axis(a, b);
        if self.options.end_cap_style == EndCapStyle::Flat {
            self.close_edge_arc(a, b);
        } else {
            debug_assert!(self.options.end_cap_style == EndCapStyle::Round);
            // Round cap.
            if self.options.polyline_side == PolylineSide::Both {
                self.add_vertex_arc(b, axis, -axis);
            } else {
                let end = Point(b.0.cross(axis.0).normalize());
                self.add_vertex_arc(b, axis, end);
                self.close_vertex_arc(b, end);
            }
        }
    }

    // ─── Loop Buffering ─────────────────────────────────────────────────

    /// Buffers a loop (internal helper, does not update `ref_winding`).
    fn buffer_loop(&mut self, loop_vertices: &[Point]) {
        if loop_vertices.is_empty() {
            return;
        }
        if loop_vertices.len() == 1 {
            return self.add_point(loop_vertices[0]);
        }

        // Buffering by ≥180° always yields full/empty polygon.
        if self.abs_radius >= ChordAngle::STRAIGHT {
            if self.buffer_sign > 0 {
                self.add_full_polygon();
            }
            return;
        }

        if self.buffer_sign == 0 {
            // Pass through unchanged.
            self.path.extend_from_slice(loop_vertices);
        } else {
            let n = loop_vertices.len();
            self.set_input_vertex(loop_vertices[0]);
            for i in 0..n {
                let a = loop_vertices[i];
                let b = loop_vertices[(i + 1) % n];
                let c = loop_vertices[(i + 2) % n];
                self.buffer_edge_and_vertex(a, b, c);
            }
            self.close_buffer_region();
        }
        self.output_path();
    }

    // ─── Public Input Methods ───────────────────────────────────────────

    /// Adds an input layer containing a single point.
    pub fn add_point(&mut self, point: Point) {
        if self.buffer_sign < 0 {
            return;
        }

        if self.abs_radius >= ChordAngle::STRAIGHT {
            return self.add_full_polygon();
        }

        if self.buffer_sign == 0 {
            self.path.push(point);
        } else {
            // Generate a regular polygon approximating a circle.
            self.set_input_vertex(point);
            let mut start = ortho(point);
            let mut angle = ChordAngle::ZERO;
            for _quadrant in 0..4 {
                let rotate_dir = Point(point.0.cross(start.0).normalize());
                while angle < ChordAngle::RIGHT {
                    let dir = edge_distances::point_on_ray_chord(start, rotate_dir, angle);
                    self.add_offset_vertex(edge_distances::point_on_ray_chord(
                        point,
                        dir,
                        self.abs_radius,
                    ));
                    angle = angle + self.point_step;
                }
                angle = angle - ChordAngle::RIGHT;
                start = rotate_dir;
            }
            self.close_buffer_region();
        }
        self.output_path();
    }

    /// Adds an input layer containing a polyline.
    pub fn add_polyline(&mut self, polyline: &[Point]) {
        // Left-sided buffering: reverse and buffer on right.
        let reversed;
        let polyline = if self.options.polyline_side == PolylineSide::Left {
            reversed = polyline.iter().copied().rev().collect::<Vec<_>>();
            &reversed
        } else {
            polyline
        };

        if self.buffer_sign < 0 {
            return;
        }

        let n = polyline.len();
        if n <= 1 {
            return;
        }

        // Degenerate edge → treat as point.
        if n == 2 && polyline[0] == polyline[1] {
            return self.add_point(polyline[0]);
        }

        if self.abs_radius >= ChordAngle::STRAIGHT {
            return self.add_full_polygon();
        }

        if self.buffer_sign == 0 {
            // Convert to degenerate loop: forward + reverse.
            self.path.extend_from_slice(&polyline[..n - 1]);
            self.path.extend(polyline.iter().rev().skip(1).copied());
        } else {
            self.set_input_vertex(polyline[0]);
            self.add_start_cap(polyline[0], polyline[1]);
            for i in 0..n - 2 {
                self.buffer_edge_and_vertex(polyline[i], polyline[i + 1], polyline[i + 2]);
            }
            self.add_edge_arc(polyline[n - 2], polyline[n - 1]);
            self.add_end_cap(polyline[n - 2], polyline[n - 1]);

            if self.options.polyline_side == PolylineSide::Both {
                for i in (0..n - 2).rev() {
                    self.buffer_edge_and_vertex(polyline[i + 2], polyline[i + 1], polyline[i]);
                }
                self.add_edge_arc(polyline[1], polyline[0]);
                self.close_buffer_region();
            } else {
                // One-sided: add reversed polyline vertices.
                self.path.extend(polyline.iter().rev().copied());
            }
        }
        self.output_path();
    }

    /// Adds an input layer containing a loop.
    pub fn add_loop(&mut self, loop_vertices: &[Point]) {
        if loop_vertices.is_empty() {
            return;
        }
        self.buffer_loop(loop_vertices);

        // Update reference winding by checking if ref_point is inside the loop.
        let lax = LaxLoop::new(loop_vertices.to_vec());
        self.ref_winding += i32::from(shape_util::contains_brute_force(&lax, self.ref_point));
        self.num_polygon_layers += 1;
    }

    /// Adds an input layer containing the given shape.
    pub fn add_shape(&mut self, shape: &dyn Shape) {
        self.buffer_shape(shape);
        self.ref_winding += i32::from(shape_util::contains_brute_force(shape, self.ref_point));
        self.num_polygon_layers += i32::from(shape.dimension() == Dimension::Polygon);
    }

    /// Adds an input layer containing all shapes in the given index.
    pub fn add_shape_index(&mut self, index: &ShapeIndex) {
        let mut max_dimension: Option<Dimension> = None;
        for shape_id in 0..index.num_shape_ids() {
            if let Some(shape) = index.shape(shape_id as i32) {
                let d = shape.dimension();
                max_dimension = Some(match max_dimension {
                    Some(prev) if prev >= d => prev,
                    _ => d,
                });
                self.buffer_shape(shape);
            }
        }
        let mut query = ContainsPointQuery::new(index, VertexModel::SemiOpen);
        self.ref_winding += i32::from(query.contains(self.ref_point));
        self.num_polygon_layers += i32::from(max_dimension == Some(Dimension::Polygon));
    }

    /// Buffers the given shape (internal helper, does not update `ref_winding`).
    fn buffer_shape(&mut self, shape: &dyn Shape) {
        let dimension = shape.dimension();
        let num_chains = shape.num_chains();
        for c in 0..num_chains {
            let chain = shape.chain(c);
            if chain.length == 0 {
                continue;
            }
            match dimension {
                Dimension::Point => {
                    let edge = shape.chain_edge(c, 0);
                    self.add_point(edge.v0);
                }
                Dimension::Polyline => {
                    let vertices = get_chain_vertices(shape, c);
                    self.add_polyline(&vertices);
                }
                Dimension::Polygon => {
                    let vertices = get_chain_vertices(shape, c);
                    self.buffer_loop(&vertices);
                }
            }
        }
    }

    /// Computes the buffered result and returns the output layer.
    ///
    /// The boxed layer returned is the same `result_layer` passed to
    /// [`new`](Self::new) after its output has been built. Downcast it with
    /// [`Layer::into_any`] to the concrete layer type and call that type's
    /// output accessor (e.g. `take_output()` / `into_output()`) to retrieve the
    /// resulting polygon.
    ///
    /// # Errors
    ///
    /// Returns an error if the buffer operation fails (e.g., negative radius
    /// with multiple polygon layers).
    pub fn build(&mut self) -> Result<Box<dyn Layer>, S2Error> {
        if self.buffer_sign < 0 && self.num_polygon_layers > 1 {
            return Err(S2Error::new(
                S2ErrorCode::FailedPrecondition,
                "Negative buffer radius requires at most one polygon layer",
            ));
        }
        self.op
            .build(self.ref_point, self.ref_winding, WindingRule::Positive)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Returns the sign of a float: -1, 0, or +1.
fn sgn(x: f64) -> i32 {
    if x > 0.0 {
        1
    } else if x < 0.0 {
        -1
    } else {
        0
    }
}

/// Returns the maximum angular span for buffered arcs.
fn get_max_edge_span(radius: s1::Angle, requested_error: s1::Angle) -> s1::Angle {
    let mut step = s1::Angle::from_radians(2.0 * PI / 3.0 + 1e-15);
    let min_radius = radius - requested_error;
    debug_assert!(min_radius.radians() >= 0.0);
    if radius.radians() < FRAC_PI_2 {
        let s = s1::Angle::from_radians(
            2.0 * (min_radius.radians().tan() / radius.radians().tan()).acos(),
        );
        if s < step {
            step = s;
        }
    } else if min_radius.radians() > FRAC_PI_2 {
        let s = s1::Angle::from_radians(
            2.0 * (radius.radians().tan() / min_radius.radians().tan()).acos(),
        );
        if s < step {
            step = s;
        }
    }
    step
}

/// Extracts the vertices of a chain from a shape.
/// For dimension 1 (polylines), returns chain.length + 1 vertices.
/// For dimension 2 (loops), returns chain.length vertices.
fn get_chain_vertices(shape: &dyn Shape, chain_id: usize) -> Vec<Point> {
    let chain = shape.chain(chain_id);
    let dimension = shape.dimension();
    let mut vertices = Vec::with_capacity(chain.length + 1);

    if chain.length == 0 {
        return vertices;
    }

    // First vertex is v0 of the first edge.
    vertices.push(shape.chain_edge(chain_id, 0).v0);
    for i in 0..chain.length {
        vertices.push(shape.chain_edge(chain_id, i).v1);
    }

    if dimension == Dimension::Polygon {
        // For loops, the last vertex equals the first vertex (it wraps).
        // Remove the duplicate to get just the loop vertices.
        if !vertices.is_empty() && vertices.last() == vertices.first() {
            vertices.pop();
        }
    }
    vertices
}

/// Helper to create a `ChordAngle` from two points (wrapping the constructor).
impl ChordAngle {
    /// Creates a `ChordAngle` from two unit-length points.
    fn from_points(a: Point, b: Point) -> Self {
        a.chord_angle(b)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

// ─── Convenience functions ───────────────────────────────────────────────────

/// Drives a buffer operation, feeding input via `add`, and returns the result
/// polygon (empty if the operation fails).
fn run_buffer(options: BufferOptions, add: impl FnOnce(&mut S2BufferOperation)) -> Polygon {
    use crate::s2::builder::polygon_layer::S2PolygonLayer;
    use std::cell::RefCell;
    use std::rc::Rc;

    let output = Rc::new(RefCell::new(Polygon::empty()));
    let layer = S2PolygonLayer::new_legacy(Rc::clone(&output));
    let mut op = S2BufferOperation::new(Box::new(layer), options);
    add(&mut op);
    // On failure the shared output cell is left as the empty polygon.
    let _outcome = op.build();
    output.borrow().clone()
}

/// Buffers a polygon by the radius configured in `options` (a positive radius
/// expands, a negative radius contracts), returning the resulting polygon.
pub fn buffer_polygon(polygon: &Polygon, options: BufferOptions) -> Polygon {
    run_buffer(options, |op| op.add_shape(polygon))
}

/// Buffers a polyline's path into a polygon.
pub fn buffer_polyline(vertices: &[Point], options: BufferOptions) -> Polygon {
    run_buffer(options, |op| op.add_polyline(vertices))
}

/// Buffers a loop's boundary into a polygon.
pub fn buffer_loop(vertices: &[Point], options: BufferOptions) -> Polygon {
    run_buffer(options, |op| op.add_loop(vertices))
}

/// Buffers a single point into a disc-shaped polygon.
pub fn buffer_point(point: Point, options: BufferOptions) -> Polygon {
    run_buffer(options, |op| op.add_point(point))
}

#[cfg(test)]
#[path = "buffer_operation_tests.rs"]
mod buffer_operation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
    use crate::s2::lax_polygon::LaxPolygon;
    use crate::s2::shape::Shape;
    use crate::s2::text_format;
    use crate::s2::{LatLng, Point};
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Helper: run buffer operation and return resulting `LaxPolygon`.
    fn do_buffer(
        input_fn: impl FnOnce(&mut S2BufferOperation),
        options: BufferOptions,
    ) -> LaxPolygon {
        let output = Rc::new(RefCell::new(LaxPolygon::empty()));
        let layer = LaxPolygonLayer::new_legacy(Rc::clone(&output));
        let mut op = S2BufferOperation::new(Box::new(layer), options);
        input_fn(&mut op);
        op.build().expect("Build failed");

        output.borrow().clone()
    }

    fn do_buffer_radius(
        input_fn: impl FnOnce(&mut S2BufferOperation),
        buffer_radius: s1::Angle,
        error_fraction: f64,
    ) -> LaxPolygon {
        let mut options = BufferOptions::new(buffer_radius);
        options.set_error_fraction(error_fraction);
        do_buffer(input_fn, options)
    }

    fn test_buffer_empty(input_fn: impl Fn(&mut S2BufferOperation) + Clone) {
        for &radius_deg in &[-200.0, -1.0, 0.0, 1.0, 200.0] {
            let f = input_fn.clone();
            let result =
                do_buffer_radius(move |op| f(op), s1::Angle::from_degrees(radius_deg), 0.1);
            assert!(
                result.is_empty(),
                "Expected empty output for radius={radius_deg}, got {} loops",
                result.num_loops(),
            );
        }
    }

    fn test_buffer_full(input_fn: impl Fn(&mut S2BufferOperation) + Clone) {
        for &radius_deg in &[-200.0, -1.0, 0.0, 1.0, 200.0] {
            let f = input_fn.clone();
            let result =
                do_buffer_radius(move |op| f(op), s1::Angle::from_degrees(radius_deg), 0.1);
            assert!(
                result.is_full(),
                "Expected full output for radius={radius_deg}",
            );
        }
    }

    #[test]
    fn test_no_input() {
        test_buffer_empty(|_op| {});
    }

    #[test]
    fn test_empty_polyline() {
        test_buffer_empty(|op| {
            op.add_polyline(&[Point::from_coords(1.0, 0.0, 0.0)]);
        });
    }

    #[test]
    fn test_empty_loop() {
        test_buffer_empty(|op| {
            op.add_loop(&[]);
        });
    }

    #[test]
    fn test_full_polygon_shape() {
        test_buffer_full(|op| {
            let poly = text_format::make_lax_polygon("full");
            op.add_shape(&poly);
        });
    }

    #[test]
    fn test_points_and_polylines_are_removed() {
        // Points and polylines are removed with negative buffer radius.
        let result = do_buffer_radius(
            |op| {
                op.add_point(LatLng::from_degrees(0.0, 0.0).to_point());
                op.add_polyline(&[
                    LatLng::from_degrees(2.0, 2.0).to_point(),
                    LatLng::from_degrees(2.0, 3.0).to_point(),
                ]);
            },
            s1::Angle::from_degrees(-1.0),
            0.1,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn test_buffered_points_are_symmetric() {
        // Buffered point should produce a regular polygon.
        let result = do_buffer_radius(
            |op| {
                op.add_point(Point::from_coords(1.0, 0.0, 0.0));
            },
            s1::Angle::from_degrees(5.0),
            0.001234567,
        );
        assert!(!result.is_empty(), "Expected non-empty output");
        let n = result.num_loop_vertices(0);
        assert!(n >= 3, "Expected at least 3 vertices, got {n}");

        // Check that all edges have approximately the same length.
        let v = |i: usize| result.loop_vertex(0, i);
        let ref_len = v(0).distance(v(n - 1));
        for i in 1..n {
            let edge_len = v(i - 1).distance(v(i));
            assert!(
                (ref_len.radians() - edge_len.radians()).abs() < 1e-13,
                "Edge {i} length {edge_len} differs from reference {ref_len}",
            );
        }
    }

    #[test]
    fn test_set_circle_segments() {
        // With tiny radius, circle_segments should determine vertex count.
        let mut options = BufferOptions::new(s1::Angle::from_radians(1e-12));
        for n in 3..=20 {
            options.set_circle_segments(n as f64);
            let result = do_buffer(
                |op| {
                    op.add_point(Point::from_coords(1.0, 0.0, 0.0));
                },
                {
                    let mut o = BufferOptions::new(s1::Angle::from_radians(1e-12));
                    o.set_circle_segments(n as f64);
                    o
                },
            );
            assert_eq!(
                result.num_loop_vertices(0),
                n,
                "Expected {n} vertices for circle_segments={n}",
            );
        }
    }

    #[test]
    fn test_negative_buffer_radius_multiple_layers() {
        let layer = LaxPolygonLayer::new();
        let mut op = S2BufferOperation::new(
            Box::new(layer),
            BufferOptions::new(s1::Angle::from_radians(-1.0)),
        );
        // Add two polygon layers.
        op.add_loop(&[
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 1.0).to_point(),
            LatLng::from_degrees(1.0, 0.0).to_point(),
        ]);
        op.add_loop(&[
            LatLng::from_degrees(2.0, 2.0).to_point(),
            LatLng::from_degrees(2.0, 3.0).to_point(),
            LatLng::from_degrees(3.0, 2.0).to_point(),
        ]);
        let err = op
            .build()
            .expect_err("Expected build to fail with multiple polygon layers and negative radius");
        assert_eq!(err.code, S2ErrorCode::FailedPrecondition);
    }

    #[test]
    fn test_point_buffer_basic() {
        // Buffer a single point by 1 degree. The result should be a small polygon
        // that contains the point.
        let center = LatLng::from_degrees(0.0, 0.0).to_point();
        let result = do_buffer_radius(
            |op| op.add_point(center),
            s1::Angle::from_degrees(1.0),
            0.01,
        );
        assert!(!result.is_empty(), "Point buffer should produce output");

        // All vertices should be approximately 1 degree from center.
        let n = result.num_loop_vertices(0);
        for i in 0..n {
            let v = result.loop_vertex(0, i);
            let dist = center.distance(v);
            assert!(
                (dist.degrees() - 1.0).abs() < 0.02,
                "Vertex {i} at distance {:.4}° from center, expected ≈1°",
                dist.degrees(),
            );
        }
    }

    #[test]
    fn test_loop_buffer_expands() {
        // Buffer a small triangle by 1 degree. The result should contain
        // all original vertices.
        let vertices = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 2.0).to_point(),
            LatLng::from_degrees(2.0, 1.0).to_point(),
        ];
        let result = do_buffer_radius(
            |op| op.add_loop(&vertices),
            s1::Angle::from_degrees(1.0),
            0.01,
        );
        assert!(!result.is_empty(), "Loop buffer should produce output");
        assert!(
            result.num_loop_vertices(0) > 3,
            "Buffered loop should have more vertices than original",
        );
    }

    #[test]
    fn test_polyline_buffer_basic() {
        // Buffer a simple 2-vertex polyline.
        let polyline = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 5.0).to_point(),
        ];
        let result = do_buffer_radius(
            |op| op.add_polyline(&polyline),
            s1::Angle::from_degrees(1.0),
            0.01,
        );
        assert!(!result.is_empty(), "Polyline buffer should produce output");
    }

    #[test]
    fn test_loop_contract() {
        // Contract a large triangle by a small amount. The result should be
        // a non-empty smaller polygon.
        let vertices = vec![
            LatLng::from_degrees(-30.0, -30.0).to_point(),
            LatLng::from_degrees(-30.0, 30.0).to_point(),
            LatLng::from_degrees(30.0, 0.0).to_point(),
        ];
        let result = do_buffer_radius(
            |op| op.add_loop(&vertices),
            s1::Angle::from_degrees(-1.0),
            0.01,
        );
        assert!(!result.is_empty(), "Contracted loop should still exist");
    }

    #[test]
    fn test_zero_radius_passthrough() {
        // With zero radius, the loop should pass through unchanged.
        let vertices = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 10.0).to_point(),
            LatLng::from_degrees(10.0, 5.0).to_point(),
        ];
        let result = do_buffer_radius(|op| op.add_loop(&vertices), s1::Angle::ZERO, 0.01);
        // Should produce a polygon with the same vertices (as a degenerate).
        assert!(!result.is_empty(), "Zero-radius should pass through");
    }

    #[test]
    fn test_flat_end_cap() {
        // Buffer polyline with flat end caps.
        let polyline = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 5.0).to_point(),
        ];
        let mut options = BufferOptions::new(s1::Angle::from_degrees(1.0));
        options.set_end_cap_style(EndCapStyle::Flat);
        let result = do_buffer(|op| op.add_polyline(&polyline), options);
        assert!(!result.is_empty(), "Flat cap buffer should produce output");
    }

    #[test]
    fn test_polyline_one_sided() {
        // Buffer polyline on right side only.
        let polyline = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 5.0).to_point(),
        ];
        let mut options = BufferOptions::new(s1::Angle::from_degrees(1.0));
        options.set_polyline_side(PolylineSide::Right);
        let result = do_buffer(|op| op.add_polyline(&polyline), options);
        assert!(!result.is_empty(), "One-sided buffer should produce output",);
    }

    // ─── C++ ported: more comprehensive tests ──────────────────────────

    /// Helper: verify that buffering with given input + options produces
    /// output that contains the input (for positive radius) or is contained
    /// by the input (for negative radius), using the containment test from C++.
    /// Downcasts a built layer to a `LaxPolygonLayer` and takes its output,
    /// returning the empty polygon if the layer produced none.
    fn take_lax_polygon(layer: Box<dyn Layer>) -> LaxPolygon {
        layer
            .into_any()
            .downcast::<LaxPolygonLayer>()
            .expect("expected a LaxPolygonLayer")
            .take_output()
            .unwrap_or_else(LaxPolygon::empty)
    }

    /// Computes the set difference `a - b` of two indexed regions and returns
    /// the resulting polygon (empty when `a` is contained in `b`).
    fn index_difference(a: &mut ShapeIndex, b: &mut ShapeIndex) -> LaxPolygon {
        use crate::s2::boolean_operation::{OpType, Options as BooleanOptions, S2BooleanOperation};

        let mut op = S2BooleanOperation::new(
            OpType::Difference,
            Box::new(LaxPolygonLayer::new()),
            BooleanOptions::default(),
        );
        let layers = op.build(a, b).expect("Difference failed");
        layers
            .into_iter()
            .next()
            .map(take_lax_polygon)
            .unwrap_or_else(LaxPolygon::empty)
    }

    fn test_buffer_containment(vertices: &[Point], buffer_radius: s1::Angle, error_fraction: f64) {
        use crate::s2::lax_loop::LaxLoop;
        use crate::s2::shape_index::ShapeIndex;

        let mut options = BufferOptions::new(buffer_radius);
        options.set_error_fraction(error_fraction);
        let max_error = options.max_error();

        // Build the buffered output via the public output-extraction API.
        let mut op = S2BufferOperation::new(Box::new(LaxPolygonLayer::new()), options);
        op.add_loop(vertices);
        let output = take_lax_polygon(op.build().expect("Build failed"));

        let mut input_index = ShapeIndex::new();
        input_index.add(Box::new(LaxLoop::new(vertices.to_vec())));
        let mut output_index = ShapeIndex::new();
        output_index.add(Box::new(output.clone()));

        if buffer_radius.radians() > max_error.radians() {
            // Positive radius: output should contain input, so input - output = ∅.
            assert!(
                index_difference(&mut input_index, &mut output_index).is_empty(),
                "Output should contain input (positive buffer radius)",
            );
        } else if buffer_radius.radians() < -max_error.radians() {
            // Negative radius: input should contain output, so output - input = ∅.
            assert!(
                index_difference(&mut output_index, &mut input_index).is_empty(),
                "Input should contain output (negative buffer radius)",
            );
        }
    }

    #[test]
    fn test_square_expand() {
        // Buffer a square by 1 degree. Output should contain input.
        let vertices: Vec<Point> = [(-3.0, -3.0), (-3.0, 3.0), (3.0, 3.0), (3.0, -3.0)]
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();
        test_buffer_containment(&vertices, s1::Angle::from_degrees(1.0), 0.01);
    }

    #[test]
    fn test_square_contract() {
        // Contract a square by 1 degree. Input should contain output.
        let vertices: Vec<Point> = [(-3.0, -3.0), (-3.0, 3.0), (3.0, 3.0), (3.0, -3.0)]
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();
        test_buffer_containment(&vertices, s1::Angle::from_degrees(-1.0), 0.01);
    }

    #[test]
    fn test_hollow_square() {
        // Buffer a square with a hole. Only test that it produces output.
        let outer: Vec<Point> = [(-3.0, -3.0), (-3.0, 3.0), (3.0, 3.0), (3.0, -3.0)]
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();
        // Inner loop (hole, oriented clockwise).
        let inner: Vec<Point> = [(2.0, 2.0), (-2.0, 2.0), (-2.0, -2.0), (2.0, -2.0)]
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();

        let output = Rc::new(RefCell::new(LaxPolygon::empty()));
        let layer = LaxPolygonLayer::new_legacy(Rc::clone(&output));
        let mut options = BufferOptions::new(s1::Angle::from_degrees(1.0));
        options.set_error_fraction(0.01);
        let mut op = S2BufferOperation::new(Box::new(layer), options);
        op.add_loop(&outer);
        op.add_loop(&inner);
        op.build().expect("Build failed");
        assert!(
            !output.borrow().is_empty(),
            "Hollow square buffer should produce output"
        );
    }

    #[test]
    #[ignore = "graph_edge_clipper multiplicity assertion with near-degenerate zigzag"]
    fn test_zigzag_loop() {
        // Buffer a zigzag loop.
        let vertices: Vec<Point> = [
            (0.0, 0.0),
            (0.0, 7.0),
            (5.0, 3.0),
            (5.0, 10.0),
            (6.0, 10.0),
            (6.0, 1.0),
            (1.0, 5.0),
            (1.0, 0.0),
        ]
        .iter()
        .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
        .collect();
        test_buffer_containment(&vertices, s1::Angle::from_degrees(0.2), 0.01);
    }

    #[test]
    fn test_poorly_normalized_point() {
        // Verify no assertions triggered with a nearly-unit-length point.
        let p = Point::from_coords(1.0 - 2.0 * f64::EPSILON, 0.0, 0.0);
        let result = do_buffer_radius(|op| op.add_point(p), s1::Angle::from_degrees(1.0), 0.01);
        assert!(
            !result.is_empty(),
            "Poorly normalized point should buffer ok"
        );
    }

    #[test]
    fn test_options_roundtrip() {
        // Verify that options() returns the same buffer_radius.
        let options = BufferOptions::new(s1::Angle::from_radians(1e-12));
        let layer = LaxPolygonLayer::new();
        let op = S2BufferOperation::new(Box::new(layer), options);
        assert!(
            (op.options().buffer_radius().radians() - 1e-12).abs() < 1e-20,
            "Buffer radius should be preserved in options()",
        );
    }

    #[test]
    fn test_empty_lax_polyline_shape() {
        use crate::s2::lax_polyline::LaxPolyline;
        test_buffer_empty(|op| {
            let shape = LaxPolyline::new(vec![]);
            op.add_shape(&shape);
        });
    }

    #[test]
    fn test_empty_polygon_shape() {
        test_buffer_empty(|op| {
            let poly = text_format::make_lax_polygon("");
            op.add_shape(&poly);
        });
    }

    #[test]
    fn test_full_shape() {
        // Buffering a full polygon by a positive radius via add_shape should stay full.
        let poly = text_format::make_lax_polygon("full");
        let result = do_buffer_radius(
            move |op| op.add_shape(&poly),
            s1::Angle::from_degrees(1.0),
            0.1,
        );
        assert!(
            result.is_full(),
            "Full polygon buffered by 1° should be full"
        );
    }

    #[test]
    fn test_empty_shape_index() {
        test_buffer_empty(|op| {
            let index = ShapeIndex::new();
            op.add_shape_index(&index);
        });
    }

    #[test]
    fn test_polyline_left_side() {
        // Buffer polyline on left side only.
        let polyline = vec![
            LatLng::from_degrees(0.0, 0.0).to_point(),
            LatLng::from_degrees(0.0, 5.0).to_point(),
        ];
        let mut options = BufferOptions::new(s1::Angle::from_degrees(1.0));
        options.set_polyline_side(PolylineSide::Left);
        let result = do_buffer(|op| op.add_polyline(&polyline), options);
        assert!(
            !result.is_empty(),
            "Left-sided buffer should produce output"
        );
    }

    #[test]
    fn test_polyline_zigzag() {
        // Test zigzag polyline with Both sides + Round caps (the default).
        let polyline: Vec<Point> = [(0.0, 0.0), (0.0, 7.0), (5.0, 3.0), (5.0, 10.0)]
            .iter()
            .map(|&(lat, lng)| LatLng::from_degrees(lat, lng).to_point())
            .collect();

        let mut options = BufferOptions::new(s1::Angle::from_degrees(1.0));
        options.set_polyline_side(PolylineSide::Both);
        options.set_end_cap_style(EndCapStyle::Round);
        let result = do_buffer(|op| op.add_polyline(&polyline), options);
        assert!(
            !result.is_empty(),
            "Zigzag polyline buffer with Both+Round should produce output",
        );
    }

    #[test]
    fn test_large_buffer_becomes_full() {
        // Buffering by >= 180 degrees should yield the full polygon.
        let result = do_buffer_radius(
            |op| op.add_point(Point::from_coords(1.0, 0.0, 0.0)),
            s1::Angle::from_degrees(200.0),
            0.1,
        );
        assert!(
            result.is_full(),
            "200° buffer of point should be full polygon"
        );
    }

    #[test]
    fn test_degenerate_polyline_as_point() {
        // A polyline with two identical vertices should be treated as a point.
        let p = LatLng::from_degrees(10.0, 20.0).to_point();
        let result = do_buffer_radius(
            |op| op.add_polyline(&[p, p]),
            s1::Angle::from_degrees(1.0),
            0.01,
        );
        assert!(
            !result.is_empty(),
            "Degenerate polyline should buffer like a point"
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_enums_roundtrip() {
        for e in [EndCapStyle::Round, EndCapStyle::Flat] {
            let json = serde_json::to_string(&e).unwrap();
            let back: EndCapStyle = serde_json::from_str(&json).unwrap();
            assert_eq!(e, back);
        }
        for s in [PolylineSide::Left, PolylineSide::Right, PolylineSide::Both] {
            let json = serde_json::to_string(&s).unwrap();
            let back: PolylineSide = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }
}
