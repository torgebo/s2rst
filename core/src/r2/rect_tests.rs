// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for the fixes found during the formal verification of `r2::Rect`
//! (specs/006-formal-verify-r2-rect-point). Written for this crate, not ported from upstream
//! S2.

use super::*;

/// FR-017 (K2): a negative size component gives a rectangle with one empty axis, which debug
/// builds reject, as upstream's checking constructor does (`r2rect.cc:25`).
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is_valid")]
fn from_center_size_rejects_a_negative_size_in_debug_builds() {
    let _ = Rect::from_center_size(Point::new(0.0, 0.0), Point::new(-1.0, 1.0));
}

/// FR-017: two negative components empty both axes, a valid (empty) rectangle, so the check
/// lets it through; a zero size gives a single point.
#[test]
fn from_center_size_keeps_valid_results() {
    assert!(Rect::from_center_size(Point::new(0.0, 0.0), Point::new(-1.0, -1.0)).is_empty());
    let r = Rect::from_center_size(Point::new(2.0, 3.0), Point::new(0.0, 0.0));
    assert_eq!(r, Rect::from_point(Point::new(2.0, 3.0)));
}

/// FR-019 (K3): deserialisation accepts exactly the valid rectangles. The bounds go through
/// serde's own map deserialiser, which carries every `f64` exactly, NaN included (JSON has no
/// NaN).
#[cfg(feature = "serde")]
mod deserialize {
    use super::*;
    use serde::Deserialize;
    use serde::de::IntoDeserializer;
    use std::collections::BTreeMap;

    const INF: f64 = f64::INFINITY;

    fn rect(x_lo: f64, x_hi: f64, y_lo: f64, y_hi: f64) -> Rect {
        Rect {
            x: r1::Interval { lo: x_lo, hi: x_hi },
            y: r1::Interval { lo: y_lo, hi: y_hi },
        }
    }

    fn bits(r: Rect) -> [u64; 4] {
        [r.x.lo, r.x.hi, r.y.lo, r.y.hi].map(f64::to_bits)
    }

    /// Deserialises the four bounds in `Serialize`'s format, each `f64` exactly.
    fn deserialize(r: Rect) -> Result<Rect, String> {
        let side = |s: r1::Interval| BTreeMap::from([("lo", s.lo), ("hi", s.hi)]);
        let fields = BTreeMap::from([("x", side(r.x)), ("y", side(r.y))]);
        Rect::deserialize(fields.into_deserializer())
            .map_err(|e: serde::de::value::Error| e.to_string())
    }

    /// One axis empty and the other not is rejected, and the error names the rule. Before the
    /// fix the derived impl accepted it, a rectangle that `is_valid` rejects.
    #[test]
    fn disagreeing_emptiness_is_rejected() {
        for r in [
            rect(1.0, 0.0, 0.0, 1.0),
            rect(0.0, 1.0, 1.0, 0.0),
            rect(INF, -INF, 0.0, 0.0),
        ] {
            let e = deserialize(r).expect_err("one axis empty");
            assert!(e.contains("empty"), "{e}");
        }
    }

    /// A NaN bound is rejected, though it passes `is_valid` (K4): such a rectangle contains
    /// nothing.
    #[test]
    fn a_nan_bound_is_rejected() {
        for r in [
            rect(f64::NAN, 1.0, 0.0, 1.0),
            rect(0.0, 1.0, 0.0, f64::NAN),
            rect(f64::NAN, f64::NAN, f64::NAN, f64::NAN),
        ] {
            assert!(r.is_valid(), "{r:?}");
            let e = deserialize(r).expect_err("a NaN bound");
            assert!(e.contains("NaN"), "{e}");
        }
    }

    /// Every valid rectangle deserialises unchanged, bit for bit: the empty rectangle in several
    /// representations, infinite bounds, signed zeros and single points.
    #[test]
    fn valid_rectangles_deserialise_unchanged() {
        for r in [
            Rect::empty(),
            rect(5.0, -5.0, INF, -INF),
            rect(0.0, 1.0, 2.0, 3.0),
            rect(-INF, INF, 0.0, INF),
            rect(INF, INF, -INF, -INF),
            rect(-0.0, 0.0, 0.0, -0.0),
            rect(0.5, 0.5, -2.0, -2.0),
            rect(f64::MIN_POSITIVE, f64::MAX, -f64::MAX, f64::from_bits(1)),
        ] {
            assert!(r.is_valid(), "{r:?}");
            let back = deserialize(r).unwrap_or_else(|e| panic!("{r:?}: {e}"));
            assert_eq!(bits(back), bits(r), "{r:?}");
        }
    }

    /// SC-009: 10,000 pseudo-random valid rectangles round-trip through `serde_json`: bit for
    /// bit through its `Value`, which holds each `f64` exactly, and valid through its text,
    /// whose default float parsing may move a bound by one ulp (spec 005 research R6).
    #[test]
    fn valid_rectangles_round_trip() {
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        for _ in 0..10_000 {
            let kind = next() % 3;
            // Bounds from every binade and both signs, as bit patterns, NaN redrawn.
            let mut bound = || loop {
                let v = f64::from_bits(next());
                if !v.is_nan() {
                    return v;
                }
            };
            let r = match kind {
                0 => Rect::from_point_pair(
                    Point::new(bound(), bound()),
                    Point::new(bound(), bound()),
                ),
                1 => Rect::from_point(Point::new(bound(), bound())),
                _ => Rect::empty(),
            };
            assert!(r.is_valid(), "{r:?}");
            let back: Rect = serde_json::from_value(serde_json::to_value(r).unwrap()).unwrap();
            assert_eq!(bits(back), bits(r), "{r:?}");
            if r.x.lo.is_finite() && r.x.hi.is_finite() && r.y.lo.is_finite() && r.y.hi.is_finite()
            {
                let text = serde_json::to_string(&r).unwrap();
                let back: Rect =
                    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
                assert!(back.is_valid(), "{text}");
            }
        }
    }
}
