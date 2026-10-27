// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! `Cap`'s bounds: cap, covering, rectangle, and the longitude reduction (P-CPBND, spec 008
//! data-model §4).

use s2rst::s2::Region;
use s2rst::s2::metric::MIN_WIDTH;

use crate::cap_spec;
use crate::stubs;
use crate::value_domain::{any_box_cap, any_box_point, any_lng_sum};

/// P-CPBND-01 (correct result): `cap_bound` is the cap itself, and the `Region` methods
/// `cap_bound` and `contains_point` are the inherent ones. (`Region::rect_bound` and
/// `cell_union_bound` call the inherent methods; their results go through the function
/// contracts, which do not repeat a value, so the delegation is read from `region.rs`.)
///
/// For every `a` in `BoxCap` and `p` in `BoxPoint`: `a.cap_bound()` has `a`'s centre and
/// radius bit for bit; `Region::cap_bound(&a)` too; `Region::contains_point(&a, &p) ==
/// a.contains_point(p)`.
///
/// Stubs: `pure_chord` (L-CP-01, L-CP-02).
#[kani::proof]
#[kani::stub(s2rst::s2::Point::chord_angle, crate::stubs::pure_chord)]
fn p_cpbnd_01() {
    stubs::reset();
    let (a, p) = (any_box_cap(), any_box_point());
    for b in [a.cap_bound(), Region::cap_bound(&a)] {
        assert!(b.center().0.x.to_bits() == a.center().0.x.to_bits());
        assert!(b.center().0.y.to_bits() == a.center().0.y.to_bits());
        assert!(b.center().0.z.to_bits() == a.center().0.z.to_bits());
        assert!(b.chord_radius().length2().to_bits() == a.chord_radius().length2().to_bits());
    }
    assert!(Region::contains_point(&a, &p) == a.contains_point(p));
}

/// P-CPBND-02 (correct result): the covering's level is upstream's, and keeps the cells at least
/// twice as wide as the radius.
///
/// For every finite angle radius `r` (a valid cap's is at most π, or −1 for the empty cap; the
/// checker reports `deriv / ∞` as a NaN division, though IEEE gives 0, so +∞ is left out after
/// replaying it natively: level −1, the six faces, both ways): the level
/// `cell_union_bound` computes, `MIN_WIDTH.max_level(r) − 1` (`cap.rs`, FR-017), is
/// `cap_spec::upstream_level(r)`; and where it is at least 0, `MIN_WIDTH.value(level) ≥ 2 r`.
/// (The level is computed inline in `cell_union_bound`, whose radius comes through `asin`; the
/// statement is on that expression, the link read from `cap.rs`.)
#[kani::proof]
fn p_cpbnd_02() {
    let r: f64 = kani::any();
    kani::assume(r.is_finite());
    let level = i32::from(MIN_WIDTH.max_level(r).as_u8()) - 1;
    assert!(level == cap_spec::upstream_level(r));
    if level >= 0 {
        assert!(MIN_WIDTH.value(level as u8) >= 2.0 * r);
    }
    kani::cover!(level == -1, "the six faces");
    kani::cover!(level == 29, "the finest level");
}

/// P-CPBND-04 (validity): `rect_bound` returns a valid rectangle, empty exactly for the empty
/// cap.
///
/// For every `a` in `BoxCap`: `a.rect_bound().is_valid()` and `a.rect_bound().is_empty() ==
/// a.is_empty()`.
///
/// Under contract: `asin_contract`, `atan2_contract`, `cos_contract` (research R0b, 84.6 s).
#[kani::proof]
#[kani::stub(f64::asin, crate::stubs::asin_contract)]
#[kani::stub(f64::atan2, crate::stubs::atan2_contract)]
#[kani::stub(f64::cos, crate::stubs::cos_contract)]
fn p_cpbnd_04() {
    let a = any_box_cap();
    let r = a.rect_bound();
    assert!(r.is_valid());
    assert!(r.is_empty() == a.is_empty());
}

/// P-CPBND-05 (exact output): the longitude reduction of `rect_bound` is exact: its result
/// differs from `x` by exactly 0 or ±2π (the `f64` nearest) and lies in [−π, π].
///
/// For every `x` in `LngSum` (|x| ≤ 3π/2): `r = reduce_longitude(x)` has `|r| ≤ π`, and
/// `x − r` is 0, `2π` or `−2π` with `r + (x − r) == x`. Stated on `cap_spec`'s copy of the
/// private function, which `cap_spec_tests` checks against `cap.rs`'s source (a drift guard).
#[kani::proof]
fn p_cpbnd_05() {
    let x = any_lng_sum();
    let r = cap_spec::reduce_longitude_copy(x);
    assert!(r.abs() <= std::f64::consts::PI);
    let k = x - r;
    let two_pi = 2.0 * std::f64::consts::PI;
    assert!(k == 0.0 || k == two_pi || k == -two_pi);
    assert!(r + k == x);
}
