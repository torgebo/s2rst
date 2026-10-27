// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Input-domain generators for the value-type harnesses (spec 004 data-model §2, spec 005
//! data-model §2, spec 006 data-model §2, spec 007 data-model §2, spec 008 data-model §2;
//! contracts/harness-conventions.md).
//!
//! Every domain is built from the reference descriptions, never from the implementation's own
//! `is_valid`: cells from 001's model decompositions, intervals from [`arc_spec::valid`]. A
//! harness's domain therefore does not depend on the code under test. Intervals are struct
//! literals, which the public fields allow, so a harness reaches every valid pair, including
//! those no constructor returns (research R4).

use s2rst::r1;
use s2rst::r2;
use s2rst::s1::{Angle, ChordAngle, Interval};
use s2rst::s2::{Cap, Cell, CellEdge, LatLng, Point, Rect};

use crate::arc_spec;
use crate::chord_spec;
use crate::domain;
use crate::point_spec;
use crate::rect_spec;
use crate::spec::Decomposed;
use crate::uv_spec;

/// `ValidCell`: the cell of every valid identifier, with its decomposition.
pub fn any_cell() -> (Cell, Decomposed) {
    let (id, d) = domain::any_valid();
    (Cell::from_cell_id(id), d)
}

/// `ValidCell(L)`: the cell of every valid identifier at `level`.
pub fn any_cell_at(level: u8) -> (Cell, Decomposed) {
    let (id, d) = domain::any_valid_at(level);
    (Cell::from_cell_id(id), d)
}

/// `ValidInterval`: every pair of `f64` bounds that [`arc_spec::valid`] accepts, including the
/// empty, full, inverted and single-point intervals and the K4 corner.
pub fn any_interval() -> Interval {
    let x = Interval {
        lo: kani::any(),
        hi: kani::any(),
    };
    kani::assume(arc_spec::valid(x.lo, x.hi));
    x
}

/// `Point`: every `f64` in [−π, π] (no NaN).
pub fn any_point() -> f64 {
    let p: f64 = kani::any();
    kani::assume(arc_spec::in_range(p));
    p
}

/// `AnyF64`: every `f64` bit pattern, NaN and the infinities included.
pub fn any_f64() -> f64 {
    kani::any()
}

/// `Edge`: the four edges of a cell.
pub fn any_edge() -> CellEdge {
    let k: u8 = kani::any();
    kani::assume(k < 4);
    match k {
        0 => CellEdge::Bottom,
        1 => CellEdge::Right,
        2 => CellEdge::Top,
        _ => CellEdge::Left,
    }
}

// ---- spec 005 ------------------------------------------------------------------------------

/// `Real`: every pair of non-NaN `f64` bounds, the infinities, signed zeros and empty
/// intervals included (spec 005 research R4). Struct literals, as for [`any_interval`].
pub fn any_real() -> r1::Interval {
    let x = r1::Interval {
        lo: kani::any(),
        hi: kani::any(),
    };
    kani::assume(!x.lo.is_nan() && !x.hi.is_nan());
    x
}

/// `FiniteReal`: both bounds finite, the domain of the arithmetic operations, where `∞ − ∞`
/// cannot arise (spec 005 FR-004, FR-017).
pub fn any_finite_real() -> r1::Interval {
    let x = r1::Interval {
        lo: kani::any(),
        hi: kani::any(),
    };
    kani::assume(x.lo.is_finite() && x.hi.is_finite());
    x
}

/// `Number`: every `f64` except NaN.
pub fn any_number() -> f64 {
    let p: f64 = kani::any();
    kani::assume(!p.is_nan());
    p
}

/// `ValidRect`: every rectangle that [`rect_spec::valid`] accepts, as a struct literal from
/// four arbitrary bounds, never through the library's `is_valid`. It includes the empty and
/// full rectangles, the poles, inverted longitudes and 004's seam corner.
pub fn any_rect() -> Rect {
    let r = Rect {
        lat: r1::Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
        lng: Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
    };
    kani::assume(rect_spec::valid(r));
    r
}

/// `AnyLatLng`: every latitude–longitude with non-NaN components, valid or not: the domain of
/// the operations that screen out invalid points (`add_point`, `contains_lat_lng`; spec 005
/// K2).
pub fn any_lat_lng() -> LatLng {
    LatLng::from_radians(any_number(), any_number())
}

/// `ValidLatLng`: `|lat| ≤ π/2`, `|lng| ≤ π`, the domain of the operations that check it only in
/// debug builds (spec 005 K3).
pub fn any_valid_lat_lng() -> LatLng {
    let (lat, lng): (f64, f64) = (kani::any(), kani::any());
    kani::assume(rect_spec::ll_valid(lat, lng));
    LatLng::from_radians(lat, lng)
}

// ---- spec 006 ------------------------------------------------------------------------------

/// Four arbitrary `f64` bounds as a u-v rectangle literal, NaN included: the input of the
/// generators below and of the validity check's own property (P-QVAL-01).
pub fn any_uv_bounds() -> r2::Rect {
    r2::Rect {
        x: r1::Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
        y: r1::Interval {
            lo: kani::any(),
            hi: kani::any(),
        },
    }
}

/// `UvRect`: every rectangle [`uv_spec::valid`] accepts (both axes empty or both not, no NaN
/// bound), as a struct literal, never through the library's `is_valid`, which lets a NaN
/// bound through (spec 006 K4). It includes the empty rectangle's every representation,
/// single points, infinite bounds and signed zeros.
pub fn any_uv_rect() -> r2::Rect {
    let r = any_uv_bounds();
    kani::assume(uv_spec::valid(r));
    r
}

/// `FiniteUvRect`: a `UvRect` with four finite bounds, the domain of the arithmetic
/// operations, where `∞ − ∞` cannot arise (spec 006 FR-004, FR-018).
pub fn any_finite_uv_rect() -> r2::Rect {
    let r = any_uv_bounds();
    kani::assume(uv_spec::valid(r) && uv_spec::finite(r));
    r
}

/// `UvPoint`: two coordinates other than NaN, the infinities included.
pub fn any_uv_point() -> r2::Point {
    r2::Point {
        x: any_number(),
        y: any_number(),
    }
}

/// `AnyTriple`: three arbitrary `f64` coordinates, NaN and the infinities included: the domain
/// of the order, the comparisons, the conversions and the classifications.
pub fn any_triple() -> Point {
    point_spec::point(kani::any(), kani::any(), kani::any())
}

/// `PlainTriple`: no NaN coordinate; the infinities included.
pub fn any_plain_triple() -> Point {
    let p = any_triple();
    kani::assume(!p.0.x.is_nan() && !p.0.y.is_nan() && !p.0.z.is_nan());
    p
}

/// `FiniteTriple`: three finite coordinates, the floor of the arithmetic operations' domain
/// (spec 006 FR-004), where no `∞ − ∞` or `0 · ∞` arises.
pub fn any_finite_triple() -> Point {
    let p = any_triple();
    kani::assume(p.0.x.is_finite() && p.0.y.is_finite() && p.0.z.is_finite());
    p
}

/// `2⁵¹¹`: a coordinate at most this large squares to at most `2¹⁰²²`, so three of them sum to a
/// finite squared length.
const TWO_511: f64 = f64::from_bits((1023 + 511) << 52);

/// `BoundedTriple`: every coordinate finite and at most `2⁵¹¹` in magnitude, a part of
/// `NormableTriple` (finite coordinates whose squared length is finite too,
/// [`point_spec::normable`]) stated without the products: the per-change domain of
/// `from_coords` and `normalize`. The checker reports the `1 / ∞` of an overflowing squared
/// length as NaN, though IEEE-754 gives zero, so `NormableTriple` is the largest domain it can
/// prove (spec 006 research R0, finding 1); over all of it the checker times out at 12 minutes,
/// and over this part it finishes in seconds (spec 006, T026's measurement).
pub fn any_bounded_triple() -> Point {
    let p = any_triple();
    kani::assume(p.0.x.abs() <= TWO_511 && p.0.y.abs() <= TWO_511 && p.0.z.abs() <= TWO_511);
    p
}

/// `NormableBand`: the rest of `NormableTriple`, a finite squared length with some coordinate
/// above `2⁵¹¹` (so in `(2⁵¹¹, 2⁵¹²)`): the weekly members of `from_coords` and `normalize`.
#[cfg_attr(
    not(feature = "slow"),
    allow(dead_code, reason = "only the weekly members use it")
)]
pub fn any_normable_band_triple() -> Point {
    let p = any_triple();
    kani::assume(point_spec::normable(p) && point_spec::max_abs(p) > TWO_511);
    p
}

/// `BoxPoint`: every coordinate in `[−1, 1]`, the box that holds every unit point: the domain
/// of the geometric operations (spec 006 research R4). It keeps every sum below 2 and every
/// product below 4, so the cross products cannot overflow, without the three symbolic products
/// that assuming unit length would cost.
pub fn any_box_point() -> Point {
    let p = any_triple();
    kani::assume(point_spec::in_box(p));
    p
}

/// `NormalizableBoxPoint`: a `BoxPoint` whose largest coordinate is at least `2⁻²⁴²`
/// ([`point_spec::normalizable`]), the precondition of `ortho`'s non-zero result.
pub fn any_normalizable_box_point() -> Point {
    let p = any_triple();
    kani::assume(point_spec::in_box(p) && point_spec::normalizable(p));
    p
}

/// `FiniteAngle`: every finite angle, in radians.
pub fn any_finite_angle() -> Angle {
    let a: f64 = kani::any();
    kani::assume(a.is_finite());
    Angle::from_radians(a)
}

/// `Chord`: every canonical chord angle (spec 007 data-model §2): `NEGATIVE`, every length in
/// [0, 4] (either zero), and `INFINITY`, which `from_length2` cannot build (it clamps +∞ to 4).
/// The lengths go through `from_length2` with a canonical argument, which it keeps as it is,
/// before and after FR-017, so the generator does not depend on the change under test.
pub fn any_chord() -> ChordAngle {
    if kani::any() {
        ChordAngle::INFINITY
    } else {
        let x: f64 = kani::any();
        kani::assume(x == chord_spec::NEGATIVE || (0.0..=chord_spec::STRAIGHT).contains(&x));
        ChordAngle::from_length2(x)
    }
}

/// `ChordLength`: the non-special chord angles, every length in [0, 4] (either zero): the
/// documented domain of the arithmetic and the trigonometry (FR-019).
pub fn any_chord_length() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume((0.0..=chord_spec::STRAIGHT).contains(&x));
    ChordAngle::from_length2(x)
}

/// `AnyAngle`: every angle, in radians, NaN and the infinities included; the type has no
/// validity rule (spec 007 data-model §2).
pub fn any_angle() -> Angle {
    Angle::from_radians(kani::any())
}

/// `CapRadius`: a valid cap's radius, `NEGATIVE` or a length in [0, 4] (spec 008 data-model §2).
fn any_cap_radius() -> ChordAngle {
    let x: f64 = kani::any();
    kani::assume(x == chord_spec::NEGATIVE || (0.0..=chord_spec::STRAIGHT).contains(&x));
    ChordAngle::from_length2(x)
}

/// `BoxCap`: a centre in the box [−1, 1]³ (`BoxPoint`) and a valid cap's radius. Every valid cap
/// is one (a centre outside the box is not unit length), and the laws whose meaning does not need
/// unit length are stated over all of them (spec 008 research R4).
pub fn any_box_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_cap_radius())
}

/// `NonEmptyCap`: a `BoxCap` whose radius is a length in [0, 4].
pub fn any_nonempty_cap() -> Cap {
    Cap::from_center_chord_angle(any_box_point(), any_chord_length())
}

/// `ValidCap`: a `BoxCap` with a unit centre. Assuming `is_unit` on the real code costs a
/// symbolic squared length and timed out in spec 008's R0, so **a harness using this domain
/// stubs `Point::is_unit` with `stubs::pure_is_unit`** (research R4, R6).
pub fn any_valid_cap() -> Cap {
    let c = any_box_cap();
    kani::assume(c.center().is_unit());
    c
}

/// `FiniteNonNegAngle`: a finite angle of at least 0 radians, `expanded`'s domain.
pub fn any_finite_nonneg_angle() -> Angle {
    let a: f64 = kani::any();
    kani::assume(a.is_finite() && a >= 0.0);
    Angle::from_radians(a)
}

/// `LngSum`: |x| ≤ 3π/2, the domain of `rect_bound`'s longitude reduction (spec 008 FR-020): a
/// longitude in [−π, π] plus or minus an `asin` result in [0, π/2].
pub fn any_lng_sum() -> f64 {
    let x: f64 = kani::any();
    kani::assume(x.abs() <= 1.5 * std::f64::consts::PI);
    x
}
