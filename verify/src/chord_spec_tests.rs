// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Hand-made unit tests of the chord-angle description (spec 007 T007), on the boundaries of
//! the canonical set.

use super::*;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

#[test]
fn canonical_values() {
    for l in [-1.0, -0.0, 0.0, TINY, 2.0, 4.0, f64::INFINITY] {
        assert!(canonical(l), "{l} is canonical");
    }
    for l in [
        -0.5,
        -5.0,
        f64::NEG_INFINITY,
        f64::from_bits(4.0f64.to_bits() + 1),
        5.0,
        f64::NAN,
        -TINY,
    ] {
        assert!(!canonical(l), "{l} is not canonical");
    }
}

#[test]
fn specials() {
    assert!(special(NEGATIVE) && special(INFINITY));
    assert!(!special(0.0) && !special(4.0) && !special(-0.0));
}

#[test]
fn next_at_every_boundary() {
    assert_eq!(bits(next(NEGATIVE)), bits(0.0));
    assert_eq!(bits(next(0.0)), bits(TINY));
    assert_eq!(bits(next(-0.0)), bits(TINY));
    assert_eq!(bits(next(TINY)), 2);
    assert_eq!(bits(next(2.0)), 2.0f64.to_bits() + 1);
    assert_eq!(bits(next(f64::from_bits(4.0f64.to_bits() - 1))), bits(4.0));
    assert_eq!(bits(next(4.0)), bits(f64::INFINITY));
    assert_eq!(bits(next(f64::INFINITY)), bits(f64::INFINITY));
}

#[test]
fn prev_at_every_boundary() {
    assert_eq!(bits(prev(f64::INFINITY)), bits(4.0));
    assert_eq!(bits(prev(4.0)), 4.0f64.to_bits() - 1);
    assert_eq!(bits(prev(TINY)), bits(0.0));
    assert_eq!(bits(prev(0.0)), bits(NEGATIVE));
    assert_eq!(bits(prev(-0.0)), bits(NEGATIVE));
    assert_eq!(bits(prev(NEGATIVE)), bits(NEGATIVE));
}

#[test]
fn neighbours_undo_each_other() {
    // A few hundred lengths across the binades of [0, 4].
    let mut l = TINY;
    while l < 4.0 {
        assert_eq!(bits(prev(next(l))), bits(l), "at {l:e}");
        assert_eq!(bits(next(prev(l))), bits(l), "at {l:e}");
        l *= 7.3;
    }
    assert_eq!(bits(prev(next(NEGATIVE))), bits(NEGATIVE));
    assert_eq!(bits(next(prev(f64::INFINITY))), bits(f64::INFINITY));
}

#[test]
fn neighbours_are_ordered() {
    for l in [-1.0, 0.0, TINY, 1.0, 3.999, 4.0] {
        assert!(less(l, next(l)), "{l} < next");
    }
    for l in [0.0, TINY, 1.0, 4.0, f64::INFINITY] {
        assert!(less(prev(l), l), "prev < {l}");
    }
}

#[test]
fn from_length2_restatement() {
    assert_eq!(bits(upstream_from_length2(-5.0)), bits(NEGATIVE));
    assert_eq!(
        bits(upstream_from_length2(f64::NEG_INFINITY)),
        bits(NEGATIVE)
    );
    assert_eq!(bits(upstream_from_length2(-TINY)), bits(NEGATIVE));
    assert_eq!(bits(upstream_from_length2(-0.0)), bits(-0.0));
    assert_eq!(bits(upstream_from_length2(2.5)), bits(2.5));
    assert_eq!(bits(upstream_from_length2(5.0)), bits(4.0));
    assert_eq!(bits(upstream_from_length2(f64::INFINITY)), bits(4.0));
    assert_eq!(bits(upstream_from_length2(f64::NAN)), bits(4.0));
}

#[test]
fn plus_error_restatement() {
    assert_eq!(bits(upstream_plus_error(2.0, f64::NAN)), bits(4.0));
    assert_eq!(bits(upstream_plus_error(2.0, 5.0)), bits(4.0));
    assert_eq!(bits(upstream_plus_error(2.0, -5.0)), bits(0.0));
    assert_eq!(
        bits(upstream_plus_error(-0.0, -0.0)),
        bits(0.0),
        "std::max(0.0, −0.0) is +0"
    );
    assert_eq!(bits(upstream_plus_error(1.0, 0.5)), bits(1.5));
    assert_eq!(bits(upstream_plus_error(NEGATIVE, 3.0)), bits(NEGATIVE));
    assert_eq!(bits(upstream_plus_error(INFINITY, -3.0)), bits(INFINITY));
}

#[test]
fn formulas() {
    assert_eq!(upstream_sin2(2.0), 1.0);
    assert_eq!(upstream_sin2(4.0), 0.0);
    assert_eq!(upstream_cos(0.0), 1.0);
    assert_eq!(upstream_cos(4.0), -1.0);
    assert_eq!(upstream_max_angle_error(2.0), 3.0 * f64::EPSILON);
    assert!(upstream_max_point_error(0.0) > 0.0);
}
