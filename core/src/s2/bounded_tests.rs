// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original tests for the bounded-conversion error (specs/009-bounded-types). Written for this
//! crate, not ported from upstream S2.

use super::*;

const ALL_KINDS: [BoundedKind; 5] = [
    BoundedKind::Level,
    BoundedKind::Face,
    BoundedKind::LevelMod,
    BoundedKind::CellVertex,
    BoundedKind::CellEdge,
];

#[test]
fn kind_names_and_ranges() {
    let expected: [(&str, RangeInclusive<i64>); 5] = [
        ("level", 0..=30),
        ("face", 0..=5),
        ("level_mod", 1..=3),
        ("cell vertex index", 0..=3),
        ("cell edge index", 0..=3),
    ];
    for (kind, (name, range)) in ALL_KINDS.into_iter().zip(expected) {
        assert_eq!(kind.name(), name, "{kind:?}");
        assert_eq!(kind.range(), range, "{kind:?}");
    }
}

#[test]
fn out_of_range_display() {
    let cases = [
        (BoundedKind::Level, 31, "level must be in 0..=30, got 31"),
        (BoundedKind::Face, 7, "face must be in 0..=5, got 7"),
        (
            BoundedKind::LevelMod,
            0,
            "level_mod must be in 1..=3, got 0",
        ),
        (
            BoundedKind::CellVertex,
            4,
            "cell vertex index must be in 0..=3, got 4",
        ),
        (
            BoundedKind::CellEdge,
            -1,
            "cell edge index must be in 0..=3, got -1",
        ),
    ];
    for (kind, value, text) in cases {
        assert_eq!(OutOfRange::new(kind, value).to_string(), text);
    }
}

#[test]
fn out_of_range_accessors() {
    for kind in ALL_KINDS {
        for value in [i64::MIN, -1, 99, i64::MAX] {
            let e = OutOfRange::new(kind, value);
            assert_eq!(e.kind(), kind);
            assert_eq!(e.value(), value);
            assert_eq!(e.range(), kind.range());
        }
    }
}

#[test]
fn out_of_range_is_an_error() {
    fn takes_error(_: &dyn std::error::Error) {}
    takes_error(&OutOfRange::new(BoundedKind::Level, 31));
}

#[test]
fn level_mod_accepts_exactly_one_to_three() {
    for v in 0..=u8::MAX {
        let r = LevelMod::try_from(v);
        assert_eq!(r.is_ok(), (1..=3).contains(&v), "{v}");
        match r {
            Ok(m) => assert_eq!(m.get(), v),
            Err(e) => {
                assert_eq!(e.kind(), BoundedKind::LevelMod);
                assert_eq!(e.value(), i64::from(v));
            }
        }
    }
    for v in [i64::MIN, -1, 0, 4, i64::MAX] {
        assert!(LevelMod::try_from(v).is_err(), "{v}");
    }
    for v in [usize::MAX, 0, 4] {
        assert!(LevelMod::try_from(v).is_err(), "{v}");
    }
    assert_eq!(LevelMod::try_from(2usize).map(LevelMod::get), Ok(2));
}

#[test]
fn level_mod_constants_and_measures() {
    assert_eq!(LevelMod::DEFAULT, LevelMod::new(1));
    assert_eq!(LevelMod::default(), LevelMod::DEFAULT);
    let factors: Vec<u32> = (1..=3)
        .map(|v| LevelMod::new(v).branching_factor())
        .collect();
    assert_eq!(factors, [4, 16, 64]);
    assert_eq!(LevelMod::new(2).to_string(), "2");
    assert!(LevelMod::new(1) < LevelMod::new(3));
}

#[test]
#[should_panic(expected = "level_mod must be in 1..=3")]
fn level_mod_new_panics_outside_the_range() {
    let _ = LevelMod::new(4);
}

#[cfg(feature = "serde")]
#[test]
fn level_mod_serde() {
    for v in 1..=3u8 {
        let text = serde_json::to_string(&LevelMod::new(v)).unwrap();
        assert_eq!(text, v.to_string());
        assert_eq!(
            serde_json::from_str::<LevelMod>(&text).unwrap(),
            LevelMod::new(v)
        );
    }
    for text in ["0", "4", "255"] {
        let err = serde_json::from_str::<LevelMod>(text).expect_err(text);
        assert!(
            err.to_string().contains("level_mod must be in 1..=3"),
            "{err}"
        );
    }
}
