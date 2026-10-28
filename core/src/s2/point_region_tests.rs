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
