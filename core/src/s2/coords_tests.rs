// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for the bounded coordinate types [`super::Level`] and [`super::Face`]
//! (specs/009-bounded-types). Written for this crate, not ported from upstream S2.

#[cfg(feature = "serde")]
mod serde_tests {
    use super::super::*;

    /// CO-01 (spec 009, B2): a level out of range does not deserialise.
    #[test]
    fn level_out_of_range_is_rejected() {
        for text in ["31", "255"] {
            let err = serde_json::from_str::<Level>(text).expect_err(text);
            assert!(
                err.to_string().contains("level must be in 0..=30"),
                "{text}: {err}"
            );
        }
    }

    /// CO-04 (spec 009): an in-range level keeps its serialised form, and reads back.
    #[test]
    fn level_form_is_unchanged() {
        for v in 0..=30u8 {
            let text = serde_json::to_string(&Level::new(v)).unwrap();
            assert_eq!(text, v.to_string());
            assert_eq!(serde_json::from_str::<Level>(&text).unwrap(), Level::new(v));
        }
    }
}
