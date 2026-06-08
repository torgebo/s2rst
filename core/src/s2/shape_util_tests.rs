// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::shape_to_points`]: it returns a typed
//! `S2Error` for a non-point shape instead of asserting/panicking. Written for
//! this crate, not ported from upstream S2.

use super::*;
use crate::s2::text_format;

#[test]
fn shape_to_points_accepts_point_shape() {
    let index = text_format::make_index("0:0 | 1:1 | 2:2 # #");
    let shape = index.shape(0).unwrap();
    let pts = shape_to_points(shape).expect("point shape should succeed");
    assert_eq!(pts.len(), 3);
}

#[test]
fn shape_to_points_rejects_polyline_shape() {
    let shape = text_format::make_lax_polyline("0:0, 1:0, 2:0");
    let err = shape_to_points(&shape).expect_err("polyline is not dimension 0");
    assert_eq!(err.code, S2ErrorCode::InvalidArgument);
}

#[test]
fn shape_to_points_rejects_polygon_shape() {
    let shape = text_format::make_lax_polygon("0:0, 0:1, 1:0");
    let err = shape_to_points(&shape).expect_err("polygon is not dimension 0");
    assert_eq!(err.code, S2ErrorCode::InvalidArgument);
}
