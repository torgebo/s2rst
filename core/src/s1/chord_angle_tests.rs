// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Regression tests for `ChordAngle`'s canonical values (spec 007 FR-017 to FR-020).

use super::*;

fn bits(c: ChordAngle) -> u64 {
    c.length2().to_bits()
}

/// FR-017 (option B): every negative length, −∞ included, is `NEGATIVE`.
#[test]
fn negative_lengths_are_negative() {
    let neg_subnormal = -f64::from_bits(1);
    for x in [
        -5.0,
        -0.001,
        -1.0,
        f64::NEG_INFINITY,
        neg_subnormal,
        f64::MIN,
    ] {
        assert_eq!(
            bits(ChordAngle::from_length2(x)),
            bits(ChordAngle::NEGATIVE),
            "from_length2({x:e})"
        );
    }
}

/// FR-017: with one negative value, `predecessor` steps down and `is_infinity` is +∞ only.
#[test]
fn negative_values_behave_as_one() {
    let c = ChordAngle::from_length2(-5.0);
    assert_eq!(bits(c.predecessor()), bits(ChordAngle::NEGATIVE));
    assert_eq!(c, ChordAngle::NEGATIVE);
    assert!(!ChordAngle::from_length2(f64::NEG_INFINITY).is_infinity());
    assert!(ChordAngle::from_length2(f64::NEG_INFINITY).is_negative());
}

/// `−0` is not negative: it stays a zero, as upstream keeps it.
#[test]
fn negative_zero_is_a_zero() {
    let z = ChordAngle::from_length2(-0.0);
    assert_eq!(bits(z), (-0.0f64).to_bits());
    assert!(z.is_zero() && !z.is_negative());
    assert_eq!(bits(z.predecessor()), bits(ChordAngle::NEGATIVE));
}

/// Lengths in [0, 4] are kept bit for bit, and values above 4 clamp to `STRAIGHT`, as before.
#[test]
fn other_lengths_unchanged() {
    for x in [0.0, f64::from_bits(1), 1e-300, 1.0, 2.0, 3.999_999_999, 4.0] {
        assert_eq!(bits(ChordAngle::from_length2(x)), x.to_bits(), "{x:e}");
    }
    for x in [4.000_000_000_000_001, 5.0, f64::MAX, f64::INFINITY] {
        assert_eq!(
            bits(ChordAngle::from_length2(x)),
            bits(ChordAngle::STRAIGHT),
            "{x:e}"
        );
    }
}

/// FR-018: NaN gives `STRAIGHT`, as upstream's `std::min(4.0, NaN)`.
#[test]
fn nan_gives_straight() {
    assert_eq!(
        bits(ChordAngle::from_length2(f64::NAN)),
        bits(ChordAngle::STRAIGHT)
    );
    assert_eq!(
        bits(ChordAngle::from_length2(-f64::NAN)),
        bits(ChordAngle::STRAIGHT),
        "a negative NaN is not negative"
    );
}

/// FR-018: `plus_error` is upstream's `max(0.0, min(4.0, length2 + error))`: a NaN sum gives
/// `STRAIGHT`, the infinities clamp, and a special value is unchanged.
#[test]
fn plus_error_clamps_as_upstream() {
    assert_eq!(
        bits(ChordAngle::ZERO.plus_error(f64::NAN)),
        bits(ChordAngle::STRAIGHT)
    );
    assert_eq!(
        bits(ChordAngle::RIGHT.plus_error(f64::NAN)),
        bits(ChordAngle::STRAIGHT)
    );
    assert_eq!(
        bits(ChordAngle::RIGHT.plus_error(f64::INFINITY)),
        bits(ChordAngle::STRAIGHT)
    );
    assert_eq!(
        bits(ChordAngle::RIGHT.plus_error(f64::NEG_INFINITY)),
        0.0f64.to_bits()
    );
    assert_eq!(bits(ChordAngle::RIGHT.plus_error(0.5)), 2.5f64.to_bits());
    assert_eq!(
        bits(ChordAngle::from_length2(-0.0).plus_error(-0.0)),
        0.0f64.to_bits(),
        "std::max(0.0, −0.0) is +0"
    );
    assert_eq!(
        bits(ChordAngle::NEGATIVE.plus_error(3.0)),
        bits(ChordAngle::NEGATIVE)
    );
    assert_eq!(
        bits(ChordAngle::INFINITY.plus_error(-3.0)),
        bits(ChordAngle::INFINITY)
    );
}

/// FR-019: `cos` of a special value fails upstream's debug check.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "cos of a special chord angle")]
fn cos_of_negative_panics_in_debug_builds() {
    let _ = ChordAngle::NEGATIVE.cos();
}

/// FR-019: `sin2` (and `sin` and `tan` through it) of a special value fails upstream's debug
/// check.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "sin2 of a special chord angle")]
fn sin2_of_infinity_panics_in_debug_builds() {
    let _ = ChordAngle::INFINITY.sin2();
}

/// FR-020: deserialisation accepts the canonical values and rejects every other, naming the
/// rule. NaN goes through serde's own `f64` deserialiser, which carries it (JSON has no NaN).
#[cfg(feature = "serde")]
mod serde_canonical {
    use super::*;
    use serde::Deserialize;
    use serde::de::IntoDeserializer;

    fn deserialize(x: f64) -> Result<ChordAngle, String> {
        ChordAngle::deserialize(x.into_deserializer())
            .map_err(|e: serde::de::value::Error| e.to_string())
    }

    #[test]
    fn non_canonical_values_are_rejected() {
        for x in [
            5.0,
            -5.0,
            -0.5,
            f64::NAN,
            f64::NEG_INFINITY,
            4.000_000_000_000_001,
        ] {
            let e = deserialize(x).expect_err("not canonical");
            assert!(e.contains("canonical"), "{x}: {e}");
        }
    }

    #[test]
    fn canonical_values_round_trip_bit_for_bit() {
        for x in [-1.0, 0.0, -0.0, f64::from_bits(1), 2.0, 4.0, f64::INFINITY] {
            let c = deserialize(x).unwrap_or_else(|e| panic!("{x}: {e}"));
            assert_eq!(c.length2().to_bits(), x.to_bits(), "{x}");
        }
        for c in [
            ChordAngle::NEGATIVE,
            ChordAngle::ZERO,
            ChordAngle::RIGHT,
            ChordAngle::STRAIGHT,
        ] {
            let text = serde_json::to_string(&c).unwrap();
            let back: ChordAngle = serde_json::from_str(&text).unwrap();
            assert_eq!(back.length2().to_bits(), c.length2().to_bits(), "{text}");
        }
        assert_eq!(serde_json::to_string(&ChordAngle::RIGHT).unwrap(), "2.0");
    }
}

/// D-008 (spec 007): `max_angle_error` is upstream v0.14's `GetS1AngleConstructorMaxError`,
/// `1.5 * DBL_EPSILON * length2` (`s1chord_angle.cc:68–73`); it was `DBL_EPSILON * length2`,
/// older upstream's, a bound too small for the `sin` and the multiply it covers.
#[test]
fn max_angle_error_is_upstreams() {
    assert_eq!(ChordAngle::RIGHT.max_angle_error(), 3.0 * f64::EPSILON);
    assert_eq!(ChordAngle::STRAIGHT.max_angle_error(), 6.0 * f64::EPSILON);
    assert_eq!(ChordAngle::ZERO.max_angle_error(), 0.0);
}
