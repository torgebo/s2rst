// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::PointRegion`] changes found during the formal
//! verification of `PointRegion` (specs/011-formal-verify-point-region-cell-union). Written for
//! this crate, not ported from upstream S2.

use super::*;
use crate::r3::Vector;

/// K3 (FR-019): deserialisation rejects a point that is not unit length, as upstream's
/// `S2PointRegion::Decode` (`S2::IsUnitLength`) and `Cap`'s deserialiser do, and keeps a unit
/// point bit for bit. (JSON has no NaN.)
#[cfg(feature = "serde")]
#[test]
fn deserialise_rejects_points_off_unit_length() {
    let valid = PointRegion::new(Point::from_coords(1.0, 2.0, 3.0));
    let value = serde_json::to_value(valid).unwrap();
    let back: PointRegion = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(back, valid);
    for bad in [
        Vector {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        },
        Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Vector {
            x: 1.0 + 1e-14,
            y: 0.0,
            z: 0.0,
        },
    ] {
        let mut v = value.clone();
        v["point"] = serde_json::to_value(Point(bad)).unwrap();
        let err = serde_json::from_value::<PointRegion>(v)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unit"), "{err}");
    }
    // Within upstream's tolerance (|‖p‖² − 1| ≤ 5ε), as `Point::is_unit`.
    let mut v = value.clone();
    v["point"] = serde_json::to_value(Point(Vector {
        x: 1.0 + 2.0 * f64::EPSILON,
        y: 0.0,
        z: 0.0,
    }))
    .unwrap();
    assert!(serde_json::from_value::<PointRegion>(v).is_ok());
}

fn raw(x: f64, y: f64, z: f64) -> Point {
    Point(Vector { x, y, z })
}

/// K4 (FR-019): `rect_bound` panics in debug builds for a NaN `x` or `y` (the longitude is NaN,
/// which `Rect::from_lat_lng` rejects), and returns for a NaN `z` alone.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is_valid_pair")]
fn rect_bound_panics_for_a_nan_longitude() {
    let _ = PointRegion::new(raw(1.0, f64::NAN, 0.0)).rect_bound();
}

#[test]
fn rect_bound_returns_for_a_nan_latitude() {
    let b = PointRegion::new(raw(1.0, 0.0, f64::NAN)).rect_bound();
    assert!(b.lat.lo.is_nan() && b.lng.lo == 0.0);
}

/// K4 (FR-019): `intersects_cell` panics in debug builds where `Cell::contains_point` does.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "does not belong to face")]
fn intersects_cell_panics_for_a_nan_on_the_face_axis() {
    let cell = Cell::from_cell_id(CellId::from_face(crate::s2::Face::F0));
    let _ = PointRegion::new(raw(f64::NAN, 0.0, 0.0)).intersects_cell(&cell);
}

/// K4 (FR-019): `cell_union_bound` panics in debug builds where `CellId::from_point` does:
/// every point with two or more infinite coordinates.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "does not belong to face")]
fn cell_union_bound_panics_for_two_infinities() {
    let _ = PointRegion::new(raw(0.0, f64::INFINITY, -f64::INFINITY)).cell_union_bound();
}

#[test]
fn cell_union_bound_returns_for_one_infinity() {
    assert!(
        !PointRegion::new(raw(0.0, f64::INFINITY, 1.0))
            .cell_union_bound()
            .is_empty()
    );
}
