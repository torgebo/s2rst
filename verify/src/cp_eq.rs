// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap`'s equality, approximate equality and codec law (P-CPEQ, P-CPCOD, spec 008 data-model
//! §4).

use s2rst::s1::Angle;
use s2rst::s2::encoding::{S2Decode, S2Encode};
use s2rst::s2::{Cap, Point};

use crate::cap_spec;
use crate::laws;
use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point, any_valid_cap};

/// P-CPEQ-01 (law): `==` is an equivalence on box caps (004's `laws::eq`), and any two empty
/// caps, and any two full caps, are equal whatever their centres (upstream's `operator==`).
///
/// For every `a`, `b`, `c` in `BoxCap`: reflexive, symmetric, transitive; and for every two box
/// points `p`, `q`: `Cap::empty()` around `p` equals it around `q`, and so does `full`.
#[kani::proof]
fn p_cpeq_01() {
    let (a, b, c) = (any_box_cap(), any_box_cap(), any_box_cap());
    laws::eq::all(&a, &b, &c);
    let (p, q) = (any_box_point(), any_box_point());
    let e = |x: Point| Cap::from_center_chord_angle(x, s2rst::s1::ChordAngle::NEGATIVE);
    let f = |x: Point| Cap::from_center_chord_angle(x, s2rst::s1::ChordAngle::STRAIGHT);
    assert!(e(p) == e(q));
    assert!(f(p) == f(q));
    kani::cover!(
        a == b && a.center() != b.center(),
        "equal caps, different centres"
    );
}

/// P-CPEQ-02 (correct result): `approx_eq_with` is upstream's `ApproxEquals`
/// (`s2cap.cc:312–322`), and `approx_eq` is it at the default 10⁻¹⁴.
///
/// For every `a`, `b` in `BoxCap` and every `f64` `e`: `a.approx_eq_with(b, e)` is
/// `(ApproxEquals(centres, e) && |r²_a − r²_b| ≤ e) || (empty(a) && r²_b ≤ e) ||
/// (empty(b) && r²_a ≤ e) || (full(a) && r²_b ≥ 2 − e) || (full(b) && r²_a ≥ 2 − e)`, with the
/// centres' test `Point::approx_eq_angle` (upstream's `S2::ApproxEquals`, `S1Angle(a, b) ≤ e`).
///
/// Stubs: `pure_approx_eq_angle` (purity only: the statement calls the centres' test the
/// library calls, and the checker must see one value).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::approx_eq_angle, crate::stubs::pure_approx_eq_angle)]
fn p_cpeq_02() {
    stubs::reset();
    let (a, b) = (any_box_cap(), any_box_cap());
    let e: f64 = kani::any();
    let (ra, rb) = (cap_spec::radius2(a), cap_spec::radius2(b));
    let centres = a
        .center()
        .approx_eq_angle(b.center(), Angle::from_radians(e));
    let upstream = (centres && (ra - rb).abs() <= e)
        || (cap_spec::empty(a) && rb <= e)
        || (cap_spec::empty(b) && ra <= e)
        || (cap_spec::full(a) && rb >= 2.0 - e)
        || (cap_spec::full(b) && ra >= 2.0 - e);
    assert!(a.approx_eq_with(b, e) == upstream);
    assert!(a.approx_eq(b) == a.approx_eq_with(b, 1e-14));
    kani::cover!(upstream, "approximately equal");
}

/// P-CPCOD-01 (law): every valid cap round-trips through its encoding under `Cap`'s own
/// equality (the `CORE_TRAITS.md` codec law). 003's P-FX-03 proves the bytes; this states the
/// trait law for the type's equality.
///
/// For every `c` in `ValidCap`: `Cap::decode(c.encode())` succeeds and equals `c`.
///
/// Stubs: `pure_is_unit` (L-CP-07, L-CP-04): the decoder checks the validity the domain assumes
/// (research R0b, 8.8 s).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::is_unit, crate::stubs::pure_is_unit)]
fn p_cpcod_01() {
    stubs::reset();
    let c = any_valid_cap();
    let mut buf = Vec::with_capacity(32);
    c.encode(&mut buf).unwrap();
    let d = Cap::decode(&mut buf.as_slice()).unwrap();
    assert!(d == c);
    kani::cover!(c.is_empty(), "the empty cap");
}
