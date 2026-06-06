// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

use wasm_bindgen::prelude::*;

use s2rst::s2::builder::polygon_layer::S2PolygonLayer;
use s2rst::s2::builder::polyline_layer::S2PolylineLayer;
use s2rst::s2::builder::{Options, S2Builder};

use crate::error::{js_err, s2_error_to_js};
use crate::point::Point;
use crate::polygon::Polygon;
use crate::polyline::Polyline;
use crate::s2loop::Loop;
use crate::snap::SnapFunction;

/// Accumulated input geometry, replayed into a fresh `S2Builder` per build so
/// the same inputs can be assembled into either a polygon or a polyline.
#[derive(Clone, Debug)]
enum Input {
    Loop(s2rst::s2::Loop),
    Polygon(s2rst::s2::Polygon),
    Polyline(s2rst::s2::polyline::Polyline),
    Edge(s2rst::s2::Point, s2rst::s2::Point),
}

/// Assembles input geometry (loops, polygons, polylines, raw edges) into a
/// single snapped `Polygon` or `Polyline` using `S2Builder`.
///
/// Add geometry with the `add*` methods, then call `buildPolygon()` or
/// `buildPolyline()`. The builder is reusable — building does not consume the
/// added inputs, so you can build both output types from the same input.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Builder {
    snap: SnapFunction,
    inputs: Vec<Input>,
}

#[wasm_bindgen]
impl Builder {
    /// Create a builder using the given vertex snap function.
    #[wasm_bindgen(constructor)]
    pub fn new(snap: &SnapFunction) -> Builder {
        Builder {
            snap: snap.clone(),
            inputs: Vec::new(),
        }
    }

    /// Add a loop's edges as input.
    #[wasm_bindgen(js_name = "addLoop")]
    pub fn add_loop(&mut self, loop_: &Loop) {
        self.inputs.push(Input::Loop(loop_.0.clone()));
    }

    /// Add a polygon's edges as input.
    #[wasm_bindgen(js_name = "addPolygon")]
    pub fn add_polygon(&mut self, polygon: &Polygon) {
        self.inputs.push(Input::Polygon(polygon.0.clone()));
    }

    /// Add a polyline's edges as input.
    #[wasm_bindgen(js_name = "addPolyline")]
    pub fn add_polyline(&mut self, polyline: &Polyline) {
        self.inputs.push(Input::Polyline(polyline.0.clone()));
    }

    /// Add a single directed edge as input.
    #[wasm_bindgen(js_name = "addEdge")]
    pub fn add_edge(&mut self, a: &Point, b: &Point) {
        self.inputs.push(Input::Edge(a.0, b.0));
    }

    /// Assemble the added geometry into a single polygon. Throws on a builder
    /// error (e.g. inconsistent edges that cannot form a polygon).
    #[wasm_bindgen(js_name = "buildPolygon")]
    pub fn build_polygon(&self) -> Result<Polygon, JsValue> {
        let mut builder = S2Builder::new(Options::new(self.snap.build()));
        builder.start_layer(Box::new(S2PolygonLayer::new()));
        self.feed(&mut builder);
        let mut layers = builder.build().map_err(s2_error_to_js)?;
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolygonLayer>()
            .map_err(|_| js_err("internal: expected a polygon layer"))?;
        Ok(Polygon(layer.into_output()))
    }

    /// Assemble the added geometry into a single polyline. Throws on a builder
    /// error.
    #[wasm_bindgen(js_name = "buildPolyline")]
    pub fn build_polyline(&self) -> Result<Polyline, JsValue> {
        let mut builder = S2Builder::new(Options::new(self.snap.build()));
        builder.start_layer(Box::new(S2PolylineLayer::new()));
        self.feed(&mut builder);
        let mut layers = builder.build().map_err(s2_error_to_js)?;
        let layer = layers
            .remove(0)
            .into_any()
            .downcast::<S2PolylineLayer>()
            .map_err(|_| js_err("internal: expected a polyline layer"))?;
        Ok(Polyline(layer.into_output()))
    }
}

impl Builder {
    fn feed(&self, builder: &mut S2Builder) {
        for input in &self.inputs {
            match input {
                Input::Loop(l) => builder.add_loop(l),
                Input::Polygon(p) => builder.add_polygon(p),
                Input::Polyline(p) => builder.add_polyline(p),
                Input::Edge(a, b) => builder.add_edge(*a, *b),
            }
        }
    }
}
