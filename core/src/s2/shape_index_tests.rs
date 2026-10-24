// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::ShapeIndex`] (specs/002-formal-verify-cell-union,
//! defect D-002). Written for this crate, not ported from upstream S2.

use super::*;
use crate::s2::LatLng;
use crate::s2::cell_range_iterator::CellRangeIterator;
use crate::s2::lax_polygon::LaxPolygon;
use crate::s2::text_format::make_loop;

/// The index cells of `index`, in order.
fn index_cells(index: &ShapeIndex) -> Vec<CellId> {
    let mut it = CellRangeIterator::new(index);
    let mut ids = Vec::new();
    while !it.done() {
        ids.push(it.id());
        it.next();
    }
    ids
}

/// D-002: the ranges skipped while building the index (C++
/// `MutableS2ShapeIndex::SkipCellRange`) must become exactly the cells of
/// `S2CellUnion::FromBeginEnd(begin, end)`. With a wrong range covering, the index of a
/// polygon whose interior is almost the whole sphere got overlapping cells and gaps.
#[test]
fn skipped_ranges_give_disjoint_cells_without_gaps() {
    // Clockwise loops: the sphere minus a small hole. (lat, lng) of the hole's corner.
    let holes = [
        ("35:45, 35.5:45, 35.5:45.5, 35:45.5", (35.0, 45.0)),
        (
            "40:-73.9, 40.001:-73.9, 40.001:-73.899, 40:-73.899",
            (40.0, -73.9),
        ),
    ];
    for (text, (hole_lat, hole_lng)) in holes {
        let l = make_loop(text);
        let mut index = ShapeIndex::new();
        index.add(Box::new(LaxPolygon::from_loops(&[l.vertices()])));
        index.build();
        let cells = index_cells(&index);
        for w in cells.windows(2) {
            assert!(
                w[0].range_max() < w[1].range_min(),
                "{text}: index cells {:?} and {:?} overlap",
                w[0],
                w[1]
            );
        }
        // Every point well away from the hole is inside the polygon, so some index cell
        // must contain it.
        for lat in (-85..=85).step_by(5) {
            for lng in (-180..180).step_by(5) {
                let (lat, lng) = (f64::from(lat), f64::from(lng));
                if (lat - hole_lat).abs() < 3.0 && (lng - hole_lng).abs() < 3.0 {
                    continue;
                }
                let leaf = CellId::from_point(&LatLng::from_degrees(lat, lng).to_point());
                assert!(
                    cells.iter().any(|c| c.contains(leaf)),
                    "{text}: no index cell contains ({lat}, {lng})"
                );
            }
        }
    }
}
