// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::S2DensityTree`]: an out-of-range
//! `max_level` is clamped to [`MAX_CELL_LEVEL`] instead of panicking in
//! `Level::from`, and the header varint reader rejects a tenth byte above 1.
//! Written for this crate, not ported from upstream S2.

use super::*;
use crate::s2::LatLng;
use crate::s2::point_vector::PointVector;

fn point_index() -> ShapeIndex {
    let mut index = ShapeIndex::new();
    index.add(Box::new(PointVector::new(vec![
        LatLng::from_degrees(0.0, 0.0).to_point(),
        LatLng::from_degrees(1.0, 1.0).to_point(),
        LatLng::from_degrees(2.0, 2.0).to_point(),
    ])));
    index.build();
    index
}

#[test]
fn vertex_density_clamps_out_of_range_max_level() {
    let index = point_index();
    // Values above MAX_CELL_LEVEL must clamp instead of panicking.
    for max_level in [31u8, 50, 200, 255] {
        let mut tree = S2DensityTree::new();
        assert!(
            tree.init_to_vertex_density(&index, 1_000_000, max_level)
                .is_ok(),
            "max_level={max_level} should clamp, not panic"
        );
    }
}

#[test]
fn out_of_range_matches_max_cell_level() {
    let index = point_index();

    let mut clamped = S2DensityTree::new();
    clamped
        .init_to_vertex_density(&index, 1_000_000, 255)
        .unwrap();

    let mut at_max = S2DensityTree::new();
    at_max
        .init_to_vertex_density(&index, 1_000_000, MAX_CELL_LEVEL)
        .unwrap();

    assert_eq!(clamped.decode().unwrap(), at_max.decode().unwrap());
}

#[test]
fn breadth_first_builder_clamps_max_level() {
    // Constructing the builder with an out-of-range level must not panic.
    let mut b = BreadthFirstTreeBuilder::new(10_000, 200);
    let mut tree = S2DensityTree::new();
    assert!(b.build(|_cid| Ok(1), &mut tree).is_ok());
}

#[test]
fn sum_density_clamps_max_level() {
    let index = point_index();
    let mut t1 = S2DensityTree::new();
    t1.init_to_vertex_density(&index, 1_000_000, 10).unwrap();

    let mut sum = S2DensityTree::new();
    assert!(sum.init_to_sum_density(&[&t1], 255).is_ok());
}

/// D-003 regression test for the density tree's own varint reader: a tenth byte above 1 is
/// rejected, as upstream does; 0 and 1 are accepted.
#[test]
fn decode_varint_at_rejects_a_tenth_byte_above_one() {
    let varint = |last: u8| {
        let mut b = vec![0xff; 9];
        b.push(last);
        b
    };
    assert_eq!(decode_varint_at(&varint(0x02), 0), None);
    assert_eq!(decode_varint_at(&varint(0x7f), 0), None);
    assert_eq!(decode_varint_at(&varint(0x01), 0), Some((u64::MAX, 10)));
    assert_eq!(
        decode_varint_at(&varint(0x00), 0),
        Some((u64::MAX >> 1, 10))
    );
}
