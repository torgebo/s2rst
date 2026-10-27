// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Regression tests for `Cap`'s parity with upstream (spec 008 FR-017 to FR-020, FR-022,
//! FR-023). Every expected value is upstream C++'s, observed by the survey probe
//! (`specs/008-formal-verify-cap/probe/cpp/survey_cpp.txt`, `v0.14.0-40`).

use super::*;
use crate::s2::metric::MIN_WIDTH;
use crate::s2::region::Region;
use crate::s2::{Cell, CellId};

fn ll(lat: f64, lng: f64) -> Point {
    LatLng::from_degrees(lat, lng).to_point()
}

fn level_of(ids: &[CellId]) -> i32 {
    if ids.len() == 6 {
        -1
    } else {
        i32::from(ids[0].level().as_u8())
    }
}

/// K1 (FR-017): the covering's level is upstream's `kMinWidth.GetLevelForMinValue(r) − 1`.
/// Before the fix, `floor(log2(d / r))` gave level 2 for this radius.
#[test]
fn covering_level_matches_upstream() {
    let cap = Cap::from_center_angle(
        ll(10.0, 20.0),
        Angle::from_radians(0.117_851_130_197_757_95),
    );
    let ids = cap.cell_union_bound();
    assert_eq!(ids.len(), 4);
    assert_eq!(level_of(&ids), 1, "upstream's level");
}

/// K1 over the probe's radii: 64 radii from each of the 29 powers of two `d / 2ᵏ` upwards
/// (180 of the 1,856 differed before the fix).
#[test]
fn covering_level_matches_upstream_near_every_power_of_two() {
    let d = 2.0 * std::f64::consts::SQRT_2 / 3.0;
    let center = ll(10.0, 20.0);
    for k in 1..=29 {
        let mut r = d / f64::from(1u32 << k);
        for _ in 0..64 {
            let cap = Cap::from_center_angle(center, Angle::from_radians(r));
            let expected = i32::from(MIN_WIDTH.max_level(cap.angle_radius().radians()).as_u8()) - 1;
            assert_eq!(level_of(&cap.cell_union_bound()), expected, "radius {r:e}");
            r = r.next_up();
        }
    }
}

/// Upstream's `Contains(S2Cell)` and `MayIntersect` (s2cap.cc:225–290), transcribed for the
/// grid below; the survey's transcription agreed with C++ on every case.
fn grid() -> impl Iterator<Item = (Cap, Cell)> {
    let face0 = Cell::from_cell_id(CellId::from_face(0));
    (0..=12).flat_map(move |dlat| {
        [56.0, 58.0, 60.0, 62.0, 64.0].into_iter().map(move |rad| {
            (
                Cap::from_center_angle(ll(f64::from(dlat), 0.0), Angle::from_degrees(rad)),
                face0,
            )
        })
    })
}

/// K2 (FR-018): `Region::contains_cell` is upstream's `Contains(S2Cell)`: on the survey's grid of
/// 65 caps around face cell 0, C++ says 38 contain it (the library said 30 before the fix).
#[test]
fn contains_cell_matches_upstream() {
    let n = grid().filter(|(cap, cell)| cap.contains_cell(cell)).count();
    assert_eq!(n, 38);
    let (cap, cell) = (
        Cap::from_center_angle(ll(4.0, 0.0), Angle::from_degrees(58.0)),
        Cell::from_cell_id(CellId::from_face(0)),
    );
    assert!(
        cap.contains_cell(&cell),
        "4° off the face centre, radius 58°"
    );
}

/// K3 (FR-018): `Region::intersects_cell` is upstream's `MayIntersect`: a 2° cap at (0°, 50°), on
/// face 1, does not intersect face cell 0 (the library said it did before the fix).
#[test]
fn intersects_cell_matches_upstream() {
    let cap = Cap::from_center_angle(ll(0.0, 50.0), Angle::from_degrees(2.0));
    let face0 = Cell::from_cell_id(CellId::from_face(0));
    assert!(!cap.intersects_cell(&face0));
    assert!(cap.intersects_cell(&Cell::from_cell_id(CellId::from_face(1))));
}

/// FR-018: cells inside, outside and straddling a cap, at levels 0, 5 and 20.
#[test]
fn cell_tests_inside_outside_straddling() {
    let center = ll(10.0, 20.0);
    let cap = Cap::from_center_angle(center, Angle::from_degrees(1.0));
    let id = CellId::from_point(&center);
    // Level 20 around the centre: well inside (≈ 10 m against ≈ 110 km).
    let inside = Cell::from_cell_id(id.parent_at_level(20u8));
    assert!(cap.contains_cell(&inside) && cap.intersects_cell(&inside));
    // Level 5 around the centre: much larger than the cap, so it straddles the boundary.
    let straddling = Cell::from_cell_id(id.parent_at_level(5u8));
    assert!(!cap.contains_cell(&straddling) && cap.intersects_cell(&straddling));
    // Level 0 on the opposite face: outside.
    let far = Cell::from_cell_id(CellId::from_point(&-center).parent_at_level(0u8));
    assert!(!cap.contains_cell(&far) && !cap.intersects_cell(&far));
    // The full cap contains every cell; the empty cap intersects none.
    assert!(Cap::full().contains_cell(&far));
    assert!(!Cap::empty().intersects_cell(&inside));
}

/// K4 (FR-019): the complement's radius is upstream's `FromLength2(4 − r²)`: length² 1 gives
/// exactly 3 (the library gave 2.99999999999999956 before the fix).
#[test]
fn complement_radius_is_upstream() {
    for (l, expected) in [
        (0.1_f64, 3.9_f64),
        (0.5, 3.5),
        (1.0, 3.0),
        (1.7, 2.3),
        (2.9, 1.1),
        (3.3, 0.700_000_000_000_000_2),
    ] {
        let cap = Cap::from_center_chord_angle(ll(10.0, 20.0), ChordAngle::from_length2(l));
        let k = cap.complement();
        assert_eq!(
            k.chord_radius().length2().to_bits(),
            expected.to_bits(),
            "length² {l}"
        );
        assert_eq!(k.center(), -cap.center());
    }
    assert!(Cap::empty().complement().is_full());
    assert!(Cap::full().complement().is_empty());
}

/// K5 (FR-020): `rect_bound`'s longitudes are reduced exactly, as upstream's `remainder`: the
/// rectangle of a cap at longitude 10⁻¹⁴° with radius 10⁻¹⁸ rad contains its centre (it was
/// `[0, 0]` before the fix).
#[test]
fn rect_bound_contains_centre_at_tiny_longitude() {
    let cap = Cap::from_center_angle(ll(0.0, 1e-14), Angle::from_radians(1e-18));
    let r = cap.rect_bound();
    assert!(r.contains_point(cap.center()), "{r:?}");
    // C++'s bounds (survey_cpp.txt).
    assert_eq!(r.lng.lo, 1.735_329_251_994_329_6e-16);
    assert_eq!(r.lng.hi, 1.755_329_251_994_329_4e-16);
}

/// FR-020: `reduce_longitude` is upstream's `remainder(x, 2π)`, exactly, on its domain
/// |x| ≤ 3π/2: 10⁶ inputs uniform over it, and the edges ±π, ±3π/2 and their neighbours.
#[test]
fn reduce_longitude_is_the_exact_remainder() {
    use crate::test_util::ieee_remainder_2pi;
    let check = |x: f64| {
        assert_eq!(
            reduce_longitude(x).to_bits(),
            ieee_remainder_2pi(x).to_bits(),
            "x = {x:e}"
        );
    };
    let lim = 1.5 * PI;
    for x in [
        PI,
        -PI,
        lim,
        -lim,
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
    ] {
        for y in [x.next_down(), x, x.next_up()] {
            if y.abs() <= lim {
                check(y);
            }
        }
    }
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    for _ in 0..1_000_000 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let u = (s >> 11) as f64 / (1u64 << 53) as f64;
        check(-lim + 2.0 * lim * u);
    }
}

/// K6 (FR-022): `approx_eq` compares the centres within upstream's default 10⁻¹⁴, as
/// `ApproxEquals`: centres 5.02·10⁻¹⁵ apart are approximately equal (they were not before the
/// fix, which compared centres at a fixed 10⁻¹⁵).
#[test]
fn approx_eq_uses_tolerance_for_centres() {
    let a = ll(10.0, 20.0);
    let la = LatLng::from(a);
    let b = LatLng::from_radians(la.lat.radians() + 5e-15, la.lng.radians()).to_point();
    let (ca, cb) = (
        Cap::from_center_chord_angle(a, ChordAngle::from_length2(1.0)),
        Cap::from_center_chord_angle(b, ChordAngle::from_length2(1.0)),
    );
    assert!(ca.approx_eq(cb));
    assert!(ca.approx_eq_with(cb, 1e-14));
    let c = LatLng::from_radians(la.lat.radians() + 2e-14, la.lng.radians()).to_point();
    assert!(!ca.approx_eq(Cap::from_center_chord_angle(
        c,
        ChordAngle::from_length2(1.0)
    )));
}

/// K7 (FR-023): deserialisation rejects a cap that is not valid, as the binary decoder does, and
/// keeps a valid one unchanged. (JSON has no NaN; the binary decoder's tests cover a NaN centre.)
#[cfg(feature = "serde")]
#[test]
fn deserialise_rejects_invalid_caps() {
    let valid = Cap::from_center_chord_angle(ll(10.0, 20.0), ChordAngle::from_length2(1.0));
    let value = serde_json::to_value(valid).unwrap();
    let back: Cap = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(back.center(), valid.center());
    assert_eq!(
        back.chord_radius().length2().to_bits(),
        valid.chord_radius().length2().to_bits()
    );
    for bad in [
        Point(Vector {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        }),
        Point(Vector {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }),
    ] {
        let mut v = value.clone();
        v["center"] = serde_json::to_value(bad).unwrap();
        let err = serde_json::from_value::<Cap>(v).unwrap_err().to_string();
        assert!(err.contains("unit"), "{err}");
    }
    for c in [Cap::empty(), Cap::full()] {
        let b: Cap = serde_json::from_value(serde_json::to_value(c).unwrap()).unwrap();
        assert!(b == c);
    }
    // Code review of v0.4.0..74b9bc1, finding 2: a `null` radius reads as `INFINITY`, above
    // `STRAIGHT`. Versions before 0.5 wrote a radius of −∞ or NaN as `null` too.
    let mut v = value.clone();
    v["radius"] = serde_json::Value::Null;
    let err = serde_json::from_value::<Cap>(v).unwrap_err().to_string();
    assert!(err.contains("above STRAIGHT"), "{err}");
}

/// D-009: the area of a cap whose radius has length² −0 is +0, as upstream's
/// `2π · std::max(0.0, height)`. Before the fix, `0.0_f64.max(−0.0)` left the sign to the build:
/// −0 unoptimised, +0 optimised (found by Kani, `cp_mea::p_cpmea_01__area`).
#[test]
fn area_of_a_negative_zero_radius_is_positive_zero() {
    let cap = Cap::from_center_chord_angle(ll(10.0, 20.0), ChordAngle::from_length2(-0.0));
    assert_eq!(cap.area().to_bits(), 0, "area {:e}", cap.area());
    assert_eq!(
        cap.centroid().0.x.to_bits() >> 63,
        0,
        "the centroid inherits the sign"
    );
}

/// K8: the empty cap's covering is upstream's. `GetCellUnionBound` does not treat the empty cap
/// apart: its radius, −1 radian, gives level 29 and the four cells around the centre. The centre
/// and the cells are `CAP_BOUNDS`'s empty row (`core/tests/data/cpp_cap_vectors.csv`). Before the
/// change, the library returned no cells.
#[test]
fn empty_cap_covering_matches_upstream() {
    let center = Point(Vector {
        x: f64::from_bits(0x3fed_9d03_3a6c_b461),
        y: f64::from_bits(0x3fd5_8e86_9f82_dce2),
        z: f64::from_bits(0x3fc6_3a1a_7e0b_7389),
    });
    let cap = Cap::from_center_chord_angle(center, ChordAngle::NEGATIVE);
    assert!(cap.is_empty());
    let mut got: Vec<u64> = cap.cell_union_bound().iter().map(|id| id.id()).collect();
    got.sort_unstable();
    let mut want = vec![
        0x1128_2087_039e_7fc4_u64,
        0x1128_2087_039e_7fcc,
        0x1128_2087_039e_7fbc,
        0x1128_2087_039e_7fb4,
    ];
    want.sort_unstable();
    assert_eq!(got, want);
    assert_eq!(Region::cell_union_bound(&cap).len(), 4);
}

/// Code review of v0.4.0..74b9bc1, finding 3: an empty cap built from a negative height or area
/// has the radius `NEGATIVE`, so its height is −0.5, not the value given (upstream's, and v0.4.0's,
/// `height()` returns −3 for the first). Its area is 0 either way.
#[test]
fn empty_cap_from_a_negative_height_or_area_has_height_minus_one_half() {
    for cap in [
        Cap::from_center_height(ll(10.0, 20.0), -3.0),
        Cap::from_center_area(ll(10.0, 20.0), -1.0),
    ] {
        assert!(cap.is_empty());
        assert_eq!(cap.chord_radius(), ChordAngle::NEGATIVE);
        assert_eq!(cap.height(), -0.5);
        assert_eq!(cap.area(), 0.0);
    }
}
