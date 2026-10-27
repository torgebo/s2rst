// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for [`super::RegionCoverer`]'s settings (specs/009-bounded-types). Written
//! for this crate, not ported from upstream S2.

#[cfg(feature = "serde")]
mod serde_tests {
    use super::super::*;

    /// The default coverer's serialised form, before spec 009 (CO-04).
    const DEFAULT_JSON: &str = r#"{"min_level":0,"max_level":30,"level_mod":1,"max_cells":8}"#;

    /// CO-02 (spec 009, B2): a coverer whose maximum level is out of range does not
    /// deserialise.
    #[test]
    fn max_level_out_of_range_is_rejected() {
        let text = r#"{"min_level":0,"max_level":31,"level_mod":1,"max_cells":8}"#;
        let err = serde_json::from_str::<RegionCoverer>(text).expect_err(text);
        assert!(err.to_string().contains("level must be in 0..=30"), "{err}");
    }

    /// CO-02 (spec 009, B5): a coverer whose level modulus is out of range does not
    /// deserialise. (A derived `u8` field accepted 7, and `covering` then panicked in
    /// `denormalize`.)
    #[test]
    fn level_mod_out_of_range_is_rejected() {
        for m in [0, 4, 7, 255] {
            let text = format!(r#"{{"min_level":0,"max_level":30,"level_mod":{m},"max_cells":8}}"#);
            let err = serde_json::from_str::<RegionCoverer>(&text).expect_err(&text);
            assert!(
                err.to_string().contains("level_mod must be in 1..=3"),
                "{m}: {err}"
            );
        }
    }

    /// CO-04 (spec 009): the default coverer keeps its serialised form, and reads back.
    #[test]
    fn default_form_is_unchanged() {
        let rc = RegionCoverer::default();
        assert_eq!(serde_json::to_string(&rc).unwrap(), DEFAULT_JSON);
        assert_eq!(
            serde_json::from_str::<RegionCoverer>(DEFAULT_JSON).unwrap(),
            rc
        );
    }
}

/// B12 (spec 009): a coverer whose `min_level` exceeds its `max_level` fails at once, in every
/// build, with a message naming both. Before, the check was a `debug_assert!`, `fast_covering`
/// had none, and a release build's `covering` did not finish (the Python extension, probed).
mod level_order_tests {
    use super::super::*;
    use crate::s1::Angle;
    use crate::s2::{Cap, LatLng, Level, LevelMod};

    fn reversed() -> RegionCoverer {
        RegionCoverer::new()
            .min_level(Level::new(10))
            .max_level(Level::new(5))
    }

    #[test]
    #[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
    fn covering_rejects_reversed_levels() {
        drop(reversed().covering(&Cap::full()));
    }

    #[test]
    #[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
    fn interior_covering_rejects_reversed_levels() {
        drop(reversed().interior_covering(&Cap::full()));
    }

    #[test]
    #[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
    fn fast_covering_rejects_reversed_levels() {
        drop(reversed().fast_covering(&Cap::full()));
    }

    #[test]
    #[should_panic(expected = "min_level (10) must not exceed max_level (5)")]
    fn is_canonical_rejects_reversed_levels() {
        let _ = reversed().is_canonical(&[]);
    }

    #[test]
    fn equal_levels_cover() {
        let c = RegionCoverer::new()
            .min_level(Level::new(7))
            .max_level(Level::new(7))
            .level_mod(LevelMod::new(1));
        let center = LatLng::from_degrees(10.0, 10.0).to_point();
        let cap = Cap::from_center_angle(center, Angle::from_degrees(0.1));
        assert!(!c.covering(&cap).is_empty());
    }
}
