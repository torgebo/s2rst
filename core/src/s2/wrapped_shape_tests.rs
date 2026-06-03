// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for [`super::WrappedShape`], covering the delegation
//! paths (`inner`, `reference_point`, `chain`, `chain_edge`, `chain_position`)
//! not exercised by the in-file tests. Written for this crate, not ported.

use std::sync::Arc;

use super::WrappedShape;
use crate::s2::Point;
use crate::s2::edge_vector_shape::EdgeVectorShape;
use crate::s2::shape::Shape;

fn pt(x: f64, y: f64, z: f64) -> Point {
    Point::from_coords(x, y, z)
}

fn two_edge_inner() -> EdgeVectorShape {
    let mut evs = EdgeVectorShape::new();
    evs.add(pt(1.0, 0.0, 0.0), pt(0.0, 1.0, 0.0));
    evs.add(pt(0.0, 1.0, 0.0), pt(0.0, 0.0, 1.0));
    evs
}

#[test]
fn inner_exposes_the_wrapped_shape() {
    let arc: Arc<dyn Shape> = Arc::new(two_edge_inner());
    let w = WrappedShape::new(arc);
    // `inner()` hands back the wrapped shape; delegating through it agrees with
    // delegating through the wrapper.
    assert_eq!(w.inner().num_edges(), 2);
    assert_eq!(w.inner().edge(0), w.edge(0));
}

#[test]
fn delegates_chain_and_reference_point_methods() {
    let inner = two_edge_inner();
    // Capture the inner shape's answers before it is moved behind the `Arc`,
    // then assert the wrapper returns the identical values.
    let exp_rp = inner.reference_point();
    let exp_chain0 = inner.chain(0);
    let exp_chain_edge = inner.chain_edge(0, 0);
    let exp_cp0 = inner.chain_position(0);
    let exp_cp1 = inner.chain_position(1);

    let arc: Arc<dyn Shape> = Arc::new(inner);
    let w = WrappedShape::new(arc);

    assert_eq!(w.reference_point(), exp_rp);
    assert_eq!(w.chain(0), exp_chain0);
    assert_eq!(w.chain_edge(0, 0), exp_chain_edge);
    assert_eq!(w.chain_position(0), exp_cp0);
    assert_eq!(w.chain_position(1), exp_cp1);
}
