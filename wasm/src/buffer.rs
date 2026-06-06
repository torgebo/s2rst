// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use crate::angle::Angle;
use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::s2loop::Loop;
use crate::snap::SnapFunction;

/// Options for buffering geometry (expanding/contracting by a radius).
///
/// A positive radius expands; a negative radius contracts (erodes). Setters
/// consume and return `self` for chaining.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct BufferOptions {
    radius_radians: f64,
    error_fraction: Option<f64>,
    circle_segments: Option<f64>,
    snap: Option<SnapFunction>,
}

#[wasm_bindgen]
impl BufferOptions {
    /// Create options with the given buffer radius (positive expands, negative
    /// contracts).
    #[wasm_bindgen(constructor)]
    pub fn new(radius: &Angle) -> BufferOptions {
        BufferOptions {
            radius_radians: radius.0.radians(),
            error_fraction: None,
            circle_segments: None,
            snap: None,
        }
    }

    /// Set the allowed approximation error as a fraction of the radius
    /// (clamped to the valid range).
    #[wasm_bindgen(js_name = "setErrorFraction")]
    pub fn set_error_fraction(mut self, fraction: f64) -> BufferOptions {
        self.error_fraction = Some(fraction);
        self
    }

    /// Set the number of segments approximating a full circle (clamped to the
    /// valid range).
    #[wasm_bindgen(js_name = "setCircleSegments")]
    pub fn set_circle_segments(mut self, segments: f64) -> BufferOptions {
        self.circle_segments = Some(segments);
        self
    }

    /// Set the snap function applied to the buffered result.
    #[wasm_bindgen(js_name = "setSnapFunction")]
    pub fn set_snap_function(mut self, snap: &SnapFunction) -> BufferOptions {
        self.snap = Some(snap.clone());
        self
    }
}

impl BufferOptions {
    fn build(&self) -> s2rst::s2::buffer_operation::BufferOptions {
        use s2rst::s2::buffer_operation::{MAX_CIRCLE_SEGMENTS, MIN_ERROR_FRACTION};
        let mut o = s2rst::s2::buffer_operation::BufferOptions::new(
            s2rst::s1::Angle::from_radians(self.radius_radians),
        );
        if let Some(f) = self.error_fraction {
            // Clamp to avoid the core debug_assert tripping on out-of-range input.
            o.set_error_fraction(f.clamp(MIN_ERROR_FRACTION, 1.0));
        }
        if let Some(n) = self.circle_segments {
            o.set_circle_segments(n.clamp(2.0, MAX_CIRCLE_SEGMENTS));
        }
        if let Some(s) = &self.snap {
            o.set_snap_function(s.build());
        }
        o
    }
}

/// Buffer a polygon (positive radius expands, negative contracts).
#[wasm_bindgen(js_name = "bufferPolygon")]
pub fn buffer_polygon(polygon: &Polygon, options: &BufferOptions) -> Polygon {
    Polygon(s2rst::s2::buffer_operation::buffer_polygon(
        &polygon.0,
        options.build(),
    ))
}

/// Buffer a point into a disc-shaped polygon.
#[wasm_bindgen(js_name = "bufferPoint")]
pub fn buffer_point(point: &Point, options: &BufferOptions) -> Polygon {
    Polygon(s2rst::s2::buffer_operation::buffer_point(
        point.0,
        options.build(),
    ))
}

/// Buffer a loop's boundary into a polygon.
#[wasm_bindgen(js_name = "bufferLoop")]
pub fn buffer_loop(loop_: &Loop, options: &BufferOptions) -> Polygon {
    let verts: Vec<s2rst::s2::Point> = loop_.0.vertices().to_vec();
    Polygon(s2rst::s2::buffer_operation::buffer_loop(
        &verts,
        options.build(),
    ))
}

/// Buffer a polyline's path into a polygon.
#[wasm_bindgen(js_name = "bufferPolyline")]
pub fn buffer_polyline(polyline: &Polyline, options: &BufferOptions) -> Polygon {
    let verts: Vec<s2rst::s2::Point> = polyline.0.vertices_vec().to_vec();
    Polygon(s2rst::s2::buffer_operation::buffer_polyline(
        &verts,
        options.build(),
    ))
}
