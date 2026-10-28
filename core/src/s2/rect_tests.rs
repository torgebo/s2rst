// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>

//! Original regression tests for [`super::Rect`] parity fixes, and for the fixes found during
//! the formal verification of `s2::Rect` (specs/005-formal-verify-r1-interval-rect). Written
//! for this crate, not ported from upstream S2.

use super::*;

fn rect_deg(lat_lo: f64, lng_lo: f64, lat_hi: f64, lng_hi: f64) -> Rect {
    Rect::new(
        r1::Interval::new(lat_lo.to_radians(), lat_hi.to_radians()),
        s1::Interval::new(lng_lo.to_radians(), lng_hi.to_radians()),
    )
}

/// `cap_bound` bounds a rectangle that crosses the antimeridian and spans more than 180° of
/// longitude.
///
/// C++ `S2LatLngRect::GetCapBound` (`s2latlng_rect.cc:323`) tries the cap around the centre
/// only when `lng_.GetLength() <= M_PI`, and otherwise returns the cap around a pole. The
/// port tested `(hi − lo).rem_euclid(2π) >= 0 && hi − lo <= π`, from Go's
/// `math.Remainder(...) >= 0 && ...`. `rem_euclid` is never negative, so every inverted
/// longitude interval took the centre cap, which misses part of a wide rectangle. The
/// generic `Region` property `region_union_cap_contains` (`core/tests/trait_property_tests.rs`)
/// found it.
#[test]
fn cap_bound_contains_wide_rect_across_antimeridian() {
    // The union of two caps' bounds that `region_union_cap_contains` failed on: longitude
    // from 70.9° east across 180° to 40.6° west, 248°. Its centre cap is a hair smaller than
    // the pole cap, so the old test picked it, and it misses the rectangle's western side.
    let rect = Rect::new(
        r1::Interval::new(-0.853_640_537_150_426_5, 0.574_387_856_831_333_9),
        s1::Interval::new(1.238_198_774_097_490_7, -0.708_608_117_065_030_5),
    );
    assert!(rect.lng.is_inverted());
    let p = LatLng::from_radians(-0.069_813_170_079_773_18, 1.239_183_768_915_974);
    assert!(rect.contains_lat_lng(p));
    let cap = rect.cap_bound();
    assert!(cap.contains_point(p.to_point()), "{cap:?} misses {p:?}");
    // It is the south-pole cap, as upstream returns: lat_lo + lat_hi < 0.
    assert_eq!(cap.center().0.z, -1.0);
}

/// The fix keeps the centre cap for a narrow rectangle across the antimeridian (20°), where
/// it is smaller than the pole cap, as upstream does.
#[test]
fn cap_bound_keeps_centre_cap_for_narrow_rect_across_antimeridian() {
    let rect = rect_deg(10.0, 170.0, 20.0, -170.0);
    assert!(rect.lng.is_inverted());
    let cap = rect.cap_bound();
    assert!(
        cap.center().0.z.abs() < 1.0,
        "expected the centre cap, got {cap:?}"
    );
    for v in RectVertex::iter() {
        assert!(cap.contains_point(rect.vertex(v).to_point()));
    }
}

/// Shrinking a rectangle's longitudes to almost nothing gives the empty rectangle, not one
/// with almost every longitude (spec 004 FR-023, `UPSTREAM.md` U-011).
///
/// The longitudes are the arc on which upstream's `S1Interval::Expanded` crosses its bounds,
/// found by Kani for spec 004's P-IEXP-01: shrunk by 2.0000610351562496 they become
/// [1.5331420898437502, 1.53314208984375], whose length is 2π. `expanded` inherits the
/// circular interval's fix, as does `expanded_by_distance`, which shrinks the longitudes the
/// same way.
#[test]
fn expanded_shrinking_longitudes_to_almost_nothing_is_empty() {
    let rect = Rect::new(
        r1::Interval::new(0.0, 0.1),
        s1::Interval::new(
            f64::from_bits(0xBFDD_E1FF_FFFF_FFF5),
            f64::from_bits(0xC005_FFF6_A888_5A30),
        ),
    );
    let margin = LatLng::from_radians(0.0, f64::from_bits(0xC000_001F_FFFF_FFFF));
    let shrunk = rect.expanded(margin);
    assert!(shrunk.is_empty(), "{shrunk:?}");
    assert!(rect.contains(shrunk));
}

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn rect(lat_lo: f64, lat_hi: f64, lng_lo: f64, lng_hi: f64) -> Rect {
        Rect {
            lat: r1::Interval {
                lo: lat_lo,
                hi: lat_hi,
            },
            lng: s1::Interval {
                lo: lng_lo,
                hi: lng_hi,
            },
        }
    }

    /// The four bounds as bit patterns.
    fn bits(r: Rect) -> [u64; 4] {
        [r.lat.lo, r.lat.hi, r.lng.lo, r.lng.hi].map(f64::to_bits)
    }

    /// Deserialises a rectangle written as its four bounds. `Value` carries each `f64`
    /// exactly, with no text parsing in between.
    fn deserialize(lat_lo: f64, lat_hi: f64, lng_lo: f64, lng_hi: f64) -> Result<Rect, String> {
        let v = serde_json::json!({
            "lat": { "lo": lat_lo, "hi": lat_hi },
            "lng": { "lo": lng_lo, "hi": lng_hi },
        });
        serde_json::from_value::<Rect>(v).map_err(|e| e.to_string())
    }

    /// The adjacent float toward zero, for a non-zero `x`.
    fn toward_zero(x: f64) -> f64 {
        f64::from_bits(x.to_bits() - 1)
    }

    /// FR-020 (K4): a rectangle with a latitude beyond the poles does not deserialise, and the
    /// error names the rule. Before the fix the derived impl accepted it, a way to build a
    /// rectangle that `is_valid` rejects.
    #[test]
    fn a_latitude_beyond_a_pole_is_rejected() {
        for (lo, hi) in [(0.0, FRAC_PI_2 + 1e-15), (-1.6, 0.0), (2.0, 2.0)] {
            let e = deserialize(lo, hi, 0.0, 1.0).expect_err("latitude out of range");
            assert!(e.contains("latitude"), "{e}");
        }
    }

    /// FR-020: an invalid longitude arc does not deserialise: −π may only start the full
    /// interval or end the empty one, and every bound must lie in [−π, π].
    #[test]
    fn an_invalid_longitude_arc_is_rejected() {
        for (lo, hi) in [(-PI, 0.0), (0.0, -PI), (4.0, 0.0), (0.0, -3.5)] {
            let e = deserialize(0.0, 0.5, lo, hi).expect_err("invalid arc");
            assert!(e.contains("longitude"), "{e}");
        }
    }

    /// FR-020: the latitudes and longitudes must agree on emptiness.
    #[test]
    fn disagreeing_emptiness_is_rejected() {
        let e = deserialize(1.0, 0.0, -PI, PI).expect_err("empty latitudes, full longitudes");
        assert!(e.contains("empty"), "{e}");
        let e = deserialize(-0.5, 0.5, PI, -PI).expect_err("latitudes, empty longitudes");
        assert!(e.contains("empty"), "{e}");
    }

    /// FR-020: every valid rectangle deserialises unchanged, bit for bit, the boundary values
    /// included: ±π/2, ±π, the floats just inside them, ±0, the empty and full rectangles, a
    /// rectangle across the antimeridian and spec 004's seam corner as a longitude range.
    #[test]
    fn valid_rectangles_deserialise_unchanged() {
        let h = FRAC_PI_2;
        let corner_hi = f64::from_bits((-PI).to_bits() - 1);
        let cases = [
            Rect::empty(),
            Rect::full(),
            rect(-h, h, -PI, PI),
            rect(toward_zero(h), h, PI, PI),
            rect(-h, toward_zero(-h), toward_zero(PI), PI),
            rect(-0.5, 0.5, toward_zero(-PI), PI),
            rect(-0.0, 0.0, -0.0, 0.0),
            rect(0.0, -0.0, 0.0, -0.0),
            rect(-0.5, 0.5, 3.0, -3.0),
            rect(0.0, 0.1, PI, corner_hi),
            rect(h, h, -PI, PI),
        ];
        for r in cases {
            assert!(r.is_valid(), "{r:?}");
            let v = serde_json::to_value(r).unwrap();
            let back: Rect = serde_json::from_value(v).unwrap();
            assert_eq!(bits(back), bits(r), "{r:?}");
        }
    }

    /// SC-009: pseudo-random valid rectangles through JSON text and back each deserialise and
    /// stay valid. `serde_json`'s default float parsing may move a bound by one ulp (391,960
    /// of 1,001,444 rectangles in spec 005 research R6), but never out of validity, so the
    /// check needs no tolerance.
    #[test]
    fn valid_rectangles_round_trip_through_text() {
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut moved = 0;
        for _ in 0..10_000 {
            let mut ll = || {
                LatLng::from_radians((next() * 2.0 - 1.0) * FRAC_PI_2, (next() * 2.0 - 1.0) * PI)
            };
            let r = Rect::from_point_pair(ll(), ll());
            assert!(r.is_valid(), "{r:?}");
            let text = serde_json::to_string(&r).unwrap();
            let back: Rect = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert!(back.is_valid(), "{text}");
            moved += usize::from(bits(back) != bits(r));
        }
        assert!(
            moved > 0,
            "no bound moved: the test no longer exercises lossy parsing"
        );
    }

    /// The format is unchanged: the fields and their names are the derived ones.
    #[test]
    fn the_format_is_unchanged() {
        let v = serde_json::to_value(Rect::full()).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "lat": { "lo": -FRAC_PI_2, "hi": FRAC_PI_2 },
                "lng": { "lo": -PI, "hi": PI },
            })
        );
    }
}

// --- spec 010 FR-019: the bounding cap contains every contained point (U-016) ---

/// The probes' 64-bit LCG (`specs/010-formal-verify-latlng/probe/rust/src/main.rs`), so the
/// tests below draw the probes' rectangles and points.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// A longitude of `r` at the fraction `t` of its length.
fn lng_at(r: Rect, t: f64) -> f64 {
    if r.lng.is_inverted() {
        let v = r.lng.lo + t * r.lng.length();
        if v > PI { v - 2.0 * PI } else { v }
    } else {
        r.lng.lo + t * (r.lng.hi - r.lng.lo)
    }
}

/// Upstream's `GetCapBound` (`s2latlng_rect.cc:298–332`), as the library computed it before
/// FR-019: the cap the library widens.
fn upstream_cap_bound(r: Rect) -> Cap {
    if r.is_empty() {
        return Cap::empty();
    }
    let (pole_z, pole_angle) = if r.lat.lo + r.lat.hi < 0.0 {
        (-1.0, FRAC_PI_2 + r.lat.hi)
    } else {
        (1.0, FRAC_PI_2 - r.lat.lo)
    };
    let pole_cap = Cap::from_center_angle(
        Point::new(crate::r3::Vector {
            x: 0.0,
            y: 0.0,
            z: pole_z,
        }),
        Angle::from_radians((1.0 + 2.0 * f64::EPSILON) * pole_angle),
    );
    if r.lng.length() <= PI {
        let mut mid_cap = Cap::from_point(r.center().to_point());
        for v in RectVertex::iter() {
            mid_cap = mid_cap.add_point(r.vertex(v).to_point());
        }
        if mid_cap.height() < pole_cap.height() {
            return mid_cap;
        }
    }
    pole_cap
}

/// Asserts that `r.cap_bound()` contains every point of `pts` that `r` contains, and returns
/// how many it checked.
fn assert_bound_contains(r: Rect, pts: impl IntoIterator<Item = LatLng>) -> usize {
    let cap = r.cap_bound();
    let mut n = 0;
    for ll in pts {
        let p = ll.to_point();
        if r.contains_point(p) {
            n += 1;
            assert!(
                cap.contains_point(p),
                "{r:?}: the bound {cap:?} misses {ll:?}"
            );
        }
    }
    n
}

/// Spec 010 R0's two worst cases, which upstream's `GetCapBound` misses too
/// (`probe/cpp/capbound_cases.txt`): the north pole as a rectangle and a point one ulp past
/// the pole (a pole cap of radius 0), and a point on the low latitude bound five ulps inside
/// the high longitude bound (the cap around the centre).
#[test]
fn cap_bound_contains_r0_worst_cases() {
    let pole = Rect::new(
        r1::Interval::new(FRAC_PI_2, FRAC_PI_2),
        s1::Interval::new(-PI, PI),
    );
    let ll = LatLng::from_radians(1.570_796_326_794_896_8, 4.653_912_984_578_174_6e-1);
    assert_eq!(assert_bound_contains(pole, [ll]), 1);
    let mid = Rect::new(
        r1::Interval::new(1.071_221_527_018_078_5, 1.071_237_668_898_467_3),
        s1::Interval::new(-8.905_155_217_671_079e-1, -8.905_155_217_089_906e-1),
    );
    let ll = LatLng::from_radians(1.071_221_527_018_078_5, -8.905_155_217_089_911e-1);
    assert_eq!(assert_bound_contains(mid, [ll]), 1);
    // The cap around the centre is the one widened here, as upstream chose it.
    assert!(mid.cap_bound().center().0.z < 1.0);
}

/// The survey's section G, regenerated: 20,000 random rectangles and seven points on and
/// inside each one's boundary, of which upstream's bound missed 114 (spec 010 K4).
#[test]
fn cap_bound_contains_the_survey_points() {
    let mut g = Lcg(0x0060_0060_0060_0060);
    let mut checked = 0;
    for _ in 0..20_000 {
        let a = g.range(-FRAC_PI_2, FRAC_PI_2);
        let c = g.range(-FRAC_PI_2, FRAC_PI_2);
        let (l1, l2) = (g.range(-PI, PI), g.range(-PI, PI));
        let r = Rect::new(
            r1::Interval::new(a.min(c), a.max(c)),
            s1::Interval::new(l1, l2),
        );
        let lat = g.range(r.lat.lo, r.lat.hi);
        let lng = lng_at(r, g.unit());
        checked += assert_bound_contains(
            r,
            [
                LatLng::from_radians(r.lat.lo, r.lng.lo),
                LatLng::from_radians(r.lat.hi, r.lng.hi),
                LatLng::from_radians(r.lat.lo, lng),
                LatLng::from_radians(r.lat.hi, lng),
                LatLng::from_radians(lat, r.lng.lo),
                LatLng::from_radians(lat, r.lng.hi),
                LatLng::from_radians(lat, lng),
            ],
        );
    }
    assert_eq!(checked, 129_595, "the survey's contained points");
}

/// Spec 010 R0's sweep, smaller: rectangles near the poles, at the poles, single latitudes and
/// narrow longitudes, and for each the points on both latitude bounds and 1 and 3 ulps inside
/// them, at both poles, on both longitude bounds and 1, 2, 4 and 16 ulps inside them.
#[test]
fn cap_bound_contains_boundary_points() {
    fn step(x: f64, k: i32) -> f64 {
        (0..k.unsigned_abs()).fold(x, |y, _| if k > 0 { y.next_up() } else { y.next_down() })
    }
    let mut g = Lcg(0x0100_0100_0100_0100);
    let mut checked = 0;
    for _ in 0..4_000 {
        let k = (g.next() % 17) as i32;
        let (lo, hi) = match g.next() % 6 {
            0 => {
                let (a, c) = (
                    g.range(-FRAC_PI_2, FRAC_PI_2),
                    g.range(-FRAC_PI_2, FRAC_PI_2),
                );
                (a.min(c), a.max(c))
            }
            1 => {
                let lo = FRAC_PI_2 - g.unit() * 10f64.powi(-k);
                (lo, FRAC_PI_2)
            }
            2 => {
                let hi = -FRAC_PI_2 + g.unit() * 10f64.powi(-k);
                (-FRAC_PI_2, hi)
            }
            3 => (FRAC_PI_2, FRAC_PI_2),
            4 => {
                let a = g.range(-FRAC_PI_2, FRAC_PI_2);
                (a, a)
            }
            _ => {
                let a = g.range(0.0, FRAC_PI_2);
                (-a, a)
            }
        };
        let lng = match g.next() % 4 {
            0 => s1::Interval::full(),
            1 => {
                let c = g.range(-PI, PI);
                let w = g.unit() * 10f64.powi(-((g.next() % 12) as i32));
                s1::Interval::from_point_pair(c, (c + w).min(PI))
            }
            _ => s1::Interval::new(g.range(-PI, PI), g.range(-PI, PI)),
        };
        let r = Rect::new(r1::Interval::new(lo, hi), lng);
        let lats = [
            r.lat.lo,
            r.lat.hi,
            step(r.lat.lo, 1),
            step(r.lat.hi, -1),
            step(r.lat.lo, 3),
            step(r.lat.hi, -3),
            FRAC_PI_2,
            -FRAC_PI_2,
        ];
        let mut lngs = vec![r.lng.lo, r.lng.hi, lng_at(r, 0.5), lng_at(r, g.unit())];
        for k in [1, 2, 4, 16] {
            lngs.push(step(r.lng.lo, k));
            lngs.push(step(r.lng.hi, -k));
        }
        let pts = lats
            .iter()
            .flat_map(|&la| lngs.iter().map(move |&ln| LatLng::from_radians(la, ln)));
        checked += assert_bound_contains(r, pts);
    }
    assert!(checked > 100_000, "only {checked} contained points");
}

/// FR-019 keeps upstream's choice of cap and its centre, bit for bit, and widens the radius by
/// the documented rule: `expanded` by 10⁻¹⁵ rad, then 8ε of the length² (`UPSTREAM.md` U-016).
#[test]
fn cap_bound_widens_upstreams_cap() {
    let mut g = Lcg(0x0060_0060_0060_0060);
    for _ in 0..2_000 {
        let a = g.range(-FRAC_PI_2, FRAC_PI_2);
        let c = g.range(-FRAC_PI_2, FRAC_PI_2);
        let (l1, l2) = (g.range(-PI, PI), g.range(-PI, PI));
        let r = Rect::new(
            r1::Interval::new(a.min(c), a.max(c)),
            s1::Interval::new(l1, l2),
        );
        let (up, lib) = (upstream_cap_bound(r), r.cap_bound());
        assert_eq!(lib.center(), up.center(), "{r:?}");
        let wide = up.expanded(Angle::from_radians(1e-15));
        let want = wide
            .chord_radius()
            .plus_error(8.0 * f64::EPSILON * wide.chord_radius().length2());
        assert_eq!(
            lib.chord_radius().length2().to_bits(),
            want.length2().to_bits(),
            "{r:?}"
        );
    }
    assert!(Rect::empty().cap_bound().is_empty());
    assert!(Rect::full().cap_bound().is_full());
}
