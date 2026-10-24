// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Written for this crate (not ported from upstream S2).

//! Real-interval measures (P-RMEA, spec 005 data-model §4.1): the centre, the length and the
//! directed Hausdorff distance, over finite bounds (`FiniteReal`). Their infinite corners are
//! documented, not proven (FR-017; `UPSTREAM.md` U-009, U-010).

use crate::real_spec;
use crate::value_domain::any_finite_real;

/// P-RMEA-01 (correct result): the centre of a finite non-empty interval lies in it, and the
/// length is `hi − lo`, non-negative for a non-empty interval.
///
/// For every `x` in `FiniteReal`: `¬empty(x) ⇒ member(x, x.center())`;
/// `x.length()` is `x.hi − x.lo`, bit for bit; `¬empty(x) ⇒ x.length() ≥ 0`.
#[kani::proof]
fn p_rmea_01() {
    let x = any_finite_real();
    let len = x.length();
    assert_eq!(len.to_bits(), (x.hi - x.lo).to_bits());
    if !real_spec::empty(x) {
        assert!(real_spec::member(x, x.center()));
        assert!(len >= 0.0);
    }
    kani::cover!(x.lo > f64::MAX / 2.0, "bounds beyond f64::MAX / 2 (U-010)");
    kani::cover!(len.is_infinite(), "a length that overflows");
}

/// P-RMEA-02 (correct result): the directed Hausdorff distance is non-negative; it is 0
/// exactly when the second interval contains the first, infinite when only the second is
/// empty, and 0 from an empty interval.
///
/// For every `x`, `y` in `FiniteReal`, with `d = x.directed_hausdorff_distance(y)`: `d ≥ 0`;
/// `empty(x) ⇒ d == 0`; `¬empty(x) ∧ empty(y) ⇒ d == ∞`; for non-empty `x` and `y`,
/// `d == 0 ⇔ real_spec::contains(y, x)`. (Unlike the circular interval's P-IMEA-06, the
/// converse holds exactly: a rounded difference of finite floats has the sign of the exact
/// one.)
#[kani::proof]
fn p_rmea_02() {
    let (x, y) = (any_finite_real(), any_finite_real());
    let d = x.directed_hausdorff_distance(y);
    assert!(d >= 0.0);
    if real_spec::empty(x) {
        assert!(d == 0.0);
    } else if real_spec::empty(y) {
        assert!(d == f64::INFINITY);
    } else {
        assert_eq!(d == 0.0, real_spec::contains(y, x));
    }
    kani::cover!(d > 0.0 && d.is_finite(), "a positive distance");
    kani::cover!(d == 0.0 && !x.is_empty() && !y.is_empty(), "contained");
}
