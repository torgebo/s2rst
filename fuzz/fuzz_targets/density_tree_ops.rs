// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).
#![no_main]

use libfuzzer_sys::fuzz_target;
use s2rst::s1::Angle;
use s2rst::s2::builder::S2Error;
use s2rst::s2::density_tree::{DecodedPath, S2DensityTree};

// The `decode_density_tree` target only drives `init` + `decode`. The higher-
// level operations below walk the same untrusted tree through `DecodedPath`
// (`get_normal_cell_weight`/`get_partitioning`) and `visit_cells`
// (`normalize`/`leaves`/`dilate`) — code that re-reads the encoded bytes per
// query and is exercised by no other target. Each must return `Ok`/`Err` and
// never panic, abort, or hang on arbitrary bytes.
fuzz_target!(|data: &[u8]| {
    let mut tree = S2DensityTree::new();
    // `init` accepts empty data (an empty tree) as Ok; only a malformed header
    // is an error, in which case the ops have nothing to walk.
    if tree.init(data).is_err() {
        return;
    }

    let _ = tree.normalize();
    let _ = tree.leaves();
    let _ = tree.get_partitioning(1_i64 << 30);

    // `get_normal_cell_weight` drives `DecodedPath::load_cell` directly. Probe
    // the tree's own cells (bounded) so the queried ids map to real paths.
    if let Ok(cells) = tree.decode() {
        let mut path = DecodedPath::new(&tree);
        let mut error = S2Error::ok();
        for cid in cells.keys().take(256) {
            let _ = tree.get_normal_cell_weight(*cid, &mut path, &mut error);
            error = S2Error::ok();
        }
    }

    // `dilate` fans out to each visited cell's neighbors. Clamp the level spread
    // (0..=2) so we test the decode-path hazard, not a trivially huge fan-out
    // from an attacker-chosen depth; both params still vary with the input.
    let b0 = data.first().copied().unwrap_or(0);
    let bn = data.last().copied().unwrap_or(0);
    let max_level_diff = b0 % 3;
    let radius = Angle::from_radians(f64::from(bn).mul_add(1e-4, 1e-6));
    let _ = S2DensityTree::dilate(&tree, radius, max_level_diff);
});
