// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original unit tests for [`super::S2PointVectorLayer`], covering the
//! label-tracking build path and the output accessors not exercised by the
//! in-file tests. Written for this crate, not ported from upstream S2.

use super::*;
use crate::s2::text_format::parse_point;

#[test]
fn new_and_default_start_without_output() {
    assert!(S2PointVectorLayer::new().output().is_none());
    assert!(S2PointVectorLayer::default().output().is_none());
}

#[test]
fn with_labels_populates_label_outputs() {
    use super::super::S2Builder;

    let mut builder = S2Builder::new(super::super::Options::default());
    builder.start_layer(Box::new(
        S2PointVectorLayer::with_labels(Options::default()),
    ));
    builder.add_point(parse_point("0:1"));
    builder.add_point(parse_point("0:2"));

    let mut layers = builder.build().expect("build failed");
    let layer = layers
        .remove(0)
        .into_any()
        .downcast::<S2PointVectorLayer>()
        .expect("wrong layer type");

    assert_eq!(layer.output().expect("built points").len(), 2);
    // Label tracking is on, so the per-point label sets and the lexicon exist.
    let ids = layer.label_set_ids().expect("label set ids present");
    assert_eq!(ids.len(), 2);
    assert!(layer.label_set_lexicon().is_some());
}

#[test]
fn take_output_removes_the_built_points() {
    use super::super::S2Builder;

    let mut builder = S2Builder::new(super::super::Options::default());
    builder.start_layer(Box::new(S2PointVectorLayer::with_options(
        Options::default(),
    )));
    builder.add_point(parse_point("1:1"));
    builder.add_point(parse_point("2:2"));

    let mut layers = builder.build().expect("build failed");
    let mut layer = layers
        .remove(0)
        .into_any()
        .downcast::<S2PointVectorLayer>()
        .expect("wrong layer type");

    let taken = layer.take_output().expect("points taken");
    assert_eq!(taken.len(), 2);
    // After taking, the layer no longer holds the points.
    assert!(layer.output().is_none());
}
