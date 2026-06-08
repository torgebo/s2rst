// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for retrieving [`super::S2BufferOperation`] output
//! through the layer returned by `build()`. Written for this crate, not ported
//! from upstream S2.

use super::*;
use crate::s2::LatLng;
use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
use crate::s2::builder::polygon_layer::S2PolygonLayer;

#[test]
fn build_returns_polygon_layer_output() {
    let mut op = S2BufferOperation::new(
        Box::new(S2PolygonLayer::new()),
        BufferOptions::new(s1::Angle::from_degrees(1.0)),
    );
    op.add_point(LatLng::from_degrees(0.0, 0.0).to_point());

    let polygon = op
        .build()
        .expect("build should succeed")
        .into_any()
        .downcast::<S2PolygonLayer>()
        .expect("result layer is an S2PolygonLayer")
        .into_output();
    assert!(!polygon.is_empty_polygon());
}

#[test]
fn build_returns_lax_polygon_layer_output() {
    let mut op = S2BufferOperation::new(
        Box::new(LaxPolygonLayer::new()),
        BufferOptions::new(s1::Angle::from_degrees(1.0)),
    );
    op.add_point(LatLng::from_degrees(0.0, 0.0).to_point());

    let lax = op
        .build()
        .expect("build should succeed")
        .into_any()
        .downcast::<LaxPolygonLayer>()
        .expect("result layer is a LaxPolygonLayer")
        .take_output()
        .expect("layer produced output");
    assert!(!lax.is_empty());
}
