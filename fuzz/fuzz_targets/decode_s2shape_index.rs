// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s2::contains_point_query::{ContainsPointQuery, VertexModel};
use s2rst::s2::encoded_s2shape_index::EncodedS2ShapeIndex;

// `EncodedS2ShapeIndex::init` decodes a whole `MutableS2ShapeIndex` wire image:
// a version/options varint, a tag-dispatched vector of shapes (lax polyline /
// lax polygon / point vector), a cell-id vector, a cell-data string vector with
// a length cross-check, and per-cell edge lists — the richest decode surface in
// the crate.
//
// This target then *operates* on the decoded index, which `decode_shape_index_eager`
// (decode-only) does not: the decoder only structurally checks its input, so the
// shapes can carry geometry that no `is_valid` pass would accept (e.g. non-unit
// or NaN lax-shape vertices). Iterating every edge and running a point-containment
// query at each cell center drives the edge-crosser / vertex-crossing code over
// that decoded-but-unvalidated geometry. The whole pipeline must never
// panic/abort/hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let mut index = EncodedS2ShapeIndex::new();
    if index.init(data).is_err() {
        return;
    }
    let idx = index.as_index();

    // Materialize every shape's edges (bounded). Decode caps the edge counts, so
    // the `.min` is belt-and-suspenders against a pathological-but-legal count.
    for id in 0..idx.num_shape_ids() {
        if let Some(shape) = idx.shape(id as i32) {
            let n = shape.num_edges();
            for e in 0..n.min(4096) {
                let _ = shape.edge(e);
            }
        }
    }

    // Query point-containment at each cell center (bounded).
    let mut query = ContainsPointQuery::new(idx, VertexModel::SemiOpen);
    let mut it = idx.iter();
    let mut budget = 4096usize;
    while !it.done() && budget > 0 {
        let _ = query.contains(it.center());
        it.next();
        budget -= 1;
    }
});
