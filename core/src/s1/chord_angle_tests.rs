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

/// FR-020: deserialisation returns canonical values: the canonical ones as read, any other
/// negative as `NEGATIVE` (as `from_length2` and the binary decoders), and an error naming the
/// rule for the rest. NaN goes through serde's own `f64` deserialiser, which carries it (JSON
/// has no NaN).
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
    fn lengths_above_four_and_nan_are_rejected() {
        for x in [5.0, f64::NAN, 4.000_000_000_000_001] {
            let e = deserialize(x).expect_err("not a length");
            assert!(e.contains("not a chord angle length"), "{x}: {e}");
        }
    }

    /// Code review of v0.4.0..v0.5.0, finding 3: v0.4.0 kept negative lengths, so its JSON for
    /// an empty cap can hold one; the binary decoder already maps them to `NEGATIVE`.
    #[test]
    fn negative_lengths_read_as_negative() {
        for x in [-5.0, -0.5, -1.0 / std::f64::consts::PI, f64::NEG_INFINITY] {
            let c = deserialize(x).unwrap_or_else(|e| panic!("{x}: {e}"));
            assert_eq!(c.length2().to_bits(), (-1.0f64).to_bits(), "{x}");
        }
        // `Cap::from_center_area(c, -1.0)` as v0.4.0 wrote it: today's empty cap with v0.4.0's
        // radius, −1/π.
        let today = serde_json::to_string(&crate::s2::Cap::empty()).unwrap();
        let json = today.replace("-1.0", "-0.3183098861837907");
        assert_ne!(json, today);
        let cap: crate::s2::Cap = serde_json::from_str(&json).unwrap();
        assert!(cap.is_empty());
        assert_eq!(cap.chord_radius(), ChordAngle::NEGATIVE);
    }

    /// Code review of v0.4.0..v0.5.0, finding 4: `serde_json` writes +∞ as `null`.
    #[test]
    fn infinity_round_trips_through_json() {
        let text = serde_json::to_string(&ChordAngle::INFINITY).unwrap();
        assert_eq!(text, "null");
        let back: ChordAngle = serde_json::from_str(&text).unwrap();
        assert!(back.is_infinity());
    }

    /// A format that keeps the newtype wrapper, as RON writes `ChordAngle::RIGHT` as `(2.0)`:
    /// the number reaches only `visit_newtype_struct`, and there is no bare-number form.
    struct Wrapped<D>(D);

    impl<'de, D> serde::Deserializer<'de> for Wrapped<D>
    where
        D: serde::Deserializer<'de, Error = serde::de::value::Error>,
    {
        type Error = serde::de::value::Error;

        fn deserialize_any<V: serde::de::Visitor<'de>>(
            self,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::NewtypeStruct,
                &visitor,
            ))
        }

        fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
            self,
            name: &'static str,
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            assert_eq!(name, "ChordAngle");
            visitor.visit_newtype_struct(self.0)
        }

        serde::forward_to_deserialize_any! {
            bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
            option unit unit_struct seq tuple tuple_struct map struct enum identifier ignored_any
        }
    }

    /// Code review of v0.4.0..74b9bc1, finding 2: the derived `Serialize` writes a newtype struct,
    /// and a format that keeps the wrapper reads it back. Reading a bare number failed there.
    #[test]
    fn the_newtype_wrapper_is_read() {
        let read = |x: f64| ChordAngle::deserialize(Wrapped(x.into_deserializer()));
        assert_eq!(read(2.0).unwrap(), ChordAngle::RIGHT);
        assert_eq!(read(-5.0).unwrap(), ChordAngle::NEGATIVE);
        assert!(read(f64::INFINITY).unwrap().is_infinity());
        let e = read(5.0).unwrap_err().to_string();
        assert!(e.contains("not a chord angle length"), "{e}");
        let unit: serde::de::value::UnitDeserializer<serde::de::value::Error> =
            ().into_deserializer();
        assert!(
            ChordAngle::deserialize(Wrapped(unit))
                .unwrap()
                .is_infinity()
        );
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
            ChordAngle::INFINITY,
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
