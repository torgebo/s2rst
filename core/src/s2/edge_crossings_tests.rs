// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for the fixes found during the formal verification of
//! `s2::Point` (specs/006-formal-verify-r2-rect-point), here the robust cross product's
//! scaling. Written for this crate, not ported from upstream S2.

use super::*;
use crate::r3::Vector;

fn bits(p: Point) -> [u64; 3] {
    [p.0.x, p.0.y, p.0.z].map(f64::to_bits)
}

/// The unit point `(2^-k, 0, 1)`, with `2^-k` built from its bits.
fn tilted_pole(k: i32) -> Point {
    let x = if k > 1022 {
        f64::from_bits(1u64 << (1074 - k))
    } else {
        f64::from_bits(u64::try_from(1023 - k).unwrap() << 52)
    };
    Point(Vector { x, y: 0.0, z: 1.0 })
}

/// FR-021 (K6): the antipodal pair falls back to the symbolic perturbation, whose result
/// `(-0, 2^-300, 0)` is scaled into [1, 2) as upstream's `EnsureNormalizable` does. The old
/// private copy scaled into [0.5, 1) and returned `(-0, 0.5, 0)`.
#[test]
fn robust_cross_prod_scales_as_upstream() {
    let b = tilted_pole(300);
    assert!(b.is_unit());
    let r = robust_cross_prod(-b, b);
    assert_eq!(
        bits(r),
        bits(Point(Vector {
            x: -0.0,
            y: 1.0,
            z: 0.0
        })),
        "{r:?}"
    );
}

/// FR-021 (K6, U-012): for `b = (2^-1074, 0, 1)`, a unit point, upstream's
/// `EnsureNormalizable` overflows and C++ returns `(NaN, ∞, NaN)`. The library's result is the
/// correctly scaled vector: finite, non-zero and normalizable.
#[test]
fn robust_cross_prod_is_finite_for_the_smallest_tilt() {
    let b = tilted_pole(1074);
    assert!(b.is_unit());
    let r = robust_cross_prod(-b, b);
    assert!(
        r.x().is_finite() && r.y().is_finite() && r.z().is_finite(),
        "{r:?}"
    );
    assert!(r.is_normalizable(), "{r:?}");
    assert_eq!(
        bits(r),
        bits(Point(Vector {
            x: -0.0,
            y: 1.0,
            z: 0.0
        })),
        "{r:?}"
    );
}
