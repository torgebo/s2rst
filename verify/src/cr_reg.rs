// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cell`'s bounding-rectangle law, P-CREG-02 (spec 013 US2; research R0–R4): for every valid
//! cell and every `(u, v)` in its u-v bound, `rect_bound()` contains `LatLng::from_point` of the
//! direction `face_uv_to_xyz(face, u, v)`.
//!
//! **Why it holds.** Below level 0 a cell lies in one quadrant of its face (lemma L-CR-02, the
//! stub `quadrant_uv_bound`), where each coordinate of `face_uv_to_xyz` moves the latitude and the
//! longitude one way. So a point's arguments to `atan2` are dominated by the corners' that
//! `Cell::rect_bound` picks, and an `atan2` that keeps the order of its arguments' angles puts the
//! point's angles between the corners'. At level 0 the bound is fixed, and the anchors of
//! `atan2_order` (`π/4`, `3π/4`, `POLE_MIN_LAT`) bound the point's angles.
//!
//! **Under contract:** `stubs::atan2_order` (the platform's `atan2` is monotone: an assumption,
//! tested), `stubs::sqrt_order` (IEEE-754), and for the latitude below level 0
//! `stubs::latitude_lemma`, whose sum of squares is monotone (lemma L-CR-01: IEEE-754's monotone
//! rounding, which the checker times out on; tested). Not the normalised point: rounding each
//! coordinate separately breaks the dominance, and only the bound's `2ε` expansion covers it, an
//! accuracy the contracts do not state (research R5; the survey's 5,200,000 normalised points and
//! the twin find none outside).
//!
//! The law is split as R0 measured: level 0 whole (104 s), latitude below level 0 on every face
//! (261 s), per change; longitude below level 0 one face at a time (about 6 minutes each), weekly.
//! `Rect::contains_point` is `lat.contains(lat) ∧ lng.contains(lng)` of a valid `LatLng`, so the
//! parts give the law.

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s2::coords::face_uv_to_xyz;
use s2rst::s2::{Cell, LatLng, Point};

use crate::stubs::reset;
use crate::value_domain::any_cell;

/// Any `(u, v)` in the cell's u-v bound, as the direction `face_uv_to_xyz(face, u, v)`.
fn any_direction(c: Cell) -> (Point, f64, f64) {
    let uv = c.bound_uv();
    let (u, v): (f64, f64) = (kani::any(), kani::any());
    kani::assume(uv.x.lo <= u && u <= uv.x.hi && uv.y.lo <= v && v <= uv.y.hi);
    (Point(face_uv_to_xyz(c.face(), u, v)), u, v)
}

/// P-CREG-02 (law), level 0: a face cell's rectangle bound contains every point of its face.
///
/// For every face cell and `(u, v)` in [−1, 1]²: `rect_bound().contains_point(face_uv_to_xyz(face,
/// u, v))`. Under contract: `atan2_order`, `sqrt_order`, `quadrant_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::quadrant_uv_bound)]
#[kani::stub(f64::atan2, crate::stubs::atan2_order)]
#[kani::stub(f64::sqrt, crate::stubs::sqrt_order)]
fn p_creg_02__l0() {
    reset();
    let (c, _) = any_cell();
    kani::assume(c.level().as_u8() == 0);
    let (p, u, v) = any_direction(c);
    assert!(c.rect_bound().contains_point(p));
    kani::cover!(c.face().as_u8() == 2, "a polar face");
    kani::cover!(u.abs() < 1.0 && v.abs() < 1.0, "a point inside the face");
}

/// P-CREG-02 (law), the latitude below level 0: a cell's rectangle bound holds the latitude of
/// every point of its u-v bound, on every face.
///
/// For every valid cell at level 1 to 30 and `(u, v)` in its u-v bound, with `ll =
/// LatLng::from_point(face_uv_to_xyz(face, u, v))`: `ll` is valid and `rect_bound().lat` contains
/// `ll.lat`. Under contract: `atan2_order`, `sqrt_order`, `latitude_lemma`, `quadrant_uv_bound`.
#[kani::proof]
#[kani::unwind(11)]
#[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::quadrant_uv_bound)]
#[kani::stub(s2rst::s2::LatLng::latitude, crate::stubs::latitude_lemma)]
#[kani::stub(f64::atan2, crate::stubs::atan2_order)]
#[kani::stub(f64::sqrt, crate::stubs::sqrt_order)]
fn p_creg_02__lat() {
    reset();
    let (c, _) = any_cell();
    kani::assume(c.level().as_u8() >= 1);
    let (p, u, v) = any_direction(c);
    let b = c.rect_bound();
    let ll = LatLng::from_point(p);
    assert!(ll.is_valid());
    assert!(b.lat.contains(ll.lat.radians()));
    let uv = c.bound_uv();
    kani::cover!(c.level().as_u8() == 30, "a leaf cell");
    kani::cover!(
        b.lat.hi >= std::f64::consts::FRAC_PI_2,
        "a cell with a pole corner"
    );
    kani::cover!(
        uv.x.lo < u && u < uv.x.hi && uv.y.lo < v && v < uv.y.hi,
        "a point strictly inside"
    );
}

macro_rules! p_creg_02_lng {
    ($($name:ident = $face:literal),* $(,)?) => { $(
        #[doc = concat!(
            "P-CREG-02 (law), the longitude below level 0 on face ", stringify!($face), ": a ",
            "cell's rectangle bound holds the longitude of every point of its u-v bound.\n\n",
            "For every valid cell of the face at level 1 to 30 and `(u, v)` in its u-v bound: ",
            "`rect_bound().lng` contains `LatLng::from_point(face_uv_to_xyz(face, u, v)).lng`. ",
            "Under contract: `atan2_order`, `sqrt_order`, `latitude_lemma` (the bound's latitude, ",
            "which `polar_closure` reads), `quadrant_uv_bound`. Weekly: about 6 minutes (R0)."
        )]
        #[cfg(feature = "slow")]
        #[kani::proof]
        #[kani::unwind(11)]
        #[kani::stub(s2rst::s2::ij_level_to_bound_uv, crate::stubs::quadrant_uv_bound)]
        #[kani::stub(s2rst::s2::LatLng::latitude, crate::stubs::latitude_lemma)]
        #[kani::stub(f64::atan2, crate::stubs::atan2_order)]
        #[kani::stub(f64::sqrt, crate::stubs::sqrt_order)]
        fn $name() {
            reset();
            let (c, _) = any_cell();
            kani::assume(c.level().as_u8() >= 1 && c.face().as_u8() == $face);
            let (p, _, _) = any_direction(c);
            let b = c.rect_bound();
            assert!(b.lng.contains(LatLng::from_point(p).lng.radians()));
            kani::cover!(!b.lng.is_full(), "a bound short of the full circle");
        }
    )* };
}

p_creg_02_lng!(
    p_creg_02__lng_f0 = 0,
    p_creg_02__lng_f1 = 1,
    p_creg_02__lng_f2 = 2,
    p_creg_02__lng_f3 = 3,
    p_creg_02__lng_f4 = 4,
    p_creg_02__lng_f5 = 5,
);
