// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap` construction (P-CPCON, spec 008 data-model §4).

#![allow(
    non_snake_case,
    reason = "`__` marks a split-family member (harness-conventions)"
)]

use s2rst::s1::{Angle, ChordAngle};
use s2rst::s2::Cap;
use std::f64::consts::PI;

use crate::cap_spec;
use crate::stubs;
use crate::value_domain::any_box_point;

/// The library's length² for upstream's `FromLength2` result: the same, except a negative length,
/// which spec 007's option B maps to `NEGATIVE` (−1), a documented deviation.
fn option_b(x: f64) -> f64 {
    if x < 0.0 { -1.0 } else { x }
}

/// P-CPCON-01 (correct result): `from_center_chord_angle` and `from_point` keep their arguments,
/// `from_center_height` is upstream's `FromLength2(2 h)` (with option B), and `empty()` and
/// `full()` are upstream's constants.
///
/// For every `c` in `BoxPoint` and every `f64` `x`, `h`: the cap from `(c, from_length2(x))`
/// has centre `c` and that radius, bit for bit; `from_point(c)` has centre `c` and radius `+0`;
/// `from_center_height(c, h)`'s length² is `option_b(upstream_from_height(h))`; `empty()` and
/// `full()` have centre (1, 0, 0) and length² −1 and 4.
#[kani::proof]
fn p_cpcon_01() {
    let c = any_box_point();
    let (x, h): (f64, f64) = (kani::any(), kani::any());
    let r = ChordAngle::from_length2(x);
    let a = Cap::from_center_chord_angle(c, r);
    assert!(a.center() == c && a.center().0.x.to_bits() == c.0.x.to_bits());
    assert!(a.chord_radius().length2().to_bits() == r.length2().to_bits());
    let p = Cap::from_point(c);
    assert!(
        p.center().0.y.to_bits() == c.0.y.to_bits() && p.chord_radius().length2().to_bits() == 0
    );
    let hc = Cap::from_center_height(c, h);
    assert!(
        hc.chord_radius().length2().to_bits()
            == option_b(cap_spec::upstream_from_height(h)).to_bits()
    );
    for (k, l) in [(Cap::empty(), -1.0), (Cap::full(), 4.0)] {
        assert!(k.center().0.x == 1.0 && k.center().0.y == 0.0 && k.center().0.z == 0.0);
        assert!(k.chord_radius().length2() == l);
    }
    kani::cover!(hc.is_empty(), "a negative height");
}

/// P-CPCON-01 (correct result), the area constructor: `from_center_area` is upstream's
/// `FromLength2(a / π)` (with option B).
///
/// For every `c` in `BoxPoint` and every `f64` `a`: `from_center_area(c, a)`'s length² is
/// `option_b(upstream_from_area(a))`, bit for bit. Weekly: the quotient compared with its
/// identical restatement timed out at 12 minutes (2026-10-24).
#[cfg(feature = "slow")]
#[kani::proof]
fn p_cpcon_01__area() {
    let c = any_box_point();
    let a: f64 = kani::any();
    let k = Cap::from_center_area(c, a);
    assert!(
        k.chord_radius().length2().to_bits() == option_b(cap_spec::upstream_from_area(a)).to_bits()
    );
}

/// P-CPCON-02 (correct result): `from_center_angle` clamps the angle to π: an angle above π gives
/// the cap of radius π, a negative angle the empty cap; its validity check never fires on a unit
/// centre.
///
/// For every unit `c` and every non-NaN angle `t`: if `t > π`, `from_center_angle(c, t)` equals
/// `from_center_angle(c, π)` bit for bit; if `t < 0`, it is empty; it is valid.
///
/// Stubs: `pure_is_unit` (L-CP-07, L-CP-04), `pure_from_angle` (007's P-CAVAL-04: the range of
/// `ChordAngle::from_angle`): two calls on π are one value.
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
#[kani::stub(s2rst::s1::ChordAngle::from_angle, crate::stubs::pure_from_angle)]
fn p_cpcon_02() {
    stubs::reset();
    let c = any_box_point();
    kani::assume(c.is_unit());
    let t: f64 = kani::any();
    kani::assume(!t.is_nan());
    let k = Cap::from_center_angle(c, Angle::from_radians(t));
    if t > PI {
        let pi = Cap::from_center_angle(c, Angle::from_radians(PI));
        assert!(k.chord_radius().length2().to_bits() == pi.chord_radius().length2().to_bits());
    }
    if t < 0.0 {
        assert!(k.is_empty());
    }
    assert!(k.is_valid());
    kani::cover!(t > PI, "an angle above π");
}
