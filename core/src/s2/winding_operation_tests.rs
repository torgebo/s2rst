// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for retrieving [`super::S2WindingOperation`]
//! output through the layer returned by `build()`. Written for this crate, not
//! ported from upstream S2.

use super::*;
use crate::s2::LatLng;
use crate::s2::builder::lax_polygon_layer::LaxPolygonLayer;
use crate::s2::builder::polygon_layer::S2PolygonLayer;

fn square() -> Vec<Point> {
    vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(0.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 10.0).to_point(),
        LatLng::from_degrees(10.0, 0.0).to_point(),
    ]
}

#[test]
fn build_returns_polygon_layer_output() {
    let mut op = S2WindingOperation::new(Box::new(S2PolygonLayer::new()), WindingOptions::new());
    op.add_loop(&square());

    let ref_p = LatLng::from_degrees(5.0, 5.0).to_point();
    let polygon = op
        .build(ref_p, 1, WindingRule::Positive)
        .expect("build should succeed")
        .into_any()
        .downcast::<S2PolygonLayer>()
        .expect("result layer is an S2PolygonLayer")
        .into_output();
    assert_eq!(polygon.num_loops(), 1);
}

#[test]
fn build_returns_lax_polygon_layer_output() {
    let mut op = S2WindingOperation::new(Box::new(LaxPolygonLayer::new()), WindingOptions::new());
    op.add_loop(&square());

    let ref_p = LatLng::from_degrees(5.0, 5.0).to_point();
    let lax = op
        .build(ref_p, 1, WindingRule::Positive)
        .expect("build should succeed")
        .into_any()
        .downcast::<LaxPolygonLayer>()
        .expect("result layer is a LaxPolygonLayer")
        .take_output()
        .expect("layer produced output");
    assert_eq!(lax.num_loops(), 1);
}
